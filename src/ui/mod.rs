use bevy::prelude::*;

mod map;
mod ruin_screen;
mod run_setup_screen;

pub use map::MapUiState;
// Filled by multiplayer (`net::lobby`).
pub use ruin_screen::RuinScreenState;
#[cfg(feature = "multiplayer")]
pub use run_setup_screen::LobbyRole;
pub use run_setup_screen::LobbyStatus;
// Filled by the Steam friends panel (`net::steam_lobby`, `steam` builds).
#[cfg_attr(not(feature = "steam"), allow(unused_imports))]
pub use run_setup_screen::RunSetupSideSlot;

use crate::combat::{Barriers, Health};
use crate::game_state::AppState;
use crate::ruin::RuinState;

use crate::camera::CameraTarget;
use crate::player::{LifeState, Player, PlayerCharacter};

use crate::characters::class_abilities::{ClassAbilityDisplay, ClassAbilityHud, ClassResourceHud};
use crate::weapons::{
    BELLRINGER_ID, DANCER_IN_THE_DARK_ID, Equipment, GearTier, HOLLOW_GOSPEL_ID, ItemSnapshotKind,
    KINGSLAYER_ID, LIVING_CODEX_ID, PlayerCombat, PlayerInventory, PlayerInventorySnapshot,
    RUINPIERCER_ID, SAINTS_NAIL_ID, SAINTS_TESTAMENT_ID, WIDOWS_KISS_ID, Weapon, WeaponType,
};

// =========================================================
// UI COMPONENTS
// =========================================================

#[derive(Component)]
struct PlayerHealthBar;

/// Translucent shield strip drawn over the health bar's unfilled portion,
/// showing how much of `combat::Barriers` the player currently has.
/// Absorbs damage before the red fill (see `combat::apply_player_damage`),
/// so this overlay is what tells the player "that hit won't cost you HP
/// yet."
#[derive(Component)]
struct PlayerBarrierOverlay;

#[derive(Component)]
struct WeaponIndicator;

/// Container for the optional class-resource readout (e.g. Runesage's
/// Runes) beneath the ability bar. Hidden whenever the player has no
/// `ClassResourceHud` -- see `update_class_resource_bar`.
#[derive(Component)]
struct ClassResourceBar;

#[derive(Component)]
struct ClassResourceLabel;

/// One pip in the resource bar's row. `0` is the leftmost pip.
/// `CLASS_RESOURCE_MAX_PIPS` fixes how many exist; a class whose
/// resource cap is lower than that just leaves the extra pips hidden
/// (see `update_class_resource_bar`).
#[derive(Component)]
struct ClassResourcePip(usize);

/// Headroom above Runesage's current ~6-Rune cap so a future class with
/// a slightly larger resource pool doesn't need a UI change too.
const CLASS_RESOURCE_MAX_PIPS: usize = 8;

#[derive(Component)]
struct Crosshair;

/// Transient banner shown over the HUD whenever
/// `ruin::RuinState::warning_message` has something to say -- currently
/// just the passive-growth warning ("The Ruin is spreading..."), but any
/// future Ruin event that wants a moment of on-screen notice can reuse
/// this same banner by routing through `RuinState::warning_message`
/// instead of adding a new HUD element.
#[derive(Component)]
struct RuinWarningBanner;

/// Large centred text for the Downed -> Lost flow and a party wipe
/// (`player::life`): your own state, then any teammate who needs a revive.
#[derive(Component)]
struct LifeStatusBanner;

#[derive(Component, Clone, Copy)]
enum AbilitySlotKind {
    Class(usize),
    Weapon,
}

/// What this machine's HUD shows about a player character that the HUD
/// can't read from the character's own combat components on every
/// machine. The simulating machine (offline or the host) fills it for every
/// player character (`publish_hud_snapshots`); in multiplayer it is
/// replicated, so a joined player's HUD can show their own barrier and
/// weapon ability, which their client doesn't simulate.
#[derive(Component, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub struct PlayerHudSnapshot {
    /// Total barrier (`combat::Barriers::total_amount`).
    pub barrier: f32,
    /// e.g. "Common Bow".
    pub weapon: String,
    /// The weapon ability's HUD name, e.g. "PIERCING SHOT".
    pub weapon_ability: String,
    /// Remaining weapon-ability cooldown in tenths of a second, rounded
    /// up, so it only changes ten times a second and is never 0 early.
    pub weapon_ability_cooldown_tenths: u16,
    /// The class ability slots as the HUD draws them (empty without a
    /// class kit), e.g. "[Q] LUNGE\n2.3s".
    pub class_slots: Vec<HudAbilitySlot>,
    /// The class resource (e.g. Runesage's Runes), if the class has one.
    pub class_resource: Option<HudClassResource>,
}

/// One class ability slot in `PlayerHudSnapshot`.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub struct HudAbilitySlot {
    pub label: String,
    pub ready: bool,
}

/// `ClassResourceHud` in `PlayerHudSnapshot`.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub struct HudClassResource {
    pub label: String,
    pub current: u8,
    pub max: u8,
}

#[derive(Component)]
struct EnemyHealthBar {
    enemy: Entity,
}

#[derive(Component)]
struct EnemyHealthFill {
    enemy: Entity,
}

// =========================================================
// UI PLUGIN
// =========================================================

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapUiState>();
        app.init_resource::<RuinScreenState>();
        app.add_systems(
            OnExit(AppState::InGame),
            (
                crate::game_state::reset_resource::<MapUiState>,
                crate::game_state::reset_resource::<RuinScreenState>,
                map::hide_map_screen,
                ruin_screen::hide_ruin_screen,
            ),
        );
        app.init_resource::<run_setup_screen::RunSetupUiState>();
        app.init_resource::<LobbyStatus>();

        app.add_systems(Startup, map::spawn_map_ui);
        app.add_systems(Startup, ruin_screen::spawn_ruin_screen);
        app.add_systems(Startup, run_setup_screen::spawn_run_setup_screen);
        app.add_systems(
            OnEnter(AppState::RunSetup),
            run_setup_screen::show_run_setup_screen,
        );
        app.add_systems(
            OnExit(AppState::RunSetup),
            run_setup_screen::hide_run_setup_screen,
        );

        // The HUD and the map/inventory/Ruin overlays only matter once a
        // run actually exists (their queries are harmless no-ops before
        // that -- there's no Player yet -- but gating them keeps a stray
        // Tab/M/I press during run-setup from doing anything, and keeps
        // this system list an explicit statement of "these are in-run
        // screens").
        // A joined co-op player has no local `Player` (their character is
        // simulated by the host), so they get a smaller HUD that follows
        // their own character (`CameraTarget`).
        app.add_systems(
            OnEnter(AppState::InGame),
            spawn_joined_player_hud.run_if(not(crate::net::has_simulation_authority)),
        );

        app.add_systems(
            Update,
            (
                publish_hud_snapshots.run_if(crate::net::has_simulation_authority),
                update_player_health_bar,
                update_player_barrier_overlay,
                update_weapon_indicator,
                update_ability_bar,
                update_class_resource_bar,
                update_enemy_health_bars,
                cleanup_enemy_health_bars,
                update_ruin_warning_banner,
                update_life_status_banner,
            )
                .run_if(in_state(AppState::InGame)),
        );

        // The three full-screen overlays (map, inventory, Ruin) are
        // mutually exclusive -- each one's own toggle system opens and
        // closes it, and each other screen's "close_when_X_opens" system
        // reacts by closing itself if it was open. See `ui::map` and
        // `weapons::inventory_ui` for the other two screens' half of
        // this.
        app.add_systems(
            Update,
            (
                map::toggle_map,
                map::close_map_when_inventory_opens,
                map::close_map_when_ruin_opens,
                map::update_map_display,
                ruin_screen::toggle_ruin_screen,
                ruin_screen::close_ruin_screen_when_other_ui_opens,
                ruin_screen::update_ruin_screen_display,
            )
                .run_if(in_state(AppState::InGame)),
        );

        // The run-setup screen itself -- see `ui::run_setup_screen`'s doc
        // comment for how its systems fit together. Only runs during
        // `AppState::RunSetup`; the screen is hidden the moment
        // `AppState::InGame` starts (see `OnExit` above).
        app.add_systems(
            Update,
            (
                run_setup_screen::handle_tab_clicks,
                run_setup_screen::update_tab_visuals,
                run_setup_screen::update_card_hover,
                run_setup_screen::handle_class_options,
                run_setup_screen::handle_weapon_options,
                run_setup_screen::handle_drop_pool_toggles,
                run_setup_screen::update_drop_pool_labels,
                run_setup_screen::handle_relic_options,
                run_setup_screen::handle_pact_options,
                run_setup_screen::handle_ruin_condition_options,
            )
                .run_if(in_state(AppState::RunSetup)),
        );
        app.add_systems(
            Update,
            (
                run_setup_screen::update_class_preview_label,
                run_setup_screen::update_run_setup_description,
                run_setup_screen::update_run_setup_summary,
                run_setup_screen::update_begin_run_button,
                // A joined co-op client waits for the host to start the
                // run instead of starting its own.
                run_setup_screen::handle_begin_run_click
                    .run_if(crate::net::has_simulation_authority),
                // A joined player's button toggles Ready instead.
                run_setup_screen::handle_ready_click
                    .run_if(not(crate::net::has_simulation_authority)),
            )
                .run_if(in_state(AppState::RunSetup)),
        );
    }
}

// =========================================================
// MAIN HUD
// =========================================================

pub fn spawn_hud(commands: &mut Commands) {
    spawn_player_health_bar(commands);
    spawn_crosshair(commands);
    spawn_weapon_indicator(commands);
    spawn_ability_bar(commands);
    spawn_class_resource_bar(commands);
    spawn_ruin_warning_banner(commands);
    spawn_life_status_banner(commands);
}

/// The HUD of a joined co-op player: health (and barrier), crosshair,
/// weapon, the ability bar (class slots shown once their class has a kit
/// on the host), the class resource bar, and the Ruin warning banner (Ruin
/// is shared, copied from the host by `net::ruin`). Every element reads
/// the character this machine's camera follows, like the full HUD.
fn spawn_joined_player_hud(mut commands: Commands) {
    spawn_player_health_bar(&mut commands);
    spawn_crosshair(&mut commands);
    spawn_weapon_indicator(&mut commands);
    spawn_ability_bar(&mut commands);
    spawn_class_resource_bar(&mut commands);
    spawn_ruin_warning_banner(&mut commands);
    spawn_life_status_banner(&mut commands);
}

/// Fills `PlayerHudSnapshot` for every player character this machine
/// simulates. Only writes when something shown changes.
fn publish_hud_snapshots(
    mut commands: Commands,
    mut characters: Query<
        (
            Entity,
            &Weapon,
            &PlayerCombat,
            Option<&Barriers>,
            Option<&ClassAbilityHud>,
            Option<&ClassResourceHud>,
            Option<&mut PlayerHudSnapshot>,
        ),
        With<PlayerCharacter>,
    >,
) {
    for (entity, weapon, combat, barriers, class_hud, class_resource, snapshot) in &mut characters {
        let wanted = PlayerHudSnapshot {
            barrier: barriers.map_or(0.0, Barriers::total_amount),
            weapon: format!("{:?} {}", weapon.tier, weapon_type_name(weapon.weapon_type)),
            weapon_ability: weapon_ability_name(weapon).to_string(),
            weapon_ability_cooldown_tenths: (combat.ability_cooldown.max(0.0) * 10.0).ceil() as u16,
            class_slots: class_hud.map_or_else(Vec::new, |hud| {
                hud.slots
                    .iter()
                    .map(|slot| {
                        let (label, ready) = class_ability_label(*slot);
                        HudAbilitySlot { label, ready }
                    })
                    .collect()
            }),
            class_resource: class_resource.map(|resource| HudClassResource {
                label: resource.label.to_string(),
                current: resource.current,
                max: resource.max,
            }),
        };

        match snapshot {
            Some(mut snapshot) => {
                snapshot.set_if_neq(wanted);
            }
            None => {
                commands.entity(entity).insert(wanted);
            }
        }
    }
}

// =========================================================
// ABILITY BAR
// =========================================================

fn spawn_ability_bar(commands: &mut Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: percent(50.0),
            bottom: px(22.0),
            width: px(656.0),
            height: px(66.0),
            margin: UiRect {
                left: px(-328.0),
                ..default()
            },
            flex_direction: FlexDirection::Row,
            // All four slots fill the row exactly; a joined player's HUD
            // hides the class slots, leaving the weapon slot centered.
            justify_content: JustifyContent::Center,
            column_gap: px(8.0),
            ..default()
        })
        .with_children(|parent| {
            for (kind, label) in [
                (AbilitySlotKind::Class(0), "[Q] CLASS ABILITY 1\nREADY"),
                (AbilitySlotKind::Class(1), "[E] CLASS ABILITY 2\nREADY"),
                (AbilitySlotKind::Class(2), "[R] CLASS ABILITY 3\nREADY"),
                (AbilitySlotKind::Weapon, "[RMB] WEAPON\nREADY"),
            ] {
                parent.spawn((
                    kind,
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(17.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.95, 1.0)),
                    TextLayout::justify(Justify::Center),
                    Node {
                        width: px(158.0),
                        height: px(66.0),
                        padding: UiRect::all(px(7.0)),
                        border: UiRect::all(px(2.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.25, 0.65, 0.85)),
                    BackgroundColor(Color::srgba(0.035, 0.075, 0.11, 0.92)),
                ));
            }
        });
}

/// Reads the character this machine's camera follows: the local `Player`
/// (offline / host), or a joined player's own character, which has no
/// class kit or combat components here, only a replicated
/// `PlayerHudSnapshot`. Class slots are hidden when there is no class kit.
fn update_ability_bar(
    subject: Query<
        (
            Has<Player>,
            Option<&ClassAbilityHud>,
            Option<(&PlayerCombat, &Weapon)>,
            Option<&PlayerHudSnapshot>,
        ),
        With<CameraTarget>,
    >,
    mut slots: Query<(
        &AbilitySlotKind,
        &mut Text,
        &mut TextColor,
        &mut BorderColor,
        &mut BackgroundColor,
        &mut Node,
    )>,
) {
    let Ok((is_local_player, class_hud, weapon_state, snapshot)) = subject.single() else {
        return;
    };

    // The weapon ability's name and remaining cooldown.
    let weapon_ability = match (weapon_state, snapshot) {
        (Some((combat, weapon)), _) => Some((
            weapon_ability_name(weapon).to_string(),
            combat.ability_cooldown,
        )),
        (None, Some(snapshot)) => Some((
            snapshot.weapon_ability.clone(),
            f32::from(snapshot.weapon_ability_cooldown_tenths) / 10.0,
        )),
        (None, None) => None,
    };

    for (kind, mut text, mut text_color, mut border, mut background, mut node) in &mut slots {
        let (label, ready) = match kind {
            AbilitySlotKind::Class(index) => {
                // The local `Player` reads its class kit; a joined player
                // reads the slots the host sends (none until their class
                // has a kit, in which case the slots are hidden).
                let slot = if is_local_player {
                    class_hud.map(|class_hud| class_ability_label(class_hud.slots[*index]))
                } else {
                    snapshot
                        .and_then(|snapshot| snapshot.class_slots.get(*index))
                        .map(|slot| (slot.label.clone(), slot.ready))
                };
                let display = if is_local_player || slot.is_some() {
                    Display::Flex
                } else {
                    Display::None
                };
                if node.display != display {
                    node.display = display;
                }
                let Some(slot) = slot else {
                    continue;
                };
                slot
            }
            AbilitySlotKind::Weapon => {
                let Some((name, cooldown)) = &weapon_ability else {
                    continue;
                };
                (
                    cooldown_label(&format!("[RMB] {name}"), *cooldown),
                    *cooldown <= 0.0,
                )
            }
        };

        **text = label;
        text_color.0 = if ready {
            Color::srgb(0.88, 0.98, 1.0)
        } else {
            Color::srgb(0.52, 0.58, 0.64)
        };
        *border = BorderColor::all(if ready {
            Color::srgb(0.25, 0.75, 1.0)
        } else {
            Color::srgb(0.18, 0.24, 0.29)
        });
        background.0 = if ready {
            Color::srgba(0.035, 0.10, 0.15, 0.94)
        } else {
            Color::srgba(0.025, 0.035, 0.045, 0.90)
        };
    }
}

fn class_ability_label(
    slot: crate::characters::class_abilities::ClassAbilityHudSlot,
) -> (String, bool) {
    let name = format!("[{}] {}", slot.input, slot.name);

    match slot.display {
        ClassAbilityDisplay::Ready => (format!("{}\nREADY", name), true),
        ClassAbilityDisplay::Cooldown(seconds) => (format!("{}\n{:.1}s", name, seconds), false),
        ClassAbilityDisplay::Counter { current, max } => {
            (format!("{}\n{} / {}", name, current, max), current > 0)
        }
        ClassAbilityDisplay::Active(status) => (format!("{}\n{}", name, status), true),
        ClassAbilityDisplay::Locked(reason) => (format!("{}\n{}", name, reason), false),
    }
}

fn cooldown_label(name: &str, cooldown: f32) -> String {
    if cooldown <= 0.0 {
        format!("{}\nREADY", name)
    } else {
        format!("{}\n{:.1}s", name, cooldown)
    }
}

fn weapon_ability_name(weapon: &Weapon) -> &'static str {
    match (weapon.weapon_type, weapon.tier) {
        (WeaponType::Longsword, GearTier::Common) => "LUNGING SLASH",
        (WeaponType::Longsword, GearTier::Rare) => "CROSSCUT",
        (WeaponType::Longsword, GearTier::Epic) => "BLADE RUSH",
        (WeaponType::Greatsword, GearTier::Common) => "CHARGED CLEAVE",
        (WeaponType::Greatsword, GearTier::Rare) => "EXECUTION SWEEP",
        (WeaponType::Greatsword, GearTier::Epic) => "PHANTOM CLEAVE",
        (WeaponType::Bow, GearTier::Common) => "PIERCING SHOT",
        (WeaponType::Bow, GearTier::Rare) => "SPLINTER SHOT",
        (WeaponType::Bow, GearTier::Epic) => "RICOCHET ARROW",
        (WeaponType::Hammer, GearTier::Common) => "GROUND SLAM",
        (WeaponType::Hammer, GearTier::Rare) => "SEISMIC STRIKE",
        (WeaponType::Hammer, GearTier::Epic) => "EARTHSHATTER",
        (WeaponType::Crossbow, GearTier::Common) => "HUNTER'S BOLT",
        (WeaponType::Crossbow, GearTier::Rare) => "GRAVEWIND BOLT",
        (WeaponType::Crossbow, GearTier::Epic) => "ARMORBREAKER SHOT",
        (WeaponType::Spellbook, GearTier::Common) => "FOCUSED CAST",
        (WeaponType::Spellbook, GearTier::Rare) => "SCORCHING CAST",
        (WeaponType::Spellbook, GearTier::Epic) => "ECHOING CAST",
        (WeaponType::Daggers, GearTier::Common) => "RAPID FLURRY",
        (WeaponType::Daggers, GearTier::Rare) => "SERRATED RUSH",
        (WeaponType::Daggers, GearTier::Epic) => "PHANTOM STEP",
        (WeaponType::Crossbow, GearTier::Legendary) => match weapon.metadata.definition_id {
            SAINTS_NAIL_ID => "CONSECRATED BOLT",
            RUINPIERCER_ID => "CORRUPTION-PIERCING BOLT",
            BELLRINGER_ID => "RESONANT BOLT",
            _ => "LEGENDARY",
        },
        (WeaponType::Spellbook, GearTier::Legendary) => match weapon.metadata.definition_id {
            HOLLOW_GOSPEL_ID => "RUIN BOLT",
            SAINTS_TESTAMENT_ID => "GRACE BOLT",
            LIVING_CODEX_ID => "ADAPTIVE CAST",
            _ => "LEGENDARY",
        },
        (WeaponType::Daggers, GearTier::Legendary) => match weapon.metadata.definition_id {
            KINGSLAYER_ID => "DEATHMARK",
            WIDOWS_KISS_ID => "VENOMOUS BARRAGE",
            DANCER_IN_THE_DARK_ID => "SHADOWSTEP DANCE",
            _ => "LEGENDARY",
        },
        (_, GearTier::Legendary) => "LEGENDARY",
    }
}

// =========================================================
// CLASS RESOURCE BAR
// =========================================================

fn spawn_class_resource_bar(commands: &mut Commands) {
    commands
        .spawn((
            ClassResourceBar,
            // Hidden until a class with a `ClassResourceHud` (e.g.
            // Runesage's Runes) is actually assigned to the player --
            // see `update_class_resource_bar`.
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                left: percent(50.0),
                bottom: px(96.0),
                width: px(220.0),
                margin: UiRect {
                    left: px(-110.0),
                    ..default()
                },
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(3.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                ClassResourceLabel,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.85, 0.95)),
            ));

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(4.0),
                    ..default()
                })
                .with_children(|pips| {
                    for index in 0..CLASS_RESOURCE_MAX_PIPS {
                        pips.spawn((
                            ClassResourcePip(index),
                            Node {
                                width: px(18.0),
                                height: px(18.0),
                                border: UiRect::all(px(2.0)),
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.25, 0.65, 0.85)),
                            BackgroundColor(Color::srgba(0.035, 0.075, 0.11, 0.92)),
                        ));
                    }
                });
        });
}

/// Shows/hides the whole bar depending on whether the player currently
/// has a `ClassResourceHud` (only Runesage does today), fills in the
/// label text, and lights up pips up to `current`, hiding any pip past
/// `max`.
fn update_class_resource_bar(
    // The character this machine's camera follows: the local `Player`
    // reads its own `ClassResourceHud`, a joined player the host's snapshot.
    player: Query<
        (
            Has<Player>,
            Option<&ClassResourceHud>,
            Option<&PlayerHudSnapshot>,
        ),
        With<CameraTarget>,
    >,
    mut bar_query: Query<&mut Visibility, With<ClassResourceBar>>,
    mut label_query: Query<&mut Text, With<ClassResourceLabel>>,
    mut pip_query: Query<
        (&ClassResourcePip, &mut BackgroundColor, &mut Visibility),
        Without<ClassResourceBar>,
    >,
) {
    let Ok((is_local_player, own_resource, snapshot)) = player.single() else {
        return;
    };

    let Ok(mut bar_visibility) = bar_query.single_mut() else {
        return;
    };

    // (label, current, max)
    let resource = if is_local_player {
        own_resource.map(|resource| (resource.label, resource.current, resource.max))
    } else {
        snapshot
            .and_then(|snapshot| snapshot.class_resource.as_ref())
            .map(|resource| (resource.label.as_str(), resource.current, resource.max))
    };

    let Some((resource_label, current, max)) = resource else {
        *bar_visibility = Visibility::Hidden;
        return;
    };

    *bar_visibility = Visibility::Inherited;

    if let Ok(mut label) = label_query.single_mut() {
        **label = format!("{} {} / {}", resource_label, current, max);
    }

    for (pip, mut background, mut pip_visibility) in &mut pip_query {
        if pip.0 >= max as usize {
            *pip_visibility = Visibility::Hidden;
            continue;
        }

        *pip_visibility = Visibility::Inherited;
        background.0 = if pip.0 < current as usize {
            Color::srgba(0.35, 0.85, 1.0, 0.95)
        } else {
            Color::srgba(0.035, 0.075, 0.11, 0.92)
        };
    }
}

// =========================================================
// PLAYER HEALTH BAR
// =========================================================

fn spawn_player_health_bar(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(20.0),
                top: px(20.0),
                width: px(300.0),
                height: px(28.0),
                padding: UiRect::all(px(3.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
        ))
        .with_children(|parent| {
            parent.spawn((
                PlayerHealthBar,
                Node {
                    width: percent(100.0),
                    height: percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.75, 0.08, 0.08)),
            ));

            // Spawned after the red fill so it paints on top of it. Sized
            // and positioned entirely by `update_player_barrier_overlay`;
            // starts at zero width so no barrier means no visible strip.
            parent.spawn((
                PlayerBarrierOverlay,
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(0.0),
                    top: px(0.0),
                    bottom: px(0.0),
                    width: percent(0.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.35, 0.85, 1.0, 0.6)),
            ));
        });
}

fn update_player_health_bar(
    // The character this machine's camera follows (see `update_ability_bar`).
    player_query: Query<&Health, With<CameraTarget>>,

    mut bar_query: Query<&mut Node, With<PlayerHealthBar>>,
) {
    let Ok(health) = player_query.single() else {
        return;
    };

    let Ok(mut bar) = bar_query.single_mut() else {
        return;
    };

    let health_percent = if health.max > 0.0 {
        (health.current / health.max).clamp(0.0, 1.0)
    } else {
        0.0
    };

    bar.width = percent(health_percent * 100.0);
}

/// Draws `combat::Barriers` as a translucent strip covering whatever
/// portion of the health bar the red fill doesn't already occupy. A
/// barrier only ever shows up to the bar's right edge -- it protects the
/// health the player already has rather than growing the bar past 100%,
/// which keeps this in sync with `apply_player_damage` absorbing from the
/// barrier before it ever touches `Health.current`.
fn update_player_barrier_overlay(
    // The character this machine's camera follows; a joined player's own
    // character has no `Barriers` here, only its replicated snapshot.
    player_query: Query<
        (&Health, Option<&Barriers>, Option<&PlayerHudSnapshot>),
        With<CameraTarget>,
    >,

    mut overlay_query: Query<&mut Node, With<PlayerBarrierOverlay>>,
) {
    let Ok((health, barriers, snapshot)) = player_query.single() else {
        return;
    };
    let barrier = match (barriers, snapshot) {
        (Some(barriers), _) => barriers.total_amount(),
        (None, Some(snapshot)) => snapshot.barrier,
        (None, None) => 0.0,
    };

    let Ok(mut overlay) = overlay_query.single_mut() else {
        return;
    };

    if health.max <= 0.0 {
        overlay.width = percent(0.0);
        return;
    }

    let health_percent = (health.current / health.max).clamp(0.0, 1.0);
    let remaining_percent = (1.0 - health_percent).max(0.0);
    let barrier_percent = (barrier / health.max).clamp(0.0, remaining_percent);

    overlay.left = percent(health_percent * 100.0);
    overlay.width = percent(barrier_percent * 100.0);
}

// =========================================================
// CROSSHAIR
// =========================================================

fn spawn_crosshair(commands: &mut Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,

            left: percent(50.0),
            top: percent(50.0),

            width: px(20.0),
            height: px(20.0),

            margin: UiRect {
                left: px(-10.0),
                top: px(-10.0),
                ..default()
            },

            ..default()
        })
        .with_children(|parent| {
            // Horizontal line
            parent.spawn((
                Crosshair,
                Node {
                    position_type: PositionType::Absolute,

                    left: px(2.0),
                    top: px(9.0),

                    width: px(16.0),
                    height: px(2.0),

                    ..default()
                },
                BackgroundColor(Color::WHITE),
            ));

            // Vertical line
            parent.spawn((
                Crosshair,
                Node {
                    position_type: PositionType::Absolute,

                    left: px(9.0),
                    top: px(2.0),

                    width: px(2.0),
                    height: px(16.0),

                    ..default()
                },
                BackgroundColor(Color::WHITE),
            ));
        });
}

// =========================================================
// RUIN WARNING BANNER
// =========================================================

fn spawn_ruin_warning_banner(commands: &mut Commands) {
    commands.spawn((
        RuinWarningBanner,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::srgba(0.95, 0.55, 0.35, 0.0)),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            left: percent(50.0),
            top: px(64.0),
            width: px(440.0),
            margin: UiRect {
                left: px(-220.0),
                ..default()
            },
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
}

fn spawn_life_status_banner(commands: &mut Commands) {
    commands.spawn((
        LifeStatusBanner,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(26.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.35, 0.25)),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            left: percent(50.0),
            top: percent(30.0),
            width: px(760.0),
            margin: UiRect {
                left: px(-380.0),
                ..default()
            },
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
}

/// The banner's text: a party wipe, else your own Downed/Lost state, else
/// teammates who are down (with their bleed-out and revive progress).
/// Reads every `LifeState` this machine has (replicated in multiplayer).
fn life_status_text(own: Option<&LifeState>, teammates: &[LifeState]) -> String {
    let anyone_standing =
        own.is_some_and(LifeState::is_alive) || teammates.iter().any(LifeState::is_alive);
    if own.is_some() && !anyone_standing {
        return "THE EXPEDITION HAS FALLEN\nReturning to run setup...".to_string();
    }

    match own {
        Some(LifeState::Downed {
            bleed_out_remaining,
            revive_progress,
        }) => {
            let mut text = format!(
                "DOWNED\nA teammate can revive you ({:.0}s)",
                bleed_out_remaining.max(0.0)
            );
            if *revive_progress > 0.0 {
                text.push_str(&format!(
                    "\nBeing revived... {:.0}%",
                    revive_percent(*revive_progress)
                ));
            }
            return text;
        }
        Some(LifeState::Lost) => {
            return "PLAYER LOST TO THE RUINS\nYou return if a teammate reaches the next level"
                .to_string();
        }
        _ => {}
    }

    let mut lines = Vec::new();
    for teammate in teammates {
        if let LifeState::Downed {
            bleed_out_remaining,
            revive_progress,
        } = teammate
        {
            if *revive_progress > 0.0 {
                lines.push(format!(
                    "Reviving teammate... {:.0}%",
                    revive_percent(*revive_progress)
                ));
            } else {
                lines.push(format!(
                    "A teammate is DOWNED - stand beside them to revive ({:.0}s)",
                    bleed_out_remaining.max(0.0)
                ));
            }
        }
    }
    lines.join("\n")
}

fn revive_percent(progress: f32) -> f32 {
    (progress / crate::player::REVIVE_SECONDS * 100.0).clamp(0.0, 100.0)
}

fn update_life_status_banner(
    own: Query<&LifeState, With<CameraTarget>>,
    teammates: Query<&LifeState, Without<CameraTarget>>,
    mut banner: Query<&mut Text, With<LifeStatusBanner>>,
) {
    let Ok(mut text) = banner.single_mut() else {
        return;
    };

    let teammates: Vec<LifeState> = teammates.iter().copied().collect();
    let label = life_status_text(own.single().ok(), &teammates);
    if **text != label {
        **text = label;
    }
}

/// Shows `ruin::RuinState::warning_message` (currently just "The Ruin is
/// spreading...", fired by passive growth -- see
/// `ruin::advance_run_clock_and_ruin`) for as long as it has something to
/// say, and hides it again the instant it doesn't. `RuinState` owns the
/// countdown; this system only ever reflects it.
fn update_ruin_warning_banner(
    ruin_state: Res<RuinState>,
    mut banner_query: Query<(&mut Text, &mut TextColor), With<RuinWarningBanner>>,
) {
    let Ok((mut text, mut color)) = banner_query.single_mut() else {
        return;
    };

    match ruin_state.warning_message() {
        Some(message) => {
            **text = message.to_string();
            color.0 = Color::srgba(0.95, 0.55, 0.35, 0.95);
        }
        None => {
            color.0 = Color::srgba(0.95, 0.55, 0.35, 0.0);
        }
    }
}

// =========================================================
// WEAPON INDICATOR
// =========================================================

fn spawn_weapon_indicator(commands: &mut Commands) {
    commands.spawn((
        WeaponIndicator,
        Text::new("Weapons loading..."),
        TextFont {
            font_size: FontSize::Px(24.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: px(20.0),
            bottom: px(20.0),
            ..default()
        },
    ));
}

fn weapon_type_name(weapon_type: WeaponType) -> &'static str {
    match weapon_type {
        WeaponType::Longsword => "Longsword",
        WeaponType::Greatsword => "Greatsword",
        WeaponType::Bow => "Bow",
        WeaponType::Hammer => "Hammer",
        WeaponType::Crossbow => "Crossbow",
        WeaponType::Spellbook => "Spellbook",
        WeaponType::Daggers => "Daggers",
    }
}

fn update_weapon_indicator(
    player_query: Query<(&Weapon, &Equipment, &PlayerInventory), With<Player>>,
    // A joined player has no local inventory: show their own character's
    // replicated slots instead.
    joined_player: Query<
        (&PlayerHudSnapshot, Option<&PlayerInventorySnapshot>),
        (With<CameraTarget>, Without<Player>),
    >,

    mut indicator_query: Query<&mut Text, With<WeaponIndicator>>,
) {
    let Ok(mut text) = indicator_query.single_mut() else {
        return;
    };

    let Ok((weapon, equipment, inventory)) = player_query.single() else {
        if let Ok((hud, inventory)) = joined_player.single() {
            let label = match inventory {
                Some(inventory) => weapon_indicator_text(
                    inventory.weapon_slots.each_ref().map(|slot| {
                        slot.as_ref().and_then(|item| match item.kind {
                            ItemSnapshotKind::Weapon(weapon_type) => Some((item.tier, weapon_type)),
                            ItemSnapshotKind::Armor(_) => None,
                        })
                    }),
                    inventory.active_weapon_slot as usize,
                    &hud.weapon,
                    inventory.occupied_backpack_slots(),
                ),
                None => format!("Current: {}", hud.weapon),
            };
            if **text != label {
                **text = label;
            }
        }
        return;
    };

    let label = weapon_indicator_text(
        equipment
            .weapon_slots
            .map(|slot| slot.map(|slot_weapon| (slot_weapon.tier, slot_weapon.weapon_type))),
        equipment.active_weapon_slot,
        &format!("{:?} {}", weapon.tier, weapon_type_name(weapon.weapon_type)),
        inventory.occupied_slots(),
    );
    if **text != label {
        **text = label;
    }
}

/// The weapon indicator's text: the three weapon slots (">" marks the
/// active one), the current weapon ("Rare Bow") and how full the backpack is.
fn weapon_indicator_text(
    slots: [Option<(GearTier, WeaponType)>; 3],
    active_slot: usize,
    current: &str,
    backpack_used: usize,
) -> String {
    let weapon_slots: Vec<String> = slots
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let marker = if index == active_slot { ">" } else { " " };
            match slot {
                Some((tier, weapon_type)) => {
                    format!("{}{}: {:?} {:?}", marker, index + 1, tier, weapon_type)
                }
                None => format!("{}{}: Empty", marker, index + 1),
            }
        })
        .collect();

    format!(
        "{}\n{}\n{}\nCurrent: {}\nInventory: {}/10",
        weapon_slots[0], weapon_slots[1], weapon_slots[2], current, backpack_used,
    )
}

// =========================================================
// ENEMY HEALTH BARS
// =========================================================

pub fn spawn_enemy_health_bar(commands: &mut Commands, enemy_entity: Entity) {
    spawn_world_health_bar(commands, enemy_entity, Color::srgb(0.8, 0.08, 0.08));
}

/// A green floating health bar over another player's character
/// (multiplayer, `net::vitals`). Same bar as an enemy's, so it is
/// positioned, resized and cleaned up by the same systems.
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
pub fn spawn_teammate_health_bar(commands: &mut Commands, player_entity: Entity) {
    spawn_world_health_bar(commands, player_entity, Color::srgb(0.18, 0.75, 0.25));
}

/// A screen-space bar that follows `target` and shows its `Health`.
fn spawn_world_health_bar(commands: &mut Commands, target: Entity, fill_color: Color) {
    commands
        .spawn((
            EnemyHealthBar { enemy: target },
            Node {
                position_type: PositionType::Absolute,

                width: px(90.0),
                height: px(10.0),

                padding: UiRect::all(px(1.0)),

                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
        ))
        .with_children(|parent| {
            parent.spawn((
                EnemyHealthFill { enemy: target },
                Node {
                    width: percent(100.0),
                    height: percent(100.0),
                    ..default()
                },
                BackgroundColor(fill_color),
            ));
        });
}

fn update_enemy_health_bars(
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,

    // Not filtered to `Enemy`: each bar names its own target, and in
    // multiplayer a client's copy of a host enemy has `Health` but
    // deliberately no `Enemy` marker (see `net::enemies`).
    enemy_query: Query<(&GlobalTransform, &Health)>,

    mut bar_query: Query<(&EnemyHealthBar, &mut Node), Without<EnemyHealthFill>>,

    mut fill_query: Query<(&EnemyHealthFill, &mut Node), Without<EnemyHealthBar>>,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    // Position bars above enemies.
    for (bar, mut node) in &mut bar_query {
        let Ok((enemy_transform, _health)) = enemy_query.get(bar.enemy) else {
            continue;
        };

        let world_position = enemy_transform.translation() + Vec3::Y * 1.6;

        let Ok(screen_position) = camera.world_to_viewport(camera_transform, world_position) else {
            continue;
        };

        node.left = px(screen_position.x - 45.0);

        node.top = px(screen_position.y - 5.0);
    }

    // Resize health fill.
    for (fill, mut node) in &mut fill_query {
        let Ok((_transform, health)) = enemy_query.get(fill.enemy) else {
            continue;
        };

        let health_percent = if health.max > 0.0 {
            (health.current / health.max).clamp(0.0, 1.0)
        } else {
            0.0
        };

        node.width = percent(health_percent * 100.0);
    }
}

// =========================================================
// CLEANUP
// =========================================================

fn cleanup_enemy_health_bars(
    mut commands: Commands,

    // See `update_enemy_health_bars` for why this isn't `With<Enemy>`.
    enemy_query: Query<(), With<Health>>,

    bar_query: Query<(Entity, &EnemyHealthBar)>,
) {
    for (bar_entity, bar) in &bar_query {
        // If the enemy no longer exists,
        // remove its health bar.
        if enemy_query.get(bar.enemy).is_err() {
            commands.entity(bar_entity).despawn();
        }
    }
}

#[cfg(test)]
mod life_status_tests {
    use super::*;

    fn downed(bleed: f32, revive: f32) -> LifeState {
        LifeState::Downed {
            bleed_out_remaining: bleed,
            revive_progress: revive,
        }
    }

    #[test]
    fn nothing_to_say_while_everyone_stands() {
        assert_eq!(
            life_status_text(Some(&LifeState::Alive), &[LifeState::Alive]),
            ""
        );
    }

    #[test]
    fn solo_downed_is_a_party_wipe() {
        assert_eq!(
            life_status_text(Some(&downed(20.0, 0.0)), &[]),
            "THE EXPEDITION HAS FALLEN\nReturning to run setup..."
        );
    }

    #[test]
    fn downed_with_a_standing_teammate_waits_for_a_revive() {
        let text = life_status_text(Some(&downed(20.0, 0.0)), &[LifeState::Alive]);
        assert!(text.starts_with("DOWNED"));
    }

    #[test]
    fn a_downed_teammate_asks_for_a_revive() {
        let text = life_status_text(Some(&LifeState::Alive), &[downed(12.0, 0.0)]);
        assert!(text.contains("stand beside them"));
        let text = life_status_text(Some(&LifeState::Alive), &[downed(12.0, 1.5)]);
        assert!(text.contains("Reviving teammate... 50%"));
    }
}
