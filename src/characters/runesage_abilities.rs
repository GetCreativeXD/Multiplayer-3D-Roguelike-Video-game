//! Runesage's rune resource and three class abilities: Rune Blast, Runic
//! Surge, and Grand Rune (see the project context doc, section 20).
//!
//! Structurally this mirrors `characters::vanguard_abilities` and
//! `characters::warden_abilities`: one input system per ability plus a
//! `sync_runesage_class_state` system that writes current state into the
//! shared `ClassAbilityHud`/`ClassResourceHud` the same way the other two
//! classes do. The one real structural difference is the resource itself
//! -- Runesage abilities are gated by spendable Runes rather than
//! per-ability cooldown timers, so nothing here ticks a cooldown down;
//! `RuneResource` is the only gate, and casting is always immediate. That
//! is a deliberate identity choice (see the "REJECTED: delayed rune
//! circles" note in the context doc) -- Runesage is meant to feel like a
//! fast rhythm of weapon attacks and instant casts, never a wind-up.
//!
//! Ability-power scaling: every ability below multiplies its base numbers
//! by the caster's `Equipment::ability_power_multiplier()` (see
//! `weapons::components`), exactly like Warden of Shades' abilities do.
//! Any future gear roll that raises Ability Power automatically makes
//! Rune Blast, Runic Surge, and Grand Rune all stronger together.

use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::visual_effects::{EffectShape, effect_mesh};

use crate::characters::class_abilities::{
    AbilityUseTracker, BasicAttackHitTracker, ClassAbilityDisplay, ClassAbilityHud,
    ClassAbilityHudSlot, ClassResourceHud,
};
use crate::characters::runesage::Runesage;
use crate::combat::{self, DebuffSource, EnemyDebuffs, Health};
use crate::enemies::{Enemy, EnemyCombat, EnemyState, Stagger};
use crate::player::{PlayerCharacter, PlayerIntent};
use crate::weapons::{Equipment, PlayerProjectile, Projectile, ProjectileKind, RUNIC_SURGE_SIZE};

// =========================================================
// INPUT
// =========================================================

// Class-ability slots (`player::CLASS_ABILITY_KEYS`): Q, E, R. Every
// Runesage player character (the local `Player`, or a joined player's body
// on the multiplayer host) casts from its own `PlayerIntent`, aimed with
// that player's camera (`PlayerIntent::aim_transform`).
const RUNE_BLAST_SLOT: usize = 0;
const RUNIC_SURGE_SLOT: usize = 1;
const GRAND_RUNE_SLOT: usize = 2;

// =========================================================
// RUNE RESOURCE
// =========================================================

/// Early approved tuning direction (context doc, section 20.1): a
/// maximum around 6 Runes. Exact cap is still experimental.
const RUNESAGE_MAX_RUNES: u8 = 6;

const RUNE_BLAST_COST: u8 = 1;
const RUNIC_SURGE_COST: u8 = 2;
const GRAND_RUNE_COST: u8 = 4;

/// Runesage's spendable ability resource. Every confirmed (landed)
/// basic-attack hit restores one Rune immediately -- a missed swing
/// grants nothing, and hits do not need to be consecutive. Watched the
/// same way Vanguard's Momentum watches `BasicAttackHitTracker` (see
/// `characters::vanguard_abilities`).
#[derive(Component)]
pub struct RuneResource {
    current: u8,
    max: u8,
    hits_observed: u64,
}

impl Default for RuneResource {
    fn default() -> Self {
        Self {
            current: 0,
            max: RUNESAGE_MAX_RUNES,
            hits_observed: 0,
        }
    }
}

impl RuneResource {
    pub fn current(&self) -> u8 {
        self.current
    }

    pub fn max(&self) -> u8 {
        self.max
    }

    fn can_afford(&self, cost: u8) -> bool {
        self.current >= cost
    }

    fn spend(&mut self, cost: u8) {
        self.current = self.current.saturating_sub(cost);
    }

    fn gain(&mut self, amount: u8) {
        self.current = (self.current + amount).min(self.max);
    }
}

/// Watches `BasicAttackHitTracker` for newly confirmed (landed) basic
/// attack hits and grants one Rune per hit, immediately -- a swing that
/// misses every enemy grants nothing, matching how Vanguard's Momentum
/// already reads the same tracker. Bounded to a sane number of Runes
/// granted per frame -- a player can't realistically land more than a
/// handful of basic attacks in one frame, but nothing here should ever
/// spin unbounded on unexpected input.
fn update_rune_resource(
    mut runesages: Query<
        (&mut RuneResource, &BasicAttackHitTracker),
        (With<PlayerCharacter>, With<Runesage>),
    >,
) {
    for (mut runes, basic_attack_hits) in &mut runesages {
        let new_hits = basic_attack_hits
            .total_confirmed()
            .saturating_sub(runes.hits_observed);
        runes.hits_observed = basic_attack_hits.total_confirmed();

        for _ in 0..new_hits.min(64) {
            runes.gain(1);
            println!("Rune restored: {}/{}", runes.current, runes.max);
        }
    }
}

// =========================================================
// RUNE BLAST (Q, 1 Rune)
// =========================================================

const RUNE_BLAST_SPEED: f32 = 30.0;
const RUNE_BLAST_RANGE: f32 = 26.0;
const RUNE_BLAST_LIFETIME: f32 = RUNE_BLAST_RANGE / RUNE_BLAST_SPEED;
const RUNE_BLAST_BASE_DAMAGE: f32 = 14.0;
const RUNE_BLAST_KNOCKBACK: f32 = 0.5;
const RUNE_BLAST_STAGGER_STRENGTH: u32 = 1;
const RUNE_BLAST_STAGGER_DURATION: f32 = 0.25;

fn rune_blast_input(
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
            &mut RuneResource,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Runesage>),
    >,
) {
    for (caster_entity, mut transform, mut intent, equipment, mut runes, mut ability_tracker) in
        &mut caster_query
    {
        // A press (Q, for the local player) is consumed even when it can't
        // cast, as `just_pressed` was before.
        if !intent.take_class_ability(RUNE_BLAST_SLOT) {
            continue;
        }
        let camera_transform = intent.aim_transform();

        if !runes.can_afford(RUNE_BLAST_COST) {
            continue;
        }

        let camera_origin = camera_transform.translation;
        let camera_direction = camera_transform.forward().as_vec3().normalize_or_zero();

        if camera_direction == Vec3::ZERO {
            continue;
        }

        // Crosshair-raycast-to-point aiming, same approach as
        // `weapons::bow::fire_bow`: find what the crosshair is actually
        // looking at, then aim the bolt from its spawn point toward that
        // point instead of just firing along the camera's forward vector
        // (which would drift off-target due to the camera/player offset).
        let filter = SpatialQueryFilter::from_excluded_entities([caster_entity]);
        let mut target_point = camera_origin + camera_direction * RUNE_BLAST_RANGE;

        if let Ok(ray_direction) = Dir3::new(camera_direction.adjust_precision()) {
            if let Some(hit) = spatial_query.cast_ray(
                camera_origin.adjust_precision(),
                ray_direction,
                RUNE_BLAST_RANGE,
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

        let ability_power = equipment.ability_power_multiplier();
        let damage = RUNE_BLAST_BASE_DAMAGE * ability_power;

        commands.spawn((
            PlayerProjectile,
            Projectile {
                velocity: projectile_direction * RUNE_BLAST_SPEED,
                damage,
                knockback: RUNE_BLAST_KNOCKBACK,
                stagger_strength: RUNE_BLAST_STAGGER_STRENGTH,
                stagger_duration: RUNE_BLAST_STAGGER_DURATION,
                lifetime: RUNE_BLAST_LIFETIME,
                piercing: false,
                hit_entities: Vec::new(),
                kind: ProjectileKind::RuneBlast,
                target: None,
            },
            Mesh3d(meshes.add(Sphere::new(0.22))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.55, 0.85, 1.0),
                emissive: LinearRgba::new(0.3, 0.65, 1.0, 1.0),
                unlit: true,
                ..default()
            })),
            Transform::from_translation(spawn_position),
        ));

        runes.spend(RUNE_BLAST_COST);

        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }

        println!("Rune Blast! Runes {}/{}", runes.current(), runes.max());
    }
}

// =========================================================
// RUNIC SURGE (E, 2 Runes)
// =========================================================

const RUNIC_SURGE_SPEED: f32 = 26.0;
/// How far the wave travels before fading -- short and fast enough to
/// read as an instantaneous close/mid-range answer rather than a
/// projectile the target can simply outrun.
const RUNIC_SURGE_RANGE: f32 = 7.0;
const RUNIC_SURGE_LIFETIME: f32 = RUNIC_SURGE_RANGE / RUNIC_SURGE_SPEED;
const RUNIC_SURGE_BASE_DAMAGE: f32 = 22.0;
const RUNIC_SURGE_KNOCKBACK: f32 = 2.4;
/// Comfortably above elite stagger resistance (~2, per the context doc's
/// stagger table), matching Warden's Shadow Stag -- a strong, reliable
/// interrupt without being strong enough to lock down a boss alone.
const RUNIC_SURGE_STAGGER_STRENGTH: u32 = 3;
const RUNIC_SURGE_STAGGER_DURATION: f32 = 0.85;
const RUNIC_SURGE_SPAWN_OFFSET: f32 = 1.2;

fn runic_surge_input(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut caster_query: Query<
        (
            &mut Transform,
            &mut PlayerIntent,
            &Equipment,
            &mut RuneResource,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Runesage>),
    >,
) {
    for (mut transform, mut intent, equipment, mut runes, mut ability_tracker) in &mut caster_query
    {
        // E, for the local player.
        if !intent.take_class_ability(RUNIC_SURGE_SLOT) {
            continue;
        }
        let camera_transform = intent.aim_transform();

        if !runes.can_afford(RUNIC_SURGE_COST) {
            continue;
        }

        let mut direction = camera_transform.forward().as_vec3();
        direction.y = 0.0;
        direction = direction.normalize_or_zero();

        if direction == Vec3::ZERO {
            continue;
        }

        // The wave is released instantly in front of the Runesage -- only
        // the caster's facing changes, nothing pulls or pushes the caster
        // themselves, matching "immediate rather than delayed."
        transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));

        let spawn_position =
            transform.translation + direction * RUNIC_SURGE_SPAWN_OFFSET + Vec3::Y * 0.9;

        let ability_power = equipment.ability_power_multiplier();
        let damage = RUNIC_SURGE_BASE_DAMAGE * ability_power;

        // A wide, piercing `Projectile` -- everyone caught in the wave's path
        // is hit once, same shared movement/collision system every arrow and
        // Warden's Shadow Stag already use (see
        // `weapons::projectiles::movement::move_projectiles_with_collision`).
        commands.spawn((
            PlayerProjectile,
            Projectile {
                velocity: direction * RUNIC_SURGE_SPEED,
                damage,
                knockback: RUNIC_SURGE_KNOCKBACK,
                stagger_strength: RUNIC_SURGE_STAGGER_STRENGTH,
                stagger_duration: RUNIC_SURGE_STAGGER_DURATION,
                lifetime: RUNIC_SURGE_LIFETIME,
                piercing: true,
                hit_entities: Vec::new(),
                kind: ProjectileKind::RunicSurge,
                target: None,
            },
            Mesh3d(meshes.add(Cuboid::new(
                RUNIC_SURGE_SIZE.x,
                RUNIC_SURGE_SIZE.y,
                RUNIC_SURGE_SIZE.z,
            ))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.65, 0.4, 1.0, 0.55),
                emissive: LinearRgba::new(0.5, 0.25, 1.0, 1.0),
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

        runes.spend(RUNIC_SURGE_COST);

        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }

        // Emergence burst, roughly where the wave appears from the Runesage.
        commands.spawn((
            RunesageVisualEffect { lifetime: 0.3 },
            effect_mesh(
                &mut *meshes,
                EffectShape::Torus {
                    inner_radius: 0.55,
                    outer_radius: 0.85,
                },
            ),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.65, 0.4, 1.0, 0.7),
                emissive: LinearRgba::new(0.5, 0.25, 1.0, 1.0),
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

        println!("Runic Surge! Runes {}/{}", runes.current(), runes.max());
    }
}

// =========================================================
// GRAND RUNE (R, 4 Runes)
// =========================================================

/// How far the crosshair raycast looks for "the targeted enemy." Same
/// simplification Warden's Predator's Spirit uses for its proc target.
const GRAND_RUNE_TARGET_RANGE: f32 = 30.0;
const GRAND_RUNE_RADIUS: f32 = 5.0;
const GRAND_RUNE_BASE_DAMAGE: f32 = 55.0;
const GRAND_RUNE_KNOCKBACK: f32 = 3.0;
const GRAND_RUNE_STAGGER_STRENGTH: u32 = 3;
const GRAND_RUNE_STAGGER_DURATION: f32 = 1.0;

/// "Runebroken" (context doc, section 20, Ability 3): a temporary debuff
/// that increases damage and stagger taken. Exact numbers are tuning
/// placeholders, same as everything else in this file.
const GRAND_RUNE_RUNEBROKEN_DAMAGE_MULTIPLIER: f32 = 1.35;
const GRAND_RUNE_RUNEBROKEN_STAGGER_REDUCTION: u32 = 2;
const GRAND_RUNE_RUNEBROKEN_DURATION: f32 = 6.0;

fn grand_rune_input(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spatial_query: SpatialQuery,
    mut caster_query: Query<
        (
            Entity,
            &mut PlayerIntent,
            &Equipment,
            &mut RuneResource,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Runesage>, Without<Enemy>),
    >,
    mut enemies: Query<
        (
            &mut Transform,
            &mut Health,
            &mut Stagger,
            &mut EnemyCombat,
            Option<&mut EnemyDebuffs>,
        ),
        (With<Enemy>, Without<PlayerCharacter>),
    >,
) {
    for (caster_entity, mut intent, equipment, mut runes, mut ability_tracker) in &mut caster_query
    {
        // R, for the local player.
        if !intent.take_class_ability(GRAND_RUNE_SLOT) {
            continue;
        }
        let camera_transform = intent.aim_transform();

        if !runes.can_afford(GRAND_RUNE_COST) {
            continue;
        }

        let origin = camera_transform.translation;
        let direction = camera_transform.forward().as_vec3().normalize_or_zero();

        let Ok(ray_direction) = Dir3::new(direction.adjust_precision()) else {
            continue;
        };

        let filter = SpatialQueryFilter::from_excluded_entities([caster_entity]);

        let Some(hit) = spatial_query.cast_ray(
            origin.adjust_precision(),
            ray_direction,
            GRAND_RUNE_TARGET_RANGE,
            true,
            &filter,
        ) else {
            println!("Grand Rune: no target under crosshair");
            continue;
        };

        // Only a targeted enemy triggers Grand Rune -- hitting world geometry
        // is not a valid cast, mirroring Vanguard's Lunge ("no target in
        // range") rather than spending a Rune on nothing.
        if !enemies.contains(hit.entity) {
            println!("Grand Rune: crosshair isn't over an enemy");
            continue;
        }

        let center = origin + direction * hit.distance;
        let ability_power = equipment.ability_power_multiplier();
        let damage = GRAND_RUNE_BASE_DAMAGE * ability_power;

        for (mut transform, mut health, mut stagger, mut combat, debuffs) in &mut enemies {
            let distance = transform.translation.distance(center);

            if distance > GRAND_RUNE_RADIUS {
                continue;
            }

            combat::apply_enemy_damage(&mut health, debuffs.as_deref(), damage);

            let effective_resistance =
                combat::effective_stagger_resistance(stagger.resistance, debuffs.as_deref());

            if GRAND_RUNE_STAGGER_STRENGTH > effective_resistance {
                stagger.timer = GRAND_RUNE_STAGGER_DURATION;
                combat.state = EnemyState::Chase;
                combat.timer = 0.0;
                combat.has_hit = false;
            }

            let mut knockback_direction = transform.translation - center;
            knockback_direction.y = 0.0;
            let knockback_direction = knockback_direction.normalize_or_zero();
            transform.translation += knockback_direction * GRAND_RUNE_KNOCKBACK;

            if let Some(mut debuffs) = debuffs {
                debuffs.grant(
                    DebuffSource::GrandRune,
                    GRAND_RUNE_RUNEBROKEN_DAMAGE_MULTIPLIER,
                    GRAND_RUNE_RUNEBROKEN_STAGGER_REDUCTION,
                    GRAND_RUNE_RUNEBROKEN_DURATION,
                );
            }
        }

        runes.spend(GRAND_RUNE_COST);

        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }

        commands.spawn((
            RunesageVisualEffect { lifetime: 0.4 },
            effect_mesh(
                &mut *meshes,
                EffectShape::Sphere {
                    radius: GRAND_RUNE_RADIUS * 0.9,
                },
            ),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.7, 0.35, 1.0, 0.35),
                emissive: LinearRgba::new(0.55, 0.2, 1.0, 1.0),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(center),
        ));

        println!(
            "Grand Rune detonates! Runes {}/{}",
            runes.current(),
            runes.max()
        );
    }
}

// =========================================================
// SHARED VISUAL-EFFECT CLEANUP
// =========================================================

/// Generic short-lived cosmetic mesh spawned by Runic Surge's emergence
/// burst and Grand Rune's explosion flash. Mirrors Warden's
/// `WardenVisualEffect` (see `characters::warden_abilities`).
#[derive(Component)]
struct RunesageVisualEffect {
    lifetime: f32,
}

fn update_runesage_visual_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut effects: Query<(Entity, &mut RunesageVisualEffect)>,
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
// SHARED RUNESAGE STATE / HUD
// =========================================================

fn initialize_runesage_abilities(
    mut commands: Commands,
    // Every player character (a joined Runesage too, via `net::loadout`).
    players: Query<Entity, (With<PlayerCharacter>, Added<Runesage>)>,
) {
    for player in &players {
        commands.entity(player).insert((
            RuneResource::default(),
            ClassAbilityHud::new([
                ClassAbilityHudSlot::new("Q", "RUNE BLAST", ClassAbilityDisplay::Ready),
                ClassAbilityHudSlot::new("E", "RUNIC SURGE", ClassAbilityDisplay::Ready),
                ClassAbilityHudSlot::new("R", "GRAND RUNE", ClassAbilityDisplay::Ready),
            ]),
            ClassResourceHud::new("RUNES", 0, RUNESAGE_MAX_RUNES),
        ));
    }
}

fn sync_runesage_class_state(
    mut runesages: Query<
        (&RuneResource, &mut ClassAbilityHud, &mut ClassResourceHud),
        (With<PlayerCharacter>, With<Runesage>),
    >,
) {
    for (runes, mut hud, mut resource_hud) in &mut runesages {
        hud.slots[0].display = rune_ability_display(runes.current(), RUNE_BLAST_COST);
        hud.slots[1].display = rune_ability_display(runes.current(), RUNIC_SURGE_COST);
        hud.slots[2].display = rune_ability_display(runes.current(), GRAND_RUNE_COST);

        resource_hud.current = runes.current();
        resource_hud.max = runes.max();
    }
}

fn rune_ability_display(current: u8, cost: u8) -> ClassAbilityDisplay {
    if current >= cost {
        ClassAbilityDisplay::Ready
    } else {
        ClassAbilityDisplay::Locked(rune_cost_label(cost))
    }
}

fn rune_cost_label(cost: u8) -> &'static str {
    match cost {
        1 => "NEED 1",
        2 => "NEED 2",
        4 => "NEED 4",
        _ => "NEED RUNES",
    }
}

// =========================================================
// PLUGIN
// =========================================================

pub struct RunesageAbilitiesPlugin;

impl Plugin for RunesageAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                initialize_runesage_abilities,
                update_rune_resource,
                rune_blast_input,
                runic_surge_input,
                grand_rune_input,
                update_runesage_visual_effects,
                sync_runesage_class_state,
            )
                .chain(),
        );
    }
}
