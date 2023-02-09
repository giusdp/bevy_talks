use crate::prelude::Choice;

pub struct NextDialogueEvent;
pub struct ChoicePickedEvent(pub i32);
pub struct ChoicesReachedEvent(pub Vec<Choice>);
