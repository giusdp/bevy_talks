//! `bevy_talks`: a plugin to write dialogues for your characters to say and do things. The Dialogue System for Bevy.

use bevy::prelude::*;

use data::DialogueDatabase;

pub mod prelude;

mod data;

/// The plugin that provides dialogue and conversation handling.
pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<DialogueDatabase>()
            .register_type::<DialogueDatabase>();
    }
}
