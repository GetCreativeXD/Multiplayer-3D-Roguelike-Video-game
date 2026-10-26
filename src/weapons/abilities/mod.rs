mod greatsword;
mod hammer;
mod longsword;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::characters::class_abilities::AbilityUseTracker;
use crate::combat::Health;
use crate::enemies::{Enemy, EnemyCombat, Stagger};
use crate::player::{Dodge, Player, PlayerCharacter, PlayerIntent};
use crate::ruin::RuinState;

use super::bow;
use super::{crossbow, daggers, spellbook};

use super::components::{GearTier, PlayerCombat, Weapon, WeaponAbilityState, WeaponType};

use longsword::{longsword_crosscut, longsword_lunging_slash, start_blade_rush};

use greatsword::{fire_phantom_cleave, greatsword_executioners_sweep};

use hammer::{hammer_earthshatter, hammer_seismic_strike, spawn_ground_slam_telegraph};

pub(crate) use greatsword::charged_cleave;

pub(crate) use longsword::update_blade_rush;

pub(crate) use hammer::hammer_ground_slam;
pub(crate) use hammer::{
    update_earthshatter_aftershocks, update_ground_slam_telegraphs, update_hammer_impact_visuals,
    update_unstable_ground,
};

const GREATSWORD_CLEAVE_WINDUP: f32 = 0.65;
const GREATSWORD_CLEAVE_COOLDOWN: f32 = 2.5;

const HAMMER_GROUND_SLAM_WINDUP: f32 = 0.5;
const HAMMER_GROUND_SLAM_COOLDOWN: f32 = 5.5;

const LONGSWORD_LUNGE_COOLDOWN: f32 = 1.5;

const BOW_PIERCING_COOLDOWN: f32 = 2.0;

const BOW_RICOCHET_COOLDOWN: f32 = 3.5;

// Crossbow -- slower, heavier-hitting cooldowns than Bow's, matching its
// "deliberate" identity (see `weapons::crossbow`'s module doc comment).
// Experimental values, not yet playtested -- tune alongside the rest of
// Crossbow's numbers.
const HUNTERS_BOLT_COOLDOWN: f32 = 2.4;

// =========================================================
// RARE WEAPON ABILITIES
// =========================================================

const GREATSWORD_EXECUTION_COOLDOWN: f32 = 2.8;

const LONGSWORD_CROSSCUT_COOLDOWN: f32 = 2.0;

// Bow - Splinter Shot
const BOW_SPLINTER_COOLDOWN: f32 = 2.2;

const HAMMER_SEISMIC_STRIKE_COOLDOWN: f32 = 3.2;

const GRAVEWIND_BOLT_COOLDOWN: f32 = 2.6;

// =========================================================
// EPIC WEAPON ABILITIES
// =========================================================

const GREATSWORD_PHANTOM_CLEAVE_COOLDOWN: f32 = 4.0;

const LONGSWORD_BLADE_RUSH_COOLDOWN: f32 = 3.5;

const HAMMER_EARTHSHATTER_COOLDOWN: f32 = 6.5;

const ARMORBREAKER_SHOT_COOLDOWN: f32 = 4.5;

// =========================================================
// LEGENDARY WEAPON ABILITIES -- Crossbow
// =========================================================

const SAINTS_NAIL_COOLDOWN: f32 = 3.0;
const RUINPIERCER_COOLDOWN: f32 = 3.0;
const BELLRINGER_COOLDOWN: f32 = 2.8;

// =========================================================
// DAGGERS -- see WEAPONS.md's Daggers section and
// `weapons::daggers`. Fastest-cooling ability family in the roster,
// matching Daggers' overall speed-over-power identity.
// =========================================================

const RAPID_FLURRY_COOLDOWN: f32 = 2.2;
const SERRATED_RUSH_COOLDOWN: f32 = 2.6;
const PHANTOM_STEP_COOLDOWN: f32 = 3.0;
const KINGSLAYER_DEATHMARK_COOLDOWN: f32 = 4.0;
const WIDOWS_KISS_BARRAGE_COOLDOWN: f32 = 2.6;
const DANCER_SHADOWSTEP_COOLDOWN: f32 = 3.2;

// =========================================================
// SPELLBOOK -- every tier shares Basic Arcana's baseline
// (`Weapon::spellbook`'s ability_power_bonus / ability_cooldown_multiplier)
// on top of its own tier ability below, per WEAPONS.md. Cooldowns here are
// pre-multiplier base values -- see `weapon.ability_cooldown_multiplier`
// applied at the bottom of `weapon_ability_input`.
// =========================================================

const FOCUSED_CAST_COOLDOWN: f32 = 1.8;
const SCORCHING_CAST_COOLDOWN: f32 = 2.2;
const ECHOING_CAST_COOLDOWN: f32 = 2.0;
const HOLLOW_GOSPEL_COOLDOWN: f32 = 2.2;
const SAINTS_TESTAMENT_COOLDOWN: f32 = 2.2;
const LIVING_CODEX_COOLDOWN: f32 = 2.2;

/// Weapon abilities (right click) for every player character: this
/// machine's `Player` (read into its `PlayerIntent` by
/// `player::read_local_input`) and, on a multiplayer host, each joined
/// player's body (their client's presses, via `net::intent`). Every
/// ability aims with that player's own camera
/// (`PlayerIntent::aim_transform`), the same way `melee::player_basic_attack`
/// does for basic attacks.
pub(crate) fn weapon_ability_input(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spatial_query: SpatialQuery,
    ruin_state: Res<RuinState>,

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
            &Weapon,
            &mut PlayerCombat,
            &mut Health,
            Option<&mut AbilityUseTracker>,
            Option<&mut WeaponAbilityState>,
            &mut Dodge,
        ),
        (With<PlayerCharacter>, Without<Enemy>),
    >,

    mut enemy_query: Query<
        (&mut Transform, &mut Health, &mut Stagger, &mut EnemyCombat),
        (With<Enemy>, Without<Player>),
    >,
) {
    for (
        player_entity,
        mut player_transform,
        mut intent,
        weapon,
        mut combat,
        mut player_health,
        mut ability_tracker,
        mut weapon_ability_state,
        mut dodge,
    ) in &mut player_query
    {
        // A press (right click with the cursor captured, for the local
        // player) is consumed even when it can't cast, as `just_pressed`
        // was before.
        if !intent.take_ability() {
            continue;
        }

        // This player's camera: abilities aim from its position through
        // the centre of its view, exactly as they did with the local camera.
        let aim = intent.aim_transform();
        let camera_transform = &aim;

        if combat.ability_cooldown > 0.0 {
            continue;
        }

        if combat.pending_ability.is_some() {
            continue;
        }

        // Spellbook abilities don't fire at all -- no bolt, no cooldown
        // spent -- unless something is actually within range to cast at.
        // Every other weapon's ability has no such gate (a Ground Slam or a
        // Ricochet Arrow into empty space is still a normal, cooldown-
        // consuming cast); see the matching gate in `melee::player_basic_attack`.
        if weapon.weapon_type == WeaponType::Spellbook
            && !spellbook::any_enemy_in_range(player_transform.translation, weapon.range, &enemies)
        {
            continue;
        }

        match (weapon.weapon_type, weapon.tier) {
            // =================================================
            // COMMON LONGSWORD
            // =================================================
            (WeaponType::Longsword, GearTier::Common) => {
                longsword_lunging_slash(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &mut enemy_query,
                );

                combat.ability_cooldown = LONGSWORD_LUNGE_COOLDOWN;
            }

            // =================================================
            // COMMON GREATSWORD
            // =================================================
            (WeaponType::Greatsword, GearTier::Common) => {
                combat.ability_timer = GREATSWORD_CLEAVE_WINDUP;

                combat.pending_ability = Some(WeaponType::Greatsword);

                combat.ability_cooldown = GREATSWORD_CLEAVE_COOLDOWN;

                println!("Charging Greatsword Cleave!");
            }

            // =================================================
            // COMMON BOW
            // =================================================
            (WeaponType::Bow, GearTier::Common) => {
                bow::fire_piercing_shot(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &spatial_query,
                    player_entity,
                );

                combat.ability_cooldown = BOW_PIERCING_COOLDOWN;
            }

            // =================================================
            // COMMON HAMMER -- Ground Slam
            // =================================================
            (WeaponType::Hammer, GearTier::Common) => {
                combat.ability_timer = HAMMER_GROUND_SLAM_WINDUP;

                combat.pending_ability = Some(WeaponType::Hammer);

                combat.ability_cooldown = HAMMER_GROUND_SLAM_COOLDOWN;

                spawn_ground_slam_telegraph(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &player_transform,
                    HAMMER_GROUND_SLAM_WINDUP,
                );
            }

            // =================================================
            // COMMON CROSSBOW -- Rusted Hunter's Crossbow: Hunter's Bolt
            // =================================================
            (WeaponType::Crossbow, GearTier::Common) => {
                crossbow::fire_hunters_bolt(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &spatial_query,
                    player_entity,
                );

                combat.ability_cooldown = HUNTERS_BOLT_COOLDOWN;
            }

            // =================================================
            // COMMON SPELLBOOK -- Apprentice's Grimoire: Focused Cast
            // =================================================
            (WeaponType::Spellbook, GearTier::Common) => {
                spellbook::fire_focused_cast(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &spatial_query,
                    player_entity,
                    &enemies,
                );

                combat.ability_cooldown = FOCUSED_CAST_COOLDOWN;
            }

            // =================================================
            // COMMON DAGGERS -- Twin Fangs: Rapid Flurry
            // =================================================
            (WeaponType::Daggers, GearTier::Common) => {
                daggers::start_rapid_flurry(
                    &mut commands,
                    camera_transform,
                    &mut player_transform,
                    player_entity,
                    weapon,
                );

                combat.ability_cooldown = RAPID_FLURRY_COOLDOWN;
            }

            // =================================================
            // Rare Longsword
            // =================================================
            (WeaponType::Longsword, GearTier::Rare) => {
                longsword_crosscut(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &mut enemy_query,
                );

                combat.ability_cooldown = LONGSWORD_CROSSCUT_COOLDOWN;
            }
            // =================================================
            // Rare Greatsword
            // =================================================
            (WeaponType::Greatsword, GearTier::Rare) => {
                greatsword_executioners_sweep(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &mut enemy_query,
                );

                combat.ability_cooldown = GREATSWORD_EXECUTION_COOLDOWN;
            }

            // =================================================
            // Rare BOW
            // =================================================
            (WeaponType::Bow, GearTier::Rare) => {
                bow::fire_splinter_shot(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &spatial_query,
                    player_entity,
                );

                combat.ability_cooldown = BOW_SPLINTER_COOLDOWN;
            }

            // =================================================
            // Rare Hammer -- Seismic Strike
            // =================================================
            (WeaponType::Hammer, GearTier::Rare) => {
                hammer_seismic_strike(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                );

                combat.ability_cooldown = HAMMER_SEISMIC_STRIKE_COOLDOWN;
            }

            // =================================================
            // Rare CROSSBOW -- Gravewind Crossbow: Gravewind Bolt
            // =================================================
            (WeaponType::Crossbow, GearTier::Rare) => {
                crossbow::fire_gravewind_bolt(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &spatial_query,
                    player_entity,
                );

                combat.ability_cooldown = GRAVEWIND_BOLT_COOLDOWN;
            }

            // =================================================
            // Rare SPELLBOOK -- Ember Codex: Scorching Cast
            // =================================================
            (WeaponType::Spellbook, GearTier::Rare) => {
                spellbook::fire_scorching_cast(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &spatial_query,
                    player_entity,
                    &enemies,
                );

                combat.ability_cooldown = SCORCHING_CAST_COOLDOWN;
            }

            // =================================================
            // RARE DAGGERS -- Bloodletters: Serrated Rush
            // =================================================
            (WeaponType::Daggers, GearTier::Rare) => {
                daggers::start_serrated_rush(
                    &mut commands,
                    camera_transform,
                    &mut player_transform,
                    player_entity,
                    weapon,
                );

                combat.ability_cooldown = SERRATED_RUSH_COOLDOWN;
            }

            // =================================================
            // Epic Longsword
            // =================================================
            (WeaponType::Longsword, GearTier::Epic) => {
                start_blade_rush(
                    &mut commands,
                    camera_transform,
                    &mut player_transform,
                    player_entity,
                    weapon,
                );

                combat.ability_cooldown = LONGSWORD_BLADE_RUSH_COOLDOWN;
            }
            // =================================================
            // Epic Greatsword
            // =================================================
            (WeaponType::Greatsword, GearTier::Epic) => {
                fire_phantom_cleave(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                );

                combat.ability_cooldown = GREATSWORD_PHANTOM_CLEAVE_COOLDOWN;
            }

            (WeaponType::Bow, GearTier::Epic) => {
                bow::fire_ricochet_arrow(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &spatial_query,
                    player_entity,
                );

                combat.ability_cooldown = BOW_RICOCHET_COOLDOWN;
            }

            // =================================================
            // Epic Hammer -- Earthshatter
            // =================================================
            (WeaponType::Hammer, GearTier::Epic) => {
                hammer_earthshatter(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &player_transform,
                    weapon,
                );

                combat.ability_cooldown = HAMMER_EARTHSHATTER_COOLDOWN;
            }

            // =================================================
            // Epic CROSSBOW -- Siegebreaker Crossbow: Armorbreaker Shot
            // =================================================
            (WeaponType::Crossbow, GearTier::Epic) => {
                crossbow::fire_armorbreaker_shot(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &spatial_query,
                    player_entity,
                );

                combat.ability_cooldown = ARMORBREAKER_SHOT_COOLDOWN;
            }

            // =================================================
            // Epic SPELLBOOK -- Grimoire of Echoes: Echoing Cast
            // =================================================
            (WeaponType::Spellbook, GearTier::Epic) => {
                let Some(state) = weapon_ability_state.as_deref_mut() else {
                    println!(
                        "Echoing Cast needs WeaponAbilityState -- not yet initialized on this player."
                    );
                    continue;
                };

                spellbook::fire_echoing_cast(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    camera_transform,
                    &mut player_transform,
                    weapon,
                    &spatial_query,
                    player_entity,
                    &enemies,
                    state,
                );

                combat.ability_cooldown = ECHOING_CAST_COOLDOWN;
            }

            // =================================================
            // EPIC DAGGERS -- Phantom Blades: Phantom Step
            // =================================================
            (WeaponType::Daggers, GearTier::Epic) => {
                let Some(state) = weapon_ability_state.as_deref_mut() else {
                    println!(
                        "Phantom Step needs WeaponAbilityState -- not yet initialized on this player."
                    );
                    continue;
                };

                daggers::start_phantom_step(
                    &mut commands,
                    camera_transform,
                    &mut player_transform,
                    player_entity,
                    &spatial_query,
                    &mut dodge,
                    state,
                );

                combat.ability_cooldown = PHANTOM_STEP_COOLDOWN;
            }

            // =================================================
            // LEGENDARY DAGGERS -- Kingslayer / Widow's Kiss / Dancer in the Dark
            // =================================================
            (WeaponType::Daggers, GearTier::Legendary) => match weapon.metadata.definition_id {
                daggers::KINGSLAYER_ID => {
                    daggers::cast_deathmark(
                        &mut commands,
                        camera_transform,
                        player_entity,
                        &spatial_query,
                        &enemies,
                    );

                    combat.ability_cooldown = KINGSLAYER_DEATHMARK_COOLDOWN;
                }

                daggers::WIDOWS_KISS_ID => {
                    daggers::fire_venomous_barrage(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        camera_transform,
                        &mut player_transform,
                        weapon,
                    );

                    combat.ability_cooldown = WIDOWS_KISS_BARRAGE_COOLDOWN;
                }

                daggers::DANCER_IN_THE_DARK_ID => {
                    daggers::start_shadowstep_dance(
                        &mut commands,
                        &player_transform,
                        player_entity,
                        weapon,
                        &enemies,
                    );

                    combat.ability_cooldown = DANCER_SHADOWSTEP_COOLDOWN;
                }

                _ => {
                    println!("Unrecognized Legendary Daggers -- ability not implemented.");
                }
            },

            // =================================================
            // Legendary CROSSBOW -- Saint's Nail / The Ruinpiercer / Bellringer
            // =================================================
            (WeaponType::Crossbow, GearTier::Legendary) => match weapon.metadata.definition_id {
                crossbow::SAINTS_NAIL_ID => {
                    crossbow::fire_saints_nail(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        camera_transform,
                        &mut player_transform,
                        weapon,
                        &spatial_query,
                        player_entity,
                    );

                    combat.ability_cooldown = SAINTS_NAIL_COOLDOWN;
                }

                crossbow::RUINPIERCER_ID => {
                    crossbow::fire_ruinpiercer(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        camera_transform,
                        &mut player_transform,
                        weapon,
                        &spatial_query,
                        player_entity,
                        &ruin_state,
                    );

                    combat.ability_cooldown = RUINPIERCER_COOLDOWN;
                }

                crossbow::BELLRINGER_ID => {
                    crossbow::fire_bellringer(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        camera_transform,
                        &mut player_transform,
                        weapon,
                        &spatial_query,
                        player_entity,
                    );

                    combat.ability_cooldown = BELLRINGER_COOLDOWN;
                }

                _ => {
                    println!("Unrecognized Legendary Crossbow -- ability not implemented.");
                }
            },

            // =================================================
            // Legendary SPELLBOOK -- The Hollow Gospel / Saint's Testament /
            // The Living Codex
            // =================================================
            (WeaponType::Spellbook, GearTier::Legendary) => match weapon.metadata.definition_id {
                spellbook::HOLLOW_GOSPEL_ID => {
                    spellbook::fire_ruin_bolt(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        camera_transform,
                        &mut player_transform,
                        weapon,
                        &spatial_query,
                        player_entity,
                        &enemies,
                        &ruin_state,
                        &mut player_health,
                    );

                    combat.ability_cooldown = HOLLOW_GOSPEL_COOLDOWN;
                }

                spellbook::SAINTS_TESTAMENT_ID => {
                    spellbook::fire_grace_bolt(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        camera_transform,
                        &mut player_transform,
                        weapon,
                        &spatial_query,
                        player_entity,
                        &enemies,
                    );

                    combat.ability_cooldown = SAINTS_TESTAMENT_COOLDOWN;
                }

                spellbook::LIVING_CODEX_ID => {
                    let Some(state) = weapon_ability_state.as_deref_mut() else {
                        println!(
                            "Adaptive Cast needs WeaponAbilityState -- not yet initialized on this player."
                        );
                        continue;
                    };

                    spellbook::fire_adaptive_cast(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        camera_transform,
                        &mut player_transform,
                        weapon,
                        &spatial_query,
                        player_entity,
                        &enemies,
                        state,
                    );

                    combat.ability_cooldown = LIVING_CODEX_COOLDOWN;
                }

                _ => {
                    println!("Unrecognized Legendary Spellbook -- ability not implemented.");
                }
            },

            (_, GearTier::Legendary) => {
                println!("Legendary ability not implemented yet.");
            }
        }

        // Spellbook's Basic Arcana passive: every tier's ability cooldown is
        // scaled by `weapon.ability_cooldown_multiplier` (1.0 for every other
        // weapon family -- see each `Weapon::of_type` constructor). Applied
        // once here, after the match, so no individual Spellbook arm above
        // needs to remember to apply it itself.
        combat.ability_cooldown *= weapon.ability_cooldown_multiplier;

        // Every real arm above sets `ability_cooldown`; the Legendary
        // placeholder above is the only one that doesn't. This is the single
        // point that tells gear/ability-use effects (see
        // `weapons::armor_procs`) "an ability was actually cast this frame,"
        // without needing to touch every match arm individually.
        if combat.ability_cooldown > 0.0 {
            if let Some(tracker) = ability_tracker.as_deref_mut() {
                tracker.record_ability_used();
            }
        }
    }
}
