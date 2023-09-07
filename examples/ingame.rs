use bevy::{asset::LoadState, prelude::*};
use bevy_talks::{
    prelude::*,
    talker::{Activated, TalkerBundle},
    talks::TalkNodeKind,
};

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    #[default]
    LoadAssets,
    Loaded,
}

#[derive(Resource)]
struct TalkAsset {
    sp_a: Handle<RawTalk>,
    sp_b: Handle<RawTalk>,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Interactable;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            TalksPlugin,
        ))
        .add_state::<AppState>()
        .add_systems(OnEnter(AppState::LoadAssets), load_talks)
        .add_systems(Update, check_loading.run_if(in_state(AppState::LoadAssets)))
        .add_systems(OnEnter(AppState::Loaded), setup)
        .add_systems(
            Update,
            (move_player, interact, print).run_if(in_state(AppState::Loaded)),
        )
        .add_systems(Update, bevy::window::close_on_esc)
        .run();
}

fn load_talks(mut commands: Commands, server: Res<AssetServer>) {
    let sp_a: Handle<RawTalk> = server.load("talks/interact_a.Talk.ron");
    let sp_b: Handle<RawTalk> = server.load("talks/interact_b.Talk.ron");
    commands.insert_resource(TalkAsset { sp_a, sp_b });
}

fn check_loading(
    server: Res<AssetServer>,
    sp_asset: Res<TalkAsset>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let load_state_a = server.get_load_state(&sp_asset.sp_a);
    let load_state_b = server.get_load_state(&sp_asset.sp_b);
    if load_state_a == LoadState::Loaded && load_state_b == LoadState::Loaded {
        next_state.set(AppState::Loaded);
    }
}

// fn setup_Talk(
//     mut commands: Commands,
//     raws: Res<Assets<RawTalk>>,
//     sp_asset: Res<TalkAsset>,
// ) {
//     let raw_sp = raws.get(&sp_asset.handle).unwrap();
//     let Talk = TalkBuilder::new().build(&raw_sp).unwrap();

//     commands.spawn(Talk);
//     println!();
//     println!("Press space to advance the conversation. And 1, 2 to pick a choice.");
// }

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    raws: Res<Assets<RawTalk>>,
    sp_asset: Res<TalkAsset>,
) {
    commands.spawn(Camera2dBundle::default());
    let player_handle: Handle<Image> = assets.load("images/player.png");
    commands.spawn((
        Player,
        SpriteBundle {
            texture: player_handle,
            transform: Transform::from_scale(Vec3::splat(4.))
                .with_translation(Vec3::new(0., 0., 10.)),
            ..default()
        },
    ));

    let a: Handle<Image> = assets.load("images/A.png");
    let b: Handle<Image> = assets.load("images/B.png");
    let raw_sp_a = raws.get(&sp_asset.sp_a).unwrap();
    let raw_sp_b = raws.get(&sp_asset.sp_b).unwrap();
    commands.spawn((
        Interactable,
        SpriteBundle {
            texture: a,
            transform: Transform::from_scale(Vec3::splat(2.))
                .with_translation(Vec3::new(-300., 0., 1.)),
            ..default()
        },
        TalkerBundle {
            talk: Talk::build(&raw_sp_a).unwrap(),
            ..default()
        },
    ));
    commands.spawn((
        Interactable,
        SpriteBundle {
            texture: b,
            transform: Transform::from_scale(Vec3::splat(2.))
                .with_translation(Vec3::new(300., 0., 1.)),
            ..default()
        },
        TalkerBundle {
            talk: Talk::build(&raw_sp_b).unwrap(),
            ..default()
        },
    ));
}

fn move_player(
    input: Res<Input<KeyCode>>,
    t: Res<Time>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let mut player_transform = query.single_mut();
    if input.pressed(KeyCode::A) {
        player_transform.translation.x -= 300. * t.delta_seconds();
    }

    if input.pressed(KeyCode::D) {
        player_transform.translation.x += 300. * t.delta_seconds();
    }
}

fn interact(
    input: Res<Input<KeyCode>>,
    player_query: Query<&Transform, With<Player>>,
    mut interactable_query: Query<(&Transform, &mut Activated), With<Interactable>>,
) {
    if input.just_pressed(KeyCode::E) {
        let player_transform = player_query.single();

        for (transform, mut active) in interactable_query.iter_mut() {
            if transform.translation.distance(player_transform.translation) < 100. {
                active.0 = !active.0;
                info!("Interacted with an interactable!");
            }
        }
    }
}

fn print(sp_query: Query<(&Talk, &Activated), Or<(Changed<Talk>, Changed<Activated>)>>) {
    for (sp, active) in sp_query.iter() {
        if !active.0 {
            continue;
        }

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
