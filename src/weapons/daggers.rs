//! Daggers -- see WEAPONS.md's Daggers section for the design source of
//! truth. Identity: extremely fast, short-range, low-stagger melee built
//! around mobility, backstab bonus damage, and bleed/poison effects
//! rather than the stagger/knockback identity every other melee family
//! leans on (WEAPONS.md: "Daggers should have high sustained damage but
//! require the player to stay close and avoid taking hits").
//!
//! Every tier is implemented here: the basic attack (Twin Strike, a
//! pair of fast stabs resolved as one swing), Common (Twin Fangs' Rapid
//! Flurry), Rare (Bloodletters' Serrated Rush), Epic (Phantom Blades'
//! Phantom Step), and all three Legendaries (Kingslayer, Widow's Kiss,
//! Dancer in the Dark).
//!
//! New shared mechanics introduced for this family, written to be
//! reusable by any future weapon rather than Daggers-only:
//! - `is_attack_from_behind` -- generic backstab-angle detection.
//! - `combat::DotSource::{Bleed, Venom}` and `DamageOverTime::has_source`
//!   (see `combat::dot`).
//! - `combat::DebuffSource::Envenomed` and `EnemyDebuffs::grant_with_slow`
//!   -- the first debuff in the project that slows an enemy (see
//!   `combat::enemy_debuffs`).

use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::visual_effects::{EffectShape, effect_mesh};

use crate::combat::{
    self, DamageOverTime, DebuffSource, DotSource, EnemyDebuffs, FrontalGuard, Health,
};
use crate::enemies::{Enemy, EnemyCombat, EnemyState, Stagger};
use crate::items::ItemDefinitionId;
use crate::player::{AbilityMovement, Dodge, Player, PlayerCharacter};

use super::components::{
    PlayerCombat, PlayerProjectile, Projectile, ProjectileKind, Weapon, WeaponAbilityState,
};
use super::melee::spawn_melee_arc;

// =========================================================
// SHARED
// =========================================================

/// Enemies whose `Stagger::resistance` is at or above this count as
/// "elite or boss" for every Daggers mechanic that treats the two tiers
/// the same way (Serrated Rush's "elites cannot be passed through",
/// Kingslayer's "cannot be instantly executed", Phantom Step's damage
/// cap). Matches the same cutoff Saint's Nail already uses for its own
/// "elite or boss" check (see
/// `weapons::crossbow::SAINTS_NAIL_ELITE_RESISTANCE_THRESHOLD`) and the
/// project's own stagger-resistance scale (context doc section 12.1:
/// normal ~0, elite ~2, boss ~4).
pub(crate) const DAGGER_ELITE_RESISTANCE_THRESHOLD: u32 = 2;

/// Cosine-of-angle threshold below which an incoming hit counts as a
/// backstab. `0.2` -- roughly a 78-degree rear arc -- generous enough
/// that "get behind the enemy" doesn't require frame-perfect
/// positioning, narrow enough that circling in front doesn't qualify.
/// Deliberately independent of `combat::FrontalGuard`'s own frontal-arc
/// math (that component only exists on a handful of shielded enemies
/// and is about blocking, not backstabs); every enemy can be backstabbed
/// by Daggers regardless of whether it also happens to carry a guard.
const DAGGER_BACKSTAB_DOT_THRESHOLD: f32 = 0.2;

/// True if `attacker_position` is within the rear arc of whatever is
/// facing `enemy_rotation` from `enemy_position`. Every Daggers mechanic
/// that cares about backstabs (Serrated Rush, Phantom Step, Kingslayer,
/// Shadowstep Dance) shares this one geometry check rather than each
/// reimplementing it.
pub(crate) fn is_attack_from_behind(
    enemy_rotation: Quat,
    enemy_position: Vec3,
    attacker_position: Vec3,
) -> bool {
    let mut to_attacker = attacker_position - enemy_position;
    to_attacker.y = 0.0;
    let to_attacker = to_attacker.normalize_or_zero();

    if to_attacker == Vec3::ZERO {
        return false;
    }

    let facing = enemy_rotation * Vec3::Z;

    facing.dot(to_attacker) <= DAGGER_BACKSTAB_DOT_THRESHOLD
}

/// Marks an enemy for Kingslayer's Deathmark ability (Legendary Daggers).
/// Decayed by `tick_dagger_marks`. Read-only everywhere it's consumed --
/// the mark itself doesn't track Execution stacks (those are per-player,
/// see `WeaponAbilityState::kingslayer_execution_stacks`), it only
/// answers "is this enemy currently marked."
#[derive(Component, Debug)]
pub(crate) struct Deathmark {
    remaining: f32,
}

/// Decays every active Deathmark, removing it once its duration expires.
/// Registered by `weapons::WeaponPlugin`. Mirrors
/// `weapons::crossbow::tick_resonance`.
pub(crate) fn tick_dagger_marks(
    mut commands: Commands,
    time: Res<Time>,
    mut marks: Query<(Entity, &mut Deathmark)>,
) {
    let delta = time.delta_secs();
    for (entity, mut mark) in &mut marks {
        mark.remaining -= delta;
        if mark.remaining <= 0.0 {
            commands.entity(entity).remove::<Deathmark>();
        }
    }
}

/// A pending, delayed strike spawned by Dancer in the Dark's shadow
/// copies (Shadowstep Dance's echoes, and Perfect Evasion's shadow
/// duplicate) -- a small visible marker entity that deals its damage to
/// `target` once `delay` elapses, then despawns. One shared component
/// for both sources rather than two near-identical ones.
#[derive(Component)]
pub(crate) struct ShadowCopy {
    target: Entity,
    delay: f32,
    damage: f32,
}

/// Resolves every pending `ShadowCopy` strike once its delay elapses.
/// Registered by `weapons::WeaponPlugin`.
pub(crate) fn update_shadow_copies(
    mut commands: Commands,
    time: Res<Time>,
    mut copies: Query<(Entity, &mut ShadowCopy)>,
    mut enemies: Query<(&mut Health, Option<&EnemyDebuffs>), With<Enemy>>,
) {
    let delta = time.delta_secs();

    for (copy_entity, mut copy) in &mut copies {
        copy.delay -= delta;

        if copy.delay > 0.0 {
            continue;
        }

        if let Ok((mut health, debuffs)) = enemies.get_mut(copy.target) {
            combat::apply_enemy_damage(&mut health, debuffs, copy.damage);
            println!("Shadow copy strikes!");
        }

        commands.entity(copy_entity).despawn();
    }
}

fn spawn_shadow_copy_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    target: Entity,
    delay: f32,
    damage: f32,
) {
    commands.spawn((
        ShadowCopy {
            target,
            delay,
            damage,
        },
        effect_mesh(&mut *meshes, EffectShape::Sphere { radius: 0.32 }),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.35, 0.15, 0.55, 0.55),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_translation(position),
    ));
}

// =========================================================
// BASIC ATTACK -- Twin Strike
// =========================================================

/// Two quick stabs resolved together as one swing (WEAPONS.md's
/// "extremely fast attacks... high sustained damage") rather than a
/// windup-then-hit like Greatsword/Hammer, or a multi-tick sequence like
/// Rapid Flurry below -- the basic attack stays instantaneous, matching
/// every other melee weapon's basic attack.
const DAGGER_BASIC_ATTACK_MULTIPLIER: f32 = 1.3;
const DAGGER_BASIC_VISUAL_DURATION: f32 = 0.08;

const KINGSLAYER_BACKSTAB_MULTIPLIER: f32 = 1.5;
pub(crate) const KINGSLAYER_MAX_EXECUTION_STACKS: u32 = 5;
const KINGSLAYER_BURST_MULTIPLIER: f32 = 3.0;
/// Normal enemies at or below this fraction of max health die outright
/// when backstabbed while Deathmarked (WEAPONS.md: "Normal enemies below
/// a health threshold can be executed").
const KINGSLAYER_EXECUTE_HEALTH_FRACTION: f32 = 0.2;
const KINGSLAYER_RUTHLESS_OPENING_MULTIPLIER: f32 = 1.75;
/// Ruthless Opening reads WEAPONS.md's "The first attack against a
/// full-health elite or boss" as a cooldown-gated bonus rather than a
/// true once-ever proc -- the same simplification Saint's Nail's Final
/// Judgment already established for an identical "the first X after Y"
/// wording (see `weapons::crossbow::SAINTS_NAIL_JUDGMENT_COOLDOWN`).
const KINGSLAYER_RUTHLESS_OPENING_COOLDOWN: f32 = 18.0;

const WIDOWS_KISS_BLEED_BONUS_MULTIPLIER: f32 = 1.25;

/// Phantom Step's "the empowered attack deals heavy bonus damage" --
/// capped against elites/bosses per WEAPONS.md's "Bosses cannot be
/// instantly executed by the backstab." Ruinbound has no literal execute
/// on this ability, so that line is read as "the bonus itself is capped
/// against tough enemies" rather than a separate execute check, keeping
/// this Epic-tier ability meaningfully weaker than Kingslayer's actual
/// Legendary-tier execution effect.
const PHANTOM_STEP_DAMAGE_MULTIPLIER: f32 = 1.9;
const PHANTOM_STEP_ELITE_DAMAGE_MULTIPLIER: f32 = 1.3;
/// "Hitting an enemy from behind creates a second phantom strike" --
/// applied as one combined damage number on the same hit rather than a
/// second full attack pass, so it can never double-trigger Kingslayer's
/// Execution stacks or Widow's Kiss's Bleed bonus on the same swing.
const PHANTOM_STEP_SECOND_STRIKE_FRACTION: f32 = 0.6;

const DANCER_SHADOW_DUPLICATE_DAMAGE_FRACTION: f32 = 0.7;
const DANCER_SHADOW_DUPLICATE_DELAY: f32 = 0.15;

#[allow(clippy::too_many_arguments)]
pub(crate) fn daggers_basic_attack(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    damage_multiplier: f32,
    mut weapon_ability_state: Option<&mut WeaponAbilityState>,
    enemy_query: &mut Query<
        (
            Entity,
            &mut Transform,
            &mut Health,
            &mut Stagger,
            &mut EnemyCombat,
            Option<&EnemyDebuffs>,
            Option<&FrontalGuard>,
            Option<&DamageOverTime>,
            Option<&Deathmark>,
        ),
        (With<Enemy>, Without<Player>),
    >,
) -> bool {
    let mut attack_direction = camera_transform.forward().as_vec3();

    attack_direction.y = 0.0;
    attack_direction = attack_direction.normalize_or_zero();

    if attack_direction == Vec3::ZERO {
        return false;
    }

    player_transform.rotation = Quat::from_rotation_y(attack_direction.x.atan2(attack_direction.z));

    spawn_melee_arc(
        commands,
        meshes,
        materials,
        player_transform,
        attack_direction,
        weapon.range,
        DAGGER_BASIC_VISUAL_DURATION,
    );
    spawn_melee_arc(
        commands,
        meshes,
        materials,
        player_transform,
        attack_direction,
        weapon.range * 0.85,
        DAGGER_BASIC_VISUAL_DURATION * 1.4,
    );

    let is_kingslayer = weapon.metadata.definition_id == KINGSLAYER_ID;
    let is_widows_kiss = weapon.metadata.definition_id == WIDOWS_KISS_ID;
    let is_dancer = weapon.metadata.definition_id == DANCER_IN_THE_DARK_ID;

    // Phantom Step (Epic Daggers) and Dancer in the Dark's Perfect
    // Evasion (Legendary passive) each grant a one-shot bonus consumed
    // by whichever dagger basic attack lands next -- resolved once per
    // activation here, not per enemy hit, so a multi-enemy swing can't
    // multiply either bonus.
    let mut phantom_step_empowered = false;
    let mut dancer_charge_ready = false;

    if let Some(state) = weapon_ability_state.as_deref_mut() {
        if state.phantom_step_empowered {
            state.phantom_step_empowered = false;
            phantom_step_empowered = true;
        }

        if is_dancer && state.dancer_shadow_charges >= DANCER_MAX_SHADOW_CHARGES {
            state.dancer_shadow_charges = 0;
            dancer_charge_ready = true;
        }
    }

    println!("Player attacks with Daggers");

    let mut hit_enemy = false;
    let mut duplicate_target: Option<(Entity, Vec3)> = None;

    for (
        entity,
        mut enemy_transform,
        mut enemy_health,
        mut stagger,
        mut enemy_combat,
        enemy_debuffs,
        frontal_guard,
        dot,
        deathmark,
    ) in enemy_query.iter_mut()
    {
        let mut to_enemy = enemy_transform.translation - player_transform.translation;

        to_enemy.y = 0.0;

        let distance = to_enemy.length();

        if distance > weapon.range {
            continue;
        }

        let direction_to_enemy = to_enemy.normalize_or_zero();

        if direction_to_enemy == Vec3::ZERO {
            continue;
        }

        let facing_dot = attack_direction.dot(direction_to_enemy);

        if facing_dot < weapon.attack_angle {
            continue;
        }

        hit_enemy = true;

        if duplicate_target.is_none() {
            duplicate_target = Some((entity, enemy_transform.translation));
        }

        let is_behind = is_attack_from_behind(
            enemy_transform.rotation,
            enemy_transform.translation,
            player_transform.translation,
        );

        let mut damage = weapon.damage * damage_multiplier * DAGGER_BASIC_ATTACK_MULTIPLIER;
        let mut executes_after_damage = false;

        if phantom_step_empowered {
            let is_elite_or_boss = stagger.resistance >= DAGGER_ELITE_RESISTANCE_THRESHOLD;

            damage *= if is_elite_or_boss {
                PHANTOM_STEP_ELITE_DAMAGE_MULTIPLIER
            } else {
                PHANTOM_STEP_DAMAGE_MULTIPLIER
            };

            if is_behind {
                damage += weapon.damage * damage_multiplier * PHANTOM_STEP_SECOND_STRIKE_FRACTION;
                println!("Phantom Step: second phantom strike!");
            }
        }

        if is_widows_kiss && dot.is_some_and(|d| d.has_source(DotSource::Bleed)) {
            damage *= WIDOWS_KISS_BLEED_BONUS_MULTIPLIER;
        }

        if is_kingslayer {
            if let Some(_mark) = deathmark {
                if is_behind {
                    let mut backstab_multiplier = KINGSLAYER_BACKSTAB_MULTIPLIER;

                    if let Some(state) = weapon_ability_state.as_deref_mut() {
                        state.kingslayer_execution_stacks += 1;

                        if state.kingslayer_execution_stacks >= KINGSLAYER_MAX_EXECUTION_STACKS {
                            state.kingslayer_execution_stacks = 0;
                            backstab_multiplier *= KINGSLAYER_BURST_MULTIPLIER;
                            println!("Kingslayer: Execution burst!");
                        }
                    }

                    damage *= backstab_multiplier;

                    let is_elite_or_boss = stagger.resistance >= DAGGER_ELITE_RESISTANCE_THRESHOLD;
                    executes_after_damage = !is_elite_or_boss;
                }
            } else {
                let is_elite_or_boss = stagger.resistance >= DAGGER_ELITE_RESISTANCE_THRESHOLD;
                let is_full_health = enemy_health.current >= enemy_health.max;

                if is_elite_or_boss && is_full_health {
                    if let Some(state) = weapon_ability_state.as_deref_mut() {
                        if state.kingslayer_ruthless_opening_cooldown <= 0.0 {
                            state.kingslayer_ruthless_opening_cooldown =
                                KINGSLAYER_RUTHLESS_OPENING_COOLDOWN;
                            state.kingslayer_execution_stacks = (state.kingslayer_execution_stacks
                                + 1)
                            .min(KINGSLAYER_MAX_EXECUTION_STACKS);
                            damage *= KINGSLAYER_RUTHLESS_OPENING_MULTIPLIER;
                            println!("Ruthless Opening!");
                        }
                    }
                }
            }
        }

        // Runebroken (see combat::enemy_debuffs) multiplies incoming
        // damage when the target has it active, and a FrontalGuard (see
        // combat::frontal_guard -- today only the Black Banner Knight)
        // can block or take bonus rear damage.
        let guarded = combat::apply_guarded_enemy_damage(
            &mut enemy_health,
            enemy_debuffs,
            frontal_guard,
            enemy_transform.rotation,
            enemy_transform.translation,
            player_transform.translation,
            weapon.stagger_strength,
            damage,
        );

        if executes_after_damage
            && enemy_health.current > 0.0
            && enemy_health.current <= enemy_health.max * KINGSLAYER_EXECUTE_HEALTH_FRACTION
        {
            enemy_health.current = 0.0;
            println!("Deathmark execution!");
        }

        // Daggers carry zero base stagger_strength (WEAPONS.md's speed-
        // over-stagger identity), so this is currently always a no-op --
        // kept for consistency with every other melee weapon's structure
        // and so a future gear bonus that grants Daggers real stagger
        // still behaves correctly without this file changing.
        if !guarded.stagger_negated
            && weapon.stagger_strength
                > combat::effective_stagger_resistance(stagger.resistance, enemy_debuffs)
        {
            stagger.timer = weapon.stagger_duration;

            enemy_combat.state = EnemyState::Chase;
            enemy_combat.timer = 0.0;
            enemy_combat.has_hit = false;
        }

        enemy_transform.translation += direction_to_enemy * weapon.knockback;

        println!(
            "Daggers hit! HP: {:.0}/{:.0}",
            enemy_health.current, enemy_health.max
        );
    }

    if dancer_charge_ready {
        if let Some((target_entity, target_position)) = duplicate_target {
            spawn_shadow_copy_visual(
                commands,
                meshes,
                materials,
                target_position,
                target_entity,
                DANCER_SHADOW_DUPLICATE_DELAY,
                weapon.damage * damage_multiplier * DANCER_SHADOW_DUPLICATE_DAMAGE_FRACTION,
            );
            println!("Perfect Evasion: shadow duplicate strikes!");
        }
    }

    hit_enemy
}

// =========================================================
// COMMON -- Twin Fangs: Rapid Flurry
// =========================================================

const FLURRY_HIT_COUNT: u32 = 3;
const FLURRY_HIT_INTERVAL: f32 = 0.11;
const FLURRY_HIT_DAMAGE_FRACTION: f32 = 0.55;
const FLURRY_FINISH_KNOCKBACK_MULTIPLIER: f32 = 2.5;
const FLURRY_ATTACK_ANGLE_DOT: f32 = 0.5;

#[derive(Component)]
pub(crate) struct DaggerFlurry {
    hits_remaining: u32,
    interval_timer: f32,
    direction: Vec3,
    damage: f32,
    range: f32,
    knockback: f32,
}

pub(crate) fn start_rapid_flurry(
    commands: &mut Commands,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    player_entity: Entity,
    weapon: &Weapon,
) {
    let mut direction = camera_transform.forward().as_vec3();

    direction.y = 0.0;
    direction = direction.normalize_or_zero();

    if direction == Vec3::ZERO {
        return;
    }

    player_transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));

    commands.entity(player_entity).insert(DaggerFlurry {
        hits_remaining: FLURRY_HIT_COUNT,
        // Fire the first strike on the very next tick rather than
        // waiting a full interval, so the ability feels instant on
        // press.
        interval_timer: 0.0,
        direction,
        damage: weapon.damage * FLURRY_HIT_DAMAGE_FRACTION,
        range: weapon.range,
        knockback: weapon.knockback,
    });

    println!("Rapid Flurry!");
}

/// Ticks an in-progress Rapid Flurry. Registered by `weapons::WeaponPlugin`.
pub(crate) fn update_dagger_flurry(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut player_query: Query<
        (Entity, &Transform, &Dodge, &mut DaggerFlurry),
        (With<PlayerCharacter>, Without<Enemy>),
    >,
    mut enemy_query: Query<
        (&mut Transform, &mut Health, Option<&EnemyDebuffs>),
        (With<Enemy>, Without<Player>),
    >,
) {
    for (player_entity, player_transform, dodge, mut flurry) in &mut player_query {
        // "Can be canceled into a dodge near the end of the attack." Dodging
        // is never blocked by an in-progress attack in this project (see
        // player::start_dodge), so simply ending the flurry the instant the
        // player actually dodges reads as "the dodge canceled it," without
        // needing a bespoke input-timing window.
        if dodge.is_dodging() {
            commands.entity(player_entity).remove::<DaggerFlurry>();
            continue;
        }

        flurry.interval_timer -= time.delta_secs();

        if flurry.interval_timer > 0.0 {
            continue;
        }

        flurry.interval_timer = FLURRY_HIT_INTERVAL;

        let is_final_hit = flurry.hits_remaining == 1;

        spawn_melee_arc(
            &mut commands,
            &mut meshes,
            &mut materials,
            player_transform,
            flurry.direction,
            flurry.range,
            FLURRY_HIT_INTERVAL * 0.9,
        );

        for (mut enemy_transform, mut enemy_health, enemy_debuffs) in enemy_query.iter_mut() {
            let mut to_enemy = enemy_transform.translation - player_transform.translation;

            to_enemy.y = 0.0;

            if to_enemy.length() > flurry.range {
                continue;
            }

            let direction_to_enemy = to_enemy.normalize_or_zero();

            if direction_to_enemy == Vec3::ZERO
                || flurry.direction.dot(direction_to_enemy) < FLURRY_ATTACK_ANGLE_DOT
            {
                continue;
            }

            combat::apply_enemy_damage(&mut enemy_health, enemy_debuffs, flurry.damage);

            if is_final_hit {
                enemy_transform.translation +=
                    direction_to_enemy * flurry.knockback * FLURRY_FINISH_KNOCKBACK_MULTIPLIER;
            }
        }

        flurry.hits_remaining -= 1;

        if flurry.hits_remaining == 0 {
            commands.entity(player_entity).remove::<DaggerFlurry>();
        }
    }
}

// =========================================================
// RARE -- Bloodletters: Serrated Rush
// =========================================================

const SERRATED_RUSH_SPEED: f32 = 16.0;
const SERRATED_RUSH_DURATION: f32 = 0.32;
const SERRATED_RUSH_HIT_RADIUS: f32 = 1.2;
const SERRATED_RUSH_DAMAGE_MULTIPLIER: f32 = 0.85;
/// "Deals bonus damage when striking an enemy from behind."
const SERRATED_RUSH_BACKSTAB_MULTIPLIER: f32 = 1.6;
const SERRATED_RUSH_BLEED_DAMAGE_PER_SECOND: f32 = 5.0;
const SERRATED_RUSH_BLEED_DURATION: f32 = 3.0;

#[derive(Component)]
pub(crate) struct SerratedRush {
    hit_enemies: Vec<Entity>,
    damage: f32,
    knockback: f32,
}

pub(crate) fn start_serrated_rush(
    commands: &mut Commands,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    player_entity: Entity,
    weapon: &Weapon,
) {
    let mut direction = camera_transform.forward().as_vec3();

    direction.y = 0.0;
    direction = direction.normalize_or_zero();

    if direction == Vec3::ZERO {
        return;
    }

    player_transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));

    commands.entity(player_entity).insert((
        AbilityMovement {
            direction,
            speed: SERRATED_RUSH_SPEED,
            time_remaining: SERRATED_RUSH_DURATION,
        },
        SerratedRush {
            hit_enemies: Vec::new(),
            damage: weapon.damage,
            knockback: weapon.knockback,
        },
    ));

    println!("Serrated Rush!");
}

/// Ticks an in-progress Serrated Rush. Registered by `weapons::WeaponPlugin`.
pub(crate) fn update_serrated_rush(
    mut commands: Commands,
    mut player_query: Query<
        (Entity, &Transform, &AbilityMovement, &mut SerratedRush),
        (With<PlayerCharacter>, Without<Enemy>),
    >,
    mut enemy_query: Query<
        (
            Entity,
            &mut Transform,
            &mut Health,
            &Stagger,
            Option<&EnemyDebuffs>,
            Option<&mut DamageOverTime>,
        ),
        (With<Enemy>, Without<Player>),
    >,
) {
    for (player_entity, player_transform, ability_movement, mut rush) in &mut player_query {
        if !ability_movement.is_active() {
            commands.entity(player_entity).remove::<SerratedRush>();
            commands.entity(player_entity).remove::<AbilityMovement>();
            continue;
        }

        let mut hit_an_elite = false;

        for (enemy_entity, mut enemy_transform, mut enemy_health, stagger, enemy_debuffs, dot) in
            enemy_query.iter_mut()
        {
            // Don't damage the same enemy repeatedly while passing through it.
            if rush.hit_enemies.contains(&enemy_entity) {
                continue;
            }

            let mut to_enemy = enemy_transform.translation - player_transform.translation;

            to_enemy.y = 0.0;

            if to_enemy.length() > SERRATED_RUSH_HIT_RADIUS {
                continue;
            }

            rush.hit_enemies.push(enemy_entity);

            let is_behind = is_attack_from_behind(
                enemy_transform.rotation,
                enemy_transform.translation,
                player_transform.translation,
            );

            let damage_multiplier = SERRATED_RUSH_DAMAGE_MULTIPLIER
                * if is_behind {
                    SERRATED_RUSH_BACKSTAB_MULTIPLIER
                } else {
                    1.0
                };

            combat::apply_enemy_damage(
                &mut enemy_health,
                enemy_debuffs,
                rush.damage * damage_multiplier,
            );

            match dot {
                Some(mut existing) => existing.apply(
                    DotSource::Bleed,
                    SERRATED_RUSH_BLEED_DAMAGE_PER_SECOND,
                    SERRATED_RUSH_BLEED_DURATION,
                ),
                None => {
                    commands
                        .entity(enemy_entity)
                        .insert(DamageOverTime::with_stack(
                            DotSource::Bleed,
                            SERRATED_RUSH_BLEED_DAMAGE_PER_SECOND,
                            SERRATED_RUSH_BLEED_DURATION,
                        ));
                }
            }

            let direction_to_enemy = to_enemy.normalize_or_zero();

            if direction_to_enemy != Vec3::ZERO {
                enemy_transform.translation += direction_to_enemy * rush.knockback * 0.5;
            }

            println!(
                "Serrated Rush passed through enemy! HP {:.0}/{:.0}",
                enemy_health.current, enemy_health.max,
            );

            // "Elites cannot be passed through, but still receive damage and
            // Bleed."
            if stagger.resistance >= DAGGER_ELITE_RESISTANCE_THRESHOLD {
                hit_an_elite = true;
            }
        }

        if hit_an_elite {
            commands.entity(player_entity).remove::<AbilityMovement>();
            commands.entity(player_entity).remove::<SerratedRush>();
        }
    }
}

// =========================================================
// EPIC -- Phantom Blades: Phantom Step
// =========================================================

const PHANTOM_STEP_RANGE: f32 = 9.0;
/// How far short of whatever the step's raycast hits (a wall, or an
/// enemy's own collider) the player lands, so Phantom Step ends beside
/// its target rather than inside it.
const PHANTOM_STEP_APPROACH_DISTANCE: f32 = 1.4;
const PHANTOM_STEP_MIN_TRAVEL_DISTANCE: f32 = 0.5;
/// "Grants brief protection during the step."
const PHANTOM_STEP_INVULNERABILITY_DURATION: f32 = 0.35;

pub(crate) fn start_phantom_step(
    commands: &mut Commands,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    player_entity: Entity,
    spatial_query: &SpatialQuery,
    dodge: &mut Dodge,
    weapon_ability_state: &mut WeaponAbilityState,
) {
    let mut direction = camera_transform.forward().as_vec3();

    direction.y = 0.0;
    direction = direction.normalize_or_zero();

    if direction == Vec3::ZERO {
        return;
    }

    let filter = SpatialQueryFilter::from_excluded_entities([player_entity]);
    let mut travel_distance = PHANTOM_STEP_RANGE;

    if let Ok(ray_direction) = Dir3::new(direction.adjust_precision()) {
        if let Some(hit) = spatial_query.cast_ray(
            player_transform.translation.adjust_precision(),
            ray_direction,
            PHANTOM_STEP_RANGE,
            true,
            &filter,
        ) {
            travel_distance = (hit.distance - PHANTOM_STEP_APPROACH_DISTANCE)
                .max(PHANTOM_STEP_MIN_TRAVEL_DISTANCE);
        }
    }

    player_transform.translation += direction * travel_distance;
    player_transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));

    // Reuses the same invulnerability window `player::Dodge` already
    // grants during a roll, rather than inventing a second damage-
    // immunity mechanism every enemy attack system would need to learn
    // about separately -- every enemy combat system already checks
    // `Dodge::is_invulnerable()` before landing a hit.
    dodge.invulnerability_remaining = dodge
        .invulnerability_remaining
        .max(PHANTOM_STEP_INVULNERABILITY_DURATION);

    weapon_ability_state.phantom_step_empowered = true;

    println!("Phantom Step!");
}

// =========================================================
// LEGENDARY -- Kingslayer: Deathmark
// =========================================================

pub(crate) const KINGSLAYER_ID: ItemDefinitionId =
    ItemDefinitionId("weapon.daggers.legendary.kingslayer");
pub(crate) const KINGSLAYER_NAME: &str = "Kingslayer";

const KINGSLAYER_MARK_RANGE: f32 = 9.0;
const KINGSLAYER_MARK_DURATION: f32 = 8.0;

pub(crate) fn cast_deathmark(
    commands: &mut Commands,
    camera_transform: &Transform,
    player_entity: Entity,
    spatial_query: &SpatialQuery,
    enemies: &Query<(Entity, &GlobalTransform), With<Enemy>>,
) {
    let origin = camera_transform.translation;
    let direction = camera_transform.forward().as_vec3().normalize_or_zero();

    if direction == Vec3::ZERO {
        return;
    }

    let Ok(ray_direction) = Dir3::new(direction.adjust_precision()) else {
        return;
    };

    let filter = SpatialQueryFilter::from_excluded_entities([player_entity]);

    let Some(hit) = spatial_query.cast_ray(
        origin.adjust_precision(),
        ray_direction,
        KINGSLAYER_MARK_RANGE,
        true,
        &filter,
    ) else {
        println!("Deathmark found no target.");
        return;
    };

    if enemies.get(hit.entity).is_err() {
        println!("Deathmark found no target.");
        return;
    }

    commands.entity(hit.entity).insert(Deathmark {
        remaining: KINGSLAYER_MARK_DURATION,
    });

    println!("Deathmark applied!");
}

// =========================================================
// LEGENDARY -- Widow's Kiss: Venomous Barrage
// =========================================================

pub(crate) const WIDOWS_KISS_ID: ItemDefinitionId =
    ItemDefinitionId("weapon.daggers.legendary.widows_kiss");
pub(crate) const WIDOWS_KISS_NAME: &str = "Widow's Kiss";

const WIDOWS_KISS_DAGGER_COUNT: u32 = 4;
const WIDOWS_KISS_SPREAD_RADIANS: f32 = 0.16;
const WIDOWS_KISS_DAGGER_SPEED: f32 = 34.0;
const WIDOWS_KISS_DAGGER_LIFETIME: f32 = 2.2;
const WIDOWS_KISS_DAMAGE_MULTIPLIER: f32 = 0.65;
pub(crate) const WIDOWS_KISS_VENOM_DPS: f32 = 5.0;
pub(crate) const WIDOWS_KISS_VENOM_DURATION: f32 = 5.0;
pub(crate) const WIDOWS_KISS_BLEED_DPS: f32 = 5.0;
pub(crate) const WIDOWS_KISS_BLEED_DURATION: f32 = 5.0;
/// "Venom and Bleed together cause the target to become Envenomed,
/// briefly slowing it."
pub(crate) const WIDOWS_KISS_ENVENOMED_SLOW_MULTIPLIER: f32 = 0.6;
pub(crate) const WIDOWS_KISS_ENVENOMED_DURATION: f32 = 2.5;
/// Spreading Agony passive: reduced-strength Venom/Bleed spread to
/// nearby enemies on a Bleeding/Venomed enemy's death.
const WIDOWS_KISS_SPREADING_AGONY_FRACTION: f32 = 0.5;
const WIDOWS_KISS_SPREADING_AGONY_RADIUS: f32 = 3.2;

pub(crate) fn fire_venomous_barrage(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
) {
    let mut direction = camera_transform.forward().as_vec3();

    direction.y = 0.0;
    direction = direction.normalize_or_zero();

    if direction == Vec3::ZERO {
        return;
    }

    player_transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));

    let spawn_position = player_transform.translation + Vec3::Y * 0.9 + direction * 0.6;
    let half_spread = (WIDOWS_KISS_DAGGER_COUNT as f32 - 1.0) / 2.0;

    for index in 0..WIDOWS_KISS_DAGGER_COUNT {
        let offset_radians = (index as f32 - half_spread) * WIDOWS_KISS_SPREAD_RADIANS;
        let dagger_direction = Quat::from_rotation_y(offset_radians) * direction;

        commands.spawn((
            PlayerProjectile,
            Projectile {
                velocity: dagger_direction * WIDOWS_KISS_DAGGER_SPEED,
                damage: weapon.damage * WIDOWS_KISS_DAMAGE_MULTIPLIER,
                knockback: weapon.knockback,
                stagger_strength: weapon.stagger_strength,
                stagger_duration: weapon.stagger_duration,
                lifetime: WIDOWS_KISS_DAGGER_LIFETIME,
                piercing: false,
                hit_entities: Vec::new(),
                kind: ProjectileKind::VenomousDagger,
                target: None,
            },
            Mesh3d(meshes.add(Cuboid::new(0.06, 0.06, 0.32))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.85, 0.35),
                emissive: Color::srgb(0.2, 0.6, 0.15).to_linear() * 1.5,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(spawn_position)
                .with_rotation(Quat::from_rotation_arc(Vec3::Z, dagger_direction)),
        ));
    }

    println!("Venomous Barrage!");
}

/// Spreading Agony: any enemy afflicted by Bleed or Venom that dies this
/// frame spreads reduced-strength Bleed and Venom to nearby enemies.
/// Only active while some player character has Widow's Kiss as their
/// currently equipped weapon, matching how every other Legendary's
/// passive is gated (see e.g.
/// `weapons::crossbow::ruinpiercer_cleansing_strike`). Bleed and Venom
/// don't record who applied them, so in co-op any wielder's passive
/// spreads any player's afflictions, once per death. Mirrors
/// `weapons::crossbow::saints_nail_mark_burst`'s "check `health.current
/// <= 0.0` independently of `enemies::handle_enemy_death`" shape.
/// Registered by `weapons::WeaponPlugin`.
pub(crate) fn widows_kiss_spreading_agony(
    mut commands: Commands,
    players: Query<&Weapon, With<PlayerCharacter>>,
    mut enemies: Query<(Entity, &Transform, &Health, Option<&mut DamageOverTime>), With<Enemy>>,
) {
    let anyone_wields_widows_kiss = players
        .iter()
        .any(|weapon| weapon.metadata.definition_id == WIDOWS_KISS_ID);
    if !anyone_wields_widows_kiss {
        return;
    }

    let dying_origins: Vec<(Entity, Vec3)> = enemies
        .iter()
        .filter(|(_, _, health, dot)| {
            health.current <= 0.0
                && dot.as_deref().is_some_and(|dot| {
                    dot.has_source(DotSource::Bleed) || dot.has_source(DotSource::Venom)
                })
        })
        .map(|(entity, transform, _, _)| (entity, transform.translation))
        .collect();

    if dying_origins.is_empty() {
        return;
    }

    for (source_entity, origin) in dying_origins {
        for (entity, transform, health, dot) in enemies.iter_mut() {
            if entity == source_entity || health.current <= 0.0 {
                continue;
            }

            if transform.translation.distance(origin) > WIDOWS_KISS_SPREADING_AGONY_RADIUS {
                continue;
            }

            let venom_dps = WIDOWS_KISS_VENOM_DPS * WIDOWS_KISS_SPREADING_AGONY_FRACTION;
            let bleed_dps = WIDOWS_KISS_BLEED_DPS * WIDOWS_KISS_SPREADING_AGONY_FRACTION;

            match dot {
                Some(mut existing) => {
                    existing.apply(DotSource::Venom, venom_dps, WIDOWS_KISS_VENOM_DURATION);
                    existing.apply(DotSource::Bleed, bleed_dps, WIDOWS_KISS_BLEED_DURATION);
                }
                None => {
                    let mut fresh = DamageOverTime::with_stack(
                        DotSource::Venom,
                        venom_dps,
                        WIDOWS_KISS_VENOM_DURATION,
                    );
                    fresh.apply(DotSource::Bleed, bleed_dps, WIDOWS_KISS_BLEED_DURATION);
                    commands.entity(entity).insert(fresh);
                }
            }
        }

        println!("Spreading Agony!");
    }
}

// =========================================================
// LEGENDARY -- Dancer in the Dark: Shadowstep Dance
// =========================================================

pub(crate) const DANCER_IN_THE_DARK_ID: ItemDefinitionId =
    ItemDefinitionId("weapon.daggers.legendary.dancer_in_the_dark");
pub(crate) const DANCER_IN_THE_DARK_NAME: &str = "Dancer in the Dark";

const DANCER_MAX_TARGETS: usize = 3;
const DANCER_RANGE: f32 = 10.0;
const DANCER_STEP_INTERVAL: f32 = 0.16;
const DANCER_FLANK_DISTANCE: f32 = 1.4;
const DANCER_STRIKE_DAMAGE_MULTIPLIER: f32 = 1.1;
const DANCER_ECHO_DAMAGE_FRACTION: f32 = 0.45;
const DANCER_ECHO_DELAY: f32 = 0.2;
/// "Striking enemies from behind reduces the ability's cooldown."
const DANCER_BACKSTAB_COOLDOWN_REFUND: f32 = 0.8;

pub(crate) const DANCER_MAX_SHADOW_CHARGES: u32 = 3;
/// How close the player must dodge to an enemy for Perfect Evasion to
/// count it as "dodging through" that enemy.
const DANCER_PERFECT_EVASION_RADIUS: f32 = 2.0;

#[derive(Component)]
pub(crate) struct ShadowstepDance {
    origin: Vec3,
    targets: Vec<Entity>,
    current_index: usize,
    step_timer: f32,
    damage: f32,
}

pub(crate) fn start_shadowstep_dance(
    commands: &mut Commands,
    player_transform: &Transform,
    player_entity: Entity,
    weapon: &Weapon,
    enemies: &Query<(Entity, &GlobalTransform), With<Enemy>>,
) {
    let mut candidates: Vec<(Entity, f32)> = enemies
        .iter()
        .filter_map(|(entity, transform)| {
            let distance = transform
                .translation()
                .distance(player_transform.translation);
            (distance <= DANCER_RANGE).then_some((entity, distance))
        })
        .collect();

    candidates.sort_by(|a, b| a.1.total_cmp(&b.1));

    let targets: Vec<Entity> = candidates
        .into_iter()
        .take(DANCER_MAX_TARGETS)
        .map(|(entity, _)| entity)
        .collect();

    if targets.is_empty() {
        println!("Shadowstep Dance found no targets.");
        return;
    }

    commands.entity(player_entity).insert(ShadowstepDance {
        origin: player_transform.translation,
        targets,
        current_index: 0,
        step_timer: 0.0,
        damage: weapon.damage * DANCER_STRIKE_DAMAGE_MULTIPLIER,
    });

    println!("Shadowstep Dance!");
}

/// Ticks an in-progress Shadowstep Dance. Registered by
/// `weapons::WeaponPlugin`.
pub(crate) fn update_shadowstep_dance(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut player_query: Query<
        (
            Entity,
            &mut Transform,
            &mut PlayerCombat,
            &mut ShadowstepDance,
        ),
        (With<PlayerCharacter>, Without<Enemy>),
    >,
    mut enemy_query: Query<
        (&mut Transform, &mut Health, Option<&EnemyDebuffs>),
        (With<Enemy>, Without<Player>),
    >,
) {
    for (player_entity, mut player_transform, mut combat, mut dance) in &mut player_query {
        dance.step_timer -= time.delta_secs();

        if dance.step_timer > 0.0 {
            continue;
        }

        dance.step_timer = DANCER_STEP_INTERVAL;

        if dance.current_index >= dance.targets.len() {
            player_transform.translation = dance.origin;
            commands.entity(player_entity).remove::<ShadowstepDance>();
            println!("Shadowstep Dance returns!");
            continue;
        }

        let target_entity = dance.targets[dance.current_index];

        dance.current_index += 1;

        let Ok((mut enemy_transform, mut enemy_health, enemy_debuffs)) =
            enemy_query.get_mut(target_entity)
        else {
            // This target died or despawned mid-sequence -- skip it and
            // continue to the next step next tick.
            continue;
        };

        let mut to_enemy = enemy_transform.translation - player_transform.translation;

        to_enemy.y = 0.0;

        let approach_direction = if to_enemy.normalize_or_zero() == Vec3::ZERO {
            Vec3::X
        } else {
            to_enemy.normalize_or_zero()
        };

        // Flank to a point just beside the target rather than teleporting
        // exactly on top of it.
        player_transform.translation =
            enemy_transform.translation - approach_direction * DANCER_FLANK_DISTANCE;
        player_transform.rotation =
            Quat::from_rotation_y(approach_direction.x.atan2(approach_direction.z));

        let is_behind = is_attack_from_behind(
            enemy_transform.rotation,
            enemy_transform.translation,
            player_transform.translation,
        );

        combat::apply_enemy_damage(&mut enemy_health, enemy_debuffs, dance.damage);

        spawn_shadow_copy_visual(
            &mut commands,
            &mut meshes,
            &mut materials,
            enemy_transform.translation,
            target_entity,
            DANCER_ECHO_DELAY,
            dance.damage * DANCER_ECHO_DAMAGE_FRACTION,
        );

        if is_behind {
            combat.ability_cooldown =
                (combat.ability_cooldown - DANCER_BACKSTAB_COOLDOWN_REFUND).max(0.0);
            println!("Shadowstep backstab -- cooldown reduced!");
        }

        println!(
            "Shadowstep Dance strikes! HP {:.0}/{:.0}",
            enemy_health.current, enemy_health.max
        );

        let _ = &mut enemy_transform;
    }
}

/// Perfect Evasion: dodging within `DANCER_PERFECT_EVASION_RADIUS` of an
/// enemy grants one Shadow Charge, capped at `DANCER_MAX_SHADOW_CHARGES`
/// and consumed by `daggers_basic_attack`. Only active while Dancer in
/// the Dark is equipped. One charge per dodge no matter how many
/// enemies it passes near or how long it lasts, tracked by
/// `WeaponAbilityState::dancer_shadow_charge_armed` and reset the moment
/// the player is no longer dodging. Runs for every player character, each
/// with their own charges. Registered by `weapons::WeaponPlugin`.
pub(crate) fn update_perfect_evasion(
    mut players: Query<
        (&Transform, &Dodge, &Weapon, &mut WeaponAbilityState),
        With<PlayerCharacter>,
    >,
    enemies: Query<&Transform, (With<Enemy>, Without<PlayerCharacter>)>,
) {
    for (player_transform, dodge, weapon, mut state) in &mut players {
        if weapon.metadata.definition_id != DANCER_IN_THE_DARK_ID || !dodge.is_dodging() {
            state.dancer_shadow_charge_armed = false;
            continue;
        }

        if state.dancer_shadow_charge_armed {
            continue;
        }

        let near_enemy = enemies.iter().any(|enemy_transform| {
            enemy_transform
                .translation
                .distance(player_transform.translation)
                <= DANCER_PERFECT_EVASION_RADIUS
        });

        if !near_enemy {
            continue;
        }

        state.dancer_shadow_charge_armed = true;
        state.dancer_shadow_charges =
            (state.dancer_shadow_charges + 1).min(DANCER_MAX_SHADOW_CHARGES);

        println!(
            "Perfect Evasion! Shadow Charges: {}",
            state.dancer_shadow_charges
        );
    }
}
