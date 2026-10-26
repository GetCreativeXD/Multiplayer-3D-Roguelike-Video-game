use bevy::prelude::*;

use crate::combat::{self, EnemyDebuffs, Health};
use crate::enemies::{Enemy, EnemyCombat, EnemyState, Stagger};
use crate::player::Player;

use crate::weapons::components::{PlayerProjectile, Projectile, ProjectileKind, Weapon};

use crate::weapons::melee::spawn_melee_arc;

// =========================================================
// COMMON GREAT SWORD
// =========================================================

const GREATSWORD_CLEAVE_DAMAGE_MULTIPLIER: f32 = 2.0;

// =========================================================
// RARE GREAT SWORD
// =========================================================

// Greatsword - Executioner's Sweep

const GREATSWORD_EXECUTION_DAMAGE_MULTIPLIER: f32 = 1.5;
const GREATSWORD_EXECUTION_LOW_HEALTH_MULTIPLIER: f32 = 2.0;
const GREATSWORD_EXECUTION_THRESHOLD: f32 = 0.35;

// =========================================================
// EPIC GREAT SWORD
// =========================================================

// Greatsword - Phantom Cleave

const GREATSWORD_PHANTOM_CLEAVE_SPEED: f32 = 14.0;
const GREATSWORD_PHANTOM_CLEAVE_LIFETIME: f32 = 1.5;
const GREATSWORD_PHANTOM_CLEAVE_DAMAGE_MULTIPLIER: f32 = 1.8;
const GREATSWORD_PHANTOM_CLEAVE_KNOCKBACK_MULTIPLIER: f32 = 1.25;

pub(crate) fn charged_cleave(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
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
    let mut attack_direction = camera_transform.forward().as_vec3();

    attack_direction.y = 0.0;

    attack_direction = attack_direction.normalize_or_zero();

    if attack_direction == Vec3::ZERO {
        return;
    }

    player_transform.rotation = Quat::from_rotation_y(attack_direction.x.atan2(attack_direction.z));

    spawn_melee_arc(
        commands,
        meshes,
        materials,
        player_transform,
        attack_direction,
        weapon.range * 1.25,
        0.25,
    );

    println!("CHARGED CLEAVE!");

    for (mut enemy_transform, mut enemy_health, mut stagger, mut enemy_combat, enemy_debuffs) in
        enemy_query.iter_mut()
    {
        let mut to_enemy = enemy_transform.translation - player_transform.translation;

        to_enemy.y = 0.0;

        let distance = to_enemy.length();

        if distance > weapon.range * 1.25 {
            continue;
        }

        let direction_to_enemy = to_enemy.normalize_or_zero();

        if direction_to_enemy == Vec3::ZERO {
            continue;
        }

        let facing_dot = attack_direction.dot(direction_to_enemy);

        // Very wide cleave.
        if facing_dot < 0.05 {
            continue;
        }

        // Damage -- Runebroken (see combat::enemy_debuffs) multiplies
        // incoming damage when the target has it active.
        combat::apply_enemy_damage(
            &mut enemy_health,
            enemy_debuffs,
            weapon.damage * GREATSWORD_CLEAVE_DAMAGE_MULTIPLIER,
        );

        println!(
            "Charged Cleave damage applied: {:.1}",
            weapon.damage * GREATSWORD_CLEAVE_DAMAGE_MULTIPLIER
        );

        // Stronger stagger than a basic Greatsword swing.
        if weapon.stagger_strength + 1
            > combat::effective_stagger_resistance(stagger.resistance, enemy_debuffs)
        {
            stagger.timer = weapon.stagger_duration * 1.5;

            enemy_combat.state = EnemyState::Chase;

            enemy_combat.timer = 0.0;
            enemy_combat.has_hit = false;
        }

        enemy_transform.translation += direction_to_enemy * weapon.knockback * 1.5;

        println!(
            "Charged Cleave hit! HP: {:.0}/{:.0}",
            enemy_health.current, enemy_health.max
        );
    }
}

pub(crate) fn greatsword_executioners_sweep(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    enemy_query: &mut Query<
        (&mut Transform, &mut Health, &mut Stagger, &mut EnemyCombat),
        (With<Enemy>, Without<Player>),
    >,
) {
    let mut attack_direction = camera_transform.forward().as_vec3();

    attack_direction.y = 0.0;
    attack_direction = attack_direction.normalize_or_zero();

    if attack_direction == Vec3::ZERO {
        return;
    }

    player_transform.rotation = Quat::from_rotation_y(attack_direction.x.atan2(attack_direction.z));

    spawn_melee_arc(
        commands,
        meshes,
        materials,
        player_transform,
        attack_direction,
        weapon.range * 1.2,
        0.22,
    );

    println!("EXECUTIONER'S SWEEP!");

    for (mut enemy_transform, mut enemy_health, mut stagger, mut enemy_combat) in
        enemy_query.iter_mut()
    {
        let mut to_enemy = enemy_transform.translation - player_transform.translation;

        to_enemy.y = 0.0;

        let distance = to_enemy.length();

        if distance > weapon.range * 1.2 {
            continue;
        }

        let direction_to_enemy = to_enemy.normalize_or_zero();

        if direction_to_enemy == Vec3::ZERO {
            continue;
        }

        let facing_dot = attack_direction.dot(direction_to_enemy);

        // Very wide horizontal sweep.
        if facing_dot < 0.0 {
            continue;
        }

        let health_ratio = if enemy_health.max > 0.0 {
            enemy_health.current / enemy_health.max
        } else {
            0.0
        };

        let mut damage = weapon.damage * GREATSWORD_EXECUTION_DAMAGE_MULTIPLIER;

        // Bonus execution damage below 35% health.
        if health_ratio <= GREATSWORD_EXECUTION_THRESHOLD {
            damage *= GREATSWORD_EXECUTION_LOW_HEALTH_MULTIPLIER;

            println!("EXECUTION BONUS!");
        }

        enemy_health.current -= damage;

        enemy_health.current = enemy_health.current.max(0.0);

        if weapon.stagger_strength + 1 > stagger.resistance {
            stagger.timer = weapon.stagger_duration * 1.25;

            enemy_combat.state = EnemyState::Chase;

            enemy_combat.timer = 0.0;
            enemy_combat.has_hit = false;
        }

        enemy_transform.translation += direction_to_enemy * weapon.knockback * 1.25;

        println!(
            "Executioner's Sweep hit for {:.1}! HP {:.0}/{:.0}",
            damage, enemy_health.current, enemy_health.max
        );
    }
}

pub(crate) fn fire_phantom_cleave(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
) {
    let direction = camera_transform.forward().as_vec3().normalize_or_zero();

    if direction == Vec3::ZERO {
        return;
    }

    // Face the direction of the attack.
    player_transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));

    // Spawn slightly in front of the player.
    let spawn_position = player_transform.translation + direction * 1.6 + Vec3::Y * 0.85;

    let damage = weapon.damage * GREATSWORD_PHANTOM_CLEAVE_DAMAGE_MULTIPLIER;

    commands.spawn((
        PlayerProjectile,
        Projectile {
            velocity: direction * GREATSWORD_PHANTOM_CLEAVE_SPEED,

            damage,

            knockback: weapon.knockback * GREATSWORD_PHANTOM_CLEAVE_KNOCKBACK_MULTIPLIER,

            // Give Phantom Cleave stronger stagger pressure
            // than a normal Greatsword attack.
            stagger_strength: weapon.stagger_strength + 1,

            stagger_duration: weapon.stagger_duration * 1.25,

            lifetime: GREATSWORD_PHANTOM_CLEAVE_LIFETIME,

            // Phantom Cleave passes through enemies.
            piercing: true,

            hit_entities: Vec::new(),

            kind: ProjectileKind::PhantomCleave,

            target: None,
        },
        // Temporary spectral blade visual.
        Mesh3d(meshes.add(Cuboid::new(3.6, 0.18, 0.55))),
        MeshMaterial3d(materials.add(Color::srgb(0.55, 0.25, 1.0))),
        Transform::from_translation(spawn_position),
    ));

    println!("PHANTOM CLEAVE!");
}
