//! Loading databases from disk.

pub mod error;
pub mod validate;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::reflect::TypePath;

use crate::data::DialogueDatabase;
use error::LoadError;

/// Loads a [`DialogueDatabase`] from a `.dialogue.ron` file.
#[derive(Default, TypePath)]
pub struct DialogueDatabaseLoader;

impl AssetLoader for DialogueDatabaseLoader {
    type Asset = DialogueDatabase;
    type Settings = ();
    type Error = LoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<DialogueDatabase, LoadError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let db = ron::de::from_bytes(&bytes)?;
        Ok(db)
    }

    fn extensions(&self) -> &[&str] {
        &["dialogue.ron"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TalksPlugin;
    use bevy::prelude::*;

    #[test]
    fn loads_database_from_ron() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TalksPlugin));

        let handle: Handle<DialogueDatabase> = app
            .world()
            .resource::<AssetServer>()
            .load("test.dialogue.ron");

        app.update();
        let db = app
            .world()
            .resource::<Assets<DialogueDatabase>>()
            .get(&handle)
            .unwrap();

        assert_eq!(db.conversations[0].entries[0].dialogue_text, "Hello");
    }
}
