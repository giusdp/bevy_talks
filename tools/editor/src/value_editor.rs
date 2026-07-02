//! A reusable widget that edits one [`FieldValue`] wherever it lives: a type dropdown plus a value control matching the current type.

use bevy::{
    feathers::{
        controls::{
            FeathersCheckbox, FeathersMenu, FeathersMenuButton, FeathersMenuItem,
            FeathersMenuPopup, FeathersTextInput, FeathersTextInputContainer,
        },
        theme::ThemedText,
    },
    prelude::*,
    text::EditableText,
    ui::Checked,
    ui_widgets::{Activate, ValueChange},
};
use bevy_talks::prelude::*;

use crate::panels::{SuppressInspectorRebuild, SuppressVariablesRebuild};
use crate::state::EditorState;
use crate::widgets::muted_text;

/// Where an edited value lives in the database.
#[derive(Clone, PartialEq)]
pub enum ValueSlot {
    /// A named field of an entry.
    EntryField(ConversationId, EntryId, String),
    /// A named field of an actor. Not built by any panel yet.
    #[expect(dead_code)]
    ActorField(ActorId, String),
    /// A named field of a conversation. Not built by any panel yet.
    #[expect(dead_code)]
    ConversationField(ConversationId, String),
    /// A variable's initial value, by index into the database's variables.
    VariableInitial(usize),
}

impl Default for ValueSlot {
    fn default() -> Self {
        Self::VariableInitial(0)
    }
}

impl ValueSlot {
    /// The value the slot points at, mutably.
    fn resolve_mut<'a>(&self, db: &'a mut DialogueDatabase) -> Option<&'a mut FieldValue> {
        match self {
            Self::EntryField(conversation, entry, name) => {
                let entry = db
                    .conversations
                    .iter_mut()
                    .find(|c| c.id == *conversation)?
                    .entries
                    .iter_mut()
                    .find(|e| e.id == *entry)?;
                field_mut(&mut entry.fields, name)
            }
            Self::ActorField(actor, name) => {
                let actor = db.actors.iter_mut().find(|a| a.id == *actor)?;
                field_mut(&mut actor.fields, name)
            }
            Self::ConversationField(conversation, name) => {
                let conversation = db
                    .conversations
                    .iter_mut()
                    .find(|c| c.id == *conversation)?;
                field_mut(&mut conversation.fields, name)
            }
            Self::VariableInitial(index) => db.variables.get_mut(*index).map(|v| &mut v.initial),
        }
    }
}

/// A named value in a fields bag, mutably.
fn field_mut<'a>(fields: &'a mut [Field], name: &str) -> Option<&'a mut FieldValue> {
    fields
        .iter_mut()
        .find(|f| f.title == name)
        .map(|f| &mut f.value)
}

/// Which slot a value text input writes, and as which variant.
#[derive(Component, Clone, Default, PartialEq)]
pub struct ValueTextTarget {
    /// The written slot.
    pub slot: ValueSlot,
    /// The variant the text is parsed into.
    pub kind: TextKind,
}

/// The [`FieldValue`] variants a text input can edit.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum TextKind {
    /// `FieldValue::Text`.
    #[default]
    Text,
    /// `FieldValue::Localization`.
    Localization,
    /// `FieldValue::Number`.
    Number,
}

/// The five types a value can have, for the type dropdown.
#[derive(Clone, Copy, PartialEq)]
enum ValueKind {
    /// Plain text.
    Text,
    /// A number.
    Number,
    /// A flag.
    Boolean,
    /// A localized text variant.
    Localization,
    /// An actor reference.
    Actor,
}

/// Every kind with its dropdown label.
const KINDS: [(ValueKind, &str); 5] = [
    (ValueKind::Text, "Text"),
    (ValueKind::Number, "Number"),
    (ValueKind::Boolean, "Boolean"),
    (ValueKind::Localization, "Localization"),
    (ValueKind::Actor, "Actor"),
];

/// The kind of a value.
fn kind_of(value: &FieldValue) -> ValueKind {
    match value {
        FieldValue::Text(_) => ValueKind::Text,
        FieldValue::Number(_) => ValueKind::Number,
        FieldValue::Boolean(_) => ValueKind::Boolean,
        FieldValue::Localization(_) => ValueKind::Localization,
        FieldValue::Actor(_) => ValueKind::Actor,
    }
}

/// The dropdown label of a value's kind.
fn kind_name(value: &FieldValue) -> &'static str {
    KINDS
        .iter()
        .find(|(kind, _)| *kind == kind_of(value))
        .map(|(_, name)| *name)
        .unwrap_or("Text")
}

/// A value displayed as plain text.
pub fn display_text(value: &FieldValue) -> String {
    match value {
        FieldValue::Text(text) | FieldValue::Localization(text) => text.clone(),
        FieldValue::Number(number) => number.to_string(),
        FieldValue::Boolean(boolean) => boolean.to_string(),
        FieldValue::Actor(id) => format!("actor {}", id.0),
    }
}

/// `value` converted to `kind`, carrying over what it can.
fn convert(value: &FieldValue, kind: ValueKind, first_actor: ActorId) -> FieldValue {
    match kind {
        ValueKind::Text => FieldValue::Text(display_text(value)),
        ValueKind::Localization => FieldValue::Localization(display_text(value)),
        ValueKind::Number => FieldValue::Number(match value {
            FieldValue::Number(n) => *n,
            FieldValue::Text(s) | FieldValue::Localization(s) => s.trim().parse().unwrap_or(0.0),
            _ => 0.0,
        }),
        ValueKind::Boolean => FieldValue::Boolean(matches!(value, FieldValue::Boolean(true))),
        ValueKind::Actor => match value {
            FieldValue::Actor(id) => FieldValue::Actor(*id),
            _ => FieldValue::Actor(first_actor),
        },
    }
}

/// A labeled row editing the value in `slot`: type dropdown + value control.
pub fn value_editor(
    label: String,
    slot: ValueSlot,
    value: &FieldValue,
    actors: Vec<(ActorId, String)>,
) -> Box<dyn Scene> {
    let body: Box<dyn SceneList> = Box::new(vec![
        Box::new(muted_text(label)) as Box<dyn Scene>,
        value_controls(slot, value, actors),
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

/// The unlabeled controls row: type dropdown + value control.
pub fn value_controls(
    slot: ValueSlot,
    value: &FieldValue,
    actors: Vec<(ActorId, String)>,
) -> Box<dyn Scene> {
    let parts: Box<dyn SceneList> = Box::new(vec![
        type_select(slot.clone(), value, &actors),
        value_control(slot, value, actors),
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

/// The dropdown that changes a value's type, converting in place.
fn type_select(
    slot: ValueSlot,
    value: &FieldValue,
    actors: &[(ActorId, String)],
) -> Box<dyn Scene> {
    let current = kind_name(value);
    let first_actor = actors.first().map(|(id, _)| *id).unwrap_or_default();
    let items: Vec<Box<dyn Scene>> = KINDS
        .into_iter()
        .map(|(kind, name)| {
            let slot = slot.clone();
            let write = move |_: On<Activate>, mut state: ResMut<EditorState>| {
                let db = &mut state.bypass_change_detection().db;
                let Some(value) = slot.resolve_mut(db) else {
                    return;
                };
                if kind_of(value) == kind {
                    return;
                }
                *value = convert(value, kind, first_actor);
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
    })
}

/// The control matching the value's current type.
fn value_control(
    slot: ValueSlot,
    value: &FieldValue,
    actors: Vec<(ActorId, String)>,
) -> Box<dyn Scene> {
    match value {
        FieldValue::Text(text) => text_control(slot, text.clone(), TextKind::Text),
        FieldValue::Localization(text) => text_control(slot, text.clone(), TextKind::Localization),
        FieldValue::Number(number) => text_control(slot, number.to_string(), TextKind::Number),
        FieldValue::Boolean(boolean) => bool_control(slot, *boolean),
        FieldValue::Actor(id) => actor_control(slot, *id, actors),
    }
}

/// A text input for `Text`, `Localization`, and `Number` values.
fn text_control(slot: ValueSlot, value: String, kind: TextKind) -> Box<dyn Scene> {
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
                        EditableText::new(value)
                        ValueTextTarget { slot: slot, kind: kind }
                    )
                ]
            )
        ]
    })
}

/// A checkbox for `Boolean` values.
fn bool_control(slot: ValueSlot, checked: bool) -> Box<dyn Scene> {
    let write = move |change: On<ValueChange<bool>>, mut state: ResMut<EditorState>| {
        let db = &mut state.bypass_change_detection().db;
        let Some(value) = slot.resolve_mut(db) else {
            return;
        };
        *value = FieldValue::Boolean(change.value);
        state.set_changed();
    };
    if checked {
        Box::new(bsn! {
            @FeathersCheckbox {
                @caption: bsn! { Text("true") ThemedText },
            }
            Checked
            on(write)
        })
    } else {
        Box::new(bsn! {
            @FeathersCheckbox {
                @caption: bsn! { Text("false") ThemedText },
            }
            on(write)
        })
    }
}

/// An actor dropdown for `Actor` values.
fn actor_control(
    slot: ValueSlot,
    current: ActorId,
    actors: Vec<(ActorId, String)>,
) -> Box<dyn Scene> {
    let current_name = actors
        .iter()
        .find(|(id, _)| *id == current)
        .map(|(_, name)| name.clone())
        .unwrap_or_else(|| format!("actor {}", current.0));
    let items: Vec<Box<dyn Scene>> = actors
        .into_iter()
        .map(|(id, name)| {
            let slot = slot.clone();
            let write = move |_: On<Activate>, mut state: ResMut<EditorState>| {
                let db = &mut state.bypass_change_detection().db;
                let Some(value) = slot.resolve_mut(db) else {
                    return;
                };
                *value = FieldValue::Actor(id);
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
        @FeathersMenu
        Children [
            (
                @FeathersMenuButton {
                    @caption: bsn! { Text(current_name) ThemedText },
                }
            ),
            (
                @FeathersMenuPopup
                Children [ {items} ]
            )
        ]
    })
}

/// Writes edited value text back into the slot each input targets. Number
/// text that doesn't parse keeps the last good value while the user types.
pub fn commit_value_text_edits(
    inputs: Query<(&EditableText, &ValueTextTarget), Changed<EditableText>>,
    mut state: ResMut<EditorState>,
    mut suppress: ResMut<SuppressInspectorRebuild>,
    mut suppress_variables: ResMut<SuppressVariablesRebuild>,
) {
    let mut wrote = false;
    for (input, target) in &inputs {
        let text = input.value().to_string();
        let new = match target.kind {
            TextKind::Text => FieldValue::Text(text),
            TextKind::Localization => FieldValue::Localization(text),
            TextKind::Number => match text.trim().parse::<f32>() {
                Ok(number) => FieldValue::Number(number),
                Err(_) => continue,
            },
        };
        let db = &mut state.bypass_change_detection().db;
        let Some(value) = target.slot.resolve_mut(db) else {
            continue;
        };
        // A kind mismatch means the value changed type under this input (the
        // type dropdown was used); the input is stale and must not write.
        let same_kind = matches!(
            (target.kind, &*value),
            (TextKind::Text, FieldValue::Text(_))
                | (TextKind::Localization, FieldValue::Localization(_))
                | (TextKind::Number, FieldValue::Number(_))
        );
        if !same_kind {
            continue;
        }
        if *value != new {
            *value = new;
            wrote = true;
        }
    }
    if wrote {
        state.set_changed();
        suppress.0 = true;
        suppress_variables.0 = true;
    }
}
