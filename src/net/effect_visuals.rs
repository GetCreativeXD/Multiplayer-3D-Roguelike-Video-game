//! Players' ability effects, seen by every client (Milestone 6 step 2).
//!
//! - **Host:** every entity tagged `visual_effects::SharedEffect` (Hammer's
//!   Ground Slam disk, impact bursts and unstable ground; class-ability
//!   bursts, telegraphs and summons) gets a `NetworkEffect` (its shape and
//!   colours, read from its own `StandardMaterial`), a `NetworkPose` and a
//!   `NetworkScale`, and is replicated. Pose and scale are republished when
//!   they change (pulsing disks, the Guardian Otter circling its target).
//!   The host despawning it removes it everywhere.
//! - **Client:** draws the same primitive mesh with the same colours, and
//!   follows the host's pose (smoothed) and scale.
//!
//! Visual only: every hit is decided on the host.

use bevy::prelude::*;
use lightyear::prelude::*;

use crate::enemies::HostileProjectile;
use crate::game_state::AppState;
use crate::visual_effects::SharedEffect;
use crate::weapons::PlayerProjectile;

use super::protocol::{NetworkEffect, NetworkPose, NetworkScale};

/// Same rate and snap distance `net::avatars` uses for characters.
const SMOOTHING_RATE: f32 = 15.0;
const SNAP_DISTANCE: f32 = 5.0;

// ---------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------

pub struct HostEffectVisualsPlugin;

impl Plugin for HostEffectVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            replicate_new_effects.run_if(in_state(AppState::InGame)),
        )
        // After `Update`, so this frame's pulse is sent.
        .add_systems(PostUpdate, publish_effect_scales);
    }
}

fn replicate_new_effects(
    mut commands: Commands,
    materials: Res<Assets<StandardMaterial>>,
    effects: Query<
        (
            Entity,
            &SharedEffect,
            &Transform,
            Option<&MeshMaterial3d<StandardMaterial>>,
        ),
        (
            Without<NetworkEffect>,
            // Projectiles are replicated by `net::projectiles`.
            Without<PlayerProjectile>,
            Without<HostileProjectile>,
        ),
    >,
) {
    for (entity, effect, transform, material) in &effects {
        let material = material.and_then(|material| materials.get(&material.0));
        let base = material
            .map_or(Color::WHITE, |material| material.base_color)
            .to_linear();
        let emissive = material.map_or(LinearRgba::BLACK, |material| material.emissive);

        // `try_insert`: harmless if the effect ended this same frame.
        commands.entity(entity).try_insert((
            NetworkEffect {
                shape: effect.shape,
                base_color: [base.red, base.green, base.blue, base.alpha],
                emissive: [emissive.red, emissive.green, emissive.blue, emissive.alpha],
                unlit: material.is_some_and(|material| material.unlit),
            },
            NetworkPose::from_transform(transform),
            NetworkScale(transform.scale.to_array()),
            Replicate::to_clients(NetworkTarget::All),
            DisableReplicateHierarchy,
        ));
    }
}

/// Only writes when the scale changed, because any write gets the
/// component sent again.
fn publish_effect_scales(mut effects: Query<(&Transform, &mut NetworkScale), With<SharedEffect>>) {
    for (transform, mut scale) in &mut effects {
        if Vec3::from_array(scale.0).distance_squared(transform.scale) > 0.0001 * 0.0001 {
            scale.0 = transform.scale.to_array();
        }
    }
}

// ---------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------

pub struct ClientEffectVisualsPlugin;

impl Plugin for ClientEffectVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (attach_effect_replicas, follow_effect_replicas)
                .chain()
                .run_if(in_state(AppState::InGame)),
        );
    }
}

/// A client's visual-only copy of a host effect.
#[derive(Component)]
struct EffectReplica;

fn attach_effect_replicas(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    new_effects: Query<
        (Entity, &NetworkEffect, &NetworkPose, Option<&NetworkScale>),
        Without<EffectReplica>,
    >,
) {
    for (entity, effect, pose, scale) in &new_effects {
        let [r, g, b, a] = effect.base_color;
        let [er, eg, eb, ea] = effect.emissive;
        let material = materials.add(StandardMaterial {
            base_color: Color::LinearRgba(LinearRgba::new(r, g, b, a)),
            emissive: LinearRgba::new(er, eg, eb, ea),
            unlit: effect.unlit,
            alpha_mode: if a < 1.0 {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            ..default()
        });
        let scale = scale.map_or(Vec3::ONE, |scale| Vec3::from_array(scale.0));

        // Replicated entities arrive without a Transform.
        commands.entity(entity).insert((
            EffectReplica,
            Name::new("Effect (replica)"),
            pose.to_transform().with_scale(scale),
            Visibility::default(),
            Mesh3d(meshes.add(effect.shape.mesh())),
            MeshMaterial3d(material),
        ));
    }
}

fn follow_effect_replicas(
    time: Res<Time>,
    mut effects: Query<(&NetworkPose, Option<&NetworkScale>, &mut Transform), With<EffectReplica>>,
) {
    let blend = 1.0 - (-SMOOTHING_RATE * time.delta_secs()).exp();

    for (pose, scale, mut transform) in &mut effects {
        let target = pose.position();
        if transform.translation.distance(target) > SNAP_DISTANCE {
            transform.translation = target;
            transform.rotation = pose.rotation();
        } else {
            transform.translation = transform.translation.lerp(target, blend);
            transform.rotation = transform.rotation.slerp(pose.rotation(), blend);
        }
        if let Some(scale) = scale {
            transform.scale = Vec3::from_array(scale.0);
        }
    }
}
