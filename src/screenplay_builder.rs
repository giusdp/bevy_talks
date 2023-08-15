//! The main module of the crate. It contains the Screenplay struct and its
//! builder.
use bevy::{
    prelude::{default, Assets, Handle},
    reflect::{Reflect, TypeUuid},
    utils::HashMap,
};
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex};
use serde::Deserialize;

use crate::prelude::{
    ActionId, ActionKind, ActionNode, Actor, Screenplay, ScreenplayError, ScriptAction,
};

/// A struct that represents a raw screenplay (as from the json format).
///
/// It contains a list of actors that appear in the screenplay, and a list of actions that make up the screenplay.
#[derive(Debug, Deserialize, Clone, Reflect, TypeUuid)]
#[uuid = "413be529-bfeb-8c5b-9db0-4b8b380a2c47"]
#[reflect_value]
pub struct RawScreenplay {
    /// The list of actors that appear in the screenplay.
    pub(crate) actors: HashMap<String, Actor>,
    /// The list of actions that make up the screenplay.
    pub(crate) script: Vec<ScriptAction>,
}

/// The [`ScreenplayBuilder`] is used to construct a [`Screenplay`].
/// A [`RawScreenplay`] can be used to build the Screenplay component.
#[derive(Default)]
pub struct ScreenplayBuilder {
    /// The RawScreenplay to be used to build the screenplay.
    raw_sp: Option<Handle<RawScreenplay>>,
}

impl ScreenplayBuilder {
    /// Create a new [`ScreenplayBuilder`] with default values.
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

    /// Build the screenplay from the raw screenplay.
    pub fn raw_build(raw: &RawScreenplay) -> Result<Screenplay, ScreenplayError> {
        if raw.script.is_empty() {
            return Ok(Screenplay { ..default() });
        }

        let mut graph: DiGraph<ActionNode, ()> =
            DiGraph::with_capacity(raw.script.len(), raw.script.len());

        // 1. Build auxiliary maps (I'm bad at naming maps)

        // ActionId => next_id map, so we can fill the next when it's None
        // (it means point to the next action) and throw duplicate id error
        let tmp_action_next_map = action_next_map(&raw.script)?;

        // ActionId => (NodeIndex, Next, Choices) map so we can keep track of what we added in the graph.
        let mut id_nexts_map = HashMap::with_capacity(raw.script.len());

        // 2. Add all actions as nodes with some validation
        for action in &raw.script {
            let this_action_id = action.id;

            // Grab the nexts in the choices for later validation
            let c_nexts = action
                .choices
                .clone()
                .map(|cs| cs.iter().map(|c| c.next).collect::<Vec<ActionId>>());

            // 2.a validate actors
            valdidate_actors(action, &raw.actors)?;

            // 2.b add the node to the graph
            let actors = extract_actors(action, &raw.actors)?;
            let node_idx = insert_action_node(&mut graph, action.clone(), actors);

            // 2.c add (idx, next_id) as we build the graph
            if id_nexts_map
                .insert(
                    this_action_id,
                    StrippedAction {
                        node_idx,
                        next_action: tmp_action_next_map.get(&this_action_id).copied(),
                        choices: c_nexts,
                    },
                )
                .is_some()
            {
                return Err(ScreenplayError::DuplicateActionId(this_action_id));
            };
        }

        // 3 Validate all the nexts (they should point to existing actions)
        validate_nexts(&id_nexts_map)?;

        // 4 Add edges to the graph
        for (action_id, this_action) in &id_nexts_map {
            connect_actions(&mut graph, this_action, &id_nexts_map, action_id)?;
        }

        // 5. We can drop the next/choices now and just keep action_id => NodeIndex
        let action_node_map = id_nexts_map
            .into_iter()
            .map(|(id, stripped_act)| (id, stripped_act.node_idx))
            .collect();

        Ok(Screenplay {
            graph,
            current_node: NodeIndex::new(0),
            action_node_map,
        })
    }
}

/// Validate that all the nexts point to existing actions
fn validate_nexts(id_nexts_map: &HashMap<i32, StrippedAction>) -> Result<(), ScreenplayError> {
    for (id, stripped_action) in id_nexts_map {
        if let Some(next_id) = stripped_action.next_action {
            if !id_nexts_map.contains_key(&next_id) {
                return Err(ScreenplayError::InvalidNextAction(*id, next_id));
            }
        } else if let Some(vc) = &stripped_action.choices {
            for c in vc {
                if !id_nexts_map.contains_key(c) {
                    return Err(ScreenplayError::InvalidNextAction(*id, *c));
                }
            }
        }
    }
    Ok(())
}

/// Build a map of `ActionId` => `next_id`
fn action_next_map(
    script: &Vec<ScriptAction>,
) -> Result<HashMap<ActionId, ActionId>, ScreenplayError> {
    let mut m: HashMap<ActionId, ActionId> = HashMap::with_capacity(script.len() - 1);

    for (i, action) in script.iter().enumerate() {
        match action.next {
            Some(next_id) => {
                if m.insert(action.id, next_id).is_some() {
                    // if already present, then the id is repeated
                    return Err(ScreenplayError::DuplicateActionId(action.id));
                }
            }
            None => {
                // if next not defined:
                // either action with choices or action pointing to the one below it
                // NOTE: we are not adding the last action (if next: None) as it can't have a next
                if i + 1 < script.len() {
                    m.insert(action.id, script[i + 1].id);
                }
            }
        };
    }
    Ok(m)
}

/// Validate that all the actors in the action are present in the actors list
fn valdidate_actors(
    action: &ScriptAction,
    actors_map: &HashMap<String, Actor>,
) -> Result<(), ScreenplayError> {
    for actor_key in action.actors.iter() {
        if !actors_map.contains_key(actor_key) {
            return Err(ScreenplayError::InvalidActor(
                action.id,
                actor_key.to_string(),
            ));
        }
    }
    Ok(())
}

/// Connect the actions in the graph by adding edges based on the nexts and choices
fn connect_actions(
    graph: &mut petgraph::Graph<ActionNode, ()>,
    this_action: &StrippedAction,
    id_nexts_map: &bevy::utils::hashbrown::HashMap<i32, StrippedAction>,
    action_id: &i32,
) -> Result<(), ScreenplayError> {
    // 4.a If choices are present, add an edge for each choice and skip the next field
    if let Some(choices) = &this_action.choices {
        for choice in choices {
            let chosen_action = id_nexts_map
                .get(choice)
                .ok_or(ScreenplayError::InvalidNextAction(*action_id, *choice))?;

            graph.add_edge(this_action.node_idx, chosen_action.node_idx, ());
        }
    } else if let Some(next_id) = this_action.next_action {
        // 4.b With the next field, add a single edge
        let next_node_action = id_nexts_map
            .get(&next_id)
            .ok_or(ScreenplayError::InvalidNextAction(*action_id, next_id))?;
        graph.add_edge(this_action.node_idx, next_node_action.node_idx, ());
    }
    Ok(())
}

/// Inserts an action node into a screenplay graph.
///
/// # Arguments
///
/// * `graph` - The graph to insert the action node into.
/// * `action` - The script action to create the action node from.
/// * `actors` - The actors involved in the script action.
///
/// # Returns
///
/// Returns the index of the newly created action node in the graph.
fn insert_action_node(
    graph: &mut DiGraph<ActionNode, ()>,
    action: ScriptAction,
    actors: Vec<Actor>,
) -> NodeIndex {
    let mut node = ActionNode {
        kind: action.action,
        choices: action.choices,
        text: action.text,
        sound_effect: action.sound_effect,
        actors,
    };
    if node.choices.is_some() {
        node.kind = ActionKind::Choice;
    }

    graph.add_node(node)
}

/// Extracts the actors involved in a script action from an actors map.
///
/// # Arguments
///
/// * `action` - The script action to extract actors from.
/// * `actors_map` - The map of actors to retrieve from.
///
/// # Errors
///
/// Returns a `ScreenplayError::InvalidActor` error if an actor is not found in the actors map.
///
/// # Returns
///
/// Returns a vector of actors involved in the script action.
fn extract_actors(
    action: &ScriptAction,
    actors_map: &HashMap<String, Actor>,
) -> Result<Vec<Actor>, ScreenplayError> {
    // Retrieve the actors from the actors map. In case one is not found, return an error.
    let mut actors = Vec::with_capacity(1);
    for actor_key in action.actors.iter() {
        let retrieved_actor = actors_map
            .get(actor_key)
            .ok_or_else(|| ScreenplayError::InvalidActor(action.id, actor_key.to_string()))?
            .to_owned();
        actors.push(retrieved_actor);
    }
    Ok(actors)
}

/// A minimal representation of a node for validation purposes
#[derive(Debug)]
struct StrippedAction {
    /// The index of the node in the graph
    node_idx: NodeIndex,
    /// The next action id
    next_action: Option<ActionId>,
    /// The nexts in the choices
    choices: Option<Vec<ActionId>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        prelude::Choice,
        tests::{minimal_app, test_actors_map},
    };
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
        let mut actors = test_actors_map("bob".to_owned());
        actors.insert("alice".to_owned(), Actor::default());
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
        let actor_map = test_actors_map("bob".to_owned());
        let raw_sp = RawScreenplay {
            actors: actor_map,
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
