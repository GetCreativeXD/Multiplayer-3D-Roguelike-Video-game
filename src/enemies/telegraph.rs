//! Enemy attack telegraphs and area-attack visuals, as something other
//! systems can recognise.
//!
//! Each archetype still spawns, animates and despawns its own telegraph
//! (`saint`, `knight`). Tagging them with `EnemyTelegraph` lets multiplayer
//! replicate every one of them generically (`net::telegraphs`): a client
//! draws the same visual with `attach_telegraph_visual` and animates it with
//! `telegraph_scale`, which use the same shapes, colours and timing as the
//! host's own systems, so the two can't drift apart.
//!
//! The Gatebow Marksman's aim line is not here: it follows a moving player,
//! so a client draws it between its own copies of the Marksman and that
//! player instead (`ranged::aim_line_transform`).

use bevy::prelude::*;

#[cfg(feature = "multiplayer")]
use super::{knight, saint};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum EnemyTelegraphKind {
    /// The Saint's Fist Slam direct-impact disk (during the windup).
    SaintSlamDisk,
    /// The Saint's expanding ground shockwave (jump it).
    SaintShockwave,
    /// The Black Banner Knight's Shield Rush lane.
    KnightRushLane,
    /// The Black Banner Knight's Banner Plant channel disk.
    KnightBannerDisk,
}

/// Marks a telegraph or area-attack visual an enemy spawned.
#[derive(Component, Clone, Copy, Debug)]
pub struct EnemyTelegraph {
    // Read by multiplayer only.
    #[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
    pub kind: EnemyTelegraphKind,
}

/// Adds `kind`'s meshes to `entity` (a client's copy of a host telegraph).
#[cfg(feature = "multiplayer")]
pub fn attach_telegraph_visual(
    commands: &mut Commands,
    entity: Entity,
    kind: EnemyTelegraphKind,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    match kind {
        EnemyTelegraphKind::SaintSlamDisk => {
            commands
                .entity(entity)
                .insert(saint::slam_telegraph_visual(meshes, materials));
        }
        EnemyTelegraphKind::SaintShockwave => {
            saint::attach_shockwave_visual(commands, entity, meshes, materials);
        }
        EnemyTelegraphKind::KnightRushLane => {
            commands
                .entity(entity)
                .insert(knight::rush_telegraph_visual(meshes, materials));
        }
        EnemyTelegraphKind::KnightBannerDisk => {
            commands
                .entity(entity)
                .insert(knight::banner_telegraph_visual(meshes, materials));
        }
    }
}

/// A telegraph's scale `seconds` after it appeared, given the scale it was
/// spawned with: the pulse of the disks, the growth of the shockwave (its
/// radius is the scale), or the rush lane's fixed length.
#[cfg(feature = "multiplayer")]
pub fn telegraph_scale(kind: EnemyTelegraphKind, seconds: f32, spawn_scale: Vec3) -> Vec3 {
    match kind {
        EnemyTelegraphKind::SaintSlamDisk => saint::slam_telegraph_scale(seconds),
        EnemyTelegraphKind::SaintShockwave => {
            saint::shockwave_scale(saint::shockwave_radius_after(seconds))
        }
        EnemyTelegraphKind::KnightRushLane => spawn_scale,
        EnemyTelegraphKind::KnightBannerDisk => knight::banner_telegraph_scale(seconds),
    }
}
