use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::camera::CameraTarget;
use crate::combat::Health;
use crate::player::{Player, PlayerCharacter};
use crate::ui::{MapUiState, RuinScreenState};

use super::components::{
    ArmorSlot, Equipment, INVENTORY_CAPACITY, InventoryItem, PlayerCombat, PlayerInventory,
    WEAPON_SLOT_COUNT, Weapon,
};
use super::loot::spawn_dropped_item;
use super::snapshot::{ARMOR_SLOT_ORDER, ItemSnapshot, ItemSnapshotKind, PlayerInventorySnapshot};

#[derive(Resource, Default)]
pub struct InventoryUiState {
    pub open: bool,
}

#[derive(Resource, Default)]
pub(crate) struct DraggedItem {
    source: Option<ItemLocation>,
    item: Option<ItemSnapshot>,
}

#[derive(Component)]
pub(crate) struct InventoryScreen;

#[derive(Component)]
pub(crate) struct InventoryPanel;

#[derive(Component)]
pub(crate) struct DragGhost;

#[derive(Component)]
pub(crate) struct ItemTooltip;

#[derive(Component)]
pub(crate) struct PlayerStatsSummary;

/// A slot in a player's gear: backpack slot, weapon slot or armor slot.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum ItemLocation {
    Backpack(usize),
    Weapon(usize),
    Armor(ArmorSlot),
}

impl ItemLocation {
    /// False for a slot index that doesn't exist (a joined client's
    /// request is never trusted).
    fn exists(self) -> bool {
        match self {
            ItemLocation::Backpack(index) => index < INVENTORY_CAPACITY,
            ItemLocation::Weapon(index) => index < WEAPON_SLOT_COUNT,
            ItemLocation::Armor(_) => true,
        }
    }
}

/// Where a dragged item is released: onto another slot (moving, equipping
/// or swapping), or outside the inventory panel (dropping it on the floor).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum InventoryTarget {
    Slot(ItemLocation),
    Drop,
}

/// One drag-and-drop in the inventory screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub struct InventoryMove {
    pub from: ItemLocation,
    pub to: InventoryTarget,
}

/// The inventory screen asked to move an item of this machine's player.
/// Offline and on the host it is queued on the local `Player`
/// (`queue_own_inventory_moves`); a joined client sends it to the host
/// (`net::loot`), which queues it on that player's body.
#[derive(Event, Clone, Copy, Debug)]
pub struct InventoryMoveRequested(pub InventoryMove);

/// Inventory moves waiting to be applied to this player character, by
/// `apply_inventory_requests` on the simulating machine.
#[derive(Component, Debug, Default)]
pub struct InventoryRequests(pub Vec<InventoryMove>);

/// Most moves one player may queue per frame; anything past this is a
/// misbehaving client and is ignored.
pub const MAX_QUEUED_INVENTORY_MOVES: usize = 16;

#[derive(Component)]
pub(crate) struct SlotLabel(ItemLocation);

pub(crate) fn spawn_inventory_ui(mut commands: Commands) {
    commands.insert_resource(DraggedItem::default());

    commands
        .spawn((
            InventoryScreen,
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
            ZIndex(50),
        ))
        .with_children(|screen| {
            screen
                .spawn((
                    InventoryPanel,
                    Interaction::None,
                    Node {
                        width: px(940.0),
                        height: px(600.0),
                        padding: UiRect::all(px(24.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(14.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.055, 0.06, 0.075)),
                    BorderColor::all(Color::srgb(0.38, 0.32, 0.20)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("INVENTORY"),
                        TextFont {
                            font_size: FontSize::Px(34.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.82, 0.52)),
                    ));

                    panel.spawn((
                        Text::new(
                            "Drag items to move/equip them. Release outside this panel to drop. Press I to close.",
                        ),
                        TextFont {
                            font_size: FontSize::Px(17.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.72, 0.72, 0.72)),
                    ));

                    spawn_section_title(panel, "WEAPONS");
                    panel
                        .spawn(Node {
                            width: percent(100.0),
                            column_gap: px(12.0),
                            ..default()
                        })
                        .with_children(|row| {
                            for index in 0..WEAPON_SLOT_COUNT {
                                spawn_slot(row, ItemLocation::Weapon(index), px(270.0), px(72.0));
                            }
                        });

                    spawn_section_title(panel, "ARMOR");
                    panel
                        .spawn(Node {
                            width: percent(100.0),
                            column_gap: px(10.0),
                            ..default()
                        })
                        .with_children(|row| {
                            for slot in [
                                ArmorSlot::Head,
                                ArmorSlot::Chest,
                                ArmorSlot::Hands,
                                ArmorSlot::Legs,
                                ArmorSlot::Feet,
                            ] {
                                spawn_slot(row, ItemLocation::Armor(slot), px(164.0), px(66.0));
                            }
                        });

                    panel.spawn((
                        PlayerStatsSummary,
                        Text::new("Health: 100/100   Armor Health: +0   Defense: 0   Reduction: 0%"),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.65, 0.88, 0.72)),
                    ));

                    spawn_section_title(panel, "BACKPACK — 10 SLOTS");
                    panel
                        .spawn(Node {
                            width: percent(100.0),
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: px(10.0),
                            row_gap: px(10.0),
                            ..default()
                        })
                        .with_children(|grid| {
                            for index in 0..INVENTORY_CAPACITY {
                                spawn_slot(
                                    grid,
                                    ItemLocation::Backpack(index),
                                    px(164.0),
                                    px(68.0),
                                );
                            }
                        });
                });

            screen.spawn((
                DragGhost,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(190.0),
                    height: px(42.0),
                    padding: UiRect::all(px(8.0)),
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.16, 0.18, 0.23, 0.94)),
                ZIndex(100),
            ));

            screen.spawn((
                ItemTooltip,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(24.0),
                    top: px(24.0),
                    width: px(380.0),
                    height: px(270.0),
                    padding: UiRect::all(px(16.0)),
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.035, 0.04, 0.055, 0.97)),
                BorderColor::all(Color::srgb(0.5, 0.5, 0.5)),
                ZIndex(90),
            ));
        });
}

fn spawn_section_title(parent: &mut ChildSpawnerCommands, title: &'static str) {
    parent.spawn((
        Text::new(title),
        TextFont {
            font_size: FontSize::Px(19.0),
            ..default()
        },
        TextColor(Color::srgb(0.82, 0.75, 0.62)),
    ));
}

fn spawn_slot(parent: &mut ChildSpawnerCommands, location: ItemLocation, width: Val, height: Val) {
    parent
        .spawn((
            Button,
            location,
            Node {
                width,
                height,
                padding: UiRect::all(px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.105, 0.115, 0.14)),
            BorderColor::all(Color::srgb(0.28, 0.29, 0.33)),
        ))
        .with_children(|slot| {
            slot.spawn((
                SlotLabel(location),
                Text::new("Empty"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.82, 0.82, 0.82)),
            ));
        });
}

/// A run ended (possibly with the inventory open): hide it and any dragged
/// item for the next run.
pub(crate) fn hide_inventory_screen(
    mut screen: Single<&mut Node, With<InventoryScreen>>,
    mut ghost: Single<&mut Node, (With<DragGhost>, Without<InventoryScreen>)>,
) {
    screen.display = Display::None;
    ghost.display = Display::None;
}

pub(crate) fn toggle_inventory(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<InventoryUiState>,
    mut drag: ResMut<DraggedItem>,
    mut cursor_options: Single<&mut CursorOptions>,
    mut screen: Single<&mut Node, With<InventoryScreen>>,
    mut ghost: Single<&mut Node, (With<DragGhost>, Without<InventoryScreen>)>,
) {
    if !keyboard.just_pressed(KeyCode::KeyI) {
        return;
    }

    state.open = !state.open;
    drag.source = None;
    drag.item = None;
    ghost.display = Display::None;

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

/// Closes the inventory the instant the exploration map opens, so the two
/// full-screen overlays never show at once -- mirrors
/// `ui::map::close_map_when_inventory_opens`, which does the reverse.
/// Reacting to `MapUiState` changing (rather than the map's own toggle
/// calling into this module directly) keeps the inventory's own closing
/// logic -- including resetting any in-progress drag -- owned by this
/// module. Doesn't touch the cursor: the map's own toggle already
/// releases it when opening, which is the only reason this system runs.
pub(crate) fn close_inventory_when_map_opens(
    map_state: Res<MapUiState>,
    mut state: ResMut<InventoryUiState>,
    mut drag: ResMut<DraggedItem>,
    mut screen: Single<&mut Node, With<InventoryScreen>>,
    mut ghost: Single<&mut Node, (With<DragGhost>, Without<InventoryScreen>)>,
) {
    if !map_state.is_changed() || !map_state.open || !state.open {
        return;
    }

    state.open = false;
    drag.source = None;
    drag.item = None;
    ghost.display = Display::None;
    screen.display = Display::None;
}

/// Closes the inventory the instant the Ruin screen opens, so no two
/// full-screen overlays ever show at once -- mirrors
/// `close_inventory_when_map_opens` above and
/// `ui::ruin_screen::close_ruin_screen_when_other_ui_opens`, which closes
/// the Ruin screen in return.
pub(crate) fn close_inventory_when_ruin_opens(
    ruin_screen_state: Res<RuinScreenState>,
    mut state: ResMut<InventoryUiState>,
    mut drag: ResMut<DraggedItem>,
    mut screen: Single<&mut Node, With<InventoryScreen>>,
    mut ghost: Single<&mut Node, (With<DragGhost>, Without<InventoryScreen>)>,
) {
    if !ruin_screen_state.is_changed() || !ruin_screen_state.open || !state.open {
        return;
    }

    state.open = false;
    drag.source = None;
    drag.item = None;
    ghost.display = Display::None;
    screen.display = Display::None;
}

pub(crate) fn begin_item_drag(
    state: Res<InventoryUiState>,
    mut drag: ResMut<DraggedItem>,
    slots: Query<(&Interaction, &ItemLocation), Changed<Interaction>>,
    viewer: Query<&PlayerInventorySnapshot, With<CameraTarget>>,
    mut ghost: Single<(&mut Text, &mut Node), With<DragGhost>>,
) {
    if !state.open || drag.item.is_some() {
        return;
    }

    let Ok(snapshot) = viewer.single() else {
        return;
    };

    for (interaction, location) in &slots {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Some(item) = snapshot_item_at(snapshot, *location) else {
            continue;
        };

        drag.source = Some(*location);
        **ghost.0 = item.label.clone();
        drag.item = Some(item.clone());
        ghost.1.display = Display::Flex;
        break;
    }
}

pub(crate) fn update_drag_visual(
    drag: Res<DraggedItem>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut ghost: Single<&mut Node, With<DragGhost>>,
) {
    if drag.item.is_none() {
        return;
    }

    let Some(cursor) = window.cursor_position() else {
        return;
    };

    ghost.left = px(cursor.x + 14.0);
    ghost.top = px(cursor.y + 14.0);
}

/// Turns a finished drag into an `InventoryMoveRequested`. The screen only
/// shows the player's gear (`PlayerInventorySnapshot`); the move itself is
/// made by `apply_inventory_requests` on the simulating machine, which
/// checks it again.
pub(crate) fn finish_item_drag(
    mut commands: Commands,
    state: Res<InventoryUiState>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut drag: ResMut<DraggedItem>,
    slots: Query<(&Interaction, &ItemLocation)>,
    panel: Single<&Interaction, With<InventoryPanel>>,
    mut ghost: Single<&mut Node, With<DragGhost>>,
    viewer: Query<&PlayerInventorySnapshot, With<CameraTarget>>,
) {
    if !state.open || !mouse.just_released(MouseButton::Left) {
        return;
    }

    let (Some(source), Some(dragged_item)) = (drag.source.take(), drag.item.take()) else {
        return;
    };
    ghost.display = Display::None;

    let Ok(snapshot) = viewer.single() else {
        return;
    };

    let target = slots.iter().find_map(|(interaction, location)| {
        matches!(interaction, Interaction::Hovered | Interaction::Pressed).then_some(*location)
    });

    let to = if let Some(target) = target {
        if target == source || !location_accepts_kind(target, dragged_item.kind) {
            return;
        }
        if let Some(target_item) = snapshot_item_at(snapshot, target) {
            if !location_accepts_kind(source, target_item.kind) {
                return;
            }
        }
        InventoryTarget::Slot(target)
    } else if **panel != Interaction::None {
        // A release over the panel but not over a slot cancels the drag.
        return;
    } else {
        InventoryTarget::Drop
    };

    commands.trigger(InventoryMoveRequested(InventoryMove { from: source, to }));
}

/// Offline and on the host: this machine's own moves go on its `Player`.
pub(crate) fn queue_own_inventory_moves(
    requested: On<InventoryMoveRequested>,
    mut players: Query<&mut InventoryRequests, With<Player>>,
) {
    if let Ok(mut requests) = players.single_mut() {
        if requests.0.len() < MAX_QUEUED_INVENTORY_MOVES {
            requests.0.push(requested.0);
        }
    }
}

/// Simulating machine only: applies every player character's queued
/// inventory moves (the local `Player`'s and, on a multiplayer host, each
/// joined player's). A dropped item lands in front of its owner, where
/// anyone can pick it up.
pub(crate) fn apply_inventory_requests(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut players: Query<
        (
            &GlobalTransform,
            &mut InventoryRequests,
            &mut Weapon,
            &mut PlayerCombat,
            &mut Equipment,
            &mut PlayerInventory,
        ),
        With<PlayerCharacter>,
    >,
) {
    for (player_transform, mut requests, mut weapon, mut combat, mut equipment, mut inventory) in
        &mut players
    {
        if requests.0.is_empty() {
            continue;
        }

        for inventory_move in std::mem::take(&mut requests.0) {
            let Some(dropped_item) = apply_inventory_move(
                inventory_move,
                &mut weapon,
                &mut combat,
                &mut equipment,
                &mut inventory,
            ) else {
                continue;
            };

            let forward = player_transform.forward().as_vec3();
            let mut drop_direction = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
            if drop_direction == Vec3::ZERO {
                drop_direction = Vec3::Z;
            }

            spawn_dropped_item(
                &mut commands,
                &mut meshes,
                &mut materials,
                player_transform.translation() + drop_direction * 1.6 + Vec3::Y * 0.7,
                dropped_item,
            );
        }
    }
}

/// Applies one move with the inventory screen's rules. Returns the item if
/// it was dropped on the floor. Anything invalid (an empty or unknown
/// slot, an item that doesn't fit, dropping the last weapon) changes
/// nothing.
fn apply_inventory_move(
    inventory_move: InventoryMove,
    weapon: &mut Weapon,
    combat: &mut PlayerCombat,
    equipment: &mut Equipment,
    inventory: &mut PlayerInventory,
) -> Option<InventoryItem> {
    let source = inventory_move.from;
    if !source.exists() {
        return None;
    }
    let dragged_item = item_at(source, equipment, inventory)?;

    let old_weapons = equipment.weapon_slots;
    let old_armor = equipment.armor;
    let old_inventory = inventory.slots;

    match inventory_move.to {
        InventoryTarget::Slot(target) => {
            if !target.exists() || target == source || !location_accepts(target, dragged_item) {
                return None;
            }

            let target_item = item_at(target, equipment, inventory);
            if let Some(target_item) = target_item {
                if !location_accepts(source, target_item) {
                    return None;
                }
            }

            let dragged_active_weapon = matches!(
                source,
                ItemLocation::Weapon(index) if index == equipment.active_weapon_slot
            );

            set_item(source, target_item, equipment, inventory);
            set_item(target, Some(dragged_item), equipment, inventory);

            if dragged_active_weapon {
                if let ItemLocation::Weapon(target_slot) = target {
                    equipment.active_weapon_slot = target_slot;
                }
            }

            if !sync_active_weapon(weapon, equipment) {
                equipment.weapon_slots = old_weapons;
                equipment.armor = old_armor;
                inventory.slots = old_inventory;
                return None;
            }

            cancel_basic_attack(combat);
            None
        }
        InventoryTarget::Drop => {
            set_item(source, None, equipment, inventory);

            if !sync_active_weapon(weapon, equipment) {
                equipment.weapon_slots = old_weapons;
                equipment.armor = old_armor;
                inventory.slots = old_inventory;
                return None;
            }

            cancel_basic_attack(combat);
            Some(dragged_item)
        }
    }
}

/// Fills the open inventory screen from the `PlayerInventorySnapshot` of
/// the character this machine's camera follows (its own `Player`, or a
/// joined player's own character).
pub(crate) fn refresh_inventory_ui(
    state: Res<InventoryUiState>,
    viewer: Query<(&PlayerInventorySnapshot, Option<&Health>), With<CameraTarget>>,
    mut labels: Query<(&SlotLabel, &mut Text), Without<PlayerStatsSummary>>,
    mut slots: Query<(&ItemLocation, &mut BackgroundColor, &mut BorderColor)>,
    mut stats_summary: Single<&mut Text, (With<PlayerStatsSummary>, Without<SlotLabel>)>,
) {
    if !state.open {
        return;
    }

    let Ok((snapshot, health)) = viewer.single() else {
        return;
    };

    let (current_health, max_health) =
        health.map_or((0.0, 0.0), |health| (health.current, health.max));
    let summary = format!(
        "Health: {:.0}/{:.0}     Armor Health: +{:.0}     Defense: {:.0}     Damage Reduction: {:.1}%",
        current_health,
        max_health,
        snapshot.total_health_bonus,
        snapshot.total_defense,
        snapshot.damage_reduction_percent,
    );
    if ***stats_summary != summary {
        ***stats_summary = summary;
    }

    for (label, mut text) in &mut labels {
        let prefix = location_name(label.0);
        let wanted = match snapshot_item_at(snapshot, label.0) {
            Some(item) => format!("{}\n{}", prefix, item.label),
            None => format!("{}\nEmpty", prefix),
        };
        if **text != wanted {
            **text = wanted;
        }
    }

    let active_weapon_slot = snapshot.active_weapon_slot as usize;
    for (location, mut background, mut border) in &mut slots {
        let rarity = snapshot_item_at(snapshot, *location).map(|item| item.tier);
        let rarity_color = rarity
            .map(rarity_color)
            .unwrap_or(Color::srgb(0.28, 0.29, 0.33));

        background.0 = if matches!(*location, ItemLocation::Weapon(index) if index == active_weapon_slot)
        {
            mix_color(rarity_color, Color::srgb(0.26, 0.20, 0.06), 0.55)
        } else if rarity.is_some() {
            mix_color(Color::srgb(0.075, 0.08, 0.10), rarity_color, 0.22)
        } else {
            Color::srgb(0.105, 0.115, 0.14)
        };
        *border = BorderColor::all(rarity_color);
    }
}

pub(crate) fn update_item_tooltip(
    state: Res<InventoryUiState>,
    viewer: Query<&PlayerInventorySnapshot, With<CameraTarget>>,
    slots: Query<(&Interaction, &ItemLocation)>,
    mut tooltip: Single<(&mut Text, &mut Node, &mut BorderColor), With<ItemTooltip>>,
) {
    if !state.open {
        tooltip.1.display = Display::None;
        return;
    }
    let Ok(snapshot) = viewer.single() else {
        return;
    };

    let hovered_item = slots.iter().find_map(|(interaction, location)| {
        (*interaction == Interaction::Hovered)
            .then(|| snapshot_item_at(snapshot, *location))
            .flatten()
    });

    let Some(item) = hovered_item else {
        tooltip.1.display = Display::None;
        return;
    };

    if **tooltip.0 != item.tooltip {
        **tooltip.0 = item.tooltip.clone();
    }
    *tooltip.2 = BorderColor::all(rarity_color(item.tier));
    tooltip.1.display = Display::Flex;
}

fn snapshot_item_at(
    snapshot: &PlayerInventorySnapshot,
    location: ItemLocation,
) -> Option<&ItemSnapshot> {
    match location {
        ItemLocation::Backpack(index) => snapshot.backpack.get(index)?.as_ref(),
        ItemLocation::Weapon(index) => snapshot.weapon_slots.get(index)?.as_ref(),
        ItemLocation::Armor(slot) => {
            let index = ARMOR_SLOT_ORDER.iter().position(|order| *order == slot)?;
            snapshot.armor[index].as_ref()
        }
    }
}

fn location_accepts_kind(location: ItemLocation, kind: ItemSnapshotKind) -> bool {
    match (location, kind) {
        (ItemLocation::Backpack(_), _) => true,
        (ItemLocation::Weapon(_), ItemSnapshotKind::Weapon(_)) => true,
        (ItemLocation::Armor(slot), ItemSnapshotKind::Armor(armor_slot)) => slot == armor_slot,
        _ => false,
    }
}

fn item_at(
    location: ItemLocation,
    equipment: &Equipment,
    inventory: &PlayerInventory,
) -> Option<InventoryItem> {
    match location {
        ItemLocation::Backpack(index) => inventory.slots[index],
        ItemLocation::Weapon(index) => equipment.weapon_slots[index].map(InventoryItem::Weapon),
        ItemLocation::Armor(slot) => armor_at(equipment, slot).map(InventoryItem::Armor),
    }
}

fn set_item(
    location: ItemLocation,
    item: Option<InventoryItem>,
    equipment: &mut Equipment,
    inventory: &mut PlayerInventory,
) {
    match location {
        ItemLocation::Backpack(index) => inventory.slots[index] = item,
        ItemLocation::Weapon(index) => {
            equipment.weapon_slots[index] = item.and_then(|item| match item {
                InventoryItem::Weapon(weapon) => Some(weapon),
                InventoryItem::Armor(_) => None,
            });
        }
        ItemLocation::Armor(slot) => {
            *equipment.armor.get_mut(slot) = item.and_then(|item| match item {
                InventoryItem::Armor(armor) => Some(armor),
                InventoryItem::Weapon(_) => None,
            });
        }
    }
}

fn location_accepts(location: ItemLocation, item: InventoryItem) -> bool {
    match (location, item) {
        (ItemLocation::Backpack(_), _) => true,
        (ItemLocation::Weapon(_), InventoryItem::Weapon(_)) => true,
        (ItemLocation::Armor(slot), InventoryItem::Armor(armor)) => slot == armor.slot,
        _ => false,
    }
}

fn armor_at(equipment: &Equipment, slot: ArmorSlot) -> Option<super::components::Armor> {
    match slot {
        ArmorSlot::Head => equipment.armor.head,
        ArmorSlot::Chest => equipment.armor.chest,
        ArmorSlot::Hands => equipment.armor.hands,
        ArmorSlot::Legs => equipment.armor.legs,
        ArmorSlot::Feet => equipment.armor.feet,
    }
}

fn sync_active_weapon(weapon: &mut Weapon, equipment: &mut Equipment) -> bool {
    if equipment.weapon_slots[equipment.active_weapon_slot].is_none() {
        let Some(next_slot) = equipment.weapon_slots.iter().position(Option::is_some) else {
            return false;
        };
        equipment.active_weapon_slot = next_slot;
    }

    let Some(active_weapon) = equipment.active_weapon().copied() else {
        return false;
    };
    *weapon = active_weapon;
    true
}

fn cancel_basic_attack(combat: &mut PlayerCombat) {
    combat.attack_timer = 0.0;
    combat.pending_attack = None;
    combat.attack_cooldown = 0.0;
}

pub(super) fn item_label(item: InventoryItem) -> String {
    match item {
        InventoryItem::Weapon(weapon) => format!("{:?}\n{}", weapon.tier, weapon.metadata.name),
        InventoryItem::Armor(armor) => format!("{:?}\n{}", armor.tier, armor.metadata.name),
    }
}

pub(super) fn item_tooltip(item: InventoryItem) -> String {
    match item {
        InventoryItem::Weapon(weapon) => format!(
            "{}\n{:?} {:?}\n\nDefinition: {}\nInstance: #{:06}\n\nDamage: {:.1}     Range: {:.1}\nCooldown: {:.2}s     Attacks/sec: {:.2}\nKnockback: {:.2}     Attack angle: {:.2}\nStagger power: {}     Duration: {:.2}s",
            weapon.metadata.name,
            weapon.tier,
            weapon.weapon_type,
            weapon.metadata.definition_id.0,
            weapon.metadata.instance_id.0,
            weapon.damage,
            weapon.range,
            weapon.cooldown,
            if weapon.cooldown > 0.0 {
                1.0 / weapon.cooldown
            } else {
                0.0
            },
            weapon.knockback,
            weapon.attack_angle,
            weapon.stagger_strength,
            weapon.stagger_duration,
        ),
        InventoryItem::Armor(armor) => {
            // Archetype tag: legacy placeholder armor (Iron/Gatewarden/
            // Ruinforged sets, created via `Armor::new`) has no archetype,
            // so this line is omitted for those pieces.
            let archetype_line = match armor.archetype {
                Some(archetype) => format!("{:?} Gear\n", archetype),
                None => String::new(),
            };

            // Unique effect text (Rare/Epic/Legendary archetype gear).
            // NOTE: this is flavor/rules text only -- see the `GearEffect`
            // doc comment in weapons/components.rs. The proc itself is not
            // wired into gameplay yet.
            let effect_line = if armor.effect.is_present() {
                if armor.effect.name.is_empty() {
                    format!("\n{}", armor.effect.description)
                } else {
                    format!("\n{} -- {}", armor.effect.name, armor.effect.description)
                }
            } else {
                String::new()
            };

            format!(
                "{}\n{}{:?} {:?} Armor\n\nDefinition: {}\nInstance: #{:06}\n\nDefense: +{:.1}     Health: +{:.1}{}\n\nDamage taken multiplier:\n100 / (100 + total defense)",
                armor.metadata.name,
                archetype_line,
                armor.tier,
                armor.slot,
                armor.metadata.definition_id.0,
                armor.metadata.instance_id.0,
                armor.defense,
                armor.health_bonus,
                effect_line,
            )
        }
    }
}

fn rarity_color(tier: super::components::GearTier) -> Color {
    match tier {
        super::components::GearTier::Common => Color::srgb(0.72, 0.72, 0.74),
        super::components::GearTier::Rare => Color::srgb(0.18, 0.48, 1.0),
        super::components::GearTier::Epic => Color::srgb(0.68, 0.22, 0.96),
        super::components::GearTier::Legendary => Color::srgb(1.0, 0.56, 0.08),
    }
}

fn mix_color(a: Color, b: Color, amount: f32) -> Color {
    let a = a.to_srgba();
    let b = b.to_srgba();
    let t = amount.clamp(0.0, 1.0);
    Color::srgba(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        a.alpha + (b.alpha - a.alpha) * t,
    )
}

fn location_name(location: ItemLocation) -> String {
    match location {
        ItemLocation::Backpack(index) => format!("Slot {}", index + 1),
        ItemLocation::Weapon(index) => format!("Weapon {}", index + 1),
        ItemLocation::Armor(slot) => format!("{:?}", slot),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapons::components::{Armor, GearTier, WeaponType};

    fn gear() -> (Weapon, PlayerCombat, Equipment, PlayerInventory) {
        let sword = Weapon::of_type(WeaponType::Longsword, GearTier::Common);
        let mut inventory = PlayerInventory::default();
        inventory
            .add(InventoryItem::Armor(Armor::new(
                ArmorSlot::Chest,
                GearTier::Common,
                12.0,
                18.0,
            )))
            .unwrap();
        inventory
            .add(InventoryItem::Weapon(Weapon::of_type(
                WeaponType::Bow,
                GearTier::Rare,
            )))
            .unwrap();
        (
            sword,
            PlayerCombat::default(),
            Equipment::with_starter_weapon(sword, 0.0),
            inventory,
        )
    }

    fn apply(
        from: ItemLocation,
        to: InventoryTarget,
        gear: &mut (Weapon, PlayerCombat, Equipment, PlayerInventory),
    ) -> Option<InventoryItem> {
        let (weapon, combat, equipment, inventory) = gear;
        apply_inventory_move(
            InventoryMove { from, to },
            weapon,
            combat,
            equipment,
            inventory,
        )
    }

    #[test]
    fn armor_equips_only_into_its_own_slot() {
        let mut gear = gear();
        let wrong = InventoryTarget::Slot(ItemLocation::Armor(ArmorSlot::Head));
        apply(ItemLocation::Backpack(0), wrong, &mut gear);
        assert!(gear.2.armor.head.is_none());
        assert!(gear.3.slots[0].is_some());

        let chest = InventoryTarget::Slot(ItemLocation::Armor(ArmorSlot::Chest));
        apply(ItemLocation::Backpack(0), chest, &mut gear);
        assert_eq!(gear.2.total_defense(), 12.0);
        assert!(gear.3.slots[0].is_none());
    }

    #[test]
    fn swapping_the_active_weapon_changes_the_wielded_weapon() {
        let mut gear = gear();
        let slot_one = InventoryTarget::Slot(ItemLocation::Weapon(0));
        apply(ItemLocation::Backpack(1), slot_one, &mut gear);
        assert_eq!(gear.0.weapon_type, WeaponType::Bow);
        assert!(matches!(
            gear.3.slots[1],
            Some(InventoryItem::Weapon(weapon)) if weapon.weapon_type == WeaponType::Longsword
        ));
    }

    #[test]
    fn the_last_weapon_cannot_be_dropped_and_bad_slots_are_ignored() {
        let mut gear = gear();
        assert!(apply(ItemLocation::Weapon(0), InventoryTarget::Drop, &mut gear).is_none());
        assert!(gear.2.weapon_slots[0].is_some());

        assert!(apply(ItemLocation::Backpack(99), InventoryTarget::Drop, &mut gear).is_none());
        let far = InventoryTarget::Slot(ItemLocation::Weapon(7));
        assert!(apply(ItemLocation::Backpack(1), far, &mut gear).is_none());
        assert!(gear.3.slots[1].is_some());

        assert!(apply(ItemLocation::Backpack(0), InventoryTarget::Drop, &mut gear).is_some());
        assert!(gear.3.slots[0].is_none());
    }
}
