use avian3d::prelude::*;
use bevy::prelude::*;

use crate::characters::class_abilities::{
    BasicAttackHitTracker, BasicAttackTracker, CharacterCombatModifiers,
};
use crate::combat::{self, DamageOverTime, EnemyDebuffs, FrontalGuard, Health, StatBuffs};
use crate::enemies::{Enemy, EnemyCombat, EnemyState, Stagger};
use crate::player::{Player, PlayerCharacter, PlayerIntent};

use super::bow;
use super::{crossbow, daggers, spellbook};

use super::abilities::{charged_cleave, hammer_ground_slam};

use super::components::{AttackVisual, PlayerCombat, Weapon, WeaponAbilityState, WeaponType};

use super::{
    GREATSWORD_VISUAL_DURATION, GREATSWORD_WINDUP, HAMMER_ATTACK_ANGLE_DOT,
    HAMMER_ATTACK_FORWARD_STEP, HAMMER_VISUAL_DURATION, HAMMER_WINDUP, LONGSWORD_VISUAL_DURATION,
};

/// Weapon basic attacks for every player character: this machine's
/// `Player` (left click, read into its `PlayerIntent` by
/// `player::read_local_input`) and, on a multiplayer host, each joined
/// player's body (their client's clicks, via `net::intent`). Every attack
/// aims with that player's own camera (`PlayerIntent::aim_transform`), so
/// the same code serves all six weapon types for everyone.
pub(crate) fn player_basic_attack(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    // Avian spatial queries for crosshair aiming.
    spatial_query: SpatialQuery,

    // Read-only enemy position lookup Spellbook's soft-lock targeting
    // needs (`spellbook::aim_spellbook` / `find_spellbook_target`).
    // Deliberately reads `GlobalTransform` rather than `Transform` --
    // a different component from the `&mut Transform` `enemy_query`
    // below already holds on the same `Enemy` archetype, so the two
    // queries never conflict even though both match the same enemies.
    enemies: Query<(Entity, &GlobalTransform), With<Enemy>>,

    mut player_query: Query<
        (
            Entity,
            &mut Transform,
            &mut PlayerIntent,
            &mut PlayerCombat,
            &Weapon,
            Option<&mut BasicAttackTracker>,
            Option<&mut BasicAttackHitTracker>,
            Option<&CharacterCombatModifiers>,
            Option<&StatBuffs>,
            Option<&mut WeaponAbilityState>,
        ),
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
            Option<&FrontalGuard>,
            Option<&DamageOverTime>,
            Option<&daggers::Deathmark>,
        ),
        (With<Enemy>, Without<Player>),
    >,
) {
    for (
        player_entity,
        mut player_transform,
        mut intent,
        mut combat,
        weapon,
        mut basic_attacks,
        mut basic_attack_hits,
        modifiers,
        stat_buffs,
        mut weapon_ability_state,
    ) in &mut player_query
    {
        // A press (left click with the cursor captured, for the local
        // player) is consumed even when it can't start an attack, as
        // `just_pressed` was before.
        if !intent.take_attack() {
            continue;
        }

        // Can't begin another attack while one is already pending.
        if combat.pending_attack.is_some() {
            continue;
        }

        // Attack is still on cooldown.
        if combat.attack_cooldown > 0.0 {
            continue;
        }

        // This player's camera: the weapons below aim from its position
        // through the centre of its view, exactly as they did with the
        // local camera.
        let aim = intent.aim_transform();
        let camera_transform = &aim;

        // Spellbook doesn't fire at all -- no projectile, no cooldown spent,
        // no attack recorded -- unless there's actually something within
        // range to cast at. Every other weapon has no such gate: a whiffed
        // melee swing or an arrow into empty space is still a normal,
        // cooldown-consuming attack, but a Spellbook cast with nothing in
        // range would just be a wasted button press.
        if weapon.weapon_type == WeaponType::Spellbook
            && !spellbook::any_enemy_in_range(player_transform.translation, weapon.range, &enemies)
        {
            continue;
        }

        if let Some(tracker) = basic_attacks.as_mut() {
            tracker.record_started_attack();
        }

        let attack_cooldown = weapon.cooldown / modifiers.map_or(1.0, |value| value.attack_speed);

        // Class-innate modifiers and externally granted buffs (e.g. Warden of
        // Shades' Guardian Otter, see `combat::StatBuffs`) both scale outgoing
        // weapon basic-attack damage.
        let damage_multiplier = stat_buffs.map_or(1.0, StatBuffs::damage_multiplier);

        match weapon.weapon_type {
            WeaponType::Longsword => {
                let hit_enemy = melee_attack(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform.forward().as_vec3(),
                    &mut player_transform,
                    weapon,
                    damage_multiplier,
                    &mut enemy_query,
                );
                if hit_enemy {
                    if let Some(tracker) = basic_attack_hits.as_mut() {
                        tracker.record_confirmed_hit();
                    }
                }
                combat.attack_cooldown = attack_cooldown;
            }

            WeaponType::Daggers => {
                let hit_enemy = daggers::daggers_basic_attack(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    damage_multiplier,
                    weapon_ability_state.as_deref_mut(),
                    &mut enemy_query,
                );
                if hit_enemy {
                    if let Some(tracker) = basic_attack_hits.as_mut() {
                        tracker.record_confirmed_hit();
                    }
                }
                combat.attack_cooldown = attack_cooldown;
            }

            WeaponType::Greatsword => {
                combat.attack_timer = GREATSWORD_WINDUP;
                combat.pending_attack = Some(WeaponType::Greatsword);
                combat.attack_cooldown = attack_cooldown;

                println!("Greatsword windup!");
            }

            WeaponType::Hammer => {
                combat.attack_timer = HAMMER_WINDUP;
                combat.pending_attack = Some(WeaponType::Hammer);
                combat.attack_cooldown = attack_cooldown;

                println!("Hammer windup!");
            }

            WeaponType::Bow => {
                bow::fire_bow(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    damage_multiplier,
                    &spatial_query,
                    player_entity,
                );

                combat.attack_cooldown = attack_cooldown;
            }

            WeaponType::Crossbow => {
                crossbow::fire_crossbow(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    damage_multiplier,
                    &spatial_query,
                    player_entity,
                );

                combat.attack_cooldown = attack_cooldown;
            }

            WeaponType::Spellbook => {
                spellbook::fire_arcane_bolt(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    damage_multiplier,
                    &spatial_query,
                    player_entity,
                    &enemies,
                );

                combat.attack_cooldown = attack_cooldown;
            }
        }
    }
}

/// `aim_direction` is where the attacker is aiming (for the local player,
/// the camera's forward); only its horizontal part is used.
fn melee_attack(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    aim_direction: Vec3,
    player_transform: &mut Transform,
    weapon: &Weapon,
    damage_multiplier: f32,
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
            Option<&daggers::Deathmark>,
        ),
        (With<Enemy>, Without<Player>),
    >,
) -> bool {
    let mut attack_direction = aim_direction;

    attack_direction.y = 0.0;
    attack_direction = attack_direction.normalize_or_zero();

    if attack_direction == Vec3::ZERO {
        return false;
    }

    let target_rotation = Quat::from_rotation_y(attack_direction.x.atan2(attack_direction.z));

    player_transform.rotation = target_rotation;

    spawn_melee_arc(
        commands,
        meshes,
        materials,
        player_transform,
        attack_direction,
        weapon.range,
        LONGSWORD_VISUAL_DURATION,
    );

    println!("Player attacks with {:?}", weapon.weapon_type);

    let mut hit_enemy = false;

    for (
        _entity,
        mut enemy_transform,
        mut enemy_health,
        mut stagger,
        mut enemy_combat,
        enemy_debuffs,
        frontal_guard,
        _dot,
        _deathmark,
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

        // Damage -- Runebroken (see combat::enemy_debuffs) multiplies
        // incoming damage when the target has it active, and a
        // FrontalGuard (see combat::frontal_guard -- today only the Black
        // Banner Knight) can block or take bonus rear damage.
        let guarded = combat::apply_guarded_enemy_damage(
            &mut enemy_health,
            enemy_debuffs,
            frontal_guard,
            enemy_transform.rotation,
            enemy_transform.translation,
            player_transform.translation,
            weapon.stagger_strength,
            weapon.damage * damage_multiplier,
        );

        println!(
            "Enemy hit! HP: {:.0}/{:.0}",
            enemy_health.current, enemy_health.max
        );

        // Stagger / interrupt -- a blocked hit doesn't interrupt.
        if !guarded.stagger_negated
            && weapon.stagger_strength
                > combat::effective_stagger_resistance(stagger.resistance, enemy_debuffs)
        {
            stagger.timer = weapon.stagger_duration;

            enemy_combat.state = EnemyState::Chase;
            enemy_combat.timer = 0.0;
            enemy_combat.has_hit = false;

            println!("Enemy interrupted!");
        }

        // Knockback
        enemy_transform.translation += direction_to_enemy * weapon.knockback;
    }

    hit_enemy
}

/// Fired every time a melee swing arc is drawn. Purely presentational: in
/// multiplayer the host forwards it to clients so they draw the same arc
/// (`net::effects`). Nothing observes it offline.
#[derive(Event, Clone, Copy, Debug)]
// Only read by the multiplayer build.
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
pub struct MeleeArcSpawned {
    pub origin: Vec3,
    pub direction: Vec3,
    pub radius: f32,
    pub duration: f32,
}

pub fn spawn_melee_arc(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    player_transform: &Transform,
    attack_direction: Vec3,
    radius: f32,
    duration: f32,
) {
    commands.trigger(MeleeArcSpawned {
        origin: player_transform.translation,
        direction: attack_direction,
        radius,
        duration,
    });

    let position =
        player_transform.translation + Vec3::Y * 0.7 + attack_direction * (radius * 0.55);

    let rotation = Quat::from_rotation_y(attack_direction.x.atan2(attack_direction.z));

    commands.spawn((
        AttackVisual { lifetime: duration },
        Mesh3d(meshes.add(Torus::new(radius * 0.55, radius * 0.75))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.85, 0.35, 0.65),

            alpha_mode: AlphaMode::Blend,

            unlit: true,

            ..default()
        })),
        Transform {
            translation: position,
            rotation: rotation * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            scale: Vec3::new(1.0, 0.25, 1.0),
        },
    ));
}

pub(crate) fn update_attack_visuals(
    mut commands: Commands,
    time: Res<Time>,
    mut visual_query: Query<(Entity, &mut AttackVisual)>,
) {
    for (entity, mut visual) in &mut visual_query {
        visual.lifetime -= time.delta_secs();

        if visual.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Ticks every player character's attack/ability timers and resolves
/// pending Greatsword/Hammer swings and abilities, aimed with that
/// player's own camera (`PlayerIntent::aim_transform`, as in
/// `player_basic_attack`).
pub(crate) fn update_player_combat(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,

    mut player_query: Query<
        (
            &mut Transform,
            &PlayerIntent,
            &mut PlayerCombat,
            &Weapon,
            Option<&mut BasicAttackHitTracker>,
            Option<&StatBuffs>,
        ),
        (With<PlayerCharacter>, Without<Enemy>),
    >,

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
    for (mut player_transform, intent, mut combat, weapon, mut basic_attack_hits, stat_buffs) in
        &mut player_query
    {
        let aim = intent.aim_transform();
        let camera_transform = &aim;

        let delta = time.delta_secs();

        // See the matching comment in `player_basic_attack`.
        let damage_multiplier = stat_buffs.map_or(1.0, StatBuffs::damage_multiplier);

        combat.attack_cooldown = (combat.attack_cooldown - delta).max(0.0);

        combat.ability_cooldown = (combat.ability_cooldown - delta).max(0.0);

        if combat.attack_timer > 0.0 {
            combat.attack_timer -= delta;

            if combat.attack_timer <= 0.0 {
                if combat.pending_attack == Some(WeaponType::Greatsword) {
                    let hit_enemy = greatsword_attack(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        camera_transform,
                        &mut player_transform,
                        weapon,
                        damage_multiplier,
                        &mut enemy_query,
                    );
                    if hit_enemy {
                        if let Some(tracker) = basic_attack_hits.as_mut() {
                            tracker.record_confirmed_hit();
                        }
                    }
                } else if combat.pending_attack == Some(WeaponType::Hammer) {
                    let hit_enemy = hammer_attack(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        camera_transform,
                        &mut player_transform,
                        weapon,
                        damage_multiplier,
                        &mut enemy_query,
                    );
                    if hit_enemy {
                        if let Some(tracker) = basic_attack_hits.as_mut() {
                            tracker.record_confirmed_hit();
                        }
                    }
                }

                combat.pending_attack = None;
            }
        }

        if combat.pending_ability == Some(WeaponType::Greatsword) {
            combat.ability_timer -= delta;

            if combat.ability_timer <= 0.0 {
                charged_cleave(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &mut enemy_query,
                );

                combat.pending_ability = None;
                combat.ability_timer = 0.0;
            }
        } else if combat.pending_ability == Some(WeaponType::Hammer) {
            combat.ability_timer -= delta;

            if combat.ability_timer <= 0.0 {
                hammer_ground_slam(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &player_transform,
                    weapon,
                    &mut enemy_query,
                );

                combat.pending_ability = None;
                combat.ability_timer = 0.0;
            }
        }
    }
}

fn greatsword_attack(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    damage_multiplier: f32,
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
) -> bool {
    // Get the horizontal direction the camera/crosshair is facing.
    let mut attack_direction = camera_transform.forward().as_vec3();

    attack_direction.y = 0.0;

    attack_direction = attack_direction.normalize_or_zero();

    if attack_direction == Vec3::ZERO {
        return false;
    }

    // Rotate the player toward the attack direction.
    let target_rotation = Quat::from_rotation_y(attack_direction.x.atan2(attack_direction.z));

    player_transform.rotation = target_rotation;

    // Small forward commitment to make the Greatsword
    // attack feel heavier.
    player_transform.translation += attack_direction * 0.35;

    // =====================================================
    // GREATSWORD VISUAL ARC
    // =====================================================

    spawn_melee_arc(
        commands,
        meshes,
        materials,
        player_transform,
        attack_direction,
        weapon.range,
        GREATSWORD_VISUAL_DURATION,
    );

    println!("Greatsword heavy swing!");

    // =====================================================
    // CHECK ENEMIES
    // =====================================================

    let mut hit_enemy = false;

    for (mut enemy_transform, mut enemy_health, mut stagger, mut enemy_combat, enemy_debuffs) in
        enemy_query.iter_mut()
    {
        let mut to_enemy = enemy_transform.translation - player_transform.translation;

        to_enemy.y = 0.0;

        let distance = to_enemy.length();

        // Enemy is outside Greatsword range.
        if distance > weapon.range {
            continue;
        }

        let direction_to_enemy = to_enemy.normalize_or_zero();

        if direction_to_enemy == Vec3::ZERO {
            continue;
        }

        let facing_dot = attack_direction.dot(direction_to_enemy);

        // Greatsword has a wide swing.
        if facing_dot < 0.25 {
            continue;
        }

        hit_enemy = true;

        // =================================================
        // DAMAGE
        // =================================================

        // Runebroken (see combat::enemy_debuffs) multiplies incoming
        // damage when the target has it active.
        combat::apply_enemy_damage(
            &mut enemy_health,
            enemy_debuffs,
            weapon.damage * damage_multiplier,
        );

        println!(
            "Greatsword hit! HP: {:.0}/{:.0}",
            enemy_health.current, enemy_health.max
        );

        // =================================================
        // STAGGER / INTERRUPT
        // =================================================

        if weapon.stagger_strength
            > combat::effective_stagger_resistance(stagger.resistance, enemy_debuffs)
        {
            stagger.timer = weapon.stagger_duration;

            enemy_combat.state = EnemyState::Chase;
            enemy_combat.timer = 0.0;
            enemy_combat.has_hit = false;

            println!("Enemy heavily interrupted!");
        }

        // =================================================
        // KNOCKBACK
        // =================================================

        enemy_transform.translation += direction_to_enemy * weapon.knockback;
    }

    hit_enemy
}

/// Hammer's basic attack: a short, heavy forward swing. Hammer commits
/// less forward distance than Greatsword's dash-like cleave (WEAPONS.md:
/// "slower and more committed than Longsword" -- planted, not lunging),
/// with a wider cleave angle to match a blunt two-handed weapon's arc.
/// The tier-based Ground Slam ability (see weapons::abilities::hammer) is
/// where the family's real ground-AOE identity lives -- this is just the
/// weapon's plain swing.
fn hammer_attack(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    camera_transform: &Transform,
    player_transform: &mut Transform,
    weapon: &Weapon,
    damage_multiplier: f32,
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
) -> bool {
    let mut attack_direction = camera_transform.forward().as_vec3();

    attack_direction.y = 0.0;

    attack_direction = attack_direction.normalize_or_zero();

    if attack_direction == Vec3::ZERO {
        return false;
    }

    let target_rotation = Quat::from_rotation_y(attack_direction.x.atan2(attack_direction.z));

    player_transform.rotation = target_rotation;

    player_transform.translation += attack_direction * HAMMER_ATTACK_FORWARD_STEP;

    spawn_melee_arc(
        commands,
        meshes,
        materials,
        player_transform,
        attack_direction,
        weapon.range,
        HAMMER_VISUAL_DURATION,
    );

    println!("Player swings the Hammer!");

    let mut hit_enemy = false;

    for (mut enemy_transform, mut enemy_health, mut stagger, mut enemy_combat, enemy_debuffs) in
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

        if facing_dot < HAMMER_ATTACK_ANGLE_DOT {
            continue;
        }

        hit_enemy = true;

        combat::apply_enemy_damage(
            &mut enemy_health,
            enemy_debuffs,
            weapon.damage * damage_multiplier,
        );

        if weapon.stagger_strength
            > combat::effective_stagger_resistance(stagger.resistance, enemy_debuffs)
        {
            stagger.timer = weapon.stagger_duration;

            enemy_combat.state = EnemyState::Chase;
            enemy_combat.timer = 0.0;
            enemy_combat.has_hit = false;
        }

        enemy_transform.translation += direction_to_enemy * weapon.knockback;

        println!(
            "Hammer hit! HP: {:.0}/{:.0}",
            enemy_health.current, enemy_health.max
        );
    }

    hit_enemy
}
