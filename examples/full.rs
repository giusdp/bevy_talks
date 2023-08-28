use bevy::{asset::LoadState, prelude::*};
use bevy_talks::prelude::*;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    #[default]
    LoadAssets,
    Loaded,
}

#[derive(Resource)]
struct SimpleScreenplayAsset {
    handle: Handle<RawScreenplay>,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TalksPlugin))
        .add_state::<AppState>()
        .add_systems(OnEnter(AppState::LoadAssets), load_talks)
        .add_systems(Update, check_loading.run_if(in_state(AppState::LoadAssets)))
        .add_systems(OnEnter(AppState::Loaded), setup_screenplay)
        .add_systems(
            Update,
            (interact, print, bevy::window::close_on_esc).run_if(in_state(AppState::Loaded)),
        )
        .run();
}

fn load_talks(mut commands: Commands, server: Res<AssetServer>) {
    let h: Handle<RawScreenplay> = server.load("talks/full.screenplay.ron");
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
    let raw_sp = raws.get(&simple_sp_asset.handle).unwrap();
    let screenplay = ScreenplayBuilder::new().build(&raw_sp).unwrap();

    commands.spawn(screenplay);
    println!();
    println!("Press space to advance the conversation. And 1, 2 to pick a choice.");
}

fn print(sp_query: Query<&Screenplay, Changed<Screenplay>>) {
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

        match sp.action_kind() {
            ActionKind::Talk => println!("{}: {}", speaker, sp.text()),
            ActionKind::Enter => println!("--- {actors:?} enters the scene."),
            ActionKind::Exit => println!("--- {actors:?} exit the scene."),
            ActionKind::Choice => {
                println!("Choices:");
                for (i, choice) in sp.choices().unwrap().iter().enumerate() {
                    println!("{}: {}", i + 1, choice.text);
                }
            }
        };
    }
}

fn interact(
    input: Res<Input<KeyCode>>,
    sp_query: Query<(Entity, &Screenplay)>,
    mut next_action_ev_writer: EventWriter<NextActionRequest>,
    mut jump_ev_writer: EventWriter<JumpToActionRequest>,
) {
    let (sp_e, sp) = sp_query.single();

    if sp.action_kind() == ActionKind::Choice {
        if input.just_pressed(KeyCode::Key1) {
            let c = sp.choices().unwrap()[0].next;
            jump_ev_writer.send(JumpToActionRequest(sp_e, c));
        } else if input.just_pressed(KeyCode::Key2) {
            let c = sp.choices().unwrap()[1].next;
            jump_ev_writer.send(JumpToActionRequest(sp_e, c));
        }
    }

    if input.just_pressed(KeyCode::Space) {
        next_action_ev_writer.send(NextActionRequest(sp_e));
    }
}
