use bevy::{asset::LoadState, prelude::*};
use bevy_talks::{builder::commands::TalkCommandsExt, prelude::*};

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
            (interact, print, bevy::window::close_on_esc).run_if(in_state(AppState::Loaded)),
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
    let talk_builder = TalkBuilder::default().fill_from_talk_data(talk);
    let mut talk_commands = commands.talks();
    talk_commands.spawn_talk(talk_builder, ());

    println!("-----------------------------------------");
    println!("Press space to advance the conversation.");
    println!("-----------------------------------------");
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
            NodeKind::Choice => {
                println!("Choices:");
                for (i, choice) in talk.current_choices.iter().enumerate() {
                    println!("{}: {}", i + 1, choice.text);
                }
            }
            _ => (),
        };
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
