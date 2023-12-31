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
            (interact, print, bevy::window::close_on_esc).run_if(in_state(AppState::Loaded)),
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
    let talk_builder = TalkBuilder::default().into_builder(choice_talk);
    commands.add(talk_builder.build());

    println!("-----------------------------------------");
    println!("Press space to advance the conversation. And 1, 2 to pick a choice.");
    println!("-----------------------------------------");
}

fn print(talk_comps: Query<Ref<Talk>>) {
    for talk in &talk_comps {
        if !talk.is_changed() || talk.is_added() {
            continue;
        }

        if talk.current_kind == NodeKind::Choice {
            println!("Choices:");
            for (i, choice) in talk.current_choices.iter().enumerate() {
                println!("{}: {}", i + 1, choice.text);
            }
        } else {
            println!("{}", talk.current_text);
        }
    }
}

fn interact(
    input: Res<Input<KeyCode>>,
    mut next_action_events: EventWriter<NextActionRequest>,
    mut choose_action_events: EventWriter<ChooseActionRequest>,
    talks: Query<(Entity, &Talk)>,
) {
    let (talk_ent, talk) = talks.single();

    if talk.current_kind == NodeKind::Choice {
        if input.just_pressed(KeyCode::Key1) {
            let next_ent = talk.current_choices[0].next;
            choose_action_events.send(ChooseActionRequest::new(talk_ent, next_ent));
        } else if input.just_pressed(KeyCode::Key2) {
            let next_ent = talk.current_choices[1].next;
            choose_action_events.send(ChooseActionRequest::new(talk_ent, next_ent));
        }
    }

    if input.just_pressed(KeyCode::Space) {
        next_action_events.send(NextActionRequest(talk_ent));
    }
}
