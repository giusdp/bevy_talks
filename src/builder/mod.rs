//! Builder
use thiserror::Error;

pub mod builder;
pub mod loader;
pub mod types;

/// Errors when parsing a Talk json
#[derive(Error, Debug, PartialEq, Eq)]
pub enum JsonError {
    /// Serde failed to parse the json
    #[error("serde failed to parse the json: {0}")]
    BadParse(String),
    /// The Talk json is not valid
    #[error("the script is not valid: {0:?}")]
    Validation(Vec<String>),
}
