use bevy::{asset::LoadState, prelude::*};
use bevy_talks::prelude::*;

#[derive(Component, Default)]
struct ActiveTalk(bool);

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    #[default]
    LoadAssets,
    Loaded,
}

#[derive(Resource)]
struct TalkAsset {
    bev_talk_handle: Handle<RawTalk>,
    feri_talk_handle: Handle<RawTalk>,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Interactable;

#[derive(Component)]
struct Dialogue;

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
            (
                move_player,
                interact,
                print,
                deactive_talk_when_far,
                advance_convo,
            )
                .run_if(in_state(AppState::Loaded)),
        )
        .add_systems(Update, bevy::window::close_on_esc)
        .run();
}

fn load_talks(mut commands: Commands, server: Res<AssetServer>) {
    let sp_a: Handle<RawTalk> = server.load("talks/interact_a.talk.ron");
    let sp_b: Handle<RawTalk> = server.load("talks/interact_b.talk.ron");
    commands.insert_resource(TalkAsset {
        bev_talk_handle: sp_a,
        feri_talk_handle: sp_b,
    });
}

fn check_loading(
    server: Res<AssetServer>,
    sp_asset: Res<TalkAsset>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let load_state_a = server.get_load_state(&sp_asset.bev_talk_handle);
    let load_state_b = server.get_load_state(&sp_asset.feri_talk_handle);
    if load_state_a == LoadState::Loaded && load_state_b == LoadState::Loaded {
        next_state.set(AppState::Loaded);
    }
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    raws: Res<Assets<RawTalk>>,
    talk_asset: Res<TalkAsset>,
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

    let bev: Handle<Image> = assets.load("images/bev.png");
    let raw_talk_bev = raws.get(&talk_asset.bev_talk_handle).unwrap();
    commands.spawn((
        ActiveTalk(false),
        Interactable,
        SpriteBundle {
            texture: bev,
            transform: Transform::from_scale(Vec3::splat(3.))
                .with_translation(Vec3::new(-300., 0., 1.)),
            ..default()
        },
        TalkerBundle {
            talk: Talk::build(&raw_talk_bev).unwrap(),
            ..default()
        },
    ));

    let feri: Handle<Image> = assets.load("images/feri.png");
    let raw_talk_feri = raws.get(&talk_asset.feri_talk_handle).unwrap();
    commands.spawn((
        ActiveTalk(false),
        Interactable,
        SpriteBundle {
            texture: feri,
            transform: Transform::from_scale(Vec3::splat(3.))
                .with_translation(Vec3::new(300., 0., 1.)),
            ..default()
        },
        TalkerBundle {
            talk: Talk::build(&raw_talk_feri).unwrap(),
            ..default()
        },
    ));

    // the ui for the talks
    commands.spawn((
        Dialogue,
        TextBundle::from_section(
            "A-D to move. E near a character to interact. Space to advance convo.",
            TextStyle {
                font_size: 30.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_text_alignment(TextAlignment::Center)
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.),
            margin: UiRect {
                left: Val::Auto,
                right: Val::Auto,
                bottom: Val::Px(20.),
                ..default()
            },
            ..default()
        }),
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

fn advance_convo(
    input: Res<Input<KeyCode>>,
    mut next_action_events: EventWriter<NextActionRequest>,
    talks: Query<(Entity, &ActiveTalk), With<Talk>>,
) {
    if input.just_pressed(KeyCode::Space) {
        for (entity, active) in talks.iter() {
            if active.0 {
                next_action_events.send(NextActionRequest(entity));
            }
        }
    }
}

fn interact(
    input: Res<Input<KeyCode>>,
    player_query: Query<&Transform, With<Player>>,
    mut talks: Query<(Entity, &Transform, &mut ActiveTalk), With<Interactable>>,
    mut init_talk_events: EventWriter<InitTalkRequest>,
) {
    if input.just_pressed(KeyCode::E) {
        let player_transform = player_query.single();
        for (entity, transform, mut active) in talks.iter_mut() {
            if transform.translation.distance(player_transform.translation) < 100. {
                if active.0 {
                    active.0 = false;
                } else {
                    active.0 = true;
                    init_talk_events.send(InitTalkRequest(entity));
                }
            }
        }
    }
}

fn deactive_talk_when_far(
    player_query: Query<&Transform, With<Player>>,
    mut talks: Query<(&Transform, &mut ActiveTalk)>,
) {
    let player_transform = player_query.single();
    for (transform, mut active) in talks.iter_mut() {
        if transform.translation.distance(player_transform.translation) > 100. {
            if active.0 {
                active.0 = false;
            }
        }
    }
}

fn print(
    talk_comps: Query<(Ref<CurrentText>, &CurrentActors, Ref<ActiveTalk>)>,
    mut texts: Query<&mut Text, With<Dialogue>>,
) {
    for (tt, ca, active) in talk_comps.iter() {
        // skip if comps were just added
        if active.is_added() || tt.is_added() {
            continue;
        }

        // If talk was deactivated, clear the text
        if active.is_changed() && !active.0 {
            texts.single_mut().sections[0].value =
                "A-D to move. E near a character to interact. Space to advance convo.".to_string();
            continue;
        }

        // skip if text was not changed
        if !tt.is_changed() {
            continue;
        }

        let speaker = if ca.0.len() > 0 {
            ca.0[0].name.as_str()
        } else {
            "Narrator"
        };

        texts.single_mut().sections[0].value = format!("{}: {}", speaker, tt.0)
    }
}
