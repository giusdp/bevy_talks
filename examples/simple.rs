use bevy::{asset::LoadState, prelude::*};
use bevy_talks::prelude::*;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    #[default]
    LoadAssets,
    Loaded,
}

#[derive(Resource)]
struct PrintEnabled(bool);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TalksPlugin))
        .add_state::<AppState>()
        .insert_resource(PrintEnabled(true))
        .add_systems(Update, load_talks.run_if(in_state(AppState::LoadAssets)))
        .add_systems(OnEnter(AppState::Loaded), setup_screenplay)
        .add_systems(Update, (interact, print, bevy::window::close_on_esc))
        .run();
}

fn load_talks(server: Res<AssetServer>, mut next_state: ResMut<NextState<AppState>>) {
    let h: Handle<RawScreenplay> = server.load("simple.json");
    let load_state = server.get_load_state(h);
    if load_state == LoadState::Loaded {
        next_state.set(AppState::Loaded);
    }
}

fn setup_screenplay(
    mut commands: Commands,
    server: Res<AssetServer>,
    raws: Res<Assets<RawScreenplay>>,
) {
    let handle: Handle<RawScreenplay> = server.load("talk_only.json");
    let screenplay = ScreenplayBuilder::new()
        .with_raw_screenplay(handle)
        .build(&raws)
        .unwrap();

    commands.spawn(screenplay);

    println!("Press space to advance the conversation.");
}

fn print(mut print_enabled: ResMut<PrintEnabled>, sp_query: Query<&Screenplay>) {
    if !print_enabled.0 {
        return;
    }

    for sp in &sp_query {
        let actors = sp.actors();
        let mut speaker = "Narrator";
        if actors.len() > 0 {
            speaker = actors[0].name.as_str();
        }

        println!("{}: {}", speaker, sp.text());
        print_enabled.0 = false;
    }
}

fn interact(
    input: Res<Input<KeyCode>>,
    mut sp_query: Query<&mut Screenplay>,
    mut print_enabled: ResMut<PrintEnabled>,
) {
    if input.just_pressed(KeyCode::Space) {
        for mut sp in &mut sp_query {
            match sp.next_action() {
                Ok(_) => print_enabled.0 = true,
                Err(e) => error!("Error: {:?}", e),
            }
        }
    }
}
