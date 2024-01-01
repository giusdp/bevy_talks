//! Commands for talks
use std::ops::{Deref, DerefMut};

use bevy::ecs::{
    bundle::Bundle,
    system::{Commands, EntityCommands},
};

use crate::prelude::Talk;

use super::{build_command::BuildTalkCommand, TalkBuilder};

/// Commands to spawn dialogue graphs
pub struct TalkCommands<'a, 'w, 's> {
    /// The commands
    commands: &'a mut Commands<'w, 's>,
}
impl<'a, 'w, 's> Deref for TalkCommands<'a, 'w, 's> {
    type Target = Commands<'w, 's>;

    fn deref(&self) -> &Self::Target {
        self.commands
    }
}

impl<'a, 'w, 's> DerefMut for TalkCommands<'a, 'w, 's> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.commands
    }
}

/// Extension trait for [`Commands`] to get [`TalkCommands`]
pub trait TalkCommandsExt<'w, 's> {
    /// Gets the [`TalkCommands`] to apply commands to spawn talks.
    fn talks<'a>(&'a mut self) -> TalkCommands<'a, 'w, 's>;
}

impl<'w, 's> TalkCommandsExt<'w, 's> for Commands<'w, 's> {
    /// Gets the [`TalkCommands`] to apply commands to spawn talks.
    fn talks(&mut self) -> TalkCommands<'_, 'w, 's> {
        TalkCommands { commands: self }
    }
}

impl<'a, 'w, 's> TalkCommands<'a, 'w, 's> {
    /// Spawns a dialogue graph and a parent entity with a [`Talk`] component + the input bundle.
    /// Returns a handle of the parent entity.
    pub fn spawn_talk<T>(&mut self, builder: TalkBuilder, bundle: T) -> EntityCommands<'w, 's, '_>
    where
        T: Bundle + 'static,
    {
        let parent = self.spawn(bundle).insert(Talk::default()).id();
        self.add(BuildTalkCommand::new(parent, builder));
        self.entity(parent)
    }
}
