//! `bevy_talks`: a plugin to write dialogues for your characters to say and do things. The Dialogue System for Bevy.

use bevy::prelude::*;

use data::DialogueDatabase;
use loader::DialogueDatabaseLoader;

pub mod prelude;

pub mod data;
pub mod loader;
pub mod persist;
pub mod runtime;
pub mod saver;

/// The plugin that provides dialogue and conversation handling.
pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<DialogueDatabase>()
            .register_type::<DialogueDatabase>()
            .init_asset_loader::<DialogueDatabaseLoader>()
            .init_resource::<runtime::Variables>()
            .init_resource::<runtime::Visits>()
            .add_systems(
                Update,
                (
                    runtime::variables::seed_variables,
                    runtime::runner::start_runners,
                )
                    .chain(),
            )
            .add_observer(runtime::runner::on_advance)
            .add_observer(runtime::runner::on_choose);
    }
}
