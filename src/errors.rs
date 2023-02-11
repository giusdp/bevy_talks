use thiserror::Error;

/// Possible errors when creating a conversation
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ScriptParsingError {
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

/// Errors when interacting with a conversation
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ConversationError {
    #[error("current dialogue has no next dialogue")]
    NoNextDialogue,
    #[error("called next_line() while current dialogue has choices")]
    ChoicesNotHandled,
    #[error("called choices() while current dialogue has no choices")]
    NoChoices,
    #[error("tried to jump to dialogue {0}, but it does not exist")]
    WrongJump(i32),

    #[error("failed to access the current dialogue")]
    InvalidDialogue,
}
