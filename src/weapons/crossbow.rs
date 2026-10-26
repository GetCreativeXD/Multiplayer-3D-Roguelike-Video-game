//! Crossbow -- see WEAPONS.md's Crossbow section for the design source
//! of truth. Identity: slow, powerful, deliberate ranged attacks that
//! reward positioning and commitment, distinct from Bow's precision/kiting
//! kit. Every `fire_*` function here shares one aiming helper
//! (`aim_crossbow`) instead of each tier reimplementing the
//! camera-raycast-then-rotate-player boilerplate `weapons::bow` repeats
//! per function.
//!
//! Basic attack + Common (Hunter's Bolt) + Rare (Gravewind Bolt) + Epic
//! (Armorbreaker Shot) + all three Legendaries (Saint's Nail, The
//! Ruinpiercer, Bellringer) are implemented here.

use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::combat::{self, DebuffSource, EnemyDebuffs, Health};
use crate::enemies::Enemy;
use crate::items::ItemDefinitionId;
use crate::player::{Player, PlayerCharacter};
use crate::ruin::RuinState;

use super::components::{
    PlayerProjectile, Projectile, ProjectileKind, ProjectileOwner, Weapon, WeaponAbilityState,
};

// =========================================================
// SHARED
// =========================================================

/// Bolts travel faster than an arrow (WEAPONS.md's request: the Crossbow
/// "rewards careful positioning" partly because a bolt reaches its target
/// sooner than a Bow's arrow would) -- see `weapons::ARROW_SPEED`.
pub(crate) const CROSSBOW_BOLT_SPEED: f32 = 56.0;
const CROSSBOW_BOLT_LIFETIME: f32 = 3.0;
const CROSSBOW_BOLT_RADIUS: f32 = 0.16;

const CROSSBOW_BOLT_COLOR: Color = Color::srgb(0.55, 0.55, 0.62);

/// Shared aiming step every Crossbow attack uses: raycasts from the
/// camera through the crosshair (capped at `weapon.range`), derives the
/// bolt's spawn position and direction, and turns the player to face it.
/// Pulled out once here instead of repeating the block in every
/// `fire_*` function the way `weapons::bow` does -- any new Crossbow
/// ability should call this rather than reimplementing it.
fn aim_crossbow(
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
) -> Option<(Vec3, Vec3)> {
    let camera_origin = camera_transform.translation;
    let camera_direction = camera_transform.forward().as_vec3().normalize_or_zero();

    if camera_direction == Vec3::ZERO {
        return None;
    }

    let filter = SpatialQueryFilter::from_excluded_entities([player_entity]);
    let max_distance = weapon.range;
    let mut target_point = camera_origin + camera_direction * max_distance;

    if let Ok(ray_direction) = Dir3::new(camera_direction.adjust_precision()) {
        if let Some(hit) = spatial_query.cast_ray(
            camera_origin.adjust_precision(),
            ray_direction,
            max_distance,
            true,
            &filter,
        ) {
            target_point = camera_origin + camera_direction * hit.distance;
        }
    }

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

    Some((spawn_position, bolt_direction))
}

/// How far the raycast in `aim_crossbow` actually traveled before hitting
/// something (or the weapon's max range, if nothing was hit) -- read by
/// Hunter's Bolt to reward long-range shots. A second, cheap raycast
/// rather than threading the first one's distance through every caller.
fn aim_distance(
    camera_transform: &Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
) -> f32 {
    let camera_origin = camera_transform.translation;
    let camera_direction = camera_transform.forward().as_vec3().normalize_or_zero();

    if camera_direction == Vec3::ZERO {
        return 0.0;
    }

    let filter = SpatialQueryFilter::from_excluded_entities([player_entity]);

    if let Ok(ray_direction) = Dir3::new(camera_direction.adjust_precision()) {
        if let Some(hit) = spatial_query.cast_ray(
            camera_origin.adjust_precision(),
            ray_direction,
            weapon.range,
            true,
            &filter,
        ) {
            return hit.distance;
        }
    }

    weapon.range
}

fn spawn_bolt(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    spawn_position: Vec3,
    direction: Vec3,
    speed: f32,
    damage: f32,
    weapon: &Weapon,
    lifetime: f32,
    piercing: bool,
    kind: ProjectileKind,
    radius: f32,
    color: Color,
    // The player who fired it (see `ProjectileOwner`). Basic attacks and
    // abilities both pass it, so hits credit the right player.
    owner: Entity,
) -> Entity {
    commands
        .spawn((
            PlayerProjectile,
            ProjectileOwner(owner),
            Projectile {
                velocity: direction * speed,
                damage,
                knockback: weapon.knockback,
                stagger_strength: weapon.stagger_strength,
                stagger_duration: weapon.stagger_duration,
                lifetime,
                piercing,
                hit_entities: Vec::new(),
                kind,
                target: None,
            },
            Mesh3d(meshes.add(Sphere::new(radius))),
            MeshMaterial3d(materials.add(color)),
            Transform::from_translation(spawn_position),
        ))
        .id()
}

// =========================================================
// BASIC ATTACK
// =========================================================

pub(crate) fn fire_crossbow(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    damage_multiplier: f32,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
) {
    let Some((spawn_position, direction)) = aim_crossbow(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
    ) else {
        return;
    };

    spawn_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        CROSSBOW_BOLT_SPEED,
        weapon.damage * damage_multiplier,
        weapon,
        CROSSBOW_BOLT_LIFETIME,
        false,
        ProjectileKind::Normal,
        CROSSBOW_BOLT_RADIUS,
        CROSSBOW_BOLT_COLOR,
        player_entity,
    );

    println!("Crossbow fired!");
}

// =========================================================
// COMMON -- Rusted Hunter's Crossbow: Hunter's Bolt
// =========================================================

const HUNTERS_BOLT_MIN_MULTIPLIER: f32 = 1.0;
/// Bonus damage at (or beyond) the weapon's max range -- WEAPONS.md:
/// "deals increased damage at longer range... rewards staying at a safe
/// distance." Resolved once at cast time from how far the aiming raycast
/// actually traveled, rather than needing to track distance-traveled on
/// the projectile itself.
const HUNTERS_BOLT_MAX_MULTIPLIER: f32 = 1.6;

pub(crate) fn fire_hunters_bolt(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
) {
    let Some((spawn_position, direction)) = aim_crossbow(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
    ) else {
        return;
    };

    let distance = aim_distance(camera_transform, weapon, spatial_query, player_entity);
    let range_fraction = (distance / weapon.range.max(0.01)).clamp(0.0, 1.0);
    let range_multiplier = HUNTERS_BOLT_MIN_MULTIPLIER
        + (HUNTERS_BOLT_MAX_MULTIPLIER - HUNTERS_BOLT_MIN_MULTIPLIER) * range_fraction;

    spawn_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        CROSSBOW_BOLT_SPEED,
        weapon.damage * range_multiplier,
        weapon,
        CROSSBOW_BOLT_LIFETIME,
        false,
        ProjectileKind::Normal,
        CROSSBOW_BOLT_RADIUS * 1.2,
        Color::srgb(0.8, 0.7, 0.4),
        player_entity,
    );

    println!("Hunter's Bolt! Range multiplier {:.2}", range_multiplier);
}

// =========================================================
// RARE -- Gravewind Crossbow: Gravewind Bolt
// =========================================================

const GRAVEWIND_DAMAGE_MULTIPLIER: f32 = 1.3;
/// Damage falloff applied per enemy already pierced -- WEAPONS.md:
/// "strikes enemies behind it for reduced damage." Read by
/// `weapons::projectiles::movement` at each hit.
const GRAVEWIND_FALLOFF: f32 = 0.55;

/// Damage multiplier for the `pierce_index`-th enemy a Gravewind Bolt
/// hits (`0` for the first enemy struck, `1` for the second, ...).
pub(crate) fn gravewind_damage_multiplier(pierce_index: u32) -> f32 {
    GRAVEWIND_FALLOFF.powi(pierce_index as i32)
}

pub(crate) fn fire_gravewind_bolt(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
) {
    let Some((spawn_position, direction)) = aim_crossbow(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
    ) else {
        return;
    };

    spawn_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        CROSSBOW_BOLT_SPEED,
        weapon.damage * GRAVEWIND_DAMAGE_MULTIPLIER,
        weapon,
        CROSSBOW_BOLT_LIFETIME,
        true,
        ProjectileKind::GravewindBolt,
        CROSSBOW_BOLT_RADIUS * 1.2,
        Color::srgb(0.55, 0.85, 0.85),
        player_entity,
    );

    println!("Gravewind Bolt!");
}

// =========================================================
// EPIC -- Siegebreaker Crossbow: Armorbreaker Shot
// =========================================================

const ARMORBREAKER_DAMAGE_MULTIPLIER: f32 = 1.8;
/// Extra stagger this shot carries on top of the Crossbow's base
/// `stagger_strength` -- WEAPONS.md: "heavily staggers enemies... strong
/// against armored enemies and elites."
const ARMORBREAKER_STAGGER_BONUS: u32 = 2;
/// "Fully charged shots reduce the target's defenses briefly." Enemies
/// don't have a separate defense stat (only the player does -- see
/// `Equipment::total_defense`), so this is implemented as a temporary
/// increased-damage-taken window via the existing, reusable
/// `EnemyDebuffs` system -- the same simplification Saint's Nail's Mark
/// uses (see `combat::enemy_debuffs::DebuffSource`'s doc comments).
pub(crate) const ARMORBREAKER_DEBUFF_MULTIPLIER: f32 = 1.35;
pub(crate) const ARMORBREAKER_DEBUFF_DURATION: f32 = 4.0;

pub(crate) fn fire_armorbreaker_shot(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
) {
    let Some((spawn_position, direction)) = aim_crossbow(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
    ) else {
        return;
    };

    commands.spawn((
        PlayerProjectile,
        ProjectileOwner(player_entity),
        Projectile {
            velocity: direction * CROSSBOW_BOLT_SPEED,
            damage: weapon.damage * ARMORBREAKER_DAMAGE_MULTIPLIER,
            knockback: weapon.knockback,
            stagger_strength: weapon.stagger_strength + ARMORBREAKER_STAGGER_BONUS,
            stagger_duration: weapon.stagger_duration,
            lifetime: CROSSBOW_BOLT_LIFETIME,
            piercing: false,
            hit_entities: Vec::new(),
            kind: ProjectileKind::ArmorbreakerBolt,
            target: None,
        },
        Mesh3d(meshes.add(Sphere::new(CROSSBOW_BOLT_RADIUS * 1.4))),
        MeshMaterial3d(materials.add(Color::srgb(0.95, 0.35, 0.2))),
        Transform::from_translation(spawn_position),
    ));

    println!("Armorbreaker Shot!");
}

// =========================================================
// LEGENDARY -- Saint's Nail: Consecrated Bolt
// =========================================================

pub(crate) const SAINTS_NAIL_ID: ItemDefinitionId =
    ItemDefinitionId("weapon.crossbow.legendary.saints_nail");
pub(crate) const SAINTS_NAIL_NAME: &str = "Saint's Nail";

const SAINTS_NAIL_DAMAGE_MULTIPLIER: f32 = 1.5;
const SAINTS_NAIL_MARK_MULTIPLIER: f32 = 1.25;
const SAINTS_NAIL_MARK_DURATION: f32 = 6.0;
/// Final Judgment: "The first fully charged shot against an elite or
/// boss deals bonus damage and applies the mark immediately." Ruinbound
/// doesn't have a charge-up meter yet, so this is read instead as "the
/// first Consecrated Bolt to land on a tough enemy after this cooldown"
/// -- resolved in `weapons::projectiles::movement` where the target's
/// real `Stagger::resistance` is known, using the same "elite-tier
/// resistance" cutoff `weapons::abilities::hammer` already documents.
pub(crate) const SAINTS_NAIL_ELITE_RESISTANCE_THRESHOLD: u32 = 2;
pub(crate) const SAINTS_NAIL_JUDGMENT_DAMAGE_MULTIPLIER: f32 = 1.75;
pub(crate) const SAINTS_NAIL_JUDGMENT_COOLDOWN: f32 = 20.0;

/// Radius of the holy burst a Marked enemy releases on death (WEAPONS.md:
/// "releases a small holy burst that damages nearby enemies").
const SAINTS_NAIL_BURST_RADIUS: f32 = 3.2;
const SAINTS_NAIL_BURST_DAMAGE: f32 = 16.0;

pub(crate) fn fire_saints_nail(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
) {
    let Some((spawn_position, direction)) = aim_crossbow(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
    ) else {
        return;
    };

    spawn_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        CROSSBOW_BOLT_SPEED,
        weapon.damage * SAINTS_NAIL_DAMAGE_MULTIPLIER,
        weapon,
        CROSSBOW_BOLT_LIFETIME,
        false,
        ProjectileKind::ConsecratedBolt,
        CROSSBOW_BOLT_RADIUS * 1.3,
        Color::srgb(1.0, 0.92, 0.55),
        player_entity,
    );

    println!("Consecrated Bolt!");
}

/// Applies Consecrated Mark to `target`, and -- if Final Judgment is
/// ready and `target_resistance` reads as elite/boss-tier -- consumes it
/// for bonus damage. Returns the multiplier the caller should apply to
/// the bolt's already-computed hit damage (`1.0` when Judgment doesn't
/// trigger). Called from `weapons::projectiles::movement`'s
/// ConsecratedBolt hit handling, which is the only place that has both
/// the target's `EnemyDebuffs` and its real `Stagger::resistance` on
/// hand.
pub(crate) fn apply_consecrated_mark(
    enemy_debuffs: &mut EnemyDebuffs,
    target_resistance: u32,
    state: &mut WeaponAbilityState,
) -> f32 {
    enemy_debuffs.grant(
        DebuffSource::ConsecratedMark,
        SAINTS_NAIL_MARK_MULTIPLIER,
        0,
        SAINTS_NAIL_MARK_DURATION,
    );

    if state.saints_nail_judgment_cooldown <= 0.0
        && target_resistance >= SAINTS_NAIL_ELITE_RESISTANCE_THRESHOLD
    {
        state.saints_nail_judgment_cooldown = SAINTS_NAIL_JUDGMENT_COOLDOWN;
        println!("Final Judgment!");
        return SAINTS_NAIL_JUDGMENT_DAMAGE_MULTIPLIER;
    }

    1.0
}

/// Saint's Nail's holy burst: any enemy Marked by Consecrated Bolt that
/// dies this frame damages every other nearby enemy once before
/// `enemies::handle_enemy_death` despawns it. Independent of, and safe to
/// run in any order relative to, `handle_enemy_death` -- both simply
/// check `health.current <= 0.0` on the same still-alive-this-frame
/// entity (Commands' despawn is deferred to the end of the schedule).
pub(crate) fn saints_nail_mark_burst(
    mut enemies: Query<(Entity, &Transform, &mut Health, Option<&EnemyDebuffs>), With<Enemy>>,
) {
    // First pass: find any dying, Marked enemy and remember where it was.
    // A single query borrowed twice in sequence (immutable, then mutable)
    // rather than two separate `Query` system params -- both would match
    // the same `Health` on overlapping entities, which Bevy rejects as a
    // conflicting access.
    let burst_origins: Vec<(Entity, Vec3)> = enemies
        .iter()
        .filter(|(_, _, health, debuffs)| {
            health.current <= 0.0
                && debuffs.is_some_and(|debuffs| debuffs.has_source(DebuffSource::ConsecratedMark))
        })
        .map(|(entity, transform, _, _)| (entity, transform.translation))
        .collect();

    if burst_origins.is_empty() {
        return;
    }

    for (source_entity, origin) in burst_origins {
        for (entity, transform, mut health, debuffs) in &mut enemies {
            if entity == source_entity {
                continue;
            }

            if transform.translation.distance(origin) > SAINTS_NAIL_BURST_RADIUS {
                continue;
            }

            combat::apply_enemy_damage(&mut health, debuffs, SAINTS_NAIL_BURST_DAMAGE);
        }

        println!("Saint's Nail holy burst!");
    }
}

// =========================================================
// LEGENDARY -- The Ruinpiercer: Corruption-Piercing Bolt
// =========================================================

pub(crate) const RUINPIERCER_ID: ItemDefinitionId =
    ItemDefinitionId("weapon.crossbow.legendary.ruinpiercer");
pub(crate) const RUINPIERCER_NAME: &str = "The Ruinpiercer";

/// Ruinbound doesn't have a "corrupted enemy" tag yet -- per
/// RUIN_CORRUPTION_DESIGN.md, corrupted enemies and rooms are
/// intentionally future work, not built yet. The Ruinpiercer's "increased
/// damage to corrupted enemies" is instead read against its own
/// playstyle line ("becomes more valuable as Ruin changes the run") and
/// scales with the player's *current Ruin* (`ruin::RuinState`) rather
/// than a per-enemy flag. Revisit this once corrupted enemies exist.
const RUINPIERCER_BASE_MULTIPLIER: f32 = 1.15;
const RUINPIERCER_RUIN_DAMAGE_SCALING: f32 = 0.9;
const RUINPIERCER_FALLOFF: f32 = 0.65;

pub(crate) fn ruinpiercer_damage_multiplier(pierce_index: u32) -> f32 {
    RUINPIERCER_FALLOFF.powi(pierce_index as i32)
}

/// Cleansing Strike: "has a chance to slightly reduce the player's Ruin...
/// a brief cooldown so it cannot be repeatedly farmed." Implemented as a
/// guaranteed cleanse gated purely by the cooldown (rather than a
/// probability roll on top of it) -- simpler, and just as effective at
/// preventing farming from a single encounter.
const RUINPIERCER_CLEANSE_AMOUNT: f32 = 2.0;
const RUINPIERCER_CLEANSE_COOLDOWN: f32 = 8.0;

/// Tags whatever The Ruinpiercer's bolt damages, purely so Cleansing
/// Strike's on-kill system can attribute a kill to this specific weapon
/// (Ruinbound has no general "last hit by" tracking). `owner` is the
/// player character whose bolt hit last (`ProjectileOwner`), so in co-op
/// the kill uses that player's own Cleansing Strike cooldown.
#[derive(Component)]
pub(crate) struct RuinpiercerMark {
    pub(crate) owner: Option<Entity>,
}

pub(crate) fn fire_ruinpiercer(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
    ruin_state: &RuinState,
) {
    let Some((spawn_position, direction)) = aim_crossbow(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
    ) else {
        return;
    };

    let ruin_multiplier =
        RUINPIERCER_BASE_MULTIPLIER + (ruin_state.ruin() / 100.0) * RUINPIERCER_RUIN_DAMAGE_SCALING;

    spawn_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        CROSSBOW_BOLT_SPEED,
        weapon.damage * ruin_multiplier,
        weapon,
        CROSSBOW_BOLT_LIFETIME,
        true,
        ProjectileKind::CorruptionPiercingBolt,
        CROSSBOW_BOLT_RADIUS * 1.2,
        Color::srgb(0.55, 0.85, 0.35),
        player_entity,
    );

    println!(
        "Corruption-Piercing Bolt! Ruin multiplier {:.2}",
        ruin_multiplier
    );
}

/// Cleansing Strike's on-kill check: any enemy tagged by The Ruinpiercer
/// that dies this frame reduces the run's Ruin (shared by the whole
/// party), subject to the bolt owner's `RUINPIERCER_CLEANSE_COOLDOWN`.
/// Each owner cleanses at most once per frame. A mark without an owner
/// falls back to the local `Player`. Mirrors `saints_nail_mark_burst`'s
/// "check `health.current <= 0.0` independently of `handle_enemy_death`"
/// shape.
pub(crate) fn ruinpiercer_cleansing_strike(
    dying: Query<(&Health, &RuinpiercerMark), With<Enemy>>,
    mut ruin_state: ResMut<RuinState>,
    local_player: Query<Entity, With<Player>>,
    mut owner_states: Query<&mut WeaponAbilityState, With<PlayerCharacter>>,
) {
    let mut cleansed_by: Vec<Entity> = Vec::new();

    for (health, mark) in &dying {
        if health.current > 0.0 {
            continue;
        }

        let Some(owner) = mark.owner.or_else(|| local_player.single().ok()) else {
            continue;
        };
        if cleansed_by.contains(&owner) {
            continue;
        }
        let Ok(mut state) = owner_states.get_mut(owner) else {
            continue;
        };
        if state.ruinpiercer_cleansing_cooldown > 0.0 {
            continue;
        }

        cleansed_by.push(owner);
        state.ruinpiercer_cleansing_cooldown = RUINPIERCER_CLEANSE_COOLDOWN;
        ruin_state.remove_ruin(RUINPIERCER_CLEANSE_AMOUNT);
        println!(
            "Cleansing Strike! Ruin reduced by {:.1}",
            RUINPIERCER_CLEANSE_AMOUNT
        );
    }
}

// =========================================================
// LEGENDARY -- Bellringer: Resonant Bolt
// =========================================================

pub(crate) const BELLRINGER_ID: ItemDefinitionId =
    ItemDefinitionId("weapon.crossbow.legendary.bellringer");
pub(crate) const BELLRINGER_NAME: &str = "Bellringer";

const BELLRINGER_DAMAGE_MULTIPLIER: f32 = 1.2;
pub(crate) const RESONANCE_MAX_STACKS: u32 = 4;
/// How long a Resonant target keeps its stacks without being hit again.
const RESONANCE_DECAY_SECONDS: f32 = 6.0;

pub(crate) const RESONANCE_SHOCKWAVE_RADIUS: f32 = 4.5;
pub(crate) const RESONANCE_SHOCKWAVE_DAMAGE: f32 = 20.0;
pub(crate) const RESONANCE_SHOCKWAVE_STAGGER_BONUS: u32 = 2;

/// Per-enemy Resonance stacks built by Bellringer's Resonant Bolt.
/// Inserted lazily (only enemies actually hit by Bellringer carry one),
/// mirroring `combat::DamageOverTime`.
#[derive(Component, Debug)]
pub(crate) struct Resonance {
    pub(crate) stacks: u32,
    remaining: f32,
}

impl Resonance {
    pub(crate) fn first_stack() -> Self {
        Self {
            stacks: 1,
            remaining: RESONANCE_DECAY_SECONDS,
        }
    }

    pub(crate) fn add_stack(&mut self) {
        self.stacks = (self.stacks + 1).min(RESONANCE_MAX_STACKS);
        self.remaining = RESONANCE_DECAY_SECONDS;
    }
}

/// Decays Resonance stacks that haven't been refreshed in time, removing
/// the component once it fully decays. Registered by `weapons::WeaponPlugin`.
pub(crate) fn tick_resonance(
    mut commands: Commands,
    time: Res<Time>,
    mut resonant: Query<(Entity, &mut Resonance)>,
) {
    let delta = time.delta_secs();
    for (entity, mut resonance) in &mut resonant {
        resonance.remaining -= delta;
        if resonance.remaining <= 0.0 {
            commands.entity(entity).remove::<Resonance>();
        }
    }
}

pub(crate) fn fire_bellringer(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    spatial_query: &SpatialQuery,
    player_entity: Entity,
) {
    let Some((spawn_position, direction)) = aim_crossbow(
        camera_transform,
        player_transform,
        weapon,
        spatial_query,
        player_entity,
    ) else {
        return;
    };

    spawn_bolt(
        commands,
        meshes,
        materials,
        spawn_position,
        direction,
        CROSSBOW_BOLT_SPEED,
        weapon.damage * BELLRINGER_DAMAGE_MULTIPLIER,
        weapon,
        CROSSBOW_BOLT_LIFETIME,
        false,
        ProjectileKind::ResonantBolt,
        CROSSBOW_BOLT_RADIUS * 1.2,
        Color::srgb(0.85, 0.75, 0.95),
        player_entity,
    );

    println!("Resonant Bolt!");
}

// The Resonance shockwave itself (damage + stagger to every enemy near
// the enemy that just filled Resonance) is applied inline in
// `weapons::projectiles::movement`, the same way Runesage's Rune Blast
// and Gravemind's Ruin Bolt splash damage are -- see that file's
// ResonantBolt-specific block. `RESONANCE_SHOCKWAVE_*` above are the
// numbers it reads.
