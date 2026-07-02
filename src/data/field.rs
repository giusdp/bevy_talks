//! Fields: extensible key-value data attached to database assets.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::ids::ActorId;

/// A named piece of custom data on an actor, conversation, or entry.
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
pub struct Field {
    /// Field name, unique within the owning asset.
    pub title: String,
    /// The field's value.
    pub value: FieldValue,
}

/// The typed value of a [`Field`].
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
pub enum FieldValue {
    /// Plain text.
    Text(String),
    /// A number.
    Number(f32),
    /// A flag.
    Boolean(bool),
    /// A localized variant of another text field.
    Localization(String),
    /// A reference to an actor.
    Actor(ActorId),
}
