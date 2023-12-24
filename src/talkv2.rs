//! The core Talk structs and builder.

use bevy::prelude::*;
use petgraph::graph::NodeIndex;

/// A bundle of component that defines a Talk node in the dialogue graph.
/// Use `TalkNodeBundle::new()` to create a new `TalkNodeBundle`.
#[derive(Bundle, Default)]
pub struct TalkNodeBundle {
    /// The kind of action that the node performs. This should be `NodeKind::Talk` as the TalkNodeBundle is used to create a talk node.
    pub kind: NodeKind,
    /// The text to be displayed by the talk node.
    pub text: TalkText,
}

impl TalkNodeBundle {
    /// Creates a new `TalkNodeBundle` with the specified `text` and `actors`.
    /// The node kind is set to `NodeKind::Talk`.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to be displayed in the talk node.
    /// * `actors` - The list of actors participating in the talk node.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_talks::prelude::*;
    ///
    /// let text = "Hello, world!".to_string();
    /// let actors = vec!["Alice".to_string(), "Bob".to_string()];
    /// let bundle = TalkNodeBundle::new(TalkText(text.clone()), Actors(actors.clone()));
    ///
    /// assert_eq!(bundle.kind, NodeKind::Talk);
    /// assert_eq!(bundle.text.0, text);
    /// ```
    pub fn new(text: TalkText) -> Self {
        Self {
            kind: NodeKind::Talk,
            text,
        }
    }
}

/// A bundle of component that defines a Choice node in the dialogue graph.
#[derive(Bundle)]
pub struct ChoiceNodeBundle {
    /// The kind of action that the node performs. This should be `NodeKind::Choice` as the ChoiceNodeBundle is used to create a choice node.
    kind: NodeKind,
    /// The list of choices in the choice node.
    choices: Choices,
}

impl Default for ChoiceNodeBundle {
    fn default() -> Self {
        Self {
            kind: NodeKind::Choice,
            choices: Choices::default(),
        }
    }
}

/// An enumeration of the different kinds of actions that can be performed in a Talk.
#[derive(Component, Debug, Default, Clone, PartialEq)]
pub enum NodeKind {
    /// A talk action, where a character speaks dialogue.
    #[default]
    Talk,
    /// A choice action, where the user is presented with a choice.
    Choice,
    /// An enter action, where a character enters a scene.
    Join,
    /// An exit action, where a character exits a scene.
    Leave,
}

/// The text component to be displayed from a Talk Node.
#[derive(Component, Default, Debug)]
pub struct TalkText(pub String);

/// The choices texts component to be displayed from a Choice Node.
#[derive(Component, Default, Debug)]
pub struct ChoicesTexts(pub Vec<String>);

/// The Actors participating in a dialogue node.
#[derive(Component, Default)]
pub struct Actors(pub Vec<String>);

/// The choices in a Choice node of the dialogue graph.
#[derive(Component, Default)]
pub struct Choices(pub Vec<Choice>);

/// A struct that represents a choice in a Talk.
///
/// This struct is used to define a choice in a Talk. It contains the text of the choice and
/// the ID of the next action to perform if the choice is selected.
#[derive(Debug, Clone)]
pub struct Choice {
    /// The text of the choice.
    pub text: String,
    /// The ID of the next action to jump to if the choice is selected.
    pub next: NodeIndex,
}
