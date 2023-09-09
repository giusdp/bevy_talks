//! Talker module
use bevy::prelude::{Bundle, Component};

use crate::prelude::{Actor, Choice, Talk, TalkNodeKind};

/// A bundle that contains the components needed to make an entity show a Talk
#[derive(Bundle, Default)]
pub struct TalkerBundle {
    /// The Talk to show.
    pub talk: Talk,
    /// The dialogue line component for a Talk.
    pub talk_text: CurrentText,
    /// The actor component that represents a character in a Talk.
    pub current_actors: CurrentActors,
    /// The Talk Node Kind component that represents the kind of action in a Talk.
    pub kind: CurrentNodeKind,
    /// The component that represents the current choices in a Talk.
    pub current_choices: CurrentChoices,
}

/// The dialogue line component for a Talk.
#[derive(Component, Default)]
pub struct CurrentText(pub String);

/// The actor component that represents a character in a Talk.
#[derive(Component, Default)]
pub struct CurrentActors(pub Vec<Actor>);

/// The component that represents the current choices in a Talk.
#[derive(Component, Default)]
pub struct CurrentChoices(pub Vec<Choice>);

/// The Talk Node Kind component that represents the kind of action in a Talk.
#[derive(Component, Default)]
pub struct CurrentNodeKind(pub TalkNodeKind);
