use thiserror::Error;

/// Possible errors when creating a conversation
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ScreenplayParsingError {
    #[error("an empty script was used to build the conversation")]
    EmptyScript,
    #[error("the actor action {0} has specified a non existent actor {1}")]
    ActorNotFound(i32, String),
    #[error("the action {0} is pointing to id {1} which was not found")]
    NextActionNotFound(i32, i32),
    #[error("the dialogue line {0} has the same id as another dialogue")]
    RepeatedId(i32),
    #[error("no initial action was found, add a 'start': true to one of the actions")]
    NoStartingAction,
    #[error("too many actions with 'start' flag set to true. Only one allowed.")]
    MultipleStartingAction,
}

/// Errors when picking a choice
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ChoicesError {
    #[error("current action is not a player choice")]
    NotAChoiceAction,
    #[error("choice points to {0}, but it does not exist")]
    WrongId(i32),
}

/// Errors when moving to the next action
#[derive(Error, Debug, PartialEq, Eq)]
pub enum NextRequestError {
    #[error("current action has no next")]
    NoNextAction,
    #[error("called next_action() while in a player action")]
    ChoicesNotHandled,
}
