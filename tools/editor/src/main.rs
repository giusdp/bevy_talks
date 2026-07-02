//! Editor UI for authoring `bevy_talks` dialogue assets.

mod graph;
mod panels;
mod state;
mod widgets;

use bevy::{
    asset::{
        AssetPath,
        saver::{SavedAsset, save_using_saver},
    },
    feathers::{
        FeathersPlugins,
        controls::FeathersButton,
        dark_theme::create_dark_theme,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
        tokens,
    },
    input_focus::tab_navigation::TabGroup,
    prelude::*,
    tasks::IoTaskPool,
    ui_widgets::Activate,
};
use bevy_talks::prelude::{DialogueDatabaseSaver, TalksPlugin};

use graph::CanvasBody;
use panels::{
    ActorsPanelBody, ConversationTitleText, ConversationsPanelBody, InspectorBody, StatusText,
    ValidationText,
};
use state::{DATABASE_PATH, EditorSelection, EditorState, PendingLoad};
use widgets::{
    PANEL_BORDER, feathers_row, header_text, muted_text, panel, panel_header,
    primary_toolbar_button, toolbar_button,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "bevy_talks editor".to_owned(),
                        resolution: (1280, 800).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    // The editor edits the workspace assets, not tools/editor/assets.
                    file_path: "../../assets".to_owned(),
                    ..default()
                }),
            FeathersPlugins,
            TalksPlugin,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        .init_resource::<EditorSelection>()
        .init_resource::<panels::SuppressInspectorRebuild>()
        .add_systems(Startup, (editor_scene.spawn(), state::start_database_load))
        .add_systems(
            Update,
            (
                state::finish_database_load.run_if(resource_exists::<PendingLoad>),
                (
                    panels::commit_entry_text_edits,
                    panels::rebuild_actors_panel,
                    panels::rebuild_conversations_panel,
                    panels::rebuild_inspector,
                    panels::update_conversation_title,
                    panels::update_status_text,
                    panels::update_validation_text,
                    graph::rebuild_canvas,
                    graph::apply_entry_selection,
                )
                    .chain()
                    .run_if(resource_exists::<EditorState>),
                graph::apply_graph_node_positions,
            ),
        )
        .run();
}

/// The whole editor UI plus camera.
fn editor_scene() -> impl SceneList {
    bsn_list![Camera2d, editor_root()]
}

/// The static editor frame; the marked bodies inside it are filled from data.
fn editor_root() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
        }
        TabGroup
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            toolbar(),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    column_gap: px(8),
                    padding: UiRect::axes(px(8), px(8)),
                }
                Children [
                    sidebar(),
                    conversation_board(),
                    inspector(),
                ]
            ),
            status_bar(),
        ]
    }
}

/// Top toolbar with the file name and (not yet wired) actions.
fn toolbar() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::axes(px(8), px(6)),
            border: {px(1).bottom()},
        }
        BorderColor::all(Color::srgb(0.19, 0.21, 0.24))
        Children [
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                }
                Children [
                    header_text("bevy_talks editor"),
                    muted_text(format!("assets/{DATABASE_PATH}")),
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(6),
                }
                Children [
                    toolbar_button("Open"),
                    save_button(),
                    primary_toolbar_button("Validate"),
                ]
            )
        ]
    }
}

/// Toolbar Save: writes the working copy to disk through the `AssetSaver` path.
fn save_button() -> impl Scene {
    bsn! {
        @FeathersButton {
            @caption: bsn! { Text("Save") ThemedText },
        }
        on(|_: On<Activate>, state: Option<Res<EditorState>>, assets: Res<AssetServer>| {
            let Some(state) = state else {
                warn!("nothing to save yet");
                return;
            };
            let db = state.db.clone();
            let server = assets.clone();
            IoTaskPool::get()
                .spawn(async move {
                    let path = AssetPath::from(DATABASE_PATH);
                    let saved = SavedAsset::from_asset(&db);
                    match save_using_saver(server, &DialogueDatabaseSaver, &path, saved, &()).await
                    {
                        Ok(()) => info!("saved {path}"),
                        Err(err) => error!("save failed: {err}"),
                    }
                })
                .detach();
        })
    }
}

/// Left sidebar: file, actors, and conversations.
fn sidebar() -> impl Scene {
    bsn! {
        Node {
            width: px(260),
            min_width: px(260),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
        }
        Children [
            panel("Files", bsn_list![
                feathers_row(DATABASE_PATH),
            ]),
            panel("Actors", bsn_list![
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(2),
                    }
                    ActorsPanelBody
                    Children [ muted_text("loading…") ]
                ),
            ]),
            panel("Conversations", bsn_list![
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(2),
                    }
                    ConversationsPanelBody
                    Children [ muted_text("loading…") ]
                ),
            ]),
        ]
    }
}

/// Center column: conversation heading, canvas tools, and the graph canvas.
fn conversation_board() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            min_width: px(360),
            row_gap: px(8),
        }
        Children [
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                }
                Children [
                    (
                        Text("Loading database…")
                        ThemedText
                        TextFont {
                            font_size: FontSize::Px(15.0),
                        }
                        ConversationTitleText
                    ),
                    (
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            column_gap: px(6),
                        }
                        Children [
                            toolbar_button("Frame"),
                            toolbar_button("Group"),
                            primary_toolbar_button("Add Node"),
                        ]
                    )
                ]
            ),
            graph_canvas(),
        ]
    }
}

/// The graph canvas frame: grid, dynamic node/link layer, and hints.
fn graph_canvas() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_grow: 1.0,
            position_type: PositionType::Relative,
            overflow: Overflow::clip(),
            border: {px(1).all()},
            border_radius: BorderRadius::all(px(6)),
        }
        BorderColor::all(PANEL_BORDER)
        BackgroundColor(Color::srgb(0.075, 0.080, 0.092))
        Children [
            {grid_lines()},
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: percent(100),
                    height: percent(100),
                }
                CanvasBody
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(14),
                    bottom: px(12),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(8),
                    align_items: AlignItems::Center,
                }
                Children [
                    muted_text("Click node: inspect"),
                    muted_text("Drag node: move"),
                ]
            )
        ]
    }
}

/// Background grid lines for the canvas.
fn grid_lines() -> Vec<Box<dyn Scene>> {
    let mut lines: Vec<Box<dyn Scene>> = Vec::new();
    for i in 1..=22 {
        lines.push(Box::new(grid_line(i as f32 * 40.0, true)));
    }
    for i in 1..=12 {
        lines.push(Box::new(grid_line(i as f32 * 40.0, false)));
    }
    lines
}

/// One canvas grid line.
fn grid_line(offset: f32, vertical: bool) -> impl Scene {
    let (left, top) = if vertical {
        (offset, 0.0)
    } else {
        (0.0, offset)
    };
    let (width, height) = if vertical {
        (px(1), percent(100))
    } else {
        (percent(100), px(1))
    };
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(left),
            top: px(top),
            width: width,
            height: height,
        }
        BackgroundColor(Color::srgba(0.22, 0.24, 0.28, 0.22))
    }
}

/// Right column: the entry inspector.
fn inspector() -> impl Scene {
    bsn! {
        Node {
            width: px(340),
            min_width: px(340),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
        }
        Children [
            panel_header("Inspector"),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    padding: {px(10).all()},
                    flex_grow: 1.0,
                    border: {px(1).all()},
                }
                BorderColor::all(PANEL_BORDER)
                BackgroundColor(Color::srgb(0.11, 0.12, 0.14))
                InspectorBody
                Children [ muted_text("loading…") ]
            )
        ]
    }
}

/// Bottom status bar: database summary and validation result.
fn status_bar() -> impl Scene {
    bsn! {
        Node {
            height: px(44),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::axes(px(10), px(6)),
            border: {px(1).top()},
        }
        BorderColor::all(Color::srgb(0.19, 0.21, 0.24))
        Children [
            (
                Text("Loading database…")
                ThemedText
                TextFont {
                    font_size: FontSize::Px(12.0),
                }
                TextColor(Color::srgb(0.62, 0.66, 0.72))
                StatusText
            ),
            (
                Text("")
                ThemedText
                TextFont {
                    font_size: FontSize::Px(12.0),
                }
                TextColor(Color::srgb(0.48, 0.80, 0.52))
                ValidationText
            )
        ]
    }
}
