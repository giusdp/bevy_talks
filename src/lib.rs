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

use aery::prelude::*;
use bevy::prelude::*;
use prelude::*;
use ron_loader::loader::TalksLoader;
// use trigger::{OnEnableTrigger, OnUseTrigger, TalkTriggerer};

pub mod builder;
pub mod errors;
pub mod events;
pub mod prelude;
pub mod raw_talk;
pub mod ron_loader;
pub mod talk;
pub mod talker;
pub mod talkv2;

/// The plugin that provides the basics to build and handle dialogues in games.
///
/// # Note
/// If you are using [Aery](https://crates.io/crates/aery), add it to the App before this plugin, or just add this plugin.
/// This plugin will add Aery if it's not in the app, since it is a unique plugin, having multiple will panic.
pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<Aery>() {
            app.add_plugins(Aery);
        }
        app.register_asset_loader(TalksLoader)
            .init_asset::<RawTalk>()
            .add_event::<InitTalkRequest>()
            .add_event::<NextActionRequest>()
            .add_event::<JumpToActionRequest>()
            .add_systems(
                Update,
                (init_talk_handler, next_action_handler, jump_action_handler),
            );
    }
}

/// The handler system for `InitTalkRequest` events.
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
    for ev in init_requests.read() {
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
        debug!("Talk initialized.");
    }
}

/// Handles `JumpToActionRequest` events by updating the active Talk.
///
/// This function is a Bevy system that listens for `JumpToActionRequest` events.
/// It calls `jump_to` on the active Talk and sends `ActorsEnterEvent` or `ActorsExitEvent` events
/// if the reached action is an enter or exit action, respectively.
fn jump_action_handler(
    mut jump_requests: EventReader<JumpToActionRequest>,
    mut talk_comps: Query<(
        &mut Talk,
        &mut CurrentText,
        &mut CurrentActors,
        &mut CurrentNodeKind,
        &mut CurrentChoices,
    )>,
) {
    for ev in jump_requests.read() {
        let talker_entity = talk_comps.get_mut(ev.0);
        if let Err(err) = talker_entity {
            error!("Jump action could not be done: {}", err);
            continue;
        }
        let (mut talk, mut text, mut ca, mut kind, mut cc) = talker_entity.unwrap();
        match talk.jump_to(ev.1) {
            Ok(()) => {
                text.0 = talk.text().to_string();
                ca.0 = talk.action_actors();
                kind.0 = talk.node_kind();
                cc.0 = talk.choices();
                debug!("Jumped to action {:?}.", ev.1)
            }
            Err(err) => error!("Jump action could not be set: {}", err),
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
    for ev in next_requests.read() {
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
                debug!("Next action set.");
            }
            Err(err) => error!("Next action could not be set: {}", err),
        }
    }
}

#[cfg(test)]
mod tests {

    use indexmap::IndexMap;

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
        let mut script = IndexMap::<usize, RawAction>::with_capacity(1);
        script.insert(0, RawAction { ..default() });
        let talk = RawTalk {
            script,
            ..default()
        };

        let sp = Talk::build(&talk);
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

        let mut script = IndexMap::<usize, RawAction>::with_capacity(2);
        script.insert(0, RawAction { ..default() });
        script.insert(2, RawAction { ..default() });
        let mut talk = RawTalk::default();
        talk.script = script;

        let sp = Talk::build(&talk);
        assert!(sp.is_ok());

        let e = app
            .world
            .spawn(TalkerBundle {
                talk: sp.unwrap(),
                ..default()
            })
            .id();

        app.world.send_event(NextActionRequest(e));
        app.update();

        let sp_spawned = app.world.get::<Talk>(e).unwrap();

        assert_eq!(sp_spawned.current_node.index(), 1);
    }

    #[test]
    fn jump_action_handler() {
        let mut app = minimal_app();
        let mut script = IndexMap::<usize, RawAction>::with_capacity(3);
        script.insert(0, RawAction { ..default() });
        script.insert(2, RawAction { ..default() });
        script.insert(3, RawAction { ..default() });
        let mut talk = RawTalk::default();
        talk.script = script;

        let sp = Talk::build(&talk);
        assert!(sp.is_ok());

        let e = app
            .world
            .spawn(TalkerBundle {
                talk: sp.unwrap(),
                ..default()
            })
            .id();

        app.world.send_event(JumpToActionRequest(e, 2.into()));
        app.update();

        let sp_spawned = app.world.get::<Talk>(e).unwrap();

        assert_eq!(sp_spawned.current_node.index(), 2);
    }
}
