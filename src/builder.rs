//! The main module of the crate. It contains the Talk struct and its
//! builder.
use bevy::{
    prelude::default,
    utils::{HashMap, HashSet},
};
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex, Graph};

use crate::prelude::{
    ActionId, ActorId, BuildTalkError, Choice, RawAction, RawActor, RawTalk, Talk, TalkNode,
    TalkNodeKind,
};

// /// Builds the Dialogue Graph from a `RawTalk` asset.
// ///
// /// This function performs two passes over the `RawTalk` asset: a validation pass and a graph build pass.
// /// In the validation pass, it checks that there are no duplicate ids both in actors and actions,
// /// that all the actors in the actions are present in the actors list,
// /// and that all the `next` fields and `choice.next` fields in the actions point to existing actions.
// /// In the graph build pass, it adds all the action nodes to a new `DiGraph`,
// /// and then adds edges to the graph to connect the nodes according to the `next` and `choice.next` fields in the actions.
// ///
// /// # Arguments
// ///
// /// * `raw` - A reference to a `RawTalk` asset to build the graph from.
// ///
// /// # Returns
// ///
// /// A `Result` containing the graph root `Entity` if the build was successful,
// /// or a `TalkError` if there was an error during validation or graph building.
// ///
// /// # Examples
// ///
// /// ```
// /// use bevy_talks::prelude::*;
// ///
// /// let raw = RawTalk {
// ///     script: vec![
// ///         RawAction {
// ///             id: 1,
// ///             text: Some("Action 1".to_string()),
// ///             actors: vec!["actor1".to_string()],
// ///             next: Some(2),
// ///             ..Default::default()
// ///         },
// ///         RawAction {
// ///             id: 2,
// ///             text: Some("Action 2".to_string()),
// ///             actors: vec!["actor2".to_string()],
// ///             ..Default::default()
// ///         },
// ///     ],
// ///     actors: vec![
// ///         RawActor {
// ///             id: "actor1".to_string(),
// ///             name: "Actor 1".to_string(),
// ///             ..Default::default()
// ///         },
// ///         RawActor {
// ///             id: "actor2".to_string(),
// ///             name: "Actor 2".to_string(),
// ///             ..Default::default()
// ///         },
// ///     ],
// /// };
// ///
// /// let result = Talk::build(&raw);
// ///
// /// assert!(result.is_ok());
// /// ```
// ///
// pub(crate) fn build(raw: &RawTalk) -> Result<Talk, BuildTalkError> {
//     if raw.script.is_empty() {
//         return Ok(Talk { ..default() });
//     }

//     // # Validation Pass
//     // Check that there are no duplicate ids both in actors and actions
//     // check_duplicate_action_ids(&raw.script)?;
//     // check_duplicate_actor_ids(&raw.actors)?;

//     // // Check that all the actors in the actions are present in the actors list
//     // validate_actors_in_actions(&raw.script, &raw.actors)?;

//     // // Check all the nexts and choice.next (they should point to existing actions)
//     // validate_all_nexts(&raw.script)?;

//     // # Graph build Pass
//     let graph: DiGraph<TalkNode, ()> = DiGraph::with_capacity(raw.script.len(), raw.script.len());

//     // 1. Add all action nodes
//     // let id_nodeidx_map = add_action_nodes(&mut graph, &raw.script, &raw.actors);

//     // 2. Add edges to the graph
//     // connect_action_nodes(&mut graph, &raw.script, &id_nodeidx_map);

//     Ok(Talk {
//         graph,
//         current_node: NodeIndex::new(0),
//         start_node: NodeIndex::new(0),
//         // action_node_map: id_nodeidx_map,
//     })
// }

/// Retrieve the `Actor`s corresponding to the given actor IDs.
///
/// # Arguments
///
/// * `actor_ids` - A slice of actor IDs to retrieve.
/// * `actors` - A slice of `Actor` instances to search for the given actor IDs.
///
/// # Returns
///
/// A vector of `Actor` instances corresponding to the given actor IDs.
fn retrieve_actors(actor_ids: &[ActorId], actors: &[RawActor]) -> Vec<RawActor> {
    actors
        .iter()
        .filter(|actor| actor_ids.contains(&actor.id))
        .cloned()
        .collect()
}

// /// Validate that all the actors in the actions are present in the actors list.
// ///
// /// # Arguments
// ///
// /// * `actions` - A slice of `RawAction` structs to validate.
// /// * `actors` - A slice of `Actor` structs representing the available actors.
// ///
// /// # Errors
// ///
// /// Returns a `TalkError::InvalidActor` error if any of the actors in any of the actions are not present in the actors list.
// fn validate_actors_in_actions(
//     actions: &[RawAction],
//     actors: &[RawActor],
// ) -> Result<(), BuildTalkError> {
//     for action in actions {
//         for actor_key in action.actors.iter() {
//             if !actors.iter().any(|a| a.id == *actor_key) {
//                 return Err(BuildTalkError::InvalidActor(
//                     action.id,
//                     actor_key.to_string(),
//                 ));
//             }
//         }
//     }
//     Ok(())
// }

/// Check that there are no duplicate `actor_id` values in the given `actors` vector.
///
/// # Arguments
///
/// * `actors` - A slice of `Actor` structs to check for duplicate `actor_id` values.
///
/// # Errors
///
/// Returns a `TalkError::DuplicateActorId` error if any `actor_id` value appears more than once in the `actors` vector.
fn check_duplicate_actor_ids(actors: &[RawActor]) -> Result<(), BuildTalkError> {
    let mut seen_ids = HashSet::new();
    for actor in actors {
        if !seen_ids.insert(&actor.id) {
            return Err(BuildTalkError::DuplicateActorId(actor.id.clone()));
        }
    }
    Ok(())
}

// #[cfg(test)]
// mod tests {

//     use crate::prelude::RawChoice;

//     use super::*;
//     use bevy::prelude::default;
//     #[test]
//     fn new_with_two_actor_action_nodes() {
//         let raw_sp = RawTalk {
//             actors: default(),
//             script: vec![
//                 RawAction {
//                     id: 1,
//                     next: Some(2),
//                     ..default()
//                 },
//                 RawAction { id: 2, ..default() },
//             ],
//         };

//         let res = build(&raw_sp);

//         assert!(res.is_ok());
//         let sp = res.unwrap();

//         assert_eq!(sp.graph.node_count(), 2);
//         assert_eq!(sp.graph.edge_count(), 1);
//     }

//     #[test]
//     fn new_with_actors() {
//         let actors = vec![
//             RawActor {
//                 id: "bob".to_owned(),
//                 ..default()
//             },
//             RawActor {
//                 id: "alice".to_owned(),
//                 ..default()
//             },
//         ];
//         let raw_sp = RawTalk {
//             actors,
//             script: vec![
//                 RawAction {
//                     id: 1,
//                     text: "Hello".to_string(),
//                     actors: vec!["bob".to_string()],
//                     next: Some(2),
//                     ..default()
//                 },
//                 RawAction {
//                     id: 2,
//                     text: "Whatup".to_string(),
//                     actors: vec!["alice".to_string()],
//                     ..default()
//                 },
//             ],
//         };

//         let res = build(&raw_sp);

//         assert!(res.is_ok());
//         let sp = res.unwrap();

//         assert_eq!(sp.graph.node_count(), 2);
//         assert_eq!(sp.graph.edge_count(), 1);
//         assert_eq!(sp.current_node, NodeIndex::new(0));
//     }

//     #[test]
//     fn build_missing_actor() {
//         let raw_sp = RawTalk {
//             actors: default(),
//             script: vec![RawAction {
//                 actors: vec!["bob".to_string()],
//                 ..default()
//             }],
//         };

//         let res = build(&raw_sp).err();

//         assert_eq!(
//             res,
//             Some(BuildTalkError::InvalidActor(0, String::from("bob")))
//         );
//     }

//     #[test]
//     fn build_actor_mismath() {
//         let actor_vec = vec![RawActor {
//             id: "bob".to_string(),
//             ..default()
//         }];
//         let raw_sp = RawTalk {
//             actors: actor_vec,
//             script: vec![RawAction {
//                 actors: vec!["alice".to_string()],
//                 ..default()
//             }],
//         };
//         let res = build(&raw_sp).err();
//         assert_eq!(
//             res,
//             Some(BuildTalkError::InvalidActor(0, String::from("alice")))
//         );
//     }

//     #[test]
//     fn build_with_invalid_next_action() {
//         let raw_sp = RawTalk {
//             actors: default(),
//             script: vec![RawAction {
//                 next: Some(2),
//                 ..default()
//             }],
//         };
//         let res = build(&raw_sp).err();
//         assert_eq!(res, Some(BuildTalkError::InvalidNextAction(0, 2)));
//     }

//     #[test]
//     fn build_not_found_in_choice() {
//         let raw_sp = RawTalk {
//             actors: default(),
//             script: vec![RawAction {
//                 choices: vec![RawChoice {
//                     next: 2,
//                     text: default(),
//                 }],
//                 ..default()
//             }],
//         };
//         let res = build(&raw_sp).err();
//         assert_eq!(res, Some(BuildTalkError::InvalidNextAction(0, 2)));
//     }

//     #[test]
//     fn build_duplicate_id() {
//         let raw_sp = RawTalk {
//             actors: default(),
//             script: vec![
//                 RawAction { id: 1, ..default() },
//                 RawAction { id: 1, ..default() },
//             ],
//         };
//         let res = build(&raw_sp).err();
//         assert_eq!(res, Some(BuildTalkError::DuplicateActionId(1)));
//     }
// }
