//! Talk action definitions.
use serde::Deserialize;

/// A unique identifier for an action in a Talk.
///
/// This type alias is used to define a unique identifier for an action in a Talk. Each action
/// in the Talk is assigned a unique ID, which is used to link the actions together in the
/// Talk graph.
pub type ActionId = i32;

/// A unique identifier for an actor in a Talk.
///
/// An `ActorId` is a `String` that uniquely identifies an actor in a Talk. It is used to
/// associate actions with the actors that perform them.
///
pub type ActorId = String;

/// An action node in a Talk.
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

/// A struct that represents an action in a Talk.
///
/// This struct is used to define an action in a Talk. It contains the ID of the action, the
/// kind of action, the actors involved in the action, any choices that the user can make during
/// the action, the text of the action, the ID of the next action to perform, whether the action is
/// the start of the Talk, and any sound effect associated with the action.
#[derive(Debug, Default, Deserialize, Clone)]
pub struct ScriptAction {
    /// The ID of the action.
    pub id: ActionId,
    /// The kind of action.
    #[serde(default)]
    pub action: ActionKind,
    /// The actors involved in the action.
    #[serde(default)]
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

/// A struct that represents an actor in a Talk.
///
/// This struct is used to define an actor in a Talk. It contains the ID of the actor, the
/// name of the character that the actor plays, and an optional asset that represents the actor's
/// appearance or voice.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Actor {
    /// A string identifying uniquely the actor.
    pub id: ActorId,
    /// The name of the character that the actor plays.
    pub name: String,
    /// An optional asset that represents the actor's appearance or voice.
    pub asset: Option<String>,
}
/// A struct that represents a choice in a Talk.
///
/// This struct is used to define a choice in a Talk. It contains the text of the choice and
/// the ID of the next action to perform if the choice is selected.
#[derive(Debug, Deserialize, Clone)]
pub struct Choice {
    /// The text of the choice.
    pub text: String,
    /// The ID of the next action to perform if the choice is selected.
    pub next: ActionId,
}

/// An enumeration of the different kinds of actions that can be performed in a Talk.
///
/// This enumeration is used to define the different kinds of actions that can be performed in a
/// Talk. Each variant of the enumeration represents a different kind of action, such as
/// talking, entering, exiting, or making a choice.
#[derive(Debug, Default, Deserialize, Clone, PartialEq)]
pub enum ActionKind {
    /// A talk action, where a character speaks dialogue.
    #[default]
    Talk,
    /// An enter action, where a character enters a scene.
    Enter,
    /// An exit action, where a character exits a scene.
    Exit,
    /// A choice action, where the user is presented with a choice.
    Choice,
}
