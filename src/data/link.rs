//! Links: directed edges between dialogue entries.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::ids::{ConversationId, EntryId};

/// A directed edge to a destination entry (which may be in another conversation).
#[derive(Debug, Clone, Copy, Default, PartialEq, Reflect, Serialize, Deserialize)]
pub struct Link {
    /// The conversation the destination entry lives in.
    pub dest_conversation: ConversationId,
    /// The destination entry.
    pub dest_entry: EntryId,
}
