//! Data definitions for the three Ruinbound gear archetypes: Arcanist,
//! Striker, and Bulwark (see `RUINBOUND_GEAR_ARCHETYPES.md` at the project
//! root, which is the source of truth for names, slots, and effect text).
//!
//! Each archetype gets its own file (`arcanist.rs`, `striker.rs`,
//! `bulwark.rs`) so no single file has to hold all ~19 items per
//! archetype x 3 archetypes. This module only holds the shared
//! stat-scaling helper (`build`) that every archetype file calls into.
//!
//! Stat model (deliberately conservative, see the `GearEffect` doc comment
//! in `weapons/components.rs`): every piece gets a real, working
//! `defense`/`health_bonus` value (the only two stats the current combat
//! system -- `Equipment::total_defense`, `Equipment::total_health_bonus`,
//! `combat::apply_player_damage` -- actually reads). Every other stat or
//! proc described in the design doc (ability damage, attack speed,
//! barriers, stagger resistance, blocking, momentum, etc.) is attached as
//! descriptive `GearEffect` text only, since those systems don't exist in
//! Ruinbound yet. This keeps every one of the 57 new items lootable,
//! equippable, and mechanically meaningful today, while being explicit
//! that the unique procs are a separate future implementation pass.

pub mod arcanist;
pub mod bulwark;
pub mod striker;

use crate::weapons::{Armor, ArmorSlot, GearArchetype, GearEffect, GearTier};

pub(crate) use arcanist::create as create_arcanist;
pub(crate) use bulwark::create as create_bulwark;
pub(crate) use striker::create as create_striker;

/// Stat multiplier applied to an archetype's per-slot Common-tier base
/// budget to produce Rare / Epic / Legendary values.
/// Common x1.0, Rare x1.6, Epic x2.3, Legendary x3.2.
///
/// These multipliers (and every base budget in the three archetype files)
/// are a first pass, exactly like the Saint's Fist Slam numbers -- tune
/// through playtesting, not meant to be final.
const TIER_MULTIPLIER: [f32; 4] = [1.0, 1.6, 2.3, 3.2];

fn tier_index(tier: GearTier) -> usize {
    match tier {
        GearTier::Common => 0,
        GearTier::Rare => 1,
        GearTier::Epic => 2,
        GearTier::Legendary => 3,
    }
}

/// Scales one base stat value by the current tier's multiplier, rounded to
/// one decimal place so displayed numbers stay clean (e.g. 12.8, not
/// 12.799999).
fn scale(base: f32, tier: GearTier) -> f32 {
    (base * TIER_MULTIPLIER[tier_index(tier)] * 10.0).round() / 10.0
}

/// Shared constructor used by every item in `arcanist.rs`, `striker.rs`,
/// and `bulwark.rs`. Applies the tier multiplier to the archetype's
/// Common-tier base defense/health budget for this slot, then builds the
/// `Armor` via `Armor::from_gear`.
pub(crate) fn build(
    definition_id: &'static str,
    name: &'static str,
    slot: ArmorSlot,
    tier: GearTier,
    archetype: GearArchetype,
    base_defense: f32,
    base_health: f32,
    effect: GearEffect,
) -> Armor {
    Armor::from_gear(
        definition_id,
        name,
        slot,
        tier,
        archetype,
        scale(base_defense, tier),
        scale(base_health, tier),
        effect,
    )
}
