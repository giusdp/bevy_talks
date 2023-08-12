#![allow(dead_code)]
//! This module contains the raw JSON data structures to load a screenplay from a JSON file.
use serde::Deserialize;

/// A struct that represents a raw screenplay in JSON format.
///
/// This struct is used to represent a raw screenplay in JSON format. It contains a list of actors
/// that appear in the screenplay, and a list of actions that make up the screenplay.
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct RawScreenplayJSON {
    /// The list of actors that appear in the screenplay.
    pub(crate) actors: Vec<ActorJSON>,
    /// The list of actions that make up the screenplay.
    pub(crate) script: Vec<ActionJSON>,
}

/// A struct that represents an actor in a screenplay.
///
/// This struct is used to define an actor in a screenplay. It contains the ID of the actor, the
/// name of the character that the actor plays, and an optional asset that represents the actor's
/// appearance or voice.
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ActorJSON {
    /// The ID of the actor.
    pub actor_id: String,
    /// The name of the character that the actor plays.
    pub character_name: String,
    /// An optional asset that represents the actor's appearance or voice.
    pub asset: Option<String>,
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
pub(crate) struct ActionJSON {
    /// The ID of the action.
    pub id: ActionId,
    /// The kind of action.
    pub action: ActionKind,
    /// The actors involved in the action.
    pub actors: Vec<String>,
    /// Any choices that the user can make during the action.
    pub choices: Option<Vec<ChoiceJSON>>,
    /// The text of the action.
    pub text: Option<String>,
    /// The ID of the next action to perform.
    pub next: Option<ActionId>,
    /// Whether the action is the start of the screenplay.
    pub start: Option<bool>,
    /// Any sound effect associated with the action.
    pub sound_effect: Option<String>,
}

/// A struct that represents a choice in a screenplay.
///
/// This struct is used to define a choice in a screenplay. It contains the text of the choice and
/// the ID of the next action to perform if the choice is selected.
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ChoiceJSON {
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
pub(crate) enum ActionKind {
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
