//! Dialogue entries: the nodes of a conversation graph.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::field::Field;
use super::ids::{ActorId, EntryId};
use super::link::Link;

/// A single node in a conversation: a spoken line, a menu choice, or a group.
#[derive(Component, Debug, Clone, Default, PartialEq, Reflect, Serialize, Deserialize)]
pub struct DialogueEntry {
    /// Unique id within the owning conversation.
    pub id: EntryId,
    /// Who speaks this entry.
    pub actor: ActorId,
    /// Who this entry is addressed to.
    pub conversant: ActorId,
    /// Label shown when this entry is offered as a menu choice.
    pub menu_text: String,
    /// Spoken/subtitle line. Kept separate from `menu_text`.
    pub dialogue_text: String,
    /// Whether this is the conversation's root entry.
    pub is_root: bool,
    /// Whether this is an organizational group node.
    pub is_group: bool,
    /// Outgoing links to other entries.
    pub links: Vec<Link>,
    /// Custom fields.
    #[serde(default)]
    pub fields: Vec<Field>,
    /// Rune expression gating whether this entry can be reached. Empty means always.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub condition: String,
    /// Rune code run when this entry is presented. Empty means nothing to run.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub script: String,
}
