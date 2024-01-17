//! The main types for a Talk.

use aery::prelude::*;
use bevy::prelude::*;

use crate::builder::TalkBuilder;

/// The relationship of the dialogue nodes.
/// It needs to be Poly because the choice nodes can have multiple branches.
#[derive(Relation)]
#[aery(Recursive, Poly)]
pub struct FollowedBy;

/// The relationship between dialogue nodes and actors.
/// It needs to be Poly because the nodes can have multiple actors (and vice-versa).
#[derive(Relation)]
#[aery(Recursive, Poly)]
pub struct PerformedBy;

/// Market component used to identify the parent entity of dialogue entity graphs.
/// Build entities with Talk components via the [`TalkBuilder`] to correctly setup the dialogue graph.
#[derive(Component, Default, Debug)]
pub struct Talk {
    /// Helper field to know if the talk has started.
    /// You can also check if the child `CurrentNode` has the `StartNode` component.
    pub has_started: bool,
}

impl Talk {
    /// Create a default [`TalkBuilder`].
    pub fn builder() -> TalkBuilder {
        TalkBuilder::default()
    }
}

/// Marker component for the current node in a Talk.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct CurrentNode;

/// Mark a dialogue node as a starting node.
#[derive(Component, Default, Debug)]
pub struct StartNode;

/// Mark a dialogue node as an end node.
#[derive(Component, Default, Debug)]
pub struct EndNode;

/// Component to mark a dialogue node as a text node containing some text.
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component)]
pub struct TextNode(pub String);

/// Component to mark a dialogue node as a choice node containing some choices.
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component)]
pub struct ChoiceNode(pub Vec<Choice>);

/// Component to mark a dialogue node as a join node.
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component)]
pub struct JoinNode;

/// Component to mark a dialogue node as a leave node.
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component)]
pub struct LeaveNode;

/// The text and next entity of a choice.
#[derive(Debug, Reflect, Clone)]
pub struct Choice {
    /// The text of the choice.
    pub text: String,
    /// The next entity to go to if the choice is selected.
    pub next: Entity,
}

impl Choice {
    /// Creates a new `Choice` with the given text and next entity.
    ///
    /// # Example
    /// ```rust
    /// use bevy_talks::prelude::*;
    /// use bevy::prelude::*;
    ///
    /// let mut world = World::default();
    /// let e = world.spawn_empty().id();
    ///
    /// let choice = Choice::new("Choice 1", e);
    /// assert_eq!(choice.text, "Choice 1".to_string());
    /// assert_eq!(choice.next, e);
    /// ```
    pub fn new(text: impl Into<String>, next: Entity) -> Self {
        Self {
            text: text.into(),
            next,
        }
    }
}
