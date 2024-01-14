//! This example combines the simple linear talk with choices to have a more complete example with all the built-in nodes.
use bevy::{asset::LoadState, prelude::*};
use bevy_talks::prelude::*;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    #[default]
    LoadAssets,
    Loaded,
}

#[derive(Resource)]
struct FullTalkAsset {
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
                print_join,
                print_leave,
                print_choice,
                bevy::window::close_on_esc,
            )
                .run_if(in_state(AppState::Loaded)),
        )
        .run();
}

fn load_talks(mut commands: Commands, server: Res<AssetServer>) {
    let h: Handle<TalkData> = server.load("talks/full.talk.ron");
    commands.insert_resource(FullTalkAsset { handle: h });
}

fn check_loading(
    server: Res<AssetServer>,
    full_talk_asset: Res<FullTalkAsset>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let load_state = server.get_load_state(&full_talk_asset.handle).unwrap();
    if load_state == LoadState::Loaded {
        next_state.set(AppState::Loaded);
    }
}

fn setup_talk(
    mut commands: Commands,
    talks: Res<Assets<TalkData>>,
    full_talk_asset: Res<FullTalkAsset>,
) {
    let talk = talks.get(&full_talk_asset.handle).unwrap();
    let talk_builder = TalkBuilder::default().fill_with_talk_data(talk);
    commands.spawn_talk(talk_builder, ());

    println!("-----------------------------------------");
    println!("Press space to advance the conversation.");
    println!("-----------------------------------------");
}
/// Advance the talk when the space key is pressed and select choices with 1 and 2.
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
    for txt_ev in text_events.read() {
        let mut speaker = "Narrator";
        if !txt_ev.actors.is_empty() {
            speaker = &txt_ev.actors[0];
        }

        println!("{speaker}: {}", txt_ev.text);
    }
}

fn print_join(mut join_events: EventReader<JoinNodeEvent>) {
    for join_event in join_events.read() {
        println!("--- {:?} enters the scene.", join_event.actors);
    }
}

fn print_leave(mut leave_events: EventReader<LeaveNodeEvent>) {
    for leave_event in leave_events.read() {
        println!("--- {:?} exit the scene.", leave_event.actors);
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
