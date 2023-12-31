//! Events used in the plugin.

use bevy::prelude::{Entity, Event};

/// Event to request the next action in a [`Talk`]. It requires an entity with the [`Talk`] component you want to update.
///
/// This event is typically used wired to an input from the player, e.g. a mouse click to advance the current dialogue.
/// It can fail (and logs an error) in case there is no next action or in case the current action is a choice action.
#[derive(Event)]
pub struct NextActionRequest(pub Entity);

/// An event to jump to some specific node in a graph. It requires an entity with the [`Talk`] component you want to update.
///
/// It is typically used when you want to go to a target node from a choice node.
/// The `ActionId` to jump to is the one defined in the next field for the Choice choosen by the player.
#[derive(Event)]
pub struct ChooseActionRequest {
    /// The entity with the [`Talk`] component you want to update.
    pub talk: Entity,
    /// The next entity to go to.
    pub next: Entity,
}

impl ChooseActionRequest {
    /// Creates a new `ChooseActionRequest`.
    pub fn new(talk: Entity, next: Entity) -> Self {
        Self { talk, next }
    }
}

// TODO: more events to talk to the library... (reset talk?)
