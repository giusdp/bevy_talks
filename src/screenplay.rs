//! The main module of the crate. It contains the Screenplay struct and its
//! builder.
use bevy::prelude::Component;
use bevy::utils::HashMap;
use petgraph::visit::EdgeRef;
use petgraph::{prelude::DiGraph, stable_graph::NodeIndex};

use crate::prelude::{ActionId, NextActionError, ScreenplayBuilder, ScriptAction};

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
    pub(crate) graph: DiGraph<ScriptAction, ()>,

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

#[cfg(test)]
mod test {

    use bevy::prelude::default;

    use super::*;

    #[test]
    fn next_no_next_err() {
        let res = ScreenplayBuilder::new()
            .add_action_node(ScriptAction { ..default() })
            .build();

        assert!(res.is_ok());
        let mut sp = res.unwrap();
        assert_eq!(sp.next_action().err(), Some(NextActionError::NoNextAction));
    }

    #[test]
    fn next_action() {
        // TODO: this should fail after adding validation dynamic nodes on the builder
        let res = ScreenplayBuilder::new()
            .add_action_node(ScriptAction { ..default() })
            .add_action_node(ScriptAction { ..default() })
            .build();

        assert!(res.is_ok());
        let mut sp = res.unwrap();
        assert!(sp.next_action().is_ok());
    }
}
