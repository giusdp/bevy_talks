use crate::types::Choice;

pub struct NextAction;
pub struct ChoicePickedEvent(pub i32);
pub struct ChoicesReachedEvent(pub Vec<Choice>);
