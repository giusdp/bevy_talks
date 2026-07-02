//! Data-driven side panels: actor/conversation lists, inspector, status bar.

use bevy::{
    feathers::{
        controls::{
            ButtonVariant, FeathersCheckbox, FeathersListRow, FeathersListView, FeathersMenu,
            FeathersMenuButton, FeathersMenuItem, FeathersMenuPopup, FeathersTextInput,
            FeathersTextInputContainer,
        },
        theme::ThemedText,
    },
    prelude::*,
    text::EditableText,
    ui::{Checked, Selected},
    ui_widgets::{Activate, ValueChange},
};
use bevy_talks::prelude::*;

use crate::state::{self, EditorSelection, EditorState, FieldOwner, root_entry_id};
use crate::value_editor::{ValueSlot, value_controls};
use crate::widgets::{action_button, labeled_value, muted_text, panel_header};

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

/// Marker for the current file name in the toolbar.
#[derive(Component, Default, Clone)]
pub struct FileLabelText;

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

/// Which conversation a title text input renames.
#[derive(Component, Clone, Copy, Default)]
pub struct ConversationTitleTarget {
    /// The conversation being renamed.
    pub conversation: ConversationId,
}

/// Which actor a name text input renames.
#[derive(Component, Clone, Copy, Default)]
pub struct ActorNameTarget {
    /// The actor being renamed.
    pub actor: ActorId,
}

/// The actor a sidebar list row represents.
#[derive(Component, Clone, Copy, Default)]
struct ActorRow {
    /// The represented actor.
    actor: ActorId,
}

/// The conversation a sidebar list row represents.
#[derive(Component, Clone, Copy, Default)]
struct ConversationRow {
    /// The represented conversation.
    conversation: ConversationId,
}

/// Marker for the database files list body.
#[derive(Component, Default, Clone)]
pub struct DatabaseFilesBody;

/// The database file a list row represents.
#[derive(Component, Clone, Default)]
struct DatabaseFileRow {
    /// Asset path of the file.
    path: String,
}

/// Set when an inspector edit wrote to the state, so the write-through
/// doesn't rebuild the inspector under the user's cursor.
#[derive(Resource, Default)]
pub struct SuppressInspectorRebuild(pub bool);

/// Marker for the variables list body.
#[derive(Component, Default, Clone)]
pub struct VariablesPanelBody;

/// Which variable a name input renames, by index into the database's variables.
#[derive(Component, Clone, Copy, Default)]
pub struct VariableNameTarget {
    /// Index of the renamed variable.
    pub index: usize,
}

/// Set when a variables-panel edit wrote to the state, so the write-through
/// doesn't rebuild the panel under the user's cursor.
#[derive(Resource, Default)]
pub struct SuppressVariablesRebuild(pub bool);

/// Rebuilds the database files list when the edited database changes.
pub fn rebuild_database_files(
    mut commands: Commands,
    state: Res<EditorState>,
    body: Single<Entity, With<DatabaseFilesBody>>,
) {
    if !state.is_changed() {
        return;
    }
    let rows: Vec<Box<dyn Scene>> = state::database_files()
        .into_iter()
        .map(|file| {
            let selected = file == state.path;
            database_file_row(file, selected)
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
            on(open_selected_database)
        )]);
}

/// Opens the database file behind an activated row.
fn open_selected_database(
    change: On<ValueChange<Entity>>,
    rows: Query<&DatabaseFileRow>,
    mut commands: Commands,
    mut selection: ResMut<EditorSelection>,
) {
    let Ok(row) = rows.get(change.value) else {
        return;
    };
    match state::read_database(&row.path) {
        Ok(db) => {
            let first = db.conversations.first();
            selection.conversation = first.map(|c| c.id);
            selection.entry = first.and_then(root_entry_id);
            commands.insert_resource(EditorState {
                db,
                path: row.path.clone(),
            });
        }
        Err(err) => error!("failed to open {}: {err}", row.path),
    }
}

/// A selectable database file row.
fn database_file_row(path: String, selected: bool) -> Box<dyn Scene> {
    let label = path.clone();
    if selected {
        Box::new(bsn! {
            @FeathersListRow
            Selected
            DatabaseFileRow { path: path }
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
            DatabaseFileRow { path: path }
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

/// Rebuilds the actors list when the database or selection changes.
pub fn rebuild_actors_panel(
    mut commands: Commands,
    state: Res<EditorState>,
    selection: Res<EditorSelection>,
    body: Single<Entity, With<ActorsPanelBody>>,
) {
    if !state.is_changed() && !selection.is_changed() {
        return;
    }
    let rows: Vec<Box<dyn Scene>> = state
        .db
        .actors
        .iter()
        .map(|actor| {
            let player = if actor.is_player { " (player)" } else { "" };
            actor_row(
                format!("{}  {}{}", actor.id.0, actor.name, player),
                actor.id,
                selection.actor == Some(actor.id),
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
                 rows: Query<&ActorRow>,
                 mut selection: ResMut<EditorSelection>| {
                    if let Ok(row) = rows.get(change.value) {
                        selection.actor = Some(row.actor);
                    }
                }
            )
        )]);
}

/// A selectable actor row.
fn actor_row(label: String, actor: ActorId, selected: bool) -> Box<dyn Scene> {
    if selected {
        Box::new(bsn! {
            @FeathersListRow
            Selected
            ActorRow { actor: actor }
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
            ActorRow { actor: actor }
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

/// Adds a new actor and selects it.
pub fn create_actor(
    _: On<Activate>,
    state: Option<ResMut<EditorState>>,
    mut selection: ResMut<EditorSelection>,
) {
    let Some(mut state) = state else {
        return;
    };
    let id = state::add_actor(&mut state.bypass_change_detection().db);
    state.set_changed();
    selection.actor = Some(id);
}

/// Rebuilds the variables list when the database changes.
pub fn rebuild_variables_panel(
    mut commands: Commands,
    state: Res<EditorState>,
    mut suppress: ResMut<SuppressVariablesRebuild>,
    body: Single<Entity, With<VariablesPanelBody>>,
) {
    if !state.is_changed() {
        return;
    }
    if suppress.0 {
        suppress.0 = false;
        return;
    }
    let rows: Vec<Box<dyn Scene>> = if state.db.variables.is_empty() {
        vec![Box::new(muted_text("(none)"))]
    } else {
        state
            .db
            .variables
            .iter()
            .enumerate()
            .map(|(index, variable)| variable_row(index, variable, actor_options(&state)))
            .collect()
    };
    commands.entity(*body).despawn_related::<Children>();
    commands
        .entity(*body)
        .queue_spawn_related_scenes::<Children>(rows);
}

/// One variable: name input + remove button, then the value editor.
fn variable_row(
    index: usize,
    variable: &Variable,
    actors: Vec<(ActorId, String)>,
) -> Box<dyn Scene> {
    let name = variable.name.clone();
    let name_row: Box<dyn SceneList> = Box::new(vec![
        Box::new(bsn! {
            Node {
                flex_grow: 1.0,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
            }
            Children [
                (
                    @FeathersTextInputContainer
                    Children [
                        (
                            @FeathersTextInput
                            EditableText::new(name)
                            VariableNameTarget { index: index }
                        )
                    ]
                )
            ]
        }) as Box<dyn Scene>,
        Box::new(action_button(
            "✕",
            ButtonVariant::Normal,
            move |_: On<Activate>, mut state: ResMut<EditorState>| {
                if state::remove_variable(&mut state.bypass_change_detection().db, index) {
                    state.set_changed();
                }
            },
        )),
    ]);
    let body: Box<dyn SceneList> = Box::new(vec![
        Box::new(bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
                align_items: AlignItems::Center,
            }
            Children [ {name_row} ]
        }) as Box<dyn Scene>,
        value_controls(ValueSlot::VariableInitial(index), &variable.initial, actors),
    ]);
    Box::new(bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            padding: {px(2).bottom()},
        }
        Children [ {body} ]
    })
}

/// Marker for a "new field name" text input, tagged with the fields bag the
/// neighboring Add button targets.
#[derive(Component, Default, Clone)]
struct NewFieldName {
    /// The targeted fields bag.
    owner: FieldOwner,
}

/// The value slot of a named field of `owner`.
fn owner_slot(owner: FieldOwner, title: String) -> ValueSlot {
    match owner {
        FieldOwner::Entry(conversation, entry) => ValueSlot::EntryField(conversation, entry, title),
        FieldOwner::Actor(actor) => ValueSlot::ActorField(actor, title),
        FieldOwner::Conversation(conversation) => ValueSlot::ConversationField(conversation, title),
    }
}

/// The rows of one fields bag: a row per field and the "name + Add" row.
/// `canvas_*` bookkeeping fields are hidden; the graph canvas owns them.
fn fields_section(
    owner: FieldOwner,
    fields: &[Field],
    actors: Vec<(ActorId, String)>,
) -> Vec<Box<dyn Scene>> {
    let mut rows: Vec<Box<dyn Scene>> = Vec::new();
    let visible: Vec<&Field> = fields
        .iter()
        .filter(|f| !f.title.starts_with("canvas_"))
        .collect();
    if visible.is_empty() {
        rows.push(Box::new(muted_text("(none)")));
    }
    for field in visible {
        rows.push(field_row(owner, field, actors.clone()));
    }
    rows.push(add_field_row(owner));
    rows
}

/// One custom field: title + remove button, then the value editor.
fn field_row(owner: FieldOwner, field: &Field, actors: Vec<(ActorId, String)>) -> Box<dyn Scene> {
    let title = field.title.clone();
    let remove_title = title.clone();
    let title_row: Box<dyn SceneList> = Box::new(vec![
        Box::new(bsn! {
            Node { flex_grow: 1.0 }
            Children [ muted_text(title) ]
        }) as Box<dyn Scene>,
        Box::new(action_button(
            "✕",
            ButtonVariant::Normal,
            move |_: On<Activate>, mut state: ResMut<EditorState>| {
                if state::remove_field(
                    &mut state.bypass_change_detection().db,
                    owner,
                    &remove_title,
                ) {
                    state.set_changed();
                }
            },
        )),
    ]);
    let body: Box<dyn SceneList> = Box::new(vec![
        Box::new(bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
                align_items: AlignItems::Center,
            }
            Children [ {title_row} ]
        }) as Box<dyn Scene>,
        value_controls(owner_slot(owner, field.title.clone()), &field.value, actors),
    ]);
    Box::new(bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
        }
        Children [ {body} ]
    })
}

/// The "name + Add" row that creates a new custom field on `owner`.
fn add_field_row(owner: FieldOwner) -> Box<dyn Scene> {
    let parts: Box<dyn SceneList> = Box::new(vec![
        Box::new(bsn! {
            Node {
                flex_grow: 1.0,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
            }
            Children [
                (
                    @FeathersTextInputContainer
                    Children [
                        (
                            @FeathersTextInput
                            EditableText::new("")
                            NewFieldName { owner: owner }
                        )
                    ]
                )
            ]
        }) as Box<dyn Scene>,
        Box::new(action_button(
            "Add",
            ButtonVariant::Normal,
            move |_: On<Activate>,
                  names: Query<(&EditableText, &NewFieldName)>,
                  mut state: ResMut<EditorState>| {
                let Some((name, _)) = names.iter().find(|(_, tag)| tag.owner == owner) else {
                    return;
                };
                let title = name.value().to_string().trim().to_owned();
                if state::add_field(&mut state.bypass_change_detection().db, owner, &title) {
                    state.set_changed();
                }
            },
        )),
    ]);
    Box::new(bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: px(6),
            align_items: AlignItems::Center,
        }
        Children [ {parts} ]
    })
}

/// Adds a new variable to the database.
pub fn create_variable(_: On<Activate>, state: Option<ResMut<EditorState>>) {
    let Some(mut state) = state else {
        return;
    };
    state::add_variable(&mut state.bypass_change_detection().db);
    state.set_changed();
}

/// Writes edited variable names back into the database.
pub fn commit_variable_name_edits(
    inputs: Query<(&EditableText, &VariableNameTarget), Changed<EditableText>>,
    mut state: ResMut<EditorState>,
    mut suppress: ResMut<SuppressVariablesRebuild>,
) {
    let mut wrote = false;
    for (input, target) in &inputs {
        let value = input.value().to_string();
        let db = &mut state.bypass_change_detection().db;
        let Some(variable) = db.variables.get_mut(target.index) else {
            continue;
        };
        if variable.name != value {
            variable.name = value;
            wrote = true;
        }
    }
    if wrote {
        state.set_changed();
        suppress.0 = true;
    }
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
                    selection.actor = None;
                }
            )
        )]);
}

/// Adds a new conversation with its START entry and selects it.
pub fn create_conversation(
    _: On<Activate>,
    state: Option<ResMut<EditorState>>,
    mut selection: ResMut<EditorSelection>,
) {
    let Some(mut state) = state else {
        return;
    };
    let id = state::add_conversation(&mut state.bypass_change_detection().db);
    state.set_changed();
    selection.conversation = Some(id);
    selection.entry = state.conversation(Some(id)).and_then(root_entry_id);
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
    let mut rows: Vec<Box<dyn Scene>> = vec![
        Box::new(panel_header("Conversation")),
        conversation_title_input(conversation.id, conversation.title.clone()),
        conversation_actor_select(
            "Default actor",
            state.actor_name(conversation.actor),
            actor_options(state),
            conversation.id,
            false,
        ),
        conversation_actor_select(
            "Default conversant",
            state.actor_name(conversation.conversant),
            actor_options(state),
            conversation.id,
            true,
        ),
        Box::new(panel_header("Conversation Fields")),
    ];
    rows.extend(fields_section(
        FieldOwner::Conversation(conversation.id),
        &conversation.fields,
        actor_options(state),
    ));
    if let Some(actor) = selection
        .actor
        .and_then(|id| state.db.actors.iter().find(|a| a.id == id))
    {
        rows.extend([
            Box::new(panel_header("Actor")) as Box<dyn Scene>,
            actor_name_input(actor.id, actor.name.clone()),
            actor_player_checkbox(actor.id, actor.is_player),
            Box::new(panel_header("Actor Fields")),
        ]);
        rows.extend(fields_section(
            FieldOwner::Actor(actor.id),
            &actor.fields,
            actor_options(state),
        ));
    }
    let Some(entry) = selection
        .entry
        .and_then(|id| conversation.entries.iter().find(|e| e.id == id))
    else {
        rows.push(Box::new(muted_text("No entry selected")));
        return rows;
    };

    let target = (conversation.id, entry.id);
    rows.extend([
        Box::new(panel_header("Entry")) as Box<dyn Scene>,
        Box::new(labeled_value("Entry", format!("{}", entry.id.0))),
        actor_select(
            "Actor",
            state.actor_name(entry.actor),
            actor_options(state),
            target,
            false,
        ),
        actor_select(
            "Conversant",
            state.actor_name(entry.conversant),
            actor_options(state),
            target,
            true,
        ),
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
    ]);
    rows.extend(fields_section(
        FieldOwner::Entry(conversation.id, entry.id),
        &entry.fields,
        actor_options(state),
    ));
    rows.push(Box::new(panel_header("Outgoing Links")));
    if entry.links.is_empty() {
        rows.push(Box::new(muted_text("(none)")));
    } else {
        rows.push(Box::new(muted_text("click a link to remove it")));
        let link_rows: Vec<Box<dyn Scene>> = entry
            .links
            .iter()
            .map(|&link| link_row(conversation.id, entry.id, link))
            .collect();
        let link_rows: Box<dyn SceneList> = Box::new(link_rows);
        rows.push(Box::new(bsn! {
            @FeathersListView {
                @rows: {link_rows},
            }
            on(remove_selected_link)
        }));
    }
    rows.push(add_child_button(target));
    if !entry.is_root {
        rows.push(delete_entry_button(target));
    }
    rows
}

/// The outgoing link a list row represents.
#[derive(Component, Clone, Copy, Default)]
struct LinkRow {
    /// Conversation of the source entry.
    conversation: ConversationId,
    /// The source entry.
    from: EntryId,
    /// The represented link.
    link: Link,
}

/// A removable outgoing-link row.
fn link_row(conversation: ConversationId, from: EntryId, link: Link) -> Box<dyn Scene> {
    let label = format!(
        "→ conversation {} entry {}",
        link.dest_conversation.0, link.dest_entry.0
    );
    Box::new(bsn! {
        @FeathersListRow
        LinkRow { conversation: conversation, from: from, link: link }
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

/// Removes the link behind an activated row.
fn remove_selected_link(
    change: On<ValueChange<Entity>>,
    rows: Query<&LinkRow>,
    state: Option<ResMut<EditorState>>,
) {
    let Ok(row) = rows.get(change.value) else {
        return;
    };
    let Some(mut state) = state else {
        return;
    };
    if state::remove_link(
        &mut state.bypass_change_detection().db,
        row.conversation,
        row.from,
        row.link,
    ) {
        state.set_changed();
    }
}

/// A button that deletes the given entry and its incoming links.
fn delete_entry_button((conversation, entry): (ConversationId, EntryId)) -> Box<dyn Scene> {
    Box::new(action_button(
        "Delete Entry",
        ButtonVariant::Normal,
        move |_: On<Activate>,
              state: Option<ResMut<EditorState>>,
              mut selection: ResMut<EditorSelection>| {
            let Some(mut state) = state else {
                return;
            };
            if state::delete_entry(&mut state.bypass_change_detection().db, conversation, entry) {
                state.set_changed();
                selection.entry = state
                    .conversation(Some(conversation))
                    .and_then(root_entry_id);
            }
        },
    ))
}

/// A button that creates a child entry linked from the given entry.
fn add_child_button((conversation, entry): (ConversationId, EntryId)) -> Box<dyn Scene> {
    Box::new(action_button(
        "Add Child Entry",
        ButtonVariant::Primary,
        move |_: On<Activate>,
              state: Option<ResMut<EditorState>>,
              mut selection: ResMut<EditorSelection>| {
            let Some(mut state) = state else {
                return;
            };
            let Some(child) = state::add_child_entry(
                &mut state.bypass_change_detection().db,
                conversation,
                entry,
            ) else {
                return;
            };
            state.set_changed();
            selection.entry = Some(child);
        },
    ))
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

/// A labeled text input that renames a conversation.
fn conversation_title_input(conversation: ConversationId, value: String) -> Box<dyn Scene> {
    Box::new(bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
        }
        Children [
            muted_text("Title"),
            (
                @FeathersTextInputContainer
                Children [
                    (
                        @FeathersTextInput
                        EditableText::new(value)
                        ConversationTitleTarget { conversation: conversation }
                    )
                ]
            )
        ]
    })
}

/// All actors as dropdown options.
fn actor_options(state: &EditorState) -> Vec<(ActorId, String)> {
    state
        .db
        .actors
        .iter()
        .map(|a| (a.id, a.name.clone()))
        .collect()
}

/// A labeled dropdown that assigns an entry's actor or conversant.
fn actor_select(
    label: &'static str,
    current: String,
    actors: Vec<(ActorId, String)>,
    (conversation, entry): (ConversationId, EntryId),
    conversant: bool,
) -> Box<dyn Scene> {
    let items: Vec<Box<dyn Scene>> = actors
        .into_iter()
        .map(|(id, name)| {
            let write = move |_: On<Activate>, mut state: ResMut<EditorState>| {
                let db = &mut state.bypass_change_detection().db;
                let Some(e) = entry_mut(db, conversation, entry) else {
                    return;
                };
                if conversant {
                    e.conversant = id;
                } else {
                    e.actor = id;
                }
                state.set_changed();
            };
            Box::new(bsn! {
                @FeathersMenuItem {
                    @caption: bsn! { Text(name) ThemedText },
                }
                on(write)
            }) as Box<dyn Scene>
        })
        .collect();
    let items: Box<dyn SceneList> = Box::new(items);

    Box::new(bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
        }
        Children [
            muted_text(label),
            (
                @FeathersMenu
                Children [
                    (
                        @FeathersMenuButton {
                            @caption: bsn! { Text(current) ThemedText },
                        }
                    ),
                    (
                        @FeathersMenuPopup
                        Children [ {items} ]
                    )
                ]
            )
        ]
    })
}

/// A labeled dropdown that assigns a conversation's default actor or conversant.
fn conversation_actor_select(
    label: &'static str,
    current: String,
    actors: Vec<(ActorId, String)>,
    conversation: ConversationId,
    conversant: bool,
) -> Box<dyn Scene> {
    let items: Vec<Box<dyn Scene>> = actors
        .into_iter()
        .map(|(id, name)| {
            let write = move |_: On<Activate>, mut state: ResMut<EditorState>| {
                let db = &mut state.bypass_change_detection().db;
                let Some(c) = db.conversations.iter_mut().find(|c| c.id == conversation) else {
                    return;
                };
                if conversant {
                    c.conversant = id;
                } else {
                    c.actor = id;
                }
                state.set_changed();
            };
            Box::new(bsn! {
                @FeathersMenuItem {
                    @caption: bsn! { Text(name) ThemedText },
                }
                on(write)
            }) as Box<dyn Scene>
        })
        .collect();
    let items: Box<dyn SceneList> = Box::new(items);

    Box::new(bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
        }
        Children [
            muted_text(label),
            (
                @FeathersMenu
                Children [
                    (
                        @FeathersMenuButton {
                            @caption: bsn! { Text(current) ThemedText },
                        }
                    ),
                    (
                        @FeathersMenuPopup
                        Children [ {items} ]
                    )
                ]
            )
        ]
    })
}

/// A labeled text input that renames an actor.
fn actor_name_input(actor: ActorId, value: String) -> Box<dyn Scene> {
    Box::new(bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
        }
        Children [
            muted_text("Name"),
            (
                @FeathersTextInputContainer
                Children [
                    (
                        @FeathersTextInput
                        EditableText::new(value)
                        ActorNameTarget { actor: actor }
                    )
                ]
            )
        ]
    })
}

/// A checkbox editing an actor's `is_player` flag.
fn actor_player_checkbox(actor: ActorId, checked: bool) -> Box<dyn Scene> {
    let write = move |change: On<ValueChange<bool>>, mut state: ResMut<EditorState>| {
        let db = &mut state.bypass_change_detection().db;
        let Some(a) = db.actors.iter_mut().find(|a| a.id == actor) else {
            return;
        };
        a.is_player = change.value;
        state.set_changed();
    };
    if checked {
        Box::new(bsn! {
            @FeathersCheckbox {
                @caption: bsn! { Text("Player character") ThemedText },
            }
            Checked
            on(write)
        })
    } else {
        Box::new(bsn! {
            @FeathersCheckbox {
                @caption: bsn! { Text("Player character") ThemedText },
            }
            on(write)
        })
    }
}

/// Writes edited actor names back into the database.
pub fn commit_actor_name_edits(
    inputs: Query<(&EditableText, &ActorNameTarget), Changed<EditableText>>,
    mut state: ResMut<EditorState>,
    mut suppress: ResMut<SuppressInspectorRebuild>,
) {
    let mut wrote = false;
    for (input, target) in &inputs {
        let value = input.value().to_string();
        let db = &mut state.bypass_change_detection().db;
        let Some(actor) = db.actors.iter_mut().find(|a| a.id == target.actor) else {
            continue;
        };
        if actor.name != value {
            actor.name = value;
            wrote = true;
        }
    }
    if wrote {
        state.set_changed();
        suppress.0 = true;
    }
}

/// Writes edited conversation titles back into the database.
pub fn commit_conversation_title_edits(
    inputs: Query<(&EditableText, &ConversationTitleTarget), Changed<EditableText>>,
    mut state: ResMut<EditorState>,
    mut suppress: ResMut<SuppressInspectorRebuild>,
) {
    let mut wrote = false;
    for (input, target) in &inputs {
        let value = input.value().to_string();
        let db = &mut state.bypass_change_detection().db;
        let Some(conversation) = db
            .conversations
            .iter_mut()
            .find(|c| c.id == target.conversation)
        else {
            continue;
        };
        if conversation.title != value {
            conversation.title = value;
            wrote = true;
        }
    }
    if wrote {
        state.set_changed();
        suppress.0 = true;
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

/// Keeps the toolbar file name in sync with the edited database.
pub fn update_file_label(
    state: Res<EditorState>,
    mut text: Single<&mut Text, With<FileLabelText>>,
) {
    if !state.is_changed() {
        return;
    }
    text.0 = format!("assets/{}", state.path);
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
