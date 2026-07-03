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
pub mod scripting;

/// The plugin that provides dialogue and conversation handling.
pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<DialogueDatabase>()
            .register_type::<DialogueDatabase>()
            .init_asset_loader::<DialogueDatabaseLoader>()
            .init_resource::<runtime::Variables>()
            .init_resource::<runtime::Visits>()
            .init_resource::<scripting::DialogueSystems>()
            .init_resource::<scripting::ScriptEngine>()
            .init_resource::<scripting::CompiledScripts>()
            .add_systems(
                Update,
                (
                    scripting::rebuild_engine
                        .run_if(resource_changed::<scripting::DialogueSystems>),
                    runtime::variables::seed_variables
                        .run_if(on_message::<AssetEvent<DialogueDatabase>>),
                    scripting::compile_scripts.run_if(on_message::<AssetEvent<DialogueDatabase>>),
                    runtime::runner::drive_runners
                        .run_if(any_with_component::<runtime::DialogueRunner>),
                )
                    .chain(),
            )
            .add_observer(runtime::runner::on_advance)
            .add_observer(runtime::runner::on_choose);
    }
}
