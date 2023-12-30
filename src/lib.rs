#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![forbid(unsafe_code)]
#![warn(clippy::doc_markdown)]
// Often exceeded by queries
#![allow(clippy::type_complexity)]
// Unhelpful for systems
#![allow(clippy::too_many_arguments)]

//! [`bevy_talks`] is a Bevy plugin that provides the basics to build and handle dialogues in games.

use aery::{prelude::*, tuple_traits::RelationEntries};
use bevy::prelude::*;
use prelude::*;
use ron_loader::loader::TalksLoader;

pub mod actors;
pub mod builder;
pub mod errors;
pub mod events;
pub mod prelude;
pub mod ron_loader;
pub mod talk;
// pub mod talker;

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
            .init_asset::<TalkData>()
            .add_event::<InitTalkRequest>()
            .add_event::<NextActionRequest>()
            .add_event::<JumpToActionRequest>()
            .add_systems(Update, next_handler.pipe(error_handler));
    }
}

/// Logs errors from the other systems.
fn error_handler(In(result): In<Result<(), NextActionError>>) {
    match result {
        Ok(_) => (),
        Err(err) => error!("Error: {err}"),
    }
}

// /// The handler system for `InitTalkRequest` events.
// fn init_talk_handler(
//     mut init_requests: EventReader<InitTalkRequest>,
//     mut talk_comps: Query<(
//         &mut Talk,
//         &mut CurrentText,
//         &mut CurrentActors,
//         &mut CurrentNodeKind,
//         &mut CurrentChoices,
//     )>,
// ) {
//     for ev in init_requests.read() {
//         let talker_entity = talk_comps.get_mut(ev.0);
//         if let Err(err) = talker_entity {
//             error!("Talk could not be initialized: {}", err);
//             continue;
//         }

//         let (mut talk, mut text, mut ca, mut kind, mut cc) = talker_entity.unwrap();
//         talk.start();
//         text.0 = talk.text().to_string();
//         ca.0 = talk.action_actors();
//         kind.0 = talk.node_kind();
//         cc.0 = talk.choices();
//         debug!("Talk initialized.");
//     }
// }

// /// Handles `JumpToActionRequest` events by updating the active Talk.
// ///
// /// This function is a Bevy system that listens for `JumpToActionRequest` events.
// /// It calls `jump_to` on the active Talk and sends `ActorsEnterEvent` or `ActorsExitEvent` events
// /// if the reached action is an enter or exit action, respectively.
// fn jump_action_handler(
//     mut jump_requests: EventReader<JumpToActionRequest>,
//     mut talk_comps: Query<(
//         &mut Talk,
//         &mut CurrentText,
//         &mut CurrentActors,
//         &mut CurrentNodeKind,
//         &mut CurrentChoices,
//     )>,
// ) {
//     for ev in jump_requests.read() {
//         let talker_entity = talk_comps.get_mut(ev.0);
//         if let Err(err) = talker_entity {
//             error!("Jump action could not be done: {}", err);
//             continue;
//         }
//         let (mut talk, mut text, mut ca, mut kind, mut cc) = talker_entity.unwrap();
//         match talk.jump_to(ev.1) {
//             Ok(()) => {
//                 text.0 = talk.text().to_string();
//                 ca.0 = talk.action_actors();
//                 kind.0 = talk.node_kind();
//                 cc.0 = talk.choices();
//                 debug!("Jumped to action {:?}.", ev.1)
//             }
//             Err(err) => error!("Jump action could not be set: {}", err),
//         }
//     }
// }

/// Handles `NextActionRequest` events by advancing the active Talk to the next action.
///
/// This function is a Bevy system that listens for `NextActionRequest` events.
/// It calls `next_action` on the active Talk and sends `ActorsEnterEvent` or `ActorsExitEvent` events
/// if the reached action is an enter or exit action, respectively.
fn next_handler(
    mut commands: Commands,
    mut next_requests: EventReader<NextActionRequest>,
    mut talks: Query<&mut Talk>,
    current_nodes: Query<(Entity, &Parent, Relations<FollowedBy>), With<CurrentNode>>,
    node_kind_comps: Query<&NodeKind>,
    talk_comps: Query<&TalkText>,
) -> Result<(), NextActionError> {
    let maybe_event = next_requests.read().next();
    if maybe_event.is_none() {
        return Ok(());
    }
    let event = maybe_event.unwrap();

    for (node_entity, talk_parent, edges) in &current_nodes {
        let talk_entity = talk_parent.get();
        // if this is the talk we want to advance
        if talk_entity == event.0 {
            let targets = edges.targets(FollowedBy);
            match targets.len() {
                0 => return Err(NextActionError::NoNextAction),
                1 => {
                    // move the current node component to the next one
                    let next_node = move_current_node(&mut commands, node_entity, targets);
                    let talk = talks.get_mut(talk_entity).unwrap();
                    let next_kind = node_kind_comps.get(next_node).unwrap();
                    update_talk_with_next_node(next_kind, &talk_comps, next_node, talk);

                    return Ok(());
                }
                2.. => return Err(NextActionError::ChoicesNotHandled),
            }
        }
    }

    Err(NextActionError::NoTalk)
}

/// Moves the current node component from the current node to the next one.
fn move_current_node(
    commands: &mut Commands<'_, '_>,
    node_entity: Entity,
    targets: &[Entity],
) -> Entity {
    commands.entity(node_entity).remove::<CurrentNode>();
    let next_node = targets[0];
    commands.entity(next_node).insert(CurrentNode);
    next_node
}

/// Updates the current text, actors, node kind and choices of the active Talk based on the next node kind.
fn update_talk_with_next_node(
    next_kind: &NodeKind,
    talk_comps: &Query<'_, '_, &TalkText>,
    next_node: Entity,
    mut talk: Mut<'_, Talk>,
) {
    match next_kind {
        NodeKind::Talk => {
            let next_text = talk_comps.get(next_node).unwrap().0.clone();
            talk.current_text = next_text;
            talk.current_kind = NodeKind::Talk;
        }
        NodeKind::Choice => todo!(),
        NodeKind::Join => {
            talk.current_text = "".to_string();
            talk.current_kind = NodeKind::Join
        }
        NodeKind::Leave => {
            talk.current_text = "".to_string();
            talk.current_kind = NodeKind::Leave
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::prelude::Action;
    use bevy::ecs::system::Command;
    use indexmap::indexmap;

    use super::*;

    /// A minimal Bevy app with the Talks plugin.
    pub fn minimal_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TalksPlugin));
        app
    }

    //     #[test]
    //     fn init_talk_handler() {
    //         let mut app = minimal_app();
    //         let mut script = IndexMap::<usize, RawAction>::with_capacity(1);
    //         script.insert(0, RawAction { ..default() });
    //         let talk = RawTalk {
    //             script,
    //             ..default()
    //         };

    //         let sp = Talk::build(&talk);
    //         assert!(sp.is_ok());

    //         let e = app
    //             .world
    //             .spawn(TalkerBundle {
    //                 talk: sp.unwrap(),
    //                 ..default()
    //             })
    //             .id();

    //         app.world.send_event(InitTalkRequest(e));
    //         app.update();

    //         let tt = app.world.get::<CurrentText>(e).unwrap();
    //         assert_eq!(tt.0, "Hello");
    //     }

    #[test]
    fn test_next_handler_with_talk_nodes() {
        let mut app = minimal_app();

        let script = indexmap! {
            0 => Action { text: "Hello".to_string(), next: Some(2), ..default() },
            2 => Action { text: "Hello 2".to_string(), ..default() },
        };
        let mut talk_asset = TalkData::default();
        talk_asset.script = script;

        let builder = TalkBuilder::default().into_builder(&talk_asset);

        builder.build().apply(&mut app.world);
        let (e, t) = app.world.query::<(Entity, &Talk)>().single(&app.world);
        assert_eq!(t.current_text, "".to_string());
        assert_eq!(t.current_kind, NodeKind::Talk);

        app.world.send_event(NextActionRequest(e));
        app.update();
        app.update();

        // let sp_spawned = app.world.get::<Talk>(e).unwrap();
        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "Hello".to_string());
        assert_eq!(t.current_kind, NodeKind::Talk);

        app.world.send_event(NextActionRequest(e));
        app.update();
        app.update();

        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "Hello 2".to_string());
        assert_eq!(t.current_kind, NodeKind::Talk);
    }

    #[test]
    fn test_next_handler_with_join_and_leave_nodes() {
        let mut app = minimal_app();

        let script = indexmap! {
            0 => Action { kind: NodeKind::Join, next: Some(1), ..default() },
            1 => Action { text: "Hello".to_string(), next: Some(2), ..default() },
            2 => Action { kind: NodeKind::Leave, ..default() },
        };

        let mut talk_asset = TalkData::default();
        talk_asset.script = script;

        let builder = TalkBuilder::default().into_builder(&talk_asset);

        builder.build().apply(&mut app.world);
        let (e, t) = app.world.query::<(Entity, &Talk)>().single(&app.world);
        assert_eq!(t.current_text, "".to_string());
        assert_eq!(t.current_kind, NodeKind::Talk);

        app.world.send_event(NextActionRequest(e));
        app.update();
        app.update();

        // let sp_spawned = app.world.get::<Talk>(e).unwrap();
        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "".to_string());
        assert_eq!(t.current_kind, NodeKind::Join);

        app.world.send_event(NextActionRequest(e));
        app.update();
        app.update();

        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "Hello".to_string());
        assert_eq!(t.current_kind, NodeKind::Talk);

        app.world.send_event(NextActionRequest(e));
        app.update();
        app.update();

        let t = app.world.query::<&Talk>().single(&app.world);
        assert_eq!(t.current_text, "".to_string());
        assert_eq!(t.current_kind, NodeKind::Leave);
    }

    //     #[test]
    //     fn jump_action_handler() {
    //         let mut app = minimal_app();
    //         let mut script = IndexMap::<usize, RawAction>::with_capacity(3);
    //         script.insert(0, RawAction { ..default() });
    //         script.insert(2, RawAction { ..default() });
    //         script.insert(3, RawAction { ..default() });
    //         let mut talk = RawTalk::default();
    //         talk.script = script;

    //         let sp = Talk::build(&talk);
    //         assert!(sp.is_ok());

    //         let e = app
    //             .world
    //             .spawn(TalkerBundle {
    //                 talk: sp.unwrap(),
    //                 ..default()
    //             })
    //             .id();

    //         app.world.send_event(JumpToActionRequest(e, 2.into()));
    //         app.update();

    //         let sp_spawned = app.world.get::<Talk>(e).unwrap();

    //         assert_eq!(sp_spawned.current_node.index(), 2);
    //     }
}
