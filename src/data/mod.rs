//! The dialogue database data model.

pub mod actor;
pub mod conversation;
pub mod database;
pub mod entry;
pub mod ids;
pub mod link;

pub use actor::Actor;
pub use conversation::Conversation;
pub use database::DialogueDatabase;
pub use entry::DialogueEntry;
pub use ids::{ActorId, ConversationId, EntryId};
pub use link::Link;
