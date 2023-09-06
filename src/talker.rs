//! Talker module
use bevy::prelude::{Bundle, Component};

use crate::prelude::talk::Talk;

/// A bundle that contains the components needed to make an entity show a Talk
#[derive(Bundle, Default)]
pub struct TalkerBundle {
    /// The Talk to show.
    pub talk: Talk,
    /// The component that indicates whether a talker is active or not.
    pub activated: Activated,
    /// The component that indicates whether a talker is interactable or not.
    pub interaction: Interaction,
    // pub display: dyn TalkDisplay,
}

/// A component that indicates whether a talker is active or not.
#[derive(Component, Default)]
pub struct Activated(pub bool);

/// A component that indicates whether a talker is interactable or not.
#[derive(Component, Default)]
pub struct Interaction;
