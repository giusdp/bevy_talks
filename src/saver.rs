//! Saving databases back to the `.dialogue.ron` format.

use bevy::asset::AssetPath;
use bevy::asset::io::Writer;
use bevy::asset::saver::{AssetSaver, SavedAsset};
use bevy::reflect::TypePath;
use bevy::tasks::futures_lite::AsyncWriteExt;
use thiserror::Error;

use crate::data::DialogueDatabase;
use crate::loader::DialogueDatabaseLoader;

/// Serializes a database to the on-disk `.dialogue.ron` text format.
pub fn to_ron_string(db: &DialogueDatabase) -> Result<String, ron::Error> {
    let mut out = ron::ser::to_string_pretty(db, ron::ser::PrettyConfig::default())?;
    out.push('\n');
    Ok(out)
}

/// An error produced while saving a database.
#[derive(Debug, Error)]
pub enum SaveError {
    /// Serializing to RON failed.
    #[error(transparent)]
    Ron(#[from] ron::Error),
    /// Writing the bytes failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Saves a [`DialogueDatabase`] as a `.dialogue.ron` file.
///
/// Works both in asset processing pipelines and at runtime via
/// [`save_using_saver`](bevy::asset::saver::save_using_saver).
#[derive(Default, TypePath)]
pub struct DialogueDatabaseSaver;

impl AssetSaver for DialogueDatabaseSaver {
    type Asset = DialogueDatabase;
    type Settings = ();
    type OutputLoader = DialogueDatabaseLoader;
    type Error = SaveError;

    async fn save(
        &self,
        writer: &mut Writer,
        asset: SavedAsset<'_, '_, Self::Asset>,
        _settings: &(),
        _asset_path: AssetPath<'_>,
    ) -> Result<(), SaveError> {
        let ron = to_ron_string(&asset)?;
        writer.write_all(ron.as_bytes()).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{
        Actor, ActorId, Conversation, ConversationId, DialogueDatabase, DialogueEntry, EntryId,
        Field, FieldValue, Link,
    };

    #[test]
    fn ron_roundtrip() {
        let db = DialogueDatabase {
            version: "1".to_owned(),
            variables: vec![],
            actors: vec![Actor {
                id: ActorId(0),
                name: "Player".to_owned(),
                is_player: true,
                fields: vec![],
            }],
            conversations: vec![Conversation {
                id: ConversationId(1),
                title: "Test".to_owned(),
                entries: vec![DialogueEntry {
                    id: EntryId(1),
                    dialogue_text: "Hello".to_owned(),
                    is_root: true,
                    links: vec![Link {
                        dest_conversation: ConversationId(1),
                        dest_entry: EntryId(1),
                    }],
                    fields: vec![Field {
                        title: "mood".to_owned(),
                        value: FieldValue::Text("cheerful".to_owned()),
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            }],
        };

        let ron = to_ron_string(&db).unwrap();
        let back: DialogueDatabase = ron::de::from_str(&ron).unwrap();
        assert_eq!(db, back);
    }
}
