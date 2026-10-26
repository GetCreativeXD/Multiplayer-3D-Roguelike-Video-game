mod barrier;
mod buffs;
mod death_prevention;
mod dot;
mod enemy_buffs;
mod enemy_debuffs;
mod frontal_guard;

use bevy::prelude::*;

use crate::player::PlayerCharacter;
use crate::run_setup::RunModifiers;
use crate::weapons::Equipment;

pub use barrier::{Barrier, BarrierSource, Barriers, DEFAULT_BARRIER_DURATION, tick_barriers};
pub use buffs::{BuffSource, StatBuffs, tick_stat_buffs};
pub use death_prevention::{DeathPrevention, tick_death_prevention};
pub use dot::{DamageOverTime, DotSource, tick_damage_over_time};
pub use enemy_buffs::{
    EnemyBuffSource, EnemyBuffs, enemy_damage_multiplier, enemy_movement_speed_multiplier,
    tick_enemy_buffs,
};
pub use enemy_debuffs::{
    DebuffSource, EnemyDebuffs, enemy_debuff_movement_speed_multiplier, tick_enemy_debuffs,
};
pub use frontal_guard::FrontalGuard;

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct BaseHealth(pub f32);

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DamageResult {
    pub incoming: f32,
    pub blocked: f32,
    /// How much of the post-defense damage a barrier absorbed before it
    /// reached `Health`. Zero if the entity had no barrier (or none was
    /// passed in, e.g. an enemy taking damage today).
    pub barrier_absorbed: f32,
    pub applied: f32,
    /// True if this hit would have been fatal and `DeathPrevention`
    /// stepped in to leave the entity at 1 health instead.
    pub prevented_death: bool,
}

/// Central player-damage path. Future enemies, hazards, bosses, and status
/// effects should call this instead of subtracting from Health directly.
///
/// Order of operations: defense mitigation, then barrier absorption, then
/// (only if the remaining damage would be fatal) death prevention. Both
/// `barriers` and `death_prevention` are optional so this function stays
/// usable for any entity, with or without either system -- pass `None` for
/// an entity that can't hold a barrier or can't be saved from death.
pub fn apply_player_damage(
    health: &mut Health,
    equipment: &Equipment,
    mut barriers: Option<&mut Barriers>,
    death_prevention: Option<&mut DeathPrevention>,
    incoming_damage: f32,
) -> DamageResult {
    let incoming = incoming_damage.max(0.0);
    let defense = equipment.total_defense().max(0.0);

    // Diminishing returns: 100 defense = 50% reduction.
    let mitigated = incoming * 100.0 / (100.0 + defense);
    let blocked = incoming - mitigated;

    let barrier_absorbed = barriers
        .as_deref_mut()
        .map_or(0.0, |barriers| barriers.absorb(mitigated));

    let mut applied = mitigated - barrier_absorbed;
    let mut prevented_death = false;

    if applied >= health.current {
        if let Some(death_prevention) = death_prevention {
            if death_prevention.is_active_and_ready() {
                death_prevention.trigger();
                prevented_death = true;
                applied = (health.current - 1.0).max(0.0);

                if let Some(barriers) = barriers.as_deref_mut() {
                    barriers.grant_default(
                        BarrierSource::CrownOfTheUnbroken,
                        health.max * death_prevention.barrier_fraction_of_max_health(),
                    );
                }
            }
        }
    }

    health.current = (health.current - applied).max(0.0);

    DamageResult {
        incoming,
        blocked,
        barrier_absorbed,
        applied,
        prevented_death,
    }
}

/// Central player-healing path (mirrors `apply_player_damage` on the
/// damage side). Multiplies `amount` by the run's
/// `RunModifiers::healing_multiplier` (a pact/relic effect -- see
/// `run_setup::effects`) before applying it, clamped so `health.current`
/// never exceeds `health.max`. Returns how much was actually healed.
///
/// Every future heal source (a second ability, a potion, a shrine)
/// should call this instead of writing to `Health` directly, so a
/// healing-affecting pact or relic reaches every heal at once instead of
/// each source needing to know `RunModifiers` exists.
pub fn apply_player_healing(health: &mut Health, run_modifiers: &RunModifiers, amount: f32) -> f32 {
    let requested = (amount * run_modifiers.healing_multiplier).max(0.0);
    let applied = requested.min((health.max - health.current).max(0.0));
    health.current += applied;
    applied
}

/// Multiplies `incoming_damage` by any active `EnemyDebuffs` multiplier
/// (e.g. Runesage's Runebroken -- see `combat::enemy_debuffs`), applies
/// the result to `health`, and returns the amount actually applied.
/// Mirrors `apply_player_damage`'s role on the player side, minus
/// defense/barrier mitigation (enemies don't have those yet) -- the
/// shared point every current and future enemy-damage source should call
/// instead of subtracting from `Health` directly, so a debuff like
/// Runebroken affects every hit that goes through it, not just the
/// ability that granted it.
///
/// Not every existing damage source has been switched over to this yet;
/// see the doc comments at the call sites that have (`weapons::melee`,
/// `weapons::projectiles::movement`) -- anything still subtracting from
/// `Health` directly simply won't see Runebroken's bonus damage.
pub fn apply_enemy_damage(
    health: &mut Health,
    debuffs: Option<&EnemyDebuffs>,
    incoming_damage: f32,
) -> f32 {
    let multiplier = debuffs.map_or(1.0, EnemyDebuffs::damage_taken_multiplier);
    let applied = (incoming_damage * multiplier).max(0.0);
    health.current = (health.current - applied).max(0.0);
    applied
}

/// Result of `apply_guarded_enemy_damage`.
#[derive(Clone, Copy, Debug)]
pub struct GuardedDamageResult {
    pub applied: f32,
    /// True if a `FrontalGuard` blocked this hit outright -- the caller
    /// should skip its own stagger/interrupt check when this is true, so a
    /// blocked light attack doesn't still interrupt the guarding enemy.
    pub stagger_negated: bool,
}

/// Like `apply_enemy_damage`, but additionally resolves a possible
/// `FrontalGuard` (today: the Black Banner Knight's shield, see
/// `enemies::knight`) before `EnemyDebuffs` and `Health` are touched.
/// Prefer this over calling `apply_enemy_damage` directly at any call site
/// that already has the enemy's rotation/position and the attacker's
/// position on hand, so any current or future guarded enemy is handled
/// uniformly instead of each weapon reimplementing the frontal/backstab
/// check. See `combat::frontal_guard`'s module doc comment for which
/// call sites currently do this and which don't yet.
pub fn apply_guarded_enemy_damage(
    health: &mut Health,
    debuffs: Option<&EnemyDebuffs>,
    guard: Option<&FrontalGuard>,
    enemy_rotation: Quat,
    enemy_position: Vec3,
    attacker_position: Vec3,
    incoming_stagger_strength: u32,
    incoming_damage: f32,
) -> GuardedDamageResult {
    let outcome = frontal_guard::resolve(
        guard,
        enemy_rotation,
        enemy_position,
        attacker_position,
        incoming_stagger_strength,
    );
    let applied = apply_enemy_damage(health, debuffs, incoming_damage * outcome.damage_multiplier);
    GuardedDamageResult {
        applied,
        stagger_negated: outcome.stagger_negated,
    }
}

/// `base_resistance` after any active `EnemyDebuffs` reduction (e.g.
/// Runebroken), floored at zero. Pass the result wherever a stagger check
/// would otherwise compare directly against `Stagger::resistance`.
pub fn effective_stagger_resistance(base_resistance: u32, debuffs: Option<&EnemyDebuffs>) -> u32 {
    let reduction = debuffs.map_or(0, EnemyDebuffs::stagger_resistance_reduction);
    base_resistance.saturating_sub(reduction)
}

/// Gives every newly spawned player character the shared combat-defense
/// components (barriers, death prevention) so gear effects and future
/// abilities have somewhere to write into from the moment the player
/// exists. Mirrors the `Added<Player>` pattern already used by
/// `characters::class_abilities::initialize_character_combat_state`.
///
/// Uses `PlayerCharacter` rather than `Player`, so a multiplayer host's
/// bodies for joined players get the same defenses (enemy damage goes
/// through `apply_player_damage` for them too).
fn initialize_combat_defenses(
    mut commands: Commands,
    players: Query<Entity, Added<PlayerCharacter>>,
) {
    for player in &players {
        commands.entity(player).insert((
            Barriers::default(),
            DeathPrevention::default(),
            StatBuffs::default(),
        ));
    }
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                initialize_combat_defenses,
                tick_barriers,
                tick_death_prevention,
                tick_stat_buffs,
                tick_enemy_debuffs,
                tick_enemy_buffs,
                tick_damage_over_time,
            ),
        );
    }
}
