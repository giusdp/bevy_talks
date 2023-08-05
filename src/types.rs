//! Types used in the plugin.
use bevy::prelude::Component;

/// Marker component for screenplay entities to
/// indicate to move to the next action
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ScreenplayNextAction;
