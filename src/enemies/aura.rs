//! Generic area buff source for a spawned enemy-side object -- today only
//! the Black Banner Knight's planted Corrupted Banner (see
//! `enemies::knight`), but built so a future support enemy (the roster's
//! Bell Husk: "temporarily increases nearby enemies' movement and attack
//! speed") can reuse it instead of a one-off radius check.
//!
//! An `AuraEmitter` continuously re-grants its `EnemyBuffs` entry to every
//! `Enemy` within `radius` (itself excluded) for as long as it exists and
//! they stay in range. Because `combat::EnemyBuffs::grant` refreshes
//! rather than stacks a source, standing in overlapping auras from
//! multiple emitters of the *same* source never compounds beyond the
//! single strongest active grant -- the roster's explicit "multiple Bell
//! Husk buffs should not stack without an intentional cap" requirement,
//! satisfied here for free rather than re-solved per archetype.

use bevy::prelude::*;

use crate::combat::{EnemyBuffSource, EnemyBuffs};

use super::Enemy;

/// How long a single aura grant lasts before it needs to be refreshed.
/// Comfortably longer than one frame so a brief stutter doesn't drop the
/// buff, short enough that leaving the aura (or the emitter being
/// destroyed) lets it lapse almost immediately.
const AURA_GRANT_DURATION: f32 = 0.35;

/// Attach to any entity that should continuously buff nearby enemies --
/// today, the Corrupted Banner spawned by `enemies::knight`.
#[derive(Component, Clone, Copy)]
pub struct AuraEmitter {
    pub source: EnemyBuffSource,
    pub radius: f32,
    pub movement_speed_multiplier: f32,
    pub damage_multiplier: f32,
}

/// Grants every `AuraEmitter`'s buff to every `Enemy` within range every
/// frame. Registered by `enemies::EnemyPlugin`.
pub(super) fn apply_enemy_auras(
    emitters: Query<(Entity, &GlobalTransform, &AuraEmitter)>,
    mut enemies: Query<(Entity, &GlobalTransform, &mut EnemyBuffs), With<Enemy>>,
) {
    if emitters.is_empty() {
        return;
    }

    for (emitter_entity, emitter_transform, emitter) in &emitters {
        let center = emitter_transform.translation();

        for (enemy_entity, enemy_transform, mut buffs) in &mut enemies {
            if enemy_entity == emitter_entity {
                continue;
            }
            if enemy_transform.translation().distance(center) > emitter.radius {
                continue;
            }

            buffs.grant(
                emitter.source,
                emitter.movement_speed_multiplier,
                emitter.damage_multiplier,
                AURA_GRANT_DURATION,
            );
        }
    }
}
