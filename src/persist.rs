//! Saving and restoring dialogue state across game sessions.
//!
//! The library doesn't own save files. [`DialogueSave`] is a serializable
//! snapshot of the dialogue state; your game stores it wherever it keeps its
//! saves and applies it back on load.
//!
//! ```rust,ignore
//! // Saving:
//! let text = save_to_ron(&DialogueSave::record(&variables, &visits))?;
//!
//! // Loading:
//! DialogueSave::apply(save_from_ron(&text)?, &mut variables, &mut visits);
//! ```
//!
//! Applying merges: saved values overwrite current ones, anything missing
//! from the save keeps its current value. Database seeding and old saves
//! therefore compose in either order, and saves from older game versions
//! keep working after the database grows.
//!
//! To also resume a conversation in progress, store
//! [`DialogueRunner::save_point`](crate::runtime::DialogueRunner::save_point)
//! in your own save and spawn
//! [`DialogueRunner::resume`](crate::runtime::DialogueRunner::resume) on load.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::data::{ConversationId, EntryId, FieldValue};
use crate::runtime::{Variables, Visits, visits::VisitCount};

/// A snapshot of everything the dialogue system remembers across sessions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DialogueSave {
    /// Saved variable values, keyed by name.
    #[serde(default)]
    pub variables: HashMap<String, FieldValue>,
    /// Saved visit counts, keyed by conversation and entry id.
    #[serde(default)]
    pub visits: HashMap<(ConversationId, EntryId), VisitCount>,
}

impl DialogueSave {
    /// A snapshot of the current dialogue state.
    pub fn record(variables: &Variables, visits: &Visits) -> Self {
        Self {
            variables: variables.0.clone(),
            visits: visits.0.clone(),
        }
    }

    /// Applies the snapshot, overwriting current values. Entries missing from
    /// the snapshot keep their current value.
    pub fn apply(self, variables: &mut Variables, visits: &mut Visits) {
        variables.0.extend(self.variables);
        visits.0.extend(self.visits);
    }
}

/// Serializes a save to RON text.
pub fn save_to_ron(save: &DialogueSave) -> Result<String, ron::Error> {
    ron::ser::to_string_pretty(save, ron::ser::PrettyConfig::default())
}

/// Parses a save from RON text.
pub fn save_from_ron(text: &str) -> Result<DialogueSave, ron::error::SpannedError> {
    ron::de::from_str(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{DialogueDatabase, Variable};

    fn db() -> DialogueDatabase {
        DialogueDatabase {
            variables: vec![
                Variable {
                    name: "AcceptedJob".to_owned(),
                    initial: FieldValue::Boolean(false),
                    fields: vec![],
                },
                Variable {
                    name: "Gold".to_owned(),
                    initial: FieldValue::Number(10.0),
                    fields: vec![],
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn ron_roundtrip() {
        let mut variables = Variables::default();
        variables.set("AcceptedJob", true);
        variables.set("PlayerName", "Feri");
        let mut visits = Visits::default();
        visits.record_displayed((ConversationId(1), EntryId(2)));
        visits.record_offered((ConversationId(1), EntryId(3)));

        let text = save_to_ron(&DialogueSave::record(&variables, &visits)).unwrap();
        let save = save_from_ron(&text).unwrap();

        let mut restored_variables = Variables::default();
        let mut restored_visits = Visits::default();
        save.apply(&mut restored_variables, &mut restored_visits);

        assert!(restored_variables.truthy("AcceptedJob"));
        assert_eq!(restored_variables.text("PlayerName"), "Feri");
        assert_eq!(restored_visits.displayed((ConversationId(1), EntryId(2))), 1);
        assert_eq!(restored_visits.offered((ConversationId(1), EntryId(3))), 1);
    }

    #[test]
    fn saved_values_overwrite_seeded_ones() {
        let mut variables = Variables::default();
        variables.seed(&db());
        let save = DialogueSave {
            variables: HashMap::from([("Gold".to_owned(), FieldValue::Number(99.0))]),
            visits: HashMap::new(),
        };
        save.apply(&mut variables, &mut Visits::default());
        assert_eq!(variables.number("Gold"), 99.0);
        assert!(!variables.truthy("AcceptedJob"));
    }

    #[test]
    fn seeding_after_restore_fills_only_missing_variables() {
        let mut variables = Variables::default();
        let save = DialogueSave {
            variables: HashMap::from([("Gold".to_owned(), FieldValue::Number(99.0))]),
            visits: HashMap::new(),
        };
        save.apply(&mut variables, &mut Visits::default());
        variables.seed(&db());
        assert_eq!(variables.number("Gold"), 99.0);
        assert!(!variables.truthy("AcceptedJob"));
    }
}
