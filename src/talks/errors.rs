//! Errors that can happen while using the library
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
