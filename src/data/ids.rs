//! Stable integer identifiers for database assets.

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

/// Identifies an [`Actor`](super::Actor) within a database.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct ActorId(pub i32);

/// Identifies a [`DialogueEntry`](super::DialogueEntry) within its conversation.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct EntryId(pub i32);

/// Identifies a [`Conversation`](super::Conversation) within a database.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct ConversationId(pub i32);
