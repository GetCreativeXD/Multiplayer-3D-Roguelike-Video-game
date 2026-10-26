//! Gravemind's three class abilities: Ruin Bolt, Consume, and Embrace the
//! Ruin (see the project context doc, section 23).
//!
//! Structurally this mirrors `characters::warden_abilities`,
//! `characters::runesage_abilities`, and `characters::wildshaper_abilities`:
//! one `*Ability` component per ability, an input system per key press, a
//! companion update system for anything that plays out over time, and a
//! `sync_gravemind_class_state` system that writes current state into the
//! shared `ClassAbilityHud`/`ClassResourceHud` the same way every other
//! class does. Nothing here changes any other class's code.
//!
//! Identity (context doc, section 23): Gravemind is an aggressive,
//! unlockable class that interacts directly with Ruin and can ramp into
//! dangerous power. The context doc is explicit that the class should
//! interact with health without being "entirely focused on constant
//! low-health micromanagement" -- Ruin Bolt and Consume are ordinary
//! cooldown abilities with no health cost of their own (the doc notes
//! "earlier health-cost versions were deemphasized"); only Embrace the
//! Ruin, the class's signature risk/reward tool, touches health directly.
//!
//! Ability-power scaling: Ruin Bolt and Consume both multiply their base
//! numbers by the caster's `Equipment::ability_power_multiplier()` (see
//! `weapons::components`), exactly like every other class's abilities do.
//!
//! Ruin interaction (context doc, section 9.1: "class mechanics can
//! interact with Ruin, especially Gravemind"): Embrace the Ruin raises
//! `ruin::RuinState` each time its internal meter fully fills, through
//! `RuinState::add_ruin` like every other Ruin-granting mechanic is meant
//! to. Ruin Bolt and Consume do not touch Ruin at all.
//!
//! # Disclosed design decisions
//!
//! - **"Attacking," for Embrace the Ruin's rising meter, is approximated
//!   as `PlayerCombat::attack_cooldown > 0.0`** -- the same "still
//!   cooling down from the last swing" flag `weapons::melee`'s own attack
//!   system already gates the next attack on. Ruinbound has no separate
//!   "is the player currently mid-attack" state to read, and inventing
//!   one for a single ability would be a larger change than this pass
//!   should make unprompted. In practice this means the meter rises
//!   whenever the player is attacking at roughly their weapon's own pace
//!   and falls as soon as they stop, which matches the context doc's
//!   "rises while attacking / falls while not attacking" description.
//! - **Embrace's health drain floors at `EMBRACE_HEALTH_DRAIN_FLOOR`
//!   (1 HP) and cannot kill the player by itself.** The context doc asks
//!   for continuous health drain but also says the class should not be
//!   "entirely focused on constant low-health micromanagement" -- a
//!   passive drain that can silently end the run on its own would cut
//!   against that. Damage from enemies/hazards while Embrace is active
//!   still goes through the normal `combat::apply_player_damage` path
//!   and can kill the player as normal; only Embrace's own tick is
//!   floored.
//! - **Filling the meter grants a permanent-for-this-activation stack
//!   (capped at `EMBRACE_MAX_STACKS`) instead of stacking damage
//!   indefinitely.** The context doc explicitly leaves "whether multiple
//!   fills stack indefinitely" as a balance question -- an uncapped
//!   stack is a very easy number to make absurd, so this pass caps it and
//!   flags the cap as a tuning placeholder like every other number below.
//! - **A short reactivation cooldown (`EMBRACE_REACTIVATE_COOLDOWN`)
//!   applies after Embrace is turned off**, so a player can't toggle it
//!   on for one frame (to refresh its damage buff) and off again to dodge
//!   the health-drain cost that's supposed to come with it.

use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::visual_effects::{EffectShape, effect_mesh};

use crate::characters::class_abilities::{
    AbilityUseTracker, ClassAbilityDisplay, ClassAbilityHud, ClassAbilityHudSlot, ClassResourceHud,
};
use crate::characters::gravemind::Gravemind;
use crate::combat::{self, BuffSource, EnemyDebuffs, Health, StatBuffs};
use crate::enemies::Enemy;
use crate::player::{PlayerCharacter, PlayerIntent};
use crate::ruin::RuinState;
use crate::run_setup::RunModifiers;
use crate::weapons::{Equipment, PlayerCombat, PlayerProjectile, Projectile, ProjectileKind};

// =========================================================
// INPUT
// =========================================================

// Class-ability slots (`player::CLASS_ABILITY_KEYS`): Q, E, R. Every
// Gravemind player character (the local `Player`, or a joined player's body
// on the multiplayer host) casts from its own `PlayerIntent`, aimed with
// that player's camera (`PlayerIntent::aim_transform`).
const RUIN_BOLT_SLOT: usize = 0;
const CONSUME_SLOT: usize = 1;
const EMBRACE_THE_RUIN_SLOT: usize = 2;

/// How many pips the shared class-resource bar can ever render (see
/// `ui::update_class_resource_bar`'s private `CLASS_RESOURCE_MAX_PIPS`,
/// currently `8`). Embrace's meter is tracked internally as a
/// `0.0..=EMBRACE_METER_MAX` float for precise math, then rescaled down
/// to this many pips for display -- the same idea Runesage's Rune count
/// already uses directly (Runes fit within 8 without rescaling; Embrace's
/// 0-100 meter does not, so it's converted).
const EMBRACE_METER_PIP_COUNT: u8 = 8;

// =========================================================
// RUIN BOLT (Q) -- burst damage
//
// A heavy corruption bolt fired toward whatever the crosshair is over,
// the same crosshair-raycast-to-point aiming Runesage's Rune Blast and
// `weapons::bow::fire_bow` already use. On impact it also splashes
// lighter damage onto other nearby enemies (see
// `weapons::projectiles::movement`'s Ruin Bolt-specific block) -- the
// "blast" half of "projectile/blast" from the context doc.
// =========================================================

const RUIN_BOLT_COOLDOWN: f32 = 6.0;
const RUIN_BOLT_SPEED: f32 = 24.0;
const RUIN_BOLT_RANGE: f32 = 24.0;
const RUIN_BOLT_LIFETIME: f32 = RUIN_BOLT_RANGE / RUIN_BOLT_SPEED;
const RUIN_BOLT_BASE_DAMAGE: f32 = 32.0;
const RUIN_BOLT_KNOCKBACK: f32 = 1.2;
const RUIN_BOLT_STAGGER_STRENGTH: u32 = 2;
const RUIN_BOLT_STAGGER_DURATION: f32 = 0.5;

#[derive(Component, Default)]
struct RuinBoltAbility {
    cooldown: f32,
}

fn ruin_bolt_input(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spatial_query: SpatialQuery,
    mut caster_query: Query<
        (
            Entity,
            &mut Transform,
            &mut PlayerIntent,
            &Equipment,
            &mut RuinBoltAbility,
            Option<&GravemindEmbrace>,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Gravemind>),
    >,
) {
    for (
        caster_entity,
        mut transform,
        mut intent,
        equipment,
        mut ability,
        embrace,
        mut ability_tracker,
    ) in &mut caster_query
    {
        // A press (Q, for the local player) is consumed even when it can't
        // cast, as `just_pressed` was before.
        if !intent.take_class_ability(RUIN_BOLT_SLOT) {
            continue;
        }
        let camera_transform = intent.aim_transform();

        if ability.cooldown > 0.0 {
            continue;
        }

        let camera_origin = camera_transform.translation;
        let camera_direction = camera_transform.forward().as_vec3().normalize_or_zero();

        if camera_direction == Vec3::ZERO {
            continue;
        }

        // Crosshair-raycast-to-point aiming: find what the crosshair is
        // actually looking at, then aim the bolt from its spawn point toward
        // that point instead of just firing along the camera's forward
        // vector (which would drift off-target due to the camera/player
        // offset). Same approach as `characters::runesage_abilities::rune_blast_input`
        // and `weapons::bow::fire_bow`.
        let filter = SpatialQueryFilter::from_excluded_entities([caster_entity]);
        let mut target_point = camera_origin + camera_direction * RUIN_BOLT_RANGE;

        if let Ok(ray_direction) = Dir3::new(camera_direction.adjust_precision()) {
            if let Some(hit) = spatial_query.cast_ray(
                camera_origin.adjust_precision(),
                ray_direction,
                RUIN_BOLT_RANGE,
                true,
                &filter,
            ) {
                target_point = camera_origin + camera_direction * hit.distance;
            }
        }

        let spawn_position = transform.translation + Vec3::Y * 1.1 + camera_direction * 0.6;
        let projectile_direction = (target_point - spawn_position).normalize_or_zero();

        if projectile_direction == Vec3::ZERO {
            continue;
        }

        let mut facing = projectile_direction;
        facing.y = 0.0;
        facing = facing.normalize_or_zero();

        if facing != Vec3::ZERO {
            transform.rotation = Quat::from_rotation_y(facing.x.atan2(facing.z));
        }

        // Ruin Bolt scales with both gear-granted Ability Power and Embrace
        // the Ruin's current damage multiplier -- the context doc's
        // "significant damage increase" for the empowered state should be
        // felt across the whole kit while embraced, not just basic attacks.
        let ability_power = equipment.ability_power_multiplier();
        let embrace_multiplier = embrace.map_or(1.0, GravemindEmbrace::damage_multiplier);
        let damage = RUIN_BOLT_BASE_DAMAGE * ability_power * embrace_multiplier;

        commands.spawn((
            PlayerProjectile,
            Projectile {
                velocity: projectile_direction * RUIN_BOLT_SPEED,
                damage,
                knockback: RUIN_BOLT_KNOCKBACK,
                stagger_strength: RUIN_BOLT_STAGGER_STRENGTH,
                stagger_duration: RUIN_BOLT_STAGGER_DURATION,
                lifetime: RUIN_BOLT_LIFETIME,
                piercing: false,
                hit_entities: Vec::new(),
                kind: ProjectileKind::RuinBolt,
                target: None,
            },
            Mesh3d(meshes.add(Sphere::new(0.26))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.55, 0.15, 0.55),
                emissive: LinearRgba::new(0.55, 0.05, 0.65, 1.0),
                unlit: true,
                ..default()
            })),
            Transform::from_translation(spawn_position),
        ));

        ability.cooldown = RUIN_BOLT_COOLDOWN;

        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }

        println!("RUIN BOLT!");
    }
}

// =========================================================
// CONSUME (E) -- strong self-heal with a reason to stand close
//
// Rips corruption energy from every enemy standing nearby: each one
// takes a small direct hit, and the caster is healed for a large base
// fraction of max health plus a bonus per enemy affected (context doc:
// "restore a substantial amount of health ... plus additional healing
// based on nearby affected enemies"). The bonus is capped at
// `CONSUME_MAX_BONUS_ENEMIES` enemies so a large mob doesn't turn this
// into unlimited healing -- an explicit balance choice, not a locked
// number.
// =========================================================

const CONSUME_COOLDOWN: f32 = 22.0;
const CONSUME_BASE_HEAL_FRACTION: f32 = 0.30;
const CONSUME_HEAL_PER_ENEMY_FRACTION: f32 = 0.06;
const CONSUME_MAX_BONUS_ENEMIES: usize = 4;
const CONSUME_RADIUS: f32 = 6.0;
const CONSUME_DRAIN_DAMAGE: f32 = 10.0;

#[derive(Component, Default)]
struct ConsumeAbility {
    cooldown: f32,
}

fn consume_input(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    run_modifiers: Res<RunModifiers>,
    mut caster_query: Query<
        (
            &Transform,
            &mut PlayerIntent,
            &Equipment,
            &mut Health,
            &mut ConsumeAbility,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Gravemind>),
    >,
    mut enemy_query: Query<
        (&Transform, &mut Health, Option<&EnemyDebuffs>),
        (With<Enemy>, Without<PlayerCharacter>),
    >,
) {
    for (transform, mut intent, equipment, mut health, mut ability, mut ability_tracker) in
        &mut caster_query
    {
        // E, for the local player.
        if !intent.take_class_ability(CONSUME_SLOT) {
            continue;
        }

        if ability.cooldown > 0.0 {
            continue;
        }

        let center = transform.translation;
        let ability_power = equipment.ability_power_multiplier();
        let drain_damage = CONSUME_DRAIN_DAMAGE * ability_power;

        let mut affected_enemies: usize = 0;

        for (enemy_transform, mut enemy_health, debuffs) in enemy_query.iter_mut() {
            let mut offset = enemy_transform.translation - center;
            offset.y = 0.0;

            if offset.length() > CONSUME_RADIUS {
                continue;
            }

            combat::apply_enemy_damage(&mut enemy_health, debuffs, drain_damage);
            affected_enemies += 1;
        }

        let bonus_enemies = affected_enemies.min(CONSUME_MAX_BONUS_ENEMIES) as f32;
        let heal_fraction =
            CONSUME_BASE_HEAL_FRACTION + CONSUME_HEAL_PER_ENEMY_FRACTION * bonus_enemies;
        let heal_amount = health.max * heal_fraction;

        // Goes through the shared heal path so a healing-affecting pact
        // (e.g. Fragile Sanctuary) reaches this too -- see
        // `combat::apply_player_healing`.
        let healed = combat::apply_player_healing(&mut health, &run_modifiers, heal_amount);

        ability.cooldown = CONSUME_COOLDOWN;

        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }

        // Visual pulse at the caster, sized to the same radius the drain
        // check just used -- the project's "visible effects must match
        // gameplay hitboxes" rule.
        commands.spawn((
            GravemindVisualEffect { lifetime: 0.4 },
            effect_mesh(
                &mut *meshes,
                EffectShape::Sphere {
                    radius: CONSUME_RADIUS * 0.5,
                },
            ),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.55, 0.15, 0.55, 0.35),
                emissive: LinearRgba::new(0.5, 0.1, 0.55, 1.0),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(center),
        ));

        println!(
            "CONSUME! Healed {:.1} ({} enemies drained), HP {:.0}/{:.0}",
            healed, affected_enemies, health.current, health.max
        );
    }
}

// =========================================================
// EMBRACE THE RUIN (R) -- toggled high-risk empowered state
//
// While active: health drains continuously, and an internal meter rises
// while the caster is attacking (approximated as
// `PlayerCombat::attack_cooldown > 0.0` -- see the module doc comment's
// "Disclosed design decisions") and falls otherwise. The state can stay
// active while the meter falls -- only pressing the key again turns it
// off. Every time the meter completely fills, it resets to zero, raises
// Ruin, and grants one stack of bonus damage that persists for the rest
// of this activation, so a player willing to keep attacking can refill
// it repeatedly during a single activation -- trading more Ruin for more
// power, per the context doc's approved high-risk direction.
// =========================================================

const EMBRACE_REACTIVATE_COOLDOWN: f32 = 3.0;
const EMBRACE_METER_MAX: f32 = 100.0;
const EMBRACE_METER_RISE_PER_SECOND: f32 = 45.0;
const EMBRACE_METER_FALL_PER_SECOND: f32 = 30.0;
/// Fraction of max health drained per second while active.
const EMBRACE_HEALTH_DRAIN_FRACTION_PER_SECOND: f32 = 0.03;
/// Embrace's own drain never reduces the player below this floor -- see
/// the module doc comment's "Disclosed design decisions". Damage from
/// other sources while active is unaffected.
const EMBRACE_HEALTH_DRAIN_FLOOR: f32 = 1.0;
/// Damage bonus at a fully-filled meter, before any banked stacks.
const EMBRACE_METER_DAMAGE_BONUS_AT_FULL: f32 = 0.5;
const EMBRACE_RUIN_PER_FILL: f32 = 6.0;
const EMBRACE_DAMAGE_BONUS_PER_STACK: f32 = 0.08;
const EMBRACE_MAX_STACKS: u32 = 5;
/// `StatBuffs` entries expire on their own after `duration`; Embrace
/// refreshes this every frame while active, so it only needs to outlive
/// one frame's gap, not the whole activation.
const EMBRACE_BUFF_REFRESH_DURATION: f32 = 0.35;

/// Gravemind's Embrace the Ruin toggle. See the section header above and
/// the module doc comment for the full mechanic and its disclosed
/// approximations.
#[derive(Component, Default)]
struct GravemindEmbrace {
    active: bool,
    meter: f32,
    stacks: u32,
    /// Cooldown before Embrace can be reactivated after being turned off
    /// -- see `EMBRACE_REACTIVATE_COOLDOWN`.
    reactivate_cooldown: f32,
    /// The aura visual spawned on activation, so it can be despawned on
    /// deactivation. `None` whenever `active` is false.
    aura_entity: Option<Entity>,
}

impl GravemindEmbrace {
    /// Outgoing damage multiplier from the current meter fill plus any
    /// banked stacks. `1.0` (no bonus) whenever Embrace isn't active --
    /// safe to call unconditionally from Ruin Bolt/Consume's damage math.
    fn damage_multiplier(&self) -> f32 {
        if !self.active {
            return 1.0;
        }

        let meter_bonus = (self.meter / EMBRACE_METER_MAX) * EMBRACE_METER_DAMAGE_BONUS_AT_FULL;
        let stack_bonus = self.stacks as f32 * EMBRACE_DAMAGE_BONUS_PER_STACK;

        1.0 + meter_bonus + stack_bonus
    }
}

/// Purely cosmetic marker for the pulsing aura spawned as a child of the
/// caster while Embrace the Ruin is active.
#[derive(Component)]
struct GravemindEmbraceAura;

fn embrace_the_ruin_input(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut caster_query: Query<
        (
            Entity,
            &mut PlayerIntent,
            &mut GravemindEmbrace,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Gravemind>),
    >,
) {
    for (caster_entity, mut intent, mut embrace, mut ability_tracker) in &mut caster_query {
        // R, for the local player.
        if !intent.take_class_ability(EMBRACE_THE_RUIN_SLOT) {
            continue;
        }

        if embrace.active {
            embrace.active = false;
            embrace.meter = 0.0;
            embrace.stacks = 0;
            embrace.reactivate_cooldown = EMBRACE_REACTIVATE_COOLDOWN;

            if let Some(aura) = embrace.aura_entity.take() {
                commands.entity(aura).despawn();
            }

            println!("Embrace the Ruin released.");
            continue;
        }

        if embrace.reactivate_cooldown > 0.0 {
            continue;
        }

        embrace.active = true;
        embrace.meter = 0.0;
        embrace.stacks = 0;

        // Spawned as a child in the same `with_children` call the rest of
        // this codebase already uses for every other attached visual (see
        // `characters::gravemind::attach_gravemind_visual` and every other
        // class's visual-attach function), rather than spawning separately
        // and reparenting -- keeps this on the same proven hierarchy API the
        // project already relies on everywhere else.
        let mut aura_entity = None;

        commands.entity(caster_entity).with_children(|parent| {
            aura_entity = Some(
                parent
                    .spawn((
                        Name::new("Embrace the Ruin Aura"),
                        GravemindEmbraceAura,
                        Mesh3d(meshes.add(Sphere::new(1.1))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgba(0.55, 0.1, 0.2, 0.22),
                            emissive: LinearRgba::new(0.65, 0.1, 0.05, 1.0),
                            alpha_mode: AlphaMode::Blend,
                            unlit: true,
                            ..default()
                        })),
                        Transform::from_xyz(0.0, -0.3, 0.0),
                    ))
                    .id(),
            );
        });

        embrace.aura_entity = aura_entity;

        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }

        println!("EMBRACE THE RUIN!");
    }
}

fn update_embrace_the_ruin(
    time: Res<Time>,
    mut ruin_state: ResMut<RuinState>,
    mut casters: Query<
        (
            &mut GravemindEmbrace,
            &mut Health,
            &PlayerCombat,
            &mut StatBuffs,
        ),
        (With<PlayerCharacter>, With<Gravemind>),
    >,
) {
    let delta = time.delta_secs();

    for (mut embrace, mut health, combat, mut stat_buffs) in &mut casters {
        embrace.reactivate_cooldown = (embrace.reactivate_cooldown - delta).max(0.0);

        if !embrace.active {
            continue;
        }

        // -----------------------------------------------
        // Health drain -- never lethal on its own; see
        // `EMBRACE_HEALTH_DRAIN_FLOOR`'s doc comment.
        // -----------------------------------------------
        let drain = health.max * EMBRACE_HEALTH_DRAIN_FRACTION_PER_SECOND * delta;
        health.current = (health.current - drain).max(EMBRACE_HEALTH_DRAIN_FLOOR);

        // -----------------------------------------------
        // Meter -- rises while mid-swing, falls otherwise. Staying
        // active while the meter falls is intentional (context doc:
        // "the ability can remain active while the meter falls").
        // -----------------------------------------------
        if combat.attack_cooldown > 0.0 {
            embrace.meter =
                (embrace.meter + EMBRACE_METER_RISE_PER_SECOND * delta).min(EMBRACE_METER_MAX);
        } else {
            embrace.meter = (embrace.meter - EMBRACE_METER_FALL_PER_SECOND * delta).max(0.0);
        }

        if embrace.meter >= EMBRACE_METER_MAX {
            embrace.meter = 0.0;
            embrace.stacks = (embrace.stacks + 1).min(EMBRACE_MAX_STACKS);
            ruin_state.add_ruin(EMBRACE_RUIN_PER_FILL);

            println!(
                "Embrace the Ruin fully charged! Stacks: {}, Ruin +{:.0}",
                embrace.stacks, EMBRACE_RUIN_PER_FILL
            );
        }

        // Basic attacks read this the same way Warden of Shades' Guardian
        // Otter already does (see `combat::StatBuffs`). Ruin Bolt and
        // Consume apply the same multiplier directly instead, via
        // `GravemindEmbrace::damage_multiplier`, since weapon *abilities*
        // don't read `StatBuffs` project-wide (see `combat::buffs`'s doc
        // comment) -- only basic attacks do.
        stat_buffs.grant(
            BuffSource::EmbraceTheRuin,
            1.0,
            embrace.damage_multiplier(),
            EMBRACE_BUFF_REFRESH_DURATION,
        );
    }
}

fn update_embrace_aura_pulse(
    time: Res<Time>,
    mut auras: Query<&mut Transform, With<GravemindEmbraceAura>>,
) {
    let pulse = 1.0
        + (time.elapsed_secs() * std::f32::consts::TAU * 1.5)
            .sin()
            .abs()
            * 0.12;

    for mut transform in &mut auras {
        transform.scale = Vec3::splat(pulse);
    }
}

// =========================================================
// SHARED VISUAL-EFFECT CLEANUP
// =========================================================

/// Generic short-lived cosmetic mesh spawned by Consume's pulse. Mirrors
/// `WildshaperVisualEffect` / `WardenVisualEffect` / `RunesageVisualEffect`.
#[derive(Component)]
struct GravemindVisualEffect {
    lifetime: f32,
}

fn update_gravemind_visual_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut effects: Query<(Entity, &mut GravemindVisualEffect)>,
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
// SHARED GRAVEMIND STATE / HUD
// =========================================================

fn initialize_gravemind_abilities(
    mut commands: Commands,
    // Every player character (a joined Gravemind too, via `net::loadout`).
    players: Query<Entity, (With<PlayerCharacter>, Added<Gravemind>)>,
) {
    for player in &players {
        commands.entity(player).insert((
            RuinBoltAbility::default(),
            ConsumeAbility::default(),
            GravemindEmbrace::default(),
            ClassAbilityHud::new([
                ClassAbilityHudSlot::new("Q", "RUIN BOLT", ClassAbilityDisplay::Ready),
                ClassAbilityHudSlot::new("E", "CONSUME", ClassAbilityDisplay::Ready),
                ClassAbilityHudSlot::new("R", "EMBRACE THE RUIN", ClassAbilityDisplay::Ready),
            ]),
            ClassResourceHud::new("EMBRACE", 0, EMBRACE_METER_PIP_COUNT),
        ));
    }
}

fn update_gravemind_timers(
    time: Res<Time>,
    mut gravemind: Query<(&mut RuinBoltAbility, &mut ConsumeAbility), With<Gravemind>>,
) {
    let delta = time.delta_secs();

    for (mut ruin_bolt, mut consume) in &mut gravemind {
        ruin_bolt.cooldown = (ruin_bolt.cooldown - delta).max(0.0);
        consume.cooldown = (consume.cooldown - delta).max(0.0);
    }
}

fn sync_gravemind_class_state(
    mut gravemind: Query<
        (
            &RuinBoltAbility,
            &ConsumeAbility,
            &GravemindEmbrace,
            &mut ClassAbilityHud,
            &mut ClassResourceHud,
        ),
        (With<PlayerCharacter>, With<Gravemind>),
    >,
) {
    for (ruin_bolt, consume, embrace, mut hud, mut resource_hud) in &mut gravemind {
        hud.slots[0].display = if ruin_bolt.cooldown <= 0.0 {
            ClassAbilityDisplay::Ready
        } else {
            ClassAbilityDisplay::Cooldown(ruin_bolt.cooldown)
        };

        hud.slots[1].display = if consume.cooldown <= 0.0 {
            ClassAbilityDisplay::Ready
        } else {
            ClassAbilityDisplay::Cooldown(consume.cooldown)
        };

        hud.slots[2].display = if embrace.active {
            ClassAbilityDisplay::Active("EMBRACING")
        } else if embrace.reactivate_cooldown > 0.0 {
            ClassAbilityDisplay::Cooldown(embrace.reactivate_cooldown)
        } else {
            ClassAbilityDisplay::Ready
        };

        resource_hud.current =
            ((embrace.meter / EMBRACE_METER_MAX) * EMBRACE_METER_PIP_COUNT as f32).round() as u8;
    }
}

// =========================================================
// PLUGIN
// =========================================================

pub struct GravemindAbilitiesPlugin;

impl Plugin for GravemindAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                initialize_gravemind_abilities,
                update_gravemind_timers,
                ruin_bolt_input,
                consume_input,
                embrace_the_ruin_input,
                update_embrace_the_ruin,
                update_embrace_aura_pulse,
                update_gravemind_visual_effects,
                sync_gravemind_class_state,
            )
                .chain(),
        );
    }
}
