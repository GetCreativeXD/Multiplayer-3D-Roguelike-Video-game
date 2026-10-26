//! Numeric run-wide effects granted by starting relics and run pacts
//! (see `run_setup::catalog`'s `RELICS`/`PACTS`), and `RunModifiers` --
//! the single resource every consuming system reads instead of matching
//! on `RelicId`/`PactId` itself.
//!
//! # Why this stays reusable for future relics/pacts
//!
//! A relic or pact catalog entry lists the `RunEffect`s it grants
//! (`RelicEntry::effects` / `PactEntry::effects`); `RunModifiers::from_selections`
//! folds every currently-active entry's effects into one resource. Adding
//! a new relic or pact that reuses an existing effect (say, another
//! source of `RareDropChanceBonus`) needs no new code in this file at all
//! -- only a new catalog entry. Only a genuinely new *kind* of effect
//! (one none of the existing variants can express) needs a new
//! `RunEffect` variant, one new field on `RunModifiers`, and one new
//! match arm in `apply` -- every consuming system stays untouched.
//!
//! # What each field is actually wired into today
//!
//! - `bonus_ability_power` -- added into
//!   `weapons::Equipment::total_ability_power_bonus` for the player's
//!   starting `Equipment` (see `weapons::inventory::initialize_player_inventory`).
//! - `bonus_max_health` -- added to the player's starting `BaseHealth`
//!   (see `rooms::fallen_gate::setup_fallen_gate_player`).
//! - `dodge_cooldown_multiplier` -- multiplies dodge recovery time (see
//!   `player::start_dodge`).
//! - `rare_drop_chance_bonus` -- chance a Common-tier loot roll re-rolls
//!   against the same table's Rare-or-better entries (see `loot::drops`).
//! - `ruin_gain_multiplier` -- multiplies passive Ruin growth (see
//!   `ruin::advance_run_clock_and_ruin`).
//! - `enemy_strength_multiplier` -- multiplies every enemy's max health
//!   the moment it spawns (see `enemies::apply_enemy_strength_modifier`).
//!   Only health is scaled today; no pact or relic changes enemy damage
//!   output yet.
//! - `healing_multiplier` -- multiplies healing the player receives
//!   through `combat::apply_player_healing` (today, only Gravemind's
//!   Consume goes through it -- see `characters::gravemind_abilities`).
//!
//! Two pact clauses have no system to attach to yet and are intentionally
//! left unimplemented rather than inventing one (see each `PactEntry`'s
//! own doc comment in `catalog`): Fragile Sanctuary's "shops are
//! cheaper", and Scarce Fortune's "fewer item choices are offered" (there
//! is no shop or reward-choice-offer system in the game yet).

use bevy::prelude::*;

use super::catalog::{self, PactId, RelicId};

/// One numeric effect a relic or pact catalog entry grants. See this
/// module's doc comment for exactly what each variant is wired into.
#[derive(Clone, Copy, Debug)]
pub enum RunEffect {
    /// Flat Ability Power, stacking with any granted by equipped armor
    /// (`weapons::Armor::ability_power_bonus`) through the same total.
    BonusAbilityPower(f32),
    /// Flat bonus to starting max health.
    BonusMaxHealth(f32),
    /// Multiplies dodge recovery time. Less than 1.0 recovers faster.
    DodgeCooldownMultiplier(f32),
    /// Added to the chance a Common-tier loot roll upgrades to Rare or
    /// better. Multiple sources stack additively.
    RareDropChanceBonus(f32),
    /// Multiplies passive Ruin growth. Greater than 1.0 rises faster.
    RuinGainMultiplier(f32),
    /// Multiplies every enemy's max health at spawn.
    EnemyStrengthMultiplier(f32),
    /// Multiplies healing the player receives.
    HealingMultiplier(f32),
}

/// The resolved, run-wide numeric effect of every active relic and pact,
/// computed once when the run begins (see
/// `ui::run_setup_screen::handle_begin_run_click`) and read for the rest
/// of the run by every system listed in this module's doc comment.
#[derive(Resource, Clone, Copy, Debug)]
pub struct RunModifiers {
    pub bonus_ability_power: f32,
    pub bonus_max_health: f32,
    pub dodge_cooldown_multiplier: f32,
    pub rare_drop_chance_bonus: f32,
    pub ruin_gain_multiplier: f32,
    pub enemy_strength_multiplier: f32,
    pub healing_multiplier: f32,
}

impl RunModifiers {
    /// No relics, no pacts -- every multiplier neutral, every bonus zero.
    /// Also what a fresh run-setup screen starts with, so any system that
    /// reads `RunModifiers` before "Begin Run" is clicked sees a safe,
    /// inert default rather than a missing resource.
    pub fn neutral() -> Self {
        Self {
            bonus_ability_power: 0.0,
            bonus_max_health: 0.0,
            dodge_cooldown_multiplier: 1.0,
            rare_drop_chance_bonus: 0.0,
            ruin_gain_multiplier: 1.0,
            enemy_strength_multiplier: 1.0,
            healing_multiplier: 1.0,
        }
    }

    fn apply(&mut self, effect: RunEffect) {
        match effect {
            RunEffect::BonusAbilityPower(bonus) => self.bonus_ability_power += bonus,
            RunEffect::BonusMaxHealth(bonus) => self.bonus_max_health += bonus,
            RunEffect::DodgeCooldownMultiplier(multiplier) => {
                self.dodge_cooldown_multiplier *= multiplier;
            }
            RunEffect::RareDropChanceBonus(bonus) => self.rare_drop_chance_bonus += bonus,
            RunEffect::RuinGainMultiplier(multiplier) => self.ruin_gain_multiplier *= multiplier,
            RunEffect::EnemyStrengthMultiplier(multiplier) => {
                self.enemy_strength_multiplier *= multiplier;
            }
            RunEffect::HealingMultiplier(multiplier) => self.healing_multiplier *= multiplier,
        }
    }

    /// Folds every effect granted by the given relics and pacts into one
    /// `RunModifiers`. Called once, when the run begins.
    pub fn from_selections(relics: &[RelicId], pacts: &[PactId]) -> Self {
        let mut modifiers = Self::neutral();
        for &id in relics {
            for &effect in catalog::relic_entry(id).effects {
                modifiers.apply(effect);
            }
        }
        for &id in pacts {
            for &effect in catalog::pact_entry(id).effects {
                modifiers.apply(effect);
            }
        }
        modifiers
    }
}

impl Default for RunModifiers {
    fn default() -> Self {
        Self::neutral()
    }
}
