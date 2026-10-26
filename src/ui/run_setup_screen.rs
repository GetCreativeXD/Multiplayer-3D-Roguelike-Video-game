//! The run-setup screen: the first thing the player sees, before
//! `AppState::InGame` exists. Lets the player choose their class,
//! starting weapon, item drop pool, any number of starting relics, any
//! number of run pacts, and starting Ruin condition, then locks those
//! choices into `RunConfig` (`run_setup::RunConfig`) and starts the run.
//!
//! # How this stays reusable for future choices
//!
//! Every option on every tab is built from a catalog entry in
//! `run_setup::catalog` (see that module's own doc comment) -- this file
//! never hard-codes a class/weapon/relic/pact/Ruin-condition's name or
//! description. Concretely, three pieces are shared across all six tabs
//! instead of being rebuilt per category:
//!
//! - `spawn_option_card` -- the clickable "card" widget itself (name,
//!   locked state, static tooltip text).
//! - `RunSetupCard` + `update_card_hover` -- one generic hover-recolor
//!   system for every card on the screen, regardless of category.
//! - `RunSetupTooltipContent` + `update_run_setup_description` -- one
//!   shared description strip fed by whichever card is hovered, falling
//!   back to the active tab's current selection when nothing is hovered.
//!
//! What isn't shared is each category's own click-handling + selection-
//! highlight system (`handle_class_options`, `handle_weapon_options`,
//! etc.) -- each reads a different `RunConfig` field and a differently-
//! shaped catalog entry, so, matching this project's existing style (see
//! `characters::mod`'s five near-identical `attach_*_visual` functions),
//! they stay as small, independent, easy-to-copy functions rather than
//! one clever generic system.
//!
//! # What's wired to gameplay vs. recorded for later
//!
//! Class, starting weapon, starting Ruin, item drop pool, and every
//! relic/pact effect are all wired into gameplay -- see
//! `rooms::fallen_gate::setup_fallen_gate_player` for the
//! first four and `run_setup::effects` for exactly what each relic/pact
//! does and where. `handle_begin_run_click` below is where the screen's
//! live `RunConfig` selections turn into the frozen `RunModifiers` the
//! rest of the run reads from. Only the Ruin condition's Ruin-*gain-rate*
//! and reward-quality claims beyond its starting Ruin amount remain
//! unimplemented -- see `catalog::RuinConditionEntry`'s doc comment.

use bevy::prelude::*;

use crate::characters::ClassId;
use crate::game_state::AppState;
use crate::run_setup::catalog::{self, DropPoolCategoryId, PactId, RelicId, RuinConditionId};
use crate::run_setup::{RunConfig, RunModifiers};
use crate::weapons::WeaponType;

// =========================================================
// TABS
// =========================================================

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum RunSetupTab {
    #[default]
    Class,
    Weapon,
    DropPool,
    Relic,
    Pact,
    RuinCondition,
}

const ALL_TABS: [RunSetupTab; 6] = [
    RunSetupTab::Class,
    RunSetupTab::Weapon,
    RunSetupTab::DropPool,
    RunSetupTab::Relic,
    RunSetupTab::Pact,
    RunSetupTab::RuinCondition,
];

impl RunSetupTab {
    fn label(self) -> &'static str {
        match self {
            RunSetupTab::Class => "CLASS",
            RunSetupTab::Weapon => "WEAPON",
            RunSetupTab::DropPool => "DROP POOL",
            RunSetupTab::Relic => "RELIC",
            RunSetupTab::Pact => "PACT",
            RunSetupTab::RuinCondition => "RUIN CONDITION",
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct RunSetupUiState {
    active_tab: RunSetupTab,
}

// =========================================================
// COMPONENTS
// =========================================================

#[derive(Component)]
pub(crate) struct RunSetupScreen;

#[derive(Component)]
pub(crate) struct RunSetupTabButton(RunSetupTab);

#[derive(Component)]
pub(crate) struct RunSetupTabPanel(RunSetupTab);

/// Generic hover recolor for every option card on this screen -- see
/// `update_card_hover`. Selection highlighting (this option is the
/// current choice) is a separate concern handled by each category's own
/// `BorderColor`-writing system, so hover only ever touches background.
#[derive(Component)]
pub(crate) struct RunSetupCard {
    base_bg: Color,
    hover_bg: Color,
}

/// Static description text shown in the description strip
/// (`RunSetupDescription`) while this card is hovered. Every option card
/// carries one except `ClassAbilityPreview`, whose text is computed
/// fresh each frame from the currently selected class instead of being
/// fixed at spawn time.
#[derive(Component)]
pub(crate) struct RunSetupTooltipContent(String);

#[derive(Component)]
pub(crate) struct ClassOptionButton(ClassId);

/// The "hover the player" element from the class tab: hovering it shows
/// the currently selected class's full ability kit in the description
/// strip (see `update_run_setup_description`), rather than the class's
/// own short tagline the way every other card's hover does.
#[derive(Component)]
pub(crate) struct ClassAbilityPreview;

#[derive(Component)]
pub(crate) struct ClassAbilityPreviewLabel;

#[derive(Component)]
pub(crate) struct WeaponOptionButton(WeaponType);

#[derive(Component)]
pub(crate) struct DropPoolToggleButton(DropPoolCategoryId);

#[derive(Component)]
pub(crate) struct DropPoolToggleLabel(DropPoolCategoryId);

#[derive(Component)]
pub(crate) struct RelicOptionButton(RelicId);

/// A run pact is optional -- the player may select any number of pacts,
/// including none (`RunConfig::pacts` starts empty).
#[derive(Component)]
pub(crate) struct PactOptionButton(PactId);

#[derive(Component)]
pub(crate) struct RuinConditionOptionButton(RuinConditionId);

#[derive(Component)]
pub(crate) struct RunSetupDescription;

#[derive(Component)]
pub(crate) struct RunSetupSummaryText;

/// Empty column under the run summary that other modules can fill (the
/// Steam friends panel, `net::steam_lobby`). Nothing is put here offline.
#[derive(Component)]
pub struct RunSetupSideSlot;

#[derive(Component)]
pub(crate) struct BeginRunButton;

/// The Begin Run button's text ("BEGIN RUN", or "READY" for a guest).
#[derive(Component)]
pub(crate) struct BeginRunButtonLabel;

/// Who this machine is on the run-setup screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
pub enum LobbyRole {
    /// Offline: Begin Run starts immediately.
    #[default]
    Solo,
    /// Co-op host: Begin Run waits until every guest is ready.
    Host,
    /// Joined player: the button toggles Ready; the host starts the run.
    Guest,
}

/// Co-op lobby state for the run-setup screen. Offline it stays `Solo`
/// and changes nothing. In multiplayer `net::lobby` fills it from the
/// host's view of who is ready, and reads `ready_toggle_requested` to tell
/// the host when a guest clicks READY.
#[derive(Resource, Clone, Debug, Default)]
pub struct LobbyStatus {
    pub role: LobbyRole,
    /// Guest: whether the host has this player marked ready.
    pub local_ready: bool,
    /// Joined players currently connected (excluding the host).
    pub guests: usize,
    /// How many of them are ready.
    pub ready_guests: usize,
    /// Set by a guest's READY click; consumed by `net::lobby`.
    #[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
    pub ready_toggle_requested: bool,
}

impl LobbyStatus {
    /// The host may begin once every guest is ready (always true solo or
    /// with nobody joined).
    pub fn everyone_ready(&self) -> bool {
        self.ready_guests >= self.guests
    }
}

#[derive(Component)]
pub(crate) struct BeginRunWarningText;

// =========================================================
// COLORS
//
// Plain functions rather than `const`s -- matches this file's own
// pattern of computing colors conditionally (locked/selected/hovered),
// and sidesteps needing every `Color` constructor used here to be a
// `const fn`.
// =========================================================

fn color_card_bg() -> Color {
    Color::srgb(0.105, 0.115, 0.14)
}
fn color_card_hover_bg() -> Color {
    Color::srgb(0.17, 0.18, 0.22)
}
fn color_locked_bg() -> Color {
    Color::srgb(0.07, 0.065, 0.07)
}
fn color_card_border() -> Color {
    Color::srgb(0.28, 0.29, 0.33)
}
fn color_locked_border() -> Color {
    Color::srgb(0.32, 0.22, 0.22)
}
fn color_selected_border() -> Color {
    Color::srgb(0.95, 0.82, 0.52)
}
fn color_title() -> Color {
    Color::srgb(0.95, 0.82, 0.52)
}
fn color_body_text() -> Color {
    Color::srgb(0.82, 0.82, 0.82)
}
fn color_muted_text() -> Color {
    Color::srgb(0.55, 0.58, 0.62)
}
fn color_locked_text() -> Color {
    Color::srgb(0.55, 0.46, 0.46)
}
fn color_tab_bg() -> Color {
    Color::srgb(0.09, 0.095, 0.115)
}
fn color_tab_active_bg() -> Color {
    Color::srgb(0.18, 0.155, 0.09)
}
fn color_warning_text() -> Color {
    Color::srgb(0.88, 0.42, 0.36)
}
fn color_begin_ready() -> Color {
    Color::srgb(0.2, 0.42, 0.24)
}
fn color_begin_blocked() -> Color {
    Color::srgb(0.28, 0.12, 0.12)
}

// =========================================================
// SPAWN
// =========================================================

pub(crate) fn spawn_run_setup_screen(mut commands: Commands) {
    commands
        .spawn((
            RunSetupScreen,
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                top: px(0.0),
                width: percent(100.0),
                height: percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(28.0)),
                row_gap: px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.98)),
            ZIndex(200),
        ))
        .with_children(|screen| {
            screen.spawn((
                Text::new("PREPARE YOUR DESCENT"),
                TextFont {
                    font_size: FontSize::Px(38.0),
                    ..default()
                },
                TextColor(color_title()),
            ));
            screen.spawn((
                Text::new("Choose how you begin this run. Hover any option to see what it does."),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(color_body_text()),
            ));

            // Body: tab list, tab content, and the selection summary.
            screen
                .spawn(Node {
                    width: percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(20.0),
                    min_height: px(0.0),
                    ..default()
                })
                .with_children(|body| {
                    spawn_tab_list(body);
                    spawn_tab_content(body);
                    spawn_summary_panel(body);
                });

            // Description strip: shows whatever is hovered, or the
            // active tab's current selection when nothing is.
            screen
                .spawn((
                    Node {
                        width: percent(100.0),
                        min_height: px(84.0),
                        padding: UiRect::all(px(14.0)),
                        border: UiRect::all(px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.05, 0.055, 0.07)),
                    BorderColor::all(color_card_border()),
                ))
                .with_children(|strip| {
                    strip.spawn((
                        RunSetupDescription,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(color_body_text()),
                    ));
                });

            // Footer: warning (if blocked) + Begin Run button.
            screen
                .spawn(Node {
                    width: percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|footer| {
                    footer.spawn((
                        BeginRunWarningText,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(color_warning_text()),
                        Node {
                            display: Display::None,
                            ..default()
                        },
                    ));

                    footer
                        .spawn((
                            Button,
                            BeginRunButton,
                            Node {
                                width: px(220.0),
                                height: px(54.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(px(2.0)),
                                ..default()
                            },
                            BackgroundColor(color_begin_ready()),
                            BorderColor::all(color_selected_border()),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                BeginRunButtonLabel,
                                Text::new("BEGIN RUN"),
                                TextFont {
                                    font_size: FontSize::Px(20.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                });
        });
}

fn spawn_tab_list(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            width: px(190.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(8.0),
            ..default()
        })
        .with_children(|list| {
            for tab in ALL_TABS {
                list.spawn((
                    Button,
                    RunSetupTabButton(tab),
                    Node {
                        width: percent(100.0),
                        height: px(42.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(px(2.0)),
                        ..default()
                    },
                    BackgroundColor(color_tab_bg()),
                    BorderColor::all(color_card_border()),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new(tab.label()),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(color_body_text()),
                    ));
                });
            }
        });
}

fn spawn_summary_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: px(240.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(10.0),
                padding: UiRect::all(px(14.0)),
                border: UiRect::all(px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.055, 0.07)),
            BorderColor::all(Color::srgb(0.38, 0.32, 0.20)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("YOUR RUN"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(color_title()),
            ));
            panel.spawn((
                RunSetupSummaryText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(color_body_text()),
            ));
            panel.spawn((
                RunSetupSideSlot,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6.0),
                    margin: UiRect::top(px(8.0)),
                    ..default()
                },
            ));
        });
}

fn spawn_tab_content(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(18.0),
            min_height: px(0.0),
            ..default()
        })
        .with_children(|content| {
            spawn_class_tab(content);
            spawn_weapon_tab(content);
            spawn_drop_pool_tab(content);
            spawn_relic_tab(content);
            spawn_pact_tab(content);
            spawn_ruin_condition_tab(content);
        });
}

/// Spawns one tab's panel root (`RunSetupTabPanel`, hidden unless `tab`
/// is the default/first tab) and populates it via `build`. Every
/// `spawn_*_tab` function below is this one call plus whatever content
/// is unique to that tab.
fn spawn_tab_panel(
    parent: &mut ChildSpawnerCommands,
    tab: RunSetupTab,
    build: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            RunSetupTabPanel(tab),
            Node {
                width: percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(14.0),
                display: if tab == RunSetupTab::default() {
                    Display::Flex
                } else {
                    Display::None
                },
                ..default()
            },
        ))
        .with_children(build);
}

fn spawn_card_grid(
    parent: &mut ChildSpawnerCommands,
    build: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn(Node {
            width: percent(100.0),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(12.0),
            row_gap: px(12.0),
            ..default()
        })
        .with_children(build);
}

/// Spawns one clickable option "card" -- the widget every category on
/// this screen reuses (class, weapon, relic, pact, Ruin condition, and
/// drop-pool entries alike). `marker` is the option's own identifying
/// component (e.g. `ClassOptionButton(ClassId::Vanguard)`), so each
/// category's own click-handling system finds its cards again with a
/// plain `Query<(&Interaction, &ClassOptionButton), ...>` -- adding a
/// brand new selectable thing to any category later is exactly this one
/// call plus one catalog entry, not a new widget.
fn spawn_option_card(
    parent: &mut ChildSpawnerCommands,
    marker: impl Bundle,
    name: &str,
    tooltip_text: String,
    locked: bool,
) {
    let base_bg = if locked {
        color_locked_bg()
    } else {
        color_card_bg()
    };
    let hover_bg = if locked {
        color_locked_bg()
    } else {
        color_card_hover_bg()
    };
    let border = if locked {
        color_locked_border()
    } else {
        color_card_border()
    };
    let text_color = if locked {
        color_locked_text()
    } else {
        Color::WHITE
    };

    parent
        .spawn((
            Button,
            marker,
            RunSetupCard { base_bg, hover_bg },
            RunSetupTooltipContent(tooltip_text),
            Node {
                width: px(190.0),
                min_height: px(70.0),
                padding: UiRect::all(px(10.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: px(4.0),
                border: UiRect::all(px(2.0)),
                ..default()
            },
            BackgroundColor(base_bg),
            BorderColor::all(border),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(name.to_string()),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(text_color),
            ));
            if locked {
                card.spawn((
                    Text::new("Locked"),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(color_locked_text()),
                ));
            }
        });
}

fn spawn_class_tab(parent: &mut ChildSpawnerCommands) {
    spawn_tab_panel(parent, RunSetupTab::Class, |tab| {
        tab.spawn((
            Button,
            ClassAbilityPreview,
            RunSetupCard {
                base_bg: color_card_bg(),
                hover_bg: color_card_hover_bg(),
            },
            Node {
                width: percent(100.0),
                padding: UiRect::all(px(14.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(2.0)),
                ..default()
            },
            BackgroundColor(color_card_bg()),
            BorderColor::all(color_selected_border()),
        ))
        .with_children(|card| {
            card.spawn((
                ClassAbilityPreviewLabel,
                Text::new("Hover to view this class's abilities"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(color_title()),
            ));
        });

        spawn_card_grid(tab, |grid| {
            for entry in catalog::CLASSES {
                spawn_option_card(
                    grid,
                    ClassOptionButton(entry.id),
                    entry.name,
                    format!("{}\n\n{}", entry.tagline, entry.description),
                    !entry.unlocked,
                );
            }
        });
    });
}

fn spawn_weapon_tab(parent: &mut ChildSpawnerCommands) {
    spawn_tab_panel(parent, RunSetupTab::Weapon, |tab| {
        tab.spawn((
            Text::new("Classes can use every weapon family -- this only picks how you start."),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(color_muted_text()),
        ));

        spawn_card_grid(tab, |grid| {
            for entry in catalog::WEAPONS {
                spawn_option_card(
                    grid,
                    WeaponOptionButton(entry.weapon_type),
                    entry.name,
                    entry.description.to_string(),
                    false,
                );
            }
        });
    });
}

fn spawn_drop_pool_tab(parent: &mut ChildSpawnerCommands) {
    spawn_tab_panel(parent, RunSetupTab::DropPool, |tab| {
        tab.spawn((
            Text::new(format!(
                "Choose which item categories can drop this run. At least {} must stay enabled.",
                catalog::MIN_ENABLED_DROP_POOL_CATEGORIES,
            )),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(color_muted_text()),
        ));

        spawn_card_grid(tab, |grid| {
            for entry in catalog::DROP_POOL_CATEGORIES {
                grid.spawn((
                    Button,
                    DropPoolToggleButton(entry.id),
                    RunSetupCard {
                        base_bg: color_card_bg(),
                        hover_bg: color_card_hover_bg(),
                    },
                    RunSetupTooltipContent(entry.description.to_string()),
                    Node {
                        width: px(190.0),
                        min_height: px(70.0),
                        padding: UiRect::all(px(10.0)),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        row_gap: px(4.0),
                        border: UiRect::all(px(2.0)),
                        ..default()
                    },
                    BackgroundColor(color_card_bg()),
                    BorderColor::all(color_selected_border()),
                ))
                .with_children(|card| {
                    card.spawn((
                        DropPoolToggleLabel(entry.id),
                        Text::new(entry.name.to_string()),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            }
        });
    });
}

fn spawn_relic_tab(parent: &mut ChildSpawnerCommands) {
    spawn_tab_panel(parent, RunSetupTab::Relic, |tab| {
        spawn_card_grid(tab, |grid| {
            for entry in catalog::RELICS {
                spawn_option_card(
                    grid,
                    RelicOptionButton(entry.id),
                    entry.name,
                    entry.description.to_string(),
                    false,
                );
            }
        });
    });
}

fn spawn_pact_tab(parent: &mut ChildSpawnerCommands) {
    spawn_tab_panel(parent, RunSetupTab::Pact, |tab| {
        tab.spawn((
            Text::new(
                "Optional. Select any number of pacts -- each trades safety for a stronger \
                run, or the reverse. Selecting none plays the run as normal.",
            ),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(color_muted_text()),
        ));

        spawn_card_grid(tab, |grid| {
            for entry in catalog::PACTS {
                spawn_option_card(
                    grid,
                    PactOptionButton(entry.id),
                    entry.name,
                    entry.description.to_string(),
                    false,
                );
            }
        });
    });
}

fn spawn_ruin_condition_tab(parent: &mut ChildSpawnerCommands) {
    spawn_tab_panel(parent, RunSetupTab::RuinCondition, |tab| {
        spawn_card_grid(tab, |grid| {
            for entry in catalog::RUIN_CONDITIONS {
                spawn_option_card(
                    grid,
                    RuinConditionOptionButton(entry.id),
                    entry.name,
                    entry.description.to_string(),
                    !entry.unlocked,
                );
            }
        });
    });
}

// =========================================================
// STATE TRANSITIONS (see `game_state::AppState`)
// =========================================================

pub(crate) fn show_run_setup_screen(mut screen: Single<&mut Node, With<RunSetupScreen>>) {
    screen.display = Display::Flex;
}

pub(crate) fn hide_run_setup_screen(mut screen: Single<&mut Node, With<RunSetupScreen>>) {
    screen.display = Display::None;
}

// =========================================================
// TABS
// =========================================================

pub(crate) fn handle_tab_clicks(
    mut ui_state: ResMut<RunSetupUiState>,
    clicked: Query<(&Interaction, &RunSetupTabButton), Changed<Interaction>>,
) {
    for (interaction, tab_button) in &clicked {
        if *interaction == Interaction::Pressed {
            ui_state.active_tab = tab_button.0;
        }
    }
}

pub(crate) fn update_tab_visuals(
    ui_state: Res<RunSetupUiState>,
    mut tab_buttons: Query<(&RunSetupTabButton, &mut BackgroundColor, &mut BorderColor)>,
    mut panels: Query<(&RunSetupTabPanel, &mut Node)>,
) {
    for (tab_button, mut bg, mut border) in &mut tab_buttons {
        let active = tab_button.0 == ui_state.active_tab;
        *bg = BackgroundColor(if active {
            color_tab_active_bg()
        } else {
            color_tab_bg()
        });
        *border = BorderColor::all(if active {
            color_selected_border()
        } else {
            color_card_border()
        });
    }

    for (panel, mut node) in &mut panels {
        node.display = if panel.0 == ui_state.active_tab {
            Display::Flex
        } else {
            Display::None
        };
    }
}

// =========================================================
// HOVER (shared by every card on the screen)
// =========================================================

pub(crate) fn update_card_hover(
    mut cards: Query<(&Interaction, &RunSetupCard, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, card, mut bg) in &mut cards {
        bg.0 = match interaction {
            Interaction::Hovered | Interaction::Pressed => card.hover_bg,
            Interaction::None => card.base_bg,
        };
    }
}

// =========================================================
// SELECTION (one click-handler + highlighter pair per category)
// =========================================================

pub(crate) fn handle_class_options(
    mut run_config: ResMut<RunConfig>,
    clicked: Query<(&Interaction, &ClassOptionButton), Changed<Interaction>>,
    mut cards: Query<(&ClassOptionButton, &mut BorderColor)>,
) {
    for (interaction, button) in &clicked {
        if *interaction == Interaction::Pressed && catalog::class_entry(button.0).unlocked {
            run_config.class = button.0;
        }
    }

    for (button, mut border) in &mut cards {
        let entry = catalog::class_entry(button.0);
        *border = BorderColor::all(if !entry.unlocked {
            color_locked_border()
        } else if button.0 == run_config.class {
            color_selected_border()
        } else {
            color_card_border()
        });
    }
}

pub(crate) fn handle_weapon_options(
    mut run_config: ResMut<RunConfig>,
    clicked: Query<(&Interaction, &WeaponOptionButton), Changed<Interaction>>,
    mut cards: Query<(&WeaponOptionButton, &mut BorderColor)>,
) {
    for (interaction, button) in &clicked {
        if *interaction == Interaction::Pressed {
            run_config.weapon = button.0;
        }
    }

    for (button, mut border) in &mut cards {
        let selected = button.0 == run_config.weapon;
        *border = BorderColor::all(if selected {
            color_selected_border()
        } else {
            color_card_border()
        });
    }
}

pub(crate) fn handle_drop_pool_toggles(
    mut run_config: ResMut<RunConfig>,
    clicked: Query<(&Interaction, &DropPoolToggleButton), Changed<Interaction>>,
    mut cards: Query<(&DropPoolToggleButton, &mut BorderColor)>,
) {
    for (interaction, button) in &clicked {
        if *interaction == Interaction::Pressed {
            run_config.drop_pool.toggle(button.0);
        }
    }

    for (button, mut border) in &mut cards {
        let enabled = run_config.drop_pool.is_selected(button.0);
        *border = BorderColor::all(if enabled {
            color_selected_border()
        } else {
            color_locked_border()
        });
    }
}

pub(crate) fn update_drop_pool_labels(
    run_config: Res<RunConfig>,
    mut labels: Query<(&DropPoolToggleLabel, &mut Text)>,
) {
    for (label, mut text) in &mut labels {
        let entry = catalog::drop_pool_entry(label.0);
        let enabled = run_config.drop_pool.is_selected(label.0);
        **text = format!(
            "{}\n{}",
            entry.name,
            if enabled { "Enabled" } else { "Disabled" }
        );
    }
}

/// Relics are multi-select -- clicking a card toggles it on or off
/// rather than replacing the current pick, so any number of relics
/// (including all of them) can be active at once (`RunConfig::relics`).
pub(crate) fn handle_relic_options(
    mut run_config: ResMut<RunConfig>,
    clicked: Query<(&Interaction, &RelicOptionButton), Changed<Interaction>>,
    mut cards: Query<(&RelicOptionButton, &mut BorderColor)>,
) {
    for (interaction, button) in &clicked {
        if *interaction == Interaction::Pressed {
            run_config.relics.toggle(button.0);
        }
    }

    for (button, mut border) in &mut cards {
        let selected = run_config.relics.is_selected(button.0);
        *border = BorderColor::all(if selected {
            color_selected_border()
        } else {
            color_card_border()
        });
    }
}

/// Pacts are multi-select, same as relics -- see `handle_relic_options`.
pub(crate) fn handle_pact_options(
    mut run_config: ResMut<RunConfig>,
    clicked: Query<(&Interaction, &PactOptionButton), Changed<Interaction>>,
    mut cards: Query<(&PactOptionButton, &mut BorderColor)>,
) {
    for (interaction, button) in &clicked {
        if *interaction == Interaction::Pressed {
            run_config.pacts.toggle(button.0);
        }
    }

    for (button, mut border) in &mut cards {
        let selected = run_config.pacts.is_selected(button.0);
        *border = BorderColor::all(if selected {
            color_selected_border()
        } else {
            color_card_border()
        });
    }
}

pub(crate) fn handle_ruin_condition_options(
    mut run_config: ResMut<RunConfig>,
    clicked: Query<(&Interaction, &RuinConditionOptionButton), Changed<Interaction>>,
    mut cards: Query<(&RuinConditionOptionButton, &mut BorderColor)>,
) {
    for (interaction, button) in &clicked {
        if *interaction == Interaction::Pressed && catalog::ruin_condition_entry(button.0).unlocked
        {
            run_config.ruin_condition = button.0;
        }
    }

    for (button, mut border) in &mut cards {
        let entry = catalog::ruin_condition_entry(button.0);
        *border = BorderColor::all(if !entry.unlocked {
            color_locked_border()
        } else if button.0 == run_config.ruin_condition {
            color_selected_border()
        } else {
            color_card_border()
        });
    }
}

// =========================================================
// DESCRIPTION STRIP + SUMMARY
// =========================================================

pub(crate) fn update_class_preview_label(
    run_config: Res<RunConfig>,
    mut label: Single<&mut Text, With<ClassAbilityPreviewLabel>>,
) {
    let name = catalog::class_entry(run_config.class).name;
    **label = format!("Hover to view {name}'s abilities").into();
}

fn class_abilities_description(class: ClassId) -> String {
    let entry = catalog::class_entry(class);
    let mut text = format!("{} -- Abilities", entry.name);
    for ability in entry.abilities {
        text.push_str(&format!(
            "\n[{}] {} -- {}",
            ability.key_hint, ability.name, ability.description
        ));
    }
    text
}

/// Joins every active relic's or pact's own description onto one line
/// each, or a "nothing chosen yet" placeholder when the selection is
/// empty -- shared by the Relic and Pact tabs' fallback text below.
fn joined_descriptions<'a>(
    names_and_descriptions: impl Iterator<Item = &'a str>,
    empty: &str,
) -> String {
    let joined = names_and_descriptions.collect::<Vec<_>>().join("\n");
    if joined.is_empty() {
        empty.to_string()
    } else {
        joined
    }
}

fn fallback_description(tab: RunSetupTab, run_config: &RunConfig) -> String {
    match tab {
        RunSetupTab::Class => catalog::class_entry(run_config.class)
            .description
            .to_string(),
        RunSetupTab::Weapon => catalog::weapon_entry(run_config.weapon)
            .description
            .to_string(),
        RunSetupTab::DropPool => format!(
            "{} of {} item-drop-pool categories are enabled (minimum {}).",
            run_config.drop_pool.count(),
            catalog::DROP_POOL_CATEGORIES.len(),
            catalog::MIN_ENABLED_DROP_POOL_CATEGORIES,
        ),
        RunSetupTab::Relic => joined_descriptions(
            run_config
                .relics
                .iter()
                .map(|&id| catalog::relic_entry(id).description),
            "No relics chosen -- select any number of starting relics.",
        ),
        RunSetupTab::Pact => joined_descriptions(
            run_config
                .pacts
                .iter()
                .map(|&id| catalog::pact_entry(id).description),
            "No pacts chosen -- play the run as normal.",
        ),
        RunSetupTab::RuinCondition => catalog::ruin_condition_entry(run_config.ruin_condition)
            .description
            .to_string(),
    }
}

pub(crate) fn update_run_setup_description(
    ui_state: Res<RunSetupUiState>,
    run_config: Res<RunConfig>,
    preview: Query<&Interaction, With<ClassAbilityPreview>>,
    tooltips: Query<(&Interaction, &RunSetupTooltipContent)>,
    mut description: Single<&mut Text, With<RunSetupDescription>>,
) {
    if preview
        .iter()
        .any(|interaction| *interaction == Interaction::Hovered)
    {
        **description = class_abilities_description(run_config.class).into();
        return;
    }

    if let Some((_, content)) = tooltips
        .iter()
        .find(|(interaction, _)| **interaction == Interaction::Hovered)
    {
        **description = content.0.clone().into();
        return;
    }

    **description = fallback_description(ui_state.active_tab, &run_config).into();
}

/// Comma-joins every selected relic's/pact's name, or `empty` when
/// nothing is selected -- the summary panel's compact one-line version
/// of `joined_descriptions`.
fn joined_names<'a>(names: impl Iterator<Item = &'a str>, empty: &'a str) -> String {
    let joined = names.collect::<Vec<_>>().join(", ");
    if joined.is_empty() {
        empty.to_string()
    } else {
        joined
    }
}

pub(crate) fn update_run_setup_summary(
    run_config: Res<RunConfig>,
    mut summary: Single<&mut Text, With<RunSetupSummaryText>>,
) {
    let class = catalog::class_entry(run_config.class).name;
    let weapon = catalog::weapon_entry(run_config.weapon).name;
    let relics = joined_names(
        run_config
            .relics
            .iter()
            .map(|&id| catalog::relic_entry(id).name),
        "None",
    );
    let pacts = joined_names(
        run_config
            .pacts
            .iter()
            .map(|&id| catalog::pact_entry(id).name),
        "None",
    );
    let ruin = catalog::ruin_condition_entry(run_config.ruin_condition).name;

    **summary = format!(
        "Class\n{class}\n\nWeapon\n{weapon}\n\nDrop pool\n{}/{} categories\n\nRelics\n{relics}\n\nPacts\n{pacts}\n\nRuin condition\n{ruin}",
        run_config.drop_pool.count(),
        catalog::DROP_POOL_CATEGORIES.len(),
    )
    .into();
}

// =========================================================
// BEGIN RUN
// =========================================================

pub(crate) fn update_begin_run_button(
    run_config: Res<RunConfig>,
    lobby: Res<LobbyStatus>,
    mut button: Query<(&mut BackgroundColor, &mut BorderColor), With<BeginRunButton>>,
    mut label: Query<&mut Text, (With<BeginRunButtonLabel>, Without<BeginRunWarningText>)>,
    mut warning: Single<(&mut Text, &mut Node), With<BeginRunWarningText>>,
) {
    let drop_pool_ok = run_config.drop_pool_meets_minimum();

    // What the button does and whether it can be pressed right now.
    let (button_text, enabled) = match lobby.role {
        LobbyRole::Solo => ("BEGIN RUN", drop_pool_ok),
        LobbyRole::Host => ("BEGIN RUN", drop_pool_ok && lobby.everyone_ready()),
        LobbyRole::Guest => (
            if lobby.local_ready {
                "UNREADY"
            } else {
                "READY"
            },
            true,
        ),
    };

    for (mut bg, mut border) in &mut button {
        *bg = BackgroundColor(if enabled {
            color_begin_ready()
        } else {
            color_begin_blocked()
        });
        *border = BorderColor::all(if enabled {
            color_selected_border()
        } else {
            color_locked_border()
        });
    }

    for mut text in &mut label {
        if **text != button_text {
            **text = button_text.to_string();
        }
    }

    // Every reason the run can't start yet, plus co-op status.
    let mut lines = Vec::new();
    if !drop_pool_ok {
        lines.push(format!(
            "Enable at least {} item-drop-pool categories to begin (currently {}).",
            catalog::MIN_ENABLED_DROP_POOL_CATEGORIES,
            run_config.drop_pool.count(),
        ));
    }
    match lobby.role {
        LobbyRole::Solo => {}
        LobbyRole::Host => {
            if lobby.guests > 0 {
                lines.push(format!(
                    "Players ready: {}/{}{}",
                    lobby.ready_guests,
                    lobby.guests,
                    if lobby.everyone_ready() {
                        ""
                    } else {
                        " - waiting for everyone to ready up."
                    },
                ));
            }
            lines.push(
                "Drop pool, relics, pacts and Ruin condition are shared by the whole party."
                    .to_string(),
            );
        }
        LobbyRole::Guest => {
            lines.push(if lobby.local_ready {
                "You are ready. Waiting for the host to begin.".to_string()
            } else {
                "Press READY when you're set. The host begins the run.".to_string()
            });
            lines.push(
                "Drop pool, relics, pacts and Ruin condition are shared by the whole party."
                    .to_string(),
            );
        }
    }

    if lines.is_empty() {
        warning.1.display = Display::None;
    } else {
        let text = lines.join("\n");
        if **warning.0 != text {
            **warning.0 = text;
        }
        warning.1.display = Display::Flex;
    }
}

/// A guest's READY click: ask the host to flip this player's ready state
/// (`net::lobby` sends it). The host's answer comes back as
/// `LobbyStatus::local_ready`.
pub(crate) fn handle_ready_click(
    mut lobby: ResMut<LobbyStatus>,
    clicked: Query<&Interaction, (With<BeginRunButton>, Changed<Interaction>)>,
) {
    if lobby.role != LobbyRole::Guest {
        return;
    }
    for interaction in &clicked {
        if *interaction == Interaction::Pressed {
            lobby.ready_toggle_requested = true;
        }
    }
}

pub(crate) fn handle_begin_run_click(
    run_config: Res<RunConfig>,
    lobby: Res<LobbyStatus>,
    mut run_modifiers: ResMut<RunModifiers>,
    mut next_state: ResMut<NextState<AppState>>,
    clicked: Query<&Interaction, (With<BeginRunButton>, Changed<Interaction>)>,
) {
    // Co-op: the host waits for every joined player to be ready.
    if !run_config.drop_pool_meets_minimum() || !lobby.everyone_ready() {
        return;
    }

    for interaction in &clicked {
        if *interaction == Interaction::Pressed {
            // Lock in the run's relics/pacts as one resolved set of
            // numbers right as the run starts -- see
            // `run_setup::RunModifiers::from_selections`. Every gameplay
            // system that reads `RunModifiers` for the rest of the run
            // sees this exact snapshot, even if a future screen lets the
            // player revisit `RunConfig` later.
            *run_modifiers = RunModifiers::from_selections(
                run_config.relics.as_slice(),
                run_config.pacts.as_slice(),
            );
            next_state.set(AppState::InGame);
        }
    }
}
