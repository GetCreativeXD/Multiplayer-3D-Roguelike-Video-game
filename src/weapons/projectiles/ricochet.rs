use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::player::PlayerCharacter;

use crate::weapons::components::{
    PlayerProjectile, Projectile, ProjectileKind, RicochetProjectile,
};

use crate::weapons::ARROW_RADIUS;

pub(crate) fn ricochet_surface_normal(
    spatial_query: &SpatialQuery,
    start: Vec3,
    cast_direction: Dir3,
    hit_distance: f32,
    fallback_normal: Vec3,
    filter: &SpatialQueryFilter,
) -> Vec3 {
    let corner_hits = spatial_query.ray_hits(
        start.adjust_precision(),
        cast_direction,
        hit_distance + 0.03,
        8,
        true,
        filter,
    );

    let mut combined_normal = Vec3::ZERO;

    for corner_hit in corner_hits {
        if (corner_hit.distance - hit_distance).abs() <= 0.05 {
            combined_normal += corner_hit.normal.normalize_or_zero();
        }
    }

    if combined_normal.length_squared() > 0.001 {
        combined_normal.normalize_or_zero()
    } else {
        fallback_normal.normalize_or_zero()
    }
}

pub(crate) fn ricochet_child_directions(
    reflected_direction: Vec3,
    surface_normal: Vec3,
) -> [Vec3; 3] {
    let mut side = surface_normal.cross(reflected_direction);

    if side.length_squared() < 0.001 {
        side = surface_normal.cross(Vec3::Y);

        if side.length_squared() < 0.001 {
            side = surface_normal.cross(Vec3::X);
        }
    }

    side = side.normalize_or_zero();

    [
        reflected_direction,
        (reflected_direction + side * 0.22).normalize_or_zero(),
        (reflected_direction - side * 0.22).normalize_or_zero(),
    ]
}

pub(crate) fn handle_ricochet_collision(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    spatial_query: &SpatialQuery,
    player_query: &Query<Entity, With<PlayerCharacter>>,

    projectile_entity: Entity,
    projectile_transform: &mut Transform,
    projectile: &mut Projectile,
    ricochet: &mut RicochetProjectile,

    start: Vec3,
    direction: Vec3,
    cast_direction: Dir3,
    hit_distance: f32,
    hit_normal: Vec3,
    filter: &SpatialQueryFilter,
) {
    // =====================================================
    // SHORT POST-SPLIT LOCKOUT
    // =====================================================

    if ricochet.split_lockout > 0.0 {
        let surface_normal = hit_normal.normalize_or_zero();

        if surface_normal == Vec3::ZERO {
            commands.entity(projectile_entity).despawn();

            return;
        }

        let speed = projectile.velocity.length();

        let reflected_direction = direction.reflect(surface_normal).normalize_or_zero();

        if reflected_direction == Vec3::ZERO {
            commands.entity(projectile_entity).despawn();

            return;
        }

        projectile.velocity = reflected_direction * speed;

        projectile_transform.translation = start + direction * (hit_distance - 0.05).max(0.0);

        return;
    }

    // =====================================================
    // FINAL IMPACT
    // =====================================================

    if ricochet.ricochets_remaining == 0 {
        println!("Ricochet Arrow exhausted.");

        commands.entity(projectile_entity).despawn();

        return;
    }

    // =====================================================
    // SURFACE / CORNER NORMAL
    // =====================================================

    let surface_normal = ricochet_surface_normal(
        spatial_query,
        start,
        cast_direction,
        hit_distance,
        hit_normal,
        filter,
    );

    if surface_normal == Vec3::ZERO {
        commands.entity(projectile_entity).despawn();

        return;
    }

    let speed = projectile.velocity.length();

    let reflected_direction = direction.reflect(surface_normal).normalize_or_zero();

    if reflected_direction == Vec3::ZERO {
        commands.entity(projectile_entity).despawn();

        return;
    }

    let next_ricochets = ricochet.ricochets_remaining - 1;

    let safe_impact_position = start + direction * (hit_distance - 0.25).max(0.0);

    let child_directions = ricochet_child_directions(reflected_direction, surface_normal);

    // =====================================================
    // CHILD PROJECTILES
    // =====================================================

    for mut child_direction in child_directions {
        if child_direction == Vec3::ZERO {
            continue;
        }

        if child_direction.dot(surface_normal) <= 0.01 {
            child_direction = (child_direction + surface_normal * 0.25).normalize_or_zero();
        }

        if child_direction == Vec3::ZERO {
            continue;
        }

        let child_spawn_position = safe_impact_position;

        let child_clearance_distance = 0.35;

        let child_filter = SpatialQueryFilter::from_excluded_entities(player_query.iter());

        if let Ok(child_cast_direction) = Dir3::new(child_direction.adjust_precision()) {
            if let Some(second_hit) = spatial_query.cast_ray(
                child_spawn_position.adjust_precision(),
                child_cast_direction,
                child_clearance_distance,
                true,
                &child_filter,
            ) {
                let second_normal = second_hit.normal.normalize_or_zero();

                if second_normal != Vec3::ZERO {
                    child_direction = child_direction.reflect(second_normal).normalize_or_zero();
                }
            }
        }

        commands.spawn((
            PlayerProjectile,
            Projectile {
                velocity: child_direction * speed,

                damage: projectile.damage,

                knockback: projectile.knockback,

                stagger_strength: projectile.stagger_strength,

                stagger_duration: projectile.stagger_duration,

                lifetime: projectile.lifetime,

                piercing: false,

                hit_entities: Vec::new(),

                kind: ProjectileKind::Ricochet,

                target: None,
            },
            RicochetProjectile {
                ricochets_remaining: next_ricochets,

                split_lockout: 0.08,
            },
            Mesh3d(meshes.add(Sphere::new(ARROW_RADIUS))),
            MeshMaterial3d(materials.add(Color::srgb(1.0, 0.55, 0.15))),
            Transform::from_translation(child_spawn_position),
        ));
    }

    println!("RICOCHET SPLIT! {} generations remaining", next_ricochets);

    commands.entity(projectile_entity).despawn();
}
