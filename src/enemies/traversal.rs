use avian3d::prelude::*;
use bevy::prelude::*;

use super::{enemy_step_is_safe, enemy_world_path_is_clear};

/// Selects the terrain rules for a committed enemy ability movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommittedTraversalPolicy {
    /// Charges, lunges, and pounces that must remain on connected ground.
    Grounded,
    /// Purpose-built leaps that may cross a gap after validating their landing.
    AuthorizedGap,
}

/// Validates one step of a committed movement ability. Ability-specific code
/// owns timing, damage, and cooldowns; this shared helper owns terrain safety.
pub(crate) fn enemy_committed_step_is_safe(
    entity: Entity,
    position: Vec3,
    proposed_step: Vec3,
    policy: CommittedTraversalPolicy,
    spatial_query: &SpatialQuery,
) -> bool {
    match policy {
        CommittedTraversalPolicy::Grounded => {
            enemy_step_is_safe(entity, position, proposed_step, spatial_query)
        }
        CommittedTraversalPolicy::AuthorizedGap => {
            enemy_world_path_is_clear(entity, position, proposed_step, spatial_query)
        }
    }
}
