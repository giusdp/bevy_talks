//! Components that make a Talk

use bevy::asset::HandleUntyped;
use bevy::prelude::{Component, Entity};

use crate::prelude::ActionKind;

/// The dialogue line component for a Talk.
#[derive(Component, Default)]
pub struct DialogueLine {
    text: String,
    actors: Vec<Entity>,
}

/// The actor component that represents a character in a Talk.
#[derive(Component, Default)]
pub struct Actor {
    name: String,
    asset: Option<HandleUntyped>,
}

/// The Action Kind component that represents the kind of action in a Talk.
#[derive(Component, Default)]
pub struct TalkActionKind {
    kind: ActionKind,
}
