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

/// Errors when parsing a screenplay json
#[derive(Error, Debug, PartialEq, Eq)]
pub enum JsonError {
    /// Serde failed to parse the json
    #[error("serde failed to parse the json: {0}")]
    BadParse(String),
    /// The screenplay json is not valid
    #[error("the script is not valid: {0:?}")]
    JSONValidation(Vec<String>),
}

/// Errors when parsing a screenplay json
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ScreenplayError {
    /// Multiple actions have same id
    #[error("multiple actions have same id: {0}")]
    DuplicateActionId(String),
}
