//! Hammer weapon abilities -- see WEAPONS.md's Hammer section for the
//! design source of truth. Hammer's identity is ground slams, shockwaves,
//! launching, and the roster's highest ordinary weapon stagger, so every
//! ability here shares one AOE helper (`apply_hammer_ground_impact`)
//! rather than each tier reinventing its own damage/stagger/knockback
//! loop.
//!
//! Implemented: Common (Ground Slam), Rare (Seismic Strike), Epic
//! (Earthshatter) -- the same Common/Rare/Epic depth Longsword, Greatsword,
//! and Bow already have. Legendary Hammers (Worldbreaker, Graviton Maul,
//! Stormanvil) are not itemized yet, matching every other weapon family's
//! current Legendary placeholder in `weapons::abilities::weapon_ability_input`.

use bevy::prelude::*;

use crate::visual_effects::{EffectShape, effect_mesh};

use crate::combat::{self, EnemyDebuffs, Health};
use crate::enemies::{Enemy, EnemyCombat, EnemyState, Stagger};
use crate::player::Player;

use crate::weapons::SEISMIC_FISSURE_SIZE;
use crate::weapons::components::{PlayerProjectile, Projectile, ProjectileKind, Weapon};

// =========================================================
// SHARED -- every Hammer ground-impact ability uses this
// =========================================================

/// Stagger-resistance cutoff for the bonus "launched" knockback below --
/// mirrors `characters::wildshaper_abilities::RHINO_CHARGE_LAUNCH_RESISTANCE_THRESHOLD`:
/// comfortably under elite resistance (~2, per the context doc's stagger
/// table), so only ordinary enemies get thrown the extra distance
/// WEAPONS.md calls "normal enemies can be launched" / "knocked into the
/// air." Elites and bosses still take the full stagger check below --
/// WEAPONS.md's "elites resist the knockback but still receive heavy
/// stagger" -- they just don't get flung.
const HAMMER_LAUNCH_RESISTANCE_THRESHOLD: u32 = 1;
const HAMMER_LAUNCH_KNOCKBACK_MULTIPLIER: f32 = 1.8;

/// How long a Hammer ground-impact burst (Ground Slam, each Earthshatter
/// pulse) stays visible before despawning.
const HAMMER_IMPACT_VISUAL_LIFETIME: f32 = 0.3;

/// Vertical offset from the player's `Transform` (capsule center) down to
/// just above the floor, used by every Hammer ground-level visual
/// (telegraph, impact bursts, unstable ground). -1.0 is the player
/// capsule's center-to-feet offset, the same constant
/// `characters::wildshaper_abilities`'s Bear Slam telegraph uses; +0.05
/// keeps the disk from z-fighting with the floor.
const HAMMER_GROUND_VISUAL_Y_OFFSET: f32 = 0.05 - 1.0;

#[derive(Component)]
pub(crate) struct HammerImpactVisual {
    lifetime: f32,
}

/// Applies Hammer's shared "circular ground impact" -- damage, then
/// stagger/interrupt, then knockback -- to every enemy within `radius` of
/// `center`. Used by Ground Slam, every Earthshatter pulse, and the
/// Unstable Ground collapse: the one AOE shape the whole weapon family
/// shares (see WEAPONS.md).
///
/// The stagger/interrupt check below is also what WEAPONS.md's "briefly
/// interrupts nearby enemy attacks" means in practice: any enemy mid-attack
/// caught in the radius has its combat state reset to `Chase`, exactly the
/// interrupt every other weapon in the project already triggers
/// (see `weapons::melee`).
fn apply_hammer_ground_impact(
    center: Vec3,
    radius: f32,
    damage: f32,
    stagger_strength: u32,
    stagger_duration: f32,
    knockback: f32,
    enemy_query: &mut Query<
        (
            &mut Transform,
            &mut Health,
            &mut Stagger,
            &mut EnemyCombat,
            Option<&EnemyDebuffs>,
        ),
        (With<Enemy>, Without<Player>),
    >,
) {
    for (mut enemy_transform, mut enemy_health, mut stagger, mut enemy_combat, enemy_debuffs) in
        enemy_query.iter_mut()
    {
        let mut to_enemy = enemy_transform.translation - center;
        to_enemy.y = 0.0;

        let distance = to_enemy.length();

        if distance > radius {
            continue;
        }

        combat::apply_enemy_damage(&mut enemy_health, enemy_debuffs, damage);

        let effective_resistance =
            combat::effective_stagger_resistance(stagger.resistance, enemy_debuffs);

        if stagger_strength > effective_resistance {
            stagger.timer = stagger_duration;

            enemy_combat.state = EnemyState::Chase;
            enemy_combat.timer = 0.0;
            enemy_combat.has_hit = false;
        }

        let direction_to_enemy = to_enemy.normalize_or_zero();

        if direction_to_enemy == Vec3::ZERO {
            continue;
        }

        // Ordinary enemies get thrown much further -- the closest thing to
        // "launched" Ruinbound's kinematic enemies currently support (see
        // `characters::wildshaper_abilities::RhinoCharge`'s doc comment for
        // why there's no real airborne state yet).
        let applied_knockback = if effective_resistance <= HAMMER_LAUNCH_RESISTANCE_THRESHOLD {
            knockback * HAMMER_LAUNCH_KNOCKBACK_MULTIPLIER
        } else {
            knockback
        };

        enemy_transform.translation += direction_to_enemy * applied_knockback;

        println!(
            "Hammer impact hit! Damage {:.1}, HP {:.0}/{:.0}",
            damage, enemy_health.current, enemy_health.max
        );
    }
}

/// Shared ground-impact burst visual for every Hammer ability that ends in
/// a circular slam -- same idea as `weapons::melee::spawn_melee_arc` being
/// shared across every bladed weapon's swing visual. Always sized to the
/// exact radius the AOE damage check just used, so the visible burst and
/// the gameplay hitbox never disagree (see Saint's Fist Slam shockwave and
/// Wildshaper's Bear Slam for the enemy-side and class-ability-side
/// versions of the same rule).
fn spawn_hammer_impact_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    center: Vec3,
    radius: f32,
) {
    commands.spawn((
        Name::new("Hammer Ground Impact"),
        HammerImpactVisual {
            lifetime: HAMMER_IMPACT_VISUAL_LIFETIME,
        },
        effect_mesh(
            &mut *meshes,
            EffectShape::Cylinder {
                radius,
                height: 0.12,
            },
        ),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.85, 0.45, 0.15, 0.5),
            emissive: LinearRgba::new(0.9, 0.4, 0.08, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_translation(center + Vec3::Y * HAMMER_GROUND_VISUAL_Y_OFFSET),
    ));
}

pub(crate) fn update_hammer_impact_visuals(
    mut commands: Commands,
    time: Res<Time>,
    mut visuals: Query<(Entity, &mut HammerImpactVisual)>,
) {
    let delta = time.delta_secs();

    for (entity, mut visual) in &mut visuals {
        visual.lifetime -= delta;

        if visual.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

// =========================================================
// COMMON HAMMER (Stonefall) -- Ground Slam
//
// The player raises the hammer and slams the ground in a short-range
// circular impact. A brief windup telegraphs the exact impact circle,
// matching the project's "every dangerous... attack needs a clear visual
// telegraph" rule and the same radius-must-equal-hitbox rule Saint's Fist
// Slam and Wildshaper's Bear Slam already follow -- even though this is
// the player's own outgoing attack rather than an enemy one, reusing that
// established telegraph shape keeps every ground-slam in the project
// reading the same way.
// =========================================================

const GROUND_SLAM_RADIUS: f32 = 4.4;
const GROUND_SLAM_DAMAGE_MULTIPLIER: f32 = 1.15;
const GROUND_SLAM_KNOCKBACK_MULTIPLIER: f32 = 1.4;
/// Two points above the weapon's own already-high `stagger_strength` --
/// Ground Slam is the roster's highest ordinary weapon stagger (WEAPONS.md).
const GROUND_SLAM_STAGGER_BONUS: u32 = 2;

#[derive(Component)]
pub(crate) struct GroundSlamTelegraph {
    lifetime: f32,
    total_lifetime: f32,
}

/// Spawns the pulsing warning disk shown during the Common Hammer ability's
/// windup. Called from `weapons::abilities::weapon_ability_input` the
/// instant the ability is cast; `windup` is `HAMMER_GROUND_SLAM_WINDUP`,
/// passed in rather than duplicated here so the windup timing lives in one
/// place (the ability dispatcher, next to `combat.ability_timer`).
pub(crate) fn spawn_ground_slam_telegraph(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    player_transform: &Transform,
    windup: f32,
) {
    commands.spawn((
        Name::new("Ground Slam Telegraph"),
        GroundSlamTelegraph {
            lifetime: windup,
            total_lifetime: windup,
        },
        effect_mesh(
            &mut *meshes,
            EffectShape::Cylinder {
                radius: GROUND_SLAM_RADIUS,
                height: 0.035,
            },
        ),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.85, 0.45, 0.15, 0.28),
            emissive: LinearRgba::new(0.9, 0.4, 0.08, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_translation(
            player_transform.translation + Vec3::Y * HAMMER_GROUND_VISUAL_Y_OFFSET,
        ),
    ));

    println!("Hammer winds up for Ground Slam!");
}

pub(crate) fn update_ground_slam_telegraphs(
    mut commands: Commands,
    time: Res<Time>,
    mut telegraphs: Query<(Entity, &mut GroundSlamTelegraph, &mut Transform)>,
) {
    for (entity, mut telegraph, mut transform) in &mut telegraphs {
        telegraph.lifetime -= time.delta_secs();

        if telegraph.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = 1.0 - telegraph.lifetime / telegraph.total_lifetime;
        let pulse = 0.94 + (progress * std::f32::consts::TAU * 3.0).sin().abs() * 0.08;
        transform.scale = Vec3::new(pulse, 1.0, pulse);
    }
}

/// Resolves the Ground Slam impact once its windup finishes. Called from
/// `weapons::melee::update_player_combat` the same way Greatsword's
/// `charged_cleave` already is (see `combat.pending_ability`).
pub(crate) fn hammer_ground_slam(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    player_transform: &Transform,
    weapon: &Weapon,
    enemy_query: &mut Query<
        (
            &mut Transform,
            &mut Health,
            &mut Stagger,
            &mut EnemyCombat,
            Option<&EnemyDebuffs>,
        ),
        (With<Enemy>, Without<Player>),
    >,
) {
    let center = player_transform.translation;
    let damage = weapon.damage * GROUND_SLAM_DAMAGE_MULTIPLIER;
    let stagger_strength = weapon.stagger_strength + GROUND_SLAM_STAGGER_BONUS;
    let knockback = weapon.knockback * GROUND_SLAM_KNOCKBACK_MULTIPLIER;

    println!("GROUND SLAM!");

    apply_hammer_ground_impact(
        center,
        GROUND_SLAM_RADIUS,
        damage,
        stagger_strength,
        weapon.stagger_duration * 1.2,
        knockback,
        enemy_query,
    );

    spawn_hammer_impact_visual(commands, meshes, materials, center, GROUND_SLAM_RADIUS);
}

// =========================================================
// RARE HAMMER (Faultline) -- Seismic Strike
//
// Smashes the hammer into the ground, sending a fissure forward in a
// straight line. Reuses the exact piercing-projectile pipeline every other
// wide-line ability already shares (Greatsword's Phantom Cleave, Warden's
// Shadow Stag, Runesage's Runic Surge) -- `move_projectiles_with_collision`
// already stops any piercing projectile the instant it hits world
// geometry, which is exactly WEAPONS.md's "stops when it reaches solid
// dungeon geometry."
// =========================================================

const SEISMIC_STRIKE_SPEED: f32 = 15.0;
/// Travel distance = speed * lifetime ~= 11.25 units -- comfortably past
/// Ground Slam's 4.4-unit radius, matching "longer range than Ground Slam."
const SEISMIC_STRIKE_LIFETIME: f32 = 0.75;
const SEISMIC_STRIKE_DAMAGE_MULTIPLIER: f32 = 0.95;
const SEISMIC_STRIKE_KNOCKBACK_MULTIPLIER: f32 = 1.7;
/// Two points above the weapon's base stagger_strength, same bonus Ground
/// Slam uses -- "maximum stagger near the impact point."
const SEISMIC_STRIKE_STAGGER_BONUS: u32 = 2;
const SEISMIC_STRIKE_SPAWN_OFFSET: f32 = 1.0;

pub(crate) fn hammer_seismic_strike(
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

    let spawn_position = player_transform.translation
        + direction * SEISMIC_STRIKE_SPAWN_OFFSET
        + Vec3::Y * (SEISMIC_FISSURE_SIZE.y * 0.5 + HAMMER_GROUND_VISUAL_Y_OFFSET);

    println!("SEISMIC STRIKE!");

    commands.spawn((
        PlayerProjectile,
        Projectile {
            velocity: direction * SEISMIC_STRIKE_SPEED,

            damage: weapon.damage * SEISMIC_STRIKE_DAMAGE_MULTIPLIER,

            knockback: weapon.knockback * SEISMIC_STRIKE_KNOCKBACK_MULTIPLIER,

            stagger_strength: weapon.stagger_strength + SEISMIC_STRIKE_STAGGER_BONUS,

            stagger_duration: weapon.stagger_duration * 1.1,

            lifetime: SEISMIC_STRIKE_LIFETIME,

            // Piercing -- "the fissure can hit multiple enemies."
            piercing: true,

            hit_entities: Vec::new(),

            kind: ProjectileKind::SeismicFissure,

            target: None,
        },
        // Low, wide crack -- sized identically to the collider
        // `move_projectiles_with_collision` gives `ProjectileKind::SeismicFissure`
        // (both read `SEISMIC_FISSURE_SIZE`), so the visible fissure and its
        // actual hitbox can never drift apart.
        Mesh3d(meshes.add(Cuboid::new(
            SEISMIC_FISSURE_SIZE.x,
            SEISMIC_FISSURE_SIZE.y,
            SEISMIC_FISSURE_SIZE.z,
        ))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.8, 0.4, 0.12, 0.85),
            emissive: LinearRgba::new(1.2, 0.4, 0.05, 1.0),
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
}

// =========================================================
// EPIC HAMMER (Earthshatter) -- Earthshatter
//
// A massive overhead slam creates a large impact zone followed by two
// smaller cracking aftershocks, then leaves unstable ground that collapses
// in a second damaging burst. Every pulse reuses `apply_hammer_ground_impact`
// and `spawn_hammer_impact_visual` above -- Earthshatter is Ground Slam's
// numbers repeated on a timer, not a separate system.
//
// NOTE: WEAPONS.md also calls for the unstable ground to "briefly slow
// enemies" while it's active. Ruinbound doesn't have a generic
// movement-speed debuff yet (`combat::EnemyDebuffs` only carries damage-
// taken and stagger-resistance modifiers today -- see that file's doc
// comment), and wiring one in would mean touching every enemy archetype's
// movement code, which is a larger cross-cutting change than this pass
// should make unprompted. The impact/damage/collapse side of Earthshatter
// is fully implemented below; the slow is deferred, the same way armor
// `GearEffect`s are itemized before their numbers are wired up.
// =========================================================

const EARTHSHATTER_IMPACT_RADIUS: f32 = 6.0;
const EARTHSHATTER_AFTERSHOCK_RADIUS: f32 = 4.5;
const EARTHSHATTER_IMPACT_DAMAGE_MULTIPLIER: f32 = 1.35;
const EARTHSHATTER_AFTERSHOCK_DAMAGE_MULTIPLIER: f32 = 0.55;
const EARTHSHATTER_KNOCKBACK_MULTIPLIER: f32 = 1.6;
/// Three points above the weapon's base stagger_strength (3 + 3 = 6) --
/// exactly the Saint's stagger resistance, so even Earthshatter can't
/// interrupt a boss outright ("cannot be permanently stun-locked"), while
/// comfortably clearing the ~2 resistance future elites are expected to
/// use ("elites are heavily staggered").
const EARTHSHATTER_STAGGER_BONUS: u32 = 3;
const EARTHSHATTER_AFTERSHOCK_INTERVAL: f32 = 0.35;

const EARTHSHATTER_PULSE_COUNT: usize = 3;
const EARTHSHATTER_PULSE_RADII: [f32; EARTHSHATTER_PULSE_COUNT] = [
    EARTHSHATTER_IMPACT_RADIUS,
    EARTHSHATTER_AFTERSHOCK_RADIUS,
    EARTHSHATTER_AFTERSHOCK_RADIUS,
];
const EARTHSHATTER_PULSE_DAMAGE_MULTIPLIERS: [f32; EARTHSHATTER_PULSE_COUNT] = [
    EARTHSHATTER_IMPACT_DAMAGE_MULTIPLIER,
    EARTHSHATTER_AFTERSHOCK_DAMAGE_MULTIPLIER,
    EARTHSHATTER_AFTERSHOCK_DAMAGE_MULTIPLIER,
];

const UNSTABLE_GROUND_RADIUS: f32 = 4.5;
const UNSTABLE_GROUND_DURATION: f32 = 2.0;
const UNSTABLE_GROUND_COLLAPSE_DAMAGE_MULTIPLIER: f32 = 0.7;

/// Drives Earthshatter's impact plus its two follow-up aftershocks. A
/// free-standing entity (not attached to the player) so the sequence keeps
/// ticking regardless of what the player does next, the same way Saint's
/// `SaintShockwave` outlives the frame that spawned it.
#[derive(Component)]
pub(crate) struct EarthshatterSequence {
    center: Vec3,
    base_damage: f32,
    stagger_strength: u32,
    stagger_duration: f32,
    knockback: f32,
    pulse_index: usize,
    timer: f32,
}

/// The lingering cracked ground Earthshatter leaves behind. Purely a
/// damage/visual timer today -- see the module doc comment above about the
/// deferred slow effect.
#[derive(Component)]
pub(crate) struct HammerUnstableGround {
    center: Vec3,
    lifetime: f32,
    total_lifetime: f32,
    damage: f32,
    stagger_strength: u32,
    stagger_duration: f32,
}

pub(crate) fn hammer_earthshatter(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    player_transform: &Transform,
    weapon: &Weapon,
) {
    let center = player_transform.translation;

    println!("EARTHSHATTER!");

    // Pulse 0 fires on the very next tick of `update_earthshatter_aftershocks`
    // (timer starts at 0.0), reading as an instant impact -- the same
    // "instant on cast" shape every other Epic weapon ability already uses
    // (Blade Rush, Phantom Cleave, Ricochet Arrow).
    commands.spawn(EarthshatterSequence {
        center,
        base_damage: weapon.damage,
        stagger_strength: weapon.stagger_strength + EARTHSHATTER_STAGGER_BONUS,
        stagger_duration: weapon.stagger_duration * 1.3,
        knockback: weapon.knockback * EARTHSHATTER_KNOCKBACK_MULTIPLIER,
        pulse_index: 0,
        timer: 0.0,
    });

    commands.spawn((
        Name::new("Hammer Unstable Ground"),
        HammerUnstableGround {
            center,
            lifetime: UNSTABLE_GROUND_DURATION,
            total_lifetime: UNSTABLE_GROUND_DURATION,
            damage: weapon.damage * UNSTABLE_GROUND_COLLAPSE_DAMAGE_MULTIPLIER,
            stagger_strength: weapon.stagger_strength + 1,
            stagger_duration: weapon.stagger_duration,
        },
        effect_mesh(
            &mut *meshes,
            EffectShape::Cylinder {
                radius: UNSTABLE_GROUND_RADIUS,
                height: 0.05,
            },
        ),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.55, 0.3, 0.12, 0.35),
            emissive: LinearRgba::new(0.5, 0.2, 0.04, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_translation(center + Vec3::Y * HAMMER_GROUND_VISUAL_Y_OFFSET),
    ));
}

pub(crate) fn update_earthshatter_aftershocks(
    mut commands: Commands,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sequences: Query<(Entity, &mut EarthshatterSequence)>,
    mut enemy_query: Query<
        (
            &mut Transform,
            &mut Health,
            &mut Stagger,
            &mut EnemyCombat,
            Option<&EnemyDebuffs>,
        ),
        (With<Enemy>, Without<Player>),
    >,
) {
    let delta = time.delta_secs();

    for (entity, mut sequence) in &mut sequences {
        sequence.timer -= delta;

        if sequence.timer > 0.0 {
            continue;
        }

        let radius = EARTHSHATTER_PULSE_RADII[sequence.pulse_index];
        let damage =
            sequence.base_damage * EARTHSHATTER_PULSE_DAMAGE_MULTIPLIERS[sequence.pulse_index];

        apply_hammer_ground_impact(
            sequence.center,
            radius,
            damage,
            sequence.stagger_strength,
            sequence.stagger_duration,
            sequence.knockback,
            &mut enemy_query,
        );

        spawn_hammer_impact_visual(
            &mut commands,
            &mut meshes,
            &mut materials,
            sequence.center,
            radius,
        );

        sequence.pulse_index += 1;

        if sequence.pulse_index >= EARTHSHATTER_PULSE_COUNT {
            commands.entity(entity).despawn();
        } else {
            sequence.timer = EARTHSHATTER_AFTERSHOCK_INTERVAL;
        }
    }
}

pub(crate) fn update_unstable_ground(
    mut commands: Commands,
    time: Res<Time>,
    mut grounds: Query<(Entity, &mut HammerUnstableGround, &mut Transform), Without<Enemy>>,
    mut enemy_query: Query<
        (
            &mut Transform,
            &mut Health,
            &mut Stagger,
            &mut EnemyCombat,
            Option<&EnemyDebuffs>,
        ),
        (With<Enemy>, Without<Player>),
    >,
) {
    let delta = time.delta_secs();

    for (entity, mut ground, mut transform) in &mut grounds {
        ground.lifetime -= delta;

        if ground.lifetime > 0.0 {
            let progress = 1.0 - ground.lifetime / ground.total_lifetime;
            let pulse = 0.95 + (progress * std::f32::consts::TAU * 2.0).sin().abs() * 0.05;
            transform.scale = Vec3::new(pulse, 1.0, pulse);
            continue;
        }

        // Collapse -- one final damaging burst at the epicenter, matching
        // WEAPONS.md's "the unstable ground collapses in a second damaging
        // burst." No knockback here; the collapse damages and staggers
        // without shoving anything further.
        apply_hammer_ground_impact(
            ground.center,
            UNSTABLE_GROUND_RADIUS,
            ground.damage,
            ground.stagger_strength,
            ground.stagger_duration,
            0.0,
            &mut enemy_query,
        );

        println!("Unstable ground collapses!");

        commands.entity(entity).despawn();
    }
}
