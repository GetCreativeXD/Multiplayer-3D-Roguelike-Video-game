use bevy::prelude::*;

use crate::combat::Health;
use crate::enemies::Enemy;

use crate::weapons::components::PlayerProjectile;

const SPLINTER_SEARCH_RADIUS: f32 = 12.0;
pub(crate) const SPLINTER_FRAGMENT_SPEED: f32 = 20.0;
pub(crate) const SPLINTER_FRAGMENT_LIFETIME: f32 = 2.0;
pub(crate) const SPLINTER_FRAGMENT_DAMAGE_MULTIPLIER: f32 = 0.55;

const SPLINTER_FRAGMENT_COUNT: usize = 3;

pub(crate) fn find_splinter_targets(
    impact_position: Vec3,
    main_target: Entity,
    main_target_alive: bool,
    enemy_query: &Query<(Entity, &Transform, &Health), (With<Enemy>, Without<PlayerProjectile>)>,
) -> Vec<Entity> {
    let mut candidates: Vec<(Entity, f32)> = Vec::new();

    for (entity, transform, health) in enemy_query.iter() {
        if entity == main_target {
            continue;
        }

        if health.current <= 0.0 {
            continue;
        }

        let distance = impact_position.distance(transform.translation);

        if distance <= SPLINTER_SEARCH_RADIUS {
            candidates.push((entity, distance));
        }
    }

    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut targets = Vec::new();

    // First assign fragments to unique nearby enemies.
    for (entity, _) in candidates.iter().take(SPLINTER_FRAGMENT_COUNT) {
        targets.push(*entity);
    }

    // Fill any remaining fragment slots.
    while targets.len() < SPLINTER_FRAGMENT_COUNT {
        if main_target_alive {
            // Main target survived, so extra fragments
            // return to it.
            targets.push(main_target);
        } else if !candidates.is_empty() {
            // Main target died. Redirect extra fragments
            // to the closest surviving secondary target.
            targets.push(candidates[0].0);
        } else {
            // Nothing alive nearby to target.
            break;
        }
    }

    // Return the target list.
    targets
}
