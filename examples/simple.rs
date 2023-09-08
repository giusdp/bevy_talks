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
    handle: Handle<RawTalk>,
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
    let h: Handle<RawTalk> = server.load("talks/simple.talk.ron");
    commands.insert_resource(SimpleTalkAsset { handle: h });
}

fn check_loading(
    server: Res<AssetServer>,
    simple_sp_asset: Res<SimpleTalkAsset>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let load_state = server.get_load_state(&simple_sp_asset.handle);
    if load_state == LoadState::Loaded {
        next_state.set(AppState::Loaded);
    }
}

fn setup_talk(
    mut commands: Commands,
    raws: Res<Assets<RawTalk>>,
    simple_sp_asset: Res<SimpleTalkAsset>,
) {
    let raw_sp = raws.get(&simple_sp_asset.handle).unwrap();
    let talk = Talk::build(&raw_sp).unwrap();

    commands.spawn(TalkerBundle {
        talk: talk,
        activated: Default::default(),
        interaction: Default::default(),
        // display: TerminalDisplay::default(),
    });

    // commands.spawn(Talk);

    println!();
    println!("Press space to advance the conversation.");
    println!("-----------------------------------------");
}

fn interact(
    input: Res<Input<KeyCode>>,
    mut next_action_events: EventWriter<NextActionRequest>,
    talks: Query<Entity, With<Talk>>,
) {
    if input.just_pressed(KeyCode::Space) {
        let e = talks.single();
        next_action_events.send(NextActionRequest(e));
    }
}

fn print(sp_query: Query<&Talk, Changed<Talk>>) {
    for sp in sp_query.iter() {
        // extract actors names into a vector
        let actors = sp
            .action_actors()
            .iter()
            .map(|a| a.name.to_owned())
            .collect::<Vec<String>>();

        let mut speaker = "Narrator";
        if actors.len() > 0 {
            speaker = actors[0].as_str();
        }

        match sp.node_kind() {
            TalkNodeKind::Talk => println!("{}: {}", speaker, sp.text()),
            TalkNodeKind::Join => println!("--- {actors:?} enters the scene."),
            TalkNodeKind::Leave => println!("--- {actors:?} exit the scene."),
            TalkNodeKind::Choice => {
                println!("Choices:");
                for (i, choice) in sp.choices().unwrap().iter().enumerate() {
                    println!("{}: {}", i + 1, choice.text);
                }
            }
        };
    }
}
