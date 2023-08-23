#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![forbid(unsafe_code)]
#![warn(clippy::doc_markdown)]
// Often exceeded by queries
#![allow(clippy::type_complexity)]
// Unhelpful for systems
#![allow(clippy::too_many_arguments)]

//! [`bevy_screenplay`] is a Bevy plugin that provides
//! the basics to build and handle dialogues in games.

use bevy::prelude::*;
use prelude::{
    ActorsEnterEvent, ActorsExitEvent, JumpToActionRequest, NextActionRequest, RawScreenplay,
    ScreenplayLoader,
};
use screenplay::Screenplay;

pub mod action;
pub mod errors;
pub mod events;
pub mod loader;
pub mod prelude;
pub mod screenplay;
pub mod screenplay_builder;

/// Resource that keeps track of the currently active screenplay.
#[derive(Resource, Default)]
pub struct ActiveScreenplay {
    /// The entity containing the currently active screenplay.
    pub e: Option<Entity>,
    /// Whether the active screenplay has moved to the next action.
    pub changed: bool,
}

/// The plugin that provides the basics to build and handle dialogues in games.
pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveScreenplay>()
            .init_asset_loader::<ScreenplayLoader>()
            .add_asset::<RawScreenplay>()
            .add_event::<NextActionRequest>()
            .add_event::<JumpToActionRequest>()
            .add_event::<ActorsEnterEvent>()
            .add_event::<ActorsExitEvent>()
            .add_systems(Update, (next_action_handler, jump_action_handler));
    }
}

/// Handles `JumpToActionRequest` events by updating the active screenplay.
///
/// This function is a Bevy system that listens for `JumpToActionRequest` events.
/// It calls `jump_to` on the active screenplay and sends `ActorsEnterEvent` or `ActorsExitEvent` events
/// if the reached action is an enter or exit action, respectively.
fn jump_action_handler(
    mut jump_requests: EventReader<JumpToActionRequest>,
    mut sp_comps: Query<(Entity, &mut Screenplay)>,
    mut sp_res: ResMut<ActiveScreenplay>,
    mut enter_events: EventWriter<ActorsEnterEvent>,
    mut exit_events: EventWriter<ActorsExitEvent>,
) {
    for ev in jump_requests.iter() {
        if sp_res.e.is_none() {
            warn!("Jump Action Request received but no active screenplay set!");
            continue;
        }

        let e = sp_res.e.unwrap();
        if let Ok((_, mut sp)) = sp_comps.get_mut(e) {
            match sp.jump_to(ev.0) {
                Ok(()) => {
                    sp_res.changed = true;
                    match sp.action_kind() {
                        prelude::ActionKind::Enter => {
                            enter_events.send(ActorsEnterEvent(sp.action_actors().to_vec()));
                        }
                        prelude::ActionKind::Exit => {
                            exit_events.send(ActorsExitEvent(sp.action_actors().to_vec()))
                        }
                        _ => {}
                    };
                }
                Err(err) => error!("Jump action could not be set: {}", err),
            }
        }
    }
}

/// Handles `NextActionRequest` events by advancing the active screenplay to the next action.
///
/// This function is a Bevy system that listens for `NextActionRequest` events.
/// It calls `next_action` on the active screenplay and sends `ActorsEnterEvent` or `ActorsExitEvent` events
/// if the reached action is an enter or exit action, respectively.
fn next_action_handler(
    mut next_requests: EventReader<NextActionRequest>,
    mut sp_comps: Query<(Entity, &mut Screenplay)>,
    mut sp_res: ResMut<ActiveScreenplay>,
    mut enter_events: EventWriter<ActorsEnterEvent>,
    mut exit_events: EventWriter<ActorsExitEvent>,
) {
    for _ev in next_requests.iter() {
        if sp_res.e.is_none() {
            warn!("Next Action Request received but no active screenplay set!");
            continue;
        }

        let e = sp_res.e.unwrap();
        if let Ok((_, mut sp)) = sp_comps.get_mut(e) {
            match sp.next_action() {
                Ok(()) => {
                    sp_res.changed = true;
                    match sp.action_kind() {
                        prelude::ActionKind::Enter => {
                            enter_events.send(ActorsEnterEvent(sp.action_actors().to_vec()));
                        }
                        prelude::ActionKind::Exit => {
                            exit_events.send(ActorsExitEvent(sp.action_actors().to_vec()))
                        }
                        _ => {}
                    };
                }
                Err(err) => error!("Next action could not be set: {}", err),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use action::{ActionKind, Actor, ScriptAction};
    use events::JumpToActionRequest;
    use screenplay_builder::ScreenplayBuilder;

    /// A minimal Bevy app with the Talks plugin.
    pub fn minimal_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TalksPlugin));
        app
    }

    #[test]
    fn next_action_handler() {
        let mut app = minimal_app();
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![
                ScriptAction { ..default() },
                ScriptAction { id: 2, ..default() },
            ],
        };

        let sp = ScreenplayBuilder::new().build(&raw_sp);
        assert!(sp.is_ok());

        let e = app.world.spawn(sp.unwrap()).id();

        app.world.get_resource_mut::<ActiveScreenplay>().unwrap().e = Some(e);
        app.world.send_event(NextActionRequest);
        app.update();

        let sp_spawned = app.world.get::<Screenplay>(e).unwrap();

        assert_eq!(sp_spawned.current_node.index(), 1);
        assert_eq!(
            app.world
                .get_resource::<ActiveScreenplay>()
                .unwrap()
                .changed,
            true
        );
    }

    #[test]
    fn next_action_with_enter() {
        let mut app = minimal_app();
        let raw_sp = RawScreenplay {
            actors: vec![Actor {
                id: "a".to_string(),
                name: "Actor1".to_string(),
                ..default()
            }],
            script: vec![
                ScriptAction::default(),
                ScriptAction {
                    id: 2,
                    action: ActionKind::Enter,
                    actors: vec!["a".to_string()],
                    ..default()
                },
            ],
        };

        let sp = ScreenplayBuilder::new().build(&raw_sp);
        assert!(sp.is_ok());

        let e = app.world.spawn(sp.unwrap()).id();

        app.world.get_resource_mut::<ActiveScreenplay>().unwrap().e = Some(e);
        app.world.send_event(NextActionRequest);
        app.update();

        let sp_spawned = app.world.get::<Screenplay>(e).unwrap();

        // check that Actor Enter Event was sent
        let actor_enter_events = app
            .world
            .get_resource::<Events<ActorsEnterEvent>>()
            .unwrap();
        let mut actor_enter_reader = actor_enter_events.get_reader();
        let actor_enter = actor_enter_reader.iter(actor_enter_events).next().unwrap();

        assert_eq!(actor_enter.0.len(), 1);
        assert_eq!(actor_enter.0[0].id, "a");

        assert_eq!(sp_spawned.current_node.index(), 1);
        assert_eq!(
            app.world
                .get_resource::<ActiveScreenplay>()
                .unwrap()
                .changed,
            true
        );
    }

    #[test]
    fn next_action_with_exit() {
        let mut app = minimal_app();
        let raw_sp = RawScreenplay {
            actors: vec![Actor {
                id: "a".to_string(),
                name: "Actor1".to_string(),
                ..default()
            }],
            script: vec![
                ScriptAction::default(),
                ScriptAction {
                    id: 2,
                    action: ActionKind::Exit,
                    actors: vec!["a".to_string()],
                    ..default()
                },
            ],
        };

        let sp = ScreenplayBuilder::new().build(&raw_sp);
        assert!(sp.is_ok());

        let e = app.world.spawn(sp.unwrap()).id();

        app.world.get_resource_mut::<ActiveScreenplay>().unwrap().e = Some(e);
        app.world.send_event(NextActionRequest);
        app.update();

        let sp_spawned = app.world.get::<Screenplay>(e).unwrap();

        // check that Actor Enter Event was sent
        let actor_exit_events = app.world.get_resource::<Events<ActorsExitEvent>>().unwrap();
        let mut actor_exit_reader = actor_exit_events.get_reader();
        let actor_exit = actor_exit_reader.iter(actor_exit_events).next().unwrap();

        assert_eq!(actor_exit.0.len(), 1);
        assert_eq!(actor_exit.0[0].id, "a");

        assert_eq!(sp_spawned.current_node.index(), 1);
        assert_eq!(
            app.world
                .get_resource::<ActiveScreenplay>()
                .unwrap()
                .changed,
            true
        );
    }

    #[test]
    fn jump_action_handler() {
        let mut app = minimal_app();
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![
                ScriptAction { ..default() },
                ScriptAction { id: 2, ..default() },
                ScriptAction { id: 3, ..default() },
            ],
        };

        let sp = ScreenplayBuilder::new().build(&raw_sp);
        assert!(sp.is_ok());

        let e = app.world.spawn(sp.unwrap()).id();

        app.world.get_resource_mut::<ActiveScreenplay>().unwrap().e = Some(e);
        app.world.send_event(JumpToActionRequest(3));
        app.update();

        let sp_spawned = app.world.get::<Screenplay>(e).unwrap();

        assert_eq!(sp_spawned.current_node.index(), 2);
        assert_eq!(
            app.world
                .get_resource::<ActiveScreenplay>()
                .unwrap()
                .changed,
            true
        );
    }
}
