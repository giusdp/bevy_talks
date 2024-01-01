use bevy::{asset::LoadState, prelude::*};
use bevy_talks::prelude::*;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    #[default]
    LoadAssets,
    Loaded,
}

#[derive(Resource)]
struct SimpleTalkAsset {
    handle: Handle<TalkData>,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TalksPlugin))
        // region: boilerplate to load the talk
        .add_state::<AppState>()
        .add_systems(OnEnter(AppState::LoadAssets), load_talks)
        .add_systems(Update, check_loading.run_if(in_state(AppState::LoadAssets)))
        // endregion
        .add_systems(OnEnter(AppState::Loaded), setup_talk)
        .add_systems(
            Update,
            (interact, print, bevy::window::close_on_esc).run_if(in_state(AppState::Loaded)),
        )
        .add_systems(Update, bevy::window::close_on_esc)
        .run();
}

fn load_talks(mut commands: Commands, server: Res<AssetServer>) {
    let h: Handle<TalkData> = server.load("talks/simple.talk.ron");
    commands.insert_resource(SimpleTalkAsset { handle: h });
}

fn check_loading(
    server: Res<AssetServer>,
    simple_sp_asset: Res<SimpleTalkAsset>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let load_state = server.get_load_state(&simple_sp_asset.handle).unwrap();
    if load_state == LoadState::Loaded {
        next_state.set(AppState::Loaded);
    }
}

/// Spawn the dialogue graph with the given talk asset, using the builder.
fn setup_talk(
    mut commands: Commands,
    talks: Res<Assets<TalkData>>,
    simple_talk_asset: Res<SimpleTalkAsset>,
) {
    let simple_talk = talks.get(&simple_talk_asset.handle).unwrap();
    let talk_builder = TalkBuilder::default().fill_from_talk_data(simple_talk);

    let mut talk_commands = commands.talks();
    talk_commands.spawn_talk(talk_builder, ());

    println!("-----------------------------------------");
    println!("Press space to advance the conversation.");
    println!("-----------------------------------------");
}

/// Advance the talk when the space key is pressed.
fn interact(
    input: Res<Input<KeyCode>>,
    mut next_action_events: EventWriter<NextActionRequest>,
    talks: Query<Entity, With<Talk>>,
) {
    if input.just_pressed(KeyCode::Space) {
        next_action_events.send(NextActionRequest(talks.single()));
    }
}

/// Print the current talk node (if changed) to the console.
fn print(talk_comps: Query<Ref<Talk>>) {
    for talk in &talk_comps {
        if !talk.is_changed() || talk.is_added() {
            continue;
        }

        let actors = &talk.current_actors;

        let mut speaker = "Narrator";
        if !talk.current_actors.is_empty() {
            speaker = &talk.current_actors[0];
        }

        match talk.current_kind {
            NodeKind::Talk => println!("{speaker}: {}", talk.current_text),
            NodeKind::Join => println!("--- {actors:?} enters the scene."),
            NodeKind::Leave => println!("--- {actors:?} exit the scene."),
            NodeKind::Choice => println!("Not implemented"),
            _ => (),
        };
    }
}
