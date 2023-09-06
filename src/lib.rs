#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![forbid(unsafe_code)]
#![warn(clippy::doc_markdown)]
// Often exceeded by queries
#![allow(clippy::type_complexity)]
// Unhelpful for systems
#![allow(clippy::too_many_arguments)]

//! [`bevy_talks`] is a Bevy plugin that provides
//! the basics to build and handle dialogues in games.

use bevy::prelude::*;
use prelude::{JumpToActionRequest, NextActionRequest, RawTalk, TalkLoader};
use talks::talk::Talk;
use trigger::{OnEnableTrigger, OnUseTrigger, TalkTriggerer};

pub mod action;
pub mod display;
pub mod events;
pub mod loader;
pub mod prelude;
pub mod talker;
pub mod talks;
pub mod trigger;

/// The plugin that provides the basics to build and handle dialogues in games.
pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<TalkLoader>()
            .add_asset::<RawTalk>()
            .add_event::<NextActionRequest>()
            .add_event::<JumpToActionRequest>()
            .add_systems(
                Update,
                (
                    next_action_handler,
                    jump_action_handler,
                    handle_trigger::<OnUseTrigger>,
                    handle_trigger::<OnEnableTrigger>,
                ),
            );
    }
}

/// Handles `JumpToActionRequest` events by updating the active Talk.
///
/// This function is a Bevy system that listens for `JumpToActionRequest` events.
/// It calls `jump_to` on the active Talk and sends `ActorsEnterEvent` or `ActorsExitEvent` events
/// if the reached action is an enter or exit action, respectively.
fn jump_action_handler(
    mut jump_requests: EventReader<JumpToActionRequest>,
    mut sp_comps: Query<(Entity, &mut Talk)>,
) {
    for ev in jump_requests.iter() {
        if let Ok((_, mut sp)) = sp_comps.get_mut(ev.0) {
            match sp.jump_to(ev.1) {
                Ok(()) => info!("Jumped to action {}.", ev.1),
                Err(err) => error!("Jump action could not be set: {}", err),
            }
        }
    }
}

/// Handles `NextActionRequest` events by advancing the active Talk to the next action.
///
/// This function is a Bevy system that listens for `NextActionRequest` events.
/// It calls `next_action` on the active Talk and sends `ActorsEnterEvent` or `ActorsExitEvent` events
/// if the reached action is an enter or exit action, respectively.
fn next_action_handler(
    mut commands: Commands,
    mut next_requests: EventReader<NextActionRequest>,
    mut sp_comps: Query<&mut Talk>,
) {
    for ev in next_requests.iter() {
        if let Ok(mut sp) = sp_comps.get_mut(ev.0) {
            match sp.next_action() {
                Ok(()) => {
                    let maybe_ec = commands.get_entity(ev.0);
                    if let Some(ec) = maybe_ec {}
                    info!("Moved to next action!")
                }
                Err(err) => error!("Next action could not be set: {}", err),
            }
        }
    }
}

fn handle_trigger<T: TalkTriggerer + Component>(query: Query<(&Talk, &T)>) {
    for (sp, t) in query.iter() {
        t.trigger();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use action::ScriptAction;
    use events::JumpToActionRequest;

    /// A minimal Bevy app with the Talks plugin.
    pub fn minimal_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TalksPlugin));
        app
    }

    #[test]
    fn next_action_handler() {
        let mut app = minimal_app();
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![
                ScriptAction { ..default() },
                ScriptAction { id: 2, ..default() },
            ],
        };

        let sp = Talk::build(&raw_sp);
        assert!(sp.is_ok());

        let e = app.world.spawn(sp.unwrap()).id();

        app.world.send_event(NextActionRequest(e));
        app.update();

        let sp_spawned = app.world.get::<Talk>(e).unwrap();

        assert_eq!(sp_spawned.current_node.index(), 1);
    }

    #[test]
    fn jump_action_handler() {
        let mut app = minimal_app();
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![
                ScriptAction { ..default() },
                ScriptAction { id: 2, ..default() },
                ScriptAction { id: 3, ..default() },
            ],
        };

        let sp = Talk::build(&raw_sp);
        assert!(sp.is_ok());

        let e = app.world.spawn(sp.unwrap()).id();

        app.world.send_event(JumpToActionRequest(e, 3));
        app.update();

        let sp_spawned = app.world.get::<Talk>(e).unwrap();

        assert_eq!(sp_spawned.current_node.index(), 2);
    }
}
