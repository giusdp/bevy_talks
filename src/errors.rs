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
    #[error("called next() while current dialogue has no next dialogue set")]
    NoNextDialogue,
    #[error("called next() while current dialogue has choices")]
    ChoicesNotHandled,
    #[error("called choices() while current dialogue has no choices")]
    NoChoices,

    #[error("tried to retrieve the current dialogue but there is none")]
    InvalidCurrentDialogue,
}
