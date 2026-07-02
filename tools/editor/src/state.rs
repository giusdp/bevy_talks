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
    /// The actor shown in the inspector.
    pub actor: Option<ActorId>,
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

/// Horizontal offset of a new child entry from its parent.
const CHILD_OFFSET_X: f32 = 280.0;
/// Vertical offset between siblings created under the same parent.
const CHILD_OFFSET_Y: f32 = 132.0;

/// A fresh database with defaults: a Player actor and one conversation holding only its START entry.
fn default_database() -> DialogueDatabase {
    let mut db = DialogueDatabase {
        version: "1".to_owned(),
        variables: vec![],
        actors: vec![Actor {
            id: ActorId(0),
            name: "Player".to_owned(),
            is_player: true,
            fields: vec![],
        }],
        conversations: vec![],
    };
    add_conversation(&mut db);
    db
}

/// Adds a new conversation seeded with its START root entry. Returns its id.
pub fn add_conversation(db: &mut DialogueDatabase) -> ConversationId {
    let id = ConversationId(db.conversations.iter().map(|c| c.id.0).max().unwrap_or(0) + 1);
    let actor = db
        .actors
        .iter()
        .find(|a| a.is_player)
        .map(|a| a.id)
        .unwrap_or_default();
    let conversant = db
        .actors
        .iter()
        .find(|a| !a.is_player)
        .map(|a| a.id)
        .unwrap_or_default();
    db.conversations.push(Conversation {
        id,
        title: format!("New Conversation {}", id.0),
        actor,
        conversant,
        entries: vec![DialogueEntry {
            id: EntryId(1),
            actor,
            conversant,
            is_root: true,
            ..Default::default()
        }],
        ..Default::default()
    });
    id
}

/// Adds a child entry linked from `parent`, with actor and conversant swapped
/// and a canvas position next to the parent.
/// Returns the child's id.
pub fn add_child_entry(
    db: &mut DialogueDatabase,
    conversation: ConversationId,
    parent: EntryId,
) -> Option<EntryId> {
    let conv = db.conversations.iter_mut().find(|c| c.id == conversation)?;
    let child_id = EntryId(conv.entries.iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
    let parent_entry = conv.entries.iter_mut().find(|e| e.id == parent)?;

    let actor = parent_entry.conversant;
    let conversant = parent_entry.actor;
    let siblings = parent_entry
        .links
        .iter()
        .filter(|l| l.dest_conversation == conversation)
        .count() as f32;
    let parent_pos = (
        number_field(parent_entry, "canvas_x"),
        number_field(parent_entry, "canvas_y"),
    );
    parent_entry.links.push(Link {
        dest_conversation: conversation,
        dest_entry: child_id,
    });

    let mut child = DialogueEntry {
        id: child_id,
        actor,
        conversant,
        ..Default::default()
    };
    if let (Some(x), Some(y)) = parent_pos {
        set_number_field(&mut child, "canvas_x", x + CHILD_OFFSET_X);
        set_number_field(&mut child, "canvas_y", y + siblings * CHILD_OFFSET_Y);
    }
    conv.entries.push(child);
    Some(child_id)
}

/// Adds a new actor to the database. Returns its id.
pub fn add_actor(db: &mut DialogueDatabase) -> ActorId {
    let id = ActorId(db.actors.iter().map(|a| a.id.0).max().unwrap_or(-1) + 1);
    db.actors.push(Actor {
        id,
        name: format!("New Actor {}", id.0),
        is_player: false,
        fields: vec![],
    });
    id
}

/// Adds a new variable with a unique placeholder name. Returns its index.
pub fn add_variable(db: &mut DialogueDatabase) -> usize {
    let mut n = db.variables.len() + 1;
    while db
        .variables
        .iter()
        .any(|v| v.name == format!("Variable {n}"))
    {
        n += 1;
    }
    db.variables.push(Variable {
        name: format!("Variable {n}"),
        initial: FieldValue::Text(String::new()),
        fields: vec![],
    });
    db.variables.len() - 1
}

/// Removes the variable at `index`. Returns true if it existed.
pub fn remove_variable(db: &mut DialogueDatabase, index: usize) -> bool {
    if index < db.variables.len() {
        db.variables.remove(index);
        true
    } else {
        false
    }
}

/// Which asset's fields bag a field operation targets.
#[derive(Clone, Copy, PartialEq)]
pub enum FieldOwner {
    /// An entry's fields.
    Entry(ConversationId, EntryId),
    /// An actor's fields.
    Actor(ActorId),
    /// A conversation's fields.
    Conversation(ConversationId),
}

impl Default for FieldOwner {
    fn default() -> Self {
        Self::Actor(ActorId::default())
    }
}

/// The owner's fields bag, mutably.
fn fields_mut(db: &mut DialogueDatabase, owner: FieldOwner) -> Option<&mut Vec<Field>> {
    match owner {
        FieldOwner::Entry(conversation, entry) => db
            .conversations
            .iter_mut()
            .find(|c| c.id == conversation)
            .and_then(|c| c.entries.iter_mut().find(|e| e.id == entry))
            .map(|e| &mut e.fields),
        FieldOwner::Actor(actor) => db
            .actors
            .iter_mut()
            .find(|a| a.id == actor)
            .map(|a| &mut a.fields),
        FieldOwner::Conversation(conversation) => db
            .conversations
            .iter_mut()
            .find(|c| c.id == conversation)
            .map(|c| &mut c.fields),
    }
}

/// Adds a custom field, starting as empty text. Refuses empty, `canvas_*`,
/// and duplicate titles. Returns true if added.
pub fn add_field(db: &mut DialogueDatabase, owner: FieldOwner, title: &str) -> bool {
    if title.is_empty() || title.starts_with("canvas_") {
        warn!("field name {title:?} is reserved or empty");
        return false;
    }
    let Some(fields) = fields_mut(db, owner) else {
        return false;
    };
    if fields.iter().any(|f| f.title == title) {
        warn!("field {title:?} already exists");
        return false;
    }
    fields.push(Field {
        title: title.to_owned(),
        value: FieldValue::Text(String::new()),
    });
    true
}

/// Removes a custom field by title. Returns true if it existed.
pub fn remove_field(db: &mut DialogueDatabase, owner: FieldOwner, title: &str) -> bool {
    let Some(fields) = fields_mut(db, owner) else {
        return false;
    };
    let before = fields.len();
    fields.retain(|f| f.title != title);
    fields.len() != before
}

/// Adds a link between two entries of a conversation. Refuses self-links,
/// duplicates, and missing destinations. Returns true if added.
pub fn add_link(
    db: &mut DialogueDatabase,
    conversation: ConversationId,
    from: EntryId,
    to: EntryId,
) -> bool {
    if from == to {
        return false;
    }
    let Some(conv) = db.conversations.iter_mut().find(|c| c.id == conversation) else {
        return false;
    };
    if !conv.entries.iter().any(|e| e.id == to) {
        return false;
    }
    let Some(source) = conv.entries.iter_mut().find(|e| e.id == from) else {
        return false;
    };
    let link = Link {
        dest_conversation: conversation,
        dest_entry: to,
    };
    if source.links.contains(&link) {
        return false;
    }
    source.links.push(link);
    true
}

/// Removes one outgoing link from an entry. Returns true if it was present.
pub fn remove_link(
    db: &mut DialogueDatabase,
    conversation: ConversationId,
    from: EntryId,
    link: Link,
) -> bool {
    let Some(source) = db
        .conversations
        .iter_mut()
        .find(|c| c.id == conversation)
        .and_then(|c| c.entries.iter_mut().find(|e| e.id == from))
    else {
        return false;
    };
    let before = source.links.len();
    source.links.retain(|l| *l != link);
    source.links.len() != before
}

/// Deletes an entry and every link pointing at it, in any conversation.
/// Root entries can't be deleted. Returns true if something was removed.
pub fn delete_entry(
    db: &mut DialogueDatabase,
    conversation: ConversationId,
    entry: EntryId,
) -> bool {
    let Some(conv) = db.conversations.iter_mut().find(|c| c.id == conversation) else {
        return false;
    };
    let Some(index) = conv.entries.iter().position(|e| e.id == entry) else {
        return false;
    };
    if conv.entries[index].is_root {
        warn!("the START entry cannot be deleted");
        return false;
    }
    conv.entries.remove(index);
    for conv in &mut db.conversations {
        for e in &mut conv.entries {
            e.links
                .retain(|l| !(l.dest_conversation == conversation && l.dest_entry == entry));
        }
    }
    true
}

/// A `Number` field value by title, if present.
pub fn number_field(entry: &DialogueEntry, title: &str) -> Option<f32> {
    entry
        .fields
        .iter()
        .find(|f| f.title == title)
        .and_then(|f| match f.value {
            FieldValue::Number(n) => Some(n),
            _ => None,
        })
}

/// Writes a `Number` field, adding it if missing.
pub fn set_number_field(entry: &mut DialogueEntry, title: &str, value: f32) {
    match entry.fields.iter_mut().find(|f| f.title == title) {
        Some(field) => field.value = FieldValue::Number(value),
        None => entry.fields.push(Field {
            title: title.to_owned(),
            value: FieldValue::Number(value),
        }),
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
