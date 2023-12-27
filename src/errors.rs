//! Errors that can happen when using `bevy_talks`.

use thiserror::Error;

use crate::prelude::ActionId;

/// Errors when moving to the next action
#[derive(Error, Debug, PartialEq, Eq)]
pub enum NextActionError {
    /// Talk::next_action() was called on a Talk
    /// where the current action has no next action.
    #[error("current action has no next")]
    NoNextAction,
    /// Talk::next_action() was called on a Talk
    /// where the current action is a choice action.
    #[error("current action is a choice action")]
    ChoicesNotHandled,

    /// Talk::jump_to(id) was called on a Talk
    /// where an action with given id does not exist.
    #[error("jumped to action {0}, but it does not exist")]
    WrongJump(usize),
}

/// Errors when building a Talk
#[derive(Error, Debug, PartialEq, Eq)]
pub enum BuildTalkError {
    /// The talk is empty
    #[error("the talk is empty")]
    EmptyTalk,
    /// An action has a non-existent actor
    #[error("the action {0} has specified a non existent actor {1}")]
    InvalidActor(ActionId, String),
    /// An action has the next field pointing to a non-existent action
    #[error("the action {0} is pointing to id {1} which was not found")]
    InvalidNextAction(ActionId, ActionId),
    /// The Handle did not have a Talk loaded
    #[error("the handle did not have a Talk loaded")]
    RawTalkNotLoaded,
}
