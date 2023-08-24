//! Events used in the plugin.

use bevy::prelude::{Entity, Event};

use crate::prelude::ActionId;

/// Event to request the next action in the `ActiveScreenplay`. It is sent with the entity with the `Screenplay` component to update.
///
/// This event is typically used wired to an input from the player, e.g. a mouse click to advance the current dialog.
/// `bevy_screenplay` has a system to react to these events. When it receives one, it takes
/// the screenplay in the `ActiveScreenplay` resource and changes the current action to the next one
/// if present. It can fail in case there is no next action or in case the current action is a `ActionKind::Choice` action.
#[derive(Event)]
pub struct NextActionRequest(pub Entity);

/// An event that requests to jump to a specific action in a screenplay.
///
/// This event is typically used to signal after a Player choice to jump to the action that is the result of the choice.
/// The `ActionId` to jump to is the one defined in the next field for the Choice choosen by the player.
#[derive(Event)]
pub struct JumpToActionRequest(pub Entity, pub ActionId);
