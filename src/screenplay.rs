use bevy::{prelude::default, reflect::TypeUuid, utils::HashMap};
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex, visit::EdgeRef};

use crate::{
    errors::{ScreenplayError, ScreenplayParsingError},
    types::{
        ActionId, ActionKind, Actor, ActorAction, ActorOrPlayerActionJSON, Choice, RawScreenplay,
    },
};

#[derive(Debug, TypeUuid)]
#[uuid = "413be529-bfeb-8c5b-9db0-4b8b380a2c47"]
pub struct Screenplay {
    graph: DiGraph<ActionNode, ()>,
    current: NodeIndex,
    id_to_nodeidx: HashMap<ActionId, NodeIndex>,
}

impl Screenplay {
    pub(crate) fn new(raw_script: RawScreenplay) -> Result<Self, ScreenplayParsingError> {
        if raw_script.script.is_empty() {
            return Err(ScreenplayParsingError::EmptyScript);
        }
        let mut graph: DiGraph<ActionNode, ()> = DiGraph::new();

        let mut start_action = Option::<NodeIndex>::None;

        // 1. Build auxiliary maps (I'm bad at naming maps)

        // ActionId => next_id map, so we can fill the next when it's None
        // (it means point to the next action) and throw duplicate id error
        let id_to_next_map = build_id_to_next_map(&raw_script.script)?;

        // ActionId => (NodeIndex, next_id) map so we can keep track of what we added in the graph.
        // Right now ActionId == NodeIndex so not really needed, but I'd like to have uuids as ids in the future
        let mut id_to_nodeids_map: HashMap<ActionId, StrippedNodeAction> =
            HashMap::with_capacity(raw_script.script.len());

        // 2. Add all actions as nodes with some validation
        for action in raw_script.script {
            let this_action_id = action.id();
            let start_flag = action.start();

            // Grab the nexts in the choices for later validation
            let choices_nexts = action
                .choices()
                .map(|vc| vc.iter().map(|c| c.next).collect());

            // 2.a add the node to the graph
            let node_idx = add_action_node(&mut graph, action, &raw_script.actors)?;

            // 2.b check if this is the starting action
            if check_start_flag(start_flag, start_action.is_some())? {
                start_action = Some(node_idx);
            }

            // 2.c add (idx, next_id) as we build the graph
            if id_to_nodeids_map
                .insert(
                    this_action_id,
                    StrippedNodeAction {
                        node_idx,
                        next_action_id: id_to_next_map.get(&this_action_id).copied(),
                        choices: choices_nexts,
                    },
                )
                .is_some()
            {
                return Err(ScreenplayParsingError::RepeatedId(this_action_id));
            };
        }

        // 3 Validate all the nexts (they should point to existing actions)
        validate_nexts(&id_to_nodeids_map)?;

        // 4 Add edges to the graph
        for (action_id, node_action) in &id_to_nodeids_map {
            // 4.a With the next field, add a single edge
            if let Some(next_id) = node_action.next_action_id {
                let next_node_action = id_to_nodeids_map.get(&next_id).ok_or(
                    ScreenplayParsingError::NextActionNotFound(*action_id, next_id),
                )?;

                graph.add_edge(node_action.node_idx, next_node_action.node_idx, ());
            }

            // 4.b With the choices, add an edge for each choice
            if let Some(choices) = &node_action.choices {
                for choice in choices {
                    let next_node_action = id_to_nodeids_map.get(choice).ok_or(
                        ScreenplayParsingError::NextActionNotFound(*action_id, *choice),
                    )?;

                    graph.add_edge(node_action.node_idx, next_node_action.node_idx, ());
                }
            }
        }

        // 5. We can drop the next/choices now and just keep action_id => NodeIndex
        let id_to_nodeidx = id_to_nodeids_map
            .into_iter()
            .map(|(id, node_act)| (id, node_act.node_idx))
            .collect();

        Ok(Self {
            graph,
            current: start_action.ok_or(ScreenplayParsingError::NoStartingAction)?,
            id_to_nodeidx,
        })
    }

    pub fn text(&self) -> &str {
        match &self.graph[self.current].text {
            Some(t) => t,
            None => "",
        }
    }

    pub fn next_action(&mut self) -> Result<(), ScreenplayError> {
        let cnode = self.graph.node_weight(self.current);

        // if for some magical reason the current node is not in the graph, return an error
        let cur_dial = cnode.ok_or(ScreenplayError::InvalidAction)?;

        // if it's a player action, return an error
        if cur_dial.choices.is_some() {
            return Err(ScreenplayError::ChoicesNotHandled);
        }

        // retrieve the next edge
        let edge_ref = self
            .graph
            .edges(self.current)
            .next()
            .ok_or(ScreenplayError::NoNextAction)?;

        // what's this NodeId? Is it the NodeIndex? I'm not sure. Let's assign it anyway
        self.current = edge_ref.target();
        Ok(())
    }

    pub fn jump_to(&mut self, id: i32) -> Result<(), ScreenplayError> {
        let idx = self
            .id_to_nodeidx
            .get(&id)
            .ok_or(ScreenplayError::WrongJump(id))?;

        self.current = *idx;
        Ok(())
    }

    /// Returns the choices for the current dialogue. If there are no choices, returns an error.
    pub fn choices(&self) -> Result<Vec<Choice>, ScreenplayError> {
        let cnode = self.graph.node_weight(self.current);
        // if for some fantastic reason the current node is not in the graph, return an error
        let cur_dial = cnode.ok_or(ScreenplayError::InvalidAction)?;

        if let Some(choices) = &cur_dial.choices {
            Ok(choices.clone())
        } else {
            Err(ScreenplayError::NoChoices)
        }
    }

    pub fn first_actor(&self) -> Option<Actor> {
        let cnode = self.graph.node_weight(self.current)?;
        match &cnode.actors {
            Some(actors) => actors.first().cloned(),
            None => None,
        }
    }

    pub fn current_actors(&self) -> Option<Vec<Actor>> {
        let cnode = self.graph.node_weight(self.current)?;
        cnode.actors.clone()
    }

    pub fn at_player_action(&self) -> bool {
        self.graph[self.current].kind == ActionKind::PlayerChoice
    }
    pub fn at_actor_action(&self) -> bool {
        !self.at_player_action()
    }
    pub fn action_kind(&self) -> ActionKind {
        self.graph[self.current].kind
    }
}
#[derive(Debug, Default)]
struct ActionNode {
    kind: ActionKind,
    text: Option<String>,
    actors: Option<Vec<Actor>>,
    choices: Option<Vec<Choice>>,
}

/// A minimal representation of a convo node for validation purposes
#[derive(Debug)]
struct StrippedNodeAction {
    node_idx: NodeIndex,
    next_action_id: Option<ActionId>,
    choices: Option<Vec<ActionId>>,
}

fn build_id_to_next_map(
    script: &Vec<ActorOrPlayerActionJSON>,
) -> Result<HashMap<ActionId, ActionId>, ScreenplayParsingError> {
    let mut id_to_next_map: HashMap<ActionId, ActionId> = HashMap::with_capacity(script.len() - 1);
    for (i, a) in script.iter().enumerate() {
        match a.next() {
            Some(n) => {
                if id_to_next_map.insert(a.id(), n).is_some() {
                    return Err(ScreenplayParsingError::RepeatedId(a.id()));
                }
            }
            None => {
                // if next not defined:
                // either player action (with choices) or actor action pointing to the one below it
                // NOTE: we are not adding the last action (if next: None) as it can't have a next
                if i + 1 < script.len() {
                    id_to_next_map.insert(a.id(), script[i + 1].id());
                }
            }
        };
    }
    Ok(id_to_next_map)
}

fn extract_actors(
    aaction: &ActorAction,
    actors_map: &HashMap<String, Actor>,
) -> Result<Vec<Actor>, ScreenplayParsingError> {
    // Retrieve the actors from the actors map. In case one is not found, return an error.
    let mut actors = Vec::with_capacity(1);
    for actor_key in aaction.actors.iter() {
        let retrieved_actor = actors_map
            .get(actor_key)
            .ok_or_else(|| {
                ScreenplayParsingError::ActorNotFound(aaction.id, actor_key.to_string())
            })?
            .to_owned();
        actors.push(retrieved_actor);
    }
    Ok(actors)
}

fn check_start_flag(
    start_flag: Option<bool>,
    already_have_start: bool,
) -> Result<bool, ScreenplayParsingError> {
    if let Some(true) = start_flag {
        if already_have_start {
            return Err(ScreenplayParsingError::MultipleStartingAction);
        }
        return Ok(true);
    }
    Ok(false)
}

fn add_action_node(
    graph: &mut DiGraph<ActionNode, ()>,
    action: ActorOrPlayerActionJSON,
    actors_map: &HashMap<String, Actor>,
) -> Result<NodeIndex, ScreenplayParsingError> {
    let mut node = ActionNode { ..default() };
    match action {
        ActorOrPlayerActionJSON::Actor(actor_action) => {
            node.actors = Some(extract_actors(&actor_action, actors_map)?);
            node.text = actor_action.text;
            node.kind = actor_action.action.into();
        }
        ActorOrPlayerActionJSON::Player(player_action) => {
            node.choices = Some(player_action.choices);
            node.kind = ActionKind::PlayerChoice;
        }
    }
    let node_idx = graph.add_node(node);
    Ok(node_idx)
}

fn validate_nexts(
    nodeidx_dialogue_map: &HashMap<i32, StrippedNodeAction>,
) -> Result<(), ScreenplayParsingError> {
    for (id, stripped_node) in nodeidx_dialogue_map {
        if let Some(next_id) = stripped_node.next_action_id {
            if !nodeidx_dialogue_map.contains_key(&next_id) {
                return Err(ScreenplayParsingError::NextActionNotFound(*id, next_id));
            }
        } else if let Some(vc) = &stripped_node.choices {
            for c in vc {
                if !nodeidx_dialogue_map.contains_key(c) {
                    return Err(ScreenplayParsingError::NextActionNotFound(*id, *c));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::{
        ActionKind, ActorAction, ActorActionKind, ActorOrPlayerActionJSON, PlayerAction,
    };
    use bevy::prelude::default;

    fn an_actors_map(name: String) -> HashMap<String, Actor> {
        let mut actors = HashMap::new();
        actors.insert(
            name,
            Actor {
                name: "Bob".to_string(),
                asset: "bob.png".to_string(),
            },
        );
        actors
    }

    // 'new' tests
    #[test]
    fn no_script_err() {
        let raw_script = RawScreenplay {
            actors: default(),
            script: default(),
        };

        let convo = Screenplay::new(raw_script).err();
        assert_eq!(convo, Some(ScreenplayParsingError::EmptyScript));
    }

    #[test]
    fn actor_not_found_err() {
        let raw_script = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                text: Some("Hello".to_string()),
                actors: vec!["Bob".to_string()],
                start: Some(true),
                ..default()
            })],
        };

        let convo = Screenplay::new(raw_script).err();
        assert_eq!(
            convo,
            Some(ScreenplayParsingError::ActorNotFound(0, "Bob".to_string()))
        );
    }

    #[test]
    fn actor_not_found_with_mismath_err() {
        let raw_talk = RawScreenplay {
            actors: an_actors_map("Bob".to_string()),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                actors: vec!["Alice".to_string()],
                start: Some(true),
                ..default()
            })],
        };

        let convo = Screenplay::new(raw_talk).err();
        assert_eq!(
            convo,
            Some(ScreenplayParsingError::ActorNotFound(
                0,
                "Alice".to_string()
            ))
        );
    }

    #[test]
    fn no_start_err() {
        let raw_talk = RawScreenplay {
            actors: an_actors_map("Alice".to_string()),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                actors: vec!["Alice".to_string()],

                ..default()
            })],
        };

        let convo = Screenplay::new(raw_talk).err();
        assert_eq!(convo, Some(ScreenplayParsingError::NoStartingAction));
    }

    #[test]
    fn multiple_start_actor_action_err() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    start: Some(true),
                    ..default()
                }),
            ],
        };

        let convo = Screenplay::new(raw_talk).err();
        assert_eq!(convo, Some(ScreenplayParsingError::MultipleStartingAction));
    }

    #[test]
    fn multiple_start_mixed_err() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Player(PlayerAction {
                    id: 3,
                    start: Some(true),
                    ..default()
                }),
            ],
        };

        let convo = Screenplay::new(raw_talk).err();
        assert_eq!(convo, Some(ScreenplayParsingError::MultipleStartingAction));
    }

    #[test]
    fn multiple_start_player_action_err() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Player(PlayerAction {
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Player(PlayerAction {
                    start: Some(true),
                    ..default()
                }),
            ],
        };

        let convo = Screenplay::new(raw_talk).err();
        assert_eq!(convo, Some(ScreenplayParsingError::MultipleStartingAction));
    }

    #[test]
    fn repeated_id_actor_err() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    id: 1,
                    text: Some("Hello".to_string()),
                    next: Some(1),
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    id: 1,
                    text: Some("Whatup".to_string()),
                    next: Some(2),
                    ..default()
                }),
            ],
        };

        let convo = Screenplay::new(raw_talk).err();
        assert_eq!(convo, Some(ScreenplayParsingError::RepeatedId(1)));
    }

    #[test]
    fn repeated_id_mixed_err() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    id: 1,
                    text: Some("Hello".to_string()),
                    next: Some(1),
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Player(PlayerAction { id: 1, ..default() }),
            ],
        };

        let convo = Screenplay::new(raw_talk).err();
        assert_eq!(convo, Some(ScreenplayParsingError::RepeatedId(1)));
    }

    #[test]
    fn repeated_id_player_err() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Player(PlayerAction {
                    id: 1,
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Player(PlayerAction { id: 1, ..default() }),
            ],
        };

        let convo = Screenplay::new(raw_talk).err();
        assert_eq!(convo, Some(ScreenplayParsingError::RepeatedId(1)));
    }

    #[test]
    fn next_actor_action_not_found_err() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                next: Some(2),
                start: Some(true),
                ..default()
            })],
        };

        let convo = Screenplay::new(raw_talk).err();
        assert_eq!(
            convo,
            Some(ScreenplayParsingError::NextActionNotFound(0, 2))
        );
    }

    #[test]
    fn next_not_found_in_choice_err() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Player(PlayerAction {
                choices: vec![Choice {
                    text: "Whatup".to_string(),
                    next: 2,
                }],
                start: Some(true),
                ..default()
            })],
        };

        let convo = Screenplay::new(raw_talk).err();
        assert_eq!(
            convo,
            Some(ScreenplayParsingError::NextActionNotFound(0, 2))
        );
    }

    #[test]
    fn new_with_one_action() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                start: Some(true),
                ..default() // end: None,
            })],
        };

        let convo = Screenplay::new(raw_talk).unwrap();
        assert_eq!(convo.graph.node_count(), 1);
        assert_eq!(convo.graph.edge_count(), 0);
        assert_eq!(convo.current, NodeIndex::new(0));
    }

    #[test]
    fn new_with_two_actor_action_nodes() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    id: 1,
                    next: Some(2),
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
            ],
        };

        let convo = Screenplay::new(raw_talk).unwrap();
        assert_eq!(convo.graph.node_count(), 2);
        assert_eq!(convo.graph.edge_count(), 1);
    }

    #[test]
    fn new_with_self_loop() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                id: 1,
                next: Some(1),
                start: Some(true),
                ..default()
            })],
        };

        let convo = Screenplay::new(raw_talk).unwrap();
        assert_eq!(convo.graph.node_count(), 1);
        assert_eq!(convo.graph.edge_count(), 1);
    }

    #[test]
    fn new_with_branching() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Player(PlayerAction {
                    choices: vec![
                        Choice {
                            text: "Choice 1".to_string(),
                            next: 2,
                        },
                        Choice {
                            text: "Choice 2".to_string(),
                            next: 3,
                        },
                    ],
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    id: 2,
                    text: Some("Hello".to_string()),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction { id: 3, ..default() }),
            ],
        };

        let convo = Screenplay::new(raw_talk).unwrap();
        assert_eq!(convo.graph.node_count(), 3);
        assert_eq!(convo.graph.edge_count(), 4);
        assert_eq!(convo.current, NodeIndex::new(0));
    }

    #[test]
    fn new_with_actors() {
        let mut actors_map = an_actors_map("bob".to_string());
        actors_map.insert(
            "alice".to_string(),
            Actor {
                name: "Alice".to_string(),
                asset: "alice.png".to_string(),
            },
        );

        let raw_talk = RawScreenplay {
            actors: actors_map,
            script: vec![
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    id: 1,
                    text: Some("Hello".to_string()),
                    actors: vec!["bob".to_string()],
                    next: Some(2),
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    id: 2,
                    text: Some("Whatup".to_string()),
                    actors: vec!["alice".to_string()],
                    ..default()
                }),
            ],
        };

        let convo = Screenplay::new(raw_talk).unwrap();
        assert_eq!(convo.graph.node_count(), 2);
        assert_eq!(convo.graph.edge_count(), 1);
        assert_eq!(convo.current, NodeIndex::new(0));
    }

    // 'current_text' tests
    #[test]
    fn current_text() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                id: 1,
                text: Some("Hello".to_string()),
                next: None,
                start: Some(true),
                ..default()
            })],
        };

        let convo = Screenplay::new(raw_talk).unwrap();
        assert_eq!(convo.text(), "Hello");
    }

    // 'next_line' tests
    #[test]
    fn next_no_next_err() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                id: 1,
                text: Some("Hello".to_string()),
                start: Some(true),
                ..default()
            })],
        };

        let mut convo = Screenplay::new(raw_talk).unwrap();
        assert_eq!(
            convo.next_action().err(),
            Some(ScreenplayError::NoNextAction)
        );
    }

    #[test]
    fn next_choices_not_handled_err() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Player(PlayerAction {
                    id: 1,
                    choices: vec![Choice {
                        text: "Whatup".to_string(),
                        next: 2,
                    }],
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
            ],
        };

        let mut convo = Screenplay::new(raw_talk).unwrap();
        assert_eq!(
            convo.next_action().err(),
            Some(ScreenplayError::ChoicesNotHandled)
        );
    }

    #[test]
    fn next_action() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    next: Some(2),
                    start: Some(true),
                    text: Some("Hello".to_string()),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    id: 2,
                    text: Some("Whatup".to_string()),
                    ..default()
                }),
            ],
        };

        let mut convo = Screenplay::new(raw_talk).unwrap();
        assert_eq!(convo.text(), "Hello");
        assert!(convo.next_action().is_ok());
        assert_eq!(convo.text(), "Whatup");
    }

    // 'choices' tests
    #[test]
    fn choices_no_choices_err() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                id: 1,
                start: Some(true),
                ..default()
            })],
        };

        let convo = Screenplay::new(raw_talk).unwrap();
        assert_eq!(convo.choices().err(), Some(ScreenplayError::NoChoices));
    }

    #[test]
    fn choices() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Player(PlayerAction {
                    id: 1,
                    choices: vec![
                        Choice {
                            text: "Choice 1".to_string(),
                            next: 2,
                        },
                        Choice {
                            text: "Choice 2".to_string(),
                            next: 3,
                        },
                    ],
                    start: Some(true),
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
                ActorOrPlayerActionJSON::Actor(ActorAction { id: 3, ..default() }),
            ],
        };

        let convo = Screenplay::new(raw_talk).unwrap();

        assert_eq!(convo.choices().unwrap()[0].next, 2);
        assert_eq!(convo.choices().unwrap()[1].next, 3);
        assert_eq!(convo.choices().unwrap()[0].text, "Choice 1");
        assert_eq!(convo.choices().unwrap()[1].text, "Choice 2");
    }

    // 'jump_to' tests
    #[test]
    fn jump_to_no_action_err() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                id: 1,
                start: Some(true),
                ..default()
            })],
        };

        let mut convo = Screenplay::new(raw_talk).unwrap();
        assert_eq!(convo.jump_to(2).err(), Some(ScreenplayError::WrongJump(2)));
    }

    #[test]
    fn jump_to() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Player(PlayerAction {
                    id: 1,
                    choices: vec![
                        Choice {
                            text: "Choice 1".to_string(),
                            next: 2,
                        },
                        Choice {
                            text: "Choice 2".to_string(),
                            next: 3,
                        },
                    ],
                    start: Some(true),
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    id: 2,
                    text: Some("I'm number 2".to_string()),
                    next: Some(3),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction { id: 3, ..default() }),
            ],
        };

        let mut convo = Screenplay::new(raw_talk).unwrap();
        assert!(convo.jump_to(2).is_ok());
        assert_eq!(convo.text(), "I'm number 2");
    }

    // 'current_first_actor' tests
    #[test]
    fn first_actor_none() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                start: Some(true),
                ..default()
            })],
        };

        let convo = Screenplay::new(raw_talk).unwrap();
        assert!(convo.first_actor().is_none());
    }

    #[test]
    fn first_actor() {
        let actors_map = an_actors_map("bob".to_string());

        let raw_talk = RawScreenplay {
            actors: actors_map,
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                actors: vec!["bob".to_string()],
                start: Some(true),
                ..default()
            })],
        };

        let convo = Screenplay::new(raw_talk).unwrap();
        assert!(convo.first_actor().is_some());
    }

    #[test]
    fn current_actors() {
        let mut actors_map = an_actors_map("bob".to_string());
        actors_map.insert(
            "alice".to_string(),
            Actor {
                name: "alice".to_string(),
                asset: "alice".to_string(),
            },
        );

        let raw_play = RawScreenplay {
            actors: actors_map,
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                actors: vec!["bob".to_string(), "alice".to_string()],
                start: Some(true),
                ..default()
            })],
        };

        let play = Screenplay::new(raw_play).unwrap();
        assert_eq!(play.current_actors().unwrap().len(), 2);
    }

    #[test]
    fn at_player_action() {
        let raw_talk = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Player(PlayerAction {
                    id: 1,
                    choices: vec![Choice {
                        text: "Whatup".to_string(),
                        next: 2,
                    }],
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
            ],
        };

        let convo = Screenplay::new(raw_talk).unwrap();
        assert!(convo.at_player_action());
    }

    #[test]
    fn action_kind_player() {
        let raw_play = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Player(PlayerAction {
                    id: 1,
                    choices: vec![Choice {
                        text: "Whatup".to_string(),
                        next: 2,
                    }],
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
            ],
        };

        let sp = Screenplay::new(raw_play).unwrap();
        assert_eq!(sp.action_kind(), ActionKind::PlayerChoice);
    }

    #[test]
    fn action_kind_actor() {
        let raw_play = RawScreenplay {
            actors: default(),
            script: vec![
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    id: 1,
                    start: Some(true),
                    ..default()
                }),
                ActorOrPlayerActionJSON::Actor(ActorAction {
                    id: 2,
                    action: ActorActionKind::Enter,
                    ..default()
                }),
            ],
        };

        let mut sp = Screenplay::new(raw_play).unwrap();
        assert_eq!(sp.action_kind(), ActionKind::ActorTalk);
        sp.next_action().unwrap();
        assert_eq!(sp.action_kind(), ActionKind::ActorEnter);
    }
}
