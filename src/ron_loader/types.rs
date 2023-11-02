//! Types used by the ron loader.

use serde::Deserialize;

use crate::prelude::{ActionId, ActorId, RawAction, RawChoice, TalkNodeKind};

/// The ron talk asset type.
///
/// It contains a list of actors that appear in the Talk, and a list of actions that make up the Talk.
#[derive(Deserialize, Debug)]
pub(crate) struct RonTalk {
    /// The list of actors that appear in the Talk.
    pub(crate) actors: Vec<RonActor>,
    /// The list of actions that make up the Talk.
    pub(crate) script: Vec<RonAction>,
}

/// A struct that represents an action in a Talk.
///
/// This struct is used to define an action in a Talk. It contains the ID of the action, the
/// kind of action, the actors involved in the action, any choices that the user can make during
/// the action, the text of the action, the ID of the next action to perform, whether the action is
/// the start of the Talk, and any sound effect associated with the action.
#[derive(Debug, Default, Deserialize, Clone)]
pub(crate) struct RonAction {
    /// The ID of the action.
    pub(crate) id: ActionId,
    /// The kind of action.
    #[serde(default)]
    pub(crate) action: RonActionKind,
    /// The actors involved in the action.
    #[serde(default)]
    pub(crate) actors: Vec<ActorId>,
    /// Any choices that the user can make during the action.
    pub(crate) choices: Option<Vec<RonChoice>>,
    /// The text of the action.
    pub(crate) text: Option<String>,
    /// The ID of the next action to perform.
    pub(crate) next: Option<ActionId>,
}

impl From<RonAction> for RawAction {
    fn from(val: RonAction) -> Self {
        RawAction {
            id: val.id,
            kind: val.action.into(),
            actors: val.actors,
            choices: val
                .choices
                .map(|c| c.into_iter().map(|c| c.into()).collect()),
            text: val.text,
            next: val.next,
        }
    }
}

/// A struct that represents an actor in a Talk.
///
/// This struct is used to define an actor in a Talk. It contains the ID of the actor, the
/// name of the character that the actor plays, and an optional asset that represents the actor's
/// appearance or voice.
#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct RonActor {
    /// A string identifying uniquely the actor.
    pub(crate) id: ActorId,
    /// The name of the character that the actor plays.
    pub(crate) name: String,
    // An optional asset that represents the actor's appearance or voice.
    // pub(crate) asset: Option<String>,
}
/// A struct that represents a choice in a Talk.
///
/// This struct is used to define a choice in a Talk. It contains the text of the choice and
/// the ID of the next action to perform if the choice is selected.
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct RonChoice {
    /// The text of the choice.
    pub(crate) text: String,
    /// The ID of the next action to perform if the choice is selected.
    pub(crate) next: ActionId,
}

impl From<RonChoice> for RawChoice {
    fn from(val: RonChoice) -> Self {
        RawChoice {
            text: val.text,
            next: val.next,
        }
    }
}

/// An enumeration of the different kinds of actions that can be performed in a Talk.
///
/// This enumeration is used to define the different kinds of actions that can be performed in a
/// Talk. Each variant of the enumeration represents a different kind of action, such as
/// talking, entering, exiting, or making a choice.
#[derive(Debug, Default, Deserialize, Clone, PartialEq)]
pub(crate) enum RonActionKind {
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

impl From<RonActionKind> for TalkNodeKind {
    fn from(val: RonActionKind) -> Self {
        match val {
            RonActionKind::Talk => TalkNodeKind::Talk,
            RonActionKind::Join => TalkNodeKind::Join,
            RonActionKind::Leave => TalkNodeKind::Leave,
            RonActionKind::Choice => TalkNodeKind::Choice,
        }
    }
}
