//! Commands for talks

use bevy::ecs::{
    bundle::Bundle,
    system::{Commands, EntityCommands},
};

use crate::prelude::Talk;

use super::{build_command::BuildTalkCommand, TalkBuilder};

/// Extension trait for [`Commands`] to get [`TalkCommands`]
pub trait TalkCommandsExt<'w, 's> {
    /// Spawns a dialogue graph and a parent entity with a [`Talk`] component + the input bundle.
    /// Returns a handle of the parent entity.
    /// TODO: write example
    fn spawn_talk<T>(&mut self, builder: TalkBuilder, bundle: T) -> EntityCommands<'w, 's, '_>
    where
        T: Bundle + 'static;
}

impl<'w, 's> TalkCommandsExt<'w, 's> for Commands<'w, 's> {
    fn spawn_talk<T>(&mut self, builder: TalkBuilder, bundle: T) -> EntityCommands<'w, 's, '_>
    where
        T: Bundle + 'static,
    {
        let parent = self.spawn(bundle).insert(Talk::default()).id();
        self.add(BuildTalkCommand::new(parent, builder));
        self.entity(parent)
    }
}
