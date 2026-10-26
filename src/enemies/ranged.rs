use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_landmass::{AgentDesiredVelocity3d, prelude::AgentState};

use crate::combat::{
    EnemyBuffs, EnemyDebuffs, enemy_damage_multiplier, enemy_debuff_movement_speed_multiplier,
    enemy_movement_speed_multiplier,
};
use crate::loot::LootTableId;
use crate::navigation_mesh::{NavmeshRouteCache, navmesh_pursuit_step};
use crate::player::PlayerCharacter;

use super::hostile_projectiles::spawn_hostile_projectile;
use super::targeting::{EnemyTarget, current_target};
use super::{
    Enemy, EnemyAwareness, EnemyBaseConfig, EnemyCombat, EnemyEncounterConfig, EnemyKind,
    EnemyMovement, EnemyNavigation, EnemyNavigationGrid, Stagger, safe_enemy_locomotion,
    spawn_enemy_root, visuals,
};

/// Height above a Marksman's origin its arrow (and aim line) starts from.
const AIM_ORIGIN_HEIGHT: f32 = 0.75;
/// Height above a player's origin the Marksman aims at.
const AIM_TARGET_HEIGHT: f32 = 0.65;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RangedState {
    Positioning,
    Windup,
    Recovery,
}

#[derive(Component)]
pub struct RangedEnemyCombat {
    pub state: RangedState,
    pub timer: f32,
    pub min_range: f32,
    pub preferred_range: f32,
    pub max_range: f32,
    pub windup: f32,
    pub recovery: f32,
    pub projectile_speed: f32,
    pub projectile_damage: f32,
}

#[derive(Component)]
pub struct RangedAimTelegraph {
    owner: Entity,
}

pub fn spawn_gatebow_marksman(
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
            kind: EnemyKind::GatebowMarksman,
            name: "Gatebow Marksman",
            health: 45.0,
            movement_speed: 3.4,
            stop_distance: 14.0,
            stagger_resistance: 0,
            loot_table: LootTableId::NORMAL_ENEMY,
            collider_radius: 0.40,
            collider_height: 0.95,
        },
    );

    commands.entity(enemy).insert(RangedEnemyCombat {
        state: RangedState::Positioning,
        timer: 0.0,
        min_range: 6.0,
        preferred_range: 20.0,
        max_range: 32.0,
        windup: 0.85,
        recovery: 1.2,
        projectile_speed: 45.0,
        projectile_damage: 9.0,
    });
    visuals::attach_marksman_visual(commands, enemy, meshes, materials);
    enemy
}

pub fn ranged_enemy_combat(
    mut commands: Commands,
    time: Res<Time>,
    spatial_query: SpatialQuery,
    mut navigation_grid: ResMut<EnemyNavigationGrid>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player: Query<(Entity, &Transform), (With<PlayerCharacter>, Without<Enemy>)>,
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
            &mut RangedEnemyCombat,
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
        mut common_combat,
        stagger,
        awareness,
        mut ranged,
        enemy_buffs,
        enemy_debuffs,
        desired_velocity,
        agent_state,
        route_cache,
    ) in &mut enemies
    {
        if !awareness.engaged || stagger.is_staggered() {
            ranged.state = RangedState::Positioning;
            continue;
        }

        // The player this Marksman is after (see `enemies::targeting`).
        let Some((player_entity, player_transform)) =
            current_target(&targets, entity).and_then(|target| player.get(target).ok())
        else {
            continue;
        };

        let mut offset = player_transform.translation - transform.translation;
        offset.y = 0.0;
        let distance = offset.length();
        let direction = offset.normalize_or_zero();

        if direction != Vec3::ZERO {
            let rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
            transform.rotation = transform.rotation.slerp(rotation, 8.0 * delta);
        }

        // A Black Banner Knight's Corrupted Banner aura (see enemies::aura
        // / enemies::knight) may be buffing this Marksman's speed and
        // arrow damage right now, and Widow's Kiss's Envenomed (see
        // weapons::daggers) may be slowing it; 1.0 (no change) for either
        // when it isn't active.
        let effective_speed = movement.speed
            * enemy_movement_speed_multiplier(enemy_buffs)
            * enemy_debuff_movement_speed_multiplier(enemy_debuffs);

        match ranged.state {
            RangedState::Positioning => {
                let has_line_of_sight = line_of_sight(
                    entity,
                    player_entity,
                    transform.translation + Vec3::Y * AIM_ORIGIN_HEIGHT,
                    player_transform.translation + Vec3::Y * AIM_TARGET_HEIGHT,
                    &spatial_query,
                );

                if navigation.traversal_combat_is_locked()
                    || distance > ranged.max_range
                    || !has_line_of_sight
                {
                    let (desired, navmesh_route_available) = navmesh_pursuit_step(
                        desired_velocity,
                        agent_state,
                        route_cache,
                        transform.translation,
                        direction,
                        effective_speed,
                        delta,
                    );
                    let position = transform.translation;
                    let step = safe_enemy_locomotion(
                        entity,
                        position,
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
                } else if distance < ranged.min_range {
                    let desired = -direction * effective_speed * delta;
                    let position = transform.translation;
                    let retreat_destination = position - direction * ranged.preferred_range;
                    let step = safe_enemy_locomotion(
                        entity,
                        position,
                        retreat_destination,
                        None,
                        desired,
                        false,
                        &mut navigation,
                        &spatial_query,
                        &mut navigation_grid,
                        delta,
                    );
                    transform.translation += step;
                } else {
                    ranged.state = RangedState::Windup;
                    ranged.timer = ranged.windup;
                    spawn_aim_telegraph(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        entity,
                        transform.translation,
                        player_transform.translation,
                    );
                    println!("Gatebow Marksman: AIMING");
                }
            }
            RangedState::Windup => {
                ranged.timer -= delta;
                if ranged.timer <= 0.0 {
                    let origin = transform.translation + Vec3::Y * AIM_ORIGIN_HEIGHT;
                    let target = player_transform.translation + Vec3::Y * AIM_TARGET_HEIGHT;
                    spawn_hostile_projectile(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        entity,
                        origin,
                        target - origin,
                        ranged.projectile_speed,
                        ranged.projectile_damage * enemy_damage_multiplier(enemy_buffs),
                        3.0,
                        0.12,
                        Color::srgb(1.0, 0.45, 0.08),
                    );
                    ranged.state = RangedState::Recovery;
                    ranged.timer = ranged.recovery;
                    common_combat.has_hit = false;
                    println!("Gatebow Marksman: FIRE");
                }
            }
            RangedState::Recovery => {
                if distance > ranged.preferred_range {
                    let desired = direction * effective_speed * 0.45 * delta;
                    let position = transform.translation;
                    let step = safe_enemy_locomotion(
                        entity,
                        position,
                        player_transform.translation,
                        Some(player_entity),
                        desired,
                        false,
                        &mut navigation,
                        &spatial_query,
                        &mut navigation_grid,
                        delta,
                    );
                    transform.translation += step;
                }
                ranged.timer -= delta;
                if ranged.timer <= 0.0 {
                    ranged.state = RangedState::Positioning;
                }
            }
        }
    }
}

fn spawn_aim_telegraph(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    owner: Entity,
    shooter_position: Vec3,
    target_position: Vec3,
) {
    let Some(transform) = aim_line_transform(shooter_position, target_position) else {
        return;
    };

    commands.spawn((
        RangedAimTelegraph { owner },
        aim_line_visual(meshes, materials),
        transform,
    ));
}

/// The thin red aim line, one unit long along its local Z. Shared by the
/// host and, in multiplayer, a client's copy (`net::telegraphs`).
pub fn aim_line_visual(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> impl Bundle + use<> {
    (
        Mesh3d(meshes.add(Cuboid::new(0.035, 0.035, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.18, 0.03, 0.75),
            emissive: LinearRgba::new(1.0, 0.04, 0.01, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
    )
}

/// Where the aim line sits for a Marksman at `shooter_position` aiming at a
/// player at `target_position` (both are the characters' origins). `None`
/// when they overlap.
pub fn aim_line_transform(shooter_position: Vec3, target_position: Vec3) -> Option<Transform> {
    let origin = shooter_position + Vec3::Y * AIM_ORIGIN_HEIGHT;
    let target = target_position + Vec3::Y * AIM_TARGET_HEIGHT;
    let offset = target - origin;
    let distance = offset.length();
    let direction = offset.normalize_or_zero();
    if direction == Vec3::ZERO {
        return None;
    }

    Some(Transform {
        translation: origin + offset * 0.5,
        rotation: Quat::from_rotation_arc(Vec3::Z, direction),
        scale: Vec3::new(1.0, 1.0, distance),
    })
}

/// The player a Marksman is drawing its aim line at: its current target,
/// while it is winding up a shot. The same rule `update_aim_telegraphs`
/// uses, so multiplayer can show the line on clients (`net::telegraphs`).
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
pub fn aim_line_target(combat: &RangedEnemyCombat, target: &EnemyTarget) -> Option<Entity> {
    (combat.state == RangedState::Windup)
        .then_some(target.entity)
        .flatten()
}

/// Keeps each aim line pointed at its Marksman's *current* target, which is
/// also who the arrow will be fired at.
pub fn update_aim_telegraphs(
    mut commands: Commands,
    player: Query<&Transform, (With<PlayerCharacter>, Without<Enemy>)>,
    owners: Query<(&Transform, &RangedEnemyCombat, &EnemyTarget), With<Enemy>>,
    mut telegraphs: Query<
        (Entity, &RangedAimTelegraph, &mut Transform),
        (Without<Enemy>, Without<PlayerCharacter>),
    >,
) {
    for (entity, telegraph, mut transform) in &mut telegraphs {
        let Ok((owner_transform, combat, owner_target)) = owners.get(telegraph.owner) else {
            commands.entity(entity).despawn();
            continue;
        };

        if combat.state != RangedState::Windup {
            commands.entity(entity).despawn();
            continue;
        }

        let Some(player_transform) = owner_target
            .entity
            .and_then(|target| player.get(target).ok())
        else {
            continue;
        };

        if let Some(line) =
            aim_line_transform(owner_transform.translation, player_transform.translation)
        {
            *transform = line;
        }
    }
}

fn line_of_sight(
    enemy: Entity,
    player: Entity,
    origin: Vec3,
    target: Vec3,
    spatial_query: &SpatialQuery,
) -> bool {
    let offset = target - origin;
    let distance = offset.length();
    let Ok(direction) = Dir3::new(offset.normalize_or_zero().adjust_precision()) else {
        return false;
    };
    let filter = SpatialQueryFilter::from_excluded_entities([enemy]);

    spatial_query
        .cast_ray(
            origin.adjust_precision(),
            direction,
            distance,
            true,
            &filter,
        )
        .is_some_and(|hit| hit.entity == player)
}
