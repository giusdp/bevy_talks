//! The conversation graph canvas: layout, node rendering, links, and dragging.

use std::collections::HashMap;

use bevy::{feathers::theme::ThemedText, prelude::*};
use bevy_talks::prelude::*;

use crate::state::{EditorSelection, EditorState, number_field, set_number_field};
use crate::widgets::muted_text;

/// Width of a graph node.
const NODE_WIDTH: f32 = 210.0;
/// Minimum height of a graph node.
const NODE_MIN_HEIGHT: f32 = 92.0;
/// Horizontal gap between layout columns.
const COLUMN_GAP: f32 = 70.0;
/// Vertical gap between layout rows.
const ROW_GAP: f32 = 40.0;
/// Top-left corner of the auto-layout grid.
const CANVAS_ORIGIN: Vec2 = Vec2::new(24.0, 42.0);
/// Vertical offset of link anchors from a node's top edge.
const LINK_ANCHOR_Y: f32 = 46.0;

/// Color of link segments.
const LINK_COLOR: Color = Color::srgb(0.62, 0.67, 0.74);
/// Border color of the selected node.
const SELECTED_BORDER: Color = Color::srgb(0.93, 0.94, 0.97);
/// Border color of root nodes.
const ROOT_BORDER: Color = Color::srgb(0.92, 0.56, 0.16);
/// Border color of group nodes.
const GROUP_BORDER: Color = Color::srgb(0.70, 0.22, 0.76);
/// Border color of regular nodes.
const LINE_BORDER: Color = Color::srgb(0.25, 0.39, 0.58);

/// Marker for the container that holds the rendered conversation graph.
#[derive(Component, Default, Clone)]
pub struct CanvasBody;

/// Canvas position of a draggable graph node.
#[derive(Component, Clone, Copy, Default)]
pub struct GraphNodePosition {
    /// Left offset in canvas space.
    pub x: f32,
    /// Top offset in canvas space.
    pub y: f32,
}

/// Which dialogue entry a graph node represents, and its unselected border color.
#[derive(Component, Clone)]
pub struct GraphEntryNode {
    /// The represented entry.
    pub entry: EntryId,
    /// Border color when not selected.
    pub base_border: Color,
}

impl Default for GraphEntryNode {
    fn default() -> Self {
        Self {
            entry: EntryId(0),
            base_border: LINE_BORDER,
        }
    }
}

/// Marker for the node currently being dragged.
#[derive(Component)]
struct DraggingGraphNode;

/// Rebuilds the canvas when the database or the selected conversation changes.
pub fn rebuild_canvas(
    mut commands: Commands,
    state: Res<EditorState>,
    selection: Res<EditorSelection>,
    body: Single<Entity, With<CanvasBody>>,
    mut shown: Local<Option<ConversationId>>,
) {
    if !state.is_changed() && *shown == selection.conversation {
        return;
    }
    *shown = selection.conversation;

    commands.entity(*body).despawn_related::<Children>();
    let Some(conversation) = state.conversation(selection.conversation) else {
        return;
    };

    let positions = layout(conversation);
    let mut scenes: Vec<Box<dyn Scene>> = Vec::new();

    for entry in &conversation.entries {
        let from = positions[&entry.id] + Vec2::new(NODE_WIDTH, LINK_ANCHOR_Y);
        for link in &entry.links {
            if link.dest_conversation != conversation.id {
                continue;
            }
            let Some(dest) = positions.get(&link.dest_entry) else {
                continue;
            };
            push_link_segments(&mut scenes, from, *dest + Vec2::new(0.0, LINK_ANCHOR_Y));
        }
    }
    for entry in &conversation.entries {
        let selected = selection.entry == Some(entry.id);
        scenes.push(Box::new(entry_node(
            state.as_ref(),
            entry,
            positions[&entry.id],
            selected,
        )));
    }

    commands
        .entity(*body)
        .queue_spawn_related_scenes::<Children>(scenes);
}

/// Updates node border highlights when the selected entry changes.
pub fn apply_entry_selection(
    selection: Res<EditorSelection>,
    mut nodes: Query<(&GraphEntryNode, &mut BorderColor)>,
) {
    if !selection.is_changed() {
        return;
    }
    for (node, mut border) in &mut nodes {
        let color = if selection.entry == Some(node.entry) {
            SELECTED_BORDER
        } else {
            node.base_border
        };
        *border = BorderColor::all(color);
    }
}

/// Positions every entry: explicit `canvas_x`/`canvas_y` fields win, the rest
/// get a depth-based grid from a breadth-first walk of the links.
fn layout(conversation: &Conversation) -> HashMap<EntryId, Vec2> {
    let mut positions = HashMap::new();
    for entry in &conversation.entries {
        if let (Some(x), Some(y)) = (
            number_field(entry, "canvas_x"),
            number_field(entry, "canvas_y"),
        ) {
            positions.insert(entry.id, Vec2::new(x, y));
        }
    }

    let by_id: HashMap<EntryId, &DialogueEntry> =
        conversation.entries.iter().map(|e| (e.id, e)).collect();
    let mut depths: HashMap<EntryId, usize> = HashMap::new();
    let mut queue: std::collections::VecDeque<EntryId> = conversation
        .entries
        .iter()
        .filter(|e| e.is_root)
        .map(|e| e.id)
        .collect();
    for id in &queue {
        depths.insert(*id, 0);
    }
    while let Some(id) = queue.pop_front() {
        let depth = depths[&id];
        let Some(entry) = by_id.get(&id) else {
            continue;
        };
        for link in &entry.links {
            if link.dest_conversation == conversation.id
                && by_id.contains_key(&link.dest_entry)
                && !depths.contains_key(&link.dest_entry)
            {
                depths.insert(link.dest_entry, depth + 1);
                queue.push_back(link.dest_entry);
            }
        }
    }

    let mut rows_per_depth: HashMap<usize, usize> = HashMap::new();
    for entry in &conversation.entries {
        if positions.contains_key(&entry.id) {
            continue;
        }
        let depth = depths.get(&entry.id).copied().unwrap_or(0);
        let row = rows_per_depth.entry(depth).or_default();
        positions.insert(
            entry.id,
            CANVAS_ORIGIN
                + Vec2::new(
                    depth as f32 * (NODE_WIDTH + COLUMN_GAP),
                    *row as f32 * (NODE_MIN_HEIGHT + ROW_GAP),
                ),
        );
        *row += 1;
    }
    positions
}

/// Appends the three elbow segments of one link.
fn push_link_segments(scenes: &mut Vec<Box<dyn Scene>>, from: Vec2, to: Vec2) {
    let mid_x = (from.x + to.x) / 2.0;
    scenes.push(Box::new(horizontal_segment(
        from.x.min(mid_x),
        from.y,
        (mid_x - from.x).abs(),
    )));
    scenes.push(Box::new(vertical_segment(
        mid_x,
        from.y.min(to.y),
        (to.y - from.y).abs(),
    )));
    scenes.push(Box::new(horizontal_segment(
        mid_x.min(to.x),
        to.y,
        (to.x - mid_x).abs(),
    )));
}

/// A horizontal link segment.
fn horizontal_segment(left: f32, top: f32, width: f32) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(left),
            top: px(top),
            width: px(width),
            height: px(2),
        }
        BackgroundColor(LINK_COLOR)
    }
}

/// A vertical link segment.
fn vertical_segment(left: f32, top: f32, height: f32) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(left),
            top: px(top),
            width: px(2),
            height: px(height),
        }
        BackgroundColor(LINK_COLOR)
    }
}

/// A graph node rendering one dialogue entry.
fn entry_node(
    state: &EditorState,
    entry: &DialogueEntry,
    position: Vec2,
    selected: bool,
) -> impl Scene {
    let id = entry.id;
    let title = if entry.is_root {
        "START".to_owned()
    } else if !entry.menu_text.is_empty() {
        truncate(&entry.menu_text, 22)
    } else {
        truncate(&entry.dialogue_text, 22)
    };
    let kind = if entry.is_root {
        "Root"
    } else if entry.is_group {
        "Group"
    } else if !entry.menu_text.is_empty() {
        "Choice"
    } else {
        "Line"
    };
    let base_border = if entry.is_root {
        ROOT_BORDER
    } else if entry.is_group {
        GROUP_BORDER
    } else {
        LINE_BORDER
    };
    let border = if selected {
        SELECTED_BORDER
    } else {
        base_border
    };
    let header_color = if entry.is_root {
        Color::srgb(0.58, 0.30, 0.08)
    } else {
        Color::srgb(0.13, 0.22, 0.34)
    };
    let subtitle = format!("Entry {} · {}", id.0, state.actor_name(entry.actor));
    let line = truncate(&entry.dialogue_text, 60);
    let (x, y) = (position.x, position.y);

    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(x),
            top: px(y),
            width: px(NODE_WIDTH),
            min_height: px(NODE_MIN_HEIGHT),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            border: {px(1).all()},
            border_radius: BorderRadius::all(px(6)),
        }
        BorderColor::all(border)
        BackgroundColor(Color::srgb(0.12, 0.13, 0.15))
        GraphNodePosition { x: x, y: y }
        GraphEntryNode { entry: id, base_border: base_border }
        GlobalZIndex(1)
        on(move |mut click: On<Pointer<Click>>, mut selection: ResMut<EditorSelection>| {
            selection.entry = Some(id);
            click.propagate(false);
        })
        on(start_graph_node_drag)
        on(drag_graph_node)
        on(finish_graph_node_drag)
        Children [
            (
                Node {
                    height: px(30),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::horizontal(px(8)),
                    border_radius: BorderRadius {
                        top_left: px(5),
                        top_right: px(5),
                        bottom_left: px(0),
                        bottom_right: px(0),
                    },
                }
                BackgroundColor(header_color)
                Children [
                    (
                        Text(title)
                        ThemedText
                        TextFont {
                            font_size: FontSize::Px(12.0),
                        }
                    ),
                    muted_text(kind),
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    padding: {px(8).all()},
                }
                Children [
                    muted_text(subtitle),
                    (
                        Text(line)
                        ThemedText
                        TextFont {
                            font_size: FontSize::Px(13.0),
                        }
                    )
                ]
            )
        ]
    }
}

/// Shortens text for node labels, appending an ellipsis when truncated.
fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_owned()
    } else {
        let mut short: String = text.chars().take(max_chars).collect();
        short.push('…');
        short
    }
}

/// Raises the dragged node and marks it as dragging.
fn start_graph_node_drag(
    drag: On<Pointer<DragStart>>,
    mut commands: Commands,
    mut nodes: Query<&mut GlobalZIndex, With<GraphNodePosition>>,
) {
    if let Ok(mut z_index) = nodes.get_mut(drag.event_target()) {
        z_index.0 = 10;
        commands
            .entity(drag.event_target())
            .insert(DraggingGraphNode);
    }
}

/// Moves the dragged node with the pointer.
fn drag_graph_node(mut drag: On<Pointer<Drag>>, mut nodes: Query<&mut GraphNodePosition>) {
    if let Ok(mut position) = nodes.get_mut(drag.event_target()) {
        position.x = (position.x + drag.delta.x).max(0.0);
        position.y = (position.y + drag.delta.y).max(0.0);
        drag.propagate(false);
    }
}

/// Restores the dropped node's stacking order and persists its position
/// into the entry's `canvas_x`/`canvas_y` fields.
fn finish_graph_node_drag(
    drag: On<Pointer<DragEnd>>,
    mut commands: Commands,
    mut nodes: Query<(&GraphNodePosition, &GraphEntryNode, &mut GlobalZIndex)>,
    mut state: ResMut<EditorState>,
    selection: Res<EditorSelection>,
) {
    let Ok((position, node, mut z_index)) = nodes.get_mut(drag.event_target()) else {
        return;
    };
    z_index.0 = 1;
    commands
        .entity(drag.event_target())
        .remove::<DraggingGraphNode>();

    let db = &mut state.bypass_change_detection().db;
    let Some(entry) = selection
        .conversation
        .and_then(|id| db.conversations.iter_mut().find(|c| c.id == id))
        .and_then(|c| c.entries.iter_mut().find(|e| e.id == node.entry))
    else {
        return;
    };
    set_number_field(entry, "canvas_x", position.x);
    set_number_field(entry, "canvas_y", position.y);
    state.set_changed();
}

/// Syncs [`GraphNodePosition`] into UI node offsets.
pub fn apply_graph_node_positions(
    mut nodes: Query<(&GraphNodePosition, &mut Node), Changed<GraphNodePosition>>,
) {
    for (position, mut node) in &mut nodes {
        node.left = px(position.x);
        node.top = px(position.y);
    }
}
