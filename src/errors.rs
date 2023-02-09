use thiserror::Error;

/// Possible errors when creating a conversation
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ConvoCreationError {
    #[error("an empty lines vector was used to build the conversation")]
    NoLines,
    #[error("the dialogue line {0} has specified a non existent talker {1}")]
    TalkerNotFound(i32, String),
    #[error("the dialogue line {0} is pointing to id {1} which was not found")]
    NextLineNotFound(i32, i32),
    #[error("the dialogue line {0} has the same id as another dialogue")]
    RepeatedId(i32),
    #[error("no initial dialogue was found, add a 'start': true to one of the dialogue lines")]
    NoStartingDialogue,
    #[error("too many dialogues with 'start' flag set to true. Only one allowed.")]
    MultipleStartingDialogues,
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
