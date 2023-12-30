//! Errors that can happen when using `bevy_talks`.

use thiserror::Error;

use crate::prelude::ActorSlug;

/// Errors when moving to the next action
#[derive(Error, Debug, PartialEq, Eq)]
pub enum NextActionError {
    /// NextRequest event emitted for a talk where the current action has no next action.
    #[error("No next action found.")]
    NoNextAction,
    /// NextRequest event emitted for a talk where the current action is a choice action.
    #[error("Cannot advance a choice action.")]
    ChoicesNotHandled,
    /// JumpToActionRequest event emitted for a talk
    /// where an action with given id does not exist.
    #[error("jumped to action {0}, but it does not exist")]
    WrongJump(usize),
    /// NextRequest event emitted for a talk that does not exist.
    #[error("No talk was found")]
    NoTalk,
}

/// Errors when building a Talk
#[derive(Error, Debug, PartialEq, Eq)]
pub enum BuildTalkError {
    /// The talk is empty
    #[error("the talk is empty")]
    EmptyTalk,
    /// The Handle did not have a Talk loaded
    #[error("the handle did not have a Talk loaded")]
    TalkNotLoaded,
}

/// Errors when using an actor
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ActorError {
    /// An action has a non-existent actor
    #[error("Tried to use non-existent actor {0} in the builder. Did you forget to add it?")]
    Invalid(ActorSlug),
}
