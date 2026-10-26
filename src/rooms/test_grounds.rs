//! Test Grounds -- a simple, non-GLB test zone for exercising the loot
//! system end to end (chests covering every `LootTableId` / floor tier)
//! and for spawning normal/elite/miniboss/boss enemies on demand to watch
//! them die and drop loot. This is the current Startup zone, replacing
//! both the Fallen Gate and the Saint of the Broken Siege arena as the
//! game's entry point while gear/loot content is under active test.
//!
//! Fallen Gate (`rooms::fallen_gate`) and the Saint arena
//! (`rooms::saint_arena`, `arena::ArenaPlugin`) are untouched and still
//! compile -- only `main.rs` / `RoomsPlugin` were changed to stop calling
//! them at Startup, exactly like Fallen Gate was already set aside in
//! favor of the Saint arena. Swapping the active zone back is a one-line
//! change in each place, not a rewrite.
//!
//! IMPORTANT -- "miniboss" here is still a *testing convenience*, not a
//! real difficulty system: `debug_spawn_enemy`'s F3 key spawns one of the
//! three normal archetypes (Ruinbound Footman, Gatebow Marksman, Cinder
//! Hound) unchanged, then simply overrides its `Health` and `LootTableId`
//! (and adds a `LootFloor` override -- see `enemies::LootFloor`) so the
//! MINIBOSS loot table, which nothing currently drops in real play, can
//! actually be exercised by killing something. A real miniboss system
//! (distinct archetype, visuals, behavior) is separate future work.
//!
//! F2 ("Elite") is no longer part of that convenience -- it spawns the
//! Black Banner Knight (`enemies::spawn_black_banner_knight`), the
//! project's first real elite archetype, exactly as it will appear in a
//! real encounter.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::characters::runesage::Runesage;
use crate::collision::GameLayer;
use crate::combat::{BaseHealth, Health};
use crate::enemies::{self, EnemyEncounterConfig, LootFloor};
use crate::loot::{LootTableId, spawn_repeatable_loot_chest};
use crate::player::{Dodge, Player, VerticalMovement};
use crate::ui::spawn_enemy_health_bar;
use crate::weapons::{GearTier, PlayerCombat, Weapon};
use crate::{camera, ui};

use super::components::{
    ConnectorKind, Room, RoomBounds, RoomConnector, RoomInfo, RoomKind, TestGrounds, VoidKillZone,
};
use super::geometry::spawn_solid_block;

// =========================================================
// LAYOUT
// =========================================================

/// Full side length of the square floor (a 70x70 open area is plenty for
/// chest rows plus room to fight/chase spawned enemies).
const FLOOR_SIZE: f32 = 70.0;
const WALL_HEIGHT: f32 = 6.0;
const WALL_THICKNESS: f32 = 1.0;
const HALF_FLOOR: f32 = FLOOR_SIZE / 2.0;

/// Player spawns near the back wall; the chest row sits along the opposite
/// wall, leaving the whole middle of the floor open for enemy testing.
const PLAYER_SPAWN: Vec3 = Vec3::new(0.0, 1.3, HALF_FLOOR - 7.0);
const CHEST_ROW_Z: f32 = -(HALF_FLOOR - 7.0);
const CHEST_SPACING: f32 = 3.4;

// =========================================================
// PLUGIN
// =========================================================

pub struct TestGroundsPlugin;

impl Plugin for TestGroundsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TestSpawnCounter>()
            .add_systems(Startup, setup_test_grounds_player_and_scene)
            .add_systems(Update, debug_spawn_enemy);
    }
}

// =========================================================
// ROOM GEOMETRY + CHESTS
// (called from RoomsPlugin's Startup, alongside fallen_gate/saint_arena)
// =========================================================

pub fn spawn_test_grounds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let floor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.29, 0.32),
        perceptual_roughness: 0.92,
        ..default()
    });
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.15, 0.18),
        perceptual_roughness: 0.95,
        ..default()
    });

    let room = commands
        .spawn((
            Name::new("ROOM - Test Grounds"),
            Room,
            TestGrounds,
            RoomInfo::new("Test Grounds", RoomKind::Unknown),
            RoomBounds::new(Vec2::ZERO, Vec2::new(HALF_FLOOR, HALF_FLOOR)),
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    commands.entity(room).with_children(|parent| {
        // -----------------------------------------------------
        // FLOOR + PERIMETER WALLS
        // Plain primitives (no GLB) -- this zone only needs to be
        // functional, not art-directed, per its purpose as a loot/enemy
        // test bed.
        // -----------------------------------------------------
        spawn_solid_block(
            parent,
            &mut meshes,
            floor_material,
            Vec3::new(0.0, -0.5, 0.0),
            Vec3::new(FLOOR_SIZE, 1.0, FLOOR_SIZE),
            "Test Grounds Floor",
        );

        let wall_span = FLOOR_SIZE + WALL_THICKNESS * 2.0;
        let wall_y = WALL_HEIGHT / 2.0;
        let wall_offset = HALF_FLOOR + WALL_THICKNESS / 2.0;

        spawn_solid_block(
            parent,
            &mut meshes,
            wall_material.clone(),
            Vec3::new(0.0, wall_y, -wall_offset),
            Vec3::new(wall_span, WALL_HEIGHT, WALL_THICKNESS),
            "Test Grounds Wall North",
        );
        spawn_solid_block(
            parent,
            &mut meshes,
            wall_material.clone(),
            Vec3::new(0.0, wall_y, wall_offset),
            Vec3::new(wall_span, WALL_HEIGHT, WALL_THICKNESS),
            "Test Grounds Wall South",
        );
        spawn_solid_block(
            parent,
            &mut meshes,
            wall_material.clone(),
            Vec3::new(-wall_offset, wall_y, 0.0),
            Vec3::new(WALL_THICKNESS, WALL_HEIGHT, wall_span),
            "Test Grounds Wall West",
        );
        spawn_solid_block(
            parent,
            &mut meshes,
            wall_material,
            Vec3::new(wall_offset, wall_y, 0.0),
            Vec3::new(WALL_THICKNESS, WALL_HEIGHT, wall_span),
            "Test Grounds Wall East",
        );

        // -----------------------------------------------------
        // LOOT TEST CHESTS
        // One repeatable chest per loot table, several repeated at a
        // higher `floor` value so the Rare/Epic/Legendary floor gates in
        // loot/tables.rs (min_floor 3/5/8) are actually reachable without
        // a real floor-progression system. Repeatable (see
        // loot::spawn_repeatable_loot_chest) so a tester can hold down
        // near one chest and press [F] repeatedly to sample odds instead
        // of burning through one-time chests.
        // -----------------------------------------------------
        const CHEST_SPECS: [(LootTableId, u32); 12] = [
            (LootTableId::COMMON_CHEST, 1),
            (LootTableId::RARE_CHEST, 1),
            (LootTableId::RARE_CHEST, 4),
            (LootTableId::BOSS_CHEST, 1),
            (LootTableId::BOSS_CHEST, 8),
            (LootTableId::NORMAL_ENEMY, 1),
            (LootTableId::ELITE_ENEMY, 1),
            (LootTableId::ELITE_ENEMY, 5),
            (LootTableId::MINIBOSS, 1),
            (LootTableId::MINIBOSS, 5),
            (LootTableId::BOSS, 1),
            (LootTableId::BOSS, 8),
        ];

        let start_x = -((CHEST_SPECS.len() as f32 - 1.0) * CHEST_SPACING) / 2.0;

        for (index, (table_id, floor)) in CHEST_SPECS.into_iter().enumerate() {
            let x = start_x + index as f32 * CHEST_SPACING;
            spawn_repeatable_loot_chest(
                parent,
                &mut meshes,
                &mut materials,
                Vec3::new(x, 0.45, CHEST_ROW_Z),
                table_id,
                floor,
            );
        }

        // -----------------------------------------------------
        // ROOM SYSTEM MARKERS
        // Same pattern as saint_arena.rs: an Entrance connector for the
        // void-reset system (rooms::void::reset_player_from_void) to
        // return the player to, and a kill-height fallback well below the
        // floor (shouldn't ever trigger inside a walled room, but every
        // other room has one and it costs nothing).
        // -----------------------------------------------------
        parent.spawn((
            Name::new("Player Spawn"),
            RoomConnector {
                kind: ConnectorKind::Entrance,
            },
            Transform::from_translation(PLAYER_SPAWN),
            Visibility::default(),
        ));

        parent.spawn((
            Name::new("Test Grounds Void Kill Zone"),
            VoidKillZone { kill_y: -20.0 },
            Transform::from_xyz(0.0, -30.0, 0.0),
            Visibility::default(),
        ));
    });
}

// =========================================================
// PLAYER / CAMERA / HUD
// (mirrors arena::setup_arena's role, for this zone instead of the Saint
// arena -- swapped in from main.rs in place of arena::ArenaPlugin)
// =========================================================

fn setup_test_grounds_player_and_scene(
    mut commands: Commands,
    mut cursor_options: Single<&mut CursorOptions>,
) {
    cursor_options.visible = false;
    cursor_options.grab_mode = CursorGrabMode::Locked;

    commands.spawn((
        Player,
        Runesage,
        BaseHealth(100.0),
        Health::new(100.0),
        Dodge::default(),
        VerticalMovement::default(),
        PlayerCombat::default(),
        Weapon::bow(GearTier::Common),
        Transform::from_translation(PLAYER_SPAWN),
        Visibility::default(),
        RigidBody::Kinematic,
        Collider::capsule(0.45, 1.0),
        CollisionLayers::new(GameLayer::Player, GameLayer::World),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.5, 0.0)),
    ));

    camera::spawn_camera(&mut commands);
    ui::spawn_hud(&mut commands);
    spawn_debug_spawn_legend(&mut commands);
}

/// Small always-on corner readout for the F1-F4 debug spawn keys, since
/// they aren't discoverable any other way in this zone.
fn spawn_debug_spawn_legend(commands: &mut Commands) {
    commands.spawn((
        Name::new("Test Grounds Spawn Legend"),
        Text::new(
            "TEST GROUNDS\nF1 Spawn Normal   F2 Spawn Elite\nF3 Spawn Miniboss   F4 Spawn Boss\n[F] near a chest to loot it",
        ),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            left: px(16.0),
            top: px(16.0),
            ..default()
        },
    ));
}

// =========================================================
// ON-DEMAND ENEMY SPAWNING (F1-F4)
// =========================================================

/// Fixed HP the debug spawner gives a "miniboss" (one of the three normal
/// archetypes, stat-boosted -- see the module doc comment) so it's a
/// visibly tougher test target than a normal enemy, regardless of which
/// archetype gets picked. Not a balance decision -- just enough to make
/// the health bar and fight length obviously different per tier.
const MINIBOSS_HEALTH: f32 = 400.0;

/// Floor values fed to `LootFloor` so each tier's kill can actually reach
/// its loot table's higher tiers (see loot/tables.rs gating): Elite (the
/// Black Banner Knight) up to Epic (min_floor 4), Miniboss up to its own
/// Epic cap (min_floor 5), Boss up to Legendary (min_floor 8).
const ELITE_LOOT_FLOOR: u32 = 5;
const MINIBOSS_LOOT_FLOOR: u32 = 5;
const BOSS_LOOT_FLOOR: u32 = 8;

const SPAWN_DISTANCE: f32 = 6.0;
const BOSS_SPAWN_DISTANCE: f32 = 10.0;

type NormalEnemySpawnFn = fn(
    &mut Commands,
    &mut ResMut<Assets<Mesh>>,
    &mut ResMut<Assets<StandardMaterial>>,
    Vec3,
    EnemyEncounterConfig,
) -> Entity;

/// The three confirmed normal-enemy archetypes, rotated through by
/// `TestSpawnCounter` so repeated presses of the same key don't always
/// spawn the same enemy.
const NORMAL_ENEMY_SPAWNERS: [NormalEnemySpawnFn; 3] = [
    enemies::spawn_ruinbound_footman,
    enemies::spawn_gatebow_marksman,
    enemies::spawn_cinder_hound,
];
const NORMAL_ENEMY_LABELS: [&str; 3] = ["Ruinbound Footman", "Gatebow Marksman", "Cinder Hound"];

#[derive(Resource, Default)]
struct TestSpawnCounter(usize);

fn debug_spawn_enemy(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    cursor: Single<&CursorOptions>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_query: Query<&Transform, With<Player>>,
    mut counter: ResMut<TestSpawnCounter>,
) {
    if cursor.grab_mode == CursorGrabMode::None {
        return;
    }
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let spawn_point = |distance: f32| -> Vec3 {
        let mut forward = player_transform.forward().as_vec3();
        forward.y = 0.0;
        let forward = forward.normalize_or_zero();
        player_transform.translation + forward * distance
    };

    if keyboard.just_pressed(KeyCode::F1) {
        spawn_test_normal_enemy(
            &mut commands,
            &mut meshes,
            &mut materials,
            spawn_point(SPAWN_DISTANCE),
            &mut counter,
            None,
            None,
            "Normal",
        );
    }

    if keyboard.just_pressed(KeyCode::F2) {
        let position = spawn_point(SPAWN_DISTANCE);
        let encounter = EnemyEncounterConfig::around(position);
        let knight = enemies::spawn_black_banner_knight(
            &mut commands,
            &mut meshes,
            &mut materials,
            position,
            encounter,
        );
        // spawn_black_banner_knight already sets LootTableId::ELITE_ENEMY
        // and an elite-appropriate Health; only the floor needs
        // overriding so Epic archetype gear (min_floor 4) is reachable
        // from an actual kill, same as the Saint's LootFloor override below.
        commands.entity(knight).insert(LootFloor(ELITE_LOOT_FLOOR));
        spawn_enemy_health_bar(&mut commands, knight);
        println!("Test spawn: Black Banner Knight");
    }

    if keyboard.just_pressed(KeyCode::F3) {
        spawn_test_normal_enemy(
            &mut commands,
            &mut meshes,
            &mut materials,
            spawn_point(SPAWN_DISTANCE + 1.0),
            &mut counter,
            Some((LootTableId::MINIBOSS, MINIBOSS_HEALTH)),
            Some(MINIBOSS_LOOT_FLOOR),
            "Miniboss",
        );
    }

    if keyboard.just_pressed(KeyCode::F4) {
        let position = spawn_point(BOSS_SPAWN_DISTANCE);
        let encounter = EnemyEncounterConfig::around(position);
        let boss = enemies::spawn_saint_of_the_broken_siege(
            &mut commands,
            &asset_server,
            position,
            encounter,
        );
        // The Saint's own spawn function already sets LootTableId::BOSS;
        // only the floor needs overriding so Legendary archetype gear
        // (min_floor 8) is reachable from an actual kill.
        commands.entity(boss).insert(LootFloor(BOSS_LOOT_FLOOR));
        spawn_enemy_health_bar(&mut commands, boss);
        println!("Test spawn: Saint of the Broken Siege");
    }
}

/// Spawns one of the three normal-enemy archetypes (round-robin via
/// `counter`), then optionally overrides its loot table/health (for the
/// Elite/Miniboss test tiers) and/or attaches a `LootFloor` override.
/// `None` for `difficulty` leaves a true, unmodified normal enemy.
fn spawn_test_normal_enemy(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    counter: &mut TestSpawnCounter,
    difficulty: Option<(LootTableId, f32)>,
    loot_floor: Option<u32>,
    tier_label: &str,
) {
    let index = counter.0 % NORMAL_ENEMY_SPAWNERS.len();
    counter.0 = counter.0.wrapping_add(1);

    let encounter = EnemyEncounterConfig::around(position);
    let enemy = NORMAL_ENEMY_SPAWNERS[index](commands, meshes, materials, position, encounter);

    if let Some((loot_table, health)) = difficulty {
        commands
            .entity(enemy)
            .insert((loot_table, Health::new(health)));
    }
    if let Some(floor) = loot_floor {
        commands.entity(enemy).insert(LootFloor(floor));
    }

    commands.entity(enemy).insert(Name::new(format!(
        "{tier_label} {}",
        NORMAL_ENEMY_LABELS[index]
    )));

    spawn_enemy_health_bar(commands, enemy);

    println!("Test spawn: {tier_label} {}", NORMAL_ENEMY_LABELS[index]);
}
