//! Editor state: the working copy of the database and the current selection.

use bevy::prelude::*;
use bevy_talks::prelude::*;

/// Asset path of the database being edited.
pub const DATABASE_PATH: &str = "test.dialogue.ron";

/// The editor's working copy of the loaded database.
#[derive(Resource)]
pub struct EditorState {
    /// The database being edited.
    pub db: DialogueDatabase,
}

impl EditorState {
    /// The conversation with the given id, if any.
    pub fn conversation(&self, id: Option<ConversationId>) -> Option<&Conversation> {
        self.db.conversations.iter().find(|c| Some(c.id) == id)
    }

    /// Display name for an actor id, falling back to the raw id.
    pub fn actor_name(&self, id: ActorId) -> String {
        self.db
            .actors
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| format!("actor {}", id.0))
    }
}

/// What is currently selected in the editor.
#[derive(Resource, Default)]
pub struct EditorSelection {
    /// The conversation shown on the canvas.
    pub conversation: Option<ConversationId>,
    /// The entry shown in the inspector.
    pub entry: Option<EntryId>,
}

/// Handle of a database load in flight.
#[derive(Resource)]
pub struct PendingLoad(pub Handle<DialogueDatabase>);

/// Kicks off loading the database asset.
pub fn start_database_load(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(PendingLoad(assets.load(DATABASE_PATH)));
}

/// Copies the loaded asset into [`EditorState`] and selects the first conversation.
pub fn finish_database_load(
    mut commands: Commands,
    pending: Res<PendingLoad>,
    databases: Res<Assets<DialogueDatabase>>,
    mut selection: ResMut<EditorSelection>,
) {
    let Some(db) = databases.get(&pending.0) else {
        return;
    };
    let first = db.conversations.first();
    selection.conversation = first.map(|c| c.id);
    selection.entry = first.and_then(root_entry_id);
    commands.insert_resource(EditorState { db: db.clone() });
    commands.remove_resource::<PendingLoad>();
}

/// The root entry of a conversation, falling back to the first entry.
pub fn root_entry_id(conversation: &Conversation) -> Option<EntryId> {
    conversation
        .entries
        .iter()
        .find(|e| e.is_root)
        .or_else(|| conversation.entries.first())
        .map(|e| e.id)
}
