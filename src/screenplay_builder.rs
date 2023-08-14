//! The main module of the crate. It contains the Screenplay struct and its
//! builder.
use bevy::{
    prelude::{default, Commands},
    reflect::{Reflect, TypeUuid},
    utils::HashMap,
};
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex};
use serde::Deserialize;

use crate::prelude::{ActionId, Actor, Screenplay, ScreenplayError, ScriptAction};

/// A struct that represents a raw screenplay (as from the json format).
///
/// It contains a list of actors that appear in the screenplay, and a list of actions that make up the screenplay.
#[derive(Debug, Deserialize, Clone, Reflect, TypeUuid)]
#[uuid = "413be529-bfeb-8c5b-9db0-4b8b380a2c47"]
#[reflect_value]
pub struct RawScreenplay {
    /// The list of actors that appear in the screenplay.
    pub(crate) actors: Vec<Actor>,
    /// The list of actions that make up the screenplay.
    pub(crate) script: Vec<ScriptAction>,
}

/// The [`ScreenplayBuilder`] is used to construct a [`Screenplay`].
/// A [`RawScreenplay`] can be used to build the Screenplay component.
#[derive(Default)]
pub struct ScreenplayBuilder {
    /// The nodes of the screenplay.
    nodes: Vec<ScriptAction>,
    raw_sp: Option<RawScreenplay>,
}

impl ScreenplayBuilder {
    /// Create a new [`ScreenplayBuilder`] with default values.
    pub fn new() -> ScreenplayBuilder {
        // Set the minimally required fields of Foo.
        ScreenplayBuilder {
            nodes: vec![],
            raw_sp: None,
        }
    }

    /// Set the [`RawScreenplay`] to be used to build the screenplay.
    /// If there are other action nodes defined, they will be appended at the end.
    pub fn with_raw_screenplay(mut self, sp: RawScreenplay) -> ScreenplayBuilder {
        self.raw_sp = Some(sp);
        self
    }

    /// Add an action node to the screenplay.
    pub fn add_action_node(mut self, action: ScriptAction) -> ScreenplayBuilder {
        self.nodes.push(action);
        self
    }

    /// Build the screenplay.
    pub fn build(self) -> Result<Screenplay, ScreenplayError> {
        let (raw_action_graph, action_node_map) = self.build_from_raw()?;

        // let first_action = self.nodes[0];
        // let mut prev_node = graph.add_node(first_action);
        // let current_node = prev_node;

        // // 2. Add all actions as nodes and connect them linearly
        // for action in self.nodes[1..].iter() {
        //     let curr_node = graph.add_node(*action);
        //     graph.add_edge(prev_node, curr_node, ());
        //     prev_node = curr_node;
        // }

        // let graph = raw_action_graph.map(|i, n| commands.spawn_empty().id(), |i, e| {});

        Ok(Screenplay {
            action_node_map,
            ..default()
        })
    }

    fn build_from_raw(
        self,
    ) -> Result<(DiGraph<ScriptAction, ()>, HashMap<ActionId, NodeIndex>), ScreenplayError> {
        if self.raw_sp.is_none() {
            return Ok((DiGraph::new(), HashMap::new()));
        }

        let raw = self.raw_sp.unwrap();

        if raw.script.is_empty() {
            return Ok((DiGraph::new(), HashMap::new()));
        }

        let mut graph: DiGraph<ScriptAction, ()> = DiGraph::new();

        // 1. Build auxiliary maps (I'm bad at naming maps)

        // ActionId => next_id map, so we can fill the next when it's None
        // (it means point to the next action) and throw duplicate id error
        let tmp_action_next_map = Self::action_next_map(&raw.script)?;

        // ActionId => (NodeIndex, Next, Choices) map so we can keep track of what we added in the graph.
        let mut id_nexts_map = HashMap::with_capacity(raw.script.len());

        // 2. Add all actions as nodes with some validation
        for action in raw.script {
            let this_action_id = action.id;

            // Grab the nexts in the choices for later validation
            let c_nexts = action
                .choices
                .clone()
                .map(|cs| cs.iter().map(|c| c.next).collect::<Vec<ActionId>>());

            // 2.a validate actors
            Self::valdidate_actors(&action, &raw.actors)?;

            // 2.b add the node to the graph
            let node_idx = graph.add_node(action);

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
        Self::validate_nexts(&id_nexts_map)?;

        // 4 Add edges to the graph
        for (action_id, this_action) in &id_nexts_map {
            // 4.a With the next field, add a single edge

            if let Some(next_id) = this_action.next_action {
                let next_node_action = id_nexts_map
                    .get(&next_id)
                    .ok_or(ScreenplayError::InvalidNextAction(*action_id, next_id))?;
                graph.add_edge(this_action.node_idx, next_node_action.node_idx, ());
            } else if let Some(choices) = &this_action.choices {
                // 4.b With the choices, add an edge for each choice
                for choice in choices {
                    let chosen_action = id_nexts_map
                        .get(choice)
                        .ok_or(ScreenplayError::InvalidNextAction(*action_id, *choice))?;

                    graph.add_edge(this_action.node_idx, chosen_action.node_idx, ());
                }
            }
        }

        // 5. We can drop the next/choices now and just keep action_id => NodeIndex
        let id_to_nodeidx: HashMap<ActionId, NodeIndex> = id_nexts_map
            .into_iter()
            .map(|(id, stripped_act)| (id, stripped_act.node_idx))
            .collect();

        Ok((graph, id_to_nodeidx))
    }

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

    fn valdidate_actors(action: &ScriptAction, actors: &Vec<Actor>) -> Result<(), ScreenplayError> {
        for actor_key in action.actors.iter() {
            if !actors
                .iter()
                .any(|a: &Actor| a.actor_id == actor_key.to_string())
            {
                return Err(ScreenplayError::InvalidActor(
                    action.id,
                    actor_key.to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// A minimal representation of a convo node for validation purposes
#[derive(Debug)]
struct StrippedAction {
    node_idx: NodeIndex,
    next_action: Option<ActionId>,
    choices: Option<Vec<ActionId>>,
}

#[cfg(test)]
mod tests {
    use bevy::prelude::default;

    use crate::prelude::Choice;

    use super::*;

    // #[test]
    // fn build_empty_screenplay() {
    //     let res = ScreenplayBuilder::new().build_from_raw();
    //     assert!(res.is_ok());
    //     let sp = res.unwrap();
    //     assert_eq!(sp.graph.node_count(), 0);
    //     assert_eq!(sp.current_node.index(), 0);
    // }

    // #[test]
    // fn build_one_action_screenplay() {
    //     let res = ScreenplayBuilder::new()
    //         .add_action_node(ActionNode::PLACEHOLDER)
    //         .build();

    //     assert!(res.is_ok());
    //     let sp = res.unwrap();

    //     assert_eq!(sp.graph.node_count(), 1);
    //     assert_eq!(sp.graph.edge_count(), 0);
    //     assert!(sp.current_node.index() == 0);
    // }

    // #[test]
    // fn build_two_actions_screenplay() {
    //     let res = ScreenplayBuilder::new()
    //         .add_action_node(ActionNode::PLACEHOLDER)
    //         .add_action_node(ActionNode::PLACEHOLDER)
    //         .build();
    //     assert!(res.is_ok());
    //     let sp = res.unwrap();

    //     assert_eq!(sp.graph.node_count(), 2);
    //     assert_eq!(sp.graph.edge_count(), 1);
    //     assert!(sp.current_node.index() == 0);
    // }

    #[test]
    fn build_from_raw_success() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ScriptAction { ..default() }],
        };

        let res = ScreenplayBuilder::new()
            .with_raw_screenplay(raw_sp)
            .build_from_raw();

        assert!(res.is_ok());

        let (graph, map) = res.unwrap();

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
        assert_eq!(map.len(), 1);
    }

    //     #[test]
    //     fn new_with_two_actor_action_nodes() {
    //         let raw_sp = RawScreenplay {
    //             actors: default(),
    //             script: vec![
    //                 ActorOrPlayerActionJSON::Actor(ActorAction {
    //                     id: 1,
    //                     next: Some(2),
    //                     start: Some(true),
    //                     ..default()
    //                 }),
    //                 ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
    //             ],
    //         };

    //         let play = build_screenplay(raw_sp).unwrap();
    //         assert_eq!(play.graph.node_count(), 2);
    //         assert_eq!(play.graph.edge_count(), 1);
    //     }

    //     #[test]
    //     fn new_with_self_loop() {
    //         let raw_sp = RawScreenplay {
    //             actors: default(),
    //             script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
    //                 id: 1,
    //                 next: Some(1),
    //                 start: Some(true),
    //                 ..default()
    //             })],
    //         };

    //         let play = build_screenplay(raw_sp).unwrap();
    //         assert_eq!(play.graph.node_count(), 1);
    //         assert_eq!(play.graph.edge_count(), 1);
    //     }

    //     #[test]
    //     fn new_with_branching() {
    //         let raw_sp = RawScreenplay {
    //             actors: default(),
    //             script: vec![
    //                 ActorOrPlayerActionJSON::Player(PlayerAction {
    //                     choices: vec![
    //                         Choice {
    //                             text: "Choice 1".to_string(),
    //                             next: 2,
    //                         },
    //                         Choice {
    //                             text: "Choice 2".to_string(),
    //                             next: 3,
    //                         },
    //                     ],
    //                     start: Some(true),
    //                     ..default()
    //                 }),
    //                 ActorOrPlayerActionJSON::Actor(ActorAction {
    //                     id: 2,
    //                     text: Some("Hello".to_string()),
    //                     ..default()
    //                 }),
    //                 ActorOrPlayerActionJSON::Actor(ActorAction { id: 3, ..default() }),
    //             ],
    //         };

    //         let play = build_screenplay(raw_sp).unwrap();
    //         assert_eq!(play.graph.node_count(), 3);
    //         assert_eq!(play.graph.edge_count(), 4);
    //         assert_eq!(play.current, NodeIndex::new(0));
    //     }

    //     #[test]
    //     fn new_with_actors() {
    //         let mut actors_map = an_actors_map("bob".to_string());
    //         actors_map.insert(
    //             "alice".to_string(),
    //             Actor {
    //                 name: "Alice".to_string(),
    //                 asset: "alice.png".to_string(),
    //             },
    //         );

    //         let raw_sp = RawScreenplay {
    //             actors: actors_map,
    //             script: vec![
    //                 ActorOrPlayerActionJSON::Actor(ActorAction {
    //                     id: 1,
    //                     text: Some("Hello".to_string()),
    //                     actors: vec!["bob".to_string()],
    //                     next: Some(2),
    //                     start: Some(true),
    //                     ..default()
    //                 }),
    //                 ActorOrPlayerActionJSON::Actor(ActorAction {
    //                     id: 2,
    //                     text: Some("Whatup".to_string()),
    //                     actors: vec!["alice".to_string()],
    //                     ..default()
    //                 }),
    //             ],
    //         };
    //         let play = build_screenplay(raw_sp).unwrap();

    //         assert_eq!(play.graph.node_count(), 2);
    //         assert_eq!(play.graph.edge_count(), 1);
    //         assert_eq!(play.current, NodeIndex::new(0));
    //     }

    #[test]
    fn build_from_raw_invalid_actor() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ScriptAction {
                actors: vec!["bob".to_string()],
                ..default()
            }],
        };

        let res = ScreenplayBuilder::new()
            .with_raw_screenplay(raw_sp)
            .build_from_raw()
            .err();

        assert_eq!(
            res,
            Some(ScreenplayError::InvalidActor(0, String::from("bob")))
        );
    }

    #[test]
    fn build_from_raw_invalid_actor_mismath() {
        let raw_sp = RawScreenplay {
            actors: vec![Actor {
                actor_id: "bob".to_string(),
                ..default()
            }],
            script: vec![ScriptAction {
                actors: vec!["alice".to_string()],
                ..default()
            }],
        };
        let res = ScreenplayBuilder::new()
            .with_raw_screenplay(raw_sp)
            .build_from_raw()
            .err();

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

        let res = ScreenplayBuilder::new()
            .with_raw_screenplay(raw_sp)
            .build_from_raw()
            .err();
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

        let res = ScreenplayBuilder::new()
            .with_raw_screenplay(raw_sp)
            .build_from_raw()
            .err();
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

        let res = ScreenplayBuilder::new()
            .with_raw_screenplay(raw_sp)
            .build_from_raw()
            .err();

        assert_eq!(res, Some(ScreenplayError::DuplicateActionId(1)));
    }

    #[test]
    fn build_from_raw_with_empty_ok() {
        let raw = RawScreenplay {
            actors: vec![],
            script: vec![],
        };
        let sp = ScreenplayBuilder::new()
            .with_raw_screenplay(raw)
            .build_from_raw();

        assert!(sp.is_ok())
    }
}
