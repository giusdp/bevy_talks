//! Types used in the plugin.
use bevy::prelude::{Entity, Event, Resource};

/// Event to request the next action for the active screenplay.
#[derive(Event)]
pub struct ScreenplayNextActionRequest;

/// Resource that keeps track of the currently active screenplay.
#[derive(Resource, Default)]
pub struct ActiveScreenplay {
    /// The entity containing the currently active screenplay.
    pub e: Option<Entity>,
    /// Whether the active screenplay has moved to the next action.
    pub changed: bool,
}
