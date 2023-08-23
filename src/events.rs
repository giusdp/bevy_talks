//! Events used in the plugin.

use bevy::prelude::Event;

use crate::prelude::{ActionId, Actor};

/// Event to request the next action in the `ActiveScreenplay`.
///
/// This event is typically used wired to an input from the player, e.g. a mouse click to advance the current dialog.
/// `bevy_screenplay` has a system to react to these events. When it receives one, it takes
/// the screenplay in the `ActiveScreenplay` resource and changes the current action to the next one
/// if present. It can fail in case there is no next action or in case the current action is a `ActionKind::Choice` action.
#[derive(Event)]
pub struct NextActionRequest;

/// An event that requests to jump to a specific action in a screenplay.
///
/// This event is typically used to signal after a Player choice to jump to the action that is the result of the choice.
/// The `ActionId` to jump to is the one defined in the next field for the Choice choosen by the player.
#[derive(Event)]
pub struct JumpToActionRequest(pub ActionId);

/// An event that signals that actors have entered. It is sent when the `ActiveScreenplay` reaches an `ActionKind::Enter` action.
#[derive(Event)]
pub struct ActorsEnterEvent(pub Vec<Actor>);

/// An event that signals that actors have exited. It is sent when the `ActiveScreenplay` reaches an `ActionKind::Exit` action.
#[derive(Event)]
pub struct ActorsExitEvent(pub Vec<Actor>);
