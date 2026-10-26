//! Wildshaper's three class abilities: Rhino Charge, Bear Slam, and
//! Panther Pounce (see the project context doc, section 22).
//!
//! Structurally this mirrors `characters::vanguard_abilities`,
//! `characters::warden_abilities`, and `characters::runesage_abilities`:
//! one `*Ability` component per ability, an input system per key press, a
//! companion update system for anything that plays out over time, and a
//! `sync_wildshaper_class_state` system that writes current state into the
//! shared `ClassAbilityHud` the same way the other three classes do.
//! Nothing here changes any other class's code.
//!
//! Identity (context doc, section 22): Wildshaper is a melee shapeshifter
//! that briefly transforms into an animal to execute one fast attack or
//! movement technique, then immediately returns to humanoid form. Every
//! ability below is deliberately short -- a single dash/slam/leap, never a
//! slow multi-beat form-change sequence -- so the class keeps the same
//! fast combat pace as every other class.
//!
//! Ability-power scaling: every ability below multiplies its base numbers
//! by the caster's `Equipment::ability_power_multiplier()` (see
//! `weapons::components`), exactly like Warden of Shades' and Runesage's
//! abilities do. Any future gear roll that raises Ability Power
//! automatically makes Rhino Charge, Bear Slam, and Panther Pounce all
//! stronger together.
//!
//! Damage/stagger both go through the shared systems the project context
//! doc requires ("Damage should be applied through the project's shared
//! damage and defense systems"): `combat::apply_enemy_damage` (so a
//! Runebroken-style debuff affects Wildshaper the same as every other
//! damage source) and `combat::effective_stagger_resistance` (so a
//! stagger-resistance debuff applies here too).
//!
//! Launching note: the context doc says Rhino Charge should let "smaller
//! enemies ... be launched." Ruinbound's enemies are kinematic and are
//! snapped back to their last safe grounded position by
//! `enemies::enforce_enemy_ground_safety` the instant they leave the
//! ground (see that system's doc comment) -- there is no airborne-enemy
//! state to put them into yet, and inventing one is a much larger,
//! cross-cutting change than this pass should make unprompted. Until that
//! exists, "launched" is approximated with a much larger horizontal
//! knockback distance on enemies at or below
//! `RHINO_CHARGE_LAUNCH_RESISTANCE_THRESHOLD` stagger resistance (i.e.
//! ordinary enemies, not elites/bosses) -- a visibly stronger shove than
//! anything else in the project, without silently touching the shared
//! navigation/ground-safety architecture the context doc says to change
//! cautiously.

use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::visual_effects::{EffectShape, effect_mesh};

use crate::characters::class_abilities::{
    AbilityUseTracker, ClassAbilityDisplay, ClassAbilityHud, ClassAbilityHudSlot,
};
use crate::characters::wildshaper::Wildshaper;
use crate::collision::GameLayer;
use crate::combat::{self, EnemyDebuffs, Health};
use crate::enemies::{Enemy, EnemyCombat, EnemyState, Stagger};
use crate::player::{AbilityMovement, PlayerCharacter, PlayerIntent};
use crate::weapons::Equipment;

// =========================================================
// INPUT
// =========================================================

// Class-ability slots (`player::CLASS_ABILITY_KEYS`): Q, E, R. Every
// Wildshaper player character (the local `Player`, or a joined player's
// body on the multiplayer host) casts from its own `PlayerIntent`, aimed
// with that player's camera (`PlayerIntent::aim_transform`).
const RHINO_CHARGE_SLOT: usize = 0;
const BEAR_SLAM_SLOT: usize = 1;
const PANTHER_POUNCE_SLOT: usize = 2;

// =========================================================
// RHINO CHARGE (Q) -- displacement + mobility
//
// Transform into a rhino and charge forward. Enemies in the charge's path
// are bulldozed (damaged, knocked back, and -- if they're not an
// elite/boss -- flung much further, approximating a launch). Charging
// into a wall ends the transformation immediately in a stunning crash
// that staggers anything standing nearby, rewarding using room geometry
// against enemies the same way the project's other mechanics do.
// =========================================================

const RHINO_CHARGE_COOLDOWN: f32 = 8.0;
const RHINO_CHARGE_SPEED: f32 = 16.0;
const RHINO_CHARGE_DURATION: f32 = 0.55;
const RHINO_CHARGE_HIT_RADIUS: f32 = 1.5;
const RHINO_CHARGE_BASE_DAMAGE: f32 = 24.0;
const RHINO_CHARGE_KNOCKBACK: f32 = 2.2;
const RHINO_CHARGE_STAGGER_STRENGTH: u32 = 2;
const RHINO_CHARGE_STAGGER_DURATION: f32 = 0.5;

/// Stagger-resistance cutoff for the bonus "launch" knockback below.
/// Comfortably under elite resistance (~2, per the context doc's stagger
/// table), so only ordinary enemies get bulldozed extra hard.
const RHINO_CHARGE_LAUNCH_RESISTANCE_THRESHOLD: u32 = 1;
const RHINO_CHARGE_LAUNCH_KNOCKBACK_MULTIPLIER: f32 = 2.25;

/// How far ahead of the charging player to look for a wall each frame.
/// Set comfortably beyond one frame's worth of travel at
/// `RHINO_CHARGE_SPEED` so the crash triggers before the player visually
/// clips into the geometry.
const RHINO_CHARGE_WALL_CHECK_DISTANCE: f32 = 1.2;
const RHINO_CHARGE_CRASH_RADIUS: f32 = 3.0;
const RHINO_CHARGE_CRASH_STAGGER_STRENGTH: u32 = 3;
const RHINO_CHARGE_CRASH_STAGGER_DURATION: f32 = 0.75;
const RHINO_CHARGE_CRASH_KNOCKBACK: f32 = 1.5;

#[derive(Component, Default)]
struct RhinoChargeAbility {
    cooldown: f32,
}

/// Marks the player as mid-charge and carries the numbers that frame's
/// `update_rhino_charge` needs -- mirrors `weapons::abilities::longsword::BladeRush`,
/// which uses the exact same "snapshot stats onto a marker component,
/// drain `hit_enemies` as the player passes through" shape.
#[derive(Component)]
struct RhinoCharge {
    hit_enemies: Vec<Entity>,
    damage: f32,
    knockback: f32,
    stagger_strength: u32,
    stagger_duration: f32,
}

fn rhino_charge_input(
    mut commands: Commands,
    mut caster_query: Query<
        (
            Entity,
            &mut Transform,
            &mut PlayerIntent,
            &Equipment,
            &mut RhinoChargeAbility,
            &AbilityMovement,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Wildshaper>),
    >,
) {
    for (
        caster_entity,
        mut transform,
        mut intent,
        equipment,
        mut ability,
        current_movement,
        mut ability_tracker,
    ) in &mut caster_query
    {
        // A press (Q, for the local player) is consumed even when it can't
        // cast, as `just_pressed` was before.
        if !intent.take_class_ability(RHINO_CHARGE_SLOT) {
            continue;
        }
        let camera_transform = intent.aim_transform();

        // Don't stomp an already-active movement ability (a weapon technique
        // like Blade Rush, or another Wildshaper ability), mirroring Vanguard's
        // Lunge guard against the same shared `AbilityMovement` component.
        if ability.cooldown > 0.0 || current_movement.is_active() {
            continue;
        }

        let mut direction = camera_transform.forward().as_vec3();
        direction.y = 0.0;
        direction = direction.normalize_or_zero();

        if direction == Vec3::ZERO {
            continue;
        }

        transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));

        let ability_power = equipment.ability_power_multiplier();

        commands.entity(caster_entity).insert((
            AbilityMovement {
                direction,
                speed: RHINO_CHARGE_SPEED,
                time_remaining: RHINO_CHARGE_DURATION,
            },
            RhinoCharge {
                hit_enemies: Vec::new(),
                damage: RHINO_CHARGE_BASE_DAMAGE * ability_power,
                knockback: RHINO_CHARGE_KNOCKBACK,
                stagger_strength: RHINO_CHARGE_STAGGER_STRENGTH,
                stagger_duration: RHINO_CHARGE_STAGGER_DURATION,
            },
        ));

        ability.cooldown = RHINO_CHARGE_COOLDOWN;

        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }

        println!("RHINO CHARGE!");
    }
}

/// True if world geometry is within `distance` of `origin` along
/// `direction`. Used by both Rhino Charge (to trigger its wall crash) and
/// Panther Pounce (to stop the pounce at the wall instead of clipping
/// through it) -- the same swept-ahead-of-movement idea
/// `weapons::projectiles::movement` already uses for arrows, just as a
/// single lookahead check instead of continuous swept collision.
fn wall_ahead(
    spatial_query: &SpatialQuery,
    caster: Entity,
    origin: Vec3,
    direction: Vec3,
    distance: f32,
) -> bool {
    let Ok(ray_direction) = Dir3::new(direction.adjust_precision()) else {
        return false;
    };

    let filter = SpatialQueryFilter::from_excluded_entities([caster]).with_mask(GameLayer::World);

    spatial_query
        .cast_ray(
            origin.adjust_precision(),
            ray_direction,
            distance,
            true,
            &filter,
        )
        .is_some()
}

fn update_rhino_charge(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spatial_query: SpatialQuery,
    mut player_query: Query<
        (Entity, &Transform, &AbilityMovement, &mut RhinoCharge),
        (With<PlayerCharacter>, Without<Enemy>),
    >,
    mut enemy_query: Query<
        (
            Entity,
            &mut Transform,
            &mut Health,
            &mut Stagger,
            &mut EnemyCombat,
            Option<&EnemyDebuffs>,
        ),
        (With<Enemy>, Without<PlayerCharacter>),
    >,
) {
    for (player_entity, player_transform, ability_movement, mut charge) in &mut player_query {
        if !ability_movement.is_active() {
            // Charge finished on its own (ran out of time) -- immediately
            // back to humanoid form, same as Blade Rush's finish.
            commands.entity(player_entity).remove::<RhinoCharge>();
            continue;
        }

        // -----------------------------------------------------
        // Bulldoze anything in the charge's path.
        // -----------------------------------------------------

        for (
            enemy_entity,
            mut enemy_transform,
            mut enemy_health,
            mut stagger,
            mut enemy_combat,
            debuffs,
        ) in enemy_query.iter_mut()
        {
            if charge.hit_enemies.contains(&enemy_entity) {
                continue;
            }

            let mut to_enemy = enemy_transform.translation - player_transform.translation;
            to_enemy.y = 0.0;
            let distance = to_enemy.length();

            if distance > RHINO_CHARGE_HIT_RADIUS {
                continue;
            }

            combat::apply_enemy_damage(&mut enemy_health, debuffs, charge.damage);
            charge.hit_enemies.push(enemy_entity);

            let direction_to_enemy = to_enemy.normalize_or_zero();
            let effective_resistance =
                combat::effective_stagger_resistance(stagger.resistance, debuffs);

            if charge.stagger_strength > effective_resistance {
                stagger.timer = charge.stagger_duration;
                enemy_combat.state = EnemyState::Chase;
                enemy_combat.timer = 0.0;
                enemy_combat.has_hit = false;
            }

            // Ordinary enemies get bulldozed much further than an elite/boss
            // would -- the closest thing to "launched" this project's kinematic
            // enemies currently support (see the module doc comment above).
            let knockback = if effective_resistance <= RHINO_CHARGE_LAUNCH_RESISTANCE_THRESHOLD {
                charge.knockback * RHINO_CHARGE_LAUNCH_KNOCKBACK_MULTIPLIER
            } else {
                charge.knockback
            };

            if direction_to_enemy != Vec3::ZERO {
                enemy_transform.translation += direction_to_enemy * knockback;
            }

            println!(
                "Rhino Charge bulldozed an enemy! Damage {:.1}, HP {:.0}/{:.0}",
                charge.damage, enemy_health.current, enemy_health.max
            );
        }

        // -----------------------------------------------------
        // Wall crash check.
        // -----------------------------------------------------

        if wall_ahead(
            &spatial_query,
            player_entity,
            player_transform.translation,
            ability_movement.direction,
            RHINO_CHARGE_WALL_CHECK_DISTANCE,
        ) {
            let crash_center = player_transform.translation + ability_movement.direction * 0.5;

            for (
                _enemy_entity,
                mut enemy_transform,
                mut enemy_health,
                mut stagger,
                mut enemy_combat,
                debuffs,
            ) in enemy_query.iter_mut()
            {
                let mut to_enemy = enemy_transform.translation - crash_center;
                to_enemy.y = 0.0;
                let distance = to_enemy.length();

                if distance > RHINO_CHARGE_CRASH_RADIUS {
                    continue;
                }

                combat::apply_enemy_damage(&mut enemy_health, debuffs, charge.damage * 0.5);

                let effective_resistance =
                    combat::effective_stagger_resistance(stagger.resistance, debuffs);

                if RHINO_CHARGE_CRASH_STAGGER_STRENGTH > effective_resistance {
                    stagger.timer = RHINO_CHARGE_CRASH_STAGGER_DURATION;
                    enemy_combat.state = EnemyState::Chase;
                    enemy_combat.timer = 0.0;
                    enemy_combat.has_hit = false;
                }

                let direction_to_enemy = to_enemy.normalize_or_zero();
                if direction_to_enemy != Vec3::ZERO {
                    enemy_transform.translation +=
                        direction_to_enemy * RHINO_CHARGE_CRASH_KNOCKBACK;
                }
            }

            commands.spawn((
                WildshaperVisualEffect { lifetime: 0.35 },
                effect_mesh(
                    &mut *meshes,
                    EffectShape::Sphere {
                        radius: RHINO_CHARGE_CRASH_RADIUS * 0.5,
                    },
                ),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(0.75, 0.55, 0.3, 0.4),
                    emissive: LinearRgba::new(0.6, 0.4, 0.15, 1.0),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                })),
                Transform::from_translation(crash_center),
            ));

            println!("RHINO CHARGE crashes into a wall!");

            commands.entity(player_entity).remove::<RhinoCharge>();
            commands.entity(player_entity).insert(AbilityMovement {
                direction: Vec3::ZERO,
                speed: 0.0,
                time_remaining: 0.0,
            });
        }
    }
}

// =========================================================
// BEAR SLAM (E) -- power + stagger
//
// Transform into a bear and slam the ground around the caster. A brief
// windup telegraphs the impact area -- matching the project's "every
// dangerous attack needs a clear visual telegraph" rule -- before dealing
// heavy damage and strong stagger to everything caught in the radius. The
// telegraph disk and the damage radius always use the same number
// (`BEAR_SLAM_RADIUS`), the same rule Saint of the Broken Siege's Fist
// Slam follows for its shockwave.
// =========================================================

const BEAR_SLAM_COOLDOWN: f32 = 9.0;
/// Short on purpose -- Wildshaper transformations are fast attacks, not
/// slow wind-ups. Still long enough to read as a telegraph rather than an
/// instant, unavoidable hit.
const BEAR_SLAM_WINDUP: f32 = 0.22;
const BEAR_SLAM_RADIUS: f32 = 4.0;
const BEAR_SLAM_BASE_DAMAGE: f32 = 34.0;
const BEAR_SLAM_KNOCKBACK: f32 = 2.6;
/// Comfortably above elite resistance (~2) and close to boss resistance
/// (~4, per the context doc's stagger table) -- Bear Slam is Wildshaper's
/// dedicated interrupt/power tool, matching its "power + stagger" role.
const BEAR_SLAM_STAGGER_STRENGTH: u32 = 3;
const BEAR_SLAM_STAGGER_DURATION: f32 = 0.9;

/// Ruinbound doesn't have a formal player-interruption system yet (nothing
/// currently cancels an in-progress ability once started), so the context
/// doc's "brief resistance to interruption during the transformation" is
/// satisfied by construction -- once the windup starts here, nothing in
/// the project can currently cancel it early. If a player-interrupt system
/// is added later, gate it on `BearSlamAbility::windup_remaining > 0.0`.
#[derive(Component, Default)]
struct BearSlamAbility {
    cooldown: f32,
    windup_remaining: f32,
}

/// The pulsing ground telegraph shown during the windup. Mirrors
/// `enemies::saint::SaintSlamTelegraph`'s pulse animation.
#[derive(Component)]
struct BearSlamTelegraph {
    lifetime: f32,
    total_lifetime: f32,
}

fn bear_slam_input(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut caster_query: Query<
        (
            &Transform,
            &mut PlayerIntent,
            &mut BearSlamAbility,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Wildshaper>),
    >,
) {
    for (transform, mut intent, mut ability, mut ability_tracker) in &mut caster_query {
        // E, for the local player.
        if !intent.take_class_ability(BEAR_SLAM_SLOT) {
            continue;
        }

        if ability.cooldown > 0.0 || ability.windup_remaining > 0.0 {
            continue;
        }

        ability.cooldown = BEAR_SLAM_COOLDOWN;
        ability.windup_remaining = BEAR_SLAM_WINDUP;

        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }

        commands.spawn((
            Name::new("Bear Slam Telegraph"),
            BearSlamTelegraph {
                lifetime: BEAR_SLAM_WINDUP,
                total_lifetime: BEAR_SLAM_WINDUP,
            },
            effect_mesh(
                &mut *meshes,
                EffectShape::Cylinder {
                    radius: BEAR_SLAM_RADIUS,
                    height: 0.035,
                },
            ),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.55, 0.35, 0.15, 0.3),
                emissive: LinearRgba::new(0.5, 0.28, 0.08, 1.0),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            // -1.0 is the player's capsule-center-to-feet offset (the same
            // constant `characters::animation`'s visual attachment uses), plus
            // a hair of clearance so the disk doesn't z-fight with the floor.
            Transform::from_translation(transform.translation + Vec3::Y * (0.05 - 1.0)),
        ));

        println!("Bear Slam winds up!");
    }
}

fn update_bear_slam_telegraphs(
    mut commands: Commands,
    time: Res<Time>,
    mut telegraphs: Query<(Entity, &mut BearSlamTelegraph, &mut Transform)>,
) {
    let delta = time.delta_secs();

    for (entity, mut telegraph, mut transform) in &mut telegraphs {
        telegraph.lifetime -= delta;

        if telegraph.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = 1.0 - telegraph.lifetime / telegraph.total_lifetime;
        let pulse = 0.92 + (progress * std::f32::consts::TAU * 3.0).sin().abs() * 0.1;
        transform.scale = Vec3::new(pulse, 1.0, pulse);
    }
}

fn update_bear_slam_impact(
    mut commands: Commands,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut casters: Query<
        (&Transform, &Equipment, &mut BearSlamAbility),
        (With<PlayerCharacter>, With<Wildshaper>, Without<Enemy>),
    >,
    mut enemy_query: Query<
        (
            &mut Transform,
            &mut Health,
            &mut Stagger,
            &mut EnemyCombat,
            Option<&EnemyDebuffs>,
        ),
        (With<Enemy>, Without<PlayerCharacter>),
    >,
) {
    let delta = time.delta_secs();

    for (transform, equipment, mut ability) in &mut casters {
        // Not currently winding up -- nothing to tick or trigger.
        if ability.windup_remaining <= 0.0 {
            continue;
        }

        ability.windup_remaining = (ability.windup_remaining - delta).max(0.0);

        // Still winding up -- the telegraph handles the visual, impact waits
        // for the windup to actually finish.
        if ability.windup_remaining > 0.0 {
            continue;
        }

        let center = transform.translation;
        let ability_power = equipment.ability_power_multiplier();
        let damage = BEAR_SLAM_BASE_DAMAGE * ability_power;

        println!("BEAR SLAM!");

        for (mut enemy_transform, mut enemy_health, mut stagger, mut enemy_combat, debuffs) in
            enemy_query.iter_mut()
        {
            let mut to_enemy = enemy_transform.translation - center;
            to_enemy.y = 0.0;
            let distance = to_enemy.length();

            if distance > BEAR_SLAM_RADIUS {
                continue;
            }

            combat::apply_enemy_damage(&mut enemy_health, debuffs, damage);

            let effective_resistance =
                combat::effective_stagger_resistance(stagger.resistance, debuffs);

            if BEAR_SLAM_STAGGER_STRENGTH > effective_resistance {
                stagger.timer = BEAR_SLAM_STAGGER_DURATION;
                enemy_combat.state = EnemyState::Chase;
                enemy_combat.timer = 0.0;
                enemy_combat.has_hit = false;
            }

            let direction_to_enemy = to_enemy.normalize_or_zero();
            if direction_to_enemy != Vec3::ZERO {
                enemy_transform.translation += direction_to_enemy * BEAR_SLAM_KNOCKBACK;
            }

            println!(
                "Bear Slam hit! Damage {:.1}, HP {:.0}/{:.0}",
                damage, enemy_health.current, enemy_health.max
            );
        }

        // Impact burst, sized to the same radius the damage check just used --
        // the visible leading edge and the gameplay hitbox never disagree.
        commands.spawn((
            WildshaperVisualEffect { lifetime: 0.3 },
            effect_mesh(
                &mut *meshes,
                EffectShape::Cylinder {
                    radius: BEAR_SLAM_RADIUS,
                    height: 0.12,
                },
            ),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.65, 0.42, 0.18, 0.5),
                emissive: LinearRgba::new(0.55, 0.32, 0.1, 1.0),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(center + Vec3::Y * (0.06 - 1.0)),
        ));
    }
}

// =========================================================
// PANTHER POUNCE (R) -- mobility + precision aggression
//
// Transform into a panther and leap toward the best available enemy,
// passing through it and landing beyond it. Target selection mirrors
// Vanguard's Lunge (closest enemy within a forward aim cone), but unlike
// Lunge -- which stops short of its target -- the pounce carries the
// player past it, matching "can end behind or beyond the target."
// =========================================================

const PANTHER_POUNCE_COOLDOWN: f32 = 7.0;
const PANTHER_POUNCE_SPEED: f32 = 24.0;
const PANTHER_POUNCE_RANGE: f32 = 12.0;
const PANTHER_POUNCE_MIN_AIM_DOT: f32 = 0.7;
/// Minimum distance to a candidate target -- avoids picking (and pouncing
/// oddly close past) something already adjacent to the caster.
const PANTHER_POUNCE_STOP_DISTANCE: f32 = 1.0;
/// How far past the target the pounce carries the player.
const PANTHER_POUNCE_OVERSHOOT: f32 = 2.0;
const PANTHER_POUNCE_HIT_RADIUS: f32 = 1.3;
const PANTHER_POUNCE_BASE_DAMAGE: f32 = 30.0;
const PANTHER_POUNCE_KNOCKBACK: f32 = 1.0;
const PANTHER_POUNCE_STAGGER_STRENGTH: u32 = 1;
const PANTHER_POUNCE_STAGGER_DURATION: f32 = 0.3;
/// How far ahead to check for a wall each frame -- see `wall_ahead` above.
const PANTHER_POUNCE_WALL_CHECK_DISTANCE: f32 = 1.2;

#[derive(Component, Default)]
struct PantherPounceAbility {
    cooldown: f32,
}

#[derive(Component)]
struct PantherPounce {
    hit_enemies: Vec<Entity>,
    damage: f32,
}

fn panther_pounce_input(
    mut commands: Commands,
    enemies: Query<&Transform, (With<Enemy>, Without<PlayerCharacter>)>,
    mut caster_query: Query<
        (
            Entity,
            &mut Transform,
            &mut PlayerIntent,
            &Equipment,
            &mut PantherPounceAbility,
            &AbilityMovement,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Wildshaper>, Without<Enemy>),
    >,
) {
    for (
        caster_entity,
        mut player_transform,
        mut intent,
        equipment,
        mut ability,
        current_movement,
        mut ability_tracker,
    ) in &mut caster_query
    {
        // R, for the local player.
        if !intent.take_class_ability(PANTHER_POUNCE_SLOT) {
            continue;
        }
        let camera_transform = intent.aim_transform();

        if ability.cooldown > 0.0 || current_movement.is_active() {
            continue;
        }

        let mut aim = camera_transform.forward().as_vec3();
        aim.y = 0.0;
        aim = aim.normalize_or_zero();

        // Same "closest enemy within a forward aim cone" heuristic as
        // Vanguard's Lunge (`characters::vanguard_abilities::lunge_input`).
        let mut best: Option<(f32, f32, Vec3)> = None;
        for enemy_transform in &enemies {
            let mut offset = enemy_transform.translation - player_transform.translation;
            offset.y = 0.0;
            let distance = offset.length();

            if distance <= PANTHER_POUNCE_STOP_DISTANCE || distance > PANTHER_POUNCE_RANGE {
                continue;
            }

            let direction = offset.normalize_or_zero();
            let dot = aim.dot(direction);

            if dot < PANTHER_POUNCE_MIN_AIM_DOT {
                continue;
            }

            if best.is_none_or(|(best_dot, best_distance, _)| {
                dot > best_dot + 0.02
                    || ((dot - best_dot).abs() <= 0.02 && distance < best_distance)
            }) {
                best = Some((dot, distance, direction));
            }
        }

        let Some((_, distance, direction)) = best else {
            println!("Panther Pounce: no target in range");
            continue;
        };

        player_transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));

        let travel_distance = distance + PANTHER_POUNCE_OVERSHOOT;

        commands.entity(caster_entity).insert((
            AbilityMovement {
                direction,
                speed: PANTHER_POUNCE_SPEED,
                time_remaining: (travel_distance / PANTHER_POUNCE_SPEED).max(0.05),
            },
            PantherPounce {
                hit_enemies: Vec::new(),
                damage: PANTHER_POUNCE_BASE_DAMAGE * equipment.ability_power_multiplier(),
            },
        ));

        ability.cooldown = PANTHER_POUNCE_COOLDOWN;

        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }

        println!("PANTHER POUNCE!");
    }
}

fn update_panther_pounce(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    mut player_query: Query<
        (Entity, &Transform, &AbilityMovement, &mut PantherPounce),
        (With<PlayerCharacter>, Without<Enemy>),
    >,
    mut enemy_query: Query<
        (
            Entity,
            &mut Transform,
            &mut Health,
            &mut Stagger,
            &mut EnemyCombat,
            Option<&EnemyDebuffs>,
        ),
        (With<Enemy>, Without<PlayerCharacter>),
    >,
) {
    for (player_entity, player_transform, ability_movement, mut pounce) in &mut player_query {
        if !ability_movement.is_active() {
            // Landed beyond the target -- immediately back to humanoid form.
            commands.entity(player_entity).remove::<PantherPounce>();
            continue;
        }

        for (
            enemy_entity,
            mut enemy_transform,
            mut enemy_health,
            mut stagger,
            mut enemy_combat,
            debuffs,
        ) in enemy_query.iter_mut()
        {
            if pounce.hit_enemies.contains(&enemy_entity) {
                continue;
            }

            let mut to_enemy = enemy_transform.translation - player_transform.translation;
            to_enemy.y = 0.0;
            let distance = to_enemy.length();

            if distance > PANTHER_POUNCE_HIT_RADIUS {
                continue;
            }

            combat::apply_enemy_damage(&mut enemy_health, debuffs, pounce.damage);
            pounce.hit_enemies.push(enemy_entity);

            let effective_resistance =
                combat::effective_stagger_resistance(stagger.resistance, debuffs);

            if PANTHER_POUNCE_STAGGER_STRENGTH > effective_resistance {
                stagger.timer = PANTHER_POUNCE_STAGGER_DURATION;
                enemy_combat.state = EnemyState::Chase;
                enemy_combat.timer = 0.0;
                enemy_combat.has_hit = false;
            }

            let direction_to_enemy = to_enemy.normalize_or_zero();
            if direction_to_enemy != Vec3::ZERO {
                enemy_transform.translation += direction_to_enemy * PANTHER_POUNCE_KNOCKBACK;
            }

            println!(
                "Panther Pounce struck! Damage {:.1}, HP {:.0}/{:.0}",
                pounce.damage, enemy_health.current, enemy_health.max
            );
        }

        // Land at the wall instead of clipping through it if the pounce would
        // otherwise carry the player into solid geometry beyond the target.
        if wall_ahead(
            &spatial_query,
            player_entity,
            player_transform.translation,
            ability_movement.direction,
            PANTHER_POUNCE_WALL_CHECK_DISTANCE,
        ) {
            commands.entity(player_entity).insert(AbilityMovement {
                direction: Vec3::ZERO,
                speed: 0.0,
                time_remaining: 0.0,
            });
        }
    }
}

// =========================================================
// SHARED VISUAL-EFFECT CLEANUP
// =========================================================

/// Generic short-lived cosmetic mesh spawned by Rhino Charge's wall-crash
/// burst and Bear Slam's impact burst. Mirrors Warden's
/// `WardenVisualEffect` and Runesage's `RunesageVisualEffect`.
#[derive(Component)]
struct WildshaperVisualEffect {
    lifetime: f32,
}

fn update_wildshaper_visual_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut effects: Query<(Entity, &mut WildshaperVisualEffect)>,
) {
    let delta = time.delta_secs();

    for (entity, mut effect) in &mut effects {
        effect.lifetime -= delta;

        if effect.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

// =========================================================
// SHARED WILDSHAPER STATE / HUD
// =========================================================

fn initialize_wildshaper_abilities(
    mut commands: Commands,
    // Every player character (a joined Wildshaper too, via `net::loadout`).
    players: Query<Entity, (With<PlayerCharacter>, Added<Wildshaper>)>,
) {
    for player in &players {
        commands.entity(player).insert((
            RhinoChargeAbility::default(),
            BearSlamAbility::default(),
            PantherPounceAbility::default(),
            ClassAbilityHud::new([
                ClassAbilityHudSlot::new("Q", "RHINO CHARGE", ClassAbilityDisplay::Ready),
                ClassAbilityHudSlot::new("E", "BEAR SLAM", ClassAbilityDisplay::Ready),
                ClassAbilityHudSlot::new("R", "PANTHER POUNCE", ClassAbilityDisplay::Ready),
            ]),
            AbilityMovement {
                direction: Vec3::ZERO,
                speed: 0.0,
                time_remaining: 0.0,
            },
        ));
    }
}

fn update_wildshaper_timers(
    time: Res<Time>,
    mut wildshapers: Query<
        (
            &mut RhinoChargeAbility,
            &mut BearSlamAbility,
            &mut PantherPounceAbility,
        ),
        With<Wildshaper>,
    >,
) {
    let delta = time.delta_secs();

    for (mut rhino, mut bear, mut panther) in &mut wildshapers {
        rhino.cooldown = (rhino.cooldown - delta).max(0.0);
        bear.cooldown = (bear.cooldown - delta).max(0.0);
        panther.cooldown = (panther.cooldown - delta).max(0.0);
    }
}

fn sync_wildshaper_class_state(
    mut wildshapers: Query<
        (
            &RhinoChargeAbility,
            &BearSlamAbility,
            &PantherPounceAbility,
            &mut ClassAbilityHud,
        ),
        (With<PlayerCharacter>, With<Wildshaper>),
    >,
) {
    for (rhino, bear, panther, mut hud) in &mut wildshapers {
        hud.slots[0].display = if rhino.cooldown <= 0.0 {
            ClassAbilityDisplay::Ready
        } else {
            ClassAbilityDisplay::Cooldown(rhino.cooldown)
        };

        hud.slots[1].display = if bear.windup_remaining > 0.0 {
            ClassAbilityDisplay::Active("SLAMMING")
        } else if bear.cooldown <= 0.0 {
            ClassAbilityDisplay::Ready
        } else {
            ClassAbilityDisplay::Cooldown(bear.cooldown)
        };

        hud.slots[2].display = if panther.cooldown <= 0.0 {
            ClassAbilityDisplay::Ready
        } else {
            ClassAbilityDisplay::Cooldown(panther.cooldown)
        };
    }
}

// =========================================================
// PLUGIN
// =========================================================

pub struct WildshaperAbilitiesPlugin;

impl Plugin for WildshaperAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                initialize_wildshaper_abilities,
                update_wildshaper_timers,
                rhino_charge_input,
                update_rhino_charge,
                bear_slam_input,
                update_bear_slam_telegraphs,
                update_bear_slam_impact,
                panther_pounce_input,
                update_panther_pounce,
                update_wildshaper_visual_effects,
                sync_wildshaper_class_state,
            )
                .chain(),
        );
    }
}
