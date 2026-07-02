//! Prelude for the `bevy_talks` crate.
pub use super::TalksPlugin;
pub use super::data::{
    Actor, ActorId, Conversation, ConversationId, DialogueDatabase, DialogueEntry, EntryId, Field,
    FieldValue, Link,
};
pub use super::loader::validate::{Issue, validate};
