//! The exploration map (`M` to open/close). Shows every room the party
//! has visited, plus the still-unexplored doorways leading out of them,
//! per the confirmed map design (Section 6 of the context doc: reveal as
//! you explore, never the whole floor at once).
//!
//! # How this stays reusable for future maps
//!
//! This file knows nothing about any *specific* room. It only reads three
//! generic, room-count-independent things:
//!
//! - `Room` + `RoomInfo` + `RoomBounds` (`rooms::components`) -- what
//!   rooms exist, what kind each one is, and roughly where it sits.
//! - `RoomVisited` / `CurrentRoom` (`rooms::exploration`) -- which rooms
//!   the party has actually entered, and which one they're in right now.
//! - `RoomGraph` (`rooms::graph`) -- which rooms connect to which.
//!
//! A brand new room, or an entire procedurally-generated floor full of
//! them, shows up on the map correctly as soon as it's spawned with those
//! components and one `RoomGraph::connect` call -- nothing in this file
//! needs to change. The on-screen layout is a topological grid (each
//! connection becomes one step north/south/east/west, chosen from the
//! real world-space direction between the two rooms), not a literal
//! floorplan -- the same style as the context doc's own ASCII map
//! mock-ups. That sidesteps needing to draw rotated/diagonal connector
//! lines entirely, and keeps the map readable regardless of how oddly
//! individual rooms are rotated in world space.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::rooms::components::{Room, RoomBounds, RoomInfo, RoomKind, RoomVisited};
use crate::rooms::exploration::CurrentRoom;
use crate::rooms::graph::RoomGraph;
use crate::weapons::InventoryUiState;

use super::ruin_screen::RuinScreenState;

// =========================================================
// LAYOUT CONSTANTS
// =========================================================

const MAP_CANVAS_WIDTH: f32 = 820.0;
const MAP_CANVAS_HEIGHT: f32 = 460.0;
const MAP_NODE_SIZE: f32 = 78.0;
/// Cell spacing never grows past this even when there are very few rooms
/// to lay out, so a one- or two-room map doesn't stretch its nodes to the
/// edges of the panel.
const MAP_MAX_CELL_SIZE: f32 = 150.0;
const MAP_CONNECTOR_THICKNESS: f32 = 5.0;

// =========================================================
// RESOURCE + COMPONENTS
// =========================================================

#[derive(Resource, Default)]
pub struct MapUiState {
    pub open: bool,
}

#[derive(Component)]
pub(crate) struct MapScreen;

/// The fixed-size drawing surface that room nodes and connectors are
/// spawned into (and despawned from) every time the map is rebuilt.
#[derive(Component)]
pub(crate) struct MapCanvas;

/// Marks a child of `MapCanvas` produced by the last rebuild, so the next
/// rebuild knows what to clear before laying the map out again.
#[derive(Component)]
pub(crate) struct MapNode;

// =========================================================
// PLUGIN WIRING (see `ui::UiPlugin`)
// =========================================================

pub(crate) fn spawn_map_ui(mut commands: Commands) {
    commands
        .spawn((
            MapScreen,
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                top: px(0.0),
                width: percent(100.0),
                height: percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.62)),
            ZIndex(60),
        ))
        .with_children(|screen| {
            screen
                .spawn((
                    Node {
                        width: px(MAP_CANVAS_WIDTH + 48.0),
                        padding: UiRect::all(px(24.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10.0),
                        border: UiRect::all(px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.055, 0.06, 0.075)),
                    BorderColor::all(Color::srgb(0.38, 0.32, 0.20)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("EXPLORATION MAP"),
                        TextFont {
                            font_size: FontSize::Px(34.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.82, 0.52)),
                    ));

                    panel.spawn((
                        Text::new(
                            "Rooms fill in as the party explores. Press M to close.",
                        ),
                        TextFont {
                            font_size: FontSize::Px(17.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.72, 0.72, 0.72)),
                    ));

                    // Drawing surface -- `update_map_display` fills this
                    // with `MapNode`-tagged children every frame the map
                    // is open, and clears them again first.
                    panel.spawn((
                        MapCanvas,
                        Node {
                            position_type: PositionType::Relative,
                            width: px(MAP_CANVAS_WIDTH),
                            height: px(MAP_CANVAS_HEIGHT),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.02, 0.025, 0.03, 0.6)),
                    ));

                    panel.spawn((
                        Text::new(
                            "BOSS - MINIBOSS - ELITE - TREASURE - SHRINE - EVENT -- unlabeled rooms are ordinary combat/exploration. \"?\" marks a known but unexplored doorway.",
                        ),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.55, 0.58, 0.62)),
                    ));
                });
        });
}

/// `M` opens/closes the map, exactly like `I` does for the inventory
/// (`weapons::inventory_ui::toggle_inventory`) -- release/restore the
/// cursor the same way. Doesn't need to know anything about the
/// inventory itself: opening the map here is enough to make
/// `weapons::inventory_ui::close_inventory_when_map_opens` close it on
/// its own (mirrored by `close_map_when_inventory_opens` below), and by
/// the time the map closes again the inventory is therefore already
/// closed, so it's safe to unconditionally lock the cursor back up.
/// A run ended (possibly with the map open): hide it for the next run.
pub(crate) fn hide_map_screen(mut screen: Single<&mut Node, With<MapScreen>>) {
    screen.display = Display::None;
}

pub(crate) fn toggle_map(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MapUiState>,
    mut cursor_options: Single<&mut CursorOptions>,
    mut screen: Single<&mut Node, With<MapScreen>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyM) {
        return;
    }

    state.open = !state.open;

    if state.open {
        screen.display = Display::Flex;
        cursor_options.grab_mode = CursorGrabMode::None;
        cursor_options.visible = true;
    } else {
        screen.display = Display::None;
        cursor_options.grab_mode = CursorGrabMode::Locked;
        cursor_options.visible = false;
    }
}

/// Closes the map the instant the inventory opens, so the two full-screen
/// overlays never show at once -- mirrors
/// `weapons::inventory_ui::close_inventory_when_map_opens`, which does the
/// reverse. Reacting to `InventoryUiState` changing (rather than the
/// inventory's own toggle calling into this module directly) keeps each
/// screen's closing logic -- including whatever else it needs to reset,
/// like the inventory's own drag state -- owned by that screen's own
/// module.
pub(crate) fn close_map_when_inventory_opens(
    inventory_state: Res<InventoryUiState>,
    mut state: ResMut<MapUiState>,
    mut screen: Single<&mut Node, With<MapScreen>>,
) {
    if !inventory_state.is_changed() || !inventory_state.open || !state.open {
        return;
    }

    state.open = false;
    screen.display = Display::None;
}

/// Closes the map the instant the Ruin screen opens, so no two
/// full-screen overlays ever show at once -- mirrors
/// `close_map_when_inventory_opens` above and
/// `ui::ruin_screen::close_ruin_screen_when_other_ui_opens`, which closes
/// the Ruin screen in return.
pub(crate) fn close_map_when_ruin_opens(
    ruin_screen_state: Res<RuinScreenState>,
    mut state: ResMut<MapUiState>,
    mut screen: Single<&mut Node, With<MapScreen>>,
) {
    if !ruin_screen_state.is_changed() || !ruin_screen_state.open || !state.open {
        return;
    }

    state.open = false;
    screen.display = Display::None;
}

// =========================================================
// MAP REBUILD
// =========================================================

/// Rebuilds the map's room nodes and connector lines from scratch every
/// frame the map is open. Room counts here stay small (a handful of
/// rooms even late into a run's current floor), so redrawing the whole
/// thing is far simpler than diffing it and costs nothing noticeable for
/// a screen that isn't open during combat.
pub(crate) fn update_map_display(
    map_state: Res<MapUiState>,
    current_room: Res<CurrentRoom>,
    room_graph: Res<RoomGraph>,
    rooms: Query<
        (
            Entity,
            &RoomInfo,
            &Transform,
            &RoomBounds,
            Option<&RoomVisited>,
        ),
        With<Room>,
    >,
    canvas: Query<Entity, With<MapCanvas>>,
    existing_nodes: Query<Entity, With<MapNode>>,
    mut commands: Commands,
) {
    if !map_state.open {
        return;
    }

    let Ok(canvas_entity) = canvas.single() else {
        return;
    };

    for node in &existing_nodes {
        commands.entity(node).despawn();
    }

    // World-space (X, Z) center and display info for every room that
    // exists, visited or not -- needed up front so discovered-but-
    // unvisited neighbors and the grid-layout direction math both have
    // something to read.
    let mut room_data: HashMap<Entity, (RoomInfo, Vec2)> = HashMap::new();
    let mut visited: Vec<Entity> = Vec::new();

    for (entity, info, transform, bounds, has_visited) in &rooms {
        let world_center = bounds.world_center(transform);
        room_data.insert(entity, (*info, Vec2::new(world_center.x, world_center.z)));

        if has_visited.is_some() {
            visited.push(entity);
        }
    }

    if visited.is_empty() {
        // Nothing entered yet (shouldn't last past the first frame of
        // play, since the player's spawn point is inside a `RoomBounds`
        // by construction) -- show a placeholder rather than an empty
        // panel.
        commands.entity(canvas_entity).with_children(|parent| {
            parent.spawn((
                MapNode,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    top: px(MAP_CANVAS_HEIGHT / 2.0 - 12.0),
                    width: px(MAP_CANVAS_WIDTH),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Text::new("The map fills in as you explore."),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.58, 0.62)),
                TextLayout::justify(Justify::Center),
            ));
        });
        return;
    }

    visited.sort();
    let visited_set: HashSet<Entity> = visited.iter().copied().collect();

    // Known doorways out of a visited room that haven't been walked
    // through yet -- shown as "?", per the confirmed map design.
    let mut discovered: HashSet<Entity> = HashSet::new();
    for &room in &visited {
        for neighbor in room_graph.neighbors(room) {
            if !visited_set.contains(&neighbor) {
                discovered.insert(neighbor);
            }
        }
    }
    let mut discovered: Vec<Entity> = discovered.into_iter().collect();
    discovered.sort();

    let mut visible: Vec<Entity> = visited.clone();
    visible.extend(discovered.iter().copied());
    let visible_set: HashSet<Entity> = visible.iter().copied().collect();

    // Lay out the visible rooms on an abstract N/S/E/W grid (see the
    // module doc comment for why this beats a to-scale floorplan). Prefer
    // the Start room as the root so the map tends to read the same way
    // run to run; fall back to the lowest-Entity visited room (sorted
    // above) so the layout is at least deterministic when there isn't one
    // (e.g. the Saint arena or Test Grounds used stand-alone).
    let positions: HashMap<Entity, Vec2> = room_data
        .iter()
        .map(|(&entity, &(_, pos))| (entity, pos))
        .collect();

    let root = visible
        .iter()
        .copied()
        .find(|entity| {
            room_data
                .get(entity)
                .is_some_and(|(info, _)| info.kind == RoomKind::Start)
        })
        .unwrap_or(visited[0]);

    let grid = build_room_layout(&visible, &positions, &room_graph, root);

    let min_col = grid.values().map(|pos| pos.x).min().unwrap_or(0);
    let max_col = grid.values().map(|pos| pos.x).max().unwrap_or(0);
    let min_row = grid.values().map(|pos| pos.y).min().unwrap_or(0);
    let max_row = grid.values().map(|pos| pos.y).max().unwrap_or(0);

    let grid_width = (max_col - min_col) as f32;
    let grid_height = (max_row - min_row) as f32;

    // Scale the grid to always fit the fixed-size canvas, whether it's
    // two rooms or several dozen.
    let available_width = MAP_CANVAS_WIDTH - MAP_NODE_SIZE;
    let available_height = MAP_CANVAS_HEIGHT - MAP_NODE_SIZE;
    let cell_size = {
        let by_width = if grid_width > 0.0 {
            available_width / grid_width
        } else {
            f32::INFINITY
        };
        let by_height = if grid_height > 0.0 {
            available_height / grid_height
        } else {
            f32::INFINITY
        };
        by_width.min(by_height).min(MAP_MAX_CELL_SIZE)
    };

    let center_col = (min_col + max_col) as f32 / 2.0;
    let center_row = (min_row + max_row) as f32 / 2.0;

    // +row was chosen (in `compass_offset` below) to mean "further along
    // +Z world space", so it's flipped here to appear higher on screen --
    // matching the intuitive "deeper into the dungeon = further up the
    // map" reading of the context doc's own ASCII mock-ups.
    let node_center = |pos: IVec2| -> Vec2 {
        Vec2::new(
            MAP_CANVAS_WIDTH / 2.0 + (pos.x as f32 - center_col) * cell_size,
            MAP_CANVAS_HEIGHT / 2.0 - (pos.y as f32 - center_row) * cell_size,
        )
    };

    commands.entity(canvas_entity).with_children(|parent| {
        // Connectors first so room nodes render on top of their ends.
        for (a, b) in room_graph.edges() {
            if !visible_set.contains(&a) || !visible_set.contains(&b) {
                continue;
            }

            let (Some(&pos_a), Some(&pos_b)) = (grid.get(&a), grid.get(&b)) else {
                continue;
            };

            let delta = pos_b - pos_a;
            if delta.x.abs() + delta.y.abs() != 1 {
                // Not exactly one grid step apart -- a loop the layout
                // couldn't place cleanly next to its partner. Both rooms
                // still render as nodes below; only the connecting line
                // between them is skipped.
                continue;
            }

            let center_a = node_center(pos_a);
            let center_b = node_center(pos_b);
            let both_visited = visited_set.contains(&a) && visited_set.contains(&b);

            let (left, top, width, height) = if delta.y == 0 {
                let left = center_a.x.min(center_b.x) + MAP_NODE_SIZE / 2.0;
                let width = (center_a.x - center_b.x).abs() - MAP_NODE_SIZE;
                (
                    left,
                    center_a.y - MAP_CONNECTOR_THICKNESS / 2.0,
                    width.max(0.0),
                    MAP_CONNECTOR_THICKNESS,
                )
            } else {
                let top = center_a.y.min(center_b.y) + MAP_NODE_SIZE / 2.0;
                let height = (center_a.y - center_b.y).abs() - MAP_NODE_SIZE;
                (
                    center_a.x - MAP_CONNECTOR_THICKNESS / 2.0,
                    top,
                    MAP_CONNECTOR_THICKNESS,
                    height.max(0.0),
                )
            };

            parent.spawn((
                MapNode,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(left),
                    top: px(top),
                    width: px(width),
                    height: px(height),
                    ..default()
                },
                BackgroundColor(if both_visited {
                    Color::srgba(0.45, 0.55, 0.62, 0.9)
                } else {
                    Color::srgba(0.32, 0.32, 0.35, 0.6)
                }),
            ));
        }

        // Room nodes.
        for &room in &visible {
            let (Some(&pos), Some(&(info, _))) = (grid.get(&room), room_data.get(&room)) else {
                continue;
            };

            let center = node_center(pos);
            let is_visited = visited_set.contains(&room);
            let is_current = current_room.0 == Some(room);

            let (background, border, top_text, name_text) = if !is_visited {
                (
                    Color::srgba(0.05, 0.05, 0.06, 0.85),
                    Color::srgb(0.32, 0.32, 0.35),
                    "?",
                    None,
                )
            } else if is_current {
                (
                    Color::srgba(0.14, 0.11, 0.04, 0.96),
                    Color::srgb(0.95, 0.82, 0.52),
                    "YOU",
                    Some(info.display_name),
                )
            } else {
                (
                    Color::srgba(0.035, 0.10, 0.15, 0.94),
                    kind_accent_color(info.kind),
                    kind_label(info.kind),
                    Some(info.display_name),
                )
            };

            parent
                .spawn((
                    MapNode,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(center.x - MAP_NODE_SIZE / 2.0),
                        top: px(center.y - MAP_NODE_SIZE / 2.0),
                        width: px(MAP_NODE_SIZE),
                        height: px(MAP_NODE_SIZE),
                        border: UiRect::all(px(2.5)),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(px(4.0)),
                        ..default()
                    },
                    BackgroundColor(background),
                    BorderColor::all(border),
                ))
                .with_children(|node| {
                    if !top_text.is_empty() {
                        node.spawn((
                            Text::new(top_text),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(if is_visited {
                                Color::srgb(0.92, 0.95, 1.0)
                            } else {
                                Color::srgb(0.55, 0.55, 0.58)
                            }),
                            TextLayout::justify(Justify::Center),
                        ));
                    }

                    if let Some(name) = name_text {
                        node.spawn((
                            Text::new(name),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(Color::srgba(0.75, 0.8, 0.85, 0.85)),
                            TextLayout::justify(Justify::Center),
                        ));
                    }
                });
        }
    });
}

// =========================================================
// TOPOLOGICAL GRID LAYOUT
// =========================================================

/// Assigns every room in `visible` an integer (column, row) grid
/// position, starting from `root` at the origin and walking `graph`'s
/// connections breadth-first -- so the resulting layout only ever
/// includes rooms actually reachable (through other visible rooms) from
/// `root`. Each step's direction is picked, via `compass_offset`, from
/// the real world-space direction between the two rooms' `RoomBounds`
/// centers, so the map's overall shape roughly tracks the dungeon's real
/// layout without needing to reproduce it to scale.
fn build_room_layout(
    visible: &[Entity],
    positions: &HashMap<Entity, Vec2>,
    graph: &RoomGraph,
    root: Entity,
) -> HashMap<Entity, IVec2> {
    let visible_set: HashSet<Entity> = visible.iter().copied().collect();
    let mut grid: HashMap<Entity, IVec2> = HashMap::new();
    grid.insert(root, IVec2::ZERO);

    let mut queue: VecDeque<Entity> = VecDeque::new();
    queue.push_back(root);

    while let Some(current) = queue.pop_front() {
        let current_grid = grid[&current];
        let Some(&current_world) = positions.get(&current) else {
            continue;
        };

        let mut neighbors: Vec<Entity> = graph
            .neighbors(current)
            .filter(|neighbor| visible_set.contains(neighbor) && !grid.contains_key(neighbor))
            .collect();
        // Deterministic visit order so the same explored set always lays
        // out the same way.
        neighbors.sort();

        for neighbor in neighbors {
            let Some(&neighbor_world) = positions.get(&neighbor) else {
                continue;
            };

            let offset = compass_offset(neighbor_world - current_world);

            // If that cell is already taken (only possible once loops or
            // separate branches happen to point the same direction), keep
            // stepping further the same way until a free cell turns up
            // rather than stacking two rooms on top of each other.
            let mut candidate = current_grid + offset;
            while grid.values().any(|&pos| pos == candidate) {
                candidate += offset;
            }

            grid.insert(neighbor, candidate);
            queue.push_back(neighbor);
        }
    }

    // Defensive fallback: a visible room the graph doesn't actually
    // connect back to `root` shouldn't normally happen (the party can
    // only visit a room by walking a real connection to it), but if it
    // ever does, place it in its own row below everything else instead of
    // silently dropping it from the map.
    if grid.len() < visible.len() {
        let mut fallback_row = grid.values().map(|pos| pos.y).min().unwrap_or(0) - 2;
        let mut fallback_col = grid.values().map(|pos| pos.x).min().unwrap_or(0);

        for &room in visible {
            if grid.contains_key(&room) {
                continue;
            }

            while grid
                .values()
                .any(|&pos| pos == IVec2::new(fallback_col, fallback_row))
            {
                fallback_col += 1;
            }

            grid.insert(room, IVec2::new(fallback_col, fallback_row));
            fallback_col += 1;
        }

        let _ = fallback_row;
    }

    grid
}

/// Buckets a world-space (X, Z) direction into one grid step: whichever
/// axis has the larger magnitude wins, ties favor +X. Z is treated as the
/// "north/south" axis and X as "east/west" -- an arbitrary but consistent
/// choice, since Ruinbound rooms are placed on a flat X/Z plane with no
/// preferred compass direction of their own.
fn compass_offset(delta: Vec2) -> IVec2 {
    if delta.x.abs() >= delta.y.abs() {
        IVec2::new(if delta.x >= 0.0 { 1 } else { -1 }, 0)
    } else {
        IVec2::new(0, if delta.y >= 0.0 { 1 } else { -1 })
    }
}

// =========================================================
// ROOM-KIND PRESENTATION
// =========================================================

/// The tag shown on a visited room's node. Empty for kinds the confirmed
/// map design (Section 6) says should render as a plain checkmark-style
/// room with no special callout.
fn kind_label(kind: RoomKind) -> &'static str {
    match kind {
        RoomKind::Start => "START",
        RoomKind::Combat | RoomKind::Unknown => "",
        RoomKind::Elite => "ELITE",
        RoomKind::Miniboss => "MINIBOSS",
        RoomKind::Boss => "BOSS",
        RoomKind::Treasure => "TREASURE",
        RoomKind::Shrine => "SHRINE",
        RoomKind::Event => "EVENT",
    }
}

/// Border accent per room kind, matching the HUD's existing blue accent
/// (`Color::srgb(0.25, 0.65, 0.85)`, see `ui::spawn_ability_bar`) for
/// anything without a more specific identity of its own.
fn kind_accent_color(kind: RoomKind) -> Color {
    match kind {
        RoomKind::Start | RoomKind::Combat | RoomKind::Unknown => Color::srgb(0.25, 0.65, 0.85),
        RoomKind::Elite => Color::srgb(0.85, 0.55, 0.20),
        RoomKind::Miniboss => Color::srgb(0.85, 0.40, 0.25),
        RoomKind::Boss => Color::srgb(0.85, 0.20, 0.20),
        RoomKind::Treasure => Color::srgb(0.85, 0.75, 0.25),
        RoomKind::Shrine => Color::srgb(0.62, 0.35, 0.85),
        RoomKind::Event => Color::srgb(0.35, 0.80, 0.65),
    }
}
