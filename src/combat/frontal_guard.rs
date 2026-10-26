//! Generic frontal defense for an enemy that presents a shield (or similar
//! guard) toward whatever it's currently facing.
//!
//! Today only the Black Banner Knight uses this (see `enemies::knight`),
//! but it's built as a standalone, enemy-agnostic component rather than a
//! one-off Knight check so a future shielded enemy -- the roster's Hollow
//! Castellan carries "shield and spectral sword" -- can reuse it instead
//! of reinventing the same facing math.
//!
//! An enemy with `FrontalGuard` mitigates damage from *light* attacks that
//! land within its frontal arc while it is actively blocking, and takes
//! *bonus* damage from any attack (light or heavy) that lands outside that
//! arc -- the roster's "frontal shield block against light attacks" /
//! "vulnerable from behind" pair for the Black Banner Knight. Nothing in
//! this file knows about any specific enemy or ability; the owning
//! archetype's own combat system is responsible for flipping `blocking`
//! on and off as it moves through its states (a Knight mid-swing has both
//! hands on its sword, not its shield).
//!
//! Known integration gap, disclosed the same way `combat::apply_enemy_damage`
//! already discloses Runebroken's: not every player damage source checks
//! `FrontalGuard` yet. Longsword's basic attack (`weapons::melee::melee_attack`)
//! and hostile-arrow-style projectiles (`weapons::projectiles::movement`)
//! call `combat::apply_guarded_enemy_damage` below; Greatsword/Hammer basic
//! attacks and every weapon *ability* still call `apply_enemy_damage`
//! directly and so always land as if the target had no guard at all. This
//! was a deliberate scope cut to avoid retyping the large, already-shared
//! `enemy_query` signature threaded through every weapon ability file in
//! one uncompiled pass -- widening `FrontalGuard` coverage to those paths
//! is future work, not a silent gap.

use bevy::prelude::*;

/// A directional guard an enemy can raise. Whether it's presently raised
/// is state the owning archetype's combat system controls every frame
/// (`blocking`); the rest is fixed tuning read every time an attack lands.
#[derive(Component, Clone, Copy, Debug)]
pub struct FrontalGuard {
    /// True while the shield is actively up. The owning enemy's own
    /// combat system sets this every frame from its current state (e.g.
    /// up while approaching or idle, down mid-swing or mid-charge).
    pub blocking: bool,
    /// Attacks with a stagger strength at or below this count as "light"
    /// and can be blocked; anything above punches through the shield
    /// regardless of facing or `blocking`. This lets heavier weapons
    /// behave differently against a shielded enemy instead of every hit
    /// mattering the same amount, per the project's "weapons should
    /// behave differently, not merely use different stats" combat goal.
    pub light_attack_stagger_threshold: u32,
    /// Cosine of the half-angle that counts as "in front." A hit whose
    /// direction from the enemy is within this arc of the enemy's facing
    /// is frontal; outside it counts as a flank/backstab. `0.3` (roughly
    /// a 145-degree frontal cone) is the Black Banner Knight's current
    /// EXPERIMENTAL/TUNING value -- wide enough to feel like a shield,
    /// narrow enough that "flank the shield" (the roster's stated
    /// counterplay) requires real repositioning, not a half-step.
    pub frontal_dot_threshold: f32,
    /// Multiplies damage from a blocked hit (frontal, blocking, light).
    /// `0.2` -- EXPERIMENTAL/TUNING -- blocks 80% of the hit.
    pub block_damage_multiplier: f32,
    /// Multiplies damage from any hit landing outside the frontal arc,
    /// blocking or not, light or heavy. `1.5` -- EXPERIMENTAL/TUNING.
    pub rear_damage_multiplier: f32,
}

impl FrontalGuard {
    pub fn new(
        light_attack_stagger_threshold: u32,
        frontal_dot_threshold: f32,
        block_damage_multiplier: f32,
        rear_damage_multiplier: f32,
    ) -> Self {
        Self {
            blocking: false,
            light_attack_stagger_threshold,
            frontal_dot_threshold,
            block_damage_multiplier,
            rear_damage_multiplier,
        }
    }
}

/// Result of checking one incoming hit against a possible `FrontalGuard`.
#[derive(Clone, Copy, Debug)]
pub(super) struct GuardOutcome {
    pub damage_multiplier: f32,
    /// True if the hit should be treated as blocked and therefore should
    /// not stagger/interrupt the guarding enemy.
    pub stagger_negated: bool,
}

const NO_GUARD_OUTCOME: GuardOutcome = GuardOutcome {
    damage_multiplier: 1.0,
    stagger_negated: false,
};

/// `enemy_rotation`'s local `+Z` is treated as the enemy's facing
/// direction -- the same convention every enemy archetype in this project
/// already uses when it rotates via
/// `Quat::from_rotation_y(direction.x.atan2(direction.z))`.
pub(super) fn resolve(
    guard: Option<&FrontalGuard>,
    enemy_rotation: Quat,
    enemy_position: Vec3,
    attacker_position: Vec3,
    incoming_stagger_strength: u32,
) -> GuardOutcome {
    let Some(guard) = guard else {
        return NO_GUARD_OUTCOME;
    };

    let mut to_attacker = attacker_position - enemy_position;
    to_attacker.y = 0.0;
    let to_attacker = to_attacker.normalize_or_zero();
    if to_attacker == Vec3::ZERO {
        return NO_GUARD_OUTCOME;
    }

    let facing = enemy_rotation * Vec3::Z;
    let is_frontal = facing.dot(to_attacker) >= guard.frontal_dot_threshold;

    if !is_frontal {
        return GuardOutcome {
            damage_multiplier: guard.rear_damage_multiplier,
            stagger_negated: false,
        };
    }

    let is_light = incoming_stagger_strength <= guard.light_attack_stagger_threshold;
    if guard.blocking && is_light {
        GuardOutcome {
            damage_multiplier: guard.block_damage_multiplier,
            stagger_negated: true,
        }
    } else {
        NO_GUARD_OUTCOME
    }
}
