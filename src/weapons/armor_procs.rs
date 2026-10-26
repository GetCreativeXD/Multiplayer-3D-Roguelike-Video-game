//! Wires specific gear pieces into the generic barrier / death-prevention
//! systems from `crate::combat`. Everything item-specific lives here so
//! `combat::apply_player_damage` and `combat::Barriers` never need to know
//! any item's name.
//!
//! Each proc below only checks two things: is the relevant piece equipped
//! right now (`Equipment::has_armor_piece`), and is its own cooldown
//! ready. Equipping or unequipping gear takes effect automatically because
//! these systems re-check `Equipment` every frame -- nothing is "locked
//! in" at cast time. They run for every player character, each with their
//! own gear, ability uses, barriers and cooldowns (multiplayer: Milestone 7
//! step 3).
//!
//! Wired here:
//! - Runebound Robes (Rare Arcanist chest, see `loot::gear::arcanist`):
//!   using an ability grants a small barrier.
//! - Robes of the Hollow Star (Legendary Arcanist chest): using an
//!   ability while critically wounded grants a larger barrier, on its own
//!   cooldown. Its other half -- "ability damage increases as health
//!   decreases" -- is intentionally NOT wired here: Ruinbound has no
//!   ability-damage stat yet for it to scale.
//! - Crown of the Unbroken (Legendary Bulwark head, see
//!   `loot::gear::bulwark`): fatal damage instead leaves the player at 1
//!   health and grants a barrier, on a long cooldown. Plugs into
//!   `combat::DeathPrevention` rather than reacting to ability use.
//!
//! Explicitly NOT wired yet:
//! - Spellweaver Grips ("Overflowing Power", Epic Arcanist hands): depends
//!   on an ability-resource pool that does not exist in the project yet.
//!   Its `GearEffect` text is in place; there is no proc for it here.

use bevy::prelude::*;

use crate::characters::class_abilities::AbilityUseTracker;
use crate::combat::{BarrierSource, Barriers, DeathPrevention, Health};
use crate::loot::{CROWN_OF_THE_UNBROKEN, ROBES_OF_THE_HOLLOW_STAR, RUNEBOUND_ROBES};
use crate::player::PlayerCharacter;

use super::components::Equipment;

// =========================================================
// TUNING (experimental -- expect these to move during playtesting)
// =========================================================

const RUNEBOUND_ROBES_BARRIER: f32 = 15.0;

const HOLLOW_STAR_BARRIER: f32 = 35.0;
const HOLLOW_STAR_COOLDOWN: f32 = 12.0;
/// "Critically wounded" for Robes of the Hollow Star's trigger: health at
/// or below this fraction of max.
const HOLLOW_STAR_HEALTH_THRESHOLD: f32 = 0.3;

/// Crown of the Unbroken's barrier, as a fraction of the player's max
/// health at the moment it triggers.
const UNBROKEN_BARRIER_FRACTION: f32 = 0.3;
const UNBROKEN_COOLDOWN: f32 = 60.0;

/// Per-player bookkeeping the ability-triggered procs above need (which
/// ability-use count they last reacted to, their own cooldowns). Kept
/// separate from `AbilityUseTracker` so gear-proc bookkeeping doesn't leak
/// into the generic tracker every future ability consumer reads.
#[derive(Component, Default)]
pub(crate) struct ArmorProcState {
    observed_ability_uses: u64,
    hollow_star_cooldown: f32,
}

/// Adds `ArmorProcState` to every player character (this machine's
/// `Player` and, on a multiplayer host, each joined player's body), the
/// same pattern used for the other shared per-player trackers.
pub(crate) fn initialize_armor_proc_state(
    mut commands: Commands,
    players: Query<Entity, (With<PlayerCharacter>, Without<ArmorProcState>)>,
) {
    for player in &players {
        commands.entity(player).insert(ArmorProcState::default());
    }
}

pub(crate) fn tick_armor_proc_cooldowns(time: Res<Time>, mut players: Query<&mut ArmorProcState>) {
    let delta = time.delta_secs();
    for mut state in &mut players {
        state.hollow_star_cooldown = (state.hollow_star_cooldown - delta).max(0.0);
    }
}

/// Runebound Robes and Robes of the Hollow Star both react to "an ability
/// was just used," read off the same `AbilityUseTracker` edge (mirrors how
/// Vanguard's Momentum watches `BasicAttackHitTracker` in
/// `characters::vanguard_abilities`).
pub(crate) fn ability_use_barrier_procs(
    mut players: Query<
        (
            &Equipment,
            &Health,
            &AbilityUseTracker,
            &mut ArmorProcState,
            &mut Barriers,
        ),
        With<PlayerCharacter>,
    >,
) {
    for (equipment, health, tracker, mut state, mut barriers) in &mut players {
        let new_uses = tracker
            .total_used()
            .saturating_sub(state.observed_ability_uses);
        state.observed_ability_uses = tracker.total_used();

        if new_uses == 0 {
            continue;
        }

        if equipment.has_armor_piece(RUNEBOUND_ROBES) {
            barriers.grant_default(BarrierSource::RuneboundRobes, RUNEBOUND_ROBES_BARRIER);
        }

        if equipment.has_armor_piece(ROBES_OF_THE_HOLLOW_STAR) {
            let critically_wounded = health.current <= health.max * HOLLOW_STAR_HEALTH_THRESHOLD;

            if critically_wounded && state.hollow_star_cooldown <= 0.0 {
                barriers.grant_default(BarrierSource::RobesOfTheHollowStar, HOLLOW_STAR_BARRIER);
                state.hollow_star_cooldown = HOLLOW_STAR_COOLDOWN;
            }
        }
    }
}

/// Keeps the shared, gear-agnostic `DeathPrevention` component in sync
/// with whether Crown of the Unbroken is currently equipped. `combat`
/// itself never references this item -- it only ever asks
/// `DeathPrevention::is_active_and_ready`.
pub(crate) fn sync_unbroken_death_prevention(
    mut players: Query<(&Equipment, &mut DeathPrevention), With<PlayerCharacter>>,
) {
    for (equipment, mut death_prevention) in &mut players {
        if equipment.has_armor_piece(CROWN_OF_THE_UNBROKEN) {
            death_prevention.activate(UNBROKEN_BARRIER_FRACTION, UNBROKEN_COOLDOWN);
        } else {
            death_prevention.deactivate();
        }
    }
}
