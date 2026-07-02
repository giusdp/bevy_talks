//! Data-driven side panels: actor/conversation lists, inspector, status bar.

use bevy::{
    feathers::{
        controls::{
            FeathersCheckbox, FeathersListRow, FeathersListView, FeathersTextInput,
            FeathersTextInputContainer,
        },
        theme::ThemedText,
    },
    prelude::*,
    text::EditableText,
    ui::{Checked, Selected},
    ui_widgets::ValueChange,
};
use bevy_talks::prelude::*;

use crate::state::{EditorSelection, EditorState, root_entry_id};
use crate::widgets::{feathers_row, labeled_value, muted_text, panel_header};

/// Marker for the actors list body.
#[derive(Component, Default, Clone)]
pub struct ActorsPanelBody;

/// Marker for the conversations list body.
#[derive(Component, Default, Clone)]
pub struct ConversationsPanelBody;

/// Marker for the inspector content box.
#[derive(Component, Default, Clone)]
pub struct InspectorBody;

/// Marker for the "Conversation: …" heading above the canvas.
#[derive(Component, Default, Clone)]
pub struct ConversationTitleText;

/// Marker for the database summary text in the status bar.
#[derive(Component, Default, Clone)]
pub struct StatusText;

/// Marker for the validation result text in the status bar.
#[derive(Component, Default, Clone)]
pub struct ValidationText;

/// Which entry text a text input edits. Inputs carry their exact target so a
/// pending edit can never leak into a newly selected entry.
#[derive(Component, Clone, Copy, Default, PartialEq)]
pub struct EntryTextTarget {
    /// Conversation of the target entry.
    pub conversation: ConversationId,
    /// The target entry.
    pub entry: EntryId,
    /// Edits `dialogue_text` when true, `menu_text` otherwise.
    pub dialogue: bool,
}

/// The conversation a sidebar list row represents.
#[derive(Component, Clone, Copy, Default)]
struct ConversationRow {
    /// The represented conversation.
    conversation: ConversationId,
}

/// Set when an inspector edit wrote to the state, so the write-through
/// doesn't rebuild the inspector under the user's cursor.
#[derive(Resource, Default)]
pub struct SuppressInspectorRebuild(pub bool);

/// Rebuilds the actors list when the database changes.
pub fn rebuild_actors_panel(
    mut commands: Commands,
    state: Res<EditorState>,
    body: Single<Entity, With<ActorsPanelBody>>,
) {
    if !state.is_changed() {
        return;
    }
    let rows: Vec<Box<dyn Scene>> = state
        .db
        .actors
        .iter()
        .map(|actor| {
            let player = if actor.is_player { " (player)" } else { "" };
            Box::new(feathers_row(format!(
                "{}  {}{}",
                actor.id.0, actor.name, player
            ))) as Box<dyn Scene>
        })
        .collect();
    let rows: Box<dyn SceneList> = Box::new(rows);

    commands.entity(*body).despawn_related::<Children>();
    commands
        .entity(*body)
        .queue_spawn_related_scenes::<Children>(bsn_list![(
            @FeathersListView {
                @rows: {rows},
            }
        )]);
}

/// Rebuilds the conversations list when the database or selection changes.
pub fn rebuild_conversations_panel(
    mut commands: Commands,
    state: Res<EditorState>,
    selection: Res<EditorSelection>,
    body: Single<Entity, With<ConversationsPanelBody>>,
) {
    if !state.is_changed() && !selection.is_changed() {
        return;
    }
    let rows: Vec<Box<dyn Scene>> = state
        .db
        .conversations
        .iter()
        .map(|conversation| {
            conversation_row(
                format!("{}  {}", conversation.id.0, conversation.title),
                conversation.id,
                selection.conversation == Some(conversation.id),
            )
        })
        .collect();
    let rows: Box<dyn SceneList> = Box::new(rows);

    commands.entity(*body).despawn_related::<Children>();
    commands
        .entity(*body)
        .queue_spawn_related_scenes::<Children>(bsn_list![(
            @FeathersListView {
                @rows: {rows},
            }
            on(
                |change: On<ValueChange<Entity>>,
                 rows: Query<&ConversationRow>,
                 mut selection: ResMut<EditorSelection>,
                 state: Res<EditorState>| {
                    let Ok(row) = rows.get(change.value) else {
                        return;
                    };
                    selection.conversation = Some(row.conversation);
                    selection.entry =
                        state.conversation(Some(row.conversation)).and_then(root_entry_id);
                }
            )
        )]);
}

/// A selectable conversation row.
fn conversation_row(label: String, conversation: ConversationId, selected: bool) -> Box<dyn Scene> {
    if selected {
        Box::new(bsn! {
            @FeathersListRow
            Selected
            ConversationRow { conversation: conversation }
            Children [
                (
                    Text(label)
                    ThemedText
                    TextFont {
                        font_size: FontSize::Px(12.0),
                    }
                )
            ]
        })
    } else {
        Box::new(bsn! {
            @FeathersListRow
            ConversationRow { conversation: conversation }
            Children [
                (
                    Text(label)
                    ThemedText
                    TextFont {
                        font_size: FontSize::Px(12.0),
                    }
                )
            ]
        })
    }
}

/// Rebuilds the inspector when the database or selection changes.
pub fn rebuild_inspector(
    mut commands: Commands,
    state: Res<EditorState>,
    selection: Res<EditorSelection>,
    mut suppress: ResMut<SuppressInspectorRebuild>,
    body: Single<Entity, With<InspectorBody>>,
) {
    if selection.is_changed() {
        suppress.0 = false;
    } else if state.is_changed() && suppress.0 {
        suppress.0 = false;
        return;
    }
    if !state.is_changed() && !selection.is_changed() {
        return;
    }
    commands.entity(*body).despawn_related::<Children>();
    commands
        .entity(*body)
        .queue_spawn_related_scenes::<Children>(inspector_content(&state, &selection));
}

/// The inspector rows for the current selection.
fn inspector_content(state: &EditorState, selection: &EditorSelection) -> Vec<Box<dyn Scene>> {
    let Some(conversation) = state.conversation(selection.conversation) else {
        return vec![Box::new(muted_text("No conversation selected"))];
    };
    let Some(entry) = selection
        .entry
        .and_then(|id| conversation.entries.iter().find(|e| e.id == id))
    else {
        return vec![Box::new(muted_text("No entry selected"))];
    };

    let target = (conversation.id, entry.id);
    let mut rows: Vec<Box<dyn Scene>> = vec![
        Box::new(labeled_value("Entry", format!("{}", entry.id.0))),
        Box::new(labeled_value("Actor", state.actor_name(entry.actor))),
        Box::new(labeled_value(
            "Conversant",
            state.actor_name(entry.conversant),
        )),
        Box::new(entry_text_input(
            "Menu text",
            entry.menu_text.clone(),
            target,
            false,
        )),
        Box::new(entry_text_input(
            "Dialogue text",
            entry.dialogue_text.clone(),
            target,
            true,
        )),
        entry_flag_checkbox("Root entry", entry.is_root, target, EntryFlag::Root),
        entry_flag_checkbox("Group node", entry.is_group, target, EntryFlag::Group),
        Box::new(panel_header("Custom Fields")),
    ];
    if entry.fields.is_empty() {
        rows.push(Box::new(muted_text("(none)")));
    }
    for field in &entry.fields {
        rows.push(Box::new(labeled_value(
            field.title.clone(),
            field_value_text(&field.value),
        )));
    }
    rows.push(Box::new(panel_header("Outgoing Links")));
    if entry.links.is_empty() {
        rows.push(Box::new(muted_text("(none)")));
    }
    for link in &entry.links {
        rows.push(Box::new(feathers_row(format!(
            "→ conversation {} entry {}",
            link.dest_conversation.0, link.dest_entry.0
        ))));
    }
    rows
}

/// Which boolean flag of an entry a checkbox edits.
#[derive(Clone, Copy)]
enum EntryFlag {
    /// `is_root`.
    Root,
    /// `is_group`.
    Group,
}

/// A checkbox editing one boolean flag of an entry.
fn entry_flag_checkbox(
    label: &'static str,
    checked: bool,
    (conversation, entry): (ConversationId, EntryId),
    flag: EntryFlag,
) -> Box<dyn Scene> {
    let write = move |change: On<ValueChange<bool>>, mut state: ResMut<EditorState>| {
        let db = &mut state.bypass_change_detection().db;
        let Some(entry) = entry_mut(db, conversation, entry) else {
            return;
        };
        match flag {
            EntryFlag::Root => entry.is_root = change.value,
            EntryFlag::Group => entry.is_group = change.value,
        }
        state.set_changed();
    };
    if checked {
        Box::new(bsn! {
            @FeathersCheckbox {
                @caption: bsn! { Text(label) ThemedText },
            }
            Checked
            on(write)
        })
    } else {
        Box::new(bsn! {
            @FeathersCheckbox {
                @caption: bsn! { Text(label) ThemedText },
            }
            on(write)
        })
    }
}

/// A labeled text input that edits one text of a specific entry.
fn entry_text_input(
    label: &'static str,
    value: String,
    (conversation, entry): (ConversationId, EntryId),
    dialogue: bool,
) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
        }
        Children [
            muted_text(label),
            (
                @FeathersTextInputContainer
                Children [
                    (
                        @FeathersTextInput
                        EditableText::new(value)
                        EntryTextTarget {
                            conversation: conversation,
                            entry: entry,
                            dialogue: dialogue,
                        }
                    )
                ]
            )
        ]
    }
}

/// Writes edited inspector text back into the entry each input targets.
pub fn commit_entry_text_edits(
    inputs: Query<(&EditableText, &EntryTextTarget), Changed<EditableText>>,
    mut state: ResMut<EditorState>,
    mut suppress: ResMut<SuppressInspectorRebuild>,
) {
    let mut wrote = false;
    for (input, target) in &inputs {
        let value = input.value().to_string();
        let db = &mut state.bypass_change_detection().db;
        let Some(entry) = entry_mut(db, target.conversation, target.entry) else {
            continue;
        };
        let current = if target.dialogue {
            &mut entry.dialogue_text
        } else {
            &mut entry.menu_text
        };
        if *current != value {
            *current = value;
            wrote = true;
        }
    }
    if wrote {
        state.set_changed();
        suppress.0 = true;
    }
}

/// An entry looked up by conversation and entry id, mutably.
fn entry_mut(
    db: &mut DialogueDatabase,
    conversation: ConversationId,
    entry: EntryId,
) -> Option<&mut DialogueEntry> {
    db.conversations
        .iter_mut()
        .find(|c| c.id == conversation)?
        .entries
        .iter_mut()
        .find(|e| e.id == entry)
}

/// Displays a field value as text.
fn field_value_text(value: &FieldValue) -> String {
    match value {
        FieldValue::Text(text) | FieldValue::Localization(text) => text.clone(),
        FieldValue::Number(number) => number.to_string(),
        FieldValue::Boolean(boolean) => boolean.to_string(),
        FieldValue::Actor(id) => format!("actor {}", id.0),
    }
}

/// Keeps the canvas heading in sync with the selected conversation.
pub fn update_conversation_title(
    state: Res<EditorState>,
    selection: Res<EditorSelection>,
    mut text: Single<&mut Text, With<ConversationTitleText>>,
) {
    if !state.is_changed() && !selection.is_changed() {
        return;
    }
    text.0 = match state.conversation(selection.conversation) {
        Some(conversation) => format!("Conversation: {}", conversation.title),
        None => "No conversation selected".to_owned(),
    };
}

/// Keeps the database summary in the status bar in sync.
pub fn update_status_text(state: Res<EditorState>, mut text: Single<&mut Text, With<StatusText>>) {
    if !state.is_changed() {
        return;
    }
    let entries: usize = state.db.conversations.iter().map(|c| c.entries.len()).sum();
    text.0 = format!(
        "{} actors, {} conversations, {} entries",
        state.db.actors.len(),
        state.db.conversations.len(),
        entries
    );
}

/// Runs validation on the working copy and reports it in the status bar.
pub fn update_validation_text(
    state: Res<EditorState>,
    mut text: Single<(&mut Text, &mut TextColor), With<ValidationText>>,
) {
    if !state.is_changed() {
        return;
    }
    let issues = validate(&state.db);
    let (text, color) = &mut *text;
    if issues.is_empty() {
        text.0 = "Validation clean".to_owned();
        color.0 = Color::srgb(0.48, 0.80, 0.52);
    } else {
        for issue in &issues {
            warn!("validation: {issue}");
        }
        text.0 = format!("{} validation issues", issues.len());
        color.0 = Color::srgb(0.92, 0.68, 0.25);
    }
}
