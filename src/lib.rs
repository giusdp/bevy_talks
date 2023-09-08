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
use prelude::*;
use ron_loader::loader::TalkLoader;
use trigger::{OnEnableTrigger, OnUseTrigger, TalkTriggerer};

mod builder;
pub mod display;
pub mod errors;
pub mod events;
pub mod prelude;
pub mod raw_talk;
pub mod ron_loader;
pub mod talk;
pub mod talker;
pub mod trigger;

/// The plugin that provides the basics to build and handle dialogues in games.
pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<TalkLoader>()
            .add_asset::<RawTalk>()
            .add_event::<InitTalkRequest>()
            .add_event::<NextActionRequest>()
            .add_event::<JumpToActionRequest>()
            .add_systems(
                Update,
                (
                    init_talk_handler,
                    next_action_handler,
                    jump_action_handler,
                    handle_trigger::<OnUseTrigger>,
                    handle_trigger::<OnEnableTrigger>,
                ),
            );
    }
}

fn init_talk_handler(
    mut init_requests: EventReader<InitTalkRequest>,
    mut talk_comps: Query<(
        &mut Talk,
        &mut CurrentText,
        &mut CurrentActors,
        &mut CurrentNodeKind,
        &mut CurrentChoices,
    )>,
) {
    for ev in init_requests.iter() {
        let talker_entity = talk_comps.get_mut(ev.0);
        if let Err(err) = talker_entity {
            error!("Talk could not be initialized: {}", err);
            continue;
        }

        let (mut talk, mut text, mut ca, mut kind, mut cc) = talker_entity.unwrap();
        talk.start();
        text.0 = talk.text().to_string();
        ca.0 = talk.action_actors();
        kind.0 = talk.node_kind();
        cc.0 = talk.choices();
    }
}

/// Handles `JumpToActionRequest` events by updating the active Talk.
///
/// This function is a Bevy system that listens for `JumpToActionRequest` events.
/// It calls `jump_to` on the active Talk and sends `ActorsEnterEvent` or `ActorsExitEvent` events
/// if the reached action is an enter or exit action, respectively.
fn jump_action_handler(
    mut jump_requests: EventReader<JumpToActionRequest>,
    mut talk_comps: Query<(Entity, &mut Talk)>,
) {
    for ev in jump_requests.iter() {
        if let Ok((_, mut sp)) = talk_comps.get_mut(ev.0) {
            match sp.jump_to(ev.1) {
                Ok(()) => info!("Jumped to action {:?}.", ev.1),
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
    mut next_requests: EventReader<NextActionRequest>,
    mut talk_comps: Query<(
        &mut Talk,
        &mut CurrentText,
        &mut CurrentActors,
        &mut CurrentNodeKind,
        &mut CurrentChoices,
    )>,
) {
    for ev in next_requests.iter() {
        let talker_entity = talk_comps.get_mut(ev.0);
        if let Err(err) = talker_entity {
            error!("Next action could not be set: {}", err);
            continue;
        }

        let (mut talk, mut text, mut ca, mut kind, mut cc) = talker_entity.unwrap();
        match talk.next_action() {
            Ok(()) => {
                text.0 = talk.text().to_string();
                ca.0 = talk.action_actors();
                kind.0 = talk.node_kind();
                cc.0 = talk.choices();
            }
            Err(err) => error!("Next action could not be set: {}", err),
        }
    }
}

/// Handles `OnUseTrigger` and `OnEnableTrigger` events by triggering the associated actions.
fn handle_trigger<T: TalkTriggerer + Component>(query: Query<(&Talk, &T)>) {
    for (_sp, t) in query.iter() {
        t.trigger();
    }
}

#[cfg(test)]
mod tests {

    use crate::prelude::RawAction;

    use super::*;

    /// A minimal Bevy app with the Talks plugin.
    pub fn minimal_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TalksPlugin));
        app
    }

    #[test]
    fn init_talk_handler() {
        let mut app = minimal_app();
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction {
                text: Some("Hello".to_string()),
                ..default()
            }],
        };

        let sp = Talk::build(&raw_sp);
        assert!(sp.is_ok());

        let e = app
            .world
            .spawn(TalkerBundle {
                talk: sp.unwrap(),
                ..default()
            })
            .id();

        app.world.send_event(InitTalkRequest(e));
        app.update();

        let tt = app.world.get::<CurrentText>(e).unwrap();
        assert_eq!(tt.0, "Hello");
    }

    #[test]
    fn next_action_handler() {
        let mut app = minimal_app();
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction { ..default() }, RawAction { id: 2, ..default() }],
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
                RawAction { ..default() },
                RawAction { id: 2, ..default() },
                RawAction { id: 3, ..default() },
            ],
        };

        let sp = Talk::build(&raw_sp);
        assert!(sp.is_ok());

        let e = app.world.spawn(sp.unwrap()).id();

        app.world.send_event(JumpToActionRequest(e, 2.into()));
        app.update();

        let sp_spawned = app.world.get::<Talk>(e).unwrap();

        assert_eq!(sp_spawned.current_node.index(), 2);
    }
}
