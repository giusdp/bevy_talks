//! Events the plugin can receive.

use bevy::prelude::*;

/// Event to request the current node to re-send all its events.
#[derive(Event)]
pub struct RefireNodeRequest {
    /// The entity with the `Talk` component you want to update.
    pub talk: Entity,
}

impl RefireNodeRequest {
    /// Creates a new `RefireNodeRequest`.
    pub fn new(talk: Entity) -> Self {
        Self { talk }
    }
}
/// Event to request the next node in a `Talk`. It requires an entity with the `Talk` component you want to update.
///
/// This event is typically used wired to an input from the player, e.g. a mouse click to advance the current dialogue.
/// It can fail (and logs an error) in case there is no next action or in case the current action is a choice action.
#[derive(Event)]
pub struct NextNodeRequest {
    /// The entity with the `Talk` component you want to update.
    pub talk: Entity,
}

impl NextNodeRequest {
    /// Creates a new `NextNodeRequest`.
    pub fn new(talk: Entity) -> Self {
        Self { talk }
    }
}

/// An event to jump to some specific node in a graph. It requires an entity with the `Talk` component you want to update.
///
/// It is typically used when you want to go to a target node from a choice node.
/// The `ActionId` to jump to is the one defined in the next field for the Choice choosen by the player.
#[derive(Event)]
pub struct ChooseNodeRequest {
    /// The entity with the `Talk` component you want to update.
    pub talk: Entity,
    /// The next entity to go to.
    pub next: Entity,
}

impl ChooseNodeRequest {
    /// Creates a new `ChooseNodeRequest`.
    pub fn new(talk: Entity, next: Entity) -> Self {
        Self { talk, next }
    }
}

// TODO: reset talk event request
