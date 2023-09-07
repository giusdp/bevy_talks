//! Talks module

use bevy::reflect::{Reflect, TypeUuid};
use serde::Deserialize;

use crate::builder::types::{Actor, ScriptAction};

pub mod components;
pub mod errors;
pub mod talk;

/// A struct that represents a raw Talk (as from the json format).
///
/// It contains a list of actors that appear in the Talk, and a list of actions that make up the Talk.
#[derive(Debug, Deserialize, Default, Clone, Reflect, TypeUuid)]
#[uuid = "413be529-bfeb-8c5b-9db0-4b8b380a2c47"]
#[reflect_value]
pub struct RawTalk {
    /// The list of actors that appear in the Talk.
    pub actors: Vec<Actor>,
    /// The list of actions that make up the Talk.
    pub script: Vec<ScriptAction>,
}
