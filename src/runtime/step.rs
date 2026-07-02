//! The conversation-stepping logic.

use std::collections::HashSet;

use crate::data::{
    ActorId, Conversation, ConversationId, DialogueDatabase, DialogueEntry, EntryId,
};

/// Recursion cap when flattening links through group entries.
pub const MAX_EVALUATE_DEPTH: usize = 128;

/// A conversation, referenced the human way (title) or the stable way (id).
#[derive(Debug, Clone, PartialEq)]
pub enum ConversationRef {
    /// By stable id.
    Id(ConversationId),
    /// By title; the first conversation with a matching title wins.
    Title(String),
}

/// One spoken line: who says what to whom.
#[derive(Debug, Clone, PartialEq)]
pub struct Subtitle {
    /// Conversation the line belongs to.
    pub conversation: ConversationId,
    /// The entry being spoken.
    pub entry: EntryId,
    /// The speaker.
    pub actor: ActorId,
    /// The listener.
    pub conversant: ActorId,
    /// The spoken text.
    pub text: String,
}

/// A valid destination reachable from an entry's links.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    /// Conversation of the destination entry.
    pub conversation: ConversationId,
    /// The destination entry.
    pub entry: EntryId,
    /// Menu label: `menu_text`, falling back to `dialogue_text`.
    pub text: String,
    /// Whether the destination is spoken by a player actor.
    pub is_player: bool,
}

/// What happens next from a given entry.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// An NPC line follows: present this subtitle.
    Line(Subtitle),
    /// The player chooses: present this menu.
    Menu(Vec<Response>),
    /// Nothing follows: the conversation is over.
    End,
}

/// Resolves a [`ConversationRef`] against a database.
pub fn find_conversation<'a>(
    db: &'a DialogueDatabase,
    conversation: &ConversationRef,
) -> Option<&'a Conversation> {
    match conversation {
        ConversationRef::Id(id) => db.conversations.iter().find(|c| c.id == *id),
        ConversationRef::Title(title) => db.conversations.iter().find(|c| &c.title == title),
    }
}

/// The root entry of a conversation, falling back to the first entry.
pub fn root_entry(conversation: &Conversation) -> Option<&DialogueEntry> {
    conversation
        .entries
        .iter()
        .find(|e| e.is_root)
        .or_else(|| conversation.entries.first())
}

/// Looks up an entry by conversation and entry id.
pub fn entry_at(db: &DialogueDatabase, at: (ConversationId, EntryId)) -> Option<&DialogueEntry> {
    db.conversations
        .iter()
        .find(|c| c.id == at.0)?
        .entries
        .iter()
        .find(|e| e.id == at.1)
}

/// The subtitle for the entry at the given position.
pub fn subtitle_at(db: &DialogueDatabase, at: (ConversationId, EntryId)) -> Option<Subtitle> {
    let entry = entry_at(db, at)?;
    Some(Subtitle {
        conversation: at.0,
        entry: at.1,
        actor: entry.actor,
        conversant: entry.conversant,
        text: entry.dialogue_text.clone(),
    })
}

/// All destinations reachable from an entry's links, in link order, with
/// group entries flattened (their links are followed transitively).
pub fn responses(db: &DialogueDatabase, from: (ConversationId, EntryId)) -> Vec<Response> {
    collect_responses(db, from, 0, &mut HashSet::new())
}

/// The responses behind every link of the entry at `from`.
fn collect_responses(
    db: &DialogueDatabase,
    from: (ConversationId, EntryId),
    depth: usize,
    visited: &mut HashSet<(ConversationId, EntryId)>,
) -> Vec<Response> {
    if depth > MAX_EVALUATE_DEPTH {
        return Vec::new();
    }
    entry_at(db, from)
        .into_iter()
        .flat_map(|entry| &entry.links)
        .map(|link| (link.dest_conversation, link.dest_entry))
        .flat_map(|dest| destination_responses(db, dest, depth, visited))
        .collect()
}

/// The responses one link destination contributes: none if already visited or
/// missing, its own transitive responses if it is a group, itself otherwise.
fn destination_responses(
    db: &DialogueDatabase,
    dest: (ConversationId, EntryId),
    depth: usize,
    visited: &mut HashSet<(ConversationId, EntryId)>,
) -> Vec<Response> {
    if !visited.insert(dest) {
        return Vec::new();
    }
    match entry_at(db, dest) {
        Some(entry) if entry.is_group => collect_responses(db, dest, depth + 1, visited),
        Some(entry) => vec![response(db, dest, entry)],
        None => Vec::new(),
    }
}

/// A [`Response`] presenting the given destination entry.
fn response(
    db: &DialogueDatabase,
    (conversation, entry): (ConversationId, EntryId),
    destination: &DialogueEntry,
) -> Response {
    let text = if destination.menu_text.is_empty() {
        &destination.dialogue_text
    } else {
        &destination.menu_text
    };
    Response {
        conversation,
        entry,
        text: text.clone(),
        is_player: actor_is_player(db, destination.actor),
    }
}

/// Whether an actor id refers to a player actor.
fn actor_is_player(db: &DialogueDatabase, actor: ActorId) -> bool {
    db.actors
        .iter()
        .find(|a| a.id == actor)
        .is_some_and(|a| a.is_player)
}

/// Decides what follows the entry at `at`:
/// the first NPC response wins and is auto-followed;
/// otherwise player responses become a menu;
/// otherwise the conversation ends.
pub fn step_from(db: &DialogueDatabase, at: (ConversationId, EntryId)) -> Step {
    let responses = responses(db, at);
    if let Some(npc) = responses.iter().find(|r| !r.is_player) {
        match subtitle_at(db, (npc.conversation, npc.entry)) {
            Some(subtitle) => Step::Line(subtitle),
            None => Step::End,
        }
    } else if !responses.is_empty() {
        Step::Menu(responses)
    } else {
        Step::End
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Actor, Link};
    use rstest::{fixture, rstest};

    /// player 0, npc 1; conversation 1: START(1) -> npc line(2) -> {pc 3, pc 4},
    /// 3 -> group(5) -> npc 6; 6 -> 2 (cycle back).
    #[fixture]
    fn db() -> DialogueDatabase {
        let entry = |id: i32, actor: i32, menu: &str, text: &str, group: bool, links: Vec<i32>| {
            DialogueEntry {
                id: EntryId(id),
                actor: ActorId(actor),
                conversant: ActorId(1 - actor),
                menu_text: menu.to_owned(),
                dialogue_text: text.to_owned(),
                is_root: id == 1,
                is_group: group,
                links: links
                    .into_iter()
                    .map(|to| Link {
                        dest_conversation: ConversationId(1),
                        dest_entry: EntryId(to),
                    })
                    .collect(),
                fields: vec![],
            }
        };
        DialogueDatabase {
            version: "1".to_owned(),
            variables: vec![],
            actors: vec![
                Actor {
                    id: ActorId(0),
                    name: "Player".to_owned(),
                    is_player: true,
                    fields: vec![],
                },
                Actor {
                    id: ActorId(1),
                    name: "Feri".to_owned(),
                    is_player: false,
                    fields: vec![],
                },
            ],
            conversations: vec![Conversation {
                id: ConversationId(1),
                title: "Test".to_owned(),
                actor: ActorId(0),
                conversant: ActorId(1),
                entries: vec![
                    entry(1, 1, "", "", false, vec![2]),
                    entry(2, 1, "", "Hello", false, vec![3, 4]),
                    entry(3, 0, "Ask", "What is this?", false, vec![5]),
                    entry(4, 0, "Leave", "Bye", false, vec![]),
                    entry(5, 1, "", "", true, vec![6]),
                    entry(6, 1, "", "The dialogue system for Bevy", false, vec![2]),
                ],
                fields: vec![],
            }],
        }
    }

    #[rstest]
    fn finds_conversation_by_id_and_title(db: DialogueDatabase) {
        assert!(find_conversation(&db, &ConversationRef::Id(ConversationId(1))).is_some());
        assert!(find_conversation(&db, &ConversationRef::Title("Test".to_owned())).is_some());
        assert!(find_conversation(&db, &ConversationRef::Title("Nope".to_owned())).is_none());
    }

    #[rstest]
    fn start_skips_root_and_presents_first_npc_line(db: DialogueDatabase) {
        let step = step_from(&db, (ConversationId(1), EntryId(1)));
        let Step::Line(subtitle) = step else {
            panic!("expected a line, got {step:?}");
        };
        assert_eq!(subtitle.text, "Hello");
        assert_eq!(subtitle.actor, ActorId(1));
    }

    #[rstest]
    fn player_responses_become_a_menu_with_menu_text_labels(db: DialogueDatabase) {
        let step = step_from(&db, (ConversationId(1), EntryId(2)));
        let Step::Menu(responses) = step else {
            panic!("expected a menu, got {step:?}");
        };
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].text, "Ask");
        assert_eq!(responses[1].text, "Leave");
        assert!(responses.iter().all(|r| r.is_player));
    }

    #[rstest]
    fn groups_are_flattened(db: DialogueDatabase) {
        // entry 3 links to group 5, which links to npc 6.
        let step = step_from(&db, (ConversationId(1), EntryId(3)));
        let Step::Line(subtitle) = step else {
            panic!("expected a line, got {step:?}");
        };
        assert_eq!(subtitle.entry, EntryId(6));
    }

    #[rstest]
    fn dead_end_ends_the_conversation(db: DialogueDatabase) {
        assert_eq!(step_from(&db, (ConversationId(1), EntryId(4))), Step::End);
    }

    #[rstest]
    fn cycles_terminate(db: DialogueDatabase) {
        // 6 links back to 2; evaluation must not hang.
        let step = step_from(&db, (ConversationId(1), EntryId(6)));
        assert!(matches!(step, Step::Line(_)));
    }

    #[rstest]
    fn menu_label_falls_back_to_dialogue_text(mut db: DialogueDatabase) {
        db.conversations[0].entries[2].menu_text.clear();
        let Step::Menu(responses) = step_from(&db, (ConversationId(1), EntryId(2))) else {
            panic!("expected a menu");
        };
        assert_eq!(responses[0].text, "What is this?");
    }
}
