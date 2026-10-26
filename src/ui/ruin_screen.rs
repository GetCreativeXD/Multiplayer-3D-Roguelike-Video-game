//! The Ruin status screen (`Tab` to open/close). Shows the player's
//! current Ruin and how long the run clock has been running -- see
//! `crate::ruin::RuinState`, which owns both numbers.
//!
//! # How this stays reusable for future additions
//!
//! The design doc (`RUIN_CORRUPTION_DESIGN.md`) describes a lot more
//! that will eventually want a home on this screen: local room
//! corruption, active corruption sources, Ruinbreak history, and so on.
//! Rather than hand-laying-out two fixed lines, the panel is built from
//! a small, growing list of stat rows (`RuinStatKind` +
//! `spawn_stat_row`). Adding a new stat later is:
//!
//! 1. Add a variant to `RuinStatKind`.
//! 2. Add one `spawn_stat_row(...)` call in `spawn_ruin_screen`.
//! 3. Add one match arm in `update_ruin_screen_display`.
//!
//! Nothing about the panel itself, its open/close wiring, or its
//! exclusivity with the map and inventory screens needs to change.

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::ruin::RuinState;
use crate::ui::MapUiState;
use crate::weapons::InventoryUiState;

// =========================================================
// RESOURCE + COMPONENTS
// =========================================================

#[derive(Resource, Default)]
pub struct RuinScreenState {
    pub open: bool,
}

#[derive(Component)]
pub(crate) struct RuinScreen;

/// Which `RuinState` value a stat row's value text displays --
/// `update_ruin_screen_display` matches on this instead of one marker
/// component per row, so a new row is one enum variant away rather than
/// a whole new component + query.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuinStatKind {
    Ruin,
    Threshold,
    RunTime,
}

// =========================================================
// PLUGIN WIRING (see `ui::UiPlugin`)
// =========================================================

pub(crate) fn spawn_ruin_screen(mut commands: Commands) {
    commands
        .spawn((
            RuinScreen,
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
            ZIndex(70),
        ))
        .with_children(|screen| {
            screen
                .spawn((
                    Node {
                        width: px(520.0),
                        padding: UiRect::all(px(24.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(14.0),
                        border: UiRect::all(px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.055, 0.06, 0.075)),
                    BorderColor::all(Color::srgb(0.38, 0.32, 0.20)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("RUIN"),
                        TextFont {
                            font_size: FontSize::Px(34.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.82, 0.52)),
                    ));

                    panel.spawn((
                        Text::new("The dungeon's corruption, and how long you've endured it. Press Tab to close."),
                        TextFont {
                            font_size: FontSize::Px(17.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.72, 0.72, 0.72)),
                    ));

                    spawn_stat_row(panel, "RUIN", RuinStatKind::Ruin);
                    spawn_stat_row(panel, "STATE", RuinStatKind::Threshold);
                    spawn_stat_row(panel, "RUN TIME", RuinStatKind::RunTime);

                    panel.spawn((
                        Text::new(
                            "More will appear here as the dungeon's corruption deepens.",
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

/// Spawns one `LABEL ........ value` row and tags its value text with
/// `kind` so `update_ruin_screen_display` can fill it in every frame the
/// screen is open.
fn spawn_stat_row(parent: &mut ChildSpawnerCommands, label: &str, kind: RuinStatKind) {
    parent
        .spawn((
            Node {
                width: percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::vertical(px(6.0)),
                border: UiRect::bottom(px(1.0)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.3, 0.3, 0.32, 0.4)),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(19.0),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.8, 0.85)),
            ));

            row.spawn((
                kind,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(19.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// `Tab` opens/closes the Ruin screen, exactly like `M` does for the map
/// and `I` does for the inventory -- release/restore the cursor the same
/// way. Doesn't need to know anything about those other screens: opening
/// here is enough to make `ui::map::close_map_when_ruin_opens` and
/// `weapons::inventory_ui::close_inventory_when_ruin_opens` close them on
/// their own, mirrored by `close_ruin_screen_when_other_ui_opens` below.
/// A run ended (possibly with this screen open): hide it for the next run.
pub(crate) fn hide_ruin_screen(mut screen: Single<&mut Node, With<RuinScreen>>) {
    screen.display = Display::None;
}

pub(crate) fn toggle_ruin_screen(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<RuinScreenState>,
    mut cursor_options: Single<&mut CursorOptions>,
    mut screen: Single<&mut Node, With<RuinScreen>>,
) {
    if !keyboard.just_pressed(KeyCode::Tab) {
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

/// Closes the Ruin screen the instant the map or inventory opens, so no
/// two full-screen overlays ever show at once -- mirrors
/// `ui::map::close_map_when_inventory_opens` and its counterparts.
/// Doesn't touch the cursor: whichever screen just opened already
/// claimed it.
pub(crate) fn close_ruin_screen_when_other_ui_opens(
    map_state: Res<MapUiState>,
    inventory_state: Res<InventoryUiState>,
    mut state: ResMut<RuinScreenState>,
    mut screen: Single<&mut Node, With<RuinScreen>>,
) {
    if !state.open {
        return;
    }

    let other_screen_just_opened = (map_state.is_changed() && map_state.open)
        || (inventory_state.is_changed() && inventory_state.open);

    if !other_screen_just_opened {
        return;
    }

    state.open = false;
    screen.display = Display::None;
}

// =========================================================
// DISPLAY REFRESH
// =========================================================

/// Refreshes every stat row while the screen is open. Cheap enough (a
/// handful of rows, plain text formatting) to just redo every frame,
/// same reasoning as `ui::map::update_map_display`.
pub(crate) fn update_ruin_screen_display(
    state: Res<RuinScreenState>,
    ruin_state: Res<RuinState>,
    mut rows: Query<(&RuinStatKind, &mut Text, &mut TextColor)>,
) {
    if !state.open {
        return;
    }

    let threshold = ruin_state.threshold();

    for (kind, mut text, mut color) in &mut rows {
        match kind {
            RuinStatKind::Ruin => {
                **text = format!("{} / 100", ruin_state.ruin_rounded());
                color.0 = threshold.accent_color();
            }
            RuinStatKind::Threshold => {
                **text = threshold.label().to_string();
                color.0 = threshold.accent_color();
            }
            RuinStatKind::RunTime => {
                **text = format_run_time(ruin_state.run_elapsed_seconds());
                color.0 = Color::WHITE;
            }
        }
    }
}

/// `mm:ss`, e.g. `1305.0` seconds -> `"21:45"`. Run times are expected to
/// stay well under an hour, so minutes alone (rather than `h:mm:ss`) is
/// enough for now.
fn format_run_time(total_seconds: f32) -> String {
    let total_seconds = total_seconds.max(0.0) as u32;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}
