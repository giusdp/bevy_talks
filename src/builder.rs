//! The main module of the crate. It contains the Talk struct and its
//! builder.
use bevy::{
    prelude::default,
    utils::{HashMap, HashSet},
};
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex, Graph};

use crate::{
    errors::BuildTalkError,
    prelude::{ActionId, ActorId, RawAction, RawActor, RawTalk, Talk},
    talks::{Choice, TalkNode, TalkNodeKind},
};

/// Builds a `Talk` instance from a `RawTalk` instance.
///
/// This function performs two passes over the `RawTalk` instance: a validation pass and a graph build pass.
/// In the validation pass, it checks that there are no duplicate ids both in actors and actions,
/// that all the actors in the actions are present in the actors list,
/// and that all the `next` fields and `choice.next` fields in the actions point to existing actions.
/// In the graph build pass, it adds all the action nodes to a new `DiGraph`,
/// and then adds edges to the graph to connect the nodes according to the `next` and `choice.next` fields in the actions.
///
/// # Arguments
///
/// * `raw` - A reference to a `RawTalk` instance to build a `Talk` from.
///
/// # Returns
///
/// A `Result` containing the `Talk` instance if the build was successful,
/// or a `TalkError` if there was an error during validation or graph building.
///
/// # Examples
///
/// ```
/// use bevy_talks::prelude::*;
///
/// let raw = RawTalk {
///     script: vec![
///         RawAction {
///             id: 1,
///             text: Some("Action 1".to_string()),
///             actors: vec!["actor1".to_string()],
///             next: Some(2),
///             ..Default::default()
///         },
///         RawAction {
///             id: 2,
///             text: Some("Action 2".to_string()),
///             actors: vec!["actor2".to_string()],
///             ..Default::default()
///         },
///     ],
///     actors: vec![
///         RawActor {
///             id: "actor1".to_string(),
///             name: "Actor 1".to_string(),
///             ..Default::default()
///         },
///         RawActor {
///             id: "actor2".to_string(),
///             name: "Actor 2".to_string(),
///             ..Default::default()
///         },
///     ],
/// };
///
/// let result = Talk::build(&raw);
///
/// assert!(result.is_ok());
/// ```
///
pub(crate) fn build(raw: &RawTalk) -> Result<Talk, BuildTalkError> {
    if raw.script.is_empty() {
        return Ok(Talk { ..default() });
    }

    // # Validation Pass
    // Check that there are no duplicate ids both in actors and actions
    check_duplicate_action_ids(&raw.script)?;
    check_duplicate_actor_ids(&raw.actors)?;

    // Check that all the actors in the actions are present in the actors list
    validate_actors_in_actions(&raw.script, &raw.actors)?;

    // Check all the nexts and choice.next (they should point to existing actions)
    validate_all_nexts(&raw.script)?;

    // # Graph build Pass
    let mut graph: DiGraph<TalkNode, ()> =
        DiGraph::with_capacity(raw.script.len(), raw.script.len());

    // 1. Add all action nodes
    let id_nodeidx_map = add_action_nodes(&mut graph, &raw.script, &raw.actors);

    // 2. Add edges to the graph
    connect_action_nodes(&mut graph, &raw.script, &id_nodeidx_map);

    Ok(Talk {
        graph,
        current_node: NodeIndex::new(0),
        // action_node_map: id_nodeidx_map,
    })
}

/// Connects all the action nodes in the graph based on the `next`  and `choices` fields of each `RawAction` instance.
/// If both fields are None, the next action in the `actions` slice is connected (unless it's the last one).
///
/// When connecting the nodes, the `choices` field of the current node is populated with the `Choice` instances
///
/// # Arguments
///
/// * `graph` - A mutable reference to a `Graph` instance.
/// * `actions` - A slice of `ScriptAction` instances to connect in the graph.
/// * `id_nodeidx_map` - A `HashMap` that maps `ActionId` values to `NodeIndex` values in the graph.
fn connect_action_nodes(
    graph: &mut Graph<TalkNode, ()>,
    actions: &[RawAction],
    id_nodeidx_map: &HashMap<ActionId, NodeIndex>,
) {
    for (i, action) in actions.iter().enumerate() {
        let current_node_idx = id_nodeidx_map.get(&action.id).unwrap();
        if let Some(choices) = &action.choices {
            // Build the choice vector to put in the current node
            let choice_vec = choices
                .iter()
                .map(|c| {
                    let next = id_nodeidx_map.get(&c.next).unwrap();
                    Choice {
                        text: c.text.clone(),
                        next: *next,
                    }
                })
                .collect::<Vec<_>>();
            let current_node = graph.node_weight_mut(*current_node_idx).unwrap();
            current_node.choices = Some(choice_vec);
            for choice in choices {
                let choice_node_idx = id_nodeidx_map.get(&choice.next).unwrap();
                graph.add_edge(*current_node_idx, *choice_node_idx, ());
            }
        } else if let Some(next_id) = &action.next {
            let next_node_idx = id_nodeidx_map.get(next_id).unwrap();
            graph.add_edge(*current_node_idx, *next_node_idx, ());
        } else if i < actions.len() - 1 {
            let next_node_idx = id_nodeidx_map.get(&actions[i + 1].id).unwrap();
            graph.add_edge(*current_node_idx, *next_node_idx, ());
        }
    }
}

/// Adds all the action nodes to a graph.
///
/// # Arguments
///
/// * `graph` - A mutable reference to a `Graph` instance.
/// * `actions` - A slice of `ScriptAction` instances.
/// * `actors` - A slice of `Actor` instances.
///
/// # Returns
///
/// A `HashMap` that maps `ActionId` values to `NodeIndex` values in the graph.
fn add_action_nodes(
    graph: &mut Graph<TalkNode, ()>,
    actions: &[RawAction],
    actors: &[RawActor],
) -> HashMap<ActionId, NodeIndex> {
    let mut id_nodeidx_map = HashMap::new();

    for action in actions {
        let actors = retrieve_actors(&action.actors, actors)
            .into_iter()
            .map(|a| a.into())
            .collect();

        // Choices will be populated later, we first need to have all the nodes in place
        let mut node = TalkNode {
            kind: action.kind.clone(),
            choices: None,
            text: action.text.clone(),
            actors,
        };

        // If the raw action has choices, hardwire the kind to Choice
        if action.choices.is_some() && node.kind != TalkNodeKind::Choice {
            node.kind = TalkNodeKind::Choice;
        }

        let node_idx = graph.add_node(node);

        id_nodeidx_map.insert(action.id, node_idx);
    }

    id_nodeidx_map
}

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

/// Validate that all the actors in the actions are present in the actors list.
///
/// # Arguments
///
/// * `actions` - A slice of `ScriptAction` structs to validate.
/// * `actors` - A slice of `Actor` structs representing the available actors.
///
/// # Errors
///
/// Returns a `TalkError::InvalidActor` error if any of the actors in any of the actions are not present in the actors list.
fn validate_actors_in_actions(
    actions: &[RawAction],
    actors: &[RawActor],
) -> Result<(), BuildTalkError> {
    for action in actions {
        for actor_key in action.actors.iter() {
            if !actors.iter().any(|a| a.id == *actor_key) {
                return Err(BuildTalkError::InvalidActor(
                    action.id,
                    actor_key.to_string(),
                ));
            }
        }
    }
    Ok(())
}

/// Check that there are no duplicate `id` values in the given `actions` vector.
///
/// # Arguments
///
/// * `actions` - A slice of `ScriptAction` structs to check for duplicate `id` values.
///
/// # Errors
///
/// Returns a `TalkError::DuplicateActionId` error if any `id` value appears more than once in the `actions` vector.
fn check_duplicate_action_ids(actions: &[RawAction]) -> Result<(), BuildTalkError> {
    let mut seen_ids = HashSet::new();
    for action in actions {
        if !seen_ids.insert(action.id) {
            return Err(BuildTalkError::DuplicateActionId(action.id));
        }
    }
    Ok(())
}

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

/// Check if all `next` fields and `Choice` `next` fields in a `Vec<ScriptAction>` point to real actions.
/// If the action has choices, the `next` field is not checked.
///
/// # Arguments
///
/// * `actions` - A slice of `ScriptAction` structs representing the available actions.
///
/// # Errors
///
/// Returns a `TalkError::InvalidNextAction` error if any of the `next` fields or `Choice` `next` fields in the `ScriptAction`s do not point to real actions.
fn validate_all_nexts(actions: &[RawAction]) -> Result<(), BuildTalkError> {
    let id_set = actions.iter().map(|a| a.id).collect::<HashSet<ActionId>>();
    for action in actions {
        if let Some(choices) = &action.choices {
            for choice in choices {
                if !id_set.contains(&choice.next) {
                    return Err(BuildTalkError::InvalidNextAction(action.id, choice.next));
                }
            }
        } else if let Some(next_id) = &action.next {
            if !id_set.contains(next_id) {
                return Err(BuildTalkError::InvalidNextAction(action.id, *next_id));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::prelude::RawChoice;

    use super::*;
    use bevy::prelude::default;

    #[test]
    fn build_empty_talk() {
        let res = build(&RawTalk::default());
        assert!(res.is_ok());
        let sp = res.unwrap();
        assert_eq!(sp.graph.node_count(), 0);
        assert_eq!(sp.current_node.index(), 0);
    }

    #[test]
    fn simple_build() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction { ..default() }],
        };

        let res = build(&raw_sp);
        assert!(res.is_ok());

        let sp = res.unwrap();
        assert_eq!(sp.graph.node_count(), 1);
        assert_eq!(sp.graph.edge_count(), 0);
        // assert_eq!(sp.action_node_map.len(), 1);
        assert_eq!(sp.current_node.index(), 0);
    }

    #[test]
    fn new_with_self_loop() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction {
                id: 1,
                next: Some(1),
                ..default()
            }],
        };

        let res = build(&raw_sp);

        assert!(res.is_ok());
        let sp = res.unwrap();
        assert_eq!(sp.graph.node_count(), 1);
        assert_eq!(sp.graph.edge_count(), 1);
        // assert_eq!(sp.action_node_map.len(), 1);
        assert_eq!(sp.current_node.index(), 0);
    }

    #[test]
    fn new_with_two_actor_action_nodes() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![
                RawAction {
                    id: 1,
                    next: Some(2),
                    ..default()
                },
                RawAction { id: 2, ..default() },
            ],
        };

        let res = build(&raw_sp);

        assert!(res.is_ok());
        let sp = res.unwrap();

        assert_eq!(sp.graph.node_count(), 2);
        assert_eq!(sp.graph.edge_count(), 1);
    }

    #[test]
    fn new_with_branching() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![
                RawAction {
                    choices: Some(vec![
                        RawChoice {
                            text: "Choice 1".to_string(),
                            next: 2,
                        },
                        RawChoice {
                            text: "Choice 2".to_string(),
                            next: 3,
                        },
                    ]),
                    ..default()
                },
                RawAction {
                    id: 2,
                    text: Some("Hello".to_string()),
                    ..default()
                },
                RawAction { id: 3, ..default() },
            ],
        };

        let res = build(&raw_sp);
        assert!(res.is_ok());
        let sp = res.unwrap();

        assert_eq!(sp.graph.node_count(), 3);
        assert_eq!(sp.graph.edge_count(), 3);
        assert_eq!(sp.current_node, NodeIndex::new(0));
    }

    #[test]
    fn new_with_actors() {
        let actors = vec![
            RawActor {
                id: "bob".to_owned(),
                ..default()
            },
            RawActor {
                id: "alice".to_owned(),
                ..default()
            },
        ];
        let raw_sp = RawTalk {
            actors,
            script: vec![
                RawAction {
                    id: 1,
                    text: Some("Hello".to_string()),
                    actors: vec!["bob".to_string()],
                    next: Some(2),
                    ..default()
                },
                RawAction {
                    id: 2,
                    text: Some("Whatup".to_string()),
                    actors: vec!["alice".to_string()],
                    ..default()
                },
            ],
        };

        let res = build(&raw_sp);

        assert!(res.is_ok());
        let sp = res.unwrap();

        assert_eq!(sp.graph.node_count(), 2);
        assert_eq!(sp.graph.edge_count(), 1);
        assert_eq!(sp.current_node, NodeIndex::new(0));
    }

    #[test]
    fn build_missing_actor() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction {
                actors: vec!["bob".to_string()],
                ..default()
            }],
        };

        let res = build(&raw_sp).err();

        assert_eq!(
            res,
            Some(BuildTalkError::InvalidActor(0, String::from("bob")))
        );
    }

    #[test]
    fn build_actor_mismath() {
        let actor_vec = vec![RawActor {
            id: "bob".to_string(),
            ..default()
        }];
        let raw_sp = RawTalk {
            actors: actor_vec,
            script: vec![RawAction {
                actors: vec!["alice".to_string()],
                ..default()
            }],
        };
        let res = build(&raw_sp).err();
        assert_eq!(
            res,
            Some(BuildTalkError::InvalidActor(0, String::from("alice")))
        );
    }

    #[test]
    fn build_with_invalid_next_action() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction {
                next: Some(2),
                ..default()
            }],
        };
        let res = build(&raw_sp).err();
        assert_eq!(res, Some(BuildTalkError::InvalidNextAction(0, 2)));
    }

    #[test]
    fn build_not_found_in_choice() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction {
                choices: Some(vec![RawChoice {
                    next: 2,
                    text: default(),
                }]),
                ..default()
            }],
        };
        let res = build(&raw_sp).err();
        assert_eq!(res, Some(BuildTalkError::InvalidNextAction(0, 2)));
    }

    #[test]
    fn build_duplicate_id() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![
                RawAction { id: 1, ..default() },
                RawAction { id: 1, ..default() },
            ],
        };
        let res = build(&raw_sp).err();
        assert_eq!(res, Some(BuildTalkError::DuplicateActionId(1)));
    }
}
