//! Loader errors.

use thiserror::Error;

/// A database load failure.
#[derive(Debug, Error)]
pub enum LoadError {
    /// Read failure.
    #[error("failed to read asset: {0}")]
    Io(#[from] std::io::Error),
    /// Parse failure.
    #[error("failed to parse RON: {0}")]
    Ron(#[from] ron::error::SpannedError),
}
