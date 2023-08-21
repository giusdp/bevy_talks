//! The main module of the crate. It contains the Screenplay struct and its
//! builder.
use bevy::{
    prelude::{default, Assets, Handle},
    reflect::{Reflect, TypeUuid},
    utils::{HashMap, HashSet},
};
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex, Graph};
use serde::Deserialize;

use crate::prelude::{
    ActionId, ActionKind, ActionNode, Actor, ActorId, Screenplay, ScreenplayError, ScriptAction,
};

/// A struct that represents a raw screenplay (as from the json format).
///
/// It contains a list of actors that appear in the screenplay, and a list of actions that make up the screenplay.
#[derive(Debug, Deserialize, Clone, Reflect, TypeUuid)]
#[uuid = "413be529-bfeb-8c5b-9db0-4b8b380a2c47"]
#[reflect_value]
pub struct RawScreenplay {
    /// The list of actors that appear in the screenplay.
    pub actors: Vec<Actor>,
    /// The list of actions that make up the screenplay.
    pub script: Vec<ScriptAction>,
}

/// The [`ScreenplayBuilder`] is used to construct a [`Screenplay`].
/// A [`RawScreenplay`] can be used to build the Screenplay component.
#[derive(Default)]
pub struct ScreenplayBuilder {
    /// The RawScreenplay to be used to build the screenplay.
    raw_sp: Option<Handle<RawScreenplay>>,
}

impl ScreenplayBuilder {
    /// Creates a new `ScreenplayBuilder` instance with default values.
    pub fn new() -> ScreenplayBuilder {
        // Set the minimally required fields of Foo.
        ScreenplayBuilder { ..default() }
    }

    /// Set the [`RawScreenplay`] to be used to build the screenplay.
    /// If there are other action nodes defined, they will be appended at the end.
    pub fn with_raw_screenplay(mut self, sp: Handle<RawScreenplay>) -> ScreenplayBuilder {
        self.raw_sp = Some(sp);
        self
    }

    /// Build the screenplay.
    pub fn build(
        self,
        raw_sp_assets: &Assets<RawScreenplay>,
    ) -> Result<Screenplay, ScreenplayError> {
        let mut sp = Screenplay::default();

        // 1. Build from raw if present
        if let Some(raw_handle) = self.raw_sp {
            let raw = raw_sp_assets
                .get(&raw_handle)
                .ok_or(ScreenplayError::RawScreenplayNotLoaded)?;

            sp = Self::raw_build(raw)?;
        }

        Ok(sp)
    }

    /// Builds a `Screenplay` instance from a `RawScreenplay` instance.
    ///
    /// This function performs two passes over the `RawScreenplay` instance: a validation pass and a graph build pass.
    /// In the validation pass, it checks that there are no duplicate ids both in actors and actions,
    /// that all the actors in the actions are present in the actors list,
    /// and that all the `next` fields and `choice.next` fields in the actions point to existing actions.
    /// In the graph build pass, it adds all the action nodes to a new `DiGraph`,
    /// and then adds edges to the graph to connect the nodes according to the `next` and `choice.next` fields in the actions.
    ///
    /// # Arguments
    ///
    /// * `raw` - A reference to a `RawScreenplay` instance to build a `Screenplay` from.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Screenplay` instance if the build was successful,
    /// or a `ScreenplayError` if there was an error during validation or graph building.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_talks::prelude::{Actor, RawScreenplay, ScriptAction, Screenplay, ScreenplayBuilder};
    ///
    /// let raw = RawScreenplay {
    ///     script: vec![
    ///         ScriptAction {
    ///             id: 1,
    ///             text: Some("Action 1".to_string()),
    ///             actors: vec!["actor1".to_string()],
    ///             next: Some(2),
    ///             ..Default::default()
    ///         },
    ///         ScriptAction {
    ///             id: 2,
    ///             text: Some("Action 2".to_string()),
    ///             actors: vec!["actor2".to_string()],
    ///             ..Default::default()
    ///         },
    ///     ],
    ///     actors: vec![
    ///         Actor {
    ///             id: "actor1".to_string(),
    ///             name: "Actor 1".to_string(),
    ///             ..Default::default()
    ///         },
    ///         Actor {
    ///             id: "actor2".to_string(),
    ///             name: "Actor 2".to_string(),
    ///             ..Default::default()
    ///         },
    ///     ],
    /// };
    ///
    /// let result = ScreenplayBuilder::raw_build(&raw);
    ///
    /// assert!(result.is_ok());
    /// ```
    ///
    pub fn raw_build(raw: &RawScreenplay) -> Result<Screenplay, ScreenplayError> {
        if raw.script.is_empty() {
            return Ok(Screenplay { ..default() });
        }

        let mut graph: DiGraph<ActionNode, ()> =
            DiGraph::with_capacity(raw.script.len(), raw.script.len());

        // # Validation Pass
        // Check that there are no duplicate ids both in actors and actions
        check_duplicate_action_ids(&raw.script)?;
        check_duplicate_actor_ids(&raw.actors)?;

        // Check that all the actors in the actions are present in the actors list
        validate_actors_in_actions(&raw.script, &raw.actors)?;

        // Check all the nexts and choice.next (they should point to existing actions)
        validate_all_nexts(&raw.script)?;

        // # Graph build Pass
        // 1. Add all action nodes
        let id_nodeidx_map = add_action_nodes(&mut graph, &raw.script, &raw.actors);

        // 2. Add edges to the graph
        connect_action_nodes(&mut graph, &raw.script, &id_nodeidx_map);

        Ok(Screenplay {
            graph,
            current_node: NodeIndex::new(0),
            action_node_map: id_nodeidx_map,
        })
    }
}

/// Connects all the action nodes in the graph based on the `next`
/// and `choices` fields of each `ScriptAction` instance. If both fields are None,
/// the next action in the `actions` slice is connected (unless it's the last one).
///
/// # Arguments
///
/// * `graph` - A mutable reference to a `Graph` instance.
/// * `actions` - A slice of `ScriptAction` instances to connect in the graph.
/// * `id_nodeidx_map` - A `HashMap` that maps `ActionId` values to `NodeIndex` values in the graph.
fn connect_action_nodes(
    graph: &mut Graph<ActionNode, ()>,
    actions: &[ScriptAction],
    id_nodeidx_map: &HashMap<ActionId, NodeIndex>,
) {
    for (i, action) in actions.iter().enumerate() {
        let current_node_idx = id_nodeidx_map.get(&action.id).unwrap();
        if let Some(choices) = &action.choices {
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
    graph: &mut Graph<ActionNode, ()>,
    actions: &[ScriptAction],
    actors: &[Actor],
) -> HashMap<ActionId, NodeIndex> {
    let mut id_nodeidx_map = HashMap::new();

    for action in actions {
        let action_actors = retrieve_actors(&action.actors, actors);
        let mut node = ActionNode {
            kind: action.action.clone(),
            choices: action.choices.clone(),
            text: action.text.clone(),
            sound_effect: action.sound_effect.clone(),
            actors: action_actors,
        };
        // If the action has choices, hardwire the kind to Choice
        if node.choices.is_some() && node.kind != ActionKind::Choice {
            node.kind = ActionKind::Choice;
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
fn retrieve_actors(actor_ids: &[ActorId], actors: &[Actor]) -> Vec<Actor> {
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
/// Returns a `ScreenplayError::InvalidActor` error if any of the actors in any of the actions are not present in the actors list.
fn validate_actors_in_actions(
    actions: &[ScriptAction],
    actors: &[Actor],
) -> Result<(), ScreenplayError> {
    for action in actions {
        validate_actors_in_single_action(action, actors)?;
    }
    Ok(())
}

/// Validate that all the actors in the action are present in the actors list.
///
/// # Arguments
///
/// * `action` - A reference to the `ScriptAction` to validate.
/// * `actors` - A slice of `Actor` structs representing the available actors.
///
/// # Errors
///
/// Returns a `ScreenplayError::InvalidActor` error if any of the actors in the action are not present in the actors list.
fn validate_actors_in_single_action(
    action: &ScriptAction,
    actors: &[Actor],
) -> Result<(), ScreenplayError> {
    for actor_key in action.actors.iter() {
        if !actors.iter().any(|a| a.id == *actor_key) {
            return Err(ScreenplayError::InvalidActor(
                action.id,
                actor_key.to_string(),
            ));
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
/// Returns a `ScreenplayError::DuplicateActionId` error if any `id` value appears more than once in the `actions` vector.
fn check_duplicate_action_ids(actions: &[ScriptAction]) -> Result<(), ScreenplayError> {
    let mut seen_ids = HashSet::new();
    for action in actions {
        if !seen_ids.insert(action.id) {
            return Err(ScreenplayError::DuplicateActionId(action.id));
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
/// Returns a `ScreenplayError::DuplicateActorId` error if any `actor_id` value appears more than once in the `actors` vector.
fn check_duplicate_actor_ids(actors: &[Actor]) -> Result<(), ScreenplayError> {
    let mut seen_ids = HashSet::new();
    for actor in actors {
        if !seen_ids.insert(&actor.id) {
            return Err(ScreenplayError::DuplicateActorId(actor.id.clone()));
        }
    }
    Ok(())
}
/// Check if all `next` fields and `Choice` `next` fields in a `Vec<ScriptAction>` point to real actions.
///
/// # Arguments
///
/// * `actions` - A slice of `ScriptAction` structs representing the available actions.
///
/// # Errors
///
/// Returns a `ScreenplayError::InvalidNextAction` error if any of the `next` fields or `Choice` `next` fields in the `ScriptAction`s do not point to real actions.
fn validate_all_nexts(actions: &[ScriptAction]) -> Result<(), ScreenplayError> {
    let id_set = actions.iter().map(|a| a.id).collect::<HashSet<ActionId>>();
    for action in actions {
        if let Some(choices) = &action.choices {
            for choice in choices {
                if !id_set.contains(&choice.next) {
                    return Err(ScreenplayError::InvalidNextAction(action.id, choice.next));
                }
            }
        } else if let Some(next_id) = &action.next {
            if !id_set.contains(next_id) {
                return Err(ScreenplayError::InvalidNextAction(action.id, *next_id));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{prelude::Choice, tests::minimal_app};
    use bevy::prelude::default;

    #[test]
    fn build_empty_screenplay() {
        let app = minimal_app();
        let assets = app.world.get_resource::<Assets<RawScreenplay>>().unwrap();

        let res = ScreenplayBuilder::new().build(assets);
        assert!(res.is_ok());
        let sp = res.unwrap();
        assert_eq!(sp.graph.node_count(), 0);
        assert_eq!(sp.current_node.index(), 0);
    }

    #[test]
    fn simple_build() {
        let mut app = minimal_app();
        let mut assets = app
            .world
            .get_resource_mut::<Assets<RawScreenplay>>()
            .unwrap();

        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ScriptAction { ..default() }],
        };

        let raw_handle = assets.add(raw_sp);

        let res = ScreenplayBuilder::new()
            .with_raw_screenplay(raw_handle)
            .build(&assets);

        assert!(res.is_ok());

        let sp = res.unwrap();
        assert_eq!(sp.graph.node_count(), 1);
        assert_eq!(sp.graph.edge_count(), 0);
        assert_eq!(sp.action_node_map.len(), 1);
        assert_eq!(sp.current_node.index(), 0);
    }

    #[test]
    fn new_with_self_loop() {
        let mut app = minimal_app();
        let mut assets = app
            .world
            .get_resource_mut::<Assets<RawScreenplay>>()
            .unwrap();

        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ScriptAction {
                id: 1,
                next: Some(1),
                ..default()
            }],
        };

        let raw_handle = assets.add(raw_sp);

        let res = ScreenplayBuilder::new()
            .with_raw_screenplay(raw_handle)
            .build(&assets);

        assert!(res.is_ok());
        let sp = res.unwrap();
        assert_eq!(sp.graph.node_count(), 1);
        assert_eq!(sp.graph.edge_count(), 1);
        assert_eq!(sp.action_node_map.len(), 1);
        assert_eq!(sp.current_node.index(), 0);
    }

    #[test]
    fn new_with_two_actor_action_nodes() {
        let mut app = minimal_app();
        let mut assets = app
            .world
            .get_resource_mut::<Assets<RawScreenplay>>()
            .unwrap();

        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![
                ScriptAction {
                    id: 1,
                    next: Some(2),
                    ..default()
                },
                ScriptAction { id: 2, ..default() },
            ],
        };

        let raw_handle = assets.add(raw_sp);

        let res = ScreenplayBuilder::new()
            .with_raw_screenplay(raw_handle)
            .build(&assets);

        assert!(res.is_ok());
        let sp = res.unwrap();

        assert_eq!(sp.graph.node_count(), 2);
        assert_eq!(sp.graph.edge_count(), 1);
    }

    #[test]
    fn new_with_branching() {
        let mut app = minimal_app();
        let mut assets = app
            .world
            .get_resource_mut::<Assets<RawScreenplay>>()
            .unwrap();

        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![
                ScriptAction {
                    choices: Some(vec![
                        Choice {
                            text: "Choice 1".to_string(),
                            next: 2,
                        },
                        Choice {
                            text: "Choice 2".to_string(),
                            next: 3,
                        },
                    ]),
                    ..default()
                },
                ScriptAction {
                    id: 2,
                    text: Some("Hello".to_string()),
                    ..default()
                },
                ScriptAction { id: 3, ..default() },
            ],
        };

        let raw_handle = assets.add(raw_sp);

        let res = ScreenplayBuilder::new()
            .with_raw_screenplay(raw_handle)
            .build(&assets);

        assert!(res.is_ok());
        let sp = res.unwrap();

        assert_eq!(sp.graph.node_count(), 3);
        assert_eq!(sp.graph.edge_count(), 3);
        assert_eq!(sp.current_node, NodeIndex::new(0));
    }

    #[test]
    fn new_with_actors() {
        let actors = vec![
            Actor {
                id: "bob".to_owned(),
                ..default()
            },
            Actor {
                id: "alice".to_owned(),
                ..default()
            },
        ];
        let mut app = minimal_app();
        let mut assets = app
            .world
            .get_resource_mut::<Assets<RawScreenplay>>()
            .unwrap();
        let raw_sp = RawScreenplay {
            actors: actors,
            script: vec![
                ScriptAction {
                    id: 1,
                    text: Some("Hello".to_string()),
                    actors: vec!["bob".to_string()],
                    next: Some(2),
                    ..default()
                },
                ScriptAction {
                    id: 2,
                    text: Some("Whatup".to_string()),
                    actors: vec!["alice".to_string()],
                    ..default()
                },
            ],
        };

        let raw_handle = assets.add(raw_sp);

        let res = ScreenplayBuilder::new()
            .with_raw_screenplay(raw_handle)
            .build(&assets);

        assert!(res.is_ok());
        let sp = res.unwrap();

        assert_eq!(sp.graph.node_count(), 2);
        assert_eq!(sp.graph.edge_count(), 1);
        assert_eq!(sp.current_node, NodeIndex::new(0));
    }

    #[test]
    fn build_from_raw_invalid_actor() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ScriptAction {
                actors: vec!["bob".to_string()],
                ..default()
            }],
        };

        let res = ScreenplayBuilder::raw_build(&raw_sp).err();

        assert_eq!(
            res,
            Some(ScreenplayError::InvalidActor(0, String::from("bob")))
        );
    }

    #[test]
    fn build_from_raw_invalid_actor_mismath() {
        let actor_vec = vec![Actor {
            id: "bob".to_string(),
            ..default()
        }];
        let raw_sp = RawScreenplay {
            actors: actor_vec,
            script: vec![ScriptAction {
                actors: vec!["alice".to_string()],
                ..default()
            }],
        };
        let res = ScreenplayBuilder::raw_build(&raw_sp).err();
        assert_eq!(
            res,
            Some(ScreenplayError::InvalidActor(0, String::from("alice")))
        );
    }

    #[test]
    fn build_from_raw_with_invalid_next_action() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ScriptAction {
                next: Some(2),
                ..default()
            }],
        };
        let res = ScreenplayBuilder::raw_build(&raw_sp).err();
        assert_eq!(res, Some(ScreenplayError::InvalidNextAction(0, 2)));
    }

    #[test]
    fn next_not_found_in_choice_err() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ScriptAction {
                choices: Some(vec![Choice {
                    next: 2,
                    text: default(),
                }]),
                ..default()
            }],
        };
        let res = ScreenplayBuilder::raw_build(&raw_sp).err();
        assert_eq!(res, Some(ScreenplayError::InvalidNextAction(0, 2)));
    }

    #[test]
    fn build_from_raw_duplicate_id() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![
                ScriptAction { id: 1, ..default() },
                ScriptAction { id: 1, ..default() },
            ],
        };
        let res = ScreenplayBuilder::raw_build(&raw_sp).err();
        assert_eq!(res, Some(ScreenplayError::DuplicateActionId(1)));
    }

    #[test]
    fn build_from_raw_with_empty_ok() {
        let raw = RawScreenplay {
            actors: default(),
            script: default(),
        };

        let sp = ScreenplayBuilder::raw_build(&raw);
        assert!(sp.is_ok())
    }
}
