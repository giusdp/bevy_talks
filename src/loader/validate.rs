//! Database validation issues.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use thiserror::Error;

use crate::data::ids::{ActorId, ConversationId, EntryId};
use crate::data::{Conversation, DialogueDatabase};

/// A problem found while validating a database. Non-fatal: reported, not enforced.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum Issue {
    /// Two actors share an id.
    #[error("duplicate actor id {0:?}")]
    DuplicateActor(ActorId),
    /// Two conversations share an id.
    #[error("duplicate conversation id {0:?}")]
    DuplicateConversation(ConversationId),
    /// Two entries in a conversation share an id.
    #[error("duplicate entry id {1:?} in conversation {0:?}")]
    DuplicateEntry(ConversationId, EntryId),
    /// A conversation has no root entry.
    #[error("conversation {0:?} has no root entry")]
    NoRoot(ConversationId),
    /// A conversation has more than one root entry.
    #[error("conversation {0:?} has multiple root entries")]
    MultipleRoots(ConversationId),
    /// A link points at a missing destination.
    #[error("dangling link from conversation {0:?} entry {1:?} to conversation {2:?} entry {3:?}")]
    DanglingLink(ConversationId, EntryId, ConversationId, EntryId),
}

/// Collects every issue in the database. An empty result means it's clean.
pub fn validate(db: &DialogueDatabase) -> Vec<Issue> {
    let index = EntryIndex::build(db);

    let actors = duplicates(db.actors.iter().map(|a| a.id)).map(Issue::DuplicateActor);
    let convos =
        duplicates(db.conversations.iter().map(|c| c.id)).map(Issue::DuplicateConversation);
    let entries = db.conversations.iter().flat_map(entry_issues);
    let links = db.conversations.iter().flat_map(|c| link_issues(c, &index));

    actors.chain(convos).chain(entries).chain(links).collect()
}

/// Duplicate-entry and root-count issues within one conversation.
fn entry_issues(c: &Conversation) -> impl Iterator<Item = Issue> + '_ {
    let dupes =
        duplicates(c.entries.iter().map(|e| e.id)).map(|id| Issue::DuplicateEntry(c.id, id));
    dupes.chain(root_issue(c))
}

/// The root-count issue for a conversation, if any.
fn root_issue(c: &Conversation) -> Option<Issue> {
    match c.entries.iter().filter(|e| e.is_root).count() {
        0 => Some(Issue::NoRoot(c.id)),
        1 => None,
        _ => Some(Issue::MultipleRoots(c.id)),
    }
}

/// Dangling-link issues for every link in one conversation.
fn link_issues<'a>(c: &'a Conversation, index: &'a EntryIndex) -> impl Iterator<Item = Issue> + 'a {
    c.entries.iter().flat_map(move |e| {
        e.links.iter().filter_map(move |l| {
            (!index.contains(l.dest_conversation, l.dest_entry)).then_some(Issue::DanglingLink(
                c.id,
                e.id,
                l.dest_conversation,
                l.dest_entry,
            ))
        })
    })
}

/// Yields each value that appears more than once, once per extra occurrence.
fn duplicates<T: Copy + Eq + Hash>(items: impl Iterator<Item = T>) -> impl Iterator<Item = T> {
    let mut seen = HashSet::new();
    items.filter(move |&x| !seen.insert(x))
}

/// The id and set of entry ids of a conversation.
fn conversation_entry_ids(c: &Conversation) -> (ConversationId, HashSet<EntryId>) {
    (c.id, c.entries.iter().map(|e| e.id).collect())
}

/// Lookup of which entry ids exist in each conversation.
struct EntryIndex(HashMap<ConversationId, HashSet<EntryId>>);

impl EntryIndex {
    /// Builds the index from a database.
    fn build(db: &DialogueDatabase) -> Self {
        Self(
            db.conversations
                .iter()
                .map(conversation_entry_ids)
                .collect(),
        )
    }

    /// Whether the given conversation contains the given entry.
    fn contains(&self, conversation: ConversationId, entry: EntryId) -> bool {
        self.0
            .get(&conversation)
            .is_some_and(|s| s.contains(&entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Conversation, DialogueDatabase, DialogueEntry, Link};
    use rstest::{fixture, rstest};

    #[fixture]
    fn database() -> DialogueDatabase {
        DialogueDatabase {
            version: String::new(),
            variables: vec![],
            actors: vec![],
            conversations: vec![Conversation {
                id: ConversationId(1),
                title: String::new(),
                actor: ActorId(0),
                conversant: ActorId(0),
                entries: vec![DialogueEntry {
                    id: EntryId(1),
                    actor: ActorId(0),
                    conversant: ActorId(0),
                    menu_text: String::new(),
                    dialogue_text: String::new(),
                    is_root: true,
                    is_group: false,
                    links: vec![],
                    fields: vec![],
                    condition: String::new(),
                    script: String::new(),
                    sequence: String::new(),
                }],
                fields: vec![],
            }],
        }
    }

    #[rstest]
    fn clean_database_has_no_issues(database: DialogueDatabase) {
        assert_eq!(validate(&database), vec![]);
    }

    #[rstest]
    fn reports_dangling_link(mut database: DialogueDatabase) {
        database.conversations[0].entries[0].links.push(Link {
            dest_conversation: ConversationId(1),
            dest_entry: EntryId(99),
        });
        assert_eq!(
            validate(&database),
            vec![Issue::DanglingLink(
                ConversationId(1),
                EntryId(1),
                ConversationId(1),
                EntryId(99)
            )]
        );
    }

    #[rstest]
    fn reports_missing_root(mut database: DialogueDatabase) {
        database.conversations[0].entries[0].is_root = false;
        assert_eq!(validate(&database), vec![Issue::NoRoot(ConversationId(1))]);
    }
}
