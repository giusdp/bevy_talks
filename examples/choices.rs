//! A simple example that loads a talk with a choice from a file and you can select a choice.
use bevy::{asset::LoadState, prelude::*};
use bevy_talks::prelude::*;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    #[default]
    LoadAssets,
    Loaded,
}

#[derive(Resource)]
struct ChoiceTalkAsset {
    handle: Handle<TalkData>,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TalksPlugin))
        .add_state::<AppState>()
        .add_systems(OnEnter(AppState::LoadAssets), load_talks)
        .add_systems(Update, check_loading.run_if(in_state(AppState::LoadAssets)))
        .add_systems(OnEnter(AppState::Loaded), setup_talk)
        .add_systems(
            Update,
            (
                interact,
                print_text,
                print_choice,
                bevy::window::close_on_esc,
            )
                .run_if(in_state(AppState::Loaded)),
        )
        .run();
}

fn load_talks(mut commands: Commands, server: Res<AssetServer>) {
    let h: Handle<TalkData> = server.load("talks/choices.talk.ron");
    commands.insert_resource(ChoiceTalkAsset { handle: h });
}

fn check_loading(
    server: Res<AssetServer>,
    simple_sp_asset: Res<ChoiceTalkAsset>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let load_state = server.get_load_state(&simple_sp_asset.handle).unwrap();
    if load_state == LoadState::Loaded {
        next_state.set(AppState::Loaded);
    }
}

fn setup_talk(
    mut commands: Commands,
    talks: Res<Assets<TalkData>>,
    choice_talk_asset: Res<ChoiceTalkAsset>,
) {
    let choice_talk = talks.get(&choice_talk_asset.handle).unwrap();
    let talk_builder = TalkBuilder::default().fill_with_talk_data(choice_talk);
    commands.spawn_talk(talk_builder);

    println!("-----------------------------------------");
    println!("Press space to advance the conversation. And 1, 2 to pick a choice.");
    println!("-----------------------------------------");
}

fn interact(
    input: Res<Input<KeyCode>>,
    mut next_action_events: EventWriter<NextNodeRequest>,
    mut choose_action_events: EventWriter<ChooseNodeRequest>,
    talks: Query<Entity, With<Talk>>,
    choices: Query<&ChoiceNode, With<CurrentNode>>,
) {
    let talk_ent = talks.single();

    if input.just_pressed(KeyCode::Space) {
        next_action_events.send(NextNodeRequest::new(talk_ent));
    }

    // Note that you CAN have a TextNode component and a ChoiceNode component at the same time.
    // It would allow you to display some text beside the choices.
    if choices.iter().count() == 0 {
        return;
    }

    let choice_node = choices.single();

    if input.just_pressed(KeyCode::Key1) {
        choose_action_events.send(ChooseNodeRequest::new(talk_ent, choice_node.0[0].next));
    } else if input.just_pressed(KeyCode::Key2) {
        choose_action_events.send(ChooseNodeRequest::new(talk_ent, choice_node.0[1].next));
    }
}

fn print_text(mut text_events: EventReader<TextNodeEvent>) {
    for txt_event in text_events.read() {
        println!("{}", txt_event.text);
    }
}

fn print_choice(mut choice_events: EventReader<ChoiceNodeEvent>) {
    for choice_event in choice_events.read() {
        println!("Choices:");
        for (i, choice) in choice_event.choices.iter().enumerate() {
            println!("{}: {}", i + 1, choice.text);
        }
    }
}
