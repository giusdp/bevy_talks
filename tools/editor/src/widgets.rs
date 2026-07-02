//! Shared scene builders for editor chrome: panels, rows, labels, buttons.

use bevy::{
    feathers::{
        controls::{ButtonVariant, FeathersButton, FeathersListRow},
        theme::ThemedText,
    },
    prelude::*,
    ui_widgets::Activate,
};

/// Border color used by all panel frames.
pub const PANEL_BORDER: Color = Color::srgb(0.20, 0.22, 0.25);
/// Background color used by all panel frames.
pub const PANEL_BG: Color = Color::srgb(0.10, 0.11, 0.13);

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

/// A feathers list row with plain text content.
pub fn feathers_row(text: impl Into<String>) -> impl Scene {
    let text: String = text.into();
    bsn! {
        @FeathersListRow
        Children [
            (
                Text(text)
                ThemedText
                TextFont {
                    font_size: FontSize::Px(12.0),
                }
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
