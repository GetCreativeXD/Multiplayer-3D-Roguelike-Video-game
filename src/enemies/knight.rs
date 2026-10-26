//! Black Banner Knight -- the first elite (RUINBOUND_ENEMY_ROSTER.md
//! section 3.1, "Armored commander"). Built entirely on shared systems
//! already proven by the normal-enemy archetypes (encounter awareness,
//! ground-safe locomotion, committed-step traversal, stagger/interrupt,
//! loot) plus three genuinely new, reusable pieces this enemy needed and
//! no earlier archetype did:
//!
//! - `combat::FrontalGuard` (see `combat::frontal_guard`) -- a directional
//!   shield: blocks light attacks from the front, takes bonus damage from
//!   behind. Reusable by any future shielded enemy (the roster's Hollow
//!   Castellan).
//! - `combat::EnemyBuffs` + `enemies::aura::AuraEmitter` -- a generic
//!   "strengthen nearby enemies" aura. The Corrupted Banner this Knight
//!   plants is the first thing that uses it, but it's built for the
//!   roster's Bell Husk to reuse too.
//! - A three-state combo (`ComboWindup`/`ComboStrike` repeated with
//!   per-hit tuning) instead of the single windup/attack/recovery cycle
//!   every earlier melee archetype used -- the roster's "three-hit sword
//!   sequence with deliberately uneven timing."
//!
//! Every numeric value below is EXPERIMENTAL/TUNING, per project
//! convention, and unconfirmed in runtime until `cargo run` + a real
//! playtest happen on the actual machine.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_landmass::{AgentDesiredVelocity3d, prelude::AgentState};

use crate::combat::{
    Barriers, DeathPrevention, EnemyBuffSource, FrontalGuard, Health, apply_player_damage,
};
use crate::loot::LootTableId;
use crate::navigation_mesh::{NavmeshRouteCache, navmesh_pursuit_step};
use crate::player::{Dodge, PlayerCharacter};
use crate::ui::spawn_enemy_health_bar;
use crate::weapons::Equipment;

use super::aura::AuraEmitter;
use super::targeting::{EnemyTarget, current_target};
use super::telegraph::{EnemyTelegraph, EnemyTelegraphKind};
use super::{
    CommittedTraversalPolicy, Enemy, EnemyAwareness, EnemyBaseConfig, EnemyEncounterConfig,
    EnemyKind, EnemyMovement, EnemyNavigation, EnemyNavigationGrid, Stagger,
    enemy_committed_step_is_safe, enemy_grounded_path_is_safe, enemy_world_path_is_clear_ignoring,
    safe_enemy_locomotion, spawn_enemy_root, visuals,
};

// =========================================================
// TUNING -- all EXPERIMENTAL, see module doc comment above
// =========================================================

const KNIGHT_HEALTH: f32 = 220.0;
const KNIGHT_MOVEMENT_SPEED: f32 = 2.8;
const KNIGHT_MELEE_RANGE: f32 = 2.1;
const KNIGHT_STOP_DISTANCE: f32 = KNIGHT_MELEE_RANGE;
const KNIGHT_HIT_RANGE: f32 = 2.0;
const KNIGHT_STRIKE_SPEED: f32 = 6.0;
// Matches the project's established elite stagger-resistance convention
// (see e.g. weapons::abilities::hammer's HAMMER_LAUNCH_RESISTANCE_THRESHOLD
// comment: "elite resistance ~2, per the context doc's stagger table").
const KNIGHT_BASE_STAGGER_RESISTANCE: u32 = 2;

// -- Three-hit combo ("three-hit sword sequence with deliberately uneven
// timing") -- indices below are 0-based for hit 1/2/3.
const COMBO_WINDUPS: [f32; 3] = [0.35, 0.5, 0.85];
const COMBO_DAMAGE: [f32; 3] = [14.0, 14.0, 22.0];
// Reserved for a future player-side Stagger/poise system -- the player has
// no equivalent of the enemy `Stagger` component today, so nothing reads
// this yet. Kept here (rather than silently dropped) so per-hit tuning
// stays in one place when that system exists.
#[allow(dead_code)]
const COMBO_STAGGER_STRENGTH: [u32; 3] = [1, 1, 2];
const COMBO_STRIKE_DURATION: f32 = 0.18;
const COMBO_RECOVERY: f32 = 1.0;
const COMBO_STALL_TIMEOUT: f32 = 0.2;

// -- Shield Rush ("shield rush with strong knockback and a clear
// preparation animation")
const RUSH_MIN_RANGE: f32 = 4.0;
const RUSH_MAX_RANGE: f32 = 11.0;
const RUSH_WINDUP: f32 = 0.65;
const RUSH_SPEED: f32 = 13.0;
const RUSH_HIT_RANGE: f32 = 1.4;
const RUSH_DAMAGE: f32 = 16.0;
const RUSH_KNOCKBACK: f32 = 3.2;
const RUSH_RECOVERY: f32 = 0.9;
const RUSH_COOLDOWN: f32 = 6.0;
const RUSH_STALL_TIMEOUT: f32 = 0.2;

// -- Corrupted Banner ("plants a corrupted banner that strengthens nearby
// normal enemies" / "the banner can be destroyed or its placement
// interrupted")
const BANNER_WINDUP: f32 = 1.4;
const BANNER_RECOVERY: f32 = 0.6;
const BANNER_COOLDOWN: f32 = 16.0;
// Fully interruptible while channeling -- "the banner can be ... its
// placement interrupted" is a hard requirement, not a maybe.
const BANNER_VULNERABLE_STAGGER_RESISTANCE: u32 = 0;
const BANNER_HEALTH: f32 = 60.0;
pub(super) const BANNER_AURA_RADIUS: f32 = 9.0;
const BANNER_MOVEMENT_MULTIPLIER: f32 = 1.2;
const BANNER_DAMAGE_MULTIPLIER: f32 = 1.15;

// -- Frontal shield ("frontal shield block against light attacks" /
// "vulnerable from behind")
const FRONTAL_DOT_THRESHOLD: f32 = 0.3;
const LIGHT_ATTACK_STAGGER_THRESHOLD: u32 = 1;
const BLOCK_DAMAGE_MULTIPLIER: f32 = 0.2;
const REAR_DAMAGE_MULTIPLIER: f32 = 1.5;

// =========================================================
// COMPONENTS
// =========================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KnightState {
    /// Chasing, idling, and deciding what to do next -- the hub every
    /// other state returns to.
    Approach,
    ComboWindup(u8),
    ComboStrike(u8),
    ComboRecovery,
    RushWindup,
    RushCharge,
    RushRecovery,
    BannerWindup,
    BannerRecovery,
}

#[derive(Component)]
pub struct KnightCombat {
    pub state: KnightState,
    pub timer: f32,
    rush_cooldown_remaining: f32,
    banner_cooldown_remaining: f32,
    active_banner: Option<Entity>,
    locked_direction: Vec3,
    has_hit: bool,
    last_committed_position: Vec3,
    stall_timer: f32,
}

impl KnightCombat {
    fn reset_to_approach(&mut self) {
        self.state = KnightState::Approach;
        self.timer = 0.0;
        self.has_hit = false;
        self.stall_timer = 0.0;
    }
}

/// The Corrupted Banner planted by the Knight's Banner Plant. A genuine
/// destructible object, not a visual effect -- reuses the same
/// Health/Stagger/EnemyCombat/loot pipeline every enemy gets from
/// `spawn_enemy_root` rather than inventing a parallel "destructible
/// object" system, so hitting it with any weapon that already damages
/// enemies just works.
#[derive(Component)]
pub struct CorruptedBanner;

// `pub(super)`, not private: `update_knight_rush_telegraphs` and
// `update_knight_banner_channel_telegraphs` below are themselves
// `pub(super)` (registered as systems from `enemies::mod`), and a
// function can't be more visible than the types in its own signature
// (E0446, "private type in public interface") -- these two are only ever
// queried by those two systems, so `pub(super)` is exactly as wide as
// they actually need to be.
#[derive(Component)]
pub(super) struct KnightRushTelegraph {
    owner: Entity,
}

#[derive(Component)]
pub(super) struct KnightBannerTelegraph {
    owner: Entity,
    lifetime: f32,
    total_lifetime: f32,
}

// =========================================================
// SPAWNING
// =========================================================

pub fn spawn_black_banner_knight(
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
            kind: EnemyKind::BlackBannerKnight,
            name: "Black Banner Knight",
            health: KNIGHT_HEALTH,
            movement_speed: KNIGHT_MOVEMENT_SPEED,
            stop_distance: KNIGHT_STOP_DISTANCE,
            stagger_resistance: KNIGHT_BASE_STAGGER_RESISTANCE,
            loot_table: LootTableId::ELITE_ENEMY,
            collider_radius: 0.55,
            collider_height: 1.3,
        },
    );

    commands.entity(enemy).insert((
        KnightCombat {
            state: KnightState::Approach,
            timer: 0.0,
            rush_cooldown_remaining: 0.0,
            banner_cooldown_remaining: 0.0,
            active_banner: None,
            locked_direction: Vec3::ZERO,
            has_hit: false,
            last_committed_position: position,
            stall_timer: 0.0,
        },
        FrontalGuard::new(
            LIGHT_ATTACK_STAGGER_THRESHOLD,
            FRONTAL_DOT_THRESHOLD,
            BLOCK_DAMAGE_MULTIPLIER,
            REAR_DAMAGE_MULTIPLIER,
        ),
    ));

    visuals::attach_black_banner_knight_visual(commands, enemy, meshes, materials);
    enemy
}

fn spawn_corrupted_banner(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    encounter_id: u32,
) -> Entity {
    // Sharing the Knight's own encounter_id means damaging the banner
    // alerts the same encounter, exactly like damaging any other enemy in
    // it would (see the roster's encounter-area rule: "Damaging any enemy
    // activates and alerts every enemy with the same encounter ID").
    let banner = spawn_enemy_root(
        commands,
        position,
        EnemyEncounterConfig {
            encounter_id,
            activation_center: position,
            activation_half_extents: Vec2::ZERO,
            aggro_extension: 0.0,
        },
        EnemyBaseConfig {
            kind: EnemyKind::CorruptedBanner,
            name: "Corrupted Banner",
            health: BANNER_HEALTH,
            movement_speed: 0.0,
            stop_distance: 0.0,
            stagger_resistance: 0,
            loot_table: LootTableId::NORMAL_ENEMY,
            collider_radius: 0.3,
            collider_height: 1.6,
        },
    );

    commands.entity(banner).insert((
        CorruptedBanner,
        AuraEmitter {
            source: EnemyBuffSource::CorruptedBanner,
            radius: BANNER_AURA_RADIUS,
            movement_speed_multiplier: BANNER_MOVEMENT_MULTIPLIER,
            damage_multiplier: BANNER_DAMAGE_MULTIPLIER,
        },
    ));

    visuals::attach_corrupted_banner_visual(
        commands,
        banner,
        meshes,
        materials,
        BANNER_AURA_RADIUS,
    );
    banner
}

// =========================================================
// COMBAT
// =========================================================

#[allow(clippy::too_many_arguments)]
pub(super) fn black_banner_knight_combat(
    mut commands: Commands,
    time: Res<Time>,
    spatial_query: SpatialQuery,
    mut navigation_grid: ResMut<EnemyNavigationGrid>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    banners: Query<Entity, With<CorruptedBanner>>,
    mut player: Query<
        (
            Entity,
            &mut Transform,
            &mut Health,
            &Dodge,
            &Equipment,
            &mut Barriers,
            &mut DeathPrevention,
        ),
        (With<PlayerCharacter>, Without<Enemy>),
    >,
    targets: Query<&EnemyTarget>,
    mut knights: Query<
        (
            Entity,
            &mut Transform,
            &EnemyMovement,
            &mut EnemyNavigation,
            &mut Stagger,
            &EnemyAwareness,
            &mut KnightCombat,
            &mut FrontalGuard,
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
        mut stagger,
        awareness,
        mut combat,
        mut frontal_guard,
        desired_velocity,
        agent_state,
        route_cache,
    ) in &mut knights
    {
        let banner_alive = combat
            .active_banner
            .is_some_and(|banner| banners.get(banner).is_ok());
        if !banner_alive {
            combat.active_banner = None;
        }

        if !awareness.engaged {
            if combat.state == KnightState::BannerWindup {
                stagger.resistance = KNIGHT_BASE_STAGGER_RESISTANCE;
            }
            combat.reset_to_approach();
            frontal_guard.blocking = true;
            continue;
        }

        if stagger.is_staggered() {
            if combat.state == KnightState::BannerWindup {
                stagger.resistance = KNIGHT_BASE_STAGGER_RESISTANCE;
            }
            combat.reset_to_approach();
            frontal_guard.blocking = false;
            continue;
        }

        // The player this Knight is after (see `enemies::targeting`).
        let Some(target) = current_target(&targets, entity) else {
            continue;
        };
        let Ok((
            player_entity,
            mut player_transform,
            mut player_health,
            player_dodge,
            equipment,
            mut barriers,
            mut death_prevention,
        )) = player.get_mut(target)
        else {
            continue;
        };

        combat.rush_cooldown_remaining = (combat.rush_cooldown_remaining - delta).max(0.0);
        combat.banner_cooldown_remaining = (combat.banner_cooldown_remaining - delta).max(0.0);

        let mut to_player = player_transform.translation - transform.translation;
        to_player.y = 0.0;
        let distance = to_player.length();
        let direction = to_player.normalize_or_zero();

        // Shield is up only while approaching/idling -- both hands are
        // busy the moment the Knight commits to a sword swing, a charge,
        // or planting the banner.
        frontal_guard.blocking = matches!(combat.state, KnightState::Approach);

        match combat.state {
            // =================================================
            // APPROACH / DECIDE
            // =================================================
            KnightState::Approach => {
                if direction != Vec3::ZERO {
                    let target_rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
                    transform.rotation = transform.rotation.slerp(target_rotation, 8.0 * delta);
                }

                let grounded_safe = enemy_grounded_path_is_safe(
                    entity,
                    transform.translation,
                    to_player,
                    &spatial_query,
                );
                let clear_of_obstacles = navigation.is_on_traversal_surface()
                    || navigation_grid
                        .segment_avoids_permanent_obstacles(transform.translation, to_player);
                let world_clear = enemy_world_path_is_clear_ignoring(
                    entity,
                    Some(player_entity),
                    transform.translation,
                    to_player,
                    &spatial_query,
                );
                let reachable = grounded_safe && clear_of_obstacles && world_clear;
                let action_locked = navigation.traversal_combat_is_locked();

                let started_action =
                    if !action_locked && distance <= KNIGHT_STOP_DISTANCE && reachable {
                        combat.locked_direction = direction;
                        combat.state = KnightState::ComboWindup(1);
                        combat.timer = COMBO_WINDUPS[0];
                        combat.has_hit = false;
                        println!("Black Banner Knight: COMBO WINDUP 1");
                        true
                    } else if combat.active_banner.is_none()
                        && combat.banner_cooldown_remaining <= 0.0
                    {
                        combat.state = KnightState::BannerWindup;
                        combat.timer = BANNER_WINDUP;
                        stagger.resistance = BANNER_VULNERABLE_STAGGER_RESISTANCE;
                        spawn_banner_channel_telegraph(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            entity,
                            transform.translation,
                        );
                        println!("Black Banner Knight: PLANTING BANNER");
                        true
                    } else if !action_locked
                        && (RUSH_MIN_RANGE..=RUSH_MAX_RANGE).contains(&distance)
                        && combat.rush_cooldown_remaining <= 0.0
                        && reachable
                    {
                        combat.locked_direction = direction;
                        combat.state = KnightState::RushWindup;
                        combat.timer = RUSH_WINDUP;
                        spawn_rush_telegraph(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            entity,
                            transform.translation,
                            direction,
                            distance.min(RUSH_MAX_RANGE),
                        );
                        println!("Black Banner Knight: SHIELD RUSH WINDUP");
                        true
                    } else {
                        false
                    };

                if !started_action {
                    let (desired, navmesh_route_available) = navmesh_pursuit_step(
                        desired_velocity,
                        agent_state,
                        route_cache,
                        transform.translation,
                        direction,
                        movement.speed,
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

                    let movement_direction = step.normalize_or_zero();
                    if movement_direction.length_squared() > 0.0 {
                        let target_rotation =
                            Quat::from_rotation_y(movement_direction.x.atan2(movement_direction.z));
                        transform.rotation = transform.rotation.slerp(target_rotation, 8.0 * delta);
                    }
                }
            }

            // =================================================
            // THREE-HIT COMBO
            // =================================================
            KnightState::ComboWindup(hit) => {
                combat.timer -= delta;
                if combat.timer <= 0.0 {
                    combat.state = KnightState::ComboStrike(hit);
                    combat.timer = COMBO_STRIKE_DURATION;
                    combat.has_hit = false;
                    combat.last_committed_position = transform.translation;
                    combat.stall_timer = 0.0;
                    println!("Black Banner Knight: STRIKE {}", hit);
                }
            }
            KnightState::ComboStrike(hit) => {
                let index = (hit - 1) as usize;

                let progress = transform
                    .translation
                    .distance(combat.last_committed_position);
                if progress <= 0.01 {
                    combat.stall_timer += delta;
                } else {
                    combat.stall_timer = 0.0;
                }
                combat.last_committed_position = transform.translation;

                if combat.stall_timer >= COMBO_STALL_TIMEOUT {
                    combat.state = KnightState::ComboRecovery;
                    combat.timer = COMBO_RECOVERY;
                    combat.has_hit = false;
                    println!("Black Banner Knight: combo cancelled (blocked)");
                } else {
                    let step = combat.locked_direction * KNIGHT_STRIKE_SPEED * delta;
                    if enemy_committed_step_is_safe(
                        entity,
                        transform.translation,
                        step,
                        CommittedTraversalPolicy::Grounded,
                        &spatial_query,
                    ) {
                        transform.translation += step;
                    } else {
                        combat.state = KnightState::ComboRecovery;
                        combat.timer = COMBO_RECOVERY;
                        combat.has_hit = false;
                    }

                    if combat.state == KnightState::ComboStrike(hit) && !combat.has_hit {
                        let mut offset = player_transform.translation - transform.translation;
                        offset.y = 0.0;
                        if offset.length() <= KNIGHT_HIT_RANGE {
                            combat.has_hit = true;
                            if player_dodge.is_invulnerable() {
                                println!("Player dodged the Knight's strike {}!", hit);
                            } else {
                                let result = apply_player_damage(
                                    &mut player_health,
                                    equipment,
                                    Some(&mut barriers),
                                    Some(&mut death_prevention),
                                    COMBO_DAMAGE[index],
                                );
                                println!(
                                    "Knight strike {} hit for {:.1} ({:.1} blocked)! HP: {:.0}/{:.0}",
                                    hit,
                                    result.applied,
                                    result.blocked,
                                    player_health.current,
                                    player_health.max,
                                );
                            }
                        }
                    }

                    if combat.state == KnightState::ComboStrike(hit) {
                        combat.timer -= delta;
                        if combat.timer <= 0.0 {
                            if hit >= 3 {
                                combat.state = KnightState::ComboRecovery;
                                combat.timer = COMBO_RECOVERY;
                            } else {
                                let next = hit + 1;
                                combat.state = KnightState::ComboWindup(next);
                                combat.timer = COMBO_WINDUPS[(next - 1) as usize];
                            }
                            combat.has_hit = false;
                        }
                    }
                }
            }
            KnightState::ComboRecovery => {
                combat.timer -= delta;
                if combat.timer <= 0.0 {
                    combat.reset_to_approach();
                    println!("Black Banner Knight: CHASE");
                }
            }

            // =================================================
            // SHIELD RUSH
            // =================================================
            KnightState::RushWindup => {
                combat.timer -= delta;
                if combat.timer <= 0.0 {
                    let charge_distance = RUSH_MAX_RANGE.min(distance.max(RUSH_MIN_RANGE));
                    combat.state = KnightState::RushCharge;
                    combat.timer = charge_distance / RUSH_SPEED;
                    combat.has_hit = false;
                    combat.last_committed_position = transform.translation;
                    combat.stall_timer = 0.0;
                    println!("Black Banner Knight: SHIELD RUSH");
                }
            }
            KnightState::RushCharge => {
                let progress = transform
                    .translation
                    .distance(combat.last_committed_position);
                if progress <= 0.01 {
                    combat.stall_timer += delta;
                } else {
                    combat.stall_timer = 0.0;
                }
                combat.last_committed_position = transform.translation;

                if combat.stall_timer >= RUSH_STALL_TIMEOUT {
                    combat.state = KnightState::RushRecovery;
                    combat.timer = RUSH_RECOVERY;
                    combat.rush_cooldown_remaining = RUSH_COOLDOWN;
                    println!("Black Banner Knight: rush cancelled (blocked)");
                } else {
                    let movement_time = delta.min(combat.timer.max(0.0));
                    let step = combat.locked_direction * RUSH_SPEED * movement_time;
                    if enemy_committed_step_is_safe(
                        entity,
                        transform.translation,
                        step,
                        CommittedTraversalPolicy::Grounded,
                        &spatial_query,
                    ) {
                        transform.translation += step;
                    } else {
                        combat.state = KnightState::RushRecovery;
                        combat.timer = RUSH_RECOVERY;
                        combat.rush_cooldown_remaining = RUSH_COOLDOWN;
                    }

                    if combat.state == KnightState::RushCharge && !combat.has_hit {
                        let mut offset = player_transform.translation - transform.translation;
                        offset.y = 0.0;
                        if offset.length() <= RUSH_HIT_RANGE {
                            combat.has_hit = true;
                            if player_dodge.is_invulnerable() {
                                println!("Player dodged the shield rush!");
                            } else {
                                let result = apply_player_damage(
                                    &mut player_health,
                                    equipment,
                                    Some(&mut barriers),
                                    Some(&mut death_prevention),
                                    RUSH_DAMAGE,
                                );
                                // First player-directed knockback in the
                                // project -- every earlier enemy attack only
                                // damages. Uses the exact same plain
                                // position-offset pattern every other
                                // knockback in this codebase already uses
                                // (see weapons::melee's
                                // `enemy_transform.translation += direction *
                                // weapon.knockback`), so it's low-risk, but
                                // it hasn't been runtime-confirmed against
                                // `player::character_movement`'s per-frame
                                // position solve -- flag this specifically
                                // during playtesting.
                                let mut push = offset.normalize_or_zero();
                                if push == Vec3::ZERO {
                                    push = combat.locked_direction;
                                }
                                player_transform.translation += push * RUSH_KNOCKBACK;
                                println!(
                                    "Shield Rush hit for {:.1} ({:.1} blocked)! HP: {:.0}/{:.0}",
                                    result.applied,
                                    result.blocked,
                                    player_health.current,
                                    player_health.max,
                                );
                            }
                        }
                    }

                    if combat.state == KnightState::RushCharge {
                        combat.timer -= delta;
                        if combat.timer <= 0.0 {
                            combat.state = KnightState::RushRecovery;
                            combat.timer = RUSH_RECOVERY;
                            combat.rush_cooldown_remaining = RUSH_COOLDOWN;
                        }
                    }
                }
            }
            KnightState::RushRecovery => {
                combat.timer -= delta;
                if combat.timer <= 0.0 {
                    combat.reset_to_approach();
                }
            }

            // =================================================
            // BANNER PLANT
            // =================================================
            KnightState::BannerWindup => {
                combat.timer -= delta;
                if combat.timer <= 0.0 {
                    stagger.resistance = KNIGHT_BASE_STAGGER_RESISTANCE;
                    let banner = spawn_corrupted_banner(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        transform.translation,
                        awareness.encounter_id,
                    );
                    spawn_enemy_health_bar(&mut commands, banner);
                    combat.active_banner = Some(banner);
                    combat.banner_cooldown_remaining = BANNER_COOLDOWN;
                    combat.state = KnightState::BannerRecovery;
                    combat.timer = BANNER_RECOVERY;
                    println!("Black Banner Knight: BANNER PLANTED");
                }
            }
            KnightState::BannerRecovery => {
                combat.timer -= delta;
                if combat.timer <= 0.0 {
                    combat.reset_to_approach();
                }
            }
        }
    }
}

// =========================================================
// TELEGRAPHS
// =========================================================

fn spawn_rush_telegraph(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    owner: Entity,
    origin: Vec3,
    direction: Vec3,
    distance: f32,
) {
    if direction == Vec3::ZERO || distance <= 0.0 {
        return;
    }

    commands.spawn((
        Name::new("Knight Rush Telegraph"),
        KnightRushTelegraph { owner },
        EnemyTelegraph {
            kind: EnemyTelegraphKind::KnightRushLane,
        },
        rush_telegraph_visual(meshes, materials),
        Transform {
            translation: origin + Vec3::Y * 0.05 + direction * (distance * 0.5),
            rotation: Quat::from_rotation_arc(Vec3::Z, direction),
            scale: Vec3::new(1.0, 1.0, distance),
        },
    ));
}

/// The Shield Rush lane: a unit-long strip, stretched along its local Z by
/// the rush distance (the telegraph's scale). Shared by the host's
/// telegraph and a client's copy of it (`enemies::telegraph`).
pub(super) fn rush_telegraph_visual(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> impl Bundle + use<> {
    (
        Mesh3d(meshes.add(Cuboid::new(0.9, 0.06, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.55, 0.05, 0.08, 0.55),
            emissive: LinearRgba::new(1.1, 0.35, 0.05, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
    )
}

pub(super) fn update_knight_rush_telegraphs(
    mut commands: Commands,
    owners: Query<&KnightCombat>,
    telegraphs: Query<(Entity, &KnightRushTelegraph)>,
) {
    for (entity, telegraph) in &telegraphs {
        let Ok(combat) = owners.get(telegraph.owner) else {
            commands.entity(entity).despawn();
            continue;
        };
        if combat.state != KnightState::RushWindup {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_banner_channel_telegraph(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    owner: Entity,
    position: Vec3,
) {
    commands.spawn((
        Name::new("Knight Banner Channel Telegraph"),
        KnightBannerTelegraph {
            owner,
            lifetime: BANNER_WINDUP,
            total_lifetime: BANNER_WINDUP,
        },
        EnemyTelegraph {
            kind: EnemyTelegraphKind::KnightBannerDisk,
        },
        banner_telegraph_visual(meshes, materials),
        Transform::from_translation(position + Vec3::Y * 0.05),
    ));
}

/// The Banner Plant channel disk. Shared by the host's telegraph and a
/// client's copy of it (`enemies::telegraph`).
pub(super) fn banner_telegraph_visual(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> impl Bundle + use<> {
    (
        Mesh3d(meshes.add(Cylinder::new(1.6, 0.035))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.45, 0.02, 0.06, 0.28),
            emissive: LinearRgba::new(1.4, 0.12, 0.02, 1.0),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
    )
}

/// The banner disk's pulse at `seconds` into the channel.
pub(super) fn banner_telegraph_scale(seconds: f32) -> Vec3 {
    let progress = (seconds / BANNER_WINDUP).clamp(0.0, 1.0);
    let pulse = 0.9 + (progress * std::f32::consts::TAU * 2.5).sin().abs() * 0.12;
    Vec3::new(pulse, 1.0, pulse)
}

pub(super) fn update_knight_banner_channel_telegraphs(
    mut commands: Commands,
    time: Res<Time>,
    owners: Query<&KnightCombat>,
    mut telegraphs: Query<(Entity, &mut KnightBannerTelegraph, &mut Transform)>,
) {
    for (entity, mut telegraph, mut transform) in &mut telegraphs {
        let Ok(combat) = owners.get(telegraph.owner) else {
            commands.entity(entity).despawn();
            continue;
        };
        if combat.state != KnightState::BannerWindup {
            commands.entity(entity).despawn();
            continue;
        }

        telegraph.lifetime -= time.delta_secs();
        transform.scale = banner_telegraph_scale(telegraph.total_lifetime - telegraph.lifetime);
    }
}
