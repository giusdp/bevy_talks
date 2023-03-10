use bevy::{prelude::Component, utils::HashMap};
use serde::Deserialize;

#[derive(Clone, Default, Eq, PartialEq, Debug, Hash)]
pub enum ScreenplayState {
    #[default]
    Disabled,
    EnterPlay,
    Performing,
    PlayerChoosing,
    ExitPlay,
}

pub type ActionId = i32;

#[derive(Debug, Deserialize, Clone, Component, PartialEq, Eq)]
pub struct Actor {
    pub name: String,
    pub asset: String,
}
#[derive(Debug, Deserialize, Clone)]
pub struct Choice {
    pub text: String,
    pub next: ActionId,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    PlayerChoice,
    #[default]
    ActorTalk,
    ActorEnter,
    ActorExit,
}

impl From<ActorActionKind> for ActionKind {
    fn from(kind: ActorActionKind) -> Self {
        match kind {
            ActorActionKind::Talk => ActionKind::ActorTalk,
            ActorActionKind::Enter => ActionKind::ActorEnter,
            ActorActionKind::Exit => ActionKind::ActorExit,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawScreenplay {
    pub(crate) actors: HashMap<String, Actor>,
    pub(crate) script: Vec<ActorOrPlayerActionJSON>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ActorOrPlayerActionJSON {
    Actor(ActorAction),
    Player(PlayerAction),
}

impl ActorOrPlayerActionJSON {
    pub(crate) fn id(&self) -> ActionId {
        match self {
            ActorOrPlayerActionJSON::Actor(a) => a.id,
            ActorOrPlayerActionJSON::Player(p) => p.id,
        }
    }

    pub(crate) fn next(&self) -> Option<ActionId> {
        match self {
            ActorOrPlayerActionJSON::Actor(a) => a.next,
            ActorOrPlayerActionJSON::Player(_) => None,
        }
    }

    pub(crate) fn start(&self) -> Option<bool> {
        match self {
            ActorOrPlayerActionJSON::Actor(a) => a.start,
            ActorOrPlayerActionJSON::Player(p) => p.start,
        }
    }

    pub(crate) fn choices(&self) -> Option<&Vec<Choice>> {
        match self {
            ActorOrPlayerActionJSON::Actor(_) => None,
            ActorOrPlayerActionJSON::Player(p) => Some(&p.choices),
        }
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
pub(crate) struct ActorAction {
    pub(crate) id: ActionId,
    pub(crate) action: ActorActionKind,
    pub(crate) actors: Vec<String>,
    pub(crate) text: Option<String>,
    pub(crate) next: Option<ActionId>,
    pub(crate) start: Option<bool>,
    pub(crate) sound_effect: Option<String>,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub(crate) struct PlayerAction {
    pub(crate) id: ActionId,
    pub(crate) choices: Vec<Choice>,
    pub(crate) start: Option<bool>,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub(crate) enum ActorActionKind {
    #[default]
    #[serde(rename = "talk")]
    Talk,
    #[serde(rename = "enter")]
    Enter,
    #[serde(rename = "exit")]
    Exit,
}
