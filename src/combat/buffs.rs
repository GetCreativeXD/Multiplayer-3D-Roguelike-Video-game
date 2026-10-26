//! Generic, reusable temporary combat buffs (movement speed / outgoing
//! damage multipliers), granted by a class ability, gear proc, or
//! consumable.
//!
//! This deliberately mirrors `combat::barrier`: `StatBuffs` holds a list
//! of independently-timed entries tagged by `BuffSource`, multiple
//! sources can be active at once, and each entry expires on its own after
//! its `duration` even if nothing removes it early. Nothing in this file
//! knows about any specific ability -- today only Warden of Shades'
//! Guardian Otter grants one (see `characters::warden_abilities`), but
//! any current or future class ability, gear effect, or consumable can
//! reuse the same component the same way `combat::Barriers` is reused by
//! multiple gear procs.
//!
//! `StatBuffs` is intentionally separate from
//! `characters::class_abilities::CharacterCombatModifiers`, which is
//! "supplied by the currently selected class" (e.g. Vanguard's own
//! Momentum stacks). `StatBuffs` is for buffs *granted onto* a player from
//! an outside source -- including, in multiplayer, a teammate of a
//! different class -- so it never conflicts with a class's own
//! every-frame recompute of its own modifiers.

use bevy::prelude::*;

/// Where an active `StatBuffs` entry came from. Purely informational
/// today (HUD/debugging); `StatBuffs::grant` also uses it to refresh an
/// existing entry from the same source instead of stacking duplicates.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuffSource {
    /// Warden of Shades' Guardian Otter (see
    /// `characters::warden_abilities`): damage and movement-speed buff
    /// for the blessed player.
    GuardianOtter,
    /// Gravemind's Embrace the Ruin (see
    /// `characters::gravemind_abilities`): while active, the caster's own
    /// basic attacks are strengthened, scaling with the Embrace meter's
    /// current fill plus any banked stacks.
    EmbraceTheRuin,
    /// Anything else: a future ability, a debug command, a test.
    Other,
}

#[derive(Clone, Copy, Debug)]
struct StatBuff {
    source: BuffSource,
    movement_speed_multiplier: f32,
    damage_multiplier: f32,
    remaining: f32,
}

/// All temporary stat buffs currently active on an entity. Add this
/// component to any entity that should be able to receive one -- today
/// that's the player, initialized alongside `Health`/`Barriers` in
/// `combat::initialize_combat_defenses`.
#[derive(Component, Default, Debug)]
pub struct StatBuffs {
    active: Vec<StatBuff>,
}

impl StatBuffs {
    /// Grants (or refreshes) a buff from `source`. If `source` already has
    /// an active entry, it is replaced rather than stacked, so recasting
    /// the same ability on the same target refreshes duration/strength
    /// instead of compounding.
    pub fn grant(
        &mut self,
        source: BuffSource,
        movement_speed_multiplier: f32,
        damage_multiplier: f32,
        duration: f32,
    ) {
        if duration <= 0.0 {
            return;
        }

        self.active.retain(|buff| buff.source != source);
        self.active.push(StatBuff {
            source,
            movement_speed_multiplier,
            damage_multiplier,
            remaining: duration,
        });
    }

    /// Combined movement-speed multiplier from every active buff (product
    /// of each entry's multiplier). `1.0` when nothing is active.
    pub fn movement_speed_multiplier(&self) -> f32 {
        self.active
            .iter()
            .map(|buff| buff.movement_speed_multiplier)
            .product()
    }

    /// Combined outgoing-damage multiplier from every active buff
    /// (product of each entry's multiplier). `1.0` when nothing is
    /// active. Currently read by weapon basic attacks
    /// (`weapons::melee`, `weapons::bow`) -- see the doc comment on
    /// `weapons::melee::player_basic_attack`.
    pub fn damage_multiplier(&self) -> f32 {
        self.active
            .iter()
            .map(|buff| buff.damage_multiplier)
            .product()
    }

    /// True if a buff from `source` is currently active. Lets HUD/ability
    /// code answer "is my own buff still active on this target" without
    /// duplicating a manual scan.
    pub fn has_source(&self, source: BuffSource) -> bool {
        self.active.iter().any(|buff| buff.source == source)
    }

    fn tick(&mut self, delta: f32) {
        for buff in self.active.iter_mut() {
            buff.remaining -= delta;
        }
        self.active.retain(|buff| buff.remaining > 0.0);
    }
}

/// Ages every entity's stat buffs and removes ones whose duration
/// expired. Registered by `combat::CombatPlugin`.
pub fn tick_stat_buffs(time: Res<Time>, mut buff_holders: Query<&mut StatBuffs>) {
    let delta = time.delta_secs();
    for mut buffs in &mut buff_holders {
        buffs.tick(delta);
    }
}
