//! Which player character each enemy is after.
//!
//! Every enemy carries an `EnemyTarget` (required by `Enemy`).
//! `select_enemy_targets` fills it once per frame, before any archetype's
//! combat system runs, and every archetype, the shared navmesh targeting
//! (`navigation_mesh::update_navigation_targets`) and the Marksman's aim
//! telegraph read it. So "who is this enemy chasing?" is decided in exactly
//! one place, for every current and future enemy.
//!
//! Rule: the nearest player character (horizontal distance), preferring
//! players inside the enemy's aggro area. An enemy keeps its current target
//! unless another player is at least `TARGET_SWITCH_MARGIN` closer, so two
//! players standing at similar distances don't make it flick back and forth.
//!
//! With one player (offline, or a host on its own) the target is always that
//! player, so single-player behaviour is unchanged.
//!
//! Only players who are Alive are chosen (`player::LifeState`): a downed or
//! lost player is dropped as a target, and with nobody standing enemies have
//! no target at all.

use bevy::prelude::*;

use crate::player::{LifeState, PlayerCharacter};

use super::{Enemy, EnemyAwareness};

/// How much closer (world units) another player must be before an enemy
/// abandons its current target for them.
const TARGET_SWITCH_MARGIN: f32 = 2.0;

/// The player character this enemy is currently after, if any.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct EnemyTarget {
    pub entity: Option<Entity>,
}

/// Looks up `enemy`'s current target. A separate read-only
/// `Query<&EnemyTarget>` keeps each archetype's (already long) enemy query
/// unchanged.
pub(super) fn current_target(targets: &Query<&EnemyTarget>, enemy: Entity) -> Option<Entity> {
    targets.get(enemy).ok().and_then(|target| target.entity)
}

pub(super) fn select_enemy_targets(
    all_players: Query<(Entity, &Transform, &LifeState), (With<PlayerCharacter>, Without<Enemy>)>,
    mut enemies: Query<
        (&Transform, &EnemyAwareness, &mut EnemyTarget),
        (With<Enemy>, Without<PlayerCharacter>),
    >,
) {
    // Only standing players can be chased.
    let players: Vec<(Entity, &Transform)> = all_players
        .iter()
        .filter(|(_, _, life)| life.is_alive())
        .map(|(entity, transform, _)| (entity, transform))
        .collect();

    for (transform, awareness, mut target) in &mut enemies {
        let position = transform.translation;
        let distance_to = |player_position: Vec3| {
            let mut offset = player_position - position;
            offset.y = 0.0;
            offset.length()
        };

        let current = target
            .entity
            .and_then(|entity| players.iter().find(|(candidate, _)| *candidate == entity))
            .map(|(entity, player)| (*entity, distance_to(player.translation)));

        let any_in_aggro_area = players
            .iter()
            .any(|(_, player)| awareness.contains_aggroed_player(player.translation));
        let nearest = players
            .iter()
            .filter(|(_, player)| {
                !any_in_aggro_area || awareness.contains_aggroed_player(player.translation)
            })
            .map(|(entity, player)| (*entity, distance_to(player.translation)))
            .min_by(|a, b| a.1.total_cmp(&b.1));

        let chosen = choose_target(current, nearest);
        if target.entity != chosen {
            target.entity = chosen;
        }
    }
}

/// Keeps `current` unless `nearest` beats it by more than
/// `TARGET_SWITCH_MARGIN`. Each argument is `(who, horizontal distance)`.
fn choose_target<T: Copy>(current: Option<(T, f32)>, nearest: Option<(T, f32)>) -> Option<T> {
    match (current, nearest) {
        (Some((_, current_distance)), Some((nearest_target, nearest_distance)))
            if nearest_distance + TARGET_SWITCH_MARGIN < current_distance =>
        {
            Some(nearest_target)
        }
        (Some((current_target, _)), _) => Some(current_target),
        (None, nearest) => nearest.map(|(target, _)| target),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_nearest_player_when_it_has_no_target() {
        assert_eq!(choose_target::<u32>(None, Some((7, 3.0))), Some(7));
        assert_eq!(choose_target::<u32>(None, None), None);
    }

    #[test]
    fn keeps_its_target_against_a_slightly_closer_player() {
        assert_eq!(choose_target(Some((1u32, 5.0)), Some((2, 4.0))), Some(1));
    }

    #[test]
    fn switches_to_a_clearly_closer_player() {
        assert_eq!(choose_target(Some((1u32, 9.0)), Some((2, 3.0))), Some(2));
    }

    #[test]
    fn keeps_its_target_when_it_is_also_the_nearest() {
        assert_eq!(choose_target(Some((1u32, 3.0)), Some((1, 3.0))), Some(1));
    }
}
