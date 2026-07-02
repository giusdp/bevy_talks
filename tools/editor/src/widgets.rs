//! Shared scene builders for editor chrome: panels, rows, labels, buttons.

use bevy::{
    ecs::system::IntoObserverSystem,
    feathers::{
        controls::{ButtonVariant, FeathersButton, FeathersTextInput, FeathersTextInputContainer},
        theme::ThemedText,
    },
    prelude::*,
    text::EditableText,
    ui_widgets::Activate,
};

/// Border color used by all panel frames.
pub const PANEL_BORDER: Color = Color::srgb(0.20, 0.22, 0.25);
/// Background color used by all panel frames.
pub const PANEL_BG: Color = Color::srgb(0.10, 0.11, 0.13);
/// Background color of the selected row in a list.
pub const ROW_SELECTED_BG: Color = Color::srgb(0.18, 0.24, 0.31);

/// A bordered panel with a header and arbitrary content.
pub fn panel(title: &'static str, content: impl SceneList + 'static) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            padding: {px(8).all()},
            border: {px(1).all()},
        }
        BorderColor::all(PANEL_BORDER)
        BackgroundColor(PANEL_BG)
        Children [
            panel_header(title),
            {content}
        ]
    }
}

/// A panel section header.
pub fn panel_header(text: impl Into<String>) -> impl Scene {
    let text: String = text.into();
    bsn! {
        Text(text)
        ThemedText
        TextFont {
            font_size: FontSize::Px(15.0),
        }
    }
}

/// Large toolbar title text.
pub fn header_text(text: impl Into<String>) -> impl Scene {
    let text: String = text.into();
    bsn! {
        Text(text)
        ThemedText
        TextFont {
            font_size: FontSize::Px(18.0),
        }
    }
}

/// Small dimmed text.
pub fn muted_text(text: impl Into<String>) -> impl Scene {
    let text: String = text.into();
    bsn! {
        Text(text)
        ThemedText
        TextFont {
            font_size: FontSize::Px(12.0),
        }
        TextColor(Color::srgb(0.62, 0.66, 0.72))
    }
}

/// A plain, non-interactive list row.
pub fn list_row(text: impl Into<String>) -> impl Scene {
    let text: String = text.into();
    bsn! {
        Node {
            min_height: px(28),
            display: Display::Flex,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(px(8)),
            border_radius: BorderRadius::all(px(4)),
        }
        Children [ muted_text(text) ]
    }
}

/// A clickable list row with a selected state.
pub fn selectable_row<F, M>(text: String, selected: bool, on_click: F) -> impl Scene
where
    F: IntoObserverSystem<Pointer<Click>, (), M> + Clone + Send + Sync,
    M: 'static,
{
    let bg = if selected { ROW_SELECTED_BG } else { Color::NONE };
    let color = if selected {
        Color::srgb(0.90, 0.92, 0.95)
    } else {
        Color::srgb(0.62, 0.66, 0.72)
    };
    bsn! {
        Node {
            min_height: px(28),
            display: Display::Flex,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(px(8)),
            border_radius: BorderRadius::all(px(4)),
        }
        BackgroundColor(bg)
        on(on_click)
        Children [
            (
                Text(text)
                ThemedText
                TextFont {
                    font_size: FontSize::Px(12.0),
                }
                TextColor(color)
            )
        ]
    }
}

/// A label + value row, both read-only.
pub fn labeled_value(label: impl Into<String>, value: impl Into<String>) -> impl Scene {
    let label: String = label.into();
    let value: String = value.into();
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            min_height: px(24),
        }
        Children [
            muted_text(label),
            (
                Text(value)
                ThemedText
                TextFont {
                    font_size: FontSize::Px(12.0),
                }
            )
        ]
    }
}

/// A labeled text input showing a field of the selected entry.
pub fn form_field(label: &'static str, value: String) -> impl Scene {
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
                    )
                ]
            )
        ]
    }
}

/// A regular toolbar button that only logs for now.
pub fn toolbar_button(label: &'static str) -> impl Scene {
    bsn! {
        @FeathersButton {
            @caption: bsn! { Text(label) ThemedText },
        }
        on(|_: On<Activate>| {
            info!("editor action not implemented yet");
        })
    }
}

/// A primary (highlighted) toolbar button that only logs for now.
pub fn primary_toolbar_button(label: &'static str) -> impl Scene {
    bsn! {
        @FeathersButton {
            @caption: bsn! { Text(label) ThemedText },
            @variant: ButtonVariant::Primary,
        }
        on(|_: On<Activate>| {
            info!("editor action not implemented yet");
        })
    }
}
