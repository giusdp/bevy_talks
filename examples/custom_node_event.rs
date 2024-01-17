//! Example to show how to add an event emitter component to a node to define custom nodes.
use std::vec;

use bevy::prelude::*;
use bevy_talks::prelude::*;

#[derive(Component, Reflect, NodeEventEmitter, Default)]
#[reflect(Component)]
struct DanceStart {
    pub moves: Vec<String>,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TalksPlugin))
        .register_node_event::<DanceStart, DanceStartEvent>() // Register the component and event
        .add_systems(Startup, setup_talk)
        .add_systems(
            Update,
            (
                interact,
                print_text,
                react_to_dancing,
                bevy::window::close_on_esc,
            ),
        )
        .run();
}

/// Spawn the dialogue graph using the builder.
fn setup_talk(mut commands: Commands) {
    commands.spawn_talk(
        Talk::builder()
            .say("Oh lord he dancing")
            .with_component(DanceStart {
                moves: vec![
                    "dabs".to_string(),
                    "whips".to_string(),
                    "trips and fall".to_string(),
                ],
            }),
    );

    println!("-----------------------------------------");
    println!("Press space to advance the conversation.");
    println!("-----------------------------------------");
}

/// Advance the talk when the space key is pressed.
fn interact(
    input: Res<Input<KeyCode>>,
    mut next_action_events: EventWriter<NextNodeRequest>,
    talks: Query<Entity, With<Talk>>,
) {
    if input.just_pressed(KeyCode::Space) {
        next_action_events.send(NextNodeRequest::new(talks.single()));
    }
}

fn print_text(mut text_events: EventReader<TextNodeEvent>) {
    for txt_ev in text_events.read() {
        let mut speaker = "Narrator";
        if !txt_ev.actors.is_empty() {
            speaker = &txt_ev.actors[0];
        }

        println!("{speaker}: {}", txt_ev.text);
    }
}

fn react_to_dancing(mut dance_events: EventReader<DanceStartEvent>) {
    for dance in dance_events.read() {
        println!("He: {:?}", dance.moves);
    }
}
