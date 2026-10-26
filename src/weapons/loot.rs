use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::camera::CameraTarget;
use crate::items::ItemIdGenerator;
use crate::player::{InteractionKind, Player, PlayerCharacter, PlayerIntent};

use super::components::{
    Armor, ArmorPickup, Equipment, GearTier, InventoryItem, PlayerCombat, PlayerInventory, Weapon,
    WeaponPickup, WeaponType,
};
use super::snapshot::PlayerInventorySnapshot;

const PICKUP_RANGE: f32 = 2.75;
const PICKUP_BOB_HEIGHT: f32 = 0.18;
const PICKUP_BOB_SPEED: f32 = 2.4;
const PICKUP_ROTATION_SPEED: f32 = 1.2;

#[derive(Component)]
pub(crate) struct InteractionPrompt;

#[derive(Component)]
pub(crate) struct PickupAnimation {
    base_y: f32,
    phase: f32,
}

// Only multiplayer clients build these for replicas.
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
impl PickupAnimation {
    /// Bobs around `base_y` (world units; the entity's own `Transform`
    /// space). Used by multiplayer clients for their copies of the host's
    /// pickups (`net::loot`).
    pub(crate) fn new(base_y: f32, phase: f32) -> Self {
        Self { base_y, phase }
    }

    pub(crate) fn base_y(&self) -> f32 {
        self.base_y
    }
}

/// Something in the world a player can walk up to and press [F] on: a
/// weapon or armor pickup, or a loot chest. It says just enough to draw it
/// and to show the prompt ("[F] Pick up Knight's Longsword", "[F] Open
/// Chest"), so the prompt reads the same thing on every machine: offline
/// and on a multiplayer host it sits on the real `WeaponPickup` /
/// `ArmorPickup` / `loot::LootChest` entity; a joined client only has the
/// host's copy (`net::loot`). Picking up and opening are always decided by
/// the simulating machine.
#[derive(Component, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum WorldLoot {
    Weapon {
        name: String,
        tier: GearTier,
    },
    Armor {
        name: String,
        tier: GearTier,
    },
    /// `repeatable`: a Test Grounds chest that never despawns.
    Chest {
        repeatable: bool,
    },
}

impl WorldLoot {
    pub fn weapon(weapon: &Weapon) -> Self {
        Self::Weapon {
            name: weapon.metadata.name.to_string(),
            tier: weapon.tier,
        }
    }

    pub fn armor(armor: &Armor) -> Self {
        Self::Armor {
            name: armor.metadata.name.to_string(),
            tier: armor.tier,
        }
    }

    fn pickup_name(&self) -> Option<&str> {
        match self {
            Self::Weapon { name, .. } | Self::Armor { name, .. } => Some(name),
            Self::Chest { .. } => None,
        }
    }

    pub fn is_chest(&self) -> bool {
        matches!(self, Self::Chest { .. })
    }

    /// The mesh every machine draws this loot with.
    pub fn mesh(&self) -> Mesh {
        match self {
            Self::Weapon { .. } => Cuboid::new(0.28, 1.15, 0.18).into(),
            Self::Armor { .. } => Cuboid::new(0.75, 0.65, 0.25).into(),
            Self::Chest { .. } => Cuboid::new(1.4, 0.85, 0.95).into(),
        }
    }

    /// The material every machine draws this loot with: glowing tier
    /// colours for pickups; brown wood for a chest (teal for a Test
    /// Grounds chest that never runs out).
    pub fn material(&self) -> StandardMaterial {
        match self {
            Self::Weapon { tier, .. } | Self::Armor { tier, .. } => {
                pickup_material(tier_color(*tier))
            }
            Self::Chest { repeatable } => StandardMaterial {
                base_color: if *repeatable {
                    Color::srgb(0.05, 0.32, 0.34)
                } else {
                    Color::srgb(0.28, 0.12, 0.035)
                },
                metallic: 0.15,
                perceptual_roughness: 0.72,
                ..default()
            },
        }
    }

    /// Weapons lean to one side; everything else stands upright.
    pub fn rotation(&self) -> Quat {
        match self {
            Self::Weapon { .. } => Quat::from_rotation_z(0.35),
            _ => Quat::IDENTITY,
        }
    }

    /// Pickups bob and spin; chests stay put.
    #[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
    pub fn animates(&self) -> bool {
        !self.is_chest()
    }
}

/// A loot message for one player character ("Equipped X in weapon slot
/// 1", "Inventory full ..."). Offline and on the host, the local `Player`'s
/// messages are shown here (`show_own_loot_feedback`); a joined player's
/// are sent to their client (`net::loot`).
#[derive(Event, Clone, Debug)]
pub struct LootFeedbackShown {
    pub character: Entity,
    pub message: String,
}

#[derive(Resource, Default)]
pub(crate) struct LootFeedback {
    message: String,
    time_remaining: f32,
}

impl LootFeedback {
    pub(crate) fn show(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.time_remaining = 2.4;
    }
}

/// Shows loot messages meant for this machine's own `Player`.
pub(crate) fn show_own_loot_feedback(
    shown: On<LootFeedbackShown>,
    players: Query<(), With<Player>>,
    mut feedback: ResMut<LootFeedback>,
) {
    if players.contains(shown.character) {
        feedback.show(shown.message.clone());
    }
}

#[derive(Component)]
pub(crate) struct LootFeedbackText;

#[derive(Clone, Copy)]
enum NearbyPickup {
    Weapon(Entity, WeaponPickup),
    Armor(Entity, ArmorPickup),
}

pub(crate) fn assign_missing_item_ids(
    mut id_generator: ResMut<ItemIdGenerator>,
    mut equipped_weapons: Query<&mut Weapon>,
    mut weapon_pickups: Query<&mut WeaponPickup>,
    mut armor_pickups: Query<&mut ArmorPickup>,
) {
    for mut weapon in &mut equipped_weapons {
        if !weapon.metadata.instance_id.is_assigned() {
            weapon.metadata.instance_id = id_generator.next();
        }
    }
    for mut pickup in &mut weapon_pickups {
        if !pickup.weapon.metadata.instance_id.is_assigned() {
            pickup.weapon.metadata.instance_id = id_generator.next();
        }
    }
    for mut pickup in &mut armor_pickups {
        if !pickup.armor.metadata.instance_id.is_assigned() {
            pickup.armor.metadata.instance_id = id_generator.next();
        }
    }
}

/// Reusable helper that any room, chest, enemy drop, or encounter reward can call.
pub fn spawn_weapon_pickup(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    weapon: Weapon,
) -> Entity {
    let loot = WorldLoot::weapon(&weapon);
    parent
        .spawn((
            Name::new(format!("{:?} {:?} Pickup", weapon.tier, weapon.weapon_type)),
            WeaponPickup { weapon },
            PickupAnimation {
                base_y: position.y,
                phase: position.x * 0.37 + position.z * 0.19,
            },
            Mesh3d(meshes.add(loot.mesh())),
            MeshMaterial3d(materials.add(loot.material())),
            Transform::from_translation(position).with_rotation(loot.rotation()),
            loot,
        ))
        .id()
}

/// Reusable armor-pickup helper for rooms, enemy drops, and rewards.
pub fn spawn_armor_pickup(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    armor: Armor,
) -> Entity {
    let loot = WorldLoot::armor(&armor);
    parent
        .spawn((
            Name::new(format!("{:?} {:?} Armor Pickup", armor.tier, armor.slot)),
            ArmorPickup { armor },
            PickupAnimation {
                base_y: position.y,
                phase: position.x * 0.31 + position.z * 0.17,
            },
            Mesh3d(meshes.add(loot.mesh())),
            MeshMaterial3d(materials.add(loot.material())),
            Transform::from_translation(position).with_rotation(loot.rotation()),
            loot,
        ))
        .id()
}

pub(crate) fn spawn_interaction_prompt(mut commands: Commands) {
    commands.spawn((
        InteractionPrompt,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(24.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: percent(50.0),
            bottom: px(105.0),
            margin: UiRect {
                left: px(-170.0),
                ..default()
            },
            width: px(340.0),
            justify_content: JustifyContent::Center,
            display: Display::None,
            ..default()
        },
    ));
}

pub(crate) fn spawn_loot_feedback(mut commands: Commands) {
    commands.spawn((
        LootFeedbackText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.72, 0.22)),
        Node {
            position_type: PositionType::Absolute,
            left: percent(50.0),
            bottom: px(175.0),
            width: px(460.0),
            margin: UiRect {
                left: px(-230.0),
                ..default()
            },
            justify_content: JustifyContent::Center,
            display: Display::None,
            ..default()
        },
    ));
}

pub(crate) fn update_loot_feedback(
    time: Res<Time>,
    mut feedback: ResMut<LootFeedback>,
    mut text: Single<(&mut Text, &mut Node), With<LootFeedbackText>>,
) {
    feedback.time_remaining = (feedback.time_remaining - time.delta_secs()).max(0.0);
    if feedback.time_remaining <= 0.0 {
        text.1.display = Display::None;
        return;
    }
    **text.0 = feedback.message.clone();
    text.1.display = Display::Flex;
}

/// The "[F] Pick up ..." prompt for the character this machine's camera
/// follows: the local `Player` offline and on the host, a joined player's
/// own character on a client. Reads `WorldLoot`, which every machine has,
/// and the character's free space from its real inventory when this
/// machine simulates it, otherwise from its replicated
/// `PlayerInventorySnapshot`.
pub(crate) fn update_interaction_prompt(
    cursor_options: Single<&CursorOptions>,
    viewer: Query<
        (
            &GlobalTransform,
            Option<&Equipment>,
            Option<&PlayerInventory>,
            Option<&PlayerInventorySnapshot>,
        ),
        With<CameraTarget>,
    >,
    loot: Query<(&GlobalTransform, &WorldLoot)>,
    mut prompt: Query<(&mut Text, &mut Node), With<InteractionPrompt>>,
) {
    let Ok((mut text, mut node)) = prompt.single_mut() else {
        return;
    };
    if cursor_options.grab_mode == CursorGrabMode::None {
        node.display = Display::None;
        return;
    }

    let Ok((viewer_transform, equipment, inventory, snapshot)) = viewer.single() else {
        node.display = Display::None;
        return;
    };

    let viewer_position = viewer_transform.translation();
    let nearest = loot
        .iter()
        .filter(|(_, loot)| !loot.is_chest())
        .map(|(transform, loot)| (viewer_position.distance(transform.translation()), loot))
        .filter(|(distance, _)| *distance <= PICKUP_RANGE)
        .min_by(|(a, _), (b, _)| a.total_cmp(b));

    let Some((_, loot)) = nearest else {
        node.display = Display::None;
        return;
    };

    let (weapon_slot_free, backpack_space) = match (equipment, inventory, snapshot) {
        (Some(equipment), Some(inventory), _) => (
            equipment.first_empty_weapon_slot().is_some(),
            inventory.slots.iter().any(Option::is_none),
        ),
        (_, _, Some(snapshot)) => (
            snapshot.has_free_weapon_slot(),
            snapshot.has_backpack_space(),
        ),
        // Nothing known yet: let the simulating machine decide.
        _ => (true, true),
    };
    let has_space = match loot {
        WorldLoot::Weapon { .. } => weapon_slot_free || backpack_space,
        _ => backpack_space,
    };

    let label = match loot.pickup_name() {
        Some(name) if has_space => format!("[F] Pick up {name}"),
        _ => "Inventory full".to_string(),
    };
    if **text != label {
        **text = label;
    }
    node.display = Display::Flex;
}

/// Picks up the nearest weapon or armor piece for every player character
/// that pressed [F]: the local `Player` and, on a multiplayer host, each
/// joined player's body. Each pickup goes to the player who took it. If two
/// players reach for the same item in one frame, the first one gets it.
pub(crate) fn pickup_nearby_weapon(
    mut commands: Commands,
    weapon_pickups: Query<(Entity, &GlobalTransform, &WeaponPickup)>,
    armor_pickups: Query<(Entity, &GlobalTransform, &ArmorPickup)>,
    mut players: Query<
        (
            Entity,
            &GlobalTransform,
            &mut PlayerIntent,
            &mut Weapon,
            &mut PlayerCombat,
            &mut Equipment,
            &mut PlayerInventory,
        ),
        With<PlayerCharacter>,
    >,
) {
    let mut taken: Vec<Entity> = Vec::new();

    for (
        character,
        player_transform,
        mut intent,
        mut current_weapon,
        mut combat,
        mut equipment,
        mut inventory,
    ) in &mut players
    {
        if !intent.take_interact(InteractionKind::PickUp) {
            continue;
        }

        let Some((_, pickup)) = nearest_pickup(
            player_transform.translation(),
            &weapon_pickups,
            &armor_pickups,
            &taken,
        ) else {
            continue;
        };

        let mut feedback = |message: String| {
            commands.trigger(LootFeedbackShown { character, message });
        };

        let pickup_entity = match pickup {
            NearbyPickup::Weapon(entity, pickup) => {
                if let Some(slot) = equipment.first_empty_weapon_slot() {
                    equipment.weapon_slots[slot] = Some(pickup.weapon);
                    equipment.active_weapon_slot = slot;
                    *current_weapon = pickup.weapon;

                    // Cancel only unfinished basic attacks. Active weapon techniques and the
                    // shared ability cooldown deliberately survive equipment changes.
                    combat.attack_timer = 0.0;
                    combat.pending_attack = None;
                    combat.attack_cooldown = 0.0;

                    println!(
                        "Picked up and equipped slot {}: {:?} {:?}",
                        slot + 1,
                        pickup.weapon.tier,
                        pickup.weapon.weapon_type,
                    );
                    feedback(format!(
                        "Equipped {} in weapon slot {}",
                        pickup.weapon.metadata.name,
                        slot + 1,
                    ));
                } else if let Ok(slot) = inventory.add(InventoryItem::Weapon(pickup.weapon)) {
                    println!(
                        "Stored {:?} {:?} in inventory slot {}",
                        pickup.weapon.tier,
                        pickup.weapon.weapon_type,
                        slot + 1,
                    );
                    feedback(format!(
                        "Stored {} in backpack slot {}",
                        pickup.weapon.metadata.name,
                        slot + 1,
                    ));
                } else {
                    println!("Inventory full");
                    feedback("Inventory full — drop or equip an item first".to_string());
                    continue;
                }
                entity
            }
            NearbyPickup::Armor(entity, pickup) => {
                let Ok(slot) = inventory.add(InventoryItem::Armor(pickup.armor)) else {
                    println!("Inventory full");
                    feedback("Inventory full — drop or equip an item first".to_string());
                    continue;
                };
                println!(
                    "Stored {:?} {:?} Armor in inventory slot {}",
                    pickup.armor.tier,
                    pickup.armor.slot,
                    slot + 1,
                );
                feedback(format!(
                    "Stored {} in backpack slot {}",
                    pickup.armor.metadata.name,
                    slot + 1,
                ));
                entity
            }
        };

        taken.push(pickup_entity);
        commands.entity(pickup_entity).despawn();
    }
}

pub(crate) fn animate_weapon_pickups(
    time: Res<Time>,
    mut pickups: Query<(&mut Transform, &PickupAnimation)>,
) {
    for (mut transform, animation) in &mut pickups {
        transform.translation.y = animation.base_y
            + (time.elapsed_secs() * PICKUP_BOB_SPEED + animation.phase).sin() * PICKUP_BOB_HEIGHT;
        transform.rotate_y(PICKUP_ROTATION_SPEED * time.delta_secs());
    }
}

/// The nearest pickup within reach, skipping any already `taken` this frame.
fn nearest_pickup(
    player_position: Vec3,
    weapon_pickups: &Query<(Entity, &GlobalTransform, &WeaponPickup)>,
    armor_pickups: &Query<(Entity, &GlobalTransform, &ArmorPickup)>,
    taken: &[Entity],
) -> Option<(f32, NearbyPickup)> {
    let weapon = weapon_pickups
        .iter()
        .filter(|(entity, _, _)| !taken.contains(entity))
        .filter_map(|(entity, transform, pickup)| {
            let distance = player_position.distance(transform.translation());
            (distance <= PICKUP_RANGE).then_some((distance, NearbyPickup::Weapon(entity, *pickup)))
        })
        .min_by(|(a, _), (b, _)| a.total_cmp(b));

    let armor = armor_pickups
        .iter()
        .filter(|(entity, _, _)| !taken.contains(entity))
        .filter_map(|(entity, transform, pickup)| {
            let distance = player_position.distance(transform.translation());
            (distance <= PICKUP_RANGE).then_some((distance, NearbyPickup::Armor(entity, *pickup)))
        })
        .min_by(|(a, _), (b, _)| a.total_cmp(b));

    match (weapon, armor) {
        (Some(weapon), Some(armor)) => {
            if weapon.0 <= armor.0 {
                Some(weapon)
            } else {
                Some(armor)
            }
        }
        (Some(weapon), None) => Some(weapon),
        (None, Some(armor)) => Some(armor),
        (None, None) => None,
    }
}

pub(crate) fn spawn_dropped_item(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    item: InventoryItem,
) -> Entity {
    match item {
        InventoryItem::Weapon(weapon) => {
            let loot = WorldLoot::weapon(&weapon);
            commands
                .spawn((
                    Name::new(format!(
                        "Dropped {:?} {:?}",
                        weapon.tier, weapon.weapon_type
                    )),
                    WeaponPickup { weapon },
                    PickupAnimation {
                        base_y: position.y,
                        phase: 0.0,
                    },
                    Mesh3d(meshes.add(loot.mesh())),
                    MeshMaterial3d(materials.add(loot.material())),
                    Transform::from_translation(position).with_rotation(loot.rotation()),
                    loot,
                ))
                .id()
        }
        InventoryItem::Armor(armor) => {
            let loot = WorldLoot::armor(&armor);
            commands
                .spawn((
                    Name::new(format!("Dropped {:?} {:?} Armor", armor.tier, armor.slot)),
                    ArmorPickup { armor },
                    PickupAnimation {
                        base_y: position.y,
                        phase: 0.0,
                    },
                    Mesh3d(meshes.add(loot.mesh())),
                    MeshMaterial3d(materials.add(loot.material())),
                    Transform::from_translation(position).with_rotation(loot.rotation()),
                    loot,
                ))
                .id()
        }
    }
}

fn tier_color(tier: GearTier) -> Color {
    match tier {
        GearTier::Common => Color::srgb(0.75, 0.75, 0.75),
        GearTier::Rare => Color::srgb(0.15, 0.42, 1.0),
        GearTier::Epic => Color::srgb(0.62, 0.18, 0.95),
        GearTier::Legendary => Color::srgb(1.0, 0.55, 0.08),
    }
}

fn pickup_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        emissive: color.to_linear() * 2.0,
        metallic: 0.7,
        perceptual_roughness: 0.25,
        ..default()
    }
}

#[allow(dead_code)]
fn weapon_name(weapon_type: WeaponType) -> &'static str {
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
