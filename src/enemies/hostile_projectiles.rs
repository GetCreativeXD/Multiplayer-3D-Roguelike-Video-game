use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::combat::{Barriers, DeathPrevention, Health, apply_player_damage};
use crate::player::{Dodge, PlayerCharacter};
use crate::weapons::Equipment;

use super::Enemy;

#[derive(Component)]
pub struct HostileProjectile {
    pub velocity: Vec3,
    pub damage: f32,
    pub lifetime: f32,
    pub radius: f32,
    pub owner: Entity,
}

pub fn spawn_hostile_projectile(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    owner: Entity,
    origin: Vec3,
    direction: Vec3,
    speed: f32,
    damage: f32,
    lifetime: f32,
    radius: f32,
    color: Color,
) {
    let direction = direction.normalize_or_zero();
    if direction == Vec3::ZERO {
        return;
    }

    commands.spawn((
        Name::new("Hostile Projectile"),
        HostileProjectile {
            velocity: direction * speed,
            damage,
            lifetime,
            radius,
            owner,
        },
        Mesh3d(meshes.add(Cuboid::new(radius * 0.55, radius * 0.55, radius * 4.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: LinearRgba::new(0.8, 0.22, 0.02, 1.0),
            ..default()
        })),
        Transform {
            translation: origin,
            rotation: Quat::from_rotation_arc(Vec3::Z, direction),
            ..default()
        },
    ));
}

pub fn move_hostile_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    spatial_query: SpatialQuery,
    enemies: Query<Entity, With<Enemy>>,
    mut player: Query<
        (
            &mut Health,
            &Dodge,
            &Equipment,
            &mut Barriers,
            &mut DeathPrevention,
        ),
        (With<PlayerCharacter>, Without<Enemy>),
    >,
    mut projectiles: Query<(Entity, &mut Transform, &mut HostileProjectile)>,
) {
    let delta = time.delta_secs();

    for (entity, mut transform, mut projectile) in &mut projectiles {
        projectile.lifetime -= delta;
        if projectile.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        let movement = projectile.velocity * delta;
        let distance = movement.length();
        let direction = movement.normalize_or_zero();
        if distance <= 0.0 || direction == Vec3::ZERO {
            continue;
        }

        let mut excluded = vec![entity, projectile.owner];
        excluded.extend(enemies.iter());
        let filter = SpatialQueryFilter::from_excluded_entities(excluded);
        let Ok(cast_direction) = Dir3::new(direction.adjust_precision()) else {
            continue;
        };

        let hit = spatial_query.cast_shape(
            &Collider::sphere(projectile.radius),
            transform.translation.adjust_precision(),
            Quat::IDENTITY.adjust_precision(),
            cast_direction,
            &ShapeCastConfig::from_max_distance(distance),
            &filter,
        );

        if let Some(hit) = hit {
            transform.translation += direction * hit.distance;

            // Any player character the projectile reaches is hit, not only
            // the one the shooter was aiming at.
            if let Ok((mut health, dodge, equipment, mut barriers, mut death_prevention)) =
                player.get_mut(hit.entity)
            {
                if dodge.is_invulnerable() {
                    println!("Player dodged a hostile projectile!");
                } else {
                    let result = apply_player_damage(
                        &mut health,
                        equipment,
                        Some(&mut barriers),
                        Some(&mut death_prevention),
                        projectile.damage,
                    );
                    println!(
                        "Hostile projectile hit for {:.1} ({:.1} blocked)! HP: {:.0}/{:.0}",
                        result.applied, result.blocked, health.current, health.max,
                    );
                }
            }

            commands.entity(entity).despawn();
        } else {
            transform.translation += movement;
        }
    }
}
