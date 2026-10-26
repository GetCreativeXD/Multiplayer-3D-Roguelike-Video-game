use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::visual_effects::{EffectShape, effect_mesh};

use crate::characters::class_abilities::{
    AbilityUseTracker, BasicAttackHitTracker, BasicAttackTracker, CharacterCombatModifiers,
    ClassAbilityDisplay, ClassAbilityHud, ClassAbilityHudSlot,
};
use crate::characters::vanguard::Vanguard;
use crate::enemies::Enemy;
use crate::player::{AbilityMovement, PlayerCharacter, PlayerIntent};
use crate::weapons::{PlayerProjectile, Projectile, ProjectileKind, Weapon, WeaponType};

// Class-ability slots (`player::CLASS_ABILITY_KEYS`): Q and R.
const LUNGE_SLOT: usize = 0;
const AFTERIMAGE_SLOT: usize = 2;

const LUNGE_RANGE: f32 = 12.0;
const LUNGE_MIN_AIM_DOT: f32 = 0.72;
const LUNGE_STOP_DISTANCE: f32 = 1.5;
const LUNGE_SPEED: f32 = 20.0;
const LUNGE_COOLDOWN: f32 = 2.5;

const MOMENTUM_MAX_STACKS: u8 = 5;
const MOMENTUM_DECAY_DELAY: f32 = 1.5;
const MOMENTUM_DECAY_INTERVAL: f32 = 0.75;
const MOMENTUM_ATTACK_SPEED_PER_STACK: f32 = 0.08;
const MOMENTUM_MOVE_SPEED_PER_STACK: f32 = 0.06;

const AFTERIMAGE_DURATION: f32 = 5.0;
const AFTERIMAGE_COOLDOWN: f32 = 10.0;
const AFTERIMAGE_DAMAGE_MULTIPLIER: f32 = 0.55;
const AFTERIMAGE_PROJECTILE_SPEED: f32 = 20.0;
const AFTERIMAGE_PROJECTILE_LIFETIME: f32 = 2.5;
const AFTERIMAGE_TARGET_RANGE: f32 = 35.0;

#[derive(Component, Default)]
pub struct VanguardLunge {
    cooldown: f32,
}

#[derive(Component, Default)]
pub struct VanguardMomentum {
    stacks: u8,
    decay_timer: f32,
    observed_basic_attack_hits: u64,
}

pub struct VanguardAbilitiesPlugin;

impl Plugin for VanguardAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                initialize_vanguard_abilities,
                update_vanguard_timers,
                lunge_input,
                afterimage_input,
                update_afterimages,
                sync_vanguard_class_state,
            )
                .chain(),
        );
    }
}

impl VanguardMomentum {
    fn record_basic_attack_hit(&mut self) {
        self.stacks = (self.stacks + 1).min(MOMENTUM_MAX_STACKS);
        self.decay_timer = MOMENTUM_DECAY_DELAY;
        println!("Momentum: {}/{}", self.stacks, MOMENTUM_MAX_STACKS);
    }

    pub fn attack_speed_multiplier(&self) -> f32 {
        1.0 + self.stacks as f32 * MOMENTUM_ATTACK_SPEED_PER_STACK
    }

    pub fn movement_speed_multiplier(&self) -> f32 {
        1.0 + self.stacks as f32 * MOMENTUM_MOVE_SPEED_PER_STACK
    }

    pub fn stacks(&self) -> u8 {
        self.stacks
    }

    pub fn max_stacks(&self) -> u8 {
        MOMENTUM_MAX_STACKS
    }
}

impl VanguardLunge {
    pub fn cooldown(&self) -> f32 {
        self.cooldown
    }
}

#[derive(Component, Default)]
pub struct VanguardAfterimage {
    clone: Option<Entity>,
    cooldown: f32,
}

impl VanguardAfterimage {
    pub fn cooldown(&self) -> f32 {
        self.cooldown
    }

    pub fn is_active(&self) -> bool {
        self.clone.is_some()
    }
}

#[derive(Component)]
pub struct AfterimageClone {
    owner: Entity,
    remaining: f32,
    observed_basic_attacks: u64,
    weapon: Weapon,
}

pub fn initialize_vanguard_abilities(
    mut commands: Commands,
    // Every player character, so a joined Vanguard works too
    // (`net::loadout` gives their body the class). Abilities are cast from
    // each player's `PlayerIntent` (keys Q/E/R, aimed with their camera).
    players: Query<Entity, (With<PlayerCharacter>, Added<Vanguard>)>,
) {
    for player in &players {
        commands.entity(player).insert((
            VanguardLunge::default(),
            VanguardMomentum::default(),
            VanguardAfterimage::default(),
            ClassAbilityHud::new([
                ClassAbilityHudSlot::new("Q", "LUNGE", ClassAbilityDisplay::Ready),
                ClassAbilityHudSlot::new(
                    "PASSIVE",
                    "MOMENTUM",
                    ClassAbilityDisplay::Counter {
                        current: 0,
                        max: MOMENTUM_MAX_STACKS,
                    },
                ),
                ClassAbilityHudSlot::new("R", "AFTERIMAGE", ClassAbilityDisplay::Ready),
            ]),
            AbilityMovement {
                direction: Vec3::ZERO,
                speed: 0.0,
                time_remaining: 0.0,
            },
        ));
    }
}

pub fn sync_vanguard_class_state(
    mut players: Query<
        (
            &VanguardLunge,
            &VanguardMomentum,
            &VanguardAfterimage,
            &mut ClassAbilityHud,
            &mut CharacterCombatModifiers,
        ),
        (With<PlayerCharacter>, With<Vanguard>),
    >,
) {
    for (lunge, momentum, afterimage, mut hud, mut modifiers) in &mut players {
        hud.slots[0].display = if lunge.cooldown() <= 0.0 {
            ClassAbilityDisplay::Ready
        } else {
            ClassAbilityDisplay::Cooldown(lunge.cooldown())
        };

        hud.slots[1].display = ClassAbilityDisplay::Counter {
            current: momentum.stacks(),
            max: momentum.max_stacks(),
        };

        hud.slots[2].display = if afterimage.is_active() {
            ClassAbilityDisplay::Active("RETURN READY")
        } else if afterimage.cooldown() <= 0.0 {
            ClassAbilityDisplay::Ready
        } else {
            ClassAbilityDisplay::Cooldown(afterimage.cooldown())
        };

        modifiers.attack_speed = momentum.attack_speed_multiplier();
        modifiers.movement_speed = momentum.movement_speed_multiplier();
    }
}

pub fn update_vanguard_timers(
    time: Res<Time>,
    mut players: Query<
        (
            &mut VanguardLunge,
            &mut VanguardMomentum,
            &mut VanguardAfterimage,
            &BasicAttackHitTracker,
        ),
        (With<PlayerCharacter>, With<Vanguard>),
    >,
) {
    let delta = time.delta_secs();

    for (mut lunge, mut momentum, mut afterimage, basic_attack_hits) in &mut players {
        lunge.cooldown = (lunge.cooldown - delta).max(0.0);
        afterimage.cooldown = (afterimage.cooldown - delta).max(0.0);

        let new_hits = basic_attack_hits
            .total_confirmed()
            .saturating_sub(momentum.observed_basic_attack_hits);
        momentum.observed_basic_attack_hits = basic_attack_hits.total_confirmed();

        for _ in 0..new_hits {
            momentum.record_basic_attack_hit();
        }

        if momentum.stacks == 0 {
            continue;
        }

        momentum.decay_timer -= delta;
        if momentum.decay_timer <= 0.0 {
            momentum.stacks -= 1;
            momentum.decay_timer = MOMENTUM_DECAY_INTERVAL;
            println!("Momentum: {}/{}", momentum.stacks, MOMENTUM_MAX_STACKS);
        }
    }
}

pub fn lunge_input(
    enemies: Query<&Transform, (With<Enemy>, Without<PlayerCharacter>)>,
    mut players: Query<
        (
            &mut Transform,
            &mut PlayerIntent,
            &mut AbilityMovement,
            &mut VanguardLunge,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Vanguard>, Without<Enemy>),
    >,
) {
    for (mut player_transform, mut intent, mut movement, mut lunge, mut ability_tracker) in
        &mut players
    {
        // A press (Q, for the local player) is consumed even when it can't
        // cast, as `just_pressed` was before.
        if !intent.take_class_ability(LUNGE_SLOT) {
            continue;
        }
        let camera_transform = intent.aim_transform();

        if lunge.cooldown > 0.0 || movement.is_active() {
            continue;
        }

        let mut aim = camera_transform.forward().as_vec3();
        aim.y = 0.0;
        aim = aim.normalize_or_zero();

        let mut best: Option<(f32, f32, Vec3)> = None;
        for enemy_transform in &enemies {
            let mut offset = enemy_transform.translation - player_transform.translation;
            offset.y = 0.0;
            let distance = offset.length();
            if distance <= LUNGE_STOP_DISTANCE || distance > LUNGE_RANGE {
                continue;
            }

            let direction = offset.normalize_or_zero();
            let dot = aim.dot(direction);
            if dot < LUNGE_MIN_AIM_DOT {
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
            println!("Lunge: no target in range");
            continue;
        };

        movement.direction = direction;
        movement.speed = LUNGE_SPEED;
        movement.time_remaining = ((distance - LUNGE_STOP_DISTANCE) / LUNGE_SPEED).max(0.05);
        player_transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
        lunge.cooldown = LUNGE_COOLDOWN;
        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }
        println!("Lunge!");
    }
}

pub fn afterimage_input(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut players: Query<
        (
            Entity,
            &mut Transform,
            &mut PlayerIntent,
            &Weapon,
            &BasicAttackTracker,
            &mut VanguardAfterimage,
            Option<&mut AbilityUseTracker>,
        ),
        (With<PlayerCharacter>, With<Vanguard>),
    >,
    clones: Query<&Transform, (With<AfterimageClone>, Without<PlayerCharacter>)>,
) {
    for (
        player_entity,
        mut player_transform,
        mut intent,
        weapon,
        basic_attacks,
        mut ability,
        mut ability_tracker,
    ) in &mut players
    {
        // R, for the local player.
        if !intent.take_class_ability(AFTERIMAGE_SLOT) {
            continue;
        }

        if let Some(clone_entity) = ability.clone {
            if let Ok(clone_transform) = clones.get(clone_entity) {
                player_transform.translation = clone_transform.translation;
                commands.entity(clone_entity).despawn();
                ability.clone = None;
                println!("Afterimage return!");
                continue;
            }
            ability.clone = None;
        }

        if ability.cooldown > 0.0 {
            continue;
        }

        let clone_entity = commands
            .spawn((
                Name::new("Vanguard Afterimage"),
                AfterimageClone {
                    owner: player_entity,
                    remaining: AFTERIMAGE_DURATION,
                    observed_basic_attacks: basic_attacks.total_started(),
                    weapon: *weapon,
                },
                effect_mesh(
                    &mut *meshes,
                    EffectShape::Capsule {
                        radius: 0.42,
                        length: 1.0,
                    },
                ),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(0.25, 0.75, 1.0, 0.38),
                    emissive: LinearRgba::new(0.15, 0.65, 1.0, 1.0),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                })),
                Transform::from_translation(player_transform.translation),
            ))
            .id();

        ability.clone = Some(clone_entity);
        ability.cooldown = AFTERIMAGE_COOLDOWN;
        if let Some(tracker) = ability_tracker.as_deref_mut() {
            tracker.record_ability_used();
        }
        println!("Afterimage!");
    }
}

pub fn update_afterimages(
    mut commands: Commands,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spatial_query: SpatialQuery,
    enemies: Query<(Entity, &Transform), (With<Enemy>, Without<AfterimageClone>)>,
    // The clone's owner: whose basic attacks it echoes, aimed with that
    // player's camera (`PlayerIntent::aim_transform`).
    mut players: Query<
        (&mut VanguardAfterimage, &BasicAttackTracker, &PlayerIntent),
        (With<PlayerCharacter>, With<Vanguard>),
    >,
    mut clones: Query<(Entity, &Transform, &mut AfterimageClone), Without<PlayerCharacter>>,
) {
    let delta = time.delta_secs();

    for (clone_entity, clone_transform, mut clone) in &mut clones {
        clone.remaining -= delta;

        if clone.remaining <= 0.0 {
            commands.entity(clone_entity).despawn();
            if let Ok((mut ability, _, _)) = players.get_mut(clone.owner) {
                if ability.clone == Some(clone_entity) {
                    ability.clone = None;
                }
            }
            continue;
        }

        let Ok((_ability, basic_attacks, owner_intent)) = players.get_mut(clone.owner) else {
            continue;
        };
        let camera_transform = owner_intent.aim_transform();

        if basic_attacks.total_started() <= clone.observed_basic_attacks {
            continue;
        }

        clone.observed_basic_attacks = basic_attacks.total_started();

        let Some((target_entity, target_position)) = focused_enemy(
            &camera_transform,
            clone.owner,
            clone_entity,
            &spatial_query,
            &enemies,
        ) else {
            continue;
        };

        let spawn_position = clone_transform.translation + Vec3::Y * 0.8;
        let direction = (target_position - spawn_position).normalize_or_zero();
        if direction == Vec3::ZERO {
            continue;
        }

        // Hammer clone sends a spectral hammer-shaped shockwave rather than
        // physically walking into melee (context doc section 21) -- the
        // biggest, slowest-reading projectile of the set, matching the
        // heaviest weapon in the roster.
        let (radius, color) = match clone.weapon.weapon_type {
            WeaponType::Longsword => (0.24, Color::srgb(0.35, 0.85, 1.0)),
            WeaponType::Greatsword => (0.36, Color::srgb(0.25, 0.65, 1.0)),
            WeaponType::Bow => (0.16, Color::srgb(0.55, 0.9, 1.0)),
            WeaponType::Hammer => (0.46, Color::srgb(0.85, 0.5, 0.18)),
            // Colors match each weapon's own projectile
            // (`weapons::crossbow::CROSSBOW_BOLT_COLOR` /
            // `weapons::spellbook::ARCANE_BOLT_COLOR`) so an Afterimage
            // clone's shot reads as the same weapon.
            WeaponType::Crossbow => (0.19, Color::srgb(0.55, 0.55, 0.62)),
            WeaponType::Spellbook => (0.20, Color::srgb(0.55, 0.75, 1.0)),
            // Small and fast, matching Daggers' own low-damage/high-speed
            // identity; violet to echo Widow's Kiss / Dancer in the
            // Dark's shadow-and-venom visuals (see weapons::daggers).
            WeaponType::Daggers => (0.14, Color::srgb(0.72, 0.32, 0.85)),
        };

        commands.spawn((
            PlayerProjectile,
            Projectile {
                velocity: direction * AFTERIMAGE_PROJECTILE_SPEED,
                damage: clone.weapon.damage * AFTERIMAGE_DAMAGE_MULTIPLIER,
                knockback: clone.weapon.knockback * 0.5,
                stagger_strength: clone.weapon.stagger_strength,
                stagger_duration: clone.weapon.stagger_duration,
                lifetime: AFTERIMAGE_PROJECTILE_LIFETIME,
                piercing: false,
                hit_entities: Vec::new(),
                kind: ProjectileKind::Afterimage,
                target: Some(target_entity),
            },
            Mesh3d(meshes.add(Sphere::new(radius))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                emissive: LinearRgba::new(0.15, 0.7, 1.0, 1.0),
                unlit: true,
                ..default()
            })),
            Transform::from_translation(spawn_position),
        ));
    }
}

fn focused_enemy(
    camera: &Transform,
    player_entity: Entity,
    clone_entity: Entity,
    spatial_query: &SpatialQuery,
    enemies: &Query<(Entity, &Transform), (With<Enemy>, Without<AfterimageClone>)>,
) -> Option<(Entity, Vec3)> {
    let origin = camera.translation;
    let direction = camera.forward().as_vec3().normalize_or_zero();
    let ray_direction = Dir3::new(direction.adjust_precision()).ok()?;
    let filter = SpatialQueryFilter::from_excluded_entities([player_entity, clone_entity]);

    let hit = spatial_query.cast_ray(
        origin.adjust_precision(),
        ray_direction,
        AFTERIMAGE_TARGET_RANGE,
        true,
        &filter,
    )?;

    let (entity, transform) = enemies.get(hit.entity).ok()?;
    Some((entity, transform.translation + Vec3::Y * 0.6))
}
