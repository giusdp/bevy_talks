//! Errors for the builder module

use thiserror::Error;

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
    WrongJump(i32),
}

/// Errors when building a Talk
#[derive(Error, Debug, PartialEq, Eq)]
pub enum BuildTalkError {
    /// The actor id is duplicated
    #[error("the actor id {0} is duplicated")]
    DuplicateActorId(String),
    /// An action has a non-existent actor
    #[error("the action {0} has specified a non existent actor {1}")]
    InvalidActor(i32, String),
    /// Multiple actions have same id error
    #[error("multiple actions have same id: {0}")]
    DuplicateActionId(i32),
    /// An action has the next field pointing to a non-existent action
    #[error("the action {0} is pointing to id {1} which was not found")]
    InvalidNextAction(i32, i32),
    /// The Handle did not have a Talk loaded
    #[error("the handle did not have a Talk loaded")]
    RawTalkNotLoaded,
}
