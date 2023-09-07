//! Talks module

use bevy::prelude::{Handle, Image};
use petgraph::stable_graph::NodeIndex;

pub mod components;
pub mod errors;
pub mod raw_talk;
pub mod talk;
/// An action node in a Talk.
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct TalkNode {
    /// The kind of action.
    pub(crate) kind: TalkNodeKind,
    /// The text of the action.
    pub(crate) text: Option<String>,
    /// The actors involved in the action.
    pub(crate) actors: Vec<Actor>,
    /// The choices available after the action.
    pub(crate) choices: Option<Vec<Choice>>,
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
    /// An optional asset that represents the actor's appearance or voice.
    pub asset: Option<Handle<Image>>,
}

/// An enumeration of the different kinds of actions that can be performed in a Talk.
///
/// This enumeration is used to define the different kinds of actions that can be performed in a
/// Talk. Each variant of the enumeration represents a different kind of action, such as
/// talking, entering, exiting, or making a choice.
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

/// A struct that represents a choice in a Talk.
///
/// This struct is used to define a choice in a Talk. It contains the text of the choice and
/// the ID of the next action to perform if the choice is selected.
#[derive(Debug, Clone)]
pub struct Choice {
    /// The text of the choice.
    pub text: String,
    /// The ID of the next action to perform if the choice is selected.
    pub next: NodeIndex,
}
