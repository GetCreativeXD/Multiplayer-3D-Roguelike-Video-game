use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_landmass::{AgentDesiredVelocity3d, prelude::AgentState};

use crate::combat::{
    Barriers, DeathPrevention, EnemyBuffs, EnemyDebuffs, Health, apply_player_damage,
    enemy_damage_multiplier, enemy_debuff_movement_speed_multiplier,
    enemy_movement_speed_multiplier,
};
use crate::loot::LootTableId;
use crate::navigation_mesh::{NavmeshRouteCache, navmesh_pursuit_step};
use crate::player::{Dodge, PlayerCharacter};
use crate::weapons::Equipment;

use super::targeting::{EnemyTarget, current_target};
use super::{
    CommittedTraversalPolicy, Enemy, EnemyAwareness, EnemyBaseConfig, EnemyCombat,
    EnemyEncounterConfig, EnemyGroundSafety, EnemyKind, EnemyMovement, EnemyNavigation,
    EnemyNavigationGrid, Stagger, authorize_enemy_edge_traversal, enemy_committed_step_is_safe,
    enemy_grounded_path_is_safe, enemy_landing_is_safe, enemy_world_path_is_clear,
    finish_enemy_edge_traversal, safe_enemy_locomotion, spawn_enemy_root, visuals,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FlankingLungeState {
    Pursue,
    Lunge,
}

#[derive(Component)]
pub struct FlankingLungeCombat {
    pub state: FlankingLungeState,
    pub timer: f32,
    pub lunge_range: f32,
    pub lunge_speed: f32,
    pub lunge_duration: f32,
    pub cooldown: f32,
    pub cooldown_remaining: f32,
    pub hit_range: f32,
    pub damage: f32,
    pub locked_direction: Vec3,
    pub has_hit: bool,
    pub last_lunge_position: Vec3,
    pub stall_timer: f32,
    pub stall_timeout: f32,
}

pub fn spawn_cinder_hound(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    encounter: EnemyEncounterConfig,
) -> Entity {
    let enemy = spawn_enemy_root(
        commands,
        position,
        encounter,
        EnemyBaseConfig {
            kind: EnemyKind::CinderHound,
            name: "Cinder Hound",
            health: 35.0,
            movement_speed: 5.0,
            stop_distance: 5.0,
            stagger_resistance: 0,
            loot_table: LootTableId::NORMAL_ENEMY,
            collider_radius: 0.38,
            collider_height: 0.65,
        },
    );

    commands.entity(enemy).insert(FlankingLungeCombat {
        state: FlankingLungeState::Pursue,
        timer: 0.0,
        lunge_range: 7.0,
        lunge_speed: 15.0,
        lunge_duration: 0.45,
        cooldown: 3.0,
        cooldown_remaining: 0.0,
        hit_range: 1.25,
        damage: 8.0,
        locked_direction: Vec3::ZERO,
        has_hit: false,
        last_lunge_position: position,
        stall_timer: 0.0,
        stall_timeout: 0.2,
    });
    visuals::attach_cinder_hound_visual(commands, enemy, meshes, materials);
    enemy
}

pub fn flanking_lunge_combat(
    time: Res<Time>,
    spatial_query: SpatialQuery,
    mut navigation_grid: ResMut<EnemyNavigationGrid>,
    mut player: Query<
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
    mut enemies: Query<
        (
            Entity,
            &mut Transform,
            &EnemyMovement,
            &mut EnemyNavigation,
            &mut EnemyCombat,
            &Stagger,
            &EnemyAwareness,
            &mut EnemyGroundSafety,
            &mut FlankingLungeCombat,
            Option<&EnemyBuffs>,
            Option<&EnemyDebuffs>,
            Option<&AgentDesiredVelocity3d>,
            Option<&AgentState>,
            Option<&NavmeshRouteCache>,
        ),
        (With<Enemy>, Without<PlayerCharacter>),
    >,
) {
    let delta = time.delta_secs();

    for (
        entity,
        mut transform,
        movement,
        mut navigation,
        mut common,
        stagger,
        awareness,
        mut safety,
        mut lunge,
        enemy_buffs,
        enemy_debuffs,
        desired_velocity,
        agent_state,
        route_cache,
    ) in &mut enemies
    {
        if !awareness.engaged {
            finish_enemy_edge_traversal(&mut safety);
            lunge.state = FlankingLungeState::Pursue;
            lunge.timer = 0.0;
            lunge.stall_timer = 0.0;
            continue;
        }
        if stagger.is_staggered() {
            finish_enemy_edge_traversal(&mut safety);
            lunge.state = FlankingLungeState::Pursue;
            lunge.timer = 0.0;
            lunge.stall_timer = 0.0;
            continue;
        }

        // The player this Hound is after (see `enemies::targeting`).
        let Some(target) = current_target(&targets, entity) else {
            continue;
        };
        let Ok((
            player_entity,
            player_transform,
            mut player_health,
            dodge,
            equipment,
            mut barriers,
            mut death_prevention,
        )) = player.get_mut(target)
        else {
            continue;
        };

        lunge.cooldown_remaining = (lunge.cooldown_remaining - delta).max(0.0);

        let mut to_player = player_transform.translation - transform.translation;
        to_player.y = 0.0;
        let distance = to_player.length();
        let direction = to_player.normalize_or_zero();

        // A Black Banner Knight's Corrupted Banner aura (see enemies::aura
        // / enemies::knight) may be buffing this Hound's speed and lunge
        // damage right now, and Widow's Kiss's Envenomed (see
        // weapons::daggers) may be slowing it; 1.0 (no change) for either
        // when it isn't active.
        let effective_speed = movement.speed
            * enemy_movement_speed_multiplier(enemy_buffs)
            * enemy_debuff_movement_speed_multiplier(enemy_debuffs);

        match lunge.state {
            FlankingLungeState::Pursue => {
                let (desired, navmesh_route_available) = navmesh_pursuit_step(
                    desired_velocity,
                    agent_state,
                    route_cache,
                    transform.translation,
                    direction,
                    effective_speed,
                    delta,
                );
                let step = safe_enemy_locomotion(
                    entity,
                    transform.translation,
                    player_transform.translation,
                    Some(player_entity),
                    desired,
                    navmesh_route_available,
                    &mut navigation,
                    &spatial_query,
                    &mut navigation_grid,
                    delta,
                );
                transform.translation += step;
                face_direction(&mut transform, step.normalize_or_zero(), delta);

                let maximum_pounce_distance = lunge.lunge_speed * lunge.lunge_duration;
                let pounce_distance = distance.min(maximum_pounce_distance);
                let proposed_landing = transform.translation + direction * pounce_distance;
                if !navigation.traversal_combat_is_locked()
                    && lunge.cooldown_remaining <= 0.0
                    && distance > lunge.hit_range
                    && distance <= lunge.lunge_range
                    && enemy_grounded_path_is_safe(
                        entity,
                        transform.translation,
                        direction * pounce_distance,
                        &spatial_query,
                    )
                    && (navigation.is_on_traversal_surface()
                        || navigation_grid.segment_avoids_permanent_obstacles(
                            transform.translation,
                            direction * pounce_distance,
                        ))
                    && enemy_world_path_is_clear(
                        entity,
                        transform.translation,
                        direction * pounce_distance,
                        &spatial_query,
                    )
                    && enemy_landing_is_safe(entity, proposed_landing, &spatial_query)
                {
                    lunge.locked_direction = direction;
                    lunge.has_hit = false;
                    if authorize_enemy_edge_traversal(
                        entity,
                        proposed_landing,
                        &mut safety,
                        &spatial_query,
                    ) {
                        lunge.state = FlankingLungeState::Lunge;
                        lunge.timer = pounce_distance / lunge.lunge_speed;
                        lunge.cooldown_remaining = lunge.cooldown;
                        lunge.last_lunge_position = transform.translation;
                        lunge.stall_timer = 0.0;
                        println!("Cinder Hound: LUNGE");
                    }
                }
            }
            FlankingLungeState::Lunge => {
                let pounce_progress = transform.translation.distance(lunge.last_lunge_position);
                if pounce_progress <= 0.01 {
                    lunge.stall_timer += delta;
                } else {
                    lunge.stall_timer = 0.0;
                }
                lunge.last_lunge_position = transform.translation;

                if lunge.stall_timer >= lunge.stall_timeout {
                    finish_enemy_edge_traversal(&mut safety);
                    lunge.state = FlankingLungeState::Pursue;
                    lunge.timer = 0.0;
                    lunge.stall_timer = 0.0;
                    println!("Cinder Hound: pounce cancelled (blocked)");
                    common.has_hit = lunge.has_hit;
                    continue;
                }

                let locked_direction = lunge.locked_direction;
                let movement_time = delta.min(lunge.timer.max(0.0));
                let pounce_step = locked_direction * lunge.lunge_speed * movement_time;
                if !enemy_committed_step_is_safe(
                    entity,
                    transform.translation,
                    pounce_step,
                    CommittedTraversalPolicy::Grounded,
                    &spatial_query,
                ) {
                    finish_enemy_edge_traversal(&mut safety);
                    lunge.state = FlankingLungeState::Pursue;
                    lunge.timer = 0.0;
                    lunge.stall_timer = 0.0;
                    common.has_hit = lunge.has_hit;
                    println!("Cinder Hound: pounce cancelled (world obstacle)");
                    continue;
                }
                transform.translation += pounce_step;
                face_direction(&mut transform, locked_direction, delta);

                if !lunge.has_hit {
                    let mut current_offset = player_transform.translation - transform.translation;
                    current_offset.y = 0.0;
                    if current_offset.length() <= lunge.hit_range {
                        lunge.has_hit = true;
                        if dodge.is_invulnerable() {
                            println!("Player dodged the Cinder Hound!");
                        } else {
                            let result = apply_player_damage(
                                &mut player_health,
                                equipment,
                                Some(&mut barriers),
                                Some(&mut death_prevention),
                                lunge.damage * enemy_damage_multiplier(enemy_buffs),
                            );
                            println!(
                                "Cinder Hound hit for {:.1} ({:.1} blocked)! HP: {:.0}/{:.0}",
                                result.applied,
                                result.blocked,
                                player_health.current,
                                player_health.max,
                            );
                        }
                    }
                }

                lunge.timer -= delta;
                if lunge.timer <= 0.0 {
                    finish_enemy_edge_traversal(&mut safety);
                    lunge.state = FlankingLungeState::Pursue;
                    lunge.timer = 0.0;
                    lunge.stall_timer = 0.0;
                }
            }
        }

        common.has_hit = lunge.has_hit;
    }
}

fn face_direction(transform: &mut Transform, direction: Vec3, delta: f32) {
    if direction == Vec3::ZERO {
        return;
    }
    let target = Quat::from_rotation_y(direction.x.atan2(direction.z));
    transform.rotation = transform.rotation.slerp(target, 12.0 * delta);
}
