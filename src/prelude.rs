//! Prelude for the `bevy_talks` crate.
pub use super::TalksPlugin;
pub use super::data::{
    Actor, ActorId, Conversation, ConversationId, DialogueDatabase, DialogueEntry, EntryId, Field,
    FieldValue, Link,
};
pub use super::loader::from_ron_str;
pub use super::loader::validate::{Issue, validate};
pub use super::saver::{DialogueDatabaseSaver, SaveError, to_ron_string};
