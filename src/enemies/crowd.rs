use bevy::prelude::*;

use super::{Enemy, EnemyNavigation};

const ENEMY_SEPARATION_RADIUS: f32 = 1.5;
// Separation should gently spread a group without overpowering its path.
// Values above 1.0 could reverse an enemy's desired direction and make it
// oscillate even when the direct route to the player was clear.
const ENEMY_SEPARATION_WEIGHT: f32 = 0.55;

/// Builds a lightweight local separation vector for every enemy. Pathfinding
/// still chooses the route; this only prevents several enemies from trying to
/// occupy the same point along that route.
pub(super) fn update_enemy_crowd_steering(
    mut enemies: Query<(Entity, &Transform, &mut EnemyNavigation), With<Enemy>>,
) {
    let positions: Vec<(Entity, Vec3)> = enemies
        .iter()
        .map(|(entity, transform, _)| (entity, transform.translation))
        .collect();

    for (entity, transform, mut navigation) in &mut enemies {
        let mut separation = Vec3::ZERO;

        for (other_entity, other_position) in &positions {
            if *other_entity == entity {
                continue;
            }

            let mut away = transform.translation - *other_position;
            away.y = 0.0;
            let distance = away.length();
            if distance >= ENEMY_SEPARATION_RADIUS {
                continue;
            }

            let direction = if distance > 0.001 {
                away / distance
            } else if entity.index() < other_entity.index() {
                Vec3::X
            } else {
                Vec3::NEG_X
            };
            let strength = 1.0 - distance / ENEMY_SEPARATION_RADIUS;
            separation += direction * strength;
        }

        navigation.set_crowd_steering(separation.clamp_length_max(1.0) * ENEMY_SEPARATION_WEIGHT);
    }
}
