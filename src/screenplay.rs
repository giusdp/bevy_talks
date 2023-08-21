//! The main module of the crate. It contains the Screenplay struct and its
//! builder.
use bevy::prelude::Component;
use bevy::utils::HashMap;
use petgraph::visit::EdgeRef;
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex};

use crate::prelude::{
    ActionId, ActionKind, ActionNode, Actor, Choice, NextActionError, ScreenplayBuilder,
};

/// A screenplay is a directed graph of actions.
/// The nodes of the graph are the actions, which are
/// bevy entities with specific [`bevy_screenplay`] components.
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

    /// Returns the kind of the current action.
    pub fn action_kind(&self) -> ActionKind {
        self.graph[self.current_node].kind.clone()
    }

    /// Move to the next action. Returns an error if the current action has no next action.
    pub fn next_action(&mut self) -> Result<(), NextActionError> {
        if self.graph[self.current_node].kind == ActionKind::Choice {
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

    /// Returns an option containing the choices for the current action. Returns None if the action kind is not Choice.
    pub fn choices(&self) -> Option<Vec<Choice>> {
        let cnode = &self.graph[self.current_node];
        if cnode.kind != ActionKind::Choice {
            return None;
        }
        cnode.choices.clone()
    }

    /// Jumps to a specific action node in the screenplay.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the action node to jump to.
    ///
    /// # Errors
    ///
    /// Returns a `NextActionError::WrongJump` error if the specified ID is not found in the action node map.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the jump was successful.
    pub fn jump_to(&mut self, id: i32) -> Result<(), NextActionError> {
        let idx = self
            .action_node_map
            .get(&id)
            .ok_or(NextActionError::WrongJump(id))?;

        self.current_node = *idx;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::default;

    use crate::prelude::{Actor, RawScreenplay, ScriptAction};

    use super::*;

    #[test]
    fn choice_action_but_no_choices() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ScriptAction {
                action: ActionKind::Choice,
                ..default()
            }],
        };

        let sp = ScreenplayBuilder::raw_build(&raw_sp).unwrap();
        assert!(sp.choices().is_none());
    }

    #[test]
    fn choices() {
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
                ScriptAction { id: 2, ..default() },
                ScriptAction { id: 3, ..default() },
            ],
        };

        let sp = ScreenplayBuilder::raw_build(&raw_sp).unwrap();

        assert_eq!(sp.choices().unwrap()[0].next, 2);
        assert_eq!(sp.choices().unwrap()[0].text, "Choice 1");
        assert_eq!(sp.choices().unwrap()[1].next, 3);
        assert_eq!(sp.choices().unwrap()[1].text, "Choice 2");
    }

    #[test]
    fn jump_to_no_action_err() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![ScriptAction { ..default() }],
        };

        let mut sp = ScreenplayBuilder::raw_build(&raw_sp).unwrap();
        assert_eq!(sp.jump_to(2).err(), Some(NextActionError::WrongJump(2)));
    }

    #[test]
    fn jump_to() {
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
                    text: Some("I'm number 2".to_string()),
                    next: Some(3),
                    ..default()
                },
                ScriptAction { id: 3, ..default() },
            ],
        };

        let mut sp = ScreenplayBuilder::raw_build(&raw_sp).unwrap();
        assert!(sp.jump_to(2).is_ok());
        assert_eq!(sp.text(), "I'm number 2");
    }

    #[test]
    fn action_kind_returns_current_kind() {
        let raw_sp = RawScreenplay {
            actors: default(),
            script: vec![
                ScriptAction { ..default() },
                ScriptAction {
                    id: 2,
                    action: ActionKind::Enter,
                    ..default()
                },
            ],
        };

        let mut sp = ScreenplayBuilder::raw_build(&raw_sp).unwrap();
        assert_eq!(sp.action_kind(), ActionKind::Talk);
        sp.next_action().unwrap();
        assert_eq!(sp.action_kind(), ActionKind::Enter);
    }

    #[test]
    fn actors_returns_array_of_current_actors() {
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
        let raw = RawScreenplay {
            actors,
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

    #[test]
    fn next_action_choices_not_handled_err() {
        let raw = RawScreenplay {
            actors: default(),
            script: vec![
                ScriptAction {
                    choices: Some(vec![Choice {
                        text: "Whatup".to_string(),
                        next: 2,
                    }]),
                    ..default()
                },
                ScriptAction { id: 2, ..default() },
            ],
        };

        let sp = ScreenplayBuilder::raw_build(&raw);
        assert!(sp.is_ok());
        assert_eq!(
            sp.unwrap().next_action().err(),
            Some(NextActionError::ChoicesNotHandled)
        );
    }

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
