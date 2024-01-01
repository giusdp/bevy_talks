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
    bev_talk_handle: Handle<TalkData>,
    feri_talk_handle: Handle<TalkData>,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Interactable;

#[derive(Component)]
struct Dialogue;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.5, 0.5, 0.9)))
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
                deactive_talk_when_far,
                print,
                interact,
                advance_convo,
            )
                .run_if(in_state(AppState::Loaded)),
        )
        .add_systems(Update, bevy::window::close_on_esc)
        .run();
}

fn load_talks(mut commands: Commands, server: Res<AssetServer>) {
    let sp_a: Handle<TalkData> = server.load("talks/interact_a.talk.ron");
    let sp_b: Handle<TalkData> = server.load("talks/interact_b.talk.ron");
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
    let load_state_a = server.get_load_state(&sp_asset.bev_talk_handle).unwrap();
    let load_state_b = server.get_load_state(&sp_asset.feri_talk_handle).unwrap();
    if load_state_a == LoadState::Loaded && load_state_b == LoadState::Loaded {
        next_state.set(AppState::Loaded);
    }
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    raws: Res<Assets<TalkData>>,
    talk_asset: Res<TalkAsset>,
) {
    commands.spawn(Camera2dBundle::default());
    let player_handle: Handle<Image> = assets.load("images/player.png");
    commands.spawn((
        Player,
        SpriteBundle {
            texture: player_handle,
            transform: Transform::from_scale(Vec3::splat(4.))
                .with_translation(Vec3::new(0., 60., -1.)),
            ..default()
        },
    ));

    let bev: Handle<Image> = assets.load("images/bev.png");
    let bev_talk_data = raws.get(&talk_asset.bev_talk_handle).unwrap();
    let bev_talk_builder = Talk::builder().fill_from_talk_data(bev_talk_data);

    let mut talk_commands = commands.talks();
    let talk_graph_ent = talk_commands
        .spawn_talk(bev_talk_builder, ActiveTalk(false))
        .id();

    commands
        .spawn((
            Interactable,
            SpriteBundle {
                texture: bev,
                transform: Transform::from_scale(Vec3::splat(3.))
                    .with_translation(Vec3::new(-300., 0., 1.)),
                ..default()
            },
        ))
        .add_child(talk_graph_ent);

    let feri: Handle<Image> = assets.load("images/feri.png");
    let feri_talk_data = raws.get(&talk_asset.feri_talk_handle).unwrap();
    let feri_talk_builder = Talk::builder().fill_from_talk_data(feri_talk_data);

    let mut talk_commands = commands.talks();
    let talk_graph_ent = talk_commands
        .spawn_talk(feri_talk_builder, ActiveTalk(false))
        .id();
    commands
        .spawn((
            Interactable,
            SpriteBundle {
                texture: feri,
                transform: Transform::from_scale(Vec3::splat(3.))
                    .with_translation(Vec3::new(300., 0., 1.)),
                ..default()
            },
        ))
        .add_child(talk_graph_ent);

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
        player_transform.translation.x -= 450. * t.delta_seconds();
    }

    if input.pressed(KeyCode::D) {
        player_transform.translation.x += 450. * t.delta_seconds();
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
    characters: Query<(&Transform, &Children), With<Interactable>>,
    mut q_child: Query<(Entity, &Talk, &mut ActiveTalk)>,
    mut next_action_events: EventWriter<NextActionRequest>,
) {
    if input.just_pressed(KeyCode::E) {
        let player_transform = player_query.single();
        for (transform, children) in &characters {
            if transform.translation.distance(player_transform.translation) < 100. {
                let (e, t, mut active) = q_child.get_mut(children[0]).unwrap();
                active.0 = !active.0;
                if active.0 && t.current_kind == NodeKind::Start {
                    next_action_events.send(NextActionRequest(e));
                }
            }
        }
    }
}

fn deactive_talk_when_far(
    player_query: Query<&Transform, With<Player>>,
    characters: Query<(&Transform, &Children), With<Interactable>>,
    mut q_child: Query<&mut ActiveTalk>,
) {
    let player_transform = player_query.single();
    for (transform, children) in &characters {
        if transform.translation.distance(player_transform.translation) > 100. {
            let mut active = q_child.get_mut(children[0]).unwrap();
            if active.0 {
                active.0 = false;
            }
        }
    }
}

/// Print the current talk node (if changed) to the console.
fn print(talk_comps: Query<(&Talk, Ref<ActiveTalk>)>, mut texts: Query<&mut Text, With<Dialogue>>) {
    for (talk, active) in &talk_comps {
        // If talk was deactivated, clear the text
        if active.is_changed() && !active.0 {
            texts.single_mut().sections[0].value =
                "A-D to move. E near a character to interact. Space to advance convo.".to_string();
            continue;
        }

        // If just not active, skip
        if !active.0 {
            continue;
        }

        if talk.current_kind == NodeKind::Start {
            continue;
        }

        let speaker = &talk.current_actors[0];
        let display = match talk.current_kind {
            NodeKind::Talk => format!("{speaker}: {}", talk.current_text),
            _ => "Not implemented for this example".to_string(),
        };
        texts.single_mut().sections[0].value = display;
    }
}
