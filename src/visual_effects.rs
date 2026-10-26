//! Combat visual effects every player should see.
//!
//! Ability bursts, ground telegraphs and summons (Hammer's Ground Slam disk,
//! Bear Slam, Grand Rune, the Guardian Otter, ...) are plain mesh entities
//! that the simulating machine spawns, animates and despawns. A Bevy mesh
//! handle doesn't say what shape it was built from, so spawn sites build
//! these meshes with `effect_mesh`, which also records the shape as a
//! `SharedEffect`. In multiplayer the host replicates every `SharedEffect`
//! entity (shape, colours, pose, scale) and a joined player draws the same
//! mesh (`net::effect_visuals`). Offline it changes nothing: the mesh is
//! built exactly as before.
//!
//! Projectiles are not tagged: `net::projectiles` already replicates them.

use bevy::prelude::*;

/// A primitive mesh, with the same parameters as the Bevy primitive it is
/// built from.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum EffectShape {
    /// `Sphere::new(radius)`
    Sphere { radius: f32 },
    /// `Cylinder::new(radius, height)`
    Cylinder { radius: f32, height: f32 },
    /// `Torus::new(inner_radius, outer_radius)`
    Torus {
        inner_radius: f32,
        outer_radius: f32,
    },
    /// `Capsule3d::new(radius, length)`
    Capsule { radius: f32, length: f32 },
}

impl EffectShape {
    pub fn mesh(self) -> Mesh {
        match self {
            EffectShape::Sphere { radius } => Sphere::new(radius).into(),
            EffectShape::Cylinder { radius, height } => Cylinder::new(radius, height).into(),
            EffectShape::Torus {
                inner_radius,
                outer_radius,
            } => Torus::new(inner_radius, outer_radius).into(),
            EffectShape::Capsule { radius, length } => Capsule3d::new(radius, length).into(),
        }
    }
}

/// Marks a visual effect every player should see, and records its shape.
/// Added by `effect_mesh`.
#[derive(Component, Clone, Copy, Debug)]
pub struct SharedEffect {
    // Read by multiplayer only.
    #[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
    pub shape: EffectShape,
}

/// The mesh for a shared visual effect: use in place of
/// `Mesh3d(meshes.add(<primitive>))` when spawning an effect other players
/// should see. Put it on a top-level entity (not a child).
pub fn effect_mesh(meshes: &mut Assets<Mesh>, shape: EffectShape) -> impl Bundle + use<> {
    (Mesh3d(meshes.add(shape.mesh())), SharedEffect { shape })
}
