//! Components that make a Talk

use bevy::prelude::{Component, Entity};

use super::Actor;

/// The dialogue line component for a Talk.
#[derive(Component, Default)]
pub struct DialogueLine {
    text: String,
    actors: Vec<Entity>,
}

/// The actor component that represents a character in a Talk.
#[derive(Component, Default)]
pub struct CurrentActors {
    name: Vec<Actor>,
}

// /// The Action Kind component that represents the kind of action in a Talk.
// #[derive(Component, Default)]
// pub struct ActionKind {}
