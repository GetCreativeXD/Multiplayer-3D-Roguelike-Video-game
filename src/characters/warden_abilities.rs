//! Warden of Shades' three class abilities: Guardian Otter, Shadow Stag,
//! and Predator's Spirit (see the project context doc, section 19).
//!
//! Structurally this mirrors `characters::vanguard_abilities` closely --
//! one `*Ability`/`*Charge` component per ability, an input system per key
//! press, a companion system for anything that plays out over time, and a
//! `sync_warden_class_state` system that writes the current state into the
//! shared `ClassAbilityHud` the same way Vanguard's `sync_vanguard_class_state`
//! does. Nothing here is Vanguard-specific and nothing in
//! `characters::vanguard_abilities` was changed to build this.
//!
//! Ability-power scaling: every ability below multiplies its base numbers
//! by the caster's `Equipment::ability_power_multiplier()` (see
//! `weapons::components`). That is Ruinbound's "normal ability scaling" --
//! the same shared formula every gear-granted Ability Power point feeds
//! into, so any future gear roll that raises Ability Power automatically
//! makes Guardian Otter's shield/buffs, Shadow Stag's charge damage, and
//! Predator's Spirit's spectral strikes all stronger together, exactly as
//! requested. It's independent from `combat::StatBuffs`, which is a
//! *temporary* buff a target receives (from Guardian Otter, today) rather
//! than a persistent stat the caster's own gear provides.

use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::visual_effects::{EffectShape, effect_mesh};

use crate::characters::class_abilities::{
    AbilityUseTracker, BasicAttackHitTracker, ClassAbilityDisplay, ClassAbilityHud,
    ClassAbilityHudSlot,
};
use crate::characters::warden::Warden;
use crate::combat::{BarrierSource, Barriers, BuffSource, Health, StatBuffs};
use crate::enemies::Enemy;
use crate::player::{AbilityMovement, PlayerCharacter, PlayerIntent};
use crate::weapons::{
    Equipment, PlayerProjectile, Projectile, ProjectileKind, SHADOW_STAG_SIZE, Weapon, WeaponType,
};

// =========================================================
// INPUT
//
// Every Warden player character casts from its own `PlayerIntent`: the
// local `Player` from this machine's Q/E/R, a joined player's body (on the
// multiplayer host) from that client's presses. Each ability aims with that
// player's own camera (`PlayerIntent::aim_transform`).
// =========================================================

// Class-ability slots (`player::CLASS_ABILITY_KEYS`): Q, E, R.
const GUARDIAN_OTTER_SLOT: usize = 0;
const SHADOW_STAG_SLOT: usize = 1;
const PREDATORS_SPIRIT_SLOT: usize = 2;

// =========================================================
// GUARDIAN OTTER (experimental tuning -- expect these to move during
// playtesting, same as every other numeric value in the project)
// =========================================================

const GUARDIAN_OTTER_COOLDOWN: f32 = 14.0;
const GUARDIAN_OTTER_DURATION: f32 = 8.0;
/// Maximum crosshair-raycast distance for picking up a targeted teammate
/// (any player character: in co-op, the other players).
const GUARDIAN_OTTER_TARGET_RANGE: f32 = 25.0;
const GUARDIAN_OTTER_BASE_SHIELD: f32 = 25.0;
/// Fraction bonus at 0 Ability Power (i.e. no gear granting any yet).
const GUARDIAN_OTTER_BASE_DAMAGE_BONUS: f32 = 0.15;
const GUARDIAN_OTTER_BASE_MOVEMENT_BONUS: f32 = 0.20;
const GUARDIAN_OTTER_ORBIT_RADIUS: f32 = 1.05;
const GUARDIAN_OTTER_ORBIT_SPEED: f32 = 2.6;

/// Per-Warden Guardian Otter state: cooldown, and how long the blessing
/// this cast granted is still active (used only for the HUD -- the actual
/// buff/shield live on whichever entity was blessed, via `StatBuffs` and
/// `Barriers`, and keep ticking down independently).
#[derive(Component, Default)]
struct GuardianOtterAbility {
    cooldown: f32,
    active_remaining: f32,
}

/// The otter's spectral visual, orbiting whichever player it blessed for
/// the blessing's duration. Purely cosmetic -- the gameplay effect is
/// applied once, at cast time, onto the target's `Barriers`/`StatBuffs`.
#[derive(Component)]
struct GuardianOtterVisual {
    target: Entity,
    remaining: f32,
    angle: f32,
}

/// If the caster's crosshair is currently over another player character, the
/// blessing goes to them; otherwise it falls back to the caster --
/// exactly the approved direction ("can target the Warden when playing
/// solo"). Uses the same camera-raycast approach as Vanguard's
/// `focused_enemy` and Predator's Spirit's target search below, just
/// aimed at players instead of enemies.
fn guardian_otter_target(
    caster: Entity,
    caster_position: Vec3,
    camera_transform: &Transform,
    spatial_query: &SpatialQuery,
    players: &Query<(Entity, &Transform), With<PlayerCharacter>>,
) -> (Entity, Vec3) {
    let direction = camera_transform.forward().as_vec3().normalize_or_zero();

    if direction != Vec3::ZERO {
        if let Ok(ray_direction) = Dir3::new(direction.adjust_precision()) {
            let filter = SpatialQueryFilter::from_excluded_entities([caster]);

            if let Some(hit) = spatial_query.cast_ray(
                camera_transform.translation.adjust_precision(),
                ray_direction,
                GUARDIAN_OTTER_TARGET_RANGE,
                true,
                &filter,
            ) {
                if let Ok((entity, transform)) = players.get(hit.entity) {
                    return (entity, transform.translation);
                }
            }
        }
    }

    (caster, caster_position)
}

fn guardian_otter_input(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spatial_query: SpatialQuery,
    all_players: Query<(Entity, &Transform), With<PlayerCharacter>>,
    mut caster_query: Query<
        (
            Entity,
            &Transform,
            &mut PlayerIntent,
            &Equipment,
            &mut GuardianOtterAbility,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Warden>),
    >,
    // The blessed player: the caster, or (in co-op) another player.
    mut target_query: Query<(&mut Barriers, &mut StatBuffs), With<PlayerCharacter>>,
) {
    for (
        caster_entity,
        caster_transform,
        mut intent,
        equipment,
        mut ability,
        mut ability_tracker,
    ) in &mut caster_query
    {
        // A press (Q, for the local player) is consumed even when it can't
        // cast, as `just_pressed` was before.
        if !intent.take_class_ability(GUARDIAN_OTTER_SLOT) {
            continue;
        }

        if ability.cooldown > 0.0 {
            continue;
        }

        let camera_transform = intent.aim_transform();
        let (target_entity, target_position) = guardian_otter_target(
            caster_entity,
            caster_transform.translation,
            &camera_transform,
            &spatial_query,
            &all_players,
        );

        let ability_power = equipment.ability_power_multiplier();
        let shield = GUARDIAN_OTTER_BASE_SHIELD * ability_power;
        let damage_multiplier = 1.0 + GUARDIAN_OTTER_BASE_DAMAGE_BONUS * ability_power;
        let movement_multiplier = 1.0 + GUARDIAN_OTTER_BASE_MOVEMENT_BONUS * ability_power;

        if let Ok((mut barriers, mut stat_buffs)) = target_query.get_mut(target_entity) {
            barriers.grant_default(BarrierSource::GuardianOtter, shield);
            stat_buffs.grant(
                BuffSource::GuardianOtter,
                movement_multiplier,
                damage_multiplier,
                GUARDIAN_OTTER_DURATION,
            );
        }

        ability.cooldown = GUARDIAN_OTTER_COOLDOWN;
        ability.active_remaining = GUARDIAN_OTTER_DURATION;

        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }

        commands.spawn((
            Name::new("Guardian Otter"),
            GuardianOtterVisual {
                target: target_entity,
                remaining: GUARDIAN_OTTER_DURATION,
                angle: 0.0,
            },
            effect_mesh(
                &mut *meshes,
                EffectShape::Capsule {
                    radius: 0.26,
                    length: 0.5,
                },
            ),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.25, 0.85, 0.65, 0.85),
                emissive: LinearRgba::new(0.1, 0.55, 0.4, 1.0),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(target_position + Vec3::Y * 1.2),
        ));

        println!(
            "Guardian Otter blesses {:?}! Shield {:.1}",
            target_entity, shield
        );
    }
}

fn update_guardian_otter_visuals(
    mut commands: Commands,
    time: Res<Time>,
    targets: Query<&Transform, (With<PlayerCharacter>, Without<GuardianOtterVisual>)>,
    mut visuals: Query<(Entity, &mut Transform, &mut GuardianOtterVisual)>,
) {
    let delta = time.delta_secs();

    for (entity, mut transform, mut visual) in &mut visuals {
        visual.remaining -= delta;

        if visual.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        let Ok(target_transform) = targets.get(visual.target) else {
            // The blessed player no longer exists (despawned/disconnected).
            commands.entity(entity).despawn();
            continue;
        };

        visual.angle += GUARDIAN_OTTER_ORBIT_SPEED * delta;

        let orbit =
            Vec3::new(visual.angle.cos(), 0.0, visual.angle.sin()) * GUARDIAN_OTTER_ORBIT_RADIUS;

        transform.translation = target_transform.translation + Vec3::Y * 1.2 + orbit;
    }
}

// =========================================================
// SHADOW STAG
// =========================================================

const SHADOW_STAG_COOLDOWN: f32 = 9.0;
const SHADOW_STAG_SPEED: f32 = 22.0;
/// How far the spectral stag travels before fading. Its lifetime is
/// derived from this and its speed, so range is the one number to tune.
const SHADOW_STAG_RANGE: f32 = 13.0;
const SHADOW_STAG_LIFETIME: f32 = SHADOW_STAG_RANGE / SHADOW_STAG_SPEED;
const SHADOW_STAG_BASE_DAMAGE: f32 = 26.0;
const SHADOW_STAG_KNOCKBACK: f32 = 2.75;
/// Comfortably above elite stagger resistance (~2, per the project context
/// doc's stagger table) so Shadow Stag reliably interrupts -- "stuns" --
/// grouped normal/elite enemies without being strong enough to lock down a
/// boss (resistance ~4) on its own.
const SHADOW_STAG_STAGGER_STRENGTH: u32 = 3;
const SHADOW_STAG_STAGGER_DURATION: f32 = 0.9;
/// How far in front of the Warden the stag is spawned, so it doesn't
/// visually clip through the caster's own model the instant it appears.
const SHADOW_STAG_SPAWN_OFFSET: f32 = 1.4;

#[derive(Component, Default)]
struct ShadowStagAbility {
    cooldown: f32,
}

fn shadow_stag_input(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut caster_query: Query<
        (
            &mut Transform,
            &mut PlayerIntent,
            &Equipment,
            &mut ShadowStagAbility,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Warden>),
    >,
) {
    for (mut transform, mut intent, equipment, mut ability, mut ability_tracker) in
        &mut caster_query
    {
        // E, for the local player.
        if !intent.take_class_ability(SHADOW_STAG_SLOT) {
            continue;
        }

        if ability.cooldown > 0.0 {
            continue;
        }

        let camera_transform = intent.aim_transform();
        let mut direction = camera_transform.forward().as_vec3();
        direction.y = 0.0;
        direction = direction.normalize_or_zero();

        if direction == Vec3::ZERO {
            continue;
        }

        // The Warden only turns to face where the stag is sent -- unlike the
        // original design, nothing here ever touches the caster's own
        // `AbilityMovement`, so the player never dashes.
        transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));

        let spawn_position =
            transform.translation + direction * SHADOW_STAG_SPAWN_OFFSET + Vec3::Y * 0.9;

        let damage = SHADOW_STAG_BASE_DAMAGE * equipment.ability_power_multiplier();

        // The stag itself is a wide, piercing `Projectile` that travels on
        // its own -- the same shared movement/collision system every arrow
        // and Epic Greatsword's Phantom Cleave already use (see
        // `weapons::projectiles::move_projectiles_with_collision`). Piercing
        // means every enemy along its path is hit exactly once; it despawns
        // on its own once `SHADOW_STAG_LIFETIME` runs out.
        commands.spawn((
            PlayerProjectile,
            Projectile {
                velocity: direction * SHADOW_STAG_SPEED,
                damage,
                knockback: SHADOW_STAG_KNOCKBACK,
                stagger_strength: SHADOW_STAG_STAGGER_STRENGTH,
                stagger_duration: SHADOW_STAG_STAGGER_DURATION,
                lifetime: SHADOW_STAG_LIFETIME,
                piercing: true,
                hit_entities: Vec::new(),
                kind: ProjectileKind::ShadowStag,
                target: None,
            },
            // Spectral stag silhouette -- sized identically to the collider
            // `move_projectiles_with_collision` gives `ProjectileKind::ShadowStag`
            // (both read `SHADOW_STAG_SIZE`), so the visible shape and the
            // actual hitbox never disagree.
            Mesh3d(meshes.add(Cuboid::new(
                SHADOW_STAG_SIZE.x,
                SHADOW_STAG_SIZE.y,
                SHADOW_STAG_SIZE.z,
            ))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.35, 0.8, 0.55, 0.55),
                emissive: LinearRgba::new(0.15, 0.6, 0.35, 1.0),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform {
                translation: spawn_position,
                rotation: Quat::from_rotation_arc(Vec3::Z, direction),
                ..default()
            },
        ));

        ability.cooldown = SHADOW_STAG_COOLDOWN;

        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }

        // Emergence burst, roughly where the stag appears from the Warden.
        commands.spawn((
            WardenVisualEffect { lifetime: 0.3 },
            effect_mesh(
                &mut *meshes,
                EffectShape::Torus {
                    inner_radius: 0.6,
                    outer_radius: 0.9,
                },
            ),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.35, 0.8, 0.55, 0.7),
                emissive: LinearRgba::new(0.15, 0.6, 0.35, 1.0),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform {
                translation: transform.translation + Vec3::Y * 0.9 + direction * 0.6,
                rotation: Quat::from_rotation_arc(Vec3::Z, direction),
                ..default()
            },
        ));

        println!("Shadow Stag sent forward!");
    }
}

// =========================================================
// PREDATOR'S SPIRIT
// =========================================================

const PREDATORS_SPIRIT_COOLDOWN: f32 = 16.0;
const PREDATORS_SPIRIT_DURATION: f32 = 10.0;
/// Spectral-strike damage as a fraction of the equipped weapon's basic
/// attack damage, before Ability Power scaling. Longsword and Bow use
/// this directly; Greatsword multiplies it by
/// `PREDATORS_SPIRIT_GREATSWORD_FRACTION_MULTIPLIER` for a larger, heavier
/// proc that matches its slower swing -- "fast weapons trigger frequent
/// spectral strikes; heavy weapons create larger/slower empowered
/// impacts" is otherwise a natural consequence of how often each weapon's
/// basic attack lands, not something that needs its own timer.
const PREDATORS_SPIRIT_DAMAGE_FRACTION: f32 = 0.5;
const PREDATORS_SPIRIT_GREATSWORD_FRACTION_MULTIPLIER: f32 = 1.4;
/// Hammer is heavier than Greatsword (WEAPONS.md: highest ordinary
/// stagger/knockback in the roster), so its empowered impact scales up
/// even further than Greatsword's.
const PREDATORS_SPIRIT_HAMMER_FRACTION_MULTIPLIER: f32 = 1.6;
/// How far the spectral-strike raycast looks for "the enemy the basic
/// attack just hit." Same simplification Vanguard's Afterimage already
/// uses for its ranged clone's target (see
/// `characters::vanguard_abilities::focused_enemy`): `BasicAttackHitTracker`
/// only records that a hit landed, not which enemy, so both abilities
/// approximate it with whatever the crosshair is currently aimed at.
const PREDATORS_SPIRIT_RANGE: f32 = 30.0;

#[derive(Component, Default)]
struct PredatorsSpirit {
    cooldown: f32,
    active_remaining: f32,
    observed_basic_attack_hits: u64,
}

fn predators_spirit_input(
    mut wardens: Query<
        (
            &mut PlayerIntent,
            &BasicAttackHitTracker,
            &mut PredatorsSpirit,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Warden>),
    >,
) {
    for (mut intent, basic_attack_hits, mut spirit, mut ability_tracker) in &mut wardens {
        // R, for the local player.
        if !intent.take_class_ability(PREDATORS_SPIRIT_SLOT) {
            continue;
        }

        if spirit.cooldown > 0.0 {
            continue;
        }

        spirit.active_remaining = PREDATORS_SPIRIT_DURATION;
        spirit.cooldown = PREDATORS_SPIRIT_COOLDOWN;
        // Don't retroactively proc on hits landed before activation.
        spirit.observed_basic_attack_hits = basic_attack_hits.total_confirmed();

        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }

        println!("Predator's Spirit awakens!");
    }
}

/// Watches for newly confirmed basic-attack hits while Predator's Spirit
/// is active and, for each one, lands a bonus spectral strike -- the same
/// "observe the shared hit counter, then react" pattern
/// `characters::class_abilities::AbilityUseTracker`'s doc comment
/// describes and Vanguard's Momentum already uses against
/// `BasicAttackHitTracker`.
fn predators_spirit_proc(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spatial_query: SpatialQuery,
    mut wardens: Query<
        (
            Entity,
            &PlayerIntent,
            &Equipment,
            &Weapon,
            &BasicAttackHitTracker,
            &mut PredatorsSpirit,
        ),
        (With<PlayerCharacter>, With<Warden>),
    >,
    mut enemies: Query<(&Transform, &mut Health), (With<Enemy>, Without<PlayerCharacter>)>,
) {
    for (warden_entity, intent, equipment, weapon, basic_attack_hits, mut spirit) in &mut wardens {
        // The strike lands on whatever this Warden's crosshair is over.
        let camera_transform = intent.aim_transform();

        if spirit.active_remaining <= 0.0 {
            // Not active -- keep the observed count current so re-activating
            // later doesn't proc on hits landed while it was off.
            spirit.observed_basic_attack_hits = basic_attack_hits.total_confirmed();
            continue;
        }

        let new_hits = basic_attack_hits
            .total_confirmed()
            .saturating_sub(spirit.observed_basic_attack_hits);
        spirit.observed_basic_attack_hits = basic_attack_hits.total_confirmed();

        if new_hits == 0 {
            continue;
        }

        let origin = camera_transform.translation;
        let direction = camera_transform.forward().as_vec3().normalize_or_zero();
        let Ok(ray_direction) = Dir3::new(direction.adjust_precision()) else {
            continue;
        };
        let filter = SpatialQueryFilter::from_excluded_entities([warden_entity]);

        let Some(hit) = spatial_query.cast_ray(
            origin.adjust_precision(),
            ray_direction,
            PREDATORS_SPIRIT_RANGE,
            true,
            &filter,
        ) else {
            continue;
        };

        let Ok((enemy_transform, mut enemy_health)) = enemies.get_mut(hit.entity) else {
            continue;
        };

        let (fraction, radius, color) = match weapon.weapon_type {
            WeaponType::Longsword => (
                PREDATORS_SPIRIT_DAMAGE_FRACTION,
                0.30,
                Color::srgb(0.65, 0.95, 0.55),
            ),
            WeaponType::Bow => (
                PREDATORS_SPIRIT_DAMAGE_FRACTION,
                0.24,
                Color::srgb(0.55, 0.95, 0.75),
            ),
            WeaponType::Greatsword => (
                PREDATORS_SPIRIT_DAMAGE_FRACTION * PREDATORS_SPIRIT_GREATSWORD_FRACTION_MULTIPLIER,
                0.46,
                Color::srgb(0.45, 0.9, 0.5),
            ),
            WeaponType::Hammer => (
                PREDATORS_SPIRIT_DAMAGE_FRACTION * PREDATORS_SPIRIT_HAMMER_FRACTION_MULTIPLIER,
                0.52,
                Color::srgb(0.85, 0.75, 0.35),
            ),
            // Crossbow and Spellbook are ranged like Bow, so they take Bow's
            // un-multiplied fraction rather than one of the heavy-melee
            // bonuses above -- their higher/lower `weapon.damage` already
            // flows through the `weapon.damage * fraction` multiplication
            // below without needing a separate multiplier here.
            WeaponType::Crossbow => (
                PREDATORS_SPIRIT_DAMAGE_FRACTION,
                0.26,
                Color::srgb(0.6, 0.85, 0.65),
            ),
            WeaponType::Spellbook => (
                PREDATORS_SPIRIT_DAMAGE_FRACTION,
                0.24,
                Color::srgb(0.65, 0.9, 0.85),
            ),
            // Daggers is light melee like Longsword, not a heavy two-hander,
            // so it takes the same un-multiplied fraction -- just the
            // smallest strike visual of the set, matching its short-range,
            // fast-hitting identity (see weapons::daggers), tinted violet to
            // echo the family's Widow's Kiss / Dancer in the Dark visuals.
            WeaponType::Daggers => (
                PREDATORS_SPIRIT_DAMAGE_FRACTION,
                0.20,
                Color::srgb(0.72, 0.55, 0.9),
            ),
        };

        let damage = weapon.damage * fraction * equipment.ability_power_multiplier();
        let strike_position = enemy_transform.translation + Vec3::Y * 0.6;

        enemy_health.current = (enemy_health.current - damage).max(0.0);

        println!(
            "Predator's Spirit strike! Damage {:.1}, HP {:.0}/{:.0}",
            damage, enemy_health.current, enemy_health.max
        );

        commands.spawn((
            WardenVisualEffect { lifetime: 0.25 },
            effect_mesh(&mut *meshes, EffectShape::Sphere { radius }),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                emissive: LinearRgba::new(0.2, 0.9, 0.5, 1.0),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(strike_position),
        ));
    }
}

// =========================================================
// SHARED VISUAL-EFFECT CLEANUP
// =========================================================

/// Generic short-lived cosmetic mesh spawned by Shadow Stag's charge burst
/// and Predator's Spirit's strike burst. `GuardianOtterVisual` manages its
/// own lifetime/despawn separately since it also needs to track and orbit
/// a target every frame.
#[derive(Component)]
struct WardenVisualEffect {
    lifetime: f32,
}

fn update_warden_visual_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut effects: Query<(Entity, &mut WardenVisualEffect)>,
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
// SHARED WARDEN STATE / HUD
// =========================================================

fn initialize_warden_abilities(
    mut commands: Commands,
    // Every player character (a joined Warden too, via `net::loadout`).
    players: Query<Entity, (With<PlayerCharacter>, Added<Warden>)>,
) {
    for player in &players {
        commands.entity(player).insert((
            GuardianOtterAbility::default(),
            ShadowStagAbility::default(),
            PredatorsSpirit::default(),
            ClassAbilityHud::new([
                ClassAbilityHudSlot::new("Q", "GUARDIAN OTTER", ClassAbilityDisplay::Ready),
                ClassAbilityHudSlot::new("E", "SHADOW STAG", ClassAbilityDisplay::Ready),
                ClassAbilityHudSlot::new("R", "PREDATOR'S SPIRIT", ClassAbilityDisplay::Ready),
            ]),
            AbilityMovement {
                direction: Vec3::ZERO,
                speed: 0.0,
                time_remaining: 0.0,
            },
        ));
    }
}

fn update_warden_timers(
    time: Res<Time>,
    mut wardens: Query<
        (
            &mut GuardianOtterAbility,
            &mut ShadowStagAbility,
            &mut PredatorsSpirit,
        ),
        With<Warden>,
    >,
) {
    let delta = time.delta_secs();

    for (mut otter, mut stag, mut spirit) in &mut wardens {
        otter.cooldown = (otter.cooldown - delta).max(0.0);
        otter.active_remaining = (otter.active_remaining - delta).max(0.0);
        stag.cooldown = (stag.cooldown - delta).max(0.0);
        spirit.cooldown = (spirit.cooldown - delta).max(0.0);
        spirit.active_remaining = (spirit.active_remaining - delta).max(0.0);
    }
}

fn sync_warden_class_state(
    mut wardens: Query<
        (
            &GuardianOtterAbility,
            &ShadowStagAbility,
            &PredatorsSpirit,
            &mut ClassAbilityHud,
        ),
        (With<PlayerCharacter>, With<Warden>),
    >,
) {
    for (otter, stag, spirit, mut hud) in &mut wardens {
        hud.slots[0].display = if otter.active_remaining > 0.0 {
            ClassAbilityDisplay::Active("BLESSED")
        } else if otter.cooldown <= 0.0 {
            ClassAbilityDisplay::Ready
        } else {
            ClassAbilityDisplay::Cooldown(otter.cooldown)
        };

        hud.slots[1].display = if stag.cooldown <= 0.0 {
            ClassAbilityDisplay::Ready
        } else {
            ClassAbilityDisplay::Cooldown(stag.cooldown)
        };

        hud.slots[2].display = if spirit.active_remaining > 0.0 {
            ClassAbilityDisplay::Active("ACTIVE")
        } else if spirit.cooldown <= 0.0 {
            ClassAbilityDisplay::Ready
        } else {
            ClassAbilityDisplay::Cooldown(spirit.cooldown)
        };
    }
}

// =========================================================
// PLUGIN
// =========================================================

pub struct WardenAbilitiesPlugin;

impl Plugin for WardenAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                initialize_warden_abilities,
                update_warden_timers,
                guardian_otter_input,
                update_guardian_otter_visuals,
                shadow_stag_input,
                predators_spirit_input,
                predators_spirit_proc,
                update_warden_visual_effects,
                sync_warden_class_state,
            )
                .chain(),
        );
    }
}
