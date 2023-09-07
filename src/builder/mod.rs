//! Builder module

use bevy::reflect::{Reflect, TypeUuid};
use serde::Deserialize;

pub(crate) mod builder;
pub(crate) mod errors;
pub(crate) mod loader;

/// A struct that represents a raw Talk (as from the json format).
///
/// It contains a list of actors that appear in the Talk, and a list of actions that make up the Talk.
#[derive(Debug, Deserialize, Default, Clone, Reflect, TypeUuid)]
#[uuid = "413be529-bfeb-8c5b-9db0-4b8b380a2c47"]
#[reflect_value]
pub struct RawTalk {
    /// The list of actors that appear in the Talk.
    pub actors: Vec<RawActor>,
    /// The list of actions that make up the Talk.
    pub script: Vec<RawAction>,
}

/// A unique identifier for an action in a Talk.
///
/// This type alias is used to define a unique identifier for an action in a Talk. Each action
/// in the Talk is assigned a unique ID, which is used to link the actions together in the
/// Talk graph.
type ActionId = i32;

/// A unique identifier for an actor in a Talk.
///
/// An `ActorId` is a `String` that uniquely identifies an actor in a Talk. It is used to
/// associate actions with the actors that perform them.
///
type ActorId = String;

/// A struct that represents an action in a Talk.
///
/// This struct is used to define an action in a Talk. It contains the ID of the action, the
/// kind of action, the actors involved in the action, any choices that the user can make during
/// the action, the text of the action, the ID of the next action to perform, whether the action is
/// the start of the Talk, and any sound effect associated with the action.
#[derive(Debug, Default, Deserialize, Clone)]
struct RawAction {
    /// The ID of the action.
    pub id: ActionId,
    /// The kind of action.
    #[serde(default)]
    pub action: ActionKind,
    /// The actors involved in the action.
    #[serde(default)]
    pub actors: Vec<String>,
    /// Any choices that the user can make during the action.
    pub choices: Option<Vec<RawChoice>>,
    /// The text of the action.
    pub text: Option<String>,
    /// The ID of the next action to perform.
    pub next: Option<ActionId>,
}

/// A struct that represents an actor in a Talk.
///
/// This struct is used to define an actor in a Talk. It contains the ID of the actor, the
/// name of the character that the actor plays, and an optional asset that represents the actor's
/// appearance or voice.
#[derive(Debug, Deserialize, Clone, Default)]
struct RawActor {
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
struct RawChoice {
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
enum ActionKind {
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
