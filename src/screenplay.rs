use bevy::{reflect::TypeUuid, utils::HashMap};
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex, visit::EdgeRef};

use crate::{
    errors::ScreenplayError,
    types::{ActionId, ActionKind, Actor, Choice},
};

#[derive(Debug, TypeUuid)]
#[uuid = "413be529-bfeb-8c5b-9db0-4b8b380a2c47"]
pub struct Screenplay {
    pub(crate) graph: DiGraph<ActionNode, ()>,
    pub(crate) current: NodeIndex,
    pub(crate) id_to_nodeidx: HashMap<ActionId, NodeIndex>,
}

impl Screenplay {
    pub(crate) fn new(
        graph: DiGraph<ActionNode, ()>,
        current: NodeIndex,
        id_to_nodeidx: HashMap<ActionId, NodeIndex>,
    ) -> Self {
        Self {
            graph,
            current,
            id_to_nodeidx,
        }
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
pub(crate) struct ActionNode {
    pub(crate) kind: ActionKind,
    pub(crate) text: Option<String>,
    pub(crate) actors: Option<Vec<Actor>>,
    pub(crate) choices: Option<Vec<Choice>>,
}

#[cfg(test)]
mod test {
    use bevy::prelude::default;

    use crate::{
        loader::build_screenplay,
        types::{
            ActorAction, ActorActionKind, ActorOrPlayerActionJSON, PlayerAction, RawScreenplay,
        },
    };

    use super::*;

    // 'current_text' tests
    #[test]
    fn current_text() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                id: 1,
                text: Some("Hello".to_string()),
                next: None,
                start: Some(true),
                ..default()
            })],
        };

        let play = build_screenplay(raw_sp).unwrap();
        assert_eq!(play.text(), "Hello");
    }

    // 'next_line' tests
    #[test]
    fn next_no_next_err() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                id: 1,
                text: Some("Hello".to_string()),
                start: Some(true),
                ..default()
            })],
        };

        let mut play = build_screenplay(raw_sp).unwrap();
        assert_eq!(
            play.next_action().err(),
            Some(ScreenplayError::NoNextAction)
        );
    }

    #[test]
    fn next_choices_not_handled_err() {
        let raw_sp = RawScreenplay {
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

        let mut play = build_screenplay(raw_sp).unwrap();
        assert_eq!(
            play.next_action().err(),
            Some(ScreenplayError::ChoicesNotHandled)
        );
    }

    #[test]
    fn next_action() {
        let raw_sp = RawScreenplay {
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

        let mut play = build_screenplay(raw_sp).unwrap();
        assert_eq!(play.text(), "Hello");
        assert!(play.next_action().is_ok());
        assert_eq!(play.text(), "Whatup");
    }

    // 'choices' tests
    #[test]
    fn choices_no_choices_err() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                id: 1,
                start: Some(true),
                ..default()
            })],
        };

        let play = build_screenplay(raw_sp).unwrap();
        assert_eq!(play.choices().err(), Some(ScreenplayError::NoChoices));
    }

    #[test]
    fn choices() {
        let raw_sp = RawScreenplay {
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

        let play = build_screenplay(raw_sp).unwrap();

        assert_eq!(play.choices().unwrap()[0].next, 2);
        assert_eq!(play.choices().unwrap()[1].next, 3);
        assert_eq!(play.choices().unwrap()[0].text, "Choice 1");
        assert_eq!(play.choices().unwrap()[1].text, "Choice 2");
    }

    // 'jump_to' tests
    #[test]
    fn jump_to_no_action_err() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                id: 1,
                start: Some(true),
                ..default()
            })],
        };

        let mut play = build_screenplay(raw_sp).unwrap();
        assert_eq!(play.jump_to(2).err(), Some(ScreenplayError::WrongJump(2)));
    }

    #[test]
    fn jump_to() {
        let raw_sp = RawScreenplay {
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

        let mut play = build_screenplay(raw_sp).unwrap();
        assert!(play.jump_to(2).is_ok());
        assert_eq!(play.text(), "I'm number 2");
    }

    // 'current_first_actor' tests
    #[test]
    fn first_actor_none() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                start: Some(true),
                ..default()
            })],
        };

        let play = build_screenplay(raw_sp).unwrap();
        assert!(play.first_actor().is_none());
    }

    #[test]
    fn first_actor() {
        let mut actors = HashMap::new();
        actors.insert(
            "bob".to_string(),
            Actor {
                name: "Bob".to_string(),
                asset: "bob.png".to_string(),
            },
        );

        let raw_sp = RawScreenplay {
            actors,
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                actors: vec!["bob".to_string()],
                start: Some(true),
                ..default()
            })],
        };

        let play = build_screenplay(raw_sp).unwrap();
        assert!(play.first_actor().is_some());
    }

    #[test]
    fn current_actors() {
        let mut actors = HashMap::new();
        actors.insert(
            "bob".to_string(),
            Actor {
                name: "Bob".to_string(),
                asset: "bob.png".to_string(),
            },
        );
        actors.insert(
            "alice".to_string(),
            Actor {
                name: "alice".to_string(),
                asset: "alice".to_string(),
            },
        );

        let raw_sp = RawScreenplay {
            actors,
            script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
                actors: vec!["bob".to_string(), "alice".to_string()],
                start: Some(true),
                ..default()
            })],
        };

        let play = build_screenplay(raw_sp).unwrap();
        assert_eq!(play.current_actors().unwrap().len(), 2);
    }

    #[test]
    fn at_player_action() {
        let raw_sp = RawScreenplay {
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

        let play = build_screenplay(raw_sp).unwrap();
        assert!(play.at_player_action());
    }

    #[test]
    fn action_kind_player() {
        let raw_sp = RawScreenplay {
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

        let play = build_screenplay(raw_sp).unwrap();
        assert_eq!(play.action_kind(), ActionKind::PlayerChoice);
    }

    #[test]
    fn action_kind_actor() {
        let raw_sp = RawScreenplay {
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

        let mut play = build_screenplay(raw_sp).unwrap();
        assert_eq!(play.action_kind(), ActionKind::ActorTalk);
        play.next_action().unwrap();
        assert_eq!(play.action_kind(), ActionKind::ActorEnter);
    }
}
