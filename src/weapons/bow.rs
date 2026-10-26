use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::*;
use bevy::prelude::*;

use super::components::{
    PlayerProjectile, Projectile, ProjectileKind, ProjectileOwner, RicochetProjectile, Weapon,
};

use super::{ARROW_LIFETIME, ARROW_RADIUS, ARROW_SPEED, BOW_PIERCING_DAMAGE_MULTIPLIER};

const BOW_SPLINTER_DAMAGE_MULTIPLIER: f32 = 1.25;

const BOW_RICOCHET_DAMAGE_MULTIPLIER: f32 = 1.5;
const BOW_RICOCHET_SPEED: f32 = 40.0;
const BOW_RICOCHET_LIFETIME: f32 = 5.0;
const BOW_RICOCHET_COUNT: u32 = 3;

pub(crate) fn fire_bow(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    damage_multiplier: f32,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
) {
    let camera_origin = camera_transform.translation;

    let camera_direction = camera_transform.forward().as_vec3().normalize_or_zero();

    if camera_direction == Vec3::ZERO {
        return;
    }

    // Don't let the raycast hit the player itself.
    let filter = SpatialQueryFilter::from_excluded_entities([player_entity]);

    // Maximum aiming distance.
    let max_distance = weapon.range;

    // Default target if nothing is hit.
    let mut target_point = camera_origin + camera_direction * max_distance;

    // Raycast from the camera through the crosshair.
    if let Some(hit) = spatial_query.cast_ray(
        camera_origin.adjust_precision(),
        Dir3::new(camera_direction.adjust_precision()).unwrap(),
        max_distance,
        true,
        &filter,
    ) {
        target_point = camera_origin + camera_direction * hit.distance;
    }

    // Arrow starts from the player's position.
    let spawn_position = player_transform.translation + Vec3::Y * 0.6 + camera_direction * 0.9;

    // Aim from the actual arrow spawn point toward
    // whatever the crosshair is targeting.
    let arrow_direction = (target_point - spawn_position).normalize_or_zero();

    if arrow_direction == Vec3::ZERO {
        return;
    }

    // Rotate player horizontally toward target.
    let mut horizontal_direction = arrow_direction;
    horizontal_direction.y = 0.0;
    horizontal_direction = horizontal_direction.normalize_or_zero();

    if horizontal_direction != Vec3::ZERO {
        player_transform.rotation =
            Quat::from_rotation_y(horizontal_direction.x.atan2(horizontal_direction.z));
    }

    commands.spawn((
        PlayerProjectile,
        ProjectileOwner(player_entity),
        Projectile {
            velocity: arrow_direction * ARROW_SPEED,
            damage: weapon.damage * damage_multiplier,
            knockback: weapon.knockback,
            stagger_strength: weapon.stagger_strength,
            stagger_duration: weapon.stagger_duration,
            lifetime: ARROW_LIFETIME,

            piercing: false,
            hit_entities: Vec::new(),

            kind: ProjectileKind::Normal,
            target: None,
        },
        Mesh3d(meshes.add(Sphere::new(ARROW_RADIUS))),
        MeshMaterial3d(materials.add(Color::srgb(0.9, 0.75, 0.25))),
        Transform::from_translation(spawn_position),
    ));

    println!("Bow fired!");
}

pub(crate) fn fire_piercing_shot(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
) {
    let camera_origin = camera_transform.translation;

    let camera_direction = camera_transform.forward().as_vec3().normalize_or_zero();

    if camera_direction == Vec3::ZERO {
        return;
    }

    let filter = SpatialQueryFilter::from_excluded_entities([player_entity]);

    let max_distance = weapon.range;

    let mut target_point = camera_origin + camera_direction * max_distance;

    if let Ok(ray_direction) = Dir3::new(camera_direction.adjust_precision()) {
        if let Some(hit) = spatial_query.cast_ray(
            camera_origin.adjust_precision(),
            ray_direction,
            max_distance,
            true,
            &filter,
        ) {
            target_point = camera_origin + camera_direction * hit.distance;
        }
    }

    let spawn_position = player_transform.translation + Vec3::Y * 0.6 + camera_direction * 0.9;

    let arrow_direction = (target_point - spawn_position).normalize_or_zero();

    if arrow_direction == Vec3::ZERO {
        return;
    }

    let mut horizontal_direction = arrow_direction;

    horizontal_direction.y = 0.0;

    horizontal_direction = horizontal_direction.normalize_or_zero();

    if horizontal_direction != Vec3::ZERO {
        player_transform.rotation =
            Quat::from_rotation_y(horizontal_direction.x.atan2(horizontal_direction.z));
    }

    commands.spawn((
        PlayerProjectile,
        ProjectileOwner(player_entity),
        Projectile {
            velocity: arrow_direction * ARROW_SPEED,

            damage: weapon.damage * BOW_PIERCING_DAMAGE_MULTIPLIER,

            knockback: weapon.knockback,

            stagger_strength: weapon.stagger_strength,

            stagger_duration: weapon.stagger_duration,

            lifetime: ARROW_LIFETIME,

            piercing: true,
            hit_entities: Vec::new(),

            kind: ProjectileKind::Piercing,
            target: None,
        },
        Mesh3d(meshes.add(Sphere::new(ARROW_RADIUS * 1.25))),
        MeshMaterial3d(materials.add(Color::srgb(0.35, 0.8, 1.0))),
        Transform::from_translation(spawn_position),
    ));

    println!("Piercing Shot!");
}

pub(crate) fn fire_splinter_shot(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
) {
    let camera_origin = camera_transform.translation;

    let camera_direction = camera_transform.forward().as_vec3().normalize_or_zero();

    if camera_direction == Vec3::ZERO {
        return;
    }

    let filter = SpatialQueryFilter::from_excluded_entities([player_entity]);

    let mut target_point = camera_origin + camera_direction * weapon.range;

    if let Ok(ray_direction) = Dir3::new(camera_direction.adjust_precision()) {
        if let Some(hit) = spatial_query.cast_ray(
            camera_origin.adjust_precision(),
            ray_direction,
            weapon.range,
            true,
            &filter,
        ) {
            target_point = camera_origin + camera_direction * hit.distance;
        }
    }

    let spawn_position = player_transform.translation + Vec3::Y * 0.6 + camera_direction * 0.9;

    let arrow_direction = (target_point - spawn_position).normalize_or_zero();

    if arrow_direction == Vec3::ZERO {
        return;
    }

    commands.spawn((
        PlayerProjectile,
        ProjectileOwner(player_entity),
        Projectile {
            velocity: arrow_direction * super::ARROW_SPEED,

            damage: weapon.damage * BOW_SPLINTER_DAMAGE_MULTIPLIER,

            knockback: weapon.knockback,

            stagger_strength: weapon.stagger_strength,

            stagger_duration: weapon.stagger_duration,

            lifetime: super::ARROW_LIFETIME,

            piercing: false,
            hit_entities: Vec::new(),

            kind: ProjectileKind::Splinter,
            target: None,
        },
        Mesh3d(meshes.add(Sphere::new(super::ARROW_RADIUS * 1.2))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.4, 1.0))),
        Transform::from_translation(spawn_position),
    ));

    println!("SPLINTER SHOT!");
}

pub(crate) fn fire_ricochet_arrow(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
) {
    let camera_origin = camera_transform.translation;

    let camera_direction = camera_transform.forward().as_vec3().normalize_or_zero();

    if camera_direction == Vec3::ZERO {
        return;
    }

    // Don't let the aiming ray hit the player.
    let filter = SpatialQueryFilter::from_excluded_entities([player_entity]);

    let mut target_point = camera_origin + camera_direction * weapon.range;

    // Aim through the crosshair just like the other Bow attacks.
    if let Ok(ray_direction) = Dir3::new(camera_direction.adjust_precision()) {
        if let Some(hit) = spatial_query.cast_ray(
            camera_origin.adjust_precision(),
            ray_direction,
            weapon.range,
            true,
            &filter,
        ) {
            target_point = camera_origin + camera_direction * hit.distance;
        }
    }

    let spawn_position = player_transform.translation + Vec3::Y * 0.6 + camera_direction * 0.9;

    let arrow_direction = (target_point - spawn_position).normalize_or_zero();

    if arrow_direction == Vec3::ZERO {
        return;
    }

    // Keep the player upright while facing toward the shot.
    let mut horizontal_direction = arrow_direction;

    horizontal_direction.y = 0.0;

    horizontal_direction = horizontal_direction.normalize_or_zero();

    if horizontal_direction != Vec3::ZERO {
        player_transform.rotation =
            Quat::from_rotation_y(horizontal_direction.x.atan2(horizontal_direction.z));
    }

    commands.spawn((
        PlayerProjectile,
        ProjectileOwner(player_entity),
        Projectile {
            velocity: arrow_direction * BOW_RICOCHET_SPEED,

            damage: weapon.damage * BOW_RICOCHET_DAMAGE_MULTIPLIER,

            knockback: weapon.knockback,

            stagger_strength: weapon.stagger_strength,

            stagger_duration: weapon.stagger_duration,

            lifetime: BOW_RICOCHET_LIFETIME,

            // Important:
            // Ricochet has its own collision behavior.
            piercing: false,

            hit_entities: Vec::new(),

            kind: ProjectileKind::Ricochet,

            target: None,
        },
        RicochetProjectile {
            ricochets_remaining: BOW_RICOCHET_COUNT,
            split_lockout: 0.0,
        },
        Mesh3d(meshes.add(Sphere::new(ARROW_RADIUS * 1.35))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.55, 0.15))),
        Transform::from_translation(spawn_position),
    ));

    println!("RICOCHET ARROW! {} ricochets", BOW_RICOCHET_COUNT);
}
