//! Variables: named game-state values the database defines.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::field::{Field, FieldValue};

/// A named game-state value and what it starts as.
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
pub struct Variable {
    /// Unique name within the database.
    pub name: String,
    /// The value the variable starts with.
    pub initial: FieldValue,
    /// Custom fields.
    #[serde(default)]
    pub fields: Vec<Field>,
}
