use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use super::components::*;
use super::connections::RoomSocket;
use super::geometry::*;

use crate::characters;
use crate::combat::{BaseHealth, Health};
use crate::enemies::{EnemyNavigationGrid, EnemyStepPlatformConfig};
use crate::game_state::AppState;
use crate::loot::{LootTableId, spawn_loot_chest};
use crate::player::{Player, player_character_body};
use crate::ruin::RuinState;
use crate::run_setup::{self, RunConfig, RunModifiers};
use crate::ui;
use crate::weapons::{
    Armor, ArmorSlot, GearTier, PlayerCombat, Weapon, spawn_armor_pickup, spawn_weapon_pickup,
};

// =========================================================
// ROOM DIMENSIONS
// =========================================================

const FLOOR_THICKNESS: f32 = 1.5;

const SPAWN_ISLAND_WIDTH: f32 = 14.0;
const SPAWN_ISLAND_LENGTH: f32 = 13.0;

const BRIDGE_WIDTH: f32 = 8.0;
const BRIDGE_LENGTH: f32 = 42.0;

const COURTYARD_WIDTH: f32 = 42.0;
const COURTYARD_LENGTH: f32 = 44.0;

// Major Z positions.
const SPAWN_Z: f32 = -43.0;
const BRIDGE_Z: f32 = -15.0;
const COURTYARD_Z: f32 = 28.0;
const GATEHOUSE_Z: f32 = 52.0;

/// Full Z depth of the gate pillars (`Gate Left/Right Pillar`, spawned by
/// `spawn_gatehouse` below) -- used only to find their back (far) face,
/// the physical point where Fallen Gate's built geometry ends and a
/// connected room's own geometry should begin with no gap or overlap.
const GATEHOUSE_DEPTH: f32 = 6.0;

/// The socket a connected room joins against (see `rooms::connections`).
/// Sits at the gate pillars' back face, at the courtyard floor's own
/// top-surface height (`FLOOR_THICKNESS / 2.0`), facing further into the
/// level (+Z) -- the direction a player continues traveling as they pass
/// through the arch. Distinct from the "Room Exit" `RoomConnector` marker
/// spawned in `spawn_fallen_gate` below, which is a gameplay trigger point
/// placed a little further out and not necessarily exactly on the
/// physical seam.
pub fn exit_socket() -> RoomSocket {
    RoomSocket::new(
        Vec3::new(
            0.0,
            FLOOR_THICKNESS / 2.0,
            GATEHOUSE_Z + GATEHOUSE_DEPTH / 2.0,
        ),
        Vec3::new(0.0, 0.0, 1.0),
    )
}

#[derive(Component)]
pub(crate) struct CloudDrift {
    direction: Vec3,
    speed: f32,
    start_position: Vec3,
    max_distance: f32,
}

#[derive(Component)]
pub(crate) struct FireFlicker {
    base_intensity: f32,
    amount: f32,
    speed: f32,
    phase: f32,
}

#[derive(Component)]
pub(crate) struct FlameAnimation {
    base_scale: Vec3,
    base_rotation: Quat,
    speed: f32,
    phase: f32,
    sway: f32,
    stretch: f32,
}

// =========================================================
// MAIN ROOM SPAWNER
// =========================================================

pub fn spawn_fallen_gate(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut navigation_grid: ResMut<EnemyNavigationGrid>,
) {
    // Register the full landmark footprint once so every enemy routes around
    // the pedestal instead of discovering small pieces of it on contact.
    navigation_grid.register_obstacle_box(Vec3::new(0.0, 2.0, 27.0), Vec2::new(4.75, 4.75));

    // Route on the wide middle ledge rather than the small upper cap beneath
    // the statue torso. Heights are enemy-root world heights.
    navigation_grid.register_step_platform(
        EnemyStepPlatformConfig::new(
            Vec2::new(0.0, 27.0),
            Vec2::new(4.75, 4.75),
            Vec2::new(4.5, 4.5),
            2.0,
            3.2,
        )
        // The player may stand on either the middle ledge or upper cap; both
        // should keep an enemy in this platform's pursuit state.
        .with_target_region(Vec2::new(4.5, 4.5), 2.82, 4.35),
    );

    // -----------------------------------------------------
    // MATERIALS
    // -----------------------------------------------------

    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.26, 0.25, 0.24),
        perceptual_roughness: 0.95,
        ..default()
    });

    let dark_stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.11, 0.12, 0.14),
        perceptual_roughness: 1.0,
        ..default()
    });

    let light_stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.36, 0.33),
        perceptual_roughness: 0.92,
        ..default()
    });

    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.085, 0.035),
        perceptual_roughness: 0.92,
        ..default()
    });

    let red_banner = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.025, 0.02),
        perceptual_roughness: 0.86,
        ..default()
    });

    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.13, 0.07),
        metallic: 0.45,
        perceptual_roughness: 0.7,
        ..default()
    });

    // -----------------------------------------------------
    // ROOM ROOT
    // -----------------------------------------------------

    let room = commands
        .spawn((
            Name::new("ROOM 01 - The Fallen Gate"),
            Room,
            FallenGate,
            RoomInfo::new("The Fallen Gate", RoomKind::Start),
            // Generous box covering spawn island -> causeway -> courtyard
            // -> gatehouse (roughly Z -49..55, courtyard's 42-wide span
            // being the widest point) -- see `RoomBounds`'s doc comment
            // for why this doesn't need to be exact.
            RoomBounds::new(Vec2::new(0.0, 2.75), Vec2::new(22.0, 53.0)),
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    commands.entity(room).with_children(|parent| {
        // =================================================
        // SPAWN ISLAND
        // =================================================

        spawn_solid_block(
            parent,
            &mut meshes,
            stone.clone(),
            Vec3::new(0.0, 0.0, SPAWN_Z),
            Vec3::new(SPAWN_ISLAND_WIDTH, FLOOR_THICKNESS, SPAWN_ISLAND_LENGTH),
            "Spawn Island",
        );

        spawn_solid_rotated_block(
            parent,
            &mut meshes,
            stone.clone(),
            Vec3::new(-5.5, -0.1, SPAWN_Z),
            Vec3::new(5.0, 1.4, 10.0),
            Vec3::new(0.0, 0.18, 0.04),
            "Spawn Island Edge",
        );

        spawn_solid_rotated_block(
            parent,
            &mut meshes,
            stone.clone(),
            Vec3::new(5.5, -0.1, SPAWN_Z + 0.5),
            Vec3::new(5.0, 1.4, 9.0),
            Vec3::new(0.0, -0.20, -0.04),
            "Spawn Island Edge",
        );

        spawn_solid_rotated_block(
            parent,
            &mut meshes,
            stone.clone(),
            Vec3::new(0.0, -0.1, SPAWN_Z - 5.5),
            Vec3::new(9.0, 1.4, 4.0),
            Vec3::new(0.0, 0.08, 0.03),
            "Spawn Island Rear Edge",
        );

        // Large central underside.
        spawn_block(
            parent,
            &mut meshes,
            dark_stone.clone(),
            Vec3::new(0.0, -4.5, SPAWN_Z),
            Vec3::new(10.5, 8.0, 9.5),
            "Spawn Island Underside",
        );

        // Jagged chunks hanging underneath.
        spawn_rotated_block(
            parent,
            &mut meshes,
            dark_stone.clone(),
            Vec3::new(-3.5, -9.0, SPAWN_Z - 1.0),
            Vec3::new(3.0, 8.0, 3.0),
            Vec3::new(0.1, 0.0, 0.2),
            "Spawn Island Hanging Rock",
        );

        spawn_rotated_block(
            parent,
            &mut meshes,
            dark_stone.clone(),
            Vec3::new(3.0, -10.5, SPAWN_Z + 1.5),
            Vec3::new(2.8, 10.0, 3.2),
            Vec3::new(-0.15, 0.2, -0.2),
            "Spawn Island Hanging Rock",
        );

        // =================================================
        // BROKEN CAUSEWAY
        // =================================================

        spawn_stylized_causeway(
            parent,
            &mut meshes,
            stone.clone(),
            dark_stone.clone(),
            light_stone.clone(),
        );

        // =================================================
        // COURTYARD
        // =================================================

        spawn_stylized_courtyard_platform(
            parent,
            &mut meshes,
            stone.clone(),
            dark_stone.clone(),
            light_stone.clone(),
        );

        // =================================================
        // BROKEN COURTYARD WALLS
        // =================================================

        spawn_courtyard_walls(parent, &mut meshes, dark_stone.clone());

        // =================================================
        // CENTRAL STATUE
        // =================================================

        spawn_broken_statue(parent, &mut meshes, light_stone.clone());

        // =================================================
        // COLLAPSED GUARD POST
        // =================================================

        spawn_guard_post(parent, &mut meshes, dark_stone.clone(), wood.clone());

        // =================================================
        // BREAKABLE CRATES
        // =================================================

        spawn_crates(parent, &mut meshes, wood.clone());

        // =================================================
        // CASTLE DETAILS
        // =================================================

        spawn_castle_details(
            parent,
            &mut meshes,
            dark_stone.clone(),
            metal.clone(),
            red_banner.clone(),
        );

        // =================================================
        // GATEHOUSE
        // =================================================

        spawn_gatehouse(parent, &mut meshes, dark_stone.clone(), red_banner.clone());

        // =================================================
        // GATE ENTRANCE TORCHES
        // =================================================

        spawn_wall_torch(
            parent,
            &mut meshes,
            &mut materials,
            Vec3::new(-5.0, 5.0, GATEHOUSE_Z - 3.3),
        );

        spawn_wall_torch(
            parent,
            &mut meshes,
            &mut materials,
            Vec3::new(5.0, 5.0, GATEHOUSE_Z - 3.3),
        );

        // =================================================
        // BRAZIERS
        // =================================================

        spawn_brazier(
            parent,
            &mut meshes,
            metal.clone(),
            Vec3::new(-13.0, 1.1, 44.5),
        );

        spawn_brazier(
            parent,
            &mut meshes,
            metal.clone(),
            Vec3::new(13.0, 1.1, 44.5),
        );

        // =================================================
        // DISTANT FLOATING RUINS
        // =================================================

        spawn_floating_ruins(parent, &mut meshes, dark_stone.clone());

        spawn_void_atmosphere(parent, &mut meshes, &mut materials);

        // First real reusable loot pickup. Future rooms, enemy drops, and
        // reward chests can call the same helper with any Weapon/GearTier.
        spawn_weapon_pickup(
            parent,
            &mut meshes,
            &mut materials,
            Vec3::new(-15.0, 2.0, 36.0),
            Weapon::greatsword(GearTier::Rare),
        );

        // Test armor pickup for inventory drag-and-drop.
        spawn_armor_pickup(
            parent,
            &mut meshes,
            &mut materials,
            Vec3::new(-12.5, 2.0, 36.0),
            Armor::new(ArmorSlot::Chest, GearTier::Common, 12.0, 18.0),
        );

        // Reusable randomized chest backed by the central loot-table registry.
        spawn_loot_chest(
            parent,
            &mut meshes,
            &mut materials,
            Vec3::new(-9.5, 1.7, 36.0),
            LootTableId::COMMON_CHEST,
            1,
        );

        // =================================================
        // ROOM SYSTEM MARKERS
        // =================================================

        parent.spawn((
            Name::new("Player Spawn"),
            RoomConnector {
                kind: ConnectorKind::Entrance,
            },
            Transform::from_xyz(0.0, 2.0, SPAWN_Z),
            Visibility::default(),
        ));

        parent.spawn((
            Name::new("Room Exit"),
            RoomConnector {
                kind: ConnectorKind::Exit,
            },
            Transform::from_xyz(0.0, 1.0, 60.0),
            Visibility::default(),
        ));

        parent.spawn((
            Name::new("Guard Post Loot Spawn"),
            LootSpawn,
            Transform::from_xyz(-15.0, 1.0, 36.0),
            Visibility::default(),
        ));

        parent.spawn((
            Name::new("Statue Lore Interaction"),
            LoreInteraction,
            Transform::from_xyz(0.0, 1.0, 32.0),
            Visibility::default(),
        ));

        // This does not kill anything yet.
        // It is only a marker for our later void system.
        parent.spawn((
            Name::new("Void Kill Zone"),
            VoidKillZone { kill_y: -25.0 },
            Transform::from_xyz(0.0, -35.0, 5.0),
            Visibility::default(),
        ));
    });
}

// =========================================================
// BRIDGE
// =========================================================

fn spawn_bridge_ruins(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    light_stone: Handle<StandardMaterial>,
) {
    // Loose rubble on bridge.
    spawn_solid_rotated_block(
        parent,
        meshes,
        light_stone.clone(),
        Vec3::new(-2.7, 1.0, -25.0),
        Vec3::new(2.2, 1.3, 2.6),
        Vec3::new(0.1, 0.3, 0.1),
        "Bridge Rubble",
    );

    spawn_solid_rotated_block(
        parent,
        meshes,
        light_stone.clone(),
        Vec3::new(2.6, 0.9, -13.0),
        Vec3::new(1.7, 1.2, 2.1),
        Vec3::new(-0.1, -0.3, 0.15),
        "Bridge Rubble",
    );

    spawn_solid_rotated_block(
        parent,
        meshes,
        light_stone,
        Vec3::new(-2.8, 0.8, -2.0),
        Vec3::new(2.4, 1.0, 1.5),
        Vec3::new(0.2, 0.6, -0.1),
        "Bridge Rubble",
    );
}

// =========================================================
// COURTYARD UNDERSIDE
// =========================================================

fn spawn_courtyard_underside(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
) {
    let chunks = [
        (
            Vec3::new(-12.0, -15.0, 18.0),
            Vec3::new(8.0, 17.0, 8.0),
            Vec3::new(0.12, 0.2, 0.18),
        ),
        (
            Vec3::new(9.0, -17.0, 23.0),
            Vec3::new(7.0, 22.0, 7.0),
            Vec3::new(-0.15, 0.3, -0.12),
        ),
        (
            Vec3::new(-4.0, -20.0, 36.0),
            Vec3::new(6.0, 25.0, 6.0),
            Vec3::new(0.2, -0.1, 0.12),
        ),
        (
            Vec3::new(15.0, -13.0, 38.0),
            Vec3::new(6.0, 14.0, 6.0),
            Vec3::new(-0.1, 0.5, 0.25),
        ),
    ];

    for (position, size, rotation) in chunks {
        spawn_rotated_block(
            parent,
            meshes,
            material.clone(),
            position,
            size,
            rotation,
            "Jagged Courtyard Underside",
        );
    }
}

// =========================================================
// COURTYARD WALLS
// =========================================================

fn spawn_courtyard_walls(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
) {
    // Important:
    // These do NOT form continuous walls.
    // We want plenty of openings so the void stays visible.

    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(-20.0, 3.5, 18.0),
        Vec3::new(2.0, 8.0, 12.0),
        "Broken Courtyard Wall",
    );

    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(-20.0, 4.5, 39.0),
        Vec3::new(2.0, 10.0, 10.0),
        "Broken Courtyard Wall",
    );

    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(20.0, 4.0, 20.0),
        Vec3::new(2.0, 9.0, 13.0),
        "Broken Courtyard Wall",
    );

    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(20.0, 3.0, 41.0),
        Vec3::new(2.0, 7.0, 8.0),
        "Broken Courtyard Wall",
    );

    spawn_ruined_watchtower(
        parent,
        meshes,
        material.clone(),
        Vec3::new(-20.0, 0.0, 11.0),
    );

    spawn_ruined_watchtower(parent, meshes, material.clone(), Vec3::new(20.0, 0.0, 11.0));

    spawn_ruined_watchtower(
        parent,
        meshes,
        material.clone(),
        Vec3::new(-20.0, 0.0, 47.0),
    );

    spawn_ruined_watchtower(parent, meshes, material, Vec3::new(20.0, 0.0, 47.0));
}

fn spawn_ruined_watchtower(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
) {
    // Wide foundation.
    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(position.x, 3.0, position.z),
        Vec3::new(7.0, 6.0, 7.0),
        "Ruined Watchtower Base",
    );

    // Narrow upper section.
    spawn_solid_rotated_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(position.x, 8.0, position.z),
        Vec3::new(5.8, 5.0, 5.8),
        Vec3::new(0.02, 0.06, 0.02),
        "Ruined Watchtower Upper",
    );

    // Broken top pieces.
    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(position.x - 2.0, 11.2, position.z - 2.0),
        Vec3::new(1.4, 2.4, 1.4),
        "Broken Battlement",
    );

    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(position.x + 2.0, 11.0, position.z - 2.0),
        Vec3::new(1.4, 2.0, 1.4),
        "Broken Battlement",
    );

    // One crooked remaining piece.
    spawn_solid_rotated_block(
        parent,
        meshes,
        material,
        Vec3::new(position.x - 2.0, 11.4, position.z + 2.0),
        Vec3::new(1.3, 2.8, 1.3),
        Vec3::new(0.15, 0.05, -0.18),
        "Crooked Battlement",
    );
}

// =========================================================
// STATUE
// =========================================================

fn spawn_broken_statue(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
) {
    // =====================================================
    // PEDESTAL
    // =====================================================

    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(0.0, 0.35, 27.0),
        Vec3::new(9.5, 0.7, 9.5),
        "Statue Pedestal Base",
    );

    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(0.0, 1.2, 27.0),
        // The 9x9 middle tier leaves a 1.3-unit walkable ring around the
        // 6.4x6.4 upper cap. The previous 0.55-unit ring was narrower than an
        // enemy body and could never support valid navigation around it.
        Vec3::new(9.0, 1.5, 9.0),
        "Statue Pedestal",
    );

    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(0.0, 2.0, 27.0),
        Vec3::new(6.4, 0.5, 6.4),
        "Statue Pedestal Top",
    );

    // =====================================================
    // REMAINING LEGS
    // =====================================================

    // Keep the broken-leg silhouettes, but use one simple lower-body collider
    // below instead of two narrow colliders with a tempting gap between them.
    spawn_rotated_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(-1.0, 3.2, 27.0),
        Vec3::new(1.3, 3.2, 1.5),
        Vec3::new(0.0, 0.05, -0.05),
        "Statue Left Leg",
    );

    spawn_rotated_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(1.0, 3.0, 27.2),
        Vec3::new(1.3, 2.8, 1.5),
        Vec3::new(0.0, -0.05, 0.08),
        "Statue Right Leg",
    );

    // The pedestal top remains walkable around this footprint. This invisible
    // blocker begins exactly at its upper surface (y = 2.25), fills the space
    // between and immediately around the legs, and reaches the torso. Keeping
    // this obstacle convex gives both the navmesh and physics sweeps one clear
    // boundary to route around instead of several snag-prone seams.
    parent.spawn((
        Name::new("Statue Lower Body Navigation Blocker"),
        Transform::from_xyz(0.0, 3.55, 27.1),
        RigidBody::Static,
        Collider::cuboid(3.4, 2.6, 2.3),
    ));

    // =====================================================
    // TORSO
    // =====================================================

    spawn_solid_rotated_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(0.0, 6.2, 27.0),
        Vec3::new(4.0, 4.2, 2.5),
        Vec3::new(0.05, 0.12, -0.12),
        "Broken Statue Torso",
    );

    // Shoulder pieces give it a stronger heroic silhouette.
    spawn_solid_rotated_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(-2.3, 7.0, 27.0),
        Vec3::new(2.2, 1.6, 2.2),
        Vec3::new(0.0, 0.1, -0.18),
        "Statue Left Shoulder",
    );

    spawn_solid_rotated_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(2.2, 6.8, 27.1),
        Vec3::new(2.0, 1.5, 2.0),
        Vec3::new(0.0, -0.1, 0.15),
        "Statue Right Shoulder",
    );

    // =====================================================
    // FALLEN HEAD
    // =====================================================

    parent.spawn((
        Name::new("Broken Statue Head"),
        Mesh3d(meshes.add(Sphere::new(1.45))),
        MeshMaterial3d(material.clone()),
        Transform {
            translation: Vec3::new(7.0, 1.55, 22.0),
            rotation: Quat::from_euler(EulerRot::XYZ, 0.35, -0.8, 0.55),
            scale: Vec3::new(0.9, 1.15, 0.85),
        },
    ));

    // Small crown/helmet fragment.
    spawn_rotated_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(7.2, 2.55, 21.9),
        Vec3::new(1.5, 0.65, 1.4),
        Vec3::new(0.1, -0.5, 0.15),
        "Statue Helmet Fragment",
    );

    // =====================================================
    // FALLEN ARM
    // =====================================================

    spawn_rotated_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(-5.5, 1.3, 23.0),
        Vec3::new(1.25, 5.0, 1.25),
        Vec3::new(0.2, 0.6, 1.15),
        "Broken Statue Arm",
    );

    // =====================================================
    // BROKEN SWORD
    // =====================================================

    spawn_rotated_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(-6.2, 1.0, 26.0),
        Vec3::new(0.5, 0.35, 5.5),
        Vec3::new(0.15, 0.65, -0.08),
        "Statue Broken Sword",
    );

    // =====================================================
    // STATUE RUBBLE
    // =====================================================

    let rubble = [
        (
            Vec3::new(-4.2, 0.8, 29.0),
            Vec3::new(1.8, 1.2, 1.6),
            Vec3::new(0.2, 0.6, 0.15),
        ),
        (
            Vec3::new(4.6, 0.7, 29.5),
            Vec3::new(1.4, 1.0, 2.0),
            Vec3::new(-0.15, 0.4, -0.12),
        ),
        (
            Vec3::new(3.8, 0.55, 24.0),
            Vec3::new(1.5, 0.8, 1.3),
            Vec3::new(0.3, -0.5, 0.15),
        ),
        (
            Vec3::new(-3.6, 0.55, 23.7),
            Vec3::new(1.2, 0.8, 1.7),
            Vec3::new(-0.25, 0.7, -0.1),
        ),
    ];

    for (position, size, rotation) in rubble {
        spawn_rotated_block(
            parent,
            meshes,
            material.clone(),
            position,
            size,
            rotation,
            "Statue Rubble",
        );
    }
}

// =========================================================
// GUARD POST
// =========================================================

fn spawn_guard_post(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    stone: Handle<StandardMaterial>,
    wood: Handle<StandardMaterial>,
) {
    // Remaining rear wall.
    spawn_solid_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(-15.0, 2.3, 36.5),
        Vec3::new(8.0, 4.6, 1.2),
        "Guard Post Rear Wall",
    );

    // Broken side wall.
    spawn_solid_rotated_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(-18.0, 1.7, 39.0),
        Vec3::new(1.4, 3.4, 5.5),
        Vec3::new(0.02, -0.08, -0.05),
        "Guard Post Broken Side Wall",
    );

    // One surviving post.
    spawn_solid_rotated_block(
        parent,
        meshes,
        wood.clone(),
        Vec3::new(-12.0, 3.0, 38.0),
        Vec3::new(0.7, 6.0, 0.7),
        Vec3::new(0.02, 0.05, -0.06),
        "Guard Post Standing Beam",
    );

    // Roof beam hanging down.
    spawn_rotated_block(
        parent,
        meshes,
        wood.clone(),
        Vec3::new(-15.0, 4.2, 38.0),
        Vec3::new(0.8, 0.8, 8.0),
        Vec3::new(0.25, 0.45, 0.38),
        "Guard Post Fallen Roof Beam",
    );

    spawn_rotated_block(
        parent,
        meshes,
        wood.clone(),
        Vec3::new(-16.5, 1.3, 33.5),
        Vec3::new(0.7, 0.7, 6.0),
        Vec3::new(-0.15, -0.5, -0.35),
        "Guard Post Fallen Beam",
    );

    // Small ruined roof panels.
    spawn_rotated_block(
        parent,
        meshes,
        wood.clone(),
        Vec3::new(-14.5, 3.2, 39.5),
        Vec3::new(4.5, 0.35, 3.5),
        Vec3::new(0.35, 0.2, 0.2),
        "Collapsed Guard Post Roof",
    );

    // Rubble around base.
    let rubble = [
        Vec3::new(-18.2, 0.6, 35.0),
        Vec3::new(-13.0, 0.55, 34.0),
        Vec3::new(-17.0, 0.55, 40.0),
    ];

    for position in rubble {
        spawn_rotated_block(
            parent,
            meshes,
            stone.clone(),
            position,
            Vec3::new(1.8, 1.0, 1.5),
            Vec3::new(0.2, 0.45, 0.15),
            "Guard Post Rubble",
        );
    }
}

// =========================================================
// CRATES
// =========================================================

fn spawn_crates(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
) {
    let crates = [
        (Vec3::new(14.0, 1.0, 30.0), Vec3::new(2.0, 2.0, 2.0), 0.08),
        (Vec3::new(16.3, 0.8, 32.5), Vec3::new(1.6, 1.6, 1.6), -0.18),
        (Vec3::new(12.0, 0.75, 33.0), Vec3::new(1.5, 1.5, 1.5), 0.24),
        (Vec3::new(17.2, 0.7, 28.0), Vec3::new(1.4, 1.4, 1.4), -0.30),
    ];

    for (position, size, rotation) in crates {
        spawn_stylized_crate(parent, meshes, material.clone(), position, size, rotation);
    }
}

fn spawn_stylized_crate(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
    rotation: f32,
) {
    // Main box.
    parent.spawn((
        Name::new("Breakable Crate"),
        Breakable,
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(material.clone()),
        Transform {
            translation: position,
            rotation: Quat::from_rotation_y(rotation),
            ..default()
        },
    ));

    let slat = 0.12;

    // Front horizontal slats.
    for y in [-0.32, 0.32] {
        spawn_rotated_block(
            parent,
            meshes,
            material.clone(),
            position + Vec3::new(0.0, y * size.y, -size.z * 0.52),
            Vec3::new(size.x * 1.06, slat, slat),
            Vec3::new(0.0, rotation, 0.0),
            "Crate Reinforcement",
        );
    }

    // Vertical side slat.
    spawn_rotated_block(
        parent,
        meshes,
        material,
        position + Vec3::new(-size.x * 0.52, 0.0, 0.0),
        Vec3::new(slat, size.y * 1.05, size.z * 1.02),
        Vec3::new(0.0, rotation, 0.0),
        "Crate Reinforcement",
    );
}

// =========================================================
// GATEHOUSE
// =========================================================

fn spawn_gatehouse(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    stone: Handle<StandardMaterial>,
    banner: Handle<StandardMaterial>,
) {
    // =====================================================
    // LEFT / RIGHT MAIN TOWERS
    // =====================================================

    spawn_stylized_tower(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(-17.0, 0.0, GATEHOUSE_Z),
        "Gatehouse Left Tower",
    );

    spawn_stylized_tower(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(17.0, 0.0, GATEHOUSE_Z),
        "Gatehouse Right Tower",
    );

    // =====================================================
    // CENTRAL GATE PILLARS
    // =====================================================

    spawn_solid_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(-7.0, 7.0, GATEHOUSE_Z),
        Vec3::new(5.0, 14.0, 6.0),
        "Gate Left Pillar",
    );

    spawn_solid_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(7.0, 7.0, GATEHOUSE_Z),
        Vec3::new(5.0, 14.0, 6.0),
        "Gate Right Pillar",
    );

    // =====================================================
    // ANGULAR ARCH
    //
    // These rotated blocks make the entrance feel more
    // stylized than a normal rectangular doorway.
    // =====================================================

    spawn_solid_rotated_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(-4.5, 13.5, GATEHOUSE_Z),
        Vec3::new(5.0, 3.0, 6.2),
        Vec3::new(0.0, 0.0, -0.55),
        "Gate Arch Left",
    );

    spawn_solid_rotated_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(4.5, 13.5, GATEHOUSE_Z),
        Vec3::new(5.0, 3.0, 6.2),
        Vec3::new(0.0, 0.0, 0.55),
        "Gate Arch Right",
    );

    // Keystone.
    spawn_solid_rotated_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(0.0, 16.0, GATEHOUSE_Z),
        Vec3::new(3.5, 4.0, 6.4),
        Vec3::new(0.0, 0.0, 0.78),
        "Gate Arch Keystone",
    );

    // =====================================================
    // UPPER CENTRAL STRUCTURE
    // =====================================================

    spawn_solid_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(0.0, 19.5, GATEHOUSE_Z),
        Vec3::new(17.0, 5.0, 6.0),
        "Gatehouse Upper Wall",
    );

    // =====================================================
    // UPPER BATTLEMENTS
    // =====================================================

    spawn_crenellations(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(0.0, 22.7, GATEHOUSE_Z),
        17.0,
        6,
    );

    // =====================================================
    // BUTTRESSES
    // =====================================================

    spawn_buttress(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(-10.5, 0.0, GATEHOUSE_Z - 2.5),
    );

    spawn_buttress(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(10.5, 0.0, GATEHOUSE_Z - 2.5),
    );

    // =====================================================
    // RED BANNERS
    // =====================================================

    spawn_banner(
        parent,
        meshes,
        banner.clone(),
        Vec3::new(-10.0, 12.0, GATEHOUSE_Z - 3.2),
    );

    spawn_banner(
        parent,
        meshes,
        banner,
        Vec3::new(10.0, 12.0, GATEHOUSE_Z - 3.2),
    );
}

fn spawn_stylized_tower(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    base_position: Vec3,
    name: &'static str,
) {
    // -----------------------------------------------------
    // LOWER WIDE BASE
    // -----------------------------------------------------

    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(base_position.x, 4.0, base_position.z),
        Vec3::new(10.0, 8.0, 10.0),
        name,
    );

    // -----------------------------------------------------
    // MIDDLE SECTION
    //
    // Slightly narrower, producing a tapered silhouette.
    // -----------------------------------------------------

    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(base_position.x, 11.0, base_position.z),
        Vec3::new(8.5, 7.0, 8.5),
        "Tower Middle",
    );

    // -----------------------------------------------------
    // UPPER SECTION
    // -----------------------------------------------------

    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(base_position.x, 17.0, base_position.z),
        Vec3::new(7.5, 6.0, 7.5),
        "Tower Upper",
    );

    // -----------------------------------------------------
    // OVERHANGING TOP
    // -----------------------------------------------------

    spawn_solid_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(base_position.x, 20.5, base_position.z),
        Vec3::new(9.5, 1.4, 9.5),
        "Tower Overhang",
    );

    // -----------------------------------------------------
    // BATTLEMENTS
    // -----------------------------------------------------

    spawn_tower_crenellations(
        parent,
        meshes,
        material,
        Vec3::new(base_position.x, 22.0, base_position.z),
    );
}

// =========================================================
// BRAZIERS
// =========================================================

fn spawn_brazier(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
) {
    parent.spawn((
        Name::new("Brazier"),
        Mesh3d(meshes.add(Cylinder::new(0.75, 0.8))),
        MeshMaterial3d(material),
        Transform::from_translation(position),
    ));

    parent.spawn((
        Name::new("Brazier Light"),
        PointLight {
            color: Color::srgb(1.0, 0.32, 0.06),
            intensity: 55_000.0,
            range: 13.0,
            ..default()
        },
        Transform::from_xyz(position.x, position.y + 1.8, position.z),
        FireFlicker {
            base_intensity: 55_000.0,
            amount: 10_000.0,
            speed: 6.2,
            phase: position.x * 0.41 + position.z * 0.29,
        },
    ));
}

fn spawn_wall_torch(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
) {
    // Wooden torch handle
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.07, 0.025),
        perceptual_roughness: 0.9,
        ..default()
    });

    parent.spawn((
        Name::new("Wall Torch Handle"),
        Mesh3d(meshes.add(Cylinder::new(0.12, 1.8))),
        MeshMaterial3d(wood),
        Transform {
            translation: position,
            rotation: Quat::from_rotation_z(0.25),
            ..default()
        },
    ));

    // Glowing flame material
    let flame_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.22, 0.015),
        emissive: LinearRgba::new(8.0, 1.5, 0.05, 1.0),
        ..default()
    });

    // Simple low-poly flame placeholder
    let flame_position = position + Vec3::new(0.0, 1.05, 0.0);

    // ---------------------------------------------------------
    // OUTER FLAME
    // ---------------------------------------------------------

    let outer_scale = Vec3::new(0.38, 0.80, 0.38);

    let outer_rotation = Quat::from_rotation_z(-0.08);

    parent.spawn((
        Name::new("Torch Flame Outer"),
        Mesh3d(meshes.add(Cone::new(0.5, 1.4))),
        MeshMaterial3d(flame_material.clone()),
        Transform {
            translation: flame_position,
            scale: outer_scale,
            rotation: outer_rotation,
        },
        FlameAnimation {
            base_scale: outer_scale,
            base_rotation: outer_rotation,
            speed: 8.0,
            phase: position.x + position.z * 0.27,
            sway: 0.12,
            stretch: 0.15,
        },
    ));

    // ---------------------------------------------------------
    // MIDDLE FLAME
    // ---------------------------------------------------------

    let middle_scale = Vec3::new(0.26, 0.62, 0.26);

    let middle_rotation = Quat::from_rotation_z(0.07);

    parent.spawn((
        Name::new("Torch Flame Middle"),
        Mesh3d(meshes.add(Cone::new(0.45, 1.3))),
        MeshMaterial3d(flame_material.clone()),
        Transform {
            translation: flame_position + Vec3::new(0.06, 0.12, 0.03),

            scale: middle_scale,

            rotation: middle_rotation,
        },
        FlameAnimation {
            base_scale: middle_scale,
            base_rotation: middle_rotation,
            speed: 9.7,
            phase: position.x * 0.7 + position.z * 0.41 + 1.2,
            sway: 0.16,
            stretch: 0.20,
        },
    ));

    // ---------------------------------------------------------
    // UPPER FLAME TIP
    // ---------------------------------------------------------

    let tip_scale = Vec3::new(0.18, 0.48, 0.18);

    let tip_rotation = Quat::from_rotation_z(-0.12);

    parent.spawn((
        Name::new("Torch Flame Tip"),
        Mesh3d(meshes.add(Cone::new(0.38, 1.1))),
        MeshMaterial3d(flame_material),
        Transform {
            translation: flame_position + Vec3::new(-0.04, 0.42, 0.0),

            scale: tip_scale,

            rotation: tip_rotation,
        },
        FlameAnimation {
            base_scale: tip_scale,
            base_rotation: tip_rotation,
            speed: 11.2,
            phase: position.x * 0.5 + position.z * 0.63 + 2.4,
            sway: 0.20,
            stretch: 0.25,
        },
    ));

    // Actual orange light cast by the torch
    parent.spawn((
        Name::new("Torch Light"),
        PointLight {
            color: Color::srgb(1.0, 0.30, 0.06),
            intensity: 45_000.0,
            range: 14.0,
            ..default()
        },
        Transform::from_translation(position + Vec3::new(0.0, 1.1, -0.5)),
        FireFlicker {
            base_intensity: 45_000.0,
            amount: 8_000.0,
            speed: 7.5,

            // Using position makes each torch naturally get
            // a different phase.
            phase: position.x + position.z * 0.37,
        },
    ));
}

// =========================================================
// FLOATING RUINS
// =========================================================

fn spawn_floating_ruins(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
) {
    let ruins = [
        (
            Vec3::new(-34.0, -3.0, -15.0),
            Vec3::new(7.0, 15.0, 7.0),
            0.20,
        ),
        (Vec3::new(38.0, 5.0, 1.0), Vec3::new(9.0, 18.0, 9.0), -0.15),
        (
            Vec3::new(-48.0, 12.0, 23.0),
            Vec3::new(10.0, 22.0, 10.0),
            0.32,
        ),
        (
            Vec3::new(52.0, -5.0, 35.0),
            Vec3::new(8.0, 16.0, 8.0),
            -0.24,
        ),
        (
            Vec3::new(-30.0, -10.0, 59.0),
            Vec3::new(5.0, 12.0, 5.0),
            0.1,
        ),
        (Vec3::new(34.0, 18.0, 72.0), Vec3::new(8.0, 20.0, 8.0), 0.4),
    ];

    for (position, size, rotation) in ruins {
        spawn_rotated_block(
            parent,
            meshes,
            material.clone(),
            position,
            size,
            Vec3::new(rotation, rotation * 0.5, rotation),
            "Distant Floating Ruin",
        );
    }

    let debris = [
        Vec3::new(-18.0, -5.0, -20.0),
        Vec3::new(17.0, -8.0, -8.0),
        Vec3::new(-29.0, 8.0, 4.0),
        Vec3::new(27.0, -12.0, 20.0),
        Vec3::new(-35.0, -15.0, 37.0),
        Vec3::new(40.0, 11.0, 50.0),
        Vec3::new(-25.0, 15.0, 68.0),
    ];

    for position in debris {
        spawn_rotated_block(
            parent,
            meshes,
            material.clone(),
            position,
            Vec3::new(2.8, 4.0, 2.5),
            Vec3::new(0.4, 0.7, 0.2),
            "Floating Debris",
        );
    }
}

fn spawn_tower_crenellations(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    center: Vec3,
) {
    let offsets = [
        // Front row.
        Vec3::new(-3.5, 0.0, -3.5),
        Vec3::new(0.0, 0.0, -3.5),
        Vec3::new(3.5, 0.0, -3.5),
        // Back row.
        Vec3::new(-3.5, 0.0, 3.5),
        Vec3::new(0.0, 0.0, 3.5),
        Vec3::new(3.5, 0.0, 3.5),
        // Side pieces.
        Vec3::new(-3.5, 0.0, 0.0),
        Vec3::new(3.5, 0.0, 0.0),
    ];

    for offset in offsets {
        spawn_block(
            parent,
            meshes,
            material.clone(),
            center + offset,
            Vec3::new(1.8, 2.5, 1.8),
            "Tower Battlement",
        );
    }
}

fn spawn_crenellations(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    center: Vec3,
    width: f32,
    count: usize,
) {
    if count < 2 {
        return;
    }

    let spacing = width / (count as f32 - 1.0);

    for i in 0..count {
        let x = center.x - width / 2.0 + spacing * i as f32;

        spawn_block(
            parent,
            meshes,
            material.clone(),
            Vec3::new(x, center.y, center.z),
            Vec3::new(1.8, 2.5, 2.5),
            "Gatehouse Battlement",
        );
    }
}

fn spawn_buttress(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
) {
    // Wide bottom.
    spawn_solid_rotated_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(position.x, 3.0, position.z),
        Vec3::new(3.5, 7.0, 3.5),
        Vec3::ZERO,
        "Gatehouse Buttress",
    );

    // Narrow upper section.
    spawn_solid_rotated_block(
        parent,
        meshes,
        material,
        Vec3::new(position.x, 8.0, position.z + 0.5),
        Vec3::new(2.4, 5.0, 2.8),
        Vec3::ZERO,
        "Gatehouse Buttress Upper",
    );
}

fn spawn_banner(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
) {
    // Upper banner.
    spawn_block(
        parent,
        meshes,
        material.clone(),
        position,
        Vec3::new(3.2, 6.0, 0.18),
        "Gatehouse Banner",
    );

    // Lower left torn piece.
    spawn_rotated_block(
        parent,
        meshes,
        material.clone(),
        Vec3::new(position.x - 0.75, position.y - 4.0, position.z),
        Vec3::new(1.4, 3.0, 0.17),
        Vec3::new(0.0, 0.0, -0.12),
        "Torn Banner",
    );

    // Lower right torn piece.
    spawn_rotated_block(
        parent,
        meshes,
        material,
        Vec3::new(position.x + 0.8, position.y - 3.6, position.z),
        Vec3::new(1.1, 2.3, 0.17),
        Vec3::new(0.0, 0.0, 0.18),
        "Torn Banner",
    );
}

fn spawn_stylized_causeway(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    stone: Handle<StandardMaterial>,
    dark_stone: Handle<StandardMaterial>,
    light_stone: Handle<StandardMaterial>,
) {
    // =====================================================
    // CENTRAL WALKABLE SPINE
    //
    // This stays relatively clean so the bridge remains
    // comfortable to traverse.
    // =====================================================

    spawn_solid_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(0.0, 0.0, BRIDGE_Z),
        Vec3::new(6.6, FLOOR_THICKNESS, BRIDGE_LENGTH),
        "Causeway Walkable Core",
    );

    // =====================================================
    // FRACTURED LEFT EDGE
    // =====================================================

    let left_chunks = [
        (
            Vec3::new(-3.7, 0.0, -31.0),
            Vec3::new(2.4, 1.4, 7.5),
            Vec3::new(0.0, 0.10, -0.04),
        ),
        (
            Vec3::new(-3.9, -0.1, -22.0),
            Vec3::new(2.0, 1.5, 8.0),
            Vec3::new(0.03, -0.12, 0.07),
        ),
        (
            Vec3::new(-3.5, 0.0, -12.5),
            Vec3::new(2.6, 1.4, 7.0),
            Vec3::new(-0.02, 0.15, -0.05),
        ),
        (
            Vec3::new(-4.0, -0.15, -3.5),
            Vec3::new(1.8, 1.4, 7.5),
            Vec3::new(0.06, -0.18, 0.09),
        ),
    ];

    for (position, size, rotation) in left_chunks {
        spawn_solid_rotated_block(
            parent,
            meshes,
            stone.clone(),
            position,
            size,
            rotation,
            "Fractured Causeway Edge",
        );
    }

    // =====================================================
    // FRACTURED RIGHT EDGE
    // =====================================================

    let right_chunks = [
        (
            Vec3::new(3.8, -0.05, -29.0),
            Vec3::new(2.0, 1.4, 8.0),
            Vec3::new(0.04, -0.12, -0.06),
        ),
        (
            Vec3::new(3.5, 0.0, -19.0),
            Vec3::new(2.6, 1.4, 7.5),
            Vec3::new(-0.03, 0.18, 0.05),
        ),
        (
            Vec3::new(4.0, -0.1, -9.0),
            Vec3::new(1.8, 1.5, 7.0),
            Vec3::new(0.06, -0.14, -0.08),
        ),
        (
            Vec3::new(3.6, 0.0, 0.0),
            Vec3::new(2.4, 1.4, 7.0),
            Vec3::new(-0.04, 0.12, 0.07),
        ),
    ];

    for (position, size, rotation) in right_chunks {
        spawn_solid_rotated_block(
            parent,
            meshes,
            stone.clone(),
            position,
            size,
            rotation,
            "Fractured Causeway Edge",
        );
    }

    // =====================================================
    // HANGING STONE BENEATH BRIDGE
    // =====================================================

    let underside_chunks = [
        (
            Vec3::new(-2.0, -2.5, -28.0),
            Vec3::new(2.8, 4.0, 5.0),
            Vec3::new(0.08, 0.12, 0.15),
        ),
        (
            Vec3::new(2.3, -3.0, -20.0),
            Vec3::new(3.0, 5.0, 5.5),
            Vec3::new(-0.12, -0.15, -0.12),
        ),
        (
            Vec3::new(-1.8, -2.8, -11.0),
            Vec3::new(2.5, 4.5, 5.0),
            Vec3::new(0.10, -0.1, 0.18),
        ),
        (
            Vec3::new(2.0, -3.2, -2.0),
            Vec3::new(2.8, 5.5, 4.5),
            Vec3::new(-0.15, 0.12, -0.14),
        ),
    ];

    for (position, size, rotation) in underside_chunks {
        spawn_solid_rotated_block(
            parent,
            meshes,
            dark_stone.clone(),
            position,
            size,
            rotation,
            "Causeway Hanging Stone",
        );
    }

    // =====================================================
    // SMALL BROKEN CORNERS
    // =====================================================

    spawn_solid_rotated_block(
        parent,
        meshes,
        light_stone.clone(),
        Vec3::new(-4.3, 0.45, -16.0),
        Vec3::new(1.7, 0.9, 2.2),
        Vec3::new(0.2, 0.5, 0.15),
        "Loose Causeway Stone",
    );

    spawn_solid_rotated_block(
        parent,
        meshes,
        light_stone,
        Vec3::new(4.1, 0.4, -6.0),
        Vec3::new(1.5, 0.8, 2.5),
        Vec3::new(-0.15, -0.4, -0.10),
        "Loose Causeway Stone",
    );

    // Existing rubble and partial railings.
    spawn_bridge_ruins(parent, meshes, stone);
}

fn spawn_stylized_courtyard_platform(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    stone: Handle<StandardMaterial>,
    dark_stone: Handle<StandardMaterial>,
    light_stone: Handle<StandardMaterial>,
) {
    // =====================================================
    // SAFE CENTRAL FLOOR
    // =====================================================

    spawn_solid_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(0.0, 0.0, COURTYARD_Z),
        Vec3::new(34.0, FLOOR_THICKNESS, 37.0),
        "Courtyard Walkable Core",
    );

    // =====================================================
    // CAUSEWAY -> COURTYARD WALKABLE CONNECTION
    // =====================================================

    spawn_solid_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(0.0, 0.0, 7.75),
        Vec3::new(7.5, FLOOR_THICKNESS, 4.5),
        "Causeway Courtyard Connector",
    );

    // =====================================================
    // FRONT EDGE
    // =====================================================

    // Keep these fragments visually broken, but use flat collision where they
    // overlap the causeway connector. Rotated solid colliders created a narrow
    // seam around x=4.2, z=8.5 that could push Landmass agents off the navmesh.
    spawn_rotated_visual_with_flat_collider(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(-11.5, 0.0, 8.5),
        Vec3::new(14.0, 1.5, 6.0),
        Vec3::new(0.0, 0.10, -0.02),
        "Courtyard Front Fragment",
    );

    spawn_rotated_visual_with_flat_collider(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(10.5, -0.05, 8.0),
        Vec3::new(15.0, 1.5, 6.5),
        Vec3::new(0.0, -0.11, 0.03),
        "Courtyard Front Fragment",
    );

    // Keep the center open for the causeway connection.
    spawn_solid_rotated_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(-18.0, 0.0, 13.0),
        Vec3::new(7.0, 1.4, 10.0),
        Vec3::new(0.04, -0.12, 0.02),
        "Courtyard Left Front Corner",
    );

    spawn_solid_rotated_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(18.0, -0.05, 13.5),
        Vec3::new(7.0, 1.5, 11.0),
        Vec3::new(-0.03, 0.16, -0.03),
        "Courtyard Right Front Corner",
    );

    // =====================================================
    // LEFT EDGE
    // =====================================================

    let left_edges = [
        (
            Vec3::new(-19.0, 0.0, 22.0),
            Vec3::new(6.0, 1.5, 12.0),
            Vec3::new(0.03, 0.10, 0.03),
        ),
        (
            Vec3::new(-18.5, -0.1, 34.0),
            Vec3::new(7.0, 1.5, 12.0),
            Vec3::new(-0.03, -0.13, 0.05),
        ),
        (
            Vec3::new(-18.0, 0.0, 45.0),
            Vec3::new(7.0, 1.4, 9.0),
            Vec3::new(0.05, 0.18, -0.04),
        ),
    ];

    for (position, size, rotation) in left_edges {
        spawn_solid_rotated_block(
            parent,
            meshes,
            stone.clone(),
            position,
            size,
            rotation,
            "Fractured Courtyard Edge",
        );
    }

    // =====================================================
    // RIGHT EDGE
    // =====================================================

    let right_edges = [
        (
            Vec3::new(19.0, 0.0, 22.0),
            Vec3::new(6.0, 1.5, 12.0),
            Vec3::new(-0.02, -0.11, -0.04),
        ),
        (
            Vec3::new(18.5, -0.05, 34.5),
            Vec3::new(7.0, 1.5, 12.0),
            Vec3::new(0.04, 0.14, 0.03),
        ),
        (
            Vec3::new(18.0, 0.0, 45.0),
            Vec3::new(7.0, 1.4, 9.0),
            Vec3::new(-0.04, -0.17, 0.05),
        ),
    ];

    for (position, size, rotation) in right_edges {
        spawn_solid_rotated_block(
            parent,
            meshes,
            stone.clone(),
            position,
            size,
            rotation,
            "Fractured Courtyard Edge",
        );
    }

    // =====================================================
    // REAR EDGE NEAR GATEHOUSE
    // =====================================================

    spawn_solid_rotated_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(-11.0, 0.0, 49.0),
        Vec3::new(16.0, 1.5, 7.0),
        Vec3::new(0.0, -0.06, 0.02),
        "Courtyard Rear Fragment",
    );

    spawn_solid_rotated_block(
        parent,
        meshes,
        stone.clone(),
        Vec3::new(11.0, 0.0, 49.0),
        Vec3::new(16.0, 1.5, 7.0),
        Vec3::new(0.0, 0.07, -0.02),
        "Courtyard Rear Fragment",
    );

    // =====================================================
    // LARGE FLOATING FOUNDATION
    // =====================================================

    spawn_block(
        parent,
        meshes,
        dark_stone.clone(),
        Vec3::new(0.0, -6.0, COURTYARD_Z),
        Vec3::new(31.0, 11.0, 32.0),
        "Courtyard Inner Floating Mass",
    );

    // =====================================================
    // MAJOR HANGING ROCK FORMATIONS
    // =====================================================

    spawn_courtyard_underside(parent, meshes, dark_stone.clone());

    // =====================================================
    // EXTRA EDGE ROCKS
    //
    // These make the silhouette much less rectangular when
    // viewed from below or from the approach.
    // =====================================================

    let edge_rocks = [
        (
            Vec3::new(-18.5, -4.0, 16.0),
            Vec3::new(6.0, 8.0, 7.0),
            Vec3::new(0.08, 0.18, 0.15),
        ),
        (
            Vec3::new(18.0, -5.0, 18.0),
            Vec3::new(6.5, 10.0, 7.0),
            Vec3::new(-0.12, -0.20, -0.14),
        ),
        (
            Vec3::new(-19.0, -6.0, 33.0),
            Vec3::new(7.0, 12.0, 8.0),
            Vec3::new(0.15, -0.12, 0.16),
        ),
        (
            Vec3::new(19.5, -4.5, 37.0),
            Vec3::new(6.5, 9.0, 8.0),
            Vec3::new(-0.10, 0.16, -0.12),
        ),
        (
            Vec3::new(-13.0, -5.0, 49.0),
            Vec3::new(8.0, 9.0, 7.0),
            Vec3::new(0.10, 0.08, 0.13),
        ),
        (
            Vec3::new(13.0, -6.0, 49.0),
            Vec3::new(8.0, 11.0, 7.0),
            Vec3::new(-0.13, -0.10, -0.14),
        ),
    ];

    for (position, size, rotation) in edge_rocks {
        spawn_rotated_block(
            parent,
            meshes,
            dark_stone.clone(),
            position,
            size,
            rotation,
            "Courtyard Edge Rock",
        );
    }

    // =====================================================
    // SMALL SURFACE RUBBLE NEAR EDGES
    // =====================================================

    let surface_rubble = [
        Vec3::new(-16.5, 1.0, 18.0),
        Vec3::new(17.0, 0.9, 25.0),
        Vec3::new(-16.0, 0.8, 42.0),
        Vec3::new(15.5, 0.9, 45.0),
    ];

    for position in surface_rubble {
        spawn_rotated_block(
            parent,
            meshes,
            light_stone.clone(),
            position,
            Vec3::new(2.5, 1.5, 2.3),
            Vec3::new(0.2, 0.5, 0.15),
            "Courtyard Edge Rubble",
        );
    }
}

fn spawn_void_atmosphere(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // =====================================================
    // CLOUD MATERIALS
    // =====================================================

    let cloud_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.20, 0.24, 0.31, 0.55),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 1.0,
        unlit: true,
        ..default()
    });

    let deep_cloud_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.08, 0.10, 0.15, 0.65),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 1.0,
        unlit: true,
        ..default()
    });

    // =====================================================
    // LARGE CLOUD MASSES
    //
    // These are intentionally low-poly spheres stretched
    // horizontally.
    // =====================================================

    let clouds = [
        (Vec3::new(-45.0, -30.0, -20.0), Vec3::new(45.0, 8.0, 30.0)),
        (Vec3::new(30.0, -34.0, -5.0), Vec3::new(50.0, 10.0, 35.0)),
        (Vec3::new(-20.0, -32.0, 35.0), Vec3::new(55.0, 9.0, 40.0)),
        (Vec3::new(45.0, -36.0, 55.0), Vec3::new(45.0, 11.0, 35.0)),
        (Vec3::new(-55.0, -38.0, 80.0), Vec3::new(60.0, 12.0, 40.0)),
    ];

    for (index, (position, scale)) in clouds.into_iter().enumerate() {
        let direction = if index % 2 == 0 {
            Vec3::new(1.0, 0.0, 0.25)
        } else {
            Vec3::new(-0.8, 0.0, 0.35)
        };

        parent.spawn((
            Name::new("Void Cloud"),
            Mesh3d(meshes.add(Sphere::new(1.0))),
            MeshMaterial3d(cloud_material.clone()),
            Transform {
                translation: position,
                scale,
                ..default()
            },
            CloudDrift {
                direction,
                speed: 0.45 + index as f32 * 0.06,
                start_position: position,
                max_distance: 45.0,
            },
        ));
    }

    // =====================================================
    // DEEPER CLOUD MASSES
    //
    // Darker clouds below the first layer stop the player
    // from seeing an obvious empty bottom.
    // =====================================================

    let deep_clouds = [
        (Vec3::new(-60.0, -48.0, 10.0), Vec3::new(65.0, 12.0, 50.0)),
        (Vec3::new(55.0, -52.0, 35.0), Vec3::new(70.0, 14.0, 55.0)),
        (Vec3::new(0.0, -55.0, 90.0), Vec3::new(80.0, 15.0, 60.0)),
    ];

    for (index, (position, scale)) in deep_clouds.into_iter().enumerate() {
        parent.spawn((
            Name::new("Deep Void Cloud"),
            Mesh3d(meshes.add(Sphere::new(1.0))),
            MeshMaterial3d(deep_cloud_material.clone()),
            Transform {
                translation: position,
                scale,
                ..default()
            },
            CloudDrift {
                direction: Vec3::new(-1.0, 0.0, 0.15),

                speed: 0.20 + index as f32 * 0.04,

                start_position: position,

                max_distance: 60.0,
            },
        ));
    }

    let wisps = [
        (Vec3::new(-30.0, -12.0, -5.0), Vec3::new(18.0, 2.5, 8.0)),
        (Vec3::new(32.0, -15.0, 18.0), Vec3::new(22.0, 3.0, 9.0)),
        (Vec3::new(-38.0, -18.0, 48.0), Vec3::new(25.0, 3.5, 10.0)),
    ];

    for (index, (position, scale)) in wisps.into_iter().enumerate() {
        parent.spawn((
            Name::new("High Void Mist"),
            Mesh3d(meshes.add(Sphere::new(1.0))),
            MeshMaterial3d(cloud_material.clone()),
            Transform {
                translation: position,
                scale,
                ..default()
            },
            CloudDrift {
                direction: Vec3::new(1.0, 0.02, 0.4),

                speed: 0.65 + index as f32 * 0.10,

                start_position: position,

                max_distance: 35.0,
            },
        ));
    }
}

pub fn animate_clouds(time: Res<Time>, mut clouds: Query<(&mut Transform, &CloudDrift)>) {
    for (mut transform, drift) in &mut clouds {
        transform.translation +=
            drift.direction.normalize_or_zero() * drift.speed * time.delta_secs();

        let distance = transform.translation.distance(drift.start_position);

        if distance > drift.max_distance {
            transform.translation = drift.start_position;
        }
    }
}

pub fn animate_fire_flicker(time: Res<Time>, mut lights: Query<(&mut PointLight, &FireFlicker)>) {
    let elapsed = time.elapsed_secs();

    for (mut light, flicker) in &mut lights {
        // Two overlapping sine waves keep the flicker
        // from looking like a regular pulse.
        let wave_a = (elapsed * flicker.speed + flicker.phase).sin();

        let wave_b = (elapsed * flicker.speed * 2.37 + flicker.phase * 1.7).sin();

        let combined = wave_a * 0.65 + wave_b * 0.35;

        light.intensity = flicker.base_intensity + combined * flicker.amount;
    }
}

pub fn animate_flames(time: Res<Time>, mut flames: Query<(&mut Transform, &FlameAnimation)>) {
    let elapsed = time.elapsed_secs();

    for (mut transform, flame) in &mut flames {
        let main_wave = (elapsed * flame.speed + flame.phase).sin();

        let secondary_wave = (elapsed * flame.speed * 1.83 + flame.phase * 0.73).sin();

        let sway = main_wave * flame.sway + secondary_wave * flame.sway * 0.35;

        let stretch = 1.0 + main_wave * flame.stretch + secondary_wave * flame.stretch * 0.25;

        transform.scale = Vec3::new(
            flame.base_scale.x * (1.0 - main_wave.abs() * 0.08),
            flame.base_scale.y * stretch,
            flame.base_scale.z * (1.0 - secondary_wave.abs() * 0.08),
        );

        transform.rotation =
            flame.base_rotation * Quat::from_euler(EulerRot::XYZ, sway * 0.4, sway * 0.25, -sway);
    }
}

fn spawn_castle_details(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    stone: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
    banner: Handle<StandardMaterial>,
) {
    // =====================================================
    // ABANDONED SHIELDS
    // =====================================================

    let shield_positions = [Vec3::new(-11.5, 0.8, 20.0), Vec3::new(10.5, 0.75, 41.0)];

    for (index, position) in shield_positions.into_iter().enumerate() {
        parent.spawn((
            Name::new("Abandoned Shield"),
            Mesh3d(meshes.add(Cylinder::new(0.8, 0.16))),
            MeshMaterial3d(metal.clone()),
            Transform {
                translation: position,
                rotation: Quat::from_euler(EulerRot::XYZ, 1.45, index as f32 * 0.8, 0.25),
                ..default()
            },
        ));
    }

    // =====================================================
    // BROKEN WEAPON DEBRIS
    // =====================================================

    spawn_rotated_block(
        parent,
        meshes,
        metal.clone(),
        Vec3::new(-8.0, 0.35, 17.0),
        Vec3::new(0.18, 0.18, 3.8),
        Vec3::new(0.05, 0.8, 0.0),
        "Broken Spear",
    );

    spawn_rotated_block(
        parent,
        meshes,
        metal.clone(),
        Vec3::new(9.0, 0.3, 35.0),
        Vec3::new(0.22, 0.16, 2.8),
        Vec3::new(0.15, -0.6, 0.05),
        "Broken Sword",
    );

    // =====================================================
    // FALLEN BANNER
    // =====================================================

    spawn_rotated_block(
        parent,
        meshes,
        banner,
        Vec3::new(-11.5, 0.2, 42.0),
        Vec3::new(3.5, 0.12, 6.0),
        Vec3::new(0.05, 0.5, 0.04),
        "Fallen Banner",
    );

    // =====================================================
    // COURTYARD RUBBLE CLUSTERS
    // =====================================================

    let rubble = [
        (
            Vec3::new(-10.0, 0.6, 14.0),
            Vec3::new(2.5, 1.2, 1.8),
            Vec3::new(0.2, 0.5, 0.15),
        ),
        (
            Vec3::new(13.0, 0.5, 18.0),
            Vec3::new(2.0, 1.0, 2.5),
            Vec3::new(-0.2, -0.4, 0.12),
        ),
        (
            Vec3::new(-7.0, 0.45, 44.0),
            Vec3::new(1.8, 0.9, 2.0),
            Vec3::new(0.18, 0.7, -0.15),
        ),
    ];

    for (position, size, rotation) in rubble {
        spawn_rotated_block(
            parent,
            meshes,
            stone.clone(),
            position,
            size,
            rotation,
            "Courtyard Rubble",
        );
    }
}

// =========================================================
// PLUGIN -- PLAYER / HUD
//
// spawn_fallen_gate above only builds the room's geometry and its own
// enemies/bosses (same as ruined_barracks::spawn_ruined_barracks,
// saint_arena::spawn_saint_arena, test_grounds::spawn_test_grounds) and
// stays callable on its own with no player baggage -- which is what lets
// ruined_barracks.rs spawn right alongside it in RoomsPlugin's own
// OnEnter(AppState::InGame) chain. This plugin mirrors test_grounds.rs's
// TestGroundsPlugin/setup_test_grounds_player_and_scene, spawning the
// player + HUD for Fallen Gate as the active run-start zone instead. The
// camera is not spawned here -- see `camera::CameraPlugin`, which spawns
// it unconditionally at `Startup` so the run-setup screen has a camera
// to render into before a run (and this plugin) exists at all.
//
// Runs on `OnEnter(AppState::InGame)`, not `Startup`: the player's class,
// starting weapon, and starting Ruin all come from the run-setup screen's
// confirmed choices (`run_setup::RunConfig`), which only exist once the
// player has actually pressed "Begin Run" (see `ui::run_setup_screen`).
// =========================================================

pub struct FallenGatePlugin;

impl Plugin for FallenGatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::InGame),
            (
                setup_fallen_gate_scene,
                // Co-op: only the machine that owns the simulation (offline
                // or the host) spawns the gameplay player. A joined client's
                // character is replicated from the host (see `net`).
                setup_fallen_gate_player.run_if(crate::net::has_simulation_authority),
            ),
        );
    }
}

/// Lighting and cursor capture -- needed on every machine that shows the run.
fn setup_fallen_gate_scene(mut commands: Commands, mut cursor_options: Single<&mut CursorOptions>) {
    cursor_options.visible = false;
    cursor_options.grab_mode = CursorGrabMode::Locked;

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.5, 0.0)),
    ));
}

/// The player, their HUD, and the run's starting Ruin.
fn setup_fallen_gate_player(
    mut commands: Commands,
    run_config: Res<RunConfig>,
    run_modifiers: Res<RunModifiers>,
    mut ruin_state: ResMut<RuinState>,
) {
    // Starting Ruin condition (run_setup::catalog::RUIN_CONDITIONS) --
    // the only part of that choice wired into gameplay so far. See that
    // catalog's doc comment for what Tainted/Purified are still missing.
    let ruin_condition = run_setup::catalog::ruin_condition_entry(run_config.ruin_condition);
    ruin_state.add_ruin(ruin_condition.starting_ruin);

    // Starting max health, including any flat bonus from the run's
    // active relics (e.g. Warden's Heart -- see
    // `run_setup::RunModifiers::bonus_max_health`).
    let starting_max_health = 100.0 + run_modifiers.bonus_max_health;

    let player = commands
        .spawn((
            Player,
            BaseHealth(starting_max_health),
            Health::new(starting_max_health),
            PlayerCombat::default(),
            Weapon::of_type(run_config.weapon, GearTier::Common),
            Transform::from_xyz(0.0, 2.0, SPAWN_Z),
            Visibility::default(),
            // Capsule, kinematic body, collision layers, dodge and jump
            // state. Shared with remote players' bodies in multiplayer.
            player_character_body(),
        ))
        .id();
    characters::insert_class(&mut commands, player, run_config.class);

    ui::spawn_hud(&mut commands);
}
