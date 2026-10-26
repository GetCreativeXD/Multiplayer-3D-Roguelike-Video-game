//! Generic, reusable temporary combat debuffs applied *to* an enemy --
//! the mirror image of `combat::buffs` (which grants a buff *to* a
//! player). `EnemyDebuffs` holds a list of independently-timed entries
//! tagged by `DebuffSource`; multiple sources can be active at once, and
//! each entry expires on its own after its `duration` even if nothing
//! removes it early -- the same shape as `combat::Barriers` and
//! `combat::StatBuffs`.
//!
//! Nothing in this file knows about any specific ability. Today only
//! Runesage's Grand Rune grants one -- the "Runebroken" debuff described
//! in the project context doc, section 20 (increased damage and stagger
//! taken) -- but any current or future class ability, gear effect, or
//! boss mechanic that wants to weaken an enemy's defenses can reuse the
//! same component. Every enemy gets one for free, empty, from
//! `enemies::spawn_enemy_root`, so it costs nothing until something
//! actually grants a debuff.

use bevy::prelude::*;

/// Where an active `EnemyDebuffs` entry came from. Purely informational
/// today (HUD/debugging); `EnemyDebuffs::grant` also uses it to refresh
/// an existing entry from the same source instead of stacking duplicates.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DebuffSource {
    /// Runesage's Grand Rune (see `characters::runesage_abilities`): the
    /// "Runebroken" debuff -- increased damage and stagger taken.
    GrandRune,
    /// Siegebreaker Crossbow's Armorbreaker Shot (Epic Crossbow -- see
    /// `weapons::crossbow`): "fully charged shots reduce the target's
    /// defenses briefly." Enemies don't have a separate defense stat the
    /// way the player does, so this is implemented as the same kind of
    /// increased-damage-taken window Runebroken already grants.
    ArmorbreakerShot,
    /// Saint's Nail's Consecrated Bolt (Legendary Crossbow -- see
    /// `weapons::crossbow`): "Marked enemies take bonus damage from
    /// abilities." Implemented as a general increased-damage-taken
    /// window (see the `ArmorbreakerShot` doc comment above for why) --
    /// `EnemyDebuffs::has_source` also doubles as "is this enemy
    /// currently Marked" for Saint's Nail's on-death holy burst.
    ConsecratedMark,
    /// Widow's Kiss's Envenomed state (Legendary Daggers -- see
    /// `weapons::daggers`): WEAPONS.md's "Venom and Bleed together cause
    /// the target to become Envenomed, briefly slowing it." Granted the
    /// moment an enemy carries both an active `combat::DotSource::Bleed`
    /// and `combat::DotSource::Venom` stack; reuses this generic debuff
    /// component's `movement_speed_multiplier` rather than inventing a
    /// dedicated slow effect.
    Envenomed,
    /// Anything else: a future ability, a debug command, a test.
    Other,
}

#[derive(Clone, Copy, Debug)]
struct EnemyDebuff {
    source: DebuffSource,
    damage_taken_multiplier: f32,
    stagger_resistance_reduction: u32,
    /// `1.0` (no change) for every debuff except Envenomed. Kept on
    /// every entry, not just Envenomed's, so a future debuff that also
    /// wants to slow (or speed up) an enemy can reuse this field instead
    /// of the component growing a second parallel list.
    movement_speed_multiplier: f32,
    remaining: f32,
}

/// All temporary debuffs currently weakening an enemy. Every enemy
/// archetype receives an empty one at spawn (`enemies::spawn_enemy_root`)
/// so any current or future debuff source has somewhere to write into.
#[derive(Component, Default, Debug)]
pub struct EnemyDebuffs {
    active: Vec<EnemyDebuff>,
}

impl EnemyDebuffs {
    /// Grants (or refreshes) a debuff from `source`. If `source` already
    /// has an active entry, it is replaced rather than stacked, so
    /// recasting the same ability on the same enemy refreshes
    /// duration/strength instead of compounding indefinitely.
    pub fn grant(
        &mut self,
        source: DebuffSource,
        damage_taken_multiplier: f32,
        stagger_resistance_reduction: u32,
        duration: f32,
    ) {
        self.grant_with_slow(
            source,
            damage_taken_multiplier,
            stagger_resistance_reduction,
            1.0,
            duration,
        );
    }

    /// Same as `grant`, but also sets a movement-speed multiplier for this
    /// debuff -- Widow's Kiss's Envenomed (see `DebuffSource::Envenomed`)
    /// is the only current caller; every other existing call site keeps
    /// going through `grant`, which passes `1.0` (no slow) here.
    pub fn grant_with_slow(
        &mut self,
        source: DebuffSource,
        damage_taken_multiplier: f32,
        stagger_resistance_reduction: u32,
        movement_speed_multiplier: f32,
        duration: f32,
    ) {
        if duration <= 0.0 {
            return;
        }

        self.active.retain(|debuff| debuff.source != source);
        self.active.push(EnemyDebuff {
            source,
            damage_taken_multiplier,
            stagger_resistance_reduction,
            movement_speed_multiplier,
            remaining: duration,
        });
    }

    /// Combined incoming-damage multiplier from every active debuff
    /// (product of each entry's multiplier). `1.0` when nothing is
    /// active -- read by `combat::apply_enemy_damage`.
    pub fn damage_taken_multiplier(&self) -> f32 {
        self.active
            .iter()
            .map(|debuff| debuff.damage_taken_multiplier)
            .product()
    }

    /// How much lower this enemy's effective stagger resistance is right
    /// now. Uses the single strongest active reduction rather than
    /// summing, so stacking the same kind of debuff from multiple sources
    /// can't zero out a boss's resistance entirely. `0` when nothing is
    /// active -- read by `combat::effective_stagger_resistance`.
    pub fn stagger_resistance_reduction(&self) -> u32 {
        self.active
            .iter()
            .map(|debuff| debuff.stagger_resistance_reduction)
            .max()
            .unwrap_or(0)
    }

    /// Combined movement-speed multiplier from every active debuff
    /// (product of each entry's multiplier). `1.0` when nothing is
    /// active -- see `enemy_debuff_movement_speed_multiplier` for the
    /// `Option`-friendly free-function wrapper enemy archetypes read.
    pub fn movement_speed_multiplier(&self) -> f32 {
        self.active
            .iter()
            .map(|debuff| debuff.movement_speed_multiplier)
            .product()
    }

    /// True if a debuff from `source` is currently active.
    pub fn has_source(&self, source: DebuffSource) -> bool {
        self.active.iter().any(|debuff| debuff.source == source)
    }

    fn tick(&mut self, delta: f32) {
        for debuff in self.active.iter_mut() {
            debuff.remaining -= delta;
        }
        self.active.retain(|debuff| debuff.remaining > 0.0);
    }
}

/// Ages every enemy's debuffs and removes ones whose duration expired.
/// Registered by `combat::CombatPlugin`.
pub fn tick_enemy_debuffs(time: Res<Time>, mut debuff_holders: Query<&mut EnemyDebuffs>) {
    let delta = time.delta_secs();
    for mut debuffs in &mut debuff_holders {
        debuffs.tick(delta);
    }
}

/// Convenience for any call site that only has `Option<&EnemyDebuffs>` (an
/// enemy without the component is simply unslowed). Mirrors
/// `combat::enemy_buffs::enemy_movement_speed_multiplier` -- an enemy's
/// final effective speed multiplies both this (debuffs, e.g. Envenomed)
/// and that one (buffs, e.g. a Corrupted Banner aura) together.
pub fn enemy_debuff_movement_speed_multiplier(debuffs: Option<&EnemyDebuffs>) -> f32 {
    debuffs.map_or(1.0, EnemyDebuffs::movement_speed_multiplier)
}
