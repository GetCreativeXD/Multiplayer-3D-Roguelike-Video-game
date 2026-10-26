use std::time::Duration;

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use bevy_landmass::{AgentDesiredVelocity3d, prelude::AgentState};

use crate::combat::{Barriers, DeathPrevention, Health, apply_player_damage};
use crate::loot::LootTableId;
use crate::navigation_mesh::{NavmeshRouteCache, navmesh_pursuit_step};
use crate::player::{Dodge, PlayerCharacter, VerticalMovement};
use crate::weapons::Equipment;

use super::targeting::{EnemyTarget, current_target};
use super::telegraph::{EnemyTelegraph, EnemyTelegraphKind};
use super::{
    Enemy, EnemyAwareness, EnemyBaseConfig, EnemyEncounterConfig, EnemyKind, EnemyMovement,
    EnemyNavigation, EnemyNavigationGrid, safe_enemy_locomotion, spawn_enemy_root,
};

pub const SAINT_MODEL_PATH: &str = "bosses/saint_broken_siege_rigged_v1.glb";

const IMPACT_ANIMATION: usize = 0;
const RECOVERY_ANIMATION: usize = 1;
const WINDUP_ANIMATION: usize = 2;
const IDLE_ANIMATION: usize = 3;
const WALK_ANIMATION: usize = 4;

const VISUAL_ROOT_OFFSET: f32 = 2.7;
// Trigger distance for Fist Slam. Kept below SHOCKWAVE_MAX_RADIUS (17.0) so
// the wave always has room to visibly travel and expand before it could
// reach the player, rather than firing from a distance the wave can't cover.
const SLAM_RANGE: f32 = 12.0;
const SLAM_WINDUP: f32 = 0.8;
const SLAM_IMPACT_DURATION: f32 = 0.20;
const SLAM_RECOVERY: f32 = 1.15;
const SLAM_COOLDOWN: f32 = 1.0;
const IMPACT_RADIUS: f32 = 3.2;
const IMPACT_DAMAGE: f32 = 22.0;
const SHOCKWAVE_DAMAGE: f32 = 14.0;
const SHOCKWAVE_SPEED: f32 = 11.0;
const SHOCKWAVE_MAX_RADIUS: f32 = 17.0;
const SHOCKWAVE_START_RADIUS: f32 = 0.8;

#[derive(Component)]
pub struct SaintOfTheBrokenSiege;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
// Multiplayer sends the current clip to clients (`SaintAnimationCue`).
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum SaintAnimationState {
    #[default]
    Idle,
    Walk,
    FistSlamWindup,
    FistSlamImpact,
    FistSlamRecovery,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SaintCombatState {
    #[default]
    Approach,
    FistSlamWindup,
    FistSlamImpact,
    FistSlamRecovery,
}

#[derive(Component)]
pub struct SaintBossCombat {
    pub state: SaintCombatState,
    pub timer: f32,
    pub cooldown: f32,
    pub animation: SaintAnimationState,
    impact_position: Vec3,
    impact_applied: bool,
}

/// Which clip the Saint's model should be playing. Lives on the boss root.
/// The host's combat sets it (`publish_saint_animation_cue`, from
/// `SaintBossCombat::animation`); in multiplayer it is replicated, so a
/// client's copy of the boss plays the same clips. `update_saint_animation`
/// reads only this.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub struct SaintAnimationCue(pub SaintAnimationState);

#[derive(Component)]
pub(super) struct SaintAnimationOwner(Entity);

#[derive(Component)]
pub(super) struct SaintAnimationController {
    current: SaintAnimationState,
}

#[derive(Resource)]
pub struct SaintAnimations {
    graph: Handle<AnimationGraph>,
    idle: AnimationNodeIndex,
    walk: AnimationNodeIndex,
    windup: AnimationNodeIndex,
    impact: AnimationNodeIndex,
    recovery: AnimationNodeIndex,
}

#[derive(Component)]
pub(super) struct SaintSlamTelegraph {
    owner: Entity,
    lifetime: f32,
    total_lifetime: f32,
}

#[derive(Component)]
pub(super) struct SaintShockwave {
    owner: Entity,
    previous_radius: f32,
    radius: f32,
    maximum_radius: f32,
    speed: f32,
    damage: f32,
    /// Players the leading edge has already crossed. Each player can be hit
    /// by a given shockwave at most once (jumping it still counts as crossed).
    hit_players: Vec<Entity>,
}

pub fn setup_saint_animations(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // GLB action order was verified from the exported file's animation names.
    let clips = [
        asset_server.load(GltfAssetLabel::Animation(IDLE_ANIMATION).from_asset(SAINT_MODEL_PATH)),
        asset_server.load(GltfAssetLabel::Animation(WALK_ANIMATION).from_asset(SAINT_MODEL_PATH)),
        asset_server.load(GltfAssetLabel::Animation(WINDUP_ANIMATION).from_asset(SAINT_MODEL_PATH)),
        asset_server.load(GltfAssetLabel::Animation(IMPACT_ANIMATION).from_asset(SAINT_MODEL_PATH)),
        asset_server
            .load(GltfAssetLabel::Animation(RECOVERY_ANIMATION).from_asset(SAINT_MODEL_PATH)),
    ];
    let (graph, nodes) = AnimationGraph::from_clips(clips);
    commands.insert_resource(SaintAnimations {
        graph: graphs.add(graph),
        idle: nodes[0],
        walk: nodes[1],
        windup: nodes[2],
        impact: nodes[3],
        recovery: nodes[4],
    });
}

/// Builds the boss and its animated visual. Called by
/// `rooms::saint_arena::spawn_saint_arena` (the connected-chain integration)
/// and, separately, by the standalone `arena::ArenaPlugin` test harness.
pub fn spawn_saint_of_the_broken_siege(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
    encounter: EnemyEncounterConfig,
) -> Entity {
    let boss = spawn_enemy_root(
        commands,
        position,
        encounter,
        EnemyBaseConfig {
            kind: EnemyKind::SaintOfTheBrokenSiege,
            name: "Saint of the Broken Siege",
            health: 1600.0,
            movement_speed: 2.15,
            stop_distance: 4.6,
            stagger_resistance: 6,
            loot_table: LootTableId::BOSS,
            collider_radius: 1.2,
            collider_height: 3.0,
        },
    );

    commands.entity(boss).insert((
        SaintOfTheBrokenSiege,
        SaintBossCombat {
            state: SaintCombatState::Approach,
            timer: 0.0,
            cooldown: 0.0,
            animation: SaintAnimationState::Idle,
            impact_position: position - Vec3::Y * VISUAL_ROOT_OFFSET,
            impact_applied: false,
        },
        SaintAnimationCue::default(),
    ));

    attach_saint_animated_visual(commands, boss, asset_server);
    boss
}

/// The Saint's animated model, driven by the `SaintAnimationCue` on `boss`.
/// Used for the host's boss and, in multiplayer, for a client's copy of it
/// (`enemies::attach_enemy_visual`), whose cue is replicated from the host.
pub(super) fn attach_saint_animated_visual(
    commands: &mut Commands,
    boss: Entity,
    asset_server: &AssetServer,
) {
    let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(SAINT_MODEL_PATH));
    commands.entity(boss).with_children(|parent| {
        parent
            .spawn((
                Name::new("Saint Animated Visual"),
                SaintAnimationOwner(boss),
                WorldAssetRoot(scene),
                Transform::from_translation(Vec3::NEG_Y * VISUAL_ROOT_OFFSET),
            ))
            .observe(initialize_saint_animation_player);
    });
}

/// Copies the clip the combat state machine wants into `SaintAnimationCue`,
/// only when it changes (a write is what gets it re-sent to clients).
pub fn publish_saint_animation_cue(mut bosses: Query<(&SaintBossCombat, &mut SaintAnimationCue)>) {
    for (combat, mut cue) in &mut bosses {
        cue.set_if_neq(SaintAnimationCue(combat.animation));
    }
}

pub fn initialize_saint_animation_player(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    owners: Query<&SaintAnimationOwner>,
    mut players: Query<&mut AnimationPlayer>,
    animations: Res<SaintAnimations>,
) {
    let Ok(owner) = owners.get(scene_ready.entity) else {
        return;
    };
    for descendant in children.iter_descendants(scene_ready.entity) {
        let Ok(mut player) = players.get_mut(descendant) else {
            continue;
        };
        let mut transitions = AnimationTransitions::new();
        transitions
            .play(&mut player, animations.idle, Duration::ZERO)
            .repeat();
        commands.entity(descendant).insert((
            AnimationGraphHandle(animations.graph.clone()),
            transitions,
            SaintAnimationOwner(owner.0),
            SaintAnimationController {
                current: SaintAnimationState::Idle,
            },
        ));
    }
}

pub fn update_saint_animation(
    animations: Res<SaintAnimations>,
    bosses: Query<&SaintAnimationCue>,
    mut players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &SaintAnimationOwner,
        &mut SaintAnimationController,
    )>,
) {
    for (mut player, mut transitions, owner, mut controller) in &mut players {
        let Ok(&SaintAnimationCue(animation)) = bosses.get(owner.0) else {
            continue;
        };
        if animation == controller.current {
            continue;
        }
        let node = match animation {
            SaintAnimationState::Idle => animations.idle,
            SaintAnimationState::Walk => animations.walk,
            SaintAnimationState::FistSlamWindup => animations.windup,
            SaintAnimationState::FistSlamImpact => animations.impact,
            SaintAnimationState::FistSlamRecovery => animations.recovery,
        };
        let blend = match animation {
            SaintAnimationState::FistSlamImpact => Duration::from_millis(35),
            _ => Duration::from_millis(120),
        };
        let active = transitions.play(&mut player, node, blend);
        if matches!(
            animation,
            SaintAnimationState::Idle | SaintAnimationState::Walk
        ) {
            active.repeat();
        }
        controller.current = animation;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn saint_fist_slam_combat(
    mut commands: Commands,
    time: Res<Time>,
    spatial_query: avian3d::prelude::SpatialQuery,
    mut grid: ResMut<EnemyNavigationGrid>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
    mut bosses: Query<
        (
            Entity,
            &mut Transform,
            &EnemyMovement,
            &mut EnemyNavigation,
            &EnemyAwareness,
            &mut SaintBossCombat,
            Option<&AgentDesiredVelocity3d>,
            Option<&AgentState>,
            Option<&NavmeshRouteCache>,
        ),
        (With<SaintOfTheBrokenSiege>, Without<PlayerCharacter>),
    >,
) {
    let delta = time.delta_secs();

    for (
        entity,
        mut transform,
        movement,
        mut navigation,
        awareness,
        mut combat,
        desired_velocity,
        agent_state,
        route_cache,
    ) in &mut bosses
    {
        if !awareness.engaged {
            combat.state = SaintCombatState::Approach;
            combat.animation = SaintAnimationState::Idle;
            combat.timer = 0.0;
            continue;
        }

        // The player the Saint is walking toward and aiming at (see
        // `enemies::targeting`). The slam itself hits every player in range.
        let Some((player_entity, player_position)) = current_target(&targets, entity)
            .and_then(|target| player.get(target).ok())
            .map(|(player_entity, player_transform, ..)| {
                (player_entity, player_transform.translation)
            })
        else {
            continue;
        };

        combat.cooldown = (combat.cooldown - delta).max(0.0);
        let mut to_player = player_position - transform.translation;
        to_player.y = 0.0;
        let distance = to_player.length();
        let direction = to_player.normalize_or_zero();
        if direction != Vec3::ZERO {
            let target_rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
            transform.rotation = transform.rotation.slerp(target_rotation, 3.2 * delta);
        }

        match combat.state {
            SaintCombatState::Approach => {
                if distance > SLAM_RANGE || combat.cooldown > 0.0 {
                    combat.animation = if distance > SLAM_RANGE {
                        SaintAnimationState::Walk
                    } else {
                        SaintAnimationState::Idle
                    };
                    if distance > SLAM_RANGE {
                        let (desired, shared_route) = navmesh_pursuit_step(
                            desired_velocity,
                            agent_state,
                            route_cache,
                            transform.translation,
                            direction,
                            movement.speed,
                            delta,
                        );
                        let position = transform.translation;
                        let step = safe_enemy_locomotion(
                            entity,
                            position,
                            player_position,
                            Some(player_entity),
                            desired,
                            shared_route,
                            &mut navigation,
                            &spatial_query,
                            &mut grid,
                            delta,
                        );
                        transform.translation += step;
                    }
                    continue;
                }

                let ground = transform.translation - Vec3::Y * VISUAL_ROOT_OFFSET;
                combat.impact_position = ground + direction * 2.0;
                combat.state = SaintCombatState::FistSlamWindup;
                combat.animation = SaintAnimationState::FistSlamWindup;
                combat.timer = SLAM_WINDUP;
                combat.impact_applied = false;
                spawn_slam_telegraph(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    entity,
                    combat.impact_position,
                );
            }
            SaintCombatState::FistSlamWindup => {
                combat.timer -= delta;
                if combat.timer <= 0.0 {
                    combat.state = SaintCombatState::FistSlamImpact;
                    combat.animation = SaintAnimationState::FistSlamImpact;
                    combat.timer = SLAM_IMPACT_DURATION;
                }
            }
            SaintCombatState::FistSlamImpact => {
                if !combat.impact_applied {
                    combat.impact_applied = true;
                    // Every player inside the direct-impact disk is hit.
                    for (
                        _,
                        player_transform,
                        mut player_health,
                        dodge,
                        equipment,
                        mut barriers,
                        mut death_prevention,
                    ) in &mut player
                    {
                        let mut player_offset =
                            player_transform.translation - combat.impact_position;
                        player_offset.y = 0.0;
                        if player_offset.length() <= IMPACT_RADIUS && !dodge.is_invulnerable() {
                            apply_player_damage(
                                &mut player_health,
                                equipment,
                                Some(&mut barriers),
                                Some(&mut death_prevention),
                                IMPACT_DAMAGE,
                            );
                        }
                    }
                    spawn_shockwave(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        entity,
                        combat.impact_position,
                    );
                }
                combat.timer -= delta;
                if combat.timer <= 0.0 {
                    combat.state = SaintCombatState::FistSlamRecovery;
                    combat.animation = SaintAnimationState::FistSlamRecovery;
                    combat.timer = SLAM_RECOVERY;
                }
            }
            SaintCombatState::FistSlamRecovery => {
                combat.timer -= delta;
                if combat.timer <= 0.0 {
                    combat.state = SaintCombatState::Approach;
                    combat.animation = SaintAnimationState::Idle;
                    combat.cooldown = SLAM_COOLDOWN;
                }
            }
        }
    }
}

fn spawn_slam_telegraph(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    owner: Entity,
    position: Vec3,
) {
    commands.spawn((
        Name::new("Saint Fist Slam Telegraph"),
        SaintSlamTelegraph {
            owner,
            lifetime: SLAM_WINDUP,
            total_lifetime: SLAM_WINDUP,
        },
        EnemyTelegraph {
            kind: EnemyTelegraphKind::SaintSlamDisk,
        },
        slam_telegraph_visual(meshes, materials),
        Transform::from_translation(position + Vec3::Y * 0.05),
    ));
}

/// The pulsing red-orange disk marking the direct-impact area. Shared by
/// the host's telegraph and a client's copy of it (`enemies::telegraph`).
pub(super) fn slam_telegraph_visual(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> impl Bundle + use<> {
    (
        Mesh3d(meshes.add(Cylinder::new(IMPACT_RADIUS, 0.035))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.85, 0.12, 0.03, 0.22),
            emissive: LinearRgba::new(1.8, 0.08, 0.01, 1.0),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
    )
}

/// The slam disk's pulse at `seconds` into the windup.
pub(super) fn slam_telegraph_scale(seconds: f32) -> Vec3 {
    let progress = (seconds / SLAM_WINDUP).clamp(0.0, 1.0);
    let pulse = 0.96 + (progress * std::f32::consts::TAU * 3.0).sin().abs() * 0.08;
    Vec3::new(pulse, 1.0, pulse)
}

/// The shockwave's radius `seconds` after impact. The host's gameplay radius
/// grows the same way (`update_saint_shockwaves`).
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
pub(super) fn shockwave_radius_after(seconds: f32) -> f32 {
    (SHOCKWAVE_START_RADIUS + SHOCKWAVE_SPEED * seconds.max(0.0)).min(SHOCKWAVE_MAX_RADIUS)
}

/// The shockwave's scale at a given radius. The visible crest and the
/// gameplay radius are the same value; a slight vertical pulse keeps the
/// wave readable against bright floors.
pub(super) fn shockwave_scale(radius: f32) -> Vec3 {
    let crest_height = 5.5 + (radius * 0.8).sin().abs() * 2.0;
    Vec3::new(radius, crest_height, radius)
}

fn spawn_shockwave(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    owner: Entity,
    center: Vec3,
) {
    let shockwave = commands
        .spawn((
            Name::new("Saint Ground Shockwave"),
            SaintShockwave {
                owner,
                previous_radius: SHOCKWAVE_START_RADIUS,
                radius: SHOCKWAVE_START_RADIUS,
                maximum_radius: SHOCKWAVE_MAX_RADIUS,
                speed: SHOCKWAVE_SPEED,
                damage: SHOCKWAVE_DAMAGE,
                hit_players: Vec::new(),
            },
            EnemyTelegraph {
                kind: EnemyTelegraphKind::SaintShockwave,
            },
            Transform::from_translation(center + Vec3::Y * 0.12)
                .with_scale(Vec3::new(0.8, 6.0, 0.8)),
            Visibility::default(),
        ))
        .id();
    attach_shockwave_visual(commands, shockwave, meshes, materials);
}

/// The shockwave's three rings (energy crest, hot rim, dust band), as
/// children of `shockwave`, whose scale sets the radius. Shared by the
/// host's shockwave and a client's copy of it (`enemies::telegraph`).
pub(super) fn attach_shockwave_visual(
    commands: &mut Commands,
    shockwave: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Thin normalized rings keep a readable leading edge as the radius grows;
    // vertical scale raises them into a low crest the player can see and jump.
    let crest_mesh = meshes.add(Torus::new(0.94, 1.0));
    let rim_mesh = meshes.add(Torus::new(0.985, 1.025));
    let dust_mesh = meshes.add(Torus::new(0.90, 1.06));
    let crest_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.16, 0.018),
        emissive: LinearRgba::new(4.2, 0.16, 0.008, 1.0),
        ..default()
    });
    let rim_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.52, 0.06),
        emissive: LinearRgba::new(3.1, 0.65, 0.02, 1.0),
        ..default()
    });
    let dust_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.48, 0.08, 0.025, 0.32),
        emissive: LinearRgba::new(0.65, 0.035, 0.008, 1.0),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.entity(shockwave).with_children(|parent| {
        parent.spawn((
            Name::new("Shockwave Energy Crest"),
            Mesh3d(crest_mesh),
            MeshMaterial3d(crest_material),
            Transform::from_xyz(0.0, 0.24, 0.0),
        ));
        parent.spawn((
            Name::new("Shockwave Hot Rim"),
            Mesh3d(rim_mesh),
            MeshMaterial3d(rim_material),
            Transform::from_xyz(0.0, 0.32, 0.0),
        ));
        parent.spawn((
            Name::new("Shockwave Dust Band"),
            Mesh3d(dust_mesh),
            MeshMaterial3d(dust_material),
            Transform::from_xyz(0.0, 0.08, 0.0),
        ));
    });
}

pub fn update_saint_slam_telegraphs(
    mut commands: Commands,
    time: Res<Time>,
    owners: Query<(), With<SaintOfTheBrokenSiege>>,
    mut telegraphs: Query<(Entity, &mut SaintSlamTelegraph, &mut Transform)>,
) {
    for (entity, mut telegraph, mut transform) in &mut telegraphs {
        if owners.get(telegraph.owner).is_err() {
            commands.entity(entity).despawn();
            continue;
        }
        telegraph.lifetime -= time.delta_secs();
        if telegraph.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.scale = slam_telegraph_scale(telegraph.total_lifetime - telegraph.lifetime);
    }
}

pub fn update_saint_shockwaves(
    mut commands: Commands,
    time: Res<Time>,
    owners: Query<(), With<SaintOfTheBrokenSiege>>,
    mut player: Query<
        (
            Entity,
            &Transform,
            &VerticalMovement,
            &Dodge,
            &Equipment,
            &mut Health,
            &mut Barriers,
            &mut DeathPrevention,
        ),
        (With<PlayerCharacter>, Without<SaintShockwave>),
    >,
    mut shockwaves: Query<(
        Entity,
        &GlobalTransform,
        &mut Transform,
        &mut SaintShockwave,
    )>,
) {
    for (entity, global, mut transform, mut shockwave) in &mut shockwaves {
        if owners.get(shockwave.owner).is_err() {
            commands.entity(entity).despawn();
            continue;
        }
        shockwave.previous_radius = shockwave.radius;
        shockwave.radius += shockwave.speed * time.delta_secs();
        // The visible crest and the gameplay radius are the same value.
        transform.scale = shockwave_scale(shockwave.radius);

        let center = global.translation();
        // Every player character the leading edge crosses this frame.
        for (
            player_entity,
            player_transform,
            vertical,
            dodge,
            equipment,
            mut health,
            mut barriers,
            mut death_prevention,
        ) in &mut player
        {
            if shockwave.hit_players.contains(&player_entity) {
                continue;
            }
            let mut offset = player_transform.translation - center;
            offset.y = 0.0;
            let distance = offset.length();
            let crossed_player =
                distance >= shockwave.previous_radius && distance <= shockwave.radius;
            if !crossed_player {
                continue;
            }
            shockwave.hit_players.push(player_entity);
            // The universal jump is the intended response. Dodge invulnerability
            // remains a valid emergency response and keeps all classes viable.
            if vertical.grounded && !dodge.is_invulnerable() {
                apply_player_damage(
                    &mut health,
                    equipment,
                    Some(&mut barriers),
                    Some(&mut death_prevention),
                    shockwave.damage,
                );
            }
        }
        if shockwave.radius >= shockwave.maximum_radius {
            commands.entity(entity).despawn();
        }
    }
}
