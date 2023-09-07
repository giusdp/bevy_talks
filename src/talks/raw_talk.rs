//! A module that defines the raw data structures used to build a Talk.
//!
use super::{Actor, TalkNodeKind};
use bevy::{
    prelude::{Handle, Image},
    reflect::{Reflect, TypeUuid},
};

/// A unique identifier for an action in a Talk.
///
/// This type alias is used to define a unique identifier for an action in a Talk. Each action
/// in the Talk is assigned a unique ID, which is used to link the actions together in the
/// Talk graph.
pub(crate) type ActionId = i32;

/// A unique identifier for an actor in a Talk.
///
/// An `ActorId` is a `String` that uniquely identifies an actor in a Talk. It is used to
/// associate actions with the actors that perform them.
///
pub(crate) type ActorId = String;

/// A struct that represents a Raw Talk.
#[derive(Debug, Default, Clone, Reflect, TypeUuid)]
#[uuid = "413be529-bfeb-8c5b-9db0-4b8b380a2c47"]
#[reflect_value]
pub struct RawTalk {
    /// The list of actions that make up the Talk.
    pub(crate) script: Vec<RawAction>,
    /// The list of actors that appear in the Talk.
    pub(crate) actors: Vec<RawActor>,
}

/// A struct that represents an action in a Talk.
///
/// This struct is used to define an action in a Talk. It contains the ID of the action, the
/// kind of action, the actors involved in the action, any choices that the user can make during
/// the action, the text of the action, the ID of the next action to perform, whether the action is
/// the start of the Talk, and any sound effect associated with the action.
#[derive(Debug, Default, Clone)]
pub(crate) struct RawAction {
    /// The ID of the action.
    pub(crate) id: ActionId,
    /// The kind of action.
    pub(crate) kind: TalkNodeKind,
    /// The actors involved in the action.
    pub(crate) actors: Vec<ActorId>,
    /// Any choices that the user can make during the action.
    pub(crate) choices: Option<Vec<RawChoice>>,
    /// The text of the action.
    pub(crate) text: Option<String>,
    /// The ID of the next action to perform.
    pub(crate) next: Option<i32>,
}

/// A struct that represents an actor in a Talk.
///
/// This struct is used to define an actor in a Talk. It contains the ID of the actor, the
/// name of the character that the actor plays, and an optional asset that represents the actor's
/// appearance or voice.
#[derive(Debug, Clone, Default)]
pub(crate) struct RawActor {
    pub(crate) id: ActorId,
    /// The name of the character that the actor plays.
    pub(crate) name: String,
    /// An optional asset that represents the actor's appearance or voice.
    pub(crate) asset: Option<Handle<Image>>,
}

impl Into<Actor> for RawActor {
    fn into(self) -> Actor {
        Actor {
            name: self.name,
            asset: self.asset,
        }
    }
}

/// A struct that represents a choice in a Talk.
///
/// This struct is used to define a choice in a Talk. It contains the text of the choice and
/// the ID of the next action to perform if the choice is selected.
#[derive(Debug, Clone)]
pub(crate) struct RawChoice {
    /// The text of the choice.
    pub(crate) text: String,
    /// The ID of the next action to perform if the choice is selected.
    pub(crate) next: ActionId,
}
