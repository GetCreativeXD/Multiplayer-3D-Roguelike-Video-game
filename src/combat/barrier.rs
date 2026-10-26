//! Generic, reusable temporary damage-absorbing barriers.
//!
//! A barrier soaks up incoming damage before it reaches `Health`, then
//! expires on its own after `duration` seconds even if it is never fully
//! depleted. This is intentionally gear-agnostic: any current or future
//! system -- an armor effect, a class ability, a consumable, a boss
//! mechanic that shields the player -- grants a barrier through the same
//! `Barriers` component and `combat::apply_player_damage` absorbs from it
//! automatically. Nothing in this file knows about any specific item or
//! ability.

use bevy::prelude::*;

/// How long a freshly granted barrier lasts, in seconds, unless the
/// grantor asks for a different duration via `Barriers::grant`.
///
/// Ruinbound's current design calls for all barriers (gear-granted or
/// otherwise) to last 15 seconds by default.
pub const DEFAULT_BARRIER_DURATION: f32 = 15.0;

/// Where a barrier came from. Purely informational (HUD, logging, "what
/// gave me this shield" debugging) -- absorption itself does not care
/// about source. Add a variant per notable grantor as they're wired up;
/// keep `Other` around for ad-hoc/test/future barriers that don't need
/// their own name yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BarrierSource {
    /// Runebound Robes (Rare Arcanist chest) -- using an ability grants a
    /// small barrier.
    RuneboundRobes,
    /// Robes of the Hollow Star (Legendary Arcanist chest) -- using an
    /// ability while critically wounded grants a barrier.
    RobesOfTheHollowStar,
    /// Crown of the Unbroken (Legendary Bulwark head) -- fatal damage
    /// instead leaves the player at one health and grants a barrier.
    CrownOfTheUnbroken,
    /// Warden of Shades' Guardian Otter (see
    /// `characters::warden_abilities`) -- shields whichever player it
    /// blesses.
    GuardianOtter,
    /// Saint's Testament's Graceful Casting (Legendary Spellbook -- see
    /// `weapons::spellbook`) -- damaging an enemy with Grace Bolt builds
    /// Grace; at maximum Grace the player is granted a barrier.
    SaintsTestament,
    /// Anything else: a future ability, a debug command, a test.
    Other,
}

/// One active shield charge.
#[derive(Clone, Copy, Debug)]
pub struct Barrier {
    pub source: BarrierSource,
    /// Remaining shield points. Reduced as damage is absorbed.
    pub amount: f32,
    /// Seconds left before this barrier expires on its own.
    pub remaining: f32,
}

/// All barriers currently protecting an entity. Add this component to any
/// entity that should be able to hold a barrier -- today that's the
/// player, initialized alongside `Health` in `combat::initialize_combat_defenses`.
///
/// Multiple barriers can be active at once. They absorb oldest-first, so a
/// long-lived small barrier isn't left stranded behind a fresh large one.
#[derive(Component, Default, Debug)]
pub struct Barriers {
    active: Vec<Barrier>,
}

impl Barriers {
    /// Grants a new barrier that lasts `DEFAULT_BARRIER_DURATION` seconds.
    /// This is the entry point most gear effects and abilities should use.
    pub fn grant_default(&mut self, source: BarrierSource, amount: f32) {
        self.grant(source, amount, DEFAULT_BARRIER_DURATION);
    }

    /// Grants a new barrier with an explicit duration, for the rare case
    /// a future effect needs something other than the 15-second default.
    pub fn grant(&mut self, source: BarrierSource, amount: f32, duration: f32) {
        if amount <= 0.0 || duration <= 0.0 {
            return;
        }

        self.active.push(Barrier {
            source,
            amount,
            remaining: duration,
        });
    }

    /// Total shield points currently held across all active barriers.
    pub fn total_amount(&self) -> f32 {
        self.active.iter().map(|barrier| barrier.amount).sum()
    }

    /// True if at least one barrier is currently active.
    pub fn has_any(&self) -> bool {
        !self.active.is_empty()
    }

    /// Consumes up to `incoming` points of damage from the oldest barriers
    /// first, removing any barrier that hits zero. Returns the amount that
    /// was absorbed; the caller (`combat::apply_player_damage`) applies
    /// whatever remains to `Health`.
    pub fn absorb(&mut self, incoming: f32) -> f32 {
        let mut remaining_damage = incoming.max(0.0);
        let mut absorbed = 0.0;

        for barrier in self.active.iter_mut() {
            if remaining_damage <= 0.0 {
                break;
            }

            let taken = barrier.amount.min(remaining_damage);
            barrier.amount -= taken;
            remaining_damage -= taken;
            absorbed += taken;
        }

        self.active.retain(|barrier| barrier.amount > 0.0);
        absorbed
    }

    /// Counts down every active barrier's remaining lifetime and drops the
    /// ones that expired. Called once per frame by `tick_barriers`.
    fn tick(&mut self, delta: f32) {
        for barrier in self.active.iter_mut() {
            barrier.remaining -= delta;
        }
        self.active.retain(|barrier| barrier.remaining > 0.0);
    }
}

/// Ages every entity's barriers and removes ones whose duration expired.
/// Registered by `combat::CombatPlugin`.
pub fn tick_barriers(time: Res<Time>, mut barrier_holders: Query<&mut Barriers>) {
    let delta = time.delta_secs();
    for mut barriers in &mut barrier_holders {
        barriers.tick(delta);
    }
}
