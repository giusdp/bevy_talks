//! The core Talk structs and builder.
use bevy::prelude::{Component, Handle, Image};
use petgraph::visit::EdgeRef;
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex};

use crate::builderv2::TalkBuilder;
use crate::{builder, prelude::*};

/// An action node in a Talk.
#[derive(Debug, Default)]
pub(crate) struct TalkNode {
    /// The kind of action.
    pub(crate) kind: TalkNodeKind,
    /// The text of the action.
    pub(crate) text: String,
    /// The actors involved in the action.
    pub(crate) actors: Vec<Actor>,
    /// The choices available after the action.
    pub(crate) choices: Vec<Choice>,
}

/// A struct that represents an actor in a Talk.
///
/// This struct is used to define an actor in a Talk. It contains the ID of the actor, the
/// name of the character that the actor plays, and an optional asset that represents the actor's
/// appearance or voice.
#[derive(Debug, Clone, Default)]
pub struct Actor {
    /// The name of the character that the actor plays.
    pub name: String,
    /// An optional asset for the actor.
    pub asset: Option<Handle<Image>>,
}

/// An enumeration of the different kinds of actions that can be performed in a Talk.
#[derive(Debug, Default, Clone, PartialEq)]
pub enum TalkNodeKind {
    /// A talk action, where a character speaks dialogue.
    #[default]
    Talk,
    /// An enter action, where a character enters a scene.
    Join,
    /// An exit action, where a character exits a scene.
    Leave,
    /// A choice action, where the user is presented with a choice.
    Choice,
}

/// A Talk is a directed graph.
/// A node is an [`Entity`] with with specific [`bevy_talks`] components (the [`TalkNodeBundle`]).
/// The Talk component keeps track of the current node and provides functions to move to the next one.
#[derive(Debug, Component, Default)]
pub struct Talk {
    /// The graph that represents the Talk.
    ///
    /// This field is a directed graph that represents the structure of the Talk. Each node in the
    /// graph represents an action in the Talk, and each edge represents a transition between
    /// actions.
    pub(crate) graph: DiGraph<TalkNode, ()>,

    /// The index of the current node in the Talk graph.
    ///
    /// This field is used to keep track of the current node in the Talk graph. It is updated
    /// whenever the [`next_action`] method is called.
    pub(crate) current_node: NodeIndex,

    /// The index of the start node in the Talk graph.
    pub(crate) start_node: NodeIndex,
}

// API
impl Talk {
    /// Creates a new `TalkBuilder` instance.
    pub fn builder() -> TalkBuilder {
        TalkBuilder::default()
    }

    /// Builds a `Talk` instance from a `RawTalk` instance.
    ///
    /// # Arguments
    ///
    /// * `raw_talk` - A reference to the `RawTalk` instance to build the `Talk` from.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the built `Talk` instance if successful,
    /// or a `BuildTalkError` if an error occurred during the build process.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_talks::prelude::*;
    ///
    /// let raw_talk = RawTalk::default();
    /// let talk_res = Talk::build(&raw_talk);
    ///
    /// assert!(talk_res.is_ok());
    /// ```
    pub fn build(raw_talk: &RawTalk) -> Result<Talk, BuildTalkError> {
        builder::build(raw_talk)
    }

    /// Sets the current node of the `Talk` instance to the start node.
    pub(crate) fn start(&mut self) {
        self.current_node = self.start_node;
    }

    /// Returns the `TalkNodeKind` of the current action.
    pub(crate) fn node_kind(&self) -> TalkNodeKind {
        self.graph[self.current_node].kind.clone()
    }

    /// Move to the next action. Returns an error if the current action has no next action.
    pub(crate) fn next_action(&mut self) -> Result<(), NextActionError> {
        if self.graph[self.current_node].kind == TalkNodeKind::Choice {
            return Err(NextActionError::ChoicesNotHandled);
        }
        // retrieve the next edge
        let edge_ref = self
            .graph
            .edges(self.current_node)
            .next()
            .ok_or(NextActionError::NoNextAction)?;

        self.current_node = edge_ref.target();
        Ok(())
    }

    /// Returns the text associated with the current action, or an empty string if there is none.
    pub(crate) fn text(&self) -> &str {
        if self.graph.node_count() == 0 {
            return "";
        }
        &self.graph[self.current_node].text
    }

    /// Returns a vector of the actors associated with the current action.
    pub(crate) fn action_actors(&self) -> Vec<Actor> {
        self.graph[self.current_node].actors.clone()
    }

    /// Returns a vector of the choices associated with the current action, or `None` if the current action is not a choice.
    pub(crate) fn choices(&self) -> Vec<Choice> {
        let cnode = &self.graph[self.current_node];
        if cnode.kind != TalkNodeKind::Choice {
            return vec![];
        }
        cnode.choices.clone()
    }

    /// Jumps to a specific action node in the Talk.
    ///
    /// # Arguments
    ///
    /// * `idx` - The `NodeIndex` of the action node to jump to.
    ///
    /// # Errors
    ///
    /// Returns a `NextActionError::WrongJump` error if the specified idx is not a node in the graph.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the jump was successful.
    pub(crate) fn jump_to(&mut self, idx: NodeIndex) -> Result<(), NextActionError> {
        if !self.graph.node_indices().any(|i| i == idx) {
            return Err(NextActionError::WrongJump(idx.index()));
        }

        self.current_node = idx;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::default;

    use crate::prelude::{RawAction, RawChoice};

    use super::*;

    #[test]
    fn choices_no_choice_node() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction { ..default() }],
        };

        let sp = Talk::build(&raw_sp).unwrap();
        assert_eq!(sp.choices().len(), 0);
    }

    #[test]
    fn choices() {
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
                RawAction { id: 2, ..default() },
                RawAction { id: 3, ..default() },
            ],
        };

        let sp = Talk::build(&raw_sp).unwrap();
        assert_eq!(sp.choices()[0].next, 1.into());
        assert_eq!(sp.choices()[0].text, "Choice 1");
        assert_eq!(sp.choices()[1].next, 2.into());
        assert_eq!(sp.choices()[1].text, "Choice 2");
    }

    #[test]
    fn action_actors() {
        let raw_sp = RawTalk {
            actors: vec![
                RawActor {
                    id: String::from("bob"),
                    name: String::from("Bob"),
                    ..Default::default()
                },
                RawActor {
                    id: String::from("alice"),
                    name: String::from("Alice"),
                    ..Default::default()
                },
            ],
            script: vec![RawAction {
                actors: vec![String::from("bob")],
                ..Default::default()
            }],
        };
        let sp = Talk::build(&raw_sp).unwrap();
        let actors = sp.action_actors();
        assert_eq!(actors[0].name, "Bob");
    }

    #[test]
    fn text_no_talk_node() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction {
                kind: TalkNodeKind::Join,
                ..default()
            }],
        };

        let talk = Talk::build(&raw_sp);
        assert!(talk.is_ok());
        assert_eq!(talk.unwrap().text(), "");
    }

    #[test]
    fn text() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction {
                text: Some("Hello".to_string()),
                ..default()
            }],
        };

        let talk = Talk::build(&raw_sp);
        assert!(talk.is_ok());
        assert_eq!(talk.unwrap().text(), "Hello");
    }

    #[test]
    fn node_kind() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction { ..default() }],
        };

        let talk = Talk::build(&raw_sp).unwrap();
        assert_eq!(talk.node_kind(), TalkNodeKind::Talk);
    }

    #[test]
    fn start_sets_current_node() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction { ..default() }, RawAction { id: 2, ..default() }],
        };

        let mut sp = Talk::build(&raw_sp).unwrap();
        sp.start();
        assert_eq!(sp.current_node.index(), 0);
    }

    #[test]
    fn start_resets_current_node() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction { ..default() }, RawAction { id: 2, ..default() }],
        };

        let mut sp = Talk::build(&raw_sp).unwrap();
        sp.current_node = 1.into();
        sp.start();
        assert_eq!(sp.current_node.index(), 0);
    }

    #[test]
    fn jump_to_no_action_err() {
        let raw_sp = RawTalk {
            actors: default(),
            script: vec![RawAction { ..default() }],
        };

        let mut sp = Talk::build(&raw_sp).unwrap();
        assert_eq!(
            sp.jump_to(2.into()).err(),
            Some(NextActionError::WrongJump(2))
        );
    }

    #[test]
    fn jump_to() {
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
                    text: Some("I'm number 2".to_string()),
                    next: Some(3),
                    ..default()
                },
                RawAction { id: 3, ..default() },
            ],
        };

        let mut sp = Talk::build(&raw_sp).unwrap();
        assert!(sp.jump_to(1.into()).is_ok());
        assert_eq!(sp.text(), "I'm number 2");
    }

    #[test]
    fn next_action_with_no_next() {
        let raw = RawTalk {
            actors: default(),
            script: vec![RawAction { ..default() }],
        };

        let sp = Talk::build(&raw);
        assert!(sp.is_ok());
        assert_eq!(
            sp.unwrap().next_action().err(),
            Some(NextActionError::NoNextAction)
        );
    }

    #[test]
    fn next_action_choices_not_handled_err() {
        let raw = RawTalk {
            actors: default(),
            script: vec![
                RawAction {
                    choices: Some(vec![RawChoice {
                        text: "Whatup".to_string(),
                        next: 2,
                    }]),
                    ..default()
                },
                RawAction { id: 2, ..default() },
            ],
        };

        let sp = Talk::build(&raw);
        assert!(sp.is_ok());
        assert_eq!(
            sp.unwrap().next_action().err(),
            Some(NextActionError::ChoicesNotHandled)
        );
    }

    #[test]
    fn next_action_success() {
        let raw = RawTalk {
            actors: default(),
            script: vec![RawAction { ..default() }, RawAction { id: 2, ..default() }],
        };
        let sp = Talk::build(&raw);
        assert!(sp.is_ok());
        assert!(sp.unwrap().next_action().is_ok());
    }
}
