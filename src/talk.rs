//! The main types for a Talk.

use aery::prelude::*;
use bevy::prelude::*;

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

/// The Talk component. It's used to identify the parent entity of dialogue entity graphs.
/// Build entities with Talk components via the [`TalkBuilder`] to correctly setup the dialogue graph.
#[derive(Component, Default, Debug)]
pub struct Talk {
    /// The text of the current node (if not a Talk node it's empty)
    pub current_text: String,
    /// The kind of the current node
    pub current_kind: NodeKind, // TODO: add a Start node kind?
    /// The actor(s) name of the current node
    pub current_actors: Vec<String>,
    /// The choices of the current node (if not a Choice node it's empty)
    pub current_choices: Vec<Choice>,
}

/// Marker component for the current node in a Talk.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub(crate) struct CurrentNode;

/// A component that marks a node as the start of the dialogue graph.
#[derive(Component)]
pub struct StartTalk;

/// An enumeration of the different kinds of actions that can be performed in a Talk.
#[derive(Component, Debug, Default, Clone, Hash, Eq, PartialEq, serde::Deserialize)]
pub enum NodeKind {
    #[default]
    /// A talk action, where a character speaks dialogue.
    Talk,
    /// A choice action, where the user is presented with a choice.
    Choice,
    /// An enter action, where a character enters a scene.
    Join,
    /// An exit action, where a character exits a scene.
    Leave,
}

/// The components that define a Talk node in the dialogue graph.
/// Use `TalkNodeBundle::new()` to create a new `TalkNodeBundle`.
#[derive(Bundle, Default)]
pub struct TalkNodeBundle {
    /// The kind of action that the node performs. This should be `NodeKind::Talk` as the TalkNodeBundle is used to create a talk node.
    pub kind: NodeKind,
    /// The text to be displayed by the talk node.
    pub text: TalkText,
}

impl TalkNodeBundle {
    /// Creates a new `TalkNodeBundle` with the specified `text`.
    /// The node kind is set to `NodeKind::Talk`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use bevy_talks::prelude::*;
    ///
    /// let text = "Hello, world!".to_string();
    /// let actors = vec!["Alice".to_string(), "Bob".to_string()];
    /// let bundle = TalkNodeBundle::new(text.clone());
    ///
    /// assert_eq!(bundle.kind, NodeKind::Talk);
    /// assert_eq!(bundle.text.0, text);
    /// ```
    pub fn new(text: String) -> Self {
        Self {
            kind: NodeKind::Talk,
            text: TalkText(text),
        }
    }
}

/// The components that defines a `Choice` node in the dialogue graph.
/// Use `TalkNodeBundle::new()` to create a new `TalkNodeBundle`.
#[derive(Bundle, Default)]
pub struct ChoiceNodeBundle {
    /// Should be `NodeKind::Choice` for the choice node.
    pub kind: NodeKind,
    /// The choices of the node.
    pub choices: Choices,
}

impl ChoiceNodeBundle {
    /// Creates a new `ChoiceNodeBundle`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use bevy_talks::prelude::*;
    /// use bevy::prelude::*;
    ///
    /// let mut world = World::default();
    /// let e = world.spawn_empty().id();
    ///
    /// let bundle = ChoiceNodeBundle::new(vec![Choice::new("Choice 1", e)]);
    ///
    /// assert_eq!(bundle.kind, NodeKind::Choice);
    /// assert_eq!(bundle.choices.0[0].text, "Choice 1".to_string());
    /// ```
    pub fn new(cs: Vec<Choice>) -> Self {
        Self {
            kind: NodeKind::Choice,
            choices: Choices(cs),
        }
    }
}

/// The text component to be displayed from a Talk Node.
#[derive(Component, Default, Debug)]
pub struct TalkText(pub String);

/// The choices texts component to be displayed from a Choice Node.
#[derive(Component, Default, Debug)]
pub struct Choices(pub Vec<Choice>);

/// The text and next entity of a choice.
#[derive(Debug, Clone)]
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
