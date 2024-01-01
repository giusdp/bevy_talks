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
    #[error("Current node is a Choice. Cannot just advance.")]
    ChoicesNotHandled,
    /// ChooseActionRequest event emitted for a talk
    /// where an action with given id does not exist.
    #[error("A wrong entity was given to go to in the dialogue graph.")]
    BadChoice,
    /// NextRequest event emitted for a talk that does not exist.
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
