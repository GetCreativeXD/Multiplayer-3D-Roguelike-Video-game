//! A generic "not this time" safety net for fatal damage.
//!
//! `DeathPrevention` deliberately does not know about any specific item or
//! ability. It just answers one question for `combat::apply_player_damage`:
//! "if this hit would be fatal, is something currently active and off
//! cooldown that should stop it?" Whatever equips or grants that
//! protection (today: Crown of the Unbroken) is responsible for calling
//! `activate`/`deactivate` to keep this component's numbers in sync --
//! see `weapons::armor_procs::sync_unbroken_death_prevention` for the one
//! current example. A future class ability or consumable can reuse the
//! exact same component the same way.

use bevy::prelude::*;

#[derive(Component, Default, Debug)]
pub struct DeathPrevention {
    /// Fraction of max health granted as a barrier when this triggers.
    /// Zero means "nothing is currently granting this effect" -- the
    /// component can sit on the player inert until some gear or ability
    /// activates it.
    barrier_fraction_of_max_health: f32,
    /// How long this must wait before it can trigger again, once active.
    cooldown_duration: f32,
    cooldown_remaining: f32,
}

impl DeathPrevention {
    /// Called by whatever currently grants this effect (a gear-sync
    /// system, a future ability) once per frame to (re)declare its
    /// numbers. Safe to call every frame with the same values.
    pub fn activate(&mut self, barrier_fraction_of_max_health: f32, cooldown_duration: f32) {
        self.barrier_fraction_of_max_health = barrier_fraction_of_max_health;
        self.cooldown_duration = cooldown_duration;
    }

    /// Called when the source that granted this effect is no longer
    /// present (e.g. the gear was unequipped). Leaves the cooldown alone
    /// so re-equipping mid-cooldown doesn't refresh it for free.
    pub fn deactivate(&mut self) {
        self.barrier_fraction_of_max_health = 0.0;
    }

    /// True if something is currently granting this effect and its
    /// cooldown has finished.
    pub fn is_active_and_ready(&self) -> bool {
        self.barrier_fraction_of_max_health > 0.0 && self.cooldown_remaining <= 0.0
    }

    /// Fraction of max health to grant as a barrier when this triggers.
    pub fn barrier_fraction_of_max_health(&self) -> f32 {
        self.barrier_fraction_of_max_health
    }

    /// Starts the cooldown. Call this exactly once when the effect fires.
    pub fn trigger(&mut self) {
        self.cooldown_remaining = self.cooldown_duration;
    }

    fn tick(&mut self, delta: f32) {
        self.cooldown_remaining = (self.cooldown_remaining - delta).max(0.0);
    }
}

/// Counts down every entity's death-prevention cooldown. Registered by
/// `combat::CombatPlugin`.
pub fn tick_death_prevention(time: Res<Time>, mut holders: Query<&mut DeathPrevention>) {
    let delta = time.delta_secs();
    for mut death_prevention in &mut holders {
        death_prevention.tick(delta);
    }
}
