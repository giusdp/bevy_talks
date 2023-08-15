//! Screenplay action definitions.
use serde::Deserialize;

/// An action node in a screenplay.
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct ActionNode {
    /// The kind of action.
    pub(crate) kind: ActionKind,
    /// The text of the action.
    pub(crate) text: Option<String>,
    /// The actors involved in the action.
    pub(crate) actors: Vec<Actor>,
    /// The choices available after the action.
    pub(crate) choices: Option<Vec<Choice>>,
    /// The sound effect associated with the action.
    pub(crate) sound_effect: Option<String>,
}

/// A unique identifier for an action in a screenplay.
///
/// This type alias is used to define a unique identifier for an action in a screenplay. Each action
/// in the screenplay is assigned a unique ID, which is used to link the actions together in the
/// screenplay graph.
pub(crate) type ActionId = i32;

/// A struct that represents an action in a screenplay.
///
/// This struct is used to define an action in a screenplay. It contains the ID of the action, the
/// kind of action, the actors involved in the action, any choices that the user can make during
/// the action, the text of the action, the ID of the next action to perform, whether the action is
/// the start of the screenplay, and any sound effect associated with the action.
#[derive(Debug, Default, Deserialize, Clone)]
pub struct ScriptAction {
    /// The ID of the action.
    pub id: ActionId,
    /// The kind of action.
    pub kind: ActionKind,
    /// The actors involved in the action.
    pub actors: Vec<String>,
    /// Any choices that the user can make during the action.
    pub choices: Option<Vec<Choice>>,
    /// The text of the action.
    pub text: Option<String>,
    /// The ID of the next action to perform.
    pub next: Option<ActionId>,
    /// Any sound effect associated with the action.
    pub sound_effect: Option<String>,
}

/// A struct that represents an actor in a screenplay.
///
/// This struct is used to define an actor in a screenplay. It contains the ID of the actor, the
/// name of the character that the actor plays, and an optional asset that represents the actor's
/// appearance or voice.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Actor {
    /// The name of the character that the actor plays.
    pub name: String,
    /// An optional asset that represents the actor's appearance or voice.
    pub asset: Option<String>,
}
/// A struct that represents a choice in a screenplay.
///
/// This struct is used to define a choice in a screenplay. It contains the text of the choice and
/// the ID of the next action to perform if the choice is selected.
#[derive(Debug, Deserialize, Clone)]
pub struct Choice {
    /// The text of the choice.
    pub text: String,
    /// The ID of the next action to perform if the choice is selected.
    pub next: ActionId,
}

/// An enumeration of the different kinds of actions that can be performed in a screenplay.
///
/// This enumeration is used to define the different kinds of actions that can be performed in a
/// screenplay. Each variant of the enumeration represents a different kind of action, such as
/// talking, entering, exiting, or making a choice.
#[derive(Debug, Default, Deserialize, Clone)]
pub enum ActionKind {
    /// A talk action, where a character speaks dialogue.
    #[default]
    #[serde(rename = "talk")]
    Talk,
    /// An enter action, where a character enters a scene.
    #[serde(rename = "enter")]
    Enter,
    /// An exit action, where a character exits a scene.
    #[serde(rename = "exit")]
    Exit,
    /// A choice action, where the user is presented with a choice.
    #[serde(rename = "choice")]
    Choice,
}
