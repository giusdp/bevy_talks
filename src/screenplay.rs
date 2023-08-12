//! The main module of the crate. It contains the Screenplay struct and its
//! builder.
use bevy::prelude::{Commands, Component, Entity};
use petgraph::visit::EdgeRef;
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex};

use crate::prelude::{NextActionError, ScreenplayBuilder};

/// A screenplay is a directed graph of actions.
/// The nodes of the graph are the actions, which are
/// bevy entities with specific [`bevy_talks`] components.
/// The Screenplay struct keeps track of the current action
/// and provides functions to move to the next action.
#[derive(Debug, Component)]
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
}

// Public API
impl Screenplay {
    /// Create a new [`ScreenplayBuilder`] with default values.
    pub fn builder() -> ScreenplayBuilder {
        ScreenplayBuilder::default()
    }

    /// Move to the next action. Returns an error if the current action
    /// has no next action.
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
}

/// An `ActionNode` is an entity that represents an action in a screenplay.
///
/// Action nodes are used to define the actions that characters perform in a screenplay. They can be
/// linked together to create a sequence of actions that make up a scene or an entire screenplay.
pub(crate) type ActionNode = Entity;

/// A component that indicates that the entity is a "talk".
/// It contains only the text to be displayed, without any
/// information about the speaker.
/// For example, it can be used to display text said by a narrator
/// and no speaker name is needed.
/// Use [`SpeakerTalkComp`] to have text and speaker.
#[derive(Component)]
pub struct TalkComp {
    /// The text to be displayed.
    pub text: String,
}

/// Spawn a new entity with a [`TalkComp`] component attached.
pub fn new_talk(commands: &mut Commands, text: String) -> ActionNode {
    let c = commands.spawn(TalkComp { text });
    c.id()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn next_no_next_err() {
        let mut sp: Screenplay = ScreenplayBuilder::new()
            .add_action_node(ActionNode::PLACEHOLDER)
            .build();

        assert_eq!(sp.next_action().err(), Some(NextActionError::NoNextAction));
    }

    #[test]
    fn next_action() {
        let mut sp: Screenplay = ScreenplayBuilder::new()
            .add_action_node(ActionNode::PLACEHOLDER)
            .add_action_node(ActionNode::PLACEHOLDER)
            .build();

        assert!(sp.next_action().is_ok());
    }
}
