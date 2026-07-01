//! Actors: the characters that speak and act in conversations.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::ids::ActorId;

/// A character that can participate in conversations.
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
pub struct Actor {
    /// Unique id within the database.
    pub id: ActorId,
    /// Display name.
    pub name: String,
    /// Whether this actor is a player character.
    pub is_player: bool,
}
