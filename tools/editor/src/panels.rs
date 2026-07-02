//! Data-driven side panels: actor/conversation lists, inspector, status bar.

use bevy::prelude::*;
use bevy_talks::prelude::*;

use crate::state::{EditorSelection, EditorState, root_entry_id};
use crate::widgets::{form_field, labeled_value, list_row, muted_text, panel_header, selectable_row};

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

/// Rebuilds the actors list when the database changes.
pub fn rebuild_actors_panel(
    mut commands: Commands,
    state: Res<EditorState>,
    body: Single<Entity, With<ActorsPanelBody>>,
) {
    if !state.is_changed() {
        return;
    }
    let mut rows: Vec<Box<dyn Scene>> = state
        .db
        .actors
        .iter()
        .map(|actor| {
            let player = if actor.is_player { " (player)" } else { "" };
            Box::new(list_row(format!("{}  {}{}", actor.id.0, actor.name, player)))
                as Box<dyn Scene>
        })
        .collect();
    if rows.is_empty() {
        rows.push(Box::new(muted_text("(no actors)")));
    }
    commands.entity(*body).despawn_related::<Children>();
    commands
        .entity(*body)
        .queue_spawn_related_scenes::<Children>(rows);
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
    let mut rows: Vec<Box<dyn Scene>> = state
        .db
        .conversations
        .iter()
        .map(|conversation| {
            let id = conversation.id;
            let selected = selection.conversation == Some(id);
            Box::new(selectable_row(
                format!("{}  {}", id.0, conversation.title),
                selected,
                move |_: On<Pointer<Click>>,
                      mut selection: ResMut<EditorSelection>,
                      state: Res<EditorState>| {
                    selection.conversation = Some(id);
                    selection.entry = state.conversation(Some(id)).and_then(root_entry_id);
                },
            )) as Box<dyn Scene>
        })
        .collect();
    if rows.is_empty() {
        rows.push(Box::new(muted_text("(no conversations)")));
    }
    commands.entity(*body).despawn_related::<Children>();
    commands
        .entity(*body)
        .queue_spawn_related_scenes::<Children>(rows);
}

/// Rebuilds the inspector when the database or selection changes.
pub fn rebuild_inspector(
    mut commands: Commands,
    state: Res<EditorState>,
    selection: Res<EditorSelection>,
    body: Single<Entity, With<InspectorBody>>,
) {
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

    let mut rows: Vec<Box<dyn Scene>> = vec![
        Box::new(labeled_value("Entry", format!("{}", entry.id.0))),
        Box::new(labeled_value("Actor", state.actor_name(entry.actor))),
        Box::new(labeled_value("Conversant", state.actor_name(entry.conversant))),
        Box::new(form_field("Menu text", entry.menu_text.clone())),
        Box::new(form_field("Dialogue text", entry.dialogue_text.clone())),
        Box::new(labeled_value("Root entry", bool_text(entry.is_root))),
        Box::new(labeled_value("Group node", bool_text(entry.is_group))),
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
        rows.push(Box::new(list_row(format!(
            "→ conversation {} entry {}",
            link.dest_conversation.0, link.dest_entry.0
        ))));
    }
    rows
}

/// Displays a bool as text.
fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
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
pub fn update_status_text(
    state: Res<EditorState>,
    mut text: Single<&mut Text, With<StatusText>>,
) {
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
