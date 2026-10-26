use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::characters::class_abilities::BasicAttackHitTracker;
use crate::combat::{
    self, Barriers, DamageOverTime, DebuffSource, DotSource, EnemyDebuffs, FrontalGuard, Health,
};
use crate::enemies::{Enemy, EnemyCombat, EnemyState, Stagger};
use crate::player::{Player, PlayerCharacter};

use crate::weapons::components::{
    PlayerProjectile, Projectile, ProjectileKind, ProjectileOwner, RicochetProjectile,
    WeaponAbilityState,
};

use crate::weapons::{ARROW_RADIUS, RUNIC_SURGE_SIZE, SEISMIC_FISSURE_SIZE, SHADOW_STAG_SIZE};
use crate::weapons::{crossbow, daggers, spellbook};

use super::ricochet;
use super::splinter::{
    SPLINTER_FRAGMENT_DAMAGE_MULTIPLIER, SPLINTER_FRAGMENT_LIFETIME, SPLINTER_FRAGMENT_SPEED,
    find_splinter_targets,
};

// Runesage's Rune Blast (ProjectileKind::RuneBlast) -- see
// characters::runesage_abilities. On impact it also splashes lighter
// damage to any other enemy within this radius of the impact point,
// the "explosive" half of "explosive rune projectile."
const RUNE_BLAST_EXPLOSION_RADIUS: f32 = 3.0;
const RUNE_BLAST_SPLASH_DAMAGE_FRACTION: f32 = 0.5;

// Gravemind's Ruin Bolt (ProjectileKind::RuinBolt) -- see
// characters::gravemind_abilities. Same "explosive" pattern as Rune
// Blast above, with its own independent radius/fraction so the two
// classes' tuning never has to move together.
const RUIN_BOLT_EXPLOSION_RADIUS: f32 = 2.5;
const RUIN_BOLT_SPLASH_DAMAGE_FRACTION: f32 = 0.4;

pub(crate) fn move_projectiles_with_collision(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    spatial_query: SpatialQuery,

    // Every player character: friendly projectiles pass through all of
    // them (and never hit the one that fired them).
    player_query: Query<Entity, With<PlayerCharacter>>,
    // Every player character has one (`characters::class_abilities`).
    mut player_hit_trackers: Query<&mut BasicAttackHitTracker, With<PlayerCharacter>>,
    // Saint's Nail's Final Judgment, Saint's Testament's Grace, and
    // Ruinpiercer's Cleansing Strike all read/write per-player state on
    // hit-confirmation, which only this system has (it's the only place
    // that resolves a projectile actually landing).
    // Looked up on the projectile's owner (`ProjectileOwner`), falling
    // back to this machine's `Player` for projectiles with no owner.
    mut player_ability_state: Query<
        (&mut WeaponAbilityState, &mut Barriers),
        With<PlayerCharacter>,
    >,
    local_player: Query<Entity, With<Player>>,

    mut projectile_query: Query<
        (
            Entity,
            &mut Transform,
            &mut Projectile,
            Option<&mut RicochetProjectile>,
            Option<&ProjectileOwner>,
        ),
        (With<PlayerProjectile>, Without<Enemy>),
    >,

    // `EnemyDebuffs` and `DamageOverTime` are mutable here (rather than
    // the plain `&EnemyDebuffs` earlier revisions of this file used) so
    // on-hit effects like Armorbreaker Shot's debuff, Saint's Nail's
    // Mark, and Ember Codex's Scorch can be granted right here instead
    // of needing a second, conflicting query elsewhere.
    // `crossbow::Resonance` is `Option` because, like `DamageOverTime`,
    // most enemies never carry one.
    mut enemy_queries: ParamSet<(
        Query<(Entity, &Transform, &Health), (With<Enemy>, Without<PlayerProjectile>)>,
        Query<
            (
                Entity,
                &mut Transform,
                &mut Health,
                &mut Stagger,
                &mut EnemyCombat,
                Option<&mut EnemyDebuffs>,
                Option<&FrontalGuard>,
                Option<&mut DamageOverTime>,
                Option<&mut crossbow::Resonance>,
            ),
            (With<Enemy>, Without<PlayerProjectile>),
        >,
    )>,
) {
    let delta = time.delta_secs();

    for (projectile_entity, mut projectile_transform, mut projectile, mut ricochet, owner) in
        &mut projectile_query
    {
        // Who to credit for this projectile's hits (see `ProjectileOwner`).
        let owner = owner.map(|owner| owner.0);
        // Whose per-weapon ability state (Final Judgment, Grace) this
        // projectile's hits feed.
        let ability_owner = owner.or_else(|| local_player.single().ok());

        projectile.lifetime -= delta;

        if let Some(ricochet) = ricochet.as_mut() {
            ricochet.split_lockout = (ricochet.split_lockout - delta).max(0.0);
        }

        if projectile.lifetime <= 0.0 {
            commands.entity(projectile_entity).despawn();

            continue;
        }

        let movement = projectile.velocity * delta;

        let distance = movement.length();

        if distance <= 0.0 {
            continue;
        }

        let direction = movement.normalize_or_zero();

        if direction == Vec3::ZERO {
            continue;
        }

        let start = projectile_transform.translation;

        // Ignore this projectile itself and any enemies
        // already hit by a piercing projectile.
        let mut excluded_entities = projectile.hit_entities.clone();

        excluded_entities.push(projectile_entity);

        // Player projectiles should never collide with the player who
        // fired them, or with any other player.
        for player_entity in player_query.iter() {
            excluded_entities.push(player_entity);
        }

        let filter = SpatialQueryFilter::from_excluded_entities(excluded_entities);

        let cast_direction = match Dir3::new(direction.adjust_precision()) {
            Ok(dir) => dir,
            Err(_) => continue,
        };

        let projectile_shape = match projectile.kind {
            ProjectileKind::PhantomCleave => Collider::cuboid(3.6, 1.00, 0.90),
            ProjectileKind::ShadowStag => {
                Collider::cuboid(SHADOW_STAG_SIZE.x, SHADOW_STAG_SIZE.y, SHADOW_STAG_SIZE.z)
            }
            ProjectileKind::RunicSurge => {
                Collider::cuboid(RUNIC_SURGE_SIZE.x, RUNIC_SURGE_SIZE.y, RUNIC_SURGE_SIZE.z)
            }
            ProjectileKind::SeismicFissure => Collider::cuboid(
                SEISMIC_FISSURE_SIZE.x,
                SEISMIC_FISSURE_SIZE.y,
                SEISMIC_FISSURE_SIZE.z,
            ),
            _ => Collider::sphere(ARROW_RADIUS),
        };

        // A sphere does not care about rotation, but the wide line
        // hitboxes (Phantom Cleave, Shadow Stag, Runic Surge, Seismic
        // Fissure) do.
        let shape_rotation = match projectile.kind {
            ProjectileKind::PhantomCleave
            | ProjectileKind::ShadowStag
            | ProjectileKind::RunicSurge
            | ProjectileKind::SeismicFissure => Quat::from_rotation_arc(Vec3::Z, direction),
            _ => Quat::IDENTITY,
        };

        let cast_config = ShapeCastConfig::from_max_distance(distance);

        let hit = spatial_query.cast_shape(
            &projectile_shape,
            start.adjust_precision(),
            shape_rotation.adjust_precision(),
            cast_direction,
            &cast_config,
            &filter,
        );

        // =================================================
        // COLLISION
        // =================================================

        if let Some(hit) = hit {
            if projectile.kind == ProjectileKind::PhantomCleave {
                println!(
                    "Phantom Cleave collision with {:?} at distance {:.2}",
                    hit.entity, hit.distance
                );
            }

            projectile_transform.translation = start + direction * hit.distance;

            // ---------------------------------------------
            // ENEMY HIT
            // ---------------------------------------------

            // Captured inside the block below (Bellringer's Resonance
            // reaching maximum stacks) and consumed just after it, once
            // the block's borrow of `enemy_queries.p1()` has ended --
            // the same "resolve now, splash after" shape RuneBlast/
            // RuinBolt already use for their own splash damage.
            let mut resonance_shockwave_origin: Option<Vec3> = None;

            if let Ok((
                _hit_enemy_entity,
                mut enemy_transform,
                mut enemy_health,
                mut stagger,
                mut enemy_combat,
                mut enemy_debuffs,
                frontal_guard,
                mut dot,
                mut resonance,
            )) = enemy_queries.p1().get_mut(hit.entity)
            {
                // =====================================================
                // SPLINTER FRAGMENT TARGET LOCK
                // =====================================================

                if projectile.kind == ProjectileKind::SplinterFragment {
                    if let Some(target_entity) = projectile.target {
                        // This fragment collided with an enemy that
                        // is NOT its assigned target.
                        if hit.entity != target_entity {
                            // Ignore this enemy on future casts.
                            projectile.hit_entities.push(hit.entity);

                            // Move slightly past the collision point
                            // so the fragment can continue toward
                            // its intended target.
                            projectile_transform.translation =
                                start + direction * (hit.distance + 0.05);

                            continue;
                        }
                    }
                }

                // =====================================================
                // PER-HIT DAMAGE FALLOFF (piercing Crossbow bolts)
                // =====================================================

                let pierce_index = projectile.hit_entities.len() as u32;
                let mut damage = match projectile.kind {
                    ProjectileKind::GravewindBolt => {
                        projectile.damage * crossbow::gravewind_damage_multiplier(pierce_index)
                    }
                    ProjectileKind::CorruptionPiercingBolt => {
                        projectile.damage * crossbow::ruinpiercer_damage_multiplier(pierce_index)
                    }
                    _ => projectile.damage,
                };

                // =====================================================
                // SAINT'S NAIL -- Consecrated Mark + Final Judgment
                // =====================================================

                if projectile.kind == ProjectileKind::ConsecratedBolt {
                    if let Some(debuffs) = enemy_debuffs.as_deref_mut() {
                        if let Some(Ok((mut state, _barriers))) = match ability_owner {
                            Some(owner) => Some(player_ability_state.get_mut(owner)),
                            None => None,
                        } {
                            let judgment_multiplier = crossbow::apply_consecrated_mark(
                                debuffs,
                                stagger.resistance,
                                &mut state,
                            );
                            damage *= judgment_multiplier;
                        }
                    }
                }

                // =================================================
                // DAMAGE
                // =================================================

                // Runebroken (see combat::enemy_debuffs) multiplies
                // incoming damage when the target has it active.
                // FrontalGuard (see combat::frontal_guard): a guarding enemy
                // hit by a light attack from the front blocks most of the
                // damage and is not staggered by it.
                let guarded = combat::apply_guarded_enemy_damage(
                    &mut enemy_health,
                    enemy_debuffs.as_deref(),
                    frontal_guard,
                    enemy_transform.rotation,
                    enemy_transform.translation,
                    start,
                    projectile.stagger_strength,
                    damage,
                );

                println!(
                    "Projectile hit enemy. Piercing: {}, damage: {:.1}",
                    projectile.piercing, damage
                );

                if projectile.kind == ProjectileKind::Normal {
                    // Credit the player who fired it (or, with no owner,
                    // this machine's `Player`), e.g. for Vanguard's Momentum.
                    let tracker = match ability_owner {
                        Some(owner) => player_hit_trackers.get_mut(owner).ok(),
                        None => None,
                    };
                    if let Some(mut tracker) = tracker {
                        tracker.record_confirmed_hit();
                    }
                }

                println!(
                    "Arrow hit! Enemy HP: {:.0}/{:.0}",
                    enemy_health.current, enemy_health.max
                );

                // -----------------------------------------
                // STAGGER
                // -----------------------------------------

                if !guarded.stagger_negated
                    && projectile.stagger_strength
                        > combat::effective_stagger_resistance(
                            stagger.resistance,
                            enemy_debuffs.as_deref(),
                        )
                {
                    stagger.timer = projectile.stagger_duration;

                    enemy_combat.state = EnemyState::Chase;

                    enemy_combat.timer = 0.0;
                    enemy_combat.has_hit = false;

                    println!("Arrow interrupted enemy!");
                }

                // -----------------------------------------
                // KNOCKBACK
                // -----------------------------------------

                let mut knockback_direction = direction;

                knockback_direction.y = 0.0;

                knockback_direction = knockback_direction.normalize_or_zero();

                enemy_transform.translation += knockback_direction * projectile.knockback;

                // -----------------------------------------
                // SIEGEBREAKER -- Armorbreaker Shot debuff
                // -----------------------------------------

                if projectile.kind == ProjectileKind::ArmorbreakerBolt {
                    if let Some(debuffs) = enemy_debuffs.as_deref_mut() {
                        debuffs.grant(
                            DebuffSource::ArmorbreakerShot,
                            crossbow::ARMORBREAKER_DEBUFF_MULTIPLIER,
                            0,
                            crossbow::ARMORBREAKER_DEBUFF_DURATION,
                        );
                    }
                }

                // -----------------------------------------
                // THE RUINPIERCER -- kill-attribution tag
                // -----------------------------------------

                if projectile.kind == ProjectileKind::CorruptionPiercingBolt {
                    commands
                        .entity(hit.entity)
                        .insert(crossbow::RuinpiercerMark {
                            owner: ability_owner,
                        });
                }

                // -----------------------------------------
                // EMBER CODEX -- Scorch
                // -----------------------------------------

                if projectile.kind == ProjectileKind::ScorchBolt {
                    match dot.as_deref_mut() {
                        Some(existing) => existing.apply(
                            spellbook::SCORCH_SOURCE,
                            spellbook::SCORCH_DAMAGE_PER_SECOND,
                            spellbook::SCORCH_DURATION,
                        ),
                        None => {
                            commands
                                .entity(hit.entity)
                                .insert(DamageOverTime::with_stack(
                                    spellbook::SCORCH_SOURCE,
                                    spellbook::SCORCH_DAMAGE_PER_SECOND,
                                    spellbook::SCORCH_DURATION,
                                ));
                        }
                    }
                }

                // -----------------------------------------
                // WIDOW'S KISS -- Venom + Bleed -> Envenomed
                // -----------------------------------------
                // "Venom and Bleed together cause the target to become
                // Envenomed, briefly slowing it" (see weapons::daggers'
                // module doc comment and `DotSource::Bleed`). The thrown
                // dagger itself carries both stacks, so a single landed
                // hit is enough to trigger Envenomed.

                if projectile.kind == ProjectileKind::VenomousDagger {
                    match dot.as_deref_mut() {
                        Some(existing) => {
                            existing.apply(
                                DotSource::Venom,
                                daggers::WIDOWS_KISS_VENOM_DPS,
                                daggers::WIDOWS_KISS_VENOM_DURATION,
                            );
                            existing.apply(
                                DotSource::Bleed,
                                daggers::WIDOWS_KISS_BLEED_DPS,
                                daggers::WIDOWS_KISS_BLEED_DURATION,
                            );
                        }
                        None => {
                            let mut fresh = DamageOverTime::with_stack(
                                DotSource::Venom,
                                daggers::WIDOWS_KISS_VENOM_DPS,
                                daggers::WIDOWS_KISS_VENOM_DURATION,
                            );
                            fresh.apply(
                                DotSource::Bleed,
                                daggers::WIDOWS_KISS_BLEED_DPS,
                                daggers::WIDOWS_KISS_BLEED_DURATION,
                            );
                            commands.entity(hit.entity).insert(fresh);
                        }
                    }

                    if let Some(debuffs) = enemy_debuffs.as_deref_mut() {
                        debuffs.grant_with_slow(
                            DebuffSource::Envenomed,
                            1.0,
                            0,
                            daggers::WIDOWS_KISS_ENVENOMED_SLOW_MULTIPLIER,
                            daggers::WIDOWS_KISS_ENVENOMED_DURATION,
                        );
                    }

                    println!("Venomous dagger struck! Target Envenomed.");
                }

                // -----------------------------------------
                // SAINT'S TESTAMENT -- Grace
                // -----------------------------------------

                if projectile.kind == ProjectileKind::GraceBolt {
                    if let Some(Ok((mut state, mut barriers))) = match ability_owner {
                        Some(owner) => Some(player_ability_state.get_mut(owner)),
                        None => None,
                    } {
                        spellbook::apply_grace(&mut state, &mut barriers);
                    }
                }

                // -----------------------------------------
                // BELLRINGER -- Resonance
                // -----------------------------------------

                if projectile.kind == ProjectileKind::ResonantBolt {
                    let stacks = match resonance.as_deref_mut() {
                        Some(existing) => {
                            existing.add_stack();
                            existing.stacks
                        }
                        None => {
                            commands
                                .entity(hit.entity)
                                .insert(crossbow::Resonance::first_stack());
                            1
                        }
                    };

                    if stacks >= crossbow::RESONANCE_MAX_STACKS {
                        resonance_shockwave_origin = Some(enemy_transform.translation);
                    }
                }

                // -----------------------------------------
                // PIERCING
                // -----------------------------------------

                if projectile.piercing {
                    projectile.hit_entities.push(hit.entity);

                    projectile_transform.translation = start + direction * (hit.distance + 0.05);

                    continue;
                }
            }

            // =================================================
            // BELLRINGER RESONANCE SHOCKWAVE
            // =================================================
            // The hit that just filled Resonance to maximum -- resolved
            // here, after the block above has released its borrow of
            // `enemy_queries.p1()`, the same "resolve now, splash after"
            // shape Rune Blast/Ruin Bolt below use for their own splash.
            if let Some(origin) = resonance_shockwave_origin {
                let struck_entity = hit.entity;
                let mut nearby_enemies = enemy_queries.p1();

                for (
                    entity,
                    transform,
                    mut health,
                    mut nearby_stagger,
                    mut nearby_combat,
                    nearby_debuffs,
                    _frontal_guard,
                    _dot,
                    _resonance,
                ) in nearby_enemies.iter_mut()
                {
                    if entity == struck_entity {
                        continue;
                    }

                    if transform.translation.distance(origin) > crossbow::RESONANCE_SHOCKWAVE_RADIUS
                    {
                        continue;
                    }

                    combat::apply_enemy_damage(
                        &mut health,
                        nearby_debuffs.as_deref(),
                        crossbow::RESONANCE_SHOCKWAVE_DAMAGE,
                    );

                    let effective_resistance = combat::effective_stagger_resistance(
                        nearby_stagger.resistance,
                        nearby_debuffs.as_deref(),
                    );

                    if crossbow::RESONANCE_SHOCKWAVE_STAGGER_BONUS > effective_resistance {
                        nearby_stagger.timer = nearby_stagger.timer.max(0.4);
                        nearby_combat.state = EnemyState::Chase;
                        nearby_combat.timer = 0.0;
                        nearby_combat.has_hit = false;
                    }
                }

                println!("Resonance shockwave!");
            }

            // =================================================
            // RUNE BLAST EXPLOSION
            // =================================================
            // The "explosive" half of Runesage's Rune Blast (see
            // characters::runesage_abilities): whatever the bolt directly
            // struck (an enemy, above, or world geometry) has already been
            // resolved -- this splashes lighter damage onto any *other*
            // nearby enemy, the same per-enemy-loop shape
            // weapons::melee's attacks already use, rather than a second
            // spatial-query shape cast.
            if projectile.kind == ProjectileKind::RuneBlast {
                let impact_position = projectile_transform.translation;
                let splash_damage = projectile.damage * RUNE_BLAST_SPLASH_DAMAGE_FRACTION;

                let mut nearby_enemies = enemy_queries.p1();

                for (
                    nearby_entity,
                    mut nearby_transform,
                    mut nearby_health,
                    mut nearby_stagger,
                    mut nearby_combat,
                    nearby_debuffs,
                    nearby_frontal_guard,
                    _nearby_dot,
                    _nearby_resonance,
                ) in nearby_enemies.iter_mut()
                {
                    if nearby_entity == hit.entity {
                        // Already damaged by the direct hit above.
                        continue;
                    }

                    let splash_distance = nearby_transform.translation.distance(impact_position);

                    if splash_distance > RUNE_BLAST_EXPLOSION_RADIUS {
                        continue;
                    }

                    let splash_guarded = combat::apply_guarded_enemy_damage(
                        &mut nearby_health,
                        nearby_debuffs.as_deref(),
                        nearby_frontal_guard,
                        nearby_transform.rotation,
                        nearby_transform.translation,
                        impact_position,
                        projectile.stagger_strength,
                        splash_damage,
                    );

                    let splash_resistance = combat::effective_stagger_resistance(
                        nearby_stagger.resistance,
                        nearby_debuffs.as_deref(),
                    );

                    if !splash_guarded.stagger_negated
                        && projectile.stagger_strength > splash_resistance
                    {
                        nearby_stagger.timer = projectile.stagger_duration;
                        nearby_combat.state = EnemyState::Chase;
                        nearby_combat.timer = 0.0;
                        nearby_combat.has_hit = false;
                    }

                    let mut splash_knockback = nearby_transform.translation - impact_position;
                    splash_knockback.y = 0.0;
                    splash_knockback = splash_knockback.normalize_or_zero();
                    nearby_transform.translation += splash_knockback * (projectile.knockback * 0.6);

                    println!("Rune Blast splash hit! Damage {:.1}", splash_damage);
                }
            }

            // =================================================
            // RUIN BOLT EXPLOSION
            // =================================================
            // The "blast" half of Gravemind's Ruin Bolt (see
            // characters::gravemind_abilities) -- identical shape to the
            // Rune Blast splash above, with its own radius/damage
            // fraction constants.
            if projectile.kind == ProjectileKind::RuinBolt {
                let impact_position = projectile_transform.translation;
                let splash_damage = projectile.damage * RUIN_BOLT_SPLASH_DAMAGE_FRACTION;

                let mut nearby_enemies = enemy_queries.p1();

                for (
                    nearby_entity,
                    mut nearby_transform,
                    mut nearby_health,
                    mut nearby_stagger,
                    mut nearby_combat,
                    nearby_debuffs,
                    nearby_frontal_guard,
                    _nearby_dot,
                    _nearby_resonance,
                ) in nearby_enemies.iter_mut()
                {
                    if nearby_entity == hit.entity {
                        // Already damaged by the direct hit above.
                        continue;
                    }

                    let splash_distance = nearby_transform.translation.distance(impact_position);

                    if splash_distance > RUIN_BOLT_EXPLOSION_RADIUS {
                        continue;
                    }

                    let splash_guarded = combat::apply_guarded_enemy_damage(
                        &mut nearby_health,
                        nearby_debuffs.as_deref(),
                        nearby_frontal_guard,
                        nearby_transform.rotation,
                        nearby_transform.translation,
                        impact_position,
                        projectile.stagger_strength,
                        splash_damage,
                    );

                    let splash_resistance = combat::effective_stagger_resistance(
                        nearby_stagger.resistance,
                        nearby_debuffs.as_deref(),
                    );

                    if !splash_guarded.stagger_negated
                        && projectile.stagger_strength > splash_resistance
                    {
                        nearby_stagger.timer = projectile.stagger_duration;
                        nearby_combat.state = EnemyState::Chase;
                        nearby_combat.timer = 0.0;
                        nearby_combat.has_hit = false;
                    }

                    let mut splash_knockback = nearby_transform.translation - impact_position;
                    splash_knockback.y = 0.0;
                    splash_knockback = splash_knockback.normalize_or_zero();
                    nearby_transform.translation += splash_knockback * (projectile.knockback * 0.6);

                    println!("Ruin Bolt splash hit! Damage {:.1}", splash_damage);
                }
            }

            // =================================================
            // RICOCHET ARROW
            // =================================================

            if projectile.kind == ProjectileKind::Ricochet {
                let Some(ricochet) = ricochet.as_mut() else {
                    commands.entity(projectile_entity).despawn();

                    continue;
                };

                ricochet::handle_ricochet_collision(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &spatial_query,
                    &player_query,
                    projectile_entity,
                    &mut projectile_transform,
                    &mut projectile,
                    ricochet,
                    start,
                    direction,
                    cast_direction,
                    hit.distance,
                    hit.normal1,
                    &filter,
                );

                continue;
            }

            // =================================================
            // Splinter Shot
            // =================================================

            if projectile.kind == ProjectileKind::Splinter {
                let impact_position = projectile_transform.translation;

                // Check whether the initial Splinter Shot
                // killed its main target.
                let main_target_alive = {
                    let enemy_positions = enemy_queries.p0();

                    match enemy_positions.get(hit.entity) {
                        Ok((_entity, _transform, health)) => health.current > 0.0,

                        Err(_) => false,
                    }
                };

                // Choose exactly where the fragments should go.
                let targets = {
                    let enemy_positions = enemy_queries.p0();

                    find_splinter_targets(
                        impact_position,
                        hit.entity,
                        main_target_alive,
                        &enemy_positions,
                    )
                };

                for target_entity in targets {
                    let target_position = {
                        let enemy_positions = enemy_queries.p0();

                        let Ok((_entity, target_transform, _health)) =
                            enemy_positions.get(target_entity)
                        else {
                            continue;
                        };

                        target_transform.translation
                    };

                    let direction = (target_position - impact_position).normalize_or_zero();

                    if direction == Vec3::ZERO {
                        continue;
                    }

                    commands.spawn((
                        PlayerProjectile,
                        Projectile {
                            velocity: direction * SPLINTER_FRAGMENT_SPEED,

                            damage: projectile.damage * SPLINTER_FRAGMENT_DAMAGE_MULTIPLIER,

                            knockback: projectile.knockback * 0.5,

                            stagger_strength: projectile.stagger_strength,

                            stagger_duration: projectile.stagger_duration,

                            lifetime: SPLINTER_FRAGMENT_LIFETIME,

                            piercing: false,

                            hit_entities: if target_entity == hit.entity {
                                // This fragment is intentionally returning
                                // to the original target.
                                Vec::new()
                            } else {
                                // This fragment is going somewhere else,
                                // so ignore the original target.
                                vec![hit.entity]
                            },

                            kind: ProjectileKind::SplinterFragment,

                            target: Some(target_entity),
                        },
                        Mesh3d(meshes.add(Sphere::new(ARROW_RADIUS * 0.7))),
                        MeshMaterial3d(materials.add(Color::srgb(0.75, 0.3, 1.0))),
                        Transform::from_translation(impact_position + direction * 0.2),
                    ));
                }

                println!("Splinter Shot fragmented!");
            }

            // Normal arrow hit an enemy,
            // or any arrow hit world geometry.
            commands.entity(projectile_entity).despawn();

            continue;
        }

        // =================================================
        // NO COLLISION
        // =================================================

        projectile_transform.translation += movement;
    }
}
