//! Errors that can happen while using the library
use thiserror::Error;

/// Errors when moving to the next action
#[derive(Error, Debug, PartialEq, Eq)]
pub enum NextActionError {
    /// Screenplay::next_action() was called on a screenplay
    /// where the current action has no next action.
    #[error("current action has no next")]
    NoNextAction,
}
