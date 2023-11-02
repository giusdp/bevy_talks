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
    let h: Handle<RawTalk> = server.load("talks/choices.talk.ron");
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

fn setup_talk(
    mut commands: Commands,
    raws: Res<Assets<RawTalk>>,
    simple_sp_asset: Res<SimpleTalkAsset>,
    mut init_talk_events: EventWriter<InitTalkRequest>,
) {
    let raw_sp = raws.get(&simple_sp_asset.handle).unwrap();
    let talk = Talk::build(&raw_sp).unwrap();

    let e = commands.spawn(TalkerBundle { talk, ..default() }).id();
    init_talk_events.send(InitTalkRequest(e));

    println!();
    println!("Press space to advance the conversation. And 1, 2 to pick a choice.");
}

fn print(talk_comps: Query<(Ref<CurrentText>, &CurrentNodeKind, &CurrentChoices)>) {
    for (tt, kind, cc) in talk_comps.iter() {
        if !tt.is_changed() || tt.is_added() {
            continue;
        }

        if kind.0 == TalkNodeKind::Choice {
            if !cc.0.is_empty() {
                println!("Choices:");
                for (i, choice) in cc.0.iter().enumerate() {
                    println!("{}: {}", i + 1, choice.text);
                }
            }
        } else {
            println!("{}", tt.0);
        }
    }
}

fn interact(
    input: Res<Input<KeyCode>>,
    talk_comps: Query<(Entity, &CurrentNodeKind, &CurrentChoices)>,
    mut next_action_ev_writer: EventWriter<NextActionRequest>,
    mut jump_ev_writer: EventWriter<JumpToActionRequest>,
) {
    let (talker, kind, cc) = talk_comps.single();

    if kind.0 == TalkNodeKind::Choice {
        if input.just_pressed(KeyCode::Key1) {
            let c = cc.0[0].next;
            jump_ev_writer.send(JumpToActionRequest(talker, c));
        } else if input.just_pressed(KeyCode::Key2) {
            let c = cc.0[1].next;
            jump_ev_writer.send(JumpToActionRequest(talker, c));
        }
    }

    if input.just_pressed(KeyCode::Space) {
        next_action_ev_writer.send(NextActionRequest(talker));
    }
}
