//! Prelude for the `talks` crate.
pub use super::TalksPlugin;
pub use super::{
    errors::*,
    events::*,
    talker::*,
    talks::{errors::*, raw_talk::*, talk::*, *},
};
