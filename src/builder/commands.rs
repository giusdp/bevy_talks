//! Commands for talks

use bevy::ecs::system::{Commands, EntityCommands};

use crate::prelude::Talk;

use super::{build_command::BuildTalkCommand, TalkBuilder};

/// Extension trait for [`Commands`] to spawn a talk.
pub trait TalkCommandsExt<'w, 's> {
    /// Spawns a dialogue graph and a parent entity with a [`Talk`] component + the input bundle.
    /// Returns a handle of the parent entity.
    /// TODO: write example
    fn spawn_talk(&mut self, builder: TalkBuilder) -> EntityCommands<'w, 's, '_>;
}

impl<'w, 's> TalkCommandsExt<'w, 's> for Commands<'w, 's> {
    fn spawn_talk(&mut self, builder: TalkBuilder) -> EntityCommands<'w, 's, '_> {
        let parent = self.spawn(Talk::default()).id();
        self.add(BuildTalkCommand::new(parent, builder));
        self.entity(parent)
    }
}
