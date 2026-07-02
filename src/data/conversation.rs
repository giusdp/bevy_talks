//! Conversations: directed graphs of dialogue entries.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::entry::DialogueEntry;
use super::field::Field;
use super::ids::{ActorId, ConversationId};

/// A single conversation and its dialogue entries.
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
pub struct Conversation {
    /// Unique id within the database.
    pub id: ConversationId,
    /// Display title.
    pub title: String,
    /// Default speaker for entries that don't override it.
    pub actor: ActorId,
    /// Default listener for entries that don't override it.
    pub conversant: ActorId,
    /// The entries (graph nodes) that make up this conversation.
    pub entries: Vec<DialogueEntry>,
    /// Custom fields.
    #[serde(default)]
    pub fields: Vec<Field>,
}
