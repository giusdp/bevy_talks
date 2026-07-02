//! Editor state: the working copy of the database, selection, and database creation.

use std::path::PathBuf;

use bevy::{
    asset::{
        AssetPath,
        saver::{SavedAsset, save_using_saver},
    },
    prelude::*,
    tasks::IoTaskPool,
    text::EditableText,
    ui_widgets::Activate,
};
use bevy_talks::prelude::*;

/// Asset path of the database opened at startup.
pub const DATABASE_PATH: &str = "test.dialogue.ron";

/// The editor's working copy of the loaded database.
#[derive(Resource)]
pub struct EditorState {
    /// The database being edited.
    pub db: DialogueDatabase,
    /// Asset path the database is saved to.
    pub path: String,
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

/// Marker for the "new database name" text input.
#[derive(Component, Default, Clone)]
pub struct NewDatabaseName;

/// Kicks off loading the startup database asset.
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
    commands.insert_resource(EditorState {
        db: db.clone(),
        path: DATABASE_PATH.to_owned(),
    });
    commands.remove_resource::<PendingLoad>();
}

/// Creates a new database from the name input, saves it, and switches to it.
pub fn create_new_database(
    _: On<Activate>,
    names: Query<&EditableText, With<NewDatabaseName>>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut selection: ResMut<EditorSelection>,
) {
    let raw = names
        .single()
        .map(|t| t.value().to_string())
        .unwrap_or_default();
    let name = sanitize_name(&raw);
    let path = format!("{name}.dialogue.ron");

    let file = assets_dir().join(&path);
    if file.exists() {
        warn!("{path} already exists, not overwriting");
        return;
    }

    let db = default_database();
    let first = db.conversations.first();
    selection.conversation = first.map(|c| c.id);
    selection.entry = first.and_then(root_entry_id);

    save_database(assets.clone(), db.clone(), path.clone());
    commands.insert_resource(EditorState { db, path });
}

/// The asset folder the editor works in.
pub fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

/// All `.dialogue.ron` files in the asset folder, sorted.
pub fn database_files() -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(assets_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            name.ends_with(".dialogue.ron").then_some(name)
        })
        .collect();
    files.sort();
    files
}

/// Reads a database straight from disk, bypassing the asset cache so the
/// file's current contents always win.
pub fn read_database(path: &str) -> Result<DialogueDatabase, String> {
    let text = std::fs::read_to_string(assets_dir().join(path)).map_err(|e| e.to_string())?;
    from_ron_str(&text).map_err(|e| e.to_string())
}

/// Saves a database to the given asset path in the background.
pub fn save_database(server: AssetServer, db: DialogueDatabase, path: String) {
    IoTaskPool::get()
        .spawn(async move {
            let asset_path = AssetPath::from(path);
            let saved = SavedAsset::from_asset(&db);
            match save_using_saver(server, &DialogueDatabaseSaver, &asset_path, saved, &()).await {
                Ok(()) => info!("saved {asset_path}"),
                Err(err) => error!("save failed: {err}"),
            }
        })
        .detach();
}

/// A fresh database with DSU-style defaults: a Player actor and one
/// conversation holding only its START entry.
fn default_database() -> DialogueDatabase {
    DialogueDatabase {
        version: "1".to_owned(),
        actors: vec![Actor {
            id: ActorId(0),
            name: "Player".to_owned(),
            is_player: true,
            fields: vec![],
        }],
        conversations: vec![Conversation {
            id: ConversationId(1),
            title: "New Conversation".to_owned(),
            entries: vec![DialogueEntry {
                id: EntryId(1),
                is_root: true,
                ..Default::default()
            }],
            ..Default::default()
        }],
    }
}

/// Turns free-form input into a safe file stem.
fn sanitize_name(raw: &str) -> String {
    let raw = raw.trim();
    let raw = raw.strip_suffix(".dialogue.ron").unwrap_or(raw);
    let name: String = raw
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() {
        "untitled".to_owned()
    } else {
        name
    }
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
