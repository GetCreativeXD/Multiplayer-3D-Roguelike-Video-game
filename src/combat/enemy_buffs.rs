//! Generic, reusable temporary combat buffs granted *to* an enemy by
//! another enemy or a spawned object -- the mirror image of
//! `combat::buffs` (which grants a buff to a player) and structurally
//! identical to `combat::enemy_debuffs` (which weakens an enemy instead
//! of strengthening one). `EnemyBuffs` holds a list of independently-timed
//! entries tagged by `EnemyBuffSource`; multiple sources can be active at
//! once, and each entry expires on its own after its `duration` even if
//! nothing removes it early.
//!
//! Nothing in this file knows about any specific ability. Today only the
//! Black Banner Knight's planted Corrupted Banner grants one, through the
//! generic `enemies::aura::AuraEmitter` (see `enemies::knight`), but the
//! roster's Bell Husk ("temporarily increases nearby enemies' movement and
//! attack speed", with an explicit "should not stack without an
//! intentional cap" requirement) is the exact same shape and should reuse
//! this component and `AuraEmitter` rather than inventing its own. Every
//! enemy gets one for free, empty, from `enemies::spawn_enemy_root`, so it
//! costs nothing until something actually grants a buff.
//!
//! Consumption is opt-in per archetype: an archetype's own combat system
//! reads `EnemyBuffs::movement_speed_multiplier()` /
//! `damage_multiplier()` wherever it computes its own speed or outgoing
//! damage. Today that's `enemies::melee` (Ruinbound Footman),
//! `enemies::hound` (Cinder Hound), and `enemies::ranged` (Gatebow
//! Marksman) -- the three normal enemies a banner is meant to strengthen.
//! `enemies::saint` deliberately does not read it; a boss encounter
//! sharing a room with a Black Banner Knight is not a currently supported
//! pairing, and wiring it in unverified was judged riskier than leaving a
//! disclosed gap.

use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyBuffSource {
    /// The Black Banner Knight's planted Corrupted Banner (see
    /// `enemies::knight`): movement and damage buff for enemies standing
    /// in its aura.
    CorruptedBanner,
    /// Anything else: a future ability (e.g. Bell Husk), a debug command, a test.
    Other,
}

#[derive(Clone, Copy, Debug)]
struct EnemyBuff {
    source: EnemyBuffSource,
    movement_speed_multiplier: f32,
    damage_multiplier: f32,
    remaining: f32,
}

/// All temporary buffs currently strengthening an enemy. Every enemy
/// archetype receives an empty one at spawn (`enemies::spawn_enemy_root`)
/// so any current or future buff source has somewhere to write into.
#[derive(Component, Default, Debug)]
pub struct EnemyBuffs {
    active: Vec<EnemyBuff>,
}

impl EnemyBuffs {
    /// Grants (or refreshes) a buff from `source`. If `source` already has
    /// an active entry, it is replaced rather than stacked, so standing in
    /// range of the same aura (or two auras from the same source) refreshes
    /// duration/strength instead of compounding indefinitely.
    pub fn grant(
        &mut self,
        source: EnemyBuffSource,
        movement_speed_multiplier: f32,
        damage_multiplier: f32,
        duration: f32,
    ) {
        if duration <= 0.0 {
            return;
        }

        self.active.retain(|buff| buff.source != source);
        self.active.push(EnemyBuff {
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

    /// Combined outgoing-damage multiplier from every active buff (product
    /// of each entry's multiplier). `1.0` when nothing is active.
    pub fn damage_multiplier(&self) -> f32 {
        self.active
            .iter()
            .map(|buff| buff.damage_multiplier)
            .product()
    }

    /// True if a buff from `source` is currently active.
    pub fn has_source(&self, source: EnemyBuffSource) -> bool {
        self.active.iter().any(|buff| buff.source == source)
    }

    fn tick(&mut self, delta: f32) {
        for buff in self.active.iter_mut() {
            buff.remaining -= delta;
        }
        self.active.retain(|buff| buff.remaining > 0.0);
    }
}

/// Convenience for any call site that only has `Option<&EnemyBuffs>` (an
/// enemy without the component is simply unbuffed).
pub fn enemy_movement_speed_multiplier(buffs: Option<&EnemyBuffs>) -> f32 {
    buffs.map_or(1.0, EnemyBuffs::movement_speed_multiplier)
}

/// See `enemy_movement_speed_multiplier`.
pub fn enemy_damage_multiplier(buffs: Option<&EnemyBuffs>) -> f32 {
    buffs.map_or(1.0, EnemyBuffs::damage_multiplier)
}

/// Ages every enemy's buffs and removes ones whose duration expired.
/// Registered by `combat::CombatPlugin`.
pub fn tick_enemy_buffs(time: Res<Time>, mut buff_holders: Query<&mut EnemyBuffs>) {
    let delta = time.delta_secs();
    for mut buffs in &mut buff_holders {
        buffs.tick(delta);
    }
}
