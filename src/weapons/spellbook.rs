//! Spellbook -- see WEAPONS.md's Spellbook section for the design source
//! of truth. Unlike every other weapon family, WEAPONS.md writes
//! Spellbook's tiers as passives layered onto one shared identity
//! ("Basic Arcana... establishes the spellbook's ability-focused
//! identity") rather than a mutually-exclusive ability per tier, so that
//! baseline passive lives on every Spellbook tier's base stats
//! (`Weapon::spellbook`'s `ability_power_bonus` /
//! `ability_cooldown_multiplier`) instead of only at Common.
//!
//! The basic attack and every tier ability fire the same family of
//! homing "Arcane Bolt" projectiles: click near an enemy -- within a
//! forgiving cone around the crosshair, not a pixel-perfect raycast hit
//! on its collider -- and the bolt soft-locks onto whichever qualifying
//! enemy is closest to dead-center of the crosshair, then follows it
//! until it hits (`find_spellbook_target`, `ProjectileKind::homes`); aim
//! at empty space and it flies straight, so the weapon still works at
//! range with no target. Basic attack + Common (Focused Cast) + Rare
//! (Scorching Cast) + Epic (Echoing Cast) + all three Legendaries (The
//! Hollow Gospel, Saint's Testament, The Living Codex) are implemented
//! here.

use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::combat::{BarrierSource, Barriers, DotSource, Health};
use crate::enemies::Enemy;
use crate::items::ItemDefinitionId;
use crate::ruin::RuinState;

use super::components::{
    PlayerProjectile, Projectile, ProjectileKind, ProjectileOwner, Weapon, WeaponAbilityState,
};

// =========================================================
// SHARED
// =========================================================

const ARCANE_BOLT_SPEED: f32 = 30.0;
const ARCANE_BOLT_LIFETIME: f32 = 3.5;
const ARCANE_BOLT_RADIUS: f32 = 0.17;
const ARCANE_BOLT_COLOR: Color = Color::srgb(0.55, 0.75, 1.0);

/// How close to dead-center of the crosshair (in cosine-of-angle terms)
/// an enemy must be to be picked as a Spellbook soft-lock target.
/// `cos(20 degrees)` -- generous enough that the player doesn't need a
/// pixel-perfect shot on the enemy's own collider, matching WEAPONS.md's
/// "click on an enemy" as "aim roughly at one," not "hit its hitbox."
const SPELLBOOK_TARGET_LOCK_MAX_ANGLE_DOT: f32 = 0.94;

/// True if any enemy is within `range` of `player_position` -- checked
/// before firing (`melee::player_basic_attack`,
/// `abilities::weapon_ability_input`) so the Spellbook never spends a
/// cast, a cooldown, or even fires a projectile when there's nothing
/// anywhere nearby to hit. Deliberately broader than
/// `find_spellbook_target`'s crosshair lock cone below -- this only asks
/// "is there anything in range at all," regardless of where the player
/// is currently aiming, and measures from the player's own position
/// rather than the camera's.
pub(crate) fn any_enemy_in_range(
    player_position: Vec3,
    range: f32,
    enemies: &Query<(Entity, &GlobalTransform), With<Enemy>>,
) -> bool {
    enemies
        .iter()
        .any(|(_, transform)| transform.translation().distance(player_position) <= range)
}

/// Finds whichever enemy within `max_distance` sits closest to dead-
/// center of the crosshair (highest angle-to-camera-direction cosine),
/// ignoring anything outside `SPELLBOOK_TARGET_LOCK_MAX_ANGLE_DOT`'s
/// cone. Returns the target's entity and current position. Picking by
/// angle rather than raw distance means a far enemy directly under the
/// reticle still wins over a near one at the edge of the lock cone --
/// this is what makes clicking "near" an enemy home in on it, rather
/// than requiring the crosshair's exact raycast to strike its collider.
fn find_spellbook_target(
    camera_origin: Vec3,
    camera_direction: Vec3,
    max_distance: f32,
    enemies: &Query<(Entity, &GlobalTransform), With<Enemy>>,
) -> Option<(Entity, Vec3)> {
    let mut best: Option<(Entity, Vec3, f32)> = None;

    for (entity, transform) in enemies.iter() {
        let position = transform.translation();
        let to_enemy = position - camera_origin;
        let distance = to_enemy.length();

        if distance <= 0.0 || distance > max_distance {
            continue;
        }

        let direction_to_enemy = to_enemy / distance;
        let angle_dot = camera_direction.dot(direction_to_enemy);

        if angle_dot < SPELLBOOK_TARGET_LOCK_MAX_ANGLE_DOT {
            continue;
        }

        let is_better = match best {
            Some((_, _, best_dot)) => angle_dot > best_dot,
            None => true,
        };

        if is_better {
            best = Some((entity, position, angle_dot));
        }
    }

    best.map(|(entity, position, _)| (entity, position))
}

/// Shared aiming step every Spellbook cast uses: turns the player to
/// face the shot, and -- the mechanic this weapon is built around --
/// soft-locks onto whichever enemy is closest to the crosshair
/// (`find_spellbook_target`). When one is found, the bolt homes in on it
/// (`ProjectileKind::homes`) until it hits; when nothing is within the
/// lock cone, the bolt falls back to a plain raycast shot straight down
/// the crosshair, so Spellbook still works as an ordinary ranged weapon
/// with nothing under the reticle.
fn aim_spellbook(
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
    enemies: &Query<(Entity, &GlobalTransform), With<Enemy>>,
) -> Option<(Vec3, Vec3, Option<Entity>)> {
    let camera_origin = camera_transform.translation;
    let camera_direction = camera_transform.forward().as_vec3().normalize_or_zero();

    if camera_direction == Vec3::ZERO {
        return None;
    }

    let max_distance = weapon.range;

    let locked_target =
        find_spellbook_target(camera_origin, camera_direction, max_distance, enemies);

    let target_point = match locked_target {
        Some((_, position)) => position,
        None => {
            // No enemy within the soft-lock cone -- fall back to a
            // straight raycast shot, capped at the weapon's range, the
            // same way Bow/Crossbow aim.
            let filter = SpatialQueryFilter::from_excluded_entities([player_entity]);
            let mut point = camera_origin + camera_direction * max_distance;

            if let Ok(ray_direction) = Dir3::new(camera_direction.adjust_precision()) {
                if let Some(hit) = spatial_query.cast_ray(
                    camera_origin.adjust_precision(),
                    ray_direction,
                    max_distance,
                    true,
                    &filter,
                ) {
                    point = camera_origin + camera_direction * hit.distance;
                }
            }

            point
        }
    };

    let target = locked_target.map(|(entity, _)| entity);

    let spawn_position = player_transform.translation + Vec3::Y * 0.6 + camera_direction * 0.9;
    let bolt_direction = (target_point - spawn_position).normalize_or_zero();

    if bolt_direction == Vec3::ZERO {
        return None;
    }

    let mut horizontal_direction = bolt_direction;
    horizontal_direction.y = 0.0;
    horizontal_direction = horizontal_direction.normalize_or_zero();

    if horizontal_direction != Vec3::ZERO {
        player_transform.rotation =
            Quat::from_rotation_y(horizontal_direction.x.atan2(horizontal_direction.z));
    }

    Some((spawn_position, bolt_direction, target))
}

#[allow(clippy::too_many_arguments)]
fn spawn_arcane_bolt(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    spawn_position: Vec3,
    direction: Vec3,
    target: Option<Entity>,
    damage: f32,
    weapon: &Weapon,
    kind: ProjectileKind,
    radius: f32,
    color: Color,
    // The player who cast it (see `ProjectileOwner`).
    owner: Entity,
) -> Entity {
    commands
        .spawn((
            PlayerProjectile,
            ProjectileOwner(owner),
            Projectile {
                velocity: direction * ARCANE_BOLT_SPEED,
                damage,
                knockback: weapon.knockback,
                stagger_strength: weapon.stagger_strength,
                stagger_duration: weapon.stagger_duration,
                lifetime: ARCANE_BOLT_LIFETIME,
                piercing: false,
                hit_entities: Vec::new(),
                kind,
                target,
            },
            Mesh3d(meshes.add(Sphere::new(radius))),
            MeshMaterial3d(materials.add(color)),
            Transform::from_translation(spawn_position),
        ))
        .id()
}

// =========================================================
// BASIC ATTACK -- Arcane Bolt
// =========================================================

pub(crate) fn fire_arcane_bolt(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    damage_multiplier: f32,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
    enemies: &Query<(Entity, &GlobalTransform), With<Enemy>>,
) {
    let Some((spawn_position, direction, target)) = aim_spellbook(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
        enemies,
    ) else {
        return;
    };

    spawn_arcane_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        target,
        weapon.damage * damage_multiplier,
        weapon,
        ProjectileKind::ArcaneBolt,
        ARCANE_BOLT_RADIUS,
        ARCANE_BOLT_COLOR,
        player_entity,
    );

    println!("Arcane Bolt fired! Homing: {}", target.is_some());
}

// =========================================================
// COMMON -- Apprentice's Grimoire: Focused Cast
// =========================================================

const FOCUSED_CAST_MULTIPLIER: f32 = 1.6;

pub(crate) fn fire_focused_cast(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
    enemies: &Query<(Entity, &GlobalTransform), With<Enemy>>,
) {
    let Some((spawn_position, direction, target)) = aim_spellbook(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
        enemies,
    ) else {
        return;
    };

    spawn_arcane_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        target,
        weapon.damage * FOCUSED_CAST_MULTIPLIER,
        weapon,
        ProjectileKind::ArcaneBolt,
        ARCANE_BOLT_RADIUS * 1.2,
        Color::srgb(0.65, 0.85, 1.0),
        player_entity,
    );

    println!("Focused Cast!");
}

// =========================================================
// RARE -- Ember Codex: Scorching Cast
// =========================================================

const SCORCHING_CAST_MULTIPLIER: f32 = 1.3;
/// Exact numbers from WEAPONS.md: "6 fire damage per second for 4
/// seconds." Read by `weapons::projectiles::movement`'s ScorchBolt hit
/// handling to build/refresh the target's `combat::DamageOverTime`.
pub(crate) const SCORCH_DAMAGE_PER_SECOND: f32 = 6.0;
pub(crate) const SCORCH_DURATION: f32 = 4.0;
pub(crate) const SCORCH_SOURCE: DotSource = DotSource::Scorch;

pub(crate) fn fire_scorching_cast(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
    enemies: &Query<(Entity, &GlobalTransform), With<Enemy>>,
) {
    let Some((spawn_position, direction, target)) = aim_spellbook(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
        enemies,
    ) else {
        return;
    };

    spawn_arcane_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        target,
        weapon.damage * SCORCHING_CAST_MULTIPLIER,
        weapon,
        ProjectileKind::ScorchBolt,
        ARCANE_BOLT_RADIUS * 1.2,
        Color::srgb(1.0, 0.55, 0.2),
        player_entity,
    );

    println!("Scorching Cast!");
}

// =========================================================
// EPIC -- Grimoire of Echoes: Echoing Cast
// =========================================================

const ECHOING_CAST_MULTIPLIER: f32 = 1.3;
/// "Every few ability casts repeats a weaker version." Every third cast
/// immediately fires a second, weaker bolt alongside the main one rather
/// than truly repeating it on a delay -- WEAPONS.md doesn't specify a
/// delay, and this keeps the mechanic self-contained in one cast instead
/// of needing a scheduled follow-up spawn.
const ECHOING_CAST_INTERVAL: u32 = 3;
const ECHO_DAMAGE_FRACTION: f32 = 0.5;
/// Angle (radians) the echo bolt diverges from the main bolt, so the two
/// are visually distinct rather than perfectly overlapping.
const ECHO_SPREAD_RADIANS: f32 = 0.18;

pub(crate) fn fire_echoing_cast(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
    enemies: &Query<(Entity, &GlobalTransform), With<Enemy>>,
    state: &mut WeaponAbilityState,
) {
    let Some((spawn_position, direction, target)) = aim_spellbook(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
        enemies,
    ) else {
        return;
    };

    spawn_arcane_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        target,
        weapon.damage * ECHOING_CAST_MULTIPLIER,
        weapon,
        ProjectileKind::ArcaneBolt,
        ARCANE_BOLT_RADIUS * 1.2,
        Color::srgb(0.75, 0.55, 1.0),
        player_entity,
    );

    state.echoing_cast_counter += 1;

    if state.echoing_cast_counter >= ECHOING_CAST_INTERVAL {
        state.echoing_cast_counter = 0;

        let echo_direction = Quat::from_rotation_y(ECHO_SPREAD_RADIANS) * direction;

        spawn_arcane_bolt(
            commands,
            meshes,
            materials,
            spawn_position,
            echo_direction,
            // The echo re-acquires its own target rather than reusing the
            // main bolt's -- simplest way to avoid two homing bolts
            // fighting over identical steering.
            None,
            weapon.damage * ECHOING_CAST_MULTIPLIER * ECHO_DAMAGE_FRACTION,
            weapon,
            ProjectileKind::ArcaneBolt,
            ARCANE_BOLT_RADIUS,
            Color::srgb(0.75, 0.55, 1.0),
            player_entity,
        );

        println!("Echoing Cast triggered an echo!");
    }

    println!("Echoing Cast!");
}

// =========================================================
// LEGENDARY -- The Hollow Gospel: Ruin Bolt
// =========================================================

pub(crate) const HOLLOW_GOSPEL_ID: ItemDefinitionId =
    ItemDefinitionId("weapon.spellbook.legendary.hollow_gospel");
pub(crate) const HOLLOW_GOSPEL_NAME: &str = "The Hollow Gospel";

const HOLLOW_GOSPEL_BASE_MULTIPLIER: f32 = 1.2;
/// "Abilities become stronger as the player's Ruin increases." Up to
/// +90% damage at 100 Ruin.
const HOLLOW_GOSPEL_RUIN_DAMAGE_SCALING: f32 = 0.9;
/// "At high Ruin, abilities... have a chance to cause a small backlash
/// against the player." Read as "every cast once Ruin reaches this" --
/// WEAPONS.md's Ruinpiercer passive drops the same kind of probability
/// roll for a deterministic cooldown-gated version (see
/// `weapons::crossbow`'s Cleansing Strike doc comment); this is the same
/// simplification applied to a threshold instead of a cooldown.
const HOLLOW_GOSPEL_BACKLASH_RUIN_THRESHOLD: f32 = 80.0;
const HOLLOW_GOSPEL_BACKLASH_DAMAGE: f32 = 6.0;

#[allow(clippy::too_many_arguments)]
pub(crate) fn fire_ruin_bolt(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
    enemies: &Query<(Entity, &GlobalTransform), With<Enemy>>,
    ruin_state: &RuinState,
    player_health: &mut Health,
) {
    let Some((spawn_position, direction, target)) = aim_spellbook(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
        enemies,
    ) else {
        return;
    };

    let ruin_multiplier = HOLLOW_GOSPEL_BASE_MULTIPLIER
        + (ruin_state.ruin() / 100.0) * HOLLOW_GOSPEL_RUIN_DAMAGE_SCALING;

    spawn_arcane_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        target,
        weapon.damage * ruin_multiplier,
        weapon,
        ProjectileKind::ArcaneBolt,
        ARCANE_BOLT_RADIUS * 1.3,
        Color::srgb(0.6, 0.15, 0.75),
        player_entity,
    );

    if ruin_state.ruin() >= HOLLOW_GOSPEL_BACKLASH_RUIN_THRESHOLD {
        // Self-inflicted magical backlash -- deliberately bypasses the
        // player's defense/barrier pipeline (`combat::apply_player_damage`)
        // the way a direct HP cost rather than an incoming attack should,
        // and can never itself be lethal.
        player_health.current = (player_health.current - HOLLOW_GOSPEL_BACKLASH_DAMAGE).max(1.0);
        println!("The Hollow Gospel backlash!");
    }

    println!("Ruin Bolt! Ruin multiplier {:.2}", ruin_multiplier);
}

// =========================================================
// LEGENDARY -- Saint's Testament: Grace Bolt
// =========================================================

pub(crate) const SAINTS_TESTAMENT_ID: ItemDefinitionId =
    ItemDefinitionId("weapon.spellbook.legendary.saints_testament");
pub(crate) const SAINTS_TESTAMENT_NAME: &str = "Saint's Testament";

const GRACE_BOLT_MULTIPLIER: f32 = 1.2;
pub(crate) const GRACE_MAX_STACKS: u32 = 5;
const GRACE_BARRIER_AMOUNT: f32 = 30.0;

pub(crate) fn fire_grace_bolt(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
    enemies: &Query<(Entity, &GlobalTransform), With<Enemy>>,
) {
    let Some((spawn_position, direction, target)) = aim_spellbook(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
        enemies,
    ) else {
        return;
    };

    spawn_arcane_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        target,
        weapon.damage * GRACE_BOLT_MULTIPLIER,
        weapon,
        ProjectileKind::GraceBolt,
        ARCANE_BOLT_RADIUS * 1.2,
        Color::srgb(0.95, 0.9, 0.6),
        player_entity,
    );

    println!("Grace Bolt!");
}

/// Builds Grace when Grace Bolt damages an enemy; at `GRACE_MAX_STACKS`,
/// grants a barrier and resets. Called from
/// `weapons::projectiles::movement`'s GraceBolt hit handling, which is
/// the only place that already has the caster's `WeaponAbilityState` and
/// `Barriers` on hand for a hit-confirmed effect like this one.
pub(crate) fn apply_grace(state: &mut WeaponAbilityState, barriers: &mut Barriers) {
    state.grace_stacks += 1;

    if state.grace_stacks >= GRACE_MAX_STACKS {
        state.grace_stacks = 0;
        barriers.grant_default(BarrierSource::SaintsTestament, GRACE_BARRIER_AMOUNT);
        println!("Graceful Casting: barrier granted!");
    }
}

// =========================================================
// LEGENDARY -- The Living Codex: Adaptive Cast
// =========================================================

pub(crate) const LIVING_CODEX_ID: ItemDefinitionId =
    ItemDefinitionId("weapon.spellbook.legendary.living_codex");
pub(crate) const LIVING_CODEX_NAME: &str = "The Living Codex";

const ADAPTIVE_CAST_MULTIPLIER: f32 = 1.3;
/// "Repeatedly using the same ability changes its behavior." After this
/// many consecutive casts, the next cast fires an extra projectile
/// instead of just one -- read as "the ability itself evolves" rather
/// than a persistent per-encounter/per-room state Ruinbound has no
/// existing hook to reset from (WEAPONS.md's "until the player changes
/// rooms" -- see this function's doc comment).
const ADAPTIVE_CAST_INTERVAL: u32 = 3;
const ADAPTIVE_CAST_EXTRA_SPREAD_RADIANS: f32 = 0.22;

/// Fires Adaptive Cast. WEAPONS.md's Living Codex resets its evolution
/// "until the player changes rooms"; Ruinbound doesn't have a
/// room-change hook this weapon can subscribe to yet, so the counter
/// here simply resets itself the moment it triggers an extra projectile,
/// rather than persisting indefinitely across an entire run.
pub(crate) fn fire_adaptive_cast(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
    enemies: &Query<(Entity, &GlobalTransform), With<Enemy>>,
    state: &mut WeaponAbilityState,
) {
    let Some((spawn_position, direction, target)) = aim_spellbook(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
        enemies,
    ) else {
        return;
    };

    spawn_arcane_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        target,
        weapon.damage * ADAPTIVE_CAST_MULTIPLIER,
        weapon,
        ProjectileKind::ArcaneBolt,
        ARCANE_BOLT_RADIUS * 1.3,
        Color::srgb(0.5, 0.95, 0.85),
        player_entity,
    );

    state.adaptive_cast_stacks += 1;

    if state.adaptive_cast_stacks >= ADAPTIVE_CAST_INTERVAL {
        state.adaptive_cast_stacks = 0;

        let extra_direction = Quat::from_rotation_y(ADAPTIVE_CAST_EXTRA_SPREAD_RADIANS) * direction;

        spawn_arcane_bolt(
            commands,
            meshes,
            materials,
            spawn_position,
            extra_direction,
            None,
            weapon.damage * ADAPTIVE_CAST_MULTIPLIER,
            weapon,
            ProjectileKind::ArcaneBolt,
            ARCANE_BOLT_RADIUS * 1.3,
            Color::srgb(0.5, 0.95, 0.85),
            player_entity,
        );

        println!("Evolving Spells: Adaptive Cast fires an extra bolt!");
    }

    println!("Adaptive Cast!");
}
