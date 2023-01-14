use bevy::prelude::Component;
use serde::Deserialize;

pub mod conversation;
pub mod local;
pub mod repo;

#[derive(Debug, Component)]
pub struct PrimaryConvo;

#[derive(Debug, Component)]
pub struct SecondaryConvo;

#[derive(Debug, Deserialize, Clone)]
pub struct Choice {
    pub text: String,
    pub next: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Talker {
    pub name: String,
    pub asset: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Dialogue {
    pub id: i32,
    pub text: String,
    pub talker: Talker,
    pub choices: Option<Vec<Choice>>,
    pub next: Option<i32>,
}

impl Dialogue {
    fn is_end(&self) -> bool {
        self.choices.is_none() && self.next.is_none()
    }

    fn has_next(&self) -> bool {
        self.next.is_some()
    }

    fn has_choices(&self) -> bool {
        self.choices.is_some()
    }
}
