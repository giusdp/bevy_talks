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

#[derive(Resource)]
struct SimpleScreenplayAsset {
    handle: Handle<RawScreenplay>,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TalksPlugin))
        .add_state::<AppState>()
        .insert_resource(PrintEnabled(true))
        .add_systems(
            OnEnter(AppState::LoadAssets),
            load_talks.run_if(in_state(AppState::LoadAssets)),
        )
        .add_systems(Update, check_loading.run_if(in_state(AppState::LoadAssets)))
        .add_systems(OnEnter(AppState::Loaded), setup_screenplay)
        .add_systems(
            Update,
            (interact, print, bevy::window::close_on_esc).run_if(in_state(AppState::Loaded)),
        )
        .run();
}

fn load_talks(mut commands: Commands, server: Res<AssetServer>) {
    let h: Handle<RawScreenplay> = server.load("choices.screenplay.json");
    commands.insert_resource(SimpleScreenplayAsset { handle: h });
}

fn check_loading(
    server: Res<AssetServer>,
    simple_sp_asset: Res<SimpleScreenplayAsset>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let load_state = server.get_load_state(&simple_sp_asset.handle);
    if load_state == LoadState::Loaded {
        next_state.set(AppState::Loaded);
    }
}

fn setup_screenplay(
    mut commands: Commands,
    raws: Res<Assets<RawScreenplay>>,
    simple_sp_asset: Res<SimpleScreenplayAsset>,
) {
    let screenplay = ScreenplayBuilder::new()
        .with_raw_screenplay(simple_sp_asset.handle.clone())
        .build(&raws)
        .unwrap();

    commands.spawn(screenplay);
    println!();
    println!("Press space to advance the conversation. And 1, 2 to pick a choice.");
}

fn print(mut print_enabled: ResMut<PrintEnabled>, sp_query: Query<&Screenplay>) {
    if !print_enabled.0 {
        return;
    }

    for sp in sp_query.iter() {
        if sp.action_kind() == ActionKind::Choice {
            if let Some(choices) = sp.choices() {
                println!("Choices:");
                for (i, choice) in choices.iter().enumerate() {
                    println!("{}: {}", i + 1, choice.text);
                }
            }
        } else {
            println!("{}", sp.text());
        }
    }

    print_enabled.0 = false;
}

fn interact(
    input: Res<Input<KeyCode>>,
    mut sp_query: Query<&mut Screenplay>,
    mut print_enabled: ResMut<PrintEnabled>,
) {
    for mut sp in &mut sp_query {
        if sp.action_kind() == ActionKind::Choice {
            if input.just_pressed(KeyCode::Key1) {
                let c = sp.choices().unwrap()[0].next;
                choose(&mut sp, c, &mut print_enabled);
            } else if input.just_pressed(KeyCode::Key2) {
                let c = sp.choices().unwrap()[1].next;
                choose(&mut sp, c, &mut print_enabled);
            }
        }

        if input.just_pressed(KeyCode::Space) {
            match sp.next_action() {
                Ok(_) => print_enabled.0 = true,
                Err(e) => error!("Error: {:?}", e),
            }
        }
    }
}

fn choose(sp: &mut Screenplay, id: ActionId, print: &mut PrintEnabled) {
    match sp.jump_to(id) {
        Ok(_) => print.0 = true,
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
