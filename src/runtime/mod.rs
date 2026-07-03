//! Playing conversations at runtime.
//!
//! Spawn a [`DialogueRunner`] pointing at a loaded
//! [`DialogueDatabase`](crate::data::DialogueDatabase) and observe the
//! entity events it emits:
//!
//! - [`SubtitleStarted`] : present a line, then trigger [`AdvanceConversation`].
//! - [`ResponseMenuOpened`] : present choices, then trigger [`ChooseResponse`].
//! - [`ConversationEnded`] : the runner is done and left in [`Phase::Ended`].
//!
//! The first NPC response auto-advances, player responses become a menu, group entries are
//! flattened, and the START entry's own text is skipped.

pub mod runner;
pub mod sequencer;
pub mod step;
pub mod variables;
pub mod visits;

pub use runner::{
    AdvanceConversation, ChooseResponse, ConversationEnded, DialogueRunner, Participants, Phase,
    ResponseMenuOpened, SubtitleStarted,
};
pub use sequencer::{
    Cue, CueSkipped, FinishCue, LineFinished, PlayingSequence, SequencerSettings, SkipLine, Skipped,
};
pub use step::{ConversationRef, Response, Step, Subtitle};
pub use variables::Variables;
pub use visits::{VisitCount, Visits};
