//! The dialogue database: the authored content asset.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::actor::Actor;
use super::conversation::Conversation;

/// A unit of authored dialogue content.
#[derive(Asset, Debug, Clone, Default, PartialEq, Reflect, Serialize, Deserialize)]
pub struct DialogueDatabase {
    /// Database version string.
    pub version: String,
    /// All actors in the database.
    pub actors: Vec<Actor>,
    /// All conversations in the database.
    pub conversations: Vec<Conversation>,
}
