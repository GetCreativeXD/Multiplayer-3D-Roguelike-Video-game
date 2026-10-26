use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::{camera, characters, collision::GameLayer, enemies, player, ui, weapons};

use crate::combat::{BaseHealth, Health};

pub struct ArenaPlugin;

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_arena);
    }
}

fn setup_arena(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut cursor_options: Single<&mut CursorOptions>,
) {
    // Lock and hide cursor during gameplay.
    cursor_options.visible = false;
    cursor_options.grab_mode = CursorGrabMode::Locked;

    // =========================================================
    // PLAYER
    // =========================================================
    // Matches the Saint arena's "Player Spawn" room connector
    // (rooms/saint_arena.rs), itself derived from the GLB's authored
    // SPAWN_PLAYER_Entrance marker. Re-measured 2026-08-30 after the arena
    // was scaled down 0.72x on X/Z in Blender (z=31.0 -> 22.32).

    commands.spawn((
        player::Player,
        characters::vanguard::Vanguard,
        BaseHealth(100.0),
        Health::new(100.0),
        player::Dodge::default(),
        player::VerticalMovement::default(),
        weapons::PlayerCombat::default(),
        weapons::Weapon::bow(weapons::GearTier::Common),
        Transform::from_xyz(0.0, 1.3, 22.32),
        Visibility::default(),
        // Physics
        RigidBody::Kinematic,
        Collider::capsule(0.45, 1.0),
        CollisionLayers::new(GameLayer::Player, GameLayer::World),
    ));

    // =========================================================
    // SAINT OF THE BROKEN SIEGE
    // =========================================================
    // First boss-arena integration pass: the Saint starts on the main
    // circular floor rather than the raised rear platform (the GLB's
    // authored SPAWN_BOSS marker sits on the platform) so Fist Slam can be
    // tested on one continuous surface before platform/stair traversal is
    // built. Y=2.7 matches the boss capsule's half-height (radius 1.2 +
    // half cylinder length 1.5), so it rests correctly on the floor.
    //
    // The activation region covers the full arena floor, so the fight
    // begins as soon as the player is in the room; narrow this later if a
    // slower opening beat is wanted.

    // Half-extents match the resized floor collider in saint_arena.rs
    // (52.272 / 2 = 26.136). Boss spawn Z scaled by the same 0.72 factor
    // applied to the room (-8.0 -> -5.76) so it sits in the same relative
    // spot on the main floor.
    let boss_encounter = enemies::EnemyEncounterConfig {
        encounter_id: 1,
        activation_center: Vec3::new(0.0, 0.0, 0.0),
        activation_half_extents: Vec2::new(26.136, 26.136),
        aggro_extension: 20.0,
    };

    let boss_entity = enemies::spawn_saint_of_the_broken_siege(
        &mut commands,
        &asset_server,
        Vec3::new(0.0, 2.7, -5.76),
        boss_encounter,
    );

    ui::spawn_enemy_health_bar(&mut commands, boss_entity);

    // =========================================================
    // LIGHT
    // =========================================================

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.5, 0.0)),
    ));

    // =========================================================
    // CAMERA
    // =========================================================

    camera::spawn_camera(&mut commands);

    // =========================================================
    // UI Player Health Bar, WEAPON INDICATOR, Crosshair
    // =========================================================

    ui::spawn_hud(&mut commands);
}
