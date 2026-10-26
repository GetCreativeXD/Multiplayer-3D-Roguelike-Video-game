use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_landmass::{AgentDesiredVelocity3d, prelude::AgentState};

use crate::combat::{
    Barriers, DeathPrevention, EnemyBuffs, EnemyDebuffs, Health, apply_player_damage,
    enemy_damage_multiplier, enemy_debuff_movement_speed_multiplier,
    enemy_movement_speed_multiplier,
};
use crate::navigation_mesh::{NavmeshRouteCache, navmesh_pursuit_step};
use crate::player::{Dodge, PlayerCharacter};
use crate::weapons::Equipment;

use super::targeting::{EnemyTarget, current_target};
use super::{
    ENEMY_ATTACK_DAMAGE, ENEMY_ATTACK_DURATION, ENEMY_ATTACK_SPEED, ENEMY_HIT_RANGE,
    ENEMY_RECOVERY_DURATION, Enemy, EnemyAwareness, EnemyCombat, EnemyMovement, EnemyNavigation,
    EnemyNavigationGrid, EnemyState, MeleeEnemy, Stagger, enemy_grounded_path_is_safe,
    enemy_step_is_safe, enemy_world_path_is_clear_ignoring, safe_enemy_locomotion,
};

const MELEE_ATTACK_STALL_TIMEOUT: f32 = 0.2;

pub(super) fn enemy_combat(
    time: Res<Time>,
    spatial_query: SpatialQuery,
    mut navigation_grid: ResMut<EnemyNavigationGrid>,

    mut player_query: Query<
        (
            Entity,
            &Transform,
            &mut Health,
            &Dodge,
            &Equipment,
            &mut Barriers,
            &mut DeathPrevention,
        ),
        (With<PlayerCharacter>, Without<Enemy>),
    >,
    targets: Query<&EnemyTarget>,

    mut enemy_query: Query<
        (
            Entity,
            &mut Transform,
            &EnemyMovement,
            &mut EnemyNavigation,
            &mut EnemyCombat,
            &Stagger,
            &EnemyAwareness,
            Option<&EnemyBuffs>,
            Option<&EnemyDebuffs>,
            Option<&AgentDesiredVelocity3d>,
            Option<&AgentState>,
            Option<&NavmeshRouteCache>,
        ),
        (With<Enemy>, With<MeleeEnemy>, Without<PlayerCharacter>),
    >,
) {
    let delta = time.delta_secs();

    for (
        entity,
        mut enemy_transform,
        movement,
        mut navigation,
        mut combat,
        stagger,
        awareness,
        enemy_buffs,
        enemy_debuffs,
        desired_velocity,
        agent_state,
        route_cache,
    ) in &mut enemy_query
    {
        if !awareness.engaged {
            continue;
        }

        // Enemy cannot chase or attack while staggered.
        if stagger.is_staggered() {
            continue;
        }

        // The player this Footman is after (see `enemies::targeting`).
        let Some(target) = current_target(&targets, entity) else {
            continue;
        };
        let Ok((
            player_entity,
            player_transform,
            mut player_health,
            player_dodge,
            equipment,
            mut barriers,
            mut death_prevention,
        )) = player_query.get_mut(target)
        else {
            continue;
        };

        // Direction from enemy to player.
        let mut to_player = player_transform.translation - enemy_transform.translation;

        // We only care about horizontal distance.
        to_player.y = 0.0;

        let distance = to_player.length();
        let direction = to_player.normalize_or_zero();

        match combat.state {
            // =================================================
            // CHASE
            // =================================================
            EnemyState::Chase => {
                let grounded_safe = enemy_grounded_path_is_safe(
                    entity,
                    enemy_transform.translation,
                    to_player,
                    &spatial_query,
                );
                let clear_of_obstacles = navigation.is_on_traversal_surface()
                    || navigation_grid
                        .segment_avoids_permanent_obstacles(enemy_transform.translation, to_player);
                let world_clear = enemy_world_path_is_clear_ignoring(
                    entity,
                    Some(player_entity),
                    enemy_transform.translation,
                    to_player,
                    &spatial_query,
                );
                let player_is_reachable = grounded_safe && clear_of_obstacles && world_clear;

                if !navigation.traversal_combat_is_locked()
                    && distance <= movement.stop_distance
                    && player_is_reachable
                {
                    // The Footman has no stationary windup. Lock its current
                    // direction and begin the short forward attack immediately.
                    combat.attack_direction = direction;
                    combat.state = EnemyState::Attack;
                    combat.timer = ENEMY_ATTACK_DURATION;
                    combat.has_hit = false;
                    combat.last_committed_position = enemy_transform.translation;
                    combat.committed_stall_timer = 0.0;

                    println!("Enemy: ATTACK");

                    continue;
                }

                // Move toward player. A Black Banner Knight's Corrupted
                // Banner aura (see enemies::aura / enemies::knight) may be
                // buffing this Footman's speed right now, and Widow's
                // Kiss's Envenomed (see weapons::daggers) may be slowing
                // it; 1.0 (no change) for either when it isn't active.
                let effective_speed = movement.speed
                    * enemy_movement_speed_multiplier(enemy_buffs)
                    * enemy_debuff_movement_speed_multiplier(enemy_debuffs);
                let (desired, navmesh_route_available) = navmesh_pursuit_step(
                    desired_velocity,
                    agent_state,
                    route_cache,
                    enemy_transform.translation,
                    direction,
                    effective_speed,
                    delta,
                );
                let step = safe_enemy_locomotion(
                    entity,
                    enemy_transform.translation,
                    player_transform.translation,
                    Some(player_entity),
                    desired,
                    navmesh_route_available,
                    &mut navigation,
                    &spatial_query,
                    &mut navigation_grid,
                    delta,
                );
                enemy_transform.translation += step;

                // Face the direction actually selected by navigation. During
                // a corner detour this differs from the direct player vector;
                // facing the player made correct waypoint movement appear as
                // sideways or backward walking.
                let movement_direction = step.normalize_or_zero();
                if movement_direction.length_squared() > 0.0 {
                    let target_rotation =
                        Quat::from_rotation_y(movement_direction.x.atan2(movement_direction.z));

                    enemy_transform.rotation =
                        enemy_transform.rotation.slerp(target_rotation, 8.0 * delta);
                }
            }

            // =================================================
            // ATTACK
            // =================================================
            EnemyState::Attack => {
                let attack_progress = enemy_transform
                    .translation
                    .distance(combat.last_committed_position);
                if attack_progress <= 0.01 {
                    combat.committed_stall_timer += delta;
                } else {
                    combat.committed_stall_timer = 0.0;
                }
                combat.last_committed_position = enemy_transform.translation;

                if combat.committed_stall_timer >= MELEE_ATTACK_STALL_TIMEOUT {
                    combat.state = EnemyState::Recovery;
                    combat.timer = ENEMY_RECOVERY_DURATION;
                    combat.has_hit = false;
                    combat.committed_stall_timer = 0.0;
                    println!("Enemy: attack cancelled (blocked)");
                    continue;
                }

                let attack_step = combat.attack_direction * ENEMY_ATTACK_SPEED * delta;
                if enemy_step_is_safe(
                    entity,
                    enemy_transform.translation,
                    attack_step,
                    &spatial_query,
                ) {
                    enemy_transform.translation += attack_step;
                } else {
                    combat.state = EnemyState::Recovery;
                    combat.timer = ENEMY_RECOVERY_DURATION;
                    combat.has_hit = false;
                    combat.committed_stall_timer = 0.0;
                    continue;
                }

                // Check for a hit only if this attack has not
                // already damaged the player.
                if !combat.has_hit {
                    let mut to_player = player_transform.translation - enemy_transform.translation;

                    to_player.y = 0.0;

                    let distance_to_player = to_player.length();

                    if distance_to_player <= ENEMY_HIT_RANGE {
                        combat.has_hit = true;

                        if player_dodge.is_invulnerable() {
                            println!("Player dodged the attack!");
                        } else {
                            let result = apply_player_damage(
                                &mut player_health,
                                equipment,
                                Some(&mut barriers),
                                Some(&mut death_prevention),
                                ENEMY_ATTACK_DAMAGE * enemy_damage_multiplier(enemy_buffs),
                            );

                            println!(
                                "Player hit for {:.1} ({:.1} blocked)! HP: {:.0}/{:.0}",
                                result.applied,
                                result.blocked,
                                player_health.current,
                                player_health.max,
                            );
                        }
                    }
                }

                combat.timer -= delta;

                if combat.timer <= 0.0 {
                    combat.state = EnemyState::Recovery;
                    combat.timer = ENEMY_RECOVERY_DURATION;
                    combat.committed_stall_timer = 0.0;

                    println!("Enemy: RECOVERY");
                }
            }

            // =================================================
            // RECOVERY
            // =================================================
            EnemyState::Recovery => {
                // Enemy cannot move or attack during recovery.
                combat.timer -= delta;

                if combat.timer <= 0.0 {
                    combat.state = EnemyState::Chase;

                    println!("Enemy: CHASE");
                }
            }
        }
    }
}
