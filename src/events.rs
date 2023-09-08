//! Events used in the plugin.

use bevy::prelude::{Entity, Event};
use petgraph::stable_graph::NodeIndex;

/// Event to request the next action in the `ActiveTalk`. It is sent with the entity with the `Talk` component to update.
///
/// This event is typically used wired to an input from the player, e.g. a mouse click to advance the current dialog.
/// `bevy_talks` has a system to react to these events. When it receives one, it takes
/// the Talk in the `ActiveTalk` resource and changes the current action to the next one
/// if present. It can fail in case there is no next action or in case the current action is a `ActionKind::Choice` action.
#[derive(Event)]
pub struct NextActionRequest(pub Entity);

/// An event that requests to jump to a specific action in a Talk.
///
/// This event is typically used to signal after a Player choice to jump to the action that is the result of the choice.
/// The `ActionId` to jump to is the one defined in the next field for the Choice choosen by the player.
#[derive(Event)]
pub struct JumpToActionRequest(pub Entity, pub NodeIndex);

/// An event that requests to initialize a Talk.
/// If used on an already initialized Talk, it will reset it.
#[derive(Event)]
pub struct InitTalkRequest(pub Entity);
