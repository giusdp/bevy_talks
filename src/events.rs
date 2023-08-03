use crate::{
    prelude::{ChoicesError, NextRequestError},
    types::{Actor, Choice},
};

// /// Event that the plugin can receive to move the screenplay to the next action.
// #[derive(Debug)]
// pub struct RequestNextActionEvent(pub Handle<Screenplay>);

// /// Event that the plugin can receive following a player choice.
// #[derive(Debug)]
// pub struct ChoicePickedEvent(pub Handle<Screenplay>, pub i32);

/// Event that the plugin can send to notify that the screenplay has moved to a talk action.
#[derive(Debug)]
pub struct TalkActionEvent {
    pub actors: Vec<Actor>,
    pub text: String,
    pub sound_effect: Option<String>,
}

/// Event that the plugin can send to notify that the screenplay has moved to an actor enter action.
#[derive(Debug)]
pub struct EnterActionEvent(pub Vec<Actor>);

/// Event that the plugin can send to notify that the screenplay has moved to an actor exit action.
#[derive(Debug)]
pub struct ExitActionEvent(pub Vec<Actor>);

/// Event that the plugin can send to notify that the screenplay has reached a player choice.
#[derive(Debug)]
pub struct ChoiceActionEvent(pub Vec<Choice>);

/// Event that the plugin can send to notify that it is not possible to go to the next action.
#[derive(Debug)]
pub struct NextActionErrorEvent(pub NextRequestError);

/// Event that the plugin can send to notify that it is not possible to pick a choice.
#[derive(Debug)]
pub struct ChoicePickedErrorEvent(pub ChoicesError);
