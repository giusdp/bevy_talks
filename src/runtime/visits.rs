//! Visit tracking: how often each entry has been offered and displayed.
//!
//! The runner records into [`Visits`] as conversations play.
//! The store is sparse: entries never reached have no record and cost nothing.

use std::collections::HashMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::{ConversationId, EntryId};

/// How often one entry was offered and displayed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisitCount {
    /// Times the entry appeared in a response menu.
    pub offered: u32,
    /// Times the entry was presented as a line.
    pub displayed: u32,
}

/// Visit counts for every entry that has been reached, keyed by conversation
/// and entry id.
#[derive(Resource, Debug, Default)]
pub struct Visits(pub HashMap<(ConversationId, EntryId), VisitCount>);

impl Visits {
    /// The counts for an entry; zero if never reached.
    pub fn count(&self, at: (ConversationId, EntryId)) -> VisitCount {
        self.0.get(&at).copied().unwrap_or_default()
    }

    /// Times the entry was presented as a line.
    pub fn displayed(&self, at: (ConversationId, EntryId)) -> u32 {
        self.count(at).displayed
    }

    /// Times the entry appeared in a response menu.
    pub fn offered(&self, at: (ConversationId, EntryId)) -> u32 {
        self.count(at).offered
    }

    /// Records the entry being presented as a line.
    pub fn record_displayed(&mut self, at: (ConversationId, EntryId)) {
        self.0.entry(at).or_default().displayed += 1;
    }

    /// Records the entry being offered in a menu.
    pub fn record_offered(&mut self, at: (ConversationId, EntryId)) {
        self.0.entry(at).or_default().offered += 1;
    }
}
