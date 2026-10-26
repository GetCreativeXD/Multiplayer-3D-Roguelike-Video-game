use bevy::prelude::*;

use crate::combat::Health;
use crate::enemies::{Enemy, EnemyCombat, EnemyState, Stagger};
use crate::player::{AbilityMovement, Player, PlayerCharacter};

use crate::weapons::components::Weapon;

use crate::weapons::LONGSWORD_VISUAL_DURATION;
use crate::weapons::melee::spawn_melee_arc;

// =========================================================
// COMMON LONGSWORD
// =========================================================

const LONGSWORD_LUNGE_DISTANCE: f32 = 2.0;
const LONGSWORD_LUNGE_DAMAGE_MULTIPLIER: f32 = 1.4;

// =========================================================
// RARE LONGSWORD
// =========================================================

// Longsword - Crosscut
const LONGSWORD_CROSSCUT_DAMAGE_MULTIPLIER: f32 = 0.9;
const LONGSWORD_CROSSCUT_DASH_DISTANCE: f32 = 2.4;

// =========================================================
// EPIC LONGSWORD
// =========================================================

// Longsword - Blade Rush
const LONGSWORD_BLADE_RUSH_SPEED: f32 = 18.0;
const LONGSWORD_BLADE_RUSH_DURATION: f32 = 0.40;

const LONGSWORD_BLADE_RUSH_HIT_RADIUS: f32 = 1.25;

const LONGSWORD_BLADE_RUSH_DAMAGE_MULTIPLIER: f32 = 0.8;

const LONGSWORD_BLADE_RUSH_FINISH_MULTIPLIER: f32 = 1.6;
const LONGSWORD_BLADE_RUSH_FINISH_RANGE_MULTIPLIER: f32 = 1.35;

#[derive(Component)]
pub(crate) struct BladeRush {
    direction: Vec3,
    hit_enemies: Vec<Entity>,

    damage: f32,
    range: f32,
    knockback: f32,
    stagger_strength: u32,
    stagger_duration: f32,
}

pub(crate) fn longsword_lunging_slash(
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

    // Face the direction of the crosshair.
    player_transform.rotation = Quat::from_rotation_y(attack_direction.x.atan2(attack_direction.z));

    // Quick forward lunge.
    player_transform.translation += attack_direction * LONGSWORD_LUNGE_DISTANCE;

    // Visual slash.
    spawn_melee_arc(
        commands,
        meshes,
        materials,
        player_transform,
        attack_direction,
        weapon.range,
        LONGSWORD_VISUAL_DURATION,
    );

    println!("Lunging Slash!");

    for (mut enemy_transform, mut enemy_health, mut stagger, mut enemy_combat) in
        enemy_query.iter_mut()
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

        enemy_health.current -= weapon.damage * LONGSWORD_LUNGE_DAMAGE_MULTIPLIER;

        println!(
            "Lunging Slash damage applied: {:.1}",
            weapon.damage * LONGSWORD_LUNGE_DAMAGE_MULTIPLIER
        );

        enemy_health.current = enemy_health.current.max(0.0);

        if weapon.stagger_strength > stagger.resistance {
            stagger.timer = weapon.stagger_duration;

            enemy_combat.state = EnemyState::Chase;

            enemy_combat.timer = 0.0;
            enemy_combat.has_hit = false;
        }

        enemy_transform.translation += direction_to_enemy * weapon.knockback;

        println!(
            "Lunging Slash hit! HP: {:.0}/{:.0}",
            enemy_health.current, enemy_health.max
        );
    }
}

pub(crate) fn longsword_crosscut(
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

    // Dash forward.
    player_transform.translation += attack_direction * LONGSWORD_CROSSCUT_DASH_DISTANCE;

    // First slash.
    spawn_melee_arc(
        commands,
        meshes,
        materials,
        player_transform,
        attack_direction,
        weapon.range,
        0.14,
    );

    // Second slash visual.
    spawn_melee_arc(
        commands,
        meshes,
        materials,
        player_transform,
        attack_direction,
        weapon.range * 0.9,
        0.18,
    );

    println!("CROSSCUT!");

    for (mut enemy_transform, mut enemy_health, mut stagger, mut enemy_combat) in
        enemy_query.iter_mut()
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

        if facing_dot < 0.25 {
            continue;
        }

        // Two cuts.
        let total_damage = weapon.damage * LONGSWORD_CROSSCUT_DAMAGE_MULTIPLIER * 2.0;

        enemy_health.current -= total_damage;

        enemy_health.current = enemy_health.current.max(0.0);

        if weapon.stagger_strength > stagger.resistance {
            stagger.timer = weapon.stagger_duration;

            enemy_combat.state = EnemyState::Chase;

            enemy_combat.timer = 0.0;
            enemy_combat.has_hit = false;
        }

        enemy_transform.translation += direction_to_enemy * weapon.knockback;

        println!(
            "Crosscut hit! Damage {:.1}, HP {:.0}/{:.0}",
            total_damage, enemy_health.current, enemy_health.max
        );
    }
}

pub(crate) fn start_blade_rush(
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

    // Immediately face the rush direction.
    player_transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));

    commands.entity(player_entity).insert((
        AbilityMovement {
            direction,
            speed: LONGSWORD_BLADE_RUSH_SPEED,
            time_remaining: LONGSWORD_BLADE_RUSH_DURATION,
        },
        BladeRush {
            direction,
            hit_enemies: Vec::new(),

            damage: weapon.damage,
            range: weapon.range,
            knockback: weapon.knockback,
            stagger_strength: weapon.stagger_strength,
            stagger_duration: weapon.stagger_duration,
        },
    ));

    println!("BLADE RUSH!");
}

pub(crate) fn update_blade_rush(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    mut player_query: Query<
        (Entity, &Transform, &AbilityMovement, &mut BladeRush),
        (With<PlayerCharacter>, Without<Enemy>),
    >,

    mut enemy_query: Query<
        (
            Entity,
            &mut Transform,
            &mut Health,
            &mut Stagger,
            &mut EnemyCombat,
        ),
        (With<Enemy>, Without<Player>),
    >,
) {
    for (player_entity, player_transform, ability_movement, mut blade_rush) in &mut player_query {
        // =====================================================
        // RUSH-THROUGH DAMAGE
        // =====================================================

        if ability_movement.is_active() {
            for (
                enemy_entity,
                mut enemy_transform,
                mut enemy_health,
                mut stagger,
                mut enemy_combat,
            ) in enemy_query.iter_mut()
            {
                // Don't damage the same enemy repeatedly
                // while passing through it.
                if blade_rush.hit_enemies.contains(&enemy_entity) {
                    continue;
                }

                let mut to_enemy = enemy_transform.translation - player_transform.translation;

                to_enemy.y = 0.0;

                let distance = to_enemy.length();

                if distance > LONGSWORD_BLADE_RUSH_HIT_RADIUS {
                    continue;
                }

                let damage = blade_rush.damage * LONGSWORD_BLADE_RUSH_DAMAGE_MULTIPLIER;

                enemy_health.current -= damage;

                enemy_health.current = enemy_health.current.max(0.0);

                blade_rush.hit_enemies.push(enemy_entity);

                let direction_to_enemy = to_enemy.normalize_or_zero();

                if blade_rush.stagger_strength > stagger.resistance {
                    stagger.timer = blade_rush.stagger_duration;

                    enemy_combat.state = EnemyState::Chase;

                    enemy_combat.timer = 0.0;
                    enemy_combat.has_hit = false;
                }

                if direction_to_enemy != Vec3::ZERO {
                    enemy_transform.translation += direction_to_enemy * blade_rush.knockback * 0.5;
                }

                println!(
                    "Blade Rush passed through enemy! Damage {:.1}, HP {:.0}/{:.0}",
                    damage, enemy_health.current, enemy_health.max,
                );
            }

            continue;
        }

        // =====================================================
        // FINISHING SLASH
        // =====================================================

        let finish_range = blade_rush.range * LONGSWORD_BLADE_RUSH_FINISH_RANGE_MULTIPLIER;

        spawn_melee_arc(
            &mut commands,
            &mut meshes,
            &mut materials,
            player_transform,
            blade_rush.direction,
            finish_range,
            0.22,
        );

        println!("BLADE RUSH FINISH!");

        for (_enemy_entity, mut enemy_transform, mut enemy_health, mut stagger, mut enemy_combat) in
            enemy_query.iter_mut()
        {
            let mut to_enemy = enemy_transform.translation - player_transform.translation;

            to_enemy.y = 0.0;

            let distance = to_enemy.length();

            if distance > finish_range {
                continue;
            }

            let direction_to_enemy = to_enemy.normalize_or_zero();

            if direction_to_enemy == Vec3::ZERO {
                continue;
            }

            let facing_dot = blade_rush.direction.dot(direction_to_enemy);

            // Wide final slash.
            if facing_dot < 0.0 {
                continue;
            }

            let damage = blade_rush.damage * LONGSWORD_BLADE_RUSH_FINISH_MULTIPLIER;

            enemy_health.current -= damage;

            enemy_health.current = enemy_health.current.max(0.0);

            // The finishing hit has stronger interruption
            // than the rush-through hits.
            if blade_rush.stagger_strength + 1 > stagger.resistance {
                stagger.timer = blade_rush.stagger_duration * 1.25;

                enemy_combat.state = EnemyState::Chase;

                enemy_combat.timer = 0.0;
                enemy_combat.has_hit = false;
            }

            enemy_transform.translation += direction_to_enemy * blade_rush.knockback * 1.25;

            println!(
                "Blade Rush finish hit for {:.1}! HP {:.0}/{:.0}",
                damage, enemy_health.current, enemy_health.max,
            );
        }

        // Blade Rush is finished.
        commands.entity(player_entity).remove::<BladeRush>();

        commands.entity(player_entity).remove::<AbilityMovement>();
    }
}
