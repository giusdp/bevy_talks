//! Action nodes are used to define the actions that characters perform in a screenplay.
//! An action node is ultimately a [`bevy::ecs::Entity`], with [`bevy_talks`] components
//! attached that define the action.
use bevy::prelude::{Commands, Component, Entity};

/// An `ActionNode` is an entity that represents an action in a screenplay.
///
/// Action nodes are used to define the actions that characters perform in a screenplay. They can be
/// linked together to create a sequence of actions that make up a scene or an entire screenplay.
pub(crate) type ActionNode = Entity;

/// A component that indicates that the entity is a "talk".
/// It contains only the text to be displayed, without any
/// information about the speaker.
/// For example, it can be used to display text said by a narrator
/// and no speaker name is needed.
/// Use [`SpeakerTalkComp`] to have text and speaker.
#[derive(Component)]
pub struct TalkComp {
    /// The text to be displayed.
    pub text: String,
}

/// Spawn a new entity with a [`TalkComp`] component attached.
pub fn new_talk(commands: &mut Commands, text: String) -> ActionNode {
    let c = commands.spawn(TalkComp { text });
    c.id()
}
