//! Generic, reusable damage-over-time affliction -- e.g. the Ember Codex
//! spellbook's Scorch (WEAPONS.md: "6 fire damage per second for 4
//! seconds... reapplying refreshes its duration instead of stacking").
//! Mirrors `combat::enemy_debuffs` / `combat::barrier` in shape: a
//! component holding independently-timed stacks tagged by source, ticked
//! once per frame, with reapplication from the same source refreshing
//! rather than stacking. Any current or future burn/poison/bleed effect
//! should reuse this component instead of inventing its own timer.
//!
//! Unlike `EnemyDebuffs` (which only ever *multiplies* damage another hit
//! deals), this component deals damage on its own each tick -- see
//! `tick_damage_over_time`.

use bevy::prelude::*;

use crate::enemies::Enemy;

use super::{EnemyDebuffs, Health, apply_enemy_damage};

/// Where an active `DamageOverTime` stack came from. Add a variant per
/// notable grantor as they're wired up, the same convention
/// `combat::enemy_debuffs::DebuffSource` uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DotSource {
    /// Ember Codex (Rare Spellbook) -- see `weapons::spellbook`.
    Scorch,
    /// Bleed -- see `weapons::daggers`. Applied by Bloodletters' Serrated
    /// Rush (Rare Daggers) and Widow's Kiss's Venomous Barrage (Legendary
    /// Daggers). Combining an active Bleed stack with an active Venom
    /// stack on the same enemy is what `weapons::daggers` reads as
    /// "Envenomed" (see `combat::enemy_debuffs::DebuffSource::Envenomed`).
    Bleed,
    /// Venom -- see `weapons::daggers`. Applied by Widow's Kiss's
    /// Venomous Barrage (Legendary Daggers).
    Venom,
    /// Anything else: a future ability, a debug command, a test.
    Other,
}

#[derive(Clone, Copy, Debug)]
struct DotStack {
    source: DotSource,
    damage_per_second: f32,
    remaining: f32,
}

/// All damage-over-time afflictions currently ticking on an entity. Not
/// every enemy has one -- unlike `EnemyDebuffs`, this is inserted lazily
/// (via `with_stack`) only once something actually afflicts the entity,
/// so it costs nothing for the common case of an enemy that's never been
/// burned.
#[derive(Component, Debug, Default)]
pub struct DamageOverTime {
    active: Vec<DotStack>,
}

impl DamageOverTime {
    /// Builds a fresh component already carrying one stack -- the shape
    /// needed when inserting this component onto an entity for the first
    /// time (see `weapons::spellbook`'s Scorch application), so the very
    /// first tick of damage isn't lost to a frame of latency.
    pub fn with_stack(source: DotSource, damage_per_second: f32, duration: f32) -> Self {
        let mut dot = Self::default();
        dot.apply(source, damage_per_second, duration);
        dot
    }

    /// Grants (or refreshes) a stack from `source`. If `source` already
    /// has an active stack, it is replaced rather than stacked, matching
    /// WEAPONS.md's "reapplying Scorch refreshes its duration instead of
    /// stacking the effect."
    pub fn apply(&mut self, source: DotSource, damage_per_second: f32, duration: f32) {
        if duration <= 0.0 {
            return;
        }

        self.active.retain(|stack| stack.source != source);
        self.active.push(DotStack {
            source,
            damage_per_second,
            remaining: duration,
        });
    }

    /// True if a stack from `source` is currently active. Lets an on-hit
    /// effect ask "is this enemy already Bleeding/Scorched/etc." without
    /// duplicating a manual scan -- e.g. Widow's Kiss's "Bleeding enemies
    /// take increased damage from basic dagger attacks" (see
    /// `weapons::daggers`).
    pub fn has_source(&self, source: DotSource) -> bool {
        self.active.iter().any(|stack| stack.source == source)
    }

    fn tick(&mut self, delta: f32, health: &mut Health, debuffs: Option<&EnemyDebuffs>) {
        for stack in self.active.iter_mut() {
            apply_enemy_damage(health, debuffs, stack.damage_per_second * delta);
            stack.remaining -= delta;
        }
        self.active.retain(|stack| stack.remaining > 0.0);
    }
}

/// Ticks every afflicted enemy's damage-over-time stacks, dealing damage
/// through the same `apply_enemy_damage` path every other enemy-damage
/// source uses (so Runebroken etc. still apply). Registered by
/// `combat::CombatPlugin`.
pub fn tick_damage_over_time(
    time: Res<Time>,
    mut afflicted: Query<(&mut DamageOverTime, &mut Health, Option<&EnemyDebuffs>), With<Enemy>>,
) {
    let delta = time.delta_secs();
    for (mut dot, mut health, debuffs) in &mut afflicted {
        dot.tick(delta, &mut health, debuffs);
    }
}
