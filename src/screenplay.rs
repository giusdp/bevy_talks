//! The main module of the crate. It contains the Screenplay struct and its
//! builder.
use bevy::prelude::Component;
use bevy::utils::HashMap;
use petgraph::visit::EdgeRef;
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex};

use crate::prelude::{ActionId, ActionNode, Actor, NextActionError, ScreenplayBuilder};

/// A screenplay is a directed graph of actions.
/// The nodes of the graph are the actions, which are
/// bevy entities with specific [`bevy_talks`] components.
/// The Screenplay struct keeps track of the current action
/// and provides functions to move to the next action.
#[derive(Debug, Component, Default)]
pub struct Screenplay {
    /// The graph that represents the screenplay.
    ///
    /// This field is a directed graph that represents the structure of the screenplay. Each node in the
    /// graph represents an action in the screenplay, and each edge represents a transition between
    /// actions.
    pub(crate) graph: DiGraph<ActionNode, ()>,

    /// The index of the current node in the screenplay graph.
    ///
    /// This field is used to keep track of the current node in the screenplay graph. It is updated
    /// whenever the [`next_action`] method is called.
    pub(crate) current_node: NodeIndex,

    /// The map tracking the action ids to the node indexes in the graph.
    #[allow(dead_code)]
    pub(crate) action_node_map: HashMap<ActionId, NodeIndex>,
}

// Public API
impl Screenplay {
    /// Create a new [`ScreenplayBuilder`] with default values.
    pub fn builder() -> ScreenplayBuilder {
        ScreenplayBuilder::default()
    }

    /// Move to the next action. Returns an error if the current action has no next action.
    pub fn next_action(&mut self) -> Result<(), NextActionError> {
        if self.graph.node_weight(self.current_node).is_some() {
            // retrieve the next edge
            let edge_ref = self
                .graph
                .edges(self.current_node)
                .next()
                .ok_or(NextActionError::NoNextAction)?;

            self.current_node = edge_ref.target();
        }
        Ok(())
    }

    /// Returns the current action's text. Returns an empty string if the current action has no text.
    pub fn text(&self) -> &str {
        match &self.graph[self.current_node].text {
            Some(t) => t,
            None => "",
        }
    }

    /// Returns the current action's actors. Returns an empty vector if the current action has no actors.
    pub fn actors(&self) -> Vec<Actor> {
        self.graph[self.current_node].actors.clone()
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::default;

    use crate::{
        prelude::{Actor, RawScreenplay, ScriptAction},
        tests::test_actors_map,
    };

    use super::*;

    // ______________------------------------____________________----------____---___---___-

    // // 'choices' tests
    // #[test]
    // fn choices_no_choices_err() {
    //     let raw_sp = RawScreenplay {
    //         actors: default(),
    //         script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
    //             id: 1,
    //             start: Some(true),
    //             ..default()
    //         })],
    //     };

    //     let play = build_screenplay(raw_sp).unwrap();
    //     assert_eq!(play.choices().err(), Some(ChoicesError::NotAChoiceAction));
    // }

    // #[test]
    // fn choices() {
    //     let raw_sp = RawScreenplay {
    //         actors: default(),
    //         script: vec![
    //             ActorOrPlayerActionJSON::Player(PlayerAction {
    //                 id: 1,
    //                 choices: vec![
    //                     Choice {
    //                         text: "Choice 1".to_string(),
    //                         next: 2,
    //                     },
    //                     Choice {
    //                         text: "Choice 2".to_string(),
    //                         next: 3,
    //                     },
    //                 ],
    //                 start: Some(true),
    //             }),
    //             ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
    //             ActorOrPlayerActionJSON::Actor(ActorAction { id: 3, ..default() }),
    //         ],
    //     };

    //     let play = build_screenplay(raw_sp).unwrap();

    //     assert_eq!(play.choices().unwrap()[0].next, 2);
    //     assert_eq!(play.choices().unwrap()[1].next, 3);
    //     assert_eq!(play.choices().unwrap()[0].text, "Choice 1");
    //     assert_eq!(play.choices().unwrap()[1].text, "Choice 2");
    // }

    // // 'jump_to' tests
    // #[test]
    // fn jump_to_no_action_err() {
    //     let raw_sp = RawScreenplay {
    //         actors: default(),
    //         script: vec![ActorOrPlayerActionJSON::Actor(ActorAction {
    //             id: 1,
    //             start: Some(true),
    //             ..default()
    //         })],
    //     };

    //     let mut play = build_screenplay(raw_sp).unwrap();
    //     assert_eq!(play.jump_to(2).err(), Some(ChoicesError::WrongId(2)));
    // }

    // #[test]
    // fn jump_to() {
    //     let raw_sp = RawScreenplay {
    //         actors: default(),
    //         script: vec![
    //             ActorOrPlayerActionJSON::Player(PlayerAction {
    //                 id: 1,
    //                 choices: vec![
    //                     Choice {
    //                         text: "Choice 1".to_string(),
    //                         next: 2,
    //                     },
    //                     Choice {
    //                         text: "Choice 2".to_string(),
    //                         next: 3,
    //                     },
    //                 ],
    //                 start: Some(true),
    //             }),
    //             ActorOrPlayerActionJSON::Actor(ActorAction {
    //                 id: 2,
    //                 text: Some("I'm number 2".to_string()),
    //                 next: Some(3),
    //                 ..default()
    //             }),
    //             ActorOrPlayerActionJSON::Actor(ActorAction { id: 3, ..default() }),
    //         ],
    //     };

    //     let mut play = build_screenplay(raw_sp).unwrap();
    //     assert!(play.jump_to(2).is_ok());
    //     assert_eq!(play.text(), "I'm number 2");
    // }

    // #[test]
    // fn action_kind_actor() {
    //     let raw_sp = RawScreenplay {
    //         actors: default(),
    //         script: vec![
    //             ActorOrPlayerActionJSON::Actor(ActorAction {
    //                 id: 1,
    //                 start: Some(true),
    //                 ..default()
    //             }),
    //             ActorOrPlayerActionJSON::Actor(ActorAction {
    //                 id: 2,
    //                 action: ActorActionKind::Enter,
    //                 ..default()
    //             }),
    //         ],
    //     };

    //     let mut play = build_screenplay(raw_sp).unwrap();
    //     assert_eq!(play.action_kind(), ActionKind::ActorTalk);
    //     play.next_action().unwrap();
    //     assert_eq!(play.action_kind(), ActionKind::ActorEnter);
    // }
    // ______________------------------------____________________----------____---___---___-

    #[test]
    fn actors_returns_array_of_current_actors() {
        let mut actor_map = test_actors_map("bob".to_owned());
        actor_map.insert("alice".to_owned(), Actor::default());

        let raw = RawScreenplay {
            actors: actor_map,
            script: vec![ScriptAction {
                actors: vec!["bob".to_string(), "alice".to_string()],
                ..default()
            }],
        };

        let sp = ScreenplayBuilder::raw_build(&raw);
        assert!(sp.is_ok());
        assert_eq!(sp.unwrap().actors().len(), 2);
    }

    #[test]
    fn text_returns_current_action_text() {
        let raw = RawScreenplay {
            actors: default(),
            script: vec![ScriptAction {
                text: Some(String::from("Hello")),
                ..default()
            }],
        };
        let sp = ScreenplayBuilder::raw_build(&raw);
        assert!(sp.is_ok());
        assert_eq!(sp.unwrap().text(), "Hello");
    }

    #[test]
    fn text_returns_empty_when_no_text() {
        let raw = RawScreenplay {
            actors: default(),
            script: vec![ScriptAction { ..default() }],
        };

        let sp = ScreenplayBuilder::raw_build(&raw);
        assert!(sp.is_ok());
        assert_eq!(sp.unwrap().text(), "");
    }

    #[test]
    fn next_action_with_no_next() {
        let raw = RawScreenplay {
            actors: default(),
            script: vec![ScriptAction { ..default() }],
        };

        let sp = ScreenplayBuilder::raw_build(&raw);
        assert!(sp.is_ok());
        assert_eq!(
            sp.unwrap().next_action().err(),
            Some(NextActionError::NoNextAction)
        );
    }

    // #[test]
    // fn next_choices_not_handled_err() {
    //     let raw_sp = RawScreenplay {
    //         actors: default(),
    //         script: vec![
    //             ActorOrPlayerActionJSON::Player(PlayerAction {
    //                 id: 1,
    //                 choices: vec![Choice {
    //                     text: "Whatup".to_string(),
    //                     next: 2,
    //                 }],
    //                 start: Some(true),
    //                 ..default()
    //             }),
    //             ActorOrPlayerActionJSON::Actor(ActorAction { id: 2, ..default() }),
    //         ],
    //     };

    //     let mut play = build_screenplay(raw_sp).unwrap();
    //     assert_eq!(
    //         play.next_action().err(),
    //         Some(NextRequestError::ChoicesNotHandled)
    //     );
    // }

    #[test]
    fn next_action_success() {
        let raw = RawScreenplay {
            actors: default(),
            script: vec![
                ScriptAction { ..default() },
                ScriptAction { id: 2, ..default() },
            ],
        };
        let sp = ScreenplayBuilder::raw_build(&raw);
        assert!(sp.is_ok());
        assert!(sp.unwrap().next_action().is_ok());
    }
}
