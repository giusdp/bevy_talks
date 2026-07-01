//! `bevy_talks`: a plugin to write dialogues for your characters to say and do things. The Dialogue System for Bevy.

use bevy::prelude::*;

pub mod prelude;

/// The plugin that provides dialogue and conversation handling.
pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, _app: &mut App) {}
}
