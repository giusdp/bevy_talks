//! Main actor types

use bevy::ecs::{bundle::Bundle, component::Component};

/// A unique identifier for an actor in a Talk.
///
/// The slug is a `String` that uniquely identifies an actor.
/// It is used to quickly find the actor.
///
pub(crate) type ActorSlug = String;

/// The actor component for the actor entities in a Talk.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    /// The name of the character that the actor plays.
    pub name: String,
    /// The unique slug of the character that the actor plays.
    pub slug: ActorSlug,
}

impl Actor {
    /// Creates a new actor with the given name and slug.
    pub fn new(slug: impl Into<ActorSlug>, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            slug: slug.into(),
        }
    }
}

/// A bundle that contains the components needed to make an entity an actor.
#[derive(Bundle)]
pub(crate) struct ActorBundle {
    /// The actor component.
    actor: Actor,
}
