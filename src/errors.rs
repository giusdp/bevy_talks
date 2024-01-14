//! Errors that can happen when using `bevy_talks`.

use thiserror::Error;

use crate::prelude::ActorSlug;

/// Errors when moving to the next action
#[derive(Error, Debug, PartialEq, Eq)]
pub enum NextActionError {
    /// NextActionRequest error.
    #[error("No next action found.")]
    NoNextAction,
    /// NextActionRequest error.
    #[error("Current node is a Choice. Cannot just advance.")]
    ChoicesNotHandled,
    /// ChooseActionRequest error.
    #[error("The current node has no choice with the given entity. (If the talk has not started you need to Next first)")]
    BadChoice,
    /// Requests error.
    #[error("No talk was found with the given entity from the event.")]
    NoTalk,
}

/// Errors from the builder
#[derive(Error, Debug, PartialEq, Eq)]
pub enum BuildError {
    /// An action has a non-existent actor
    #[error("Tried to use non-existent actor {0} in the builder. Did you forget to add it?")]
    InvalidActor(ActorSlug),
}
