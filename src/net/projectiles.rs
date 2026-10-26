//! Projectiles in flight, seen by every client (Milestone 5 step 4).
//!
//! - **Host:** every player projectile (`weapons::PlayerProjectile`: arrows,
//!   bolts, spells, thrown daggers) and every enemy projectile
//!   (`enemies::HostileProjectile`) gets a `NetworkProjectile` describing
//!   how it looks, a `NetworkPose` (published with every other pose by
//!   `player_bodies::publish_host_poses`) and a `NetworkVelocity`, and is
//!   replicated. Its look is read from the host's own mesh and material, so
//!   new projectile kinds need nothing here. When the host despawns a
//!   projectile (hit, expired) it disappears on every client.
//! - **Client:** each one becomes a visual-only replica: a sphere or box of
//!   the same size and colours, moved from its latest pose along its
//!   velocity so it flies smoothly between updates (about 30 a second).
//!
//! Replicas never collide or deal damage; all hits are decided on the host.
//! Enemy telegraphs (aim lines, Saint slam disk and shockwave) are in
//! `net::telegraphs`.

use bevy::camera::primitives::MeshAabb;
use bevy::prelude::*;
use lightyear::prelude::*;

use crate::enemies::HostileProjectile;
use crate::game_state::AppState;
use crate::weapons::{PlayerProjectile, Projectile};

use super::protocol::{NetworkPose, NetworkProjectile, NetworkVelocity, ProjectileShape};

/// Size used when a projectile has no mesh on its root entity.
const FALLBACK_HALF_EXTENT: f32 = 0.15;

/// A projectile whose sides differ by less than this ratio is drawn as a
/// sphere; anything more elongated (an enemy arrow) as a box.
const SPHERE_ASPECT_LIMIT: f32 = 1.2;

/// A replica stops extrapolating this long after its last pose update, so
/// a lag spike doesn't send it flying off.
const MAX_EXTRAPOLATION_SECONDS: f32 = 0.25;

// ---------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------

pub struct HostProjectileReplicationPlugin;

impl Plugin for HostProjectileReplicationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            replicate_new_projectiles.run_if(in_state(AppState::InGame)),
        )
        // After `Update`, so homing projectiles send this frame's velocity.
        .add_systems(PostUpdate, publish_projectile_velocities);
    }
}

fn replicate_new_projectiles(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    projectiles: Query<
        (
            Entity,
            &Transform,
            Option<&Mesh3d>,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<&Projectile>,
            Option<&HostileProjectile>,
        ),
        (
            Or<(With<PlayerProjectile>, With<HostileProjectile>)>,
            Without<NetworkProjectile>,
        ),
    >,
) {
    for (entity, transform, mesh, material, projectile, hostile) in &projectiles {
        let look = projectile_look(transform, mesh, material, &meshes, &materials);
        let velocity = projectile_velocity(projectile, hostile);

        // `try_insert`: harmless if the projectile already hit something.
        commands.entity(entity).try_insert((
            look,
            NetworkPose::from_transform(transform),
            NetworkVelocity(velocity.to_array()),
            Replicate::to_clients(NetworkTarget::All),
        ));
    }
}

fn projectile_velocity(
    projectile: Option<&Projectile>,
    hostile: Option<&HostileProjectile>,
) -> Vec3 {
    projectile
        .map(|projectile| projectile.velocity)
        .or(hostile.map(|hostile| hostile.velocity))
        .unwrap_or(Vec3::ZERO)
}

fn projectile_look(
    transform: &Transform,
    mesh: Option<&Mesh3d>,
    material: Option<&MeshMaterial3d<StandardMaterial>>,
    meshes: &Assets<Mesh>,
    materials: &Assets<StandardMaterial>,
) -> NetworkProjectile {
    let half_extents = mesh
        .and_then(|mesh| meshes.get(&mesh.0))
        .and_then(|mesh| mesh.compute_aabb())
        .map(|aabb| Vec3::from(aabb.half_extents))
        .unwrap_or(Vec3::splat(FALLBACK_HALF_EXTENT))
        * transform.scale;
    let half_extents = half_extents.max(Vec3::splat(0.01));

    let shape = if half_extents.max_element() <= half_extents.min_element() * SPHERE_ASPECT_LIMIT {
        ProjectileShape::Sphere
    } else {
        ProjectileShape::Box
    };

    let material = material.and_then(|material| materials.get(&material.0));
    let base = material.map_or(Color::srgb(1.0, 0.6, 0.2), |material| material.base_color);
    let base = base.to_linear();
    let emissive = material.map_or(LinearRgba::BLACK, |material| material.emissive);

    NetworkProjectile {
        shape,
        half_extents: half_extents.to_array(),
        base_color: [base.red, base.green, base.blue, base.alpha],
        emissive: [emissive.red, emissive.green, emissive.blue, emissive.alpha],
        unlit: material.is_some_and(|material| material.unlit),
    }
}

/// Only writes when the velocity changed (homing projectiles), because any
/// write gets the component sent again.
fn publish_projectile_velocities(
    mut projectiles: Query<(
        Option<&Projectile>,
        Option<&HostileProjectile>,
        &mut NetworkVelocity,
    )>,
) {
    for (projectile, hostile, mut network_velocity) in &mut projectiles {
        let velocity = projectile_velocity(projectile, hostile);
        if Vec3::from_array(network_velocity.0).distance_squared(velocity) > 0.0001 {
            network_velocity.0 = velocity.to_array();
        }
    }
}

// ---------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------

pub struct ClientProjectileReplicaPlugin;

impl Plugin for ClientProjectileReplicaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (attach_projectile_replicas, move_projectile_replicas)
                .chain()
                .run_if(in_state(AppState::InGame)),
        );
    }
}

/// A client's visual-only copy of a host projectile.
#[derive(Component, Default)]
struct ProjectileReplica {
    seconds_since_pose: f32,
}

fn attach_projectile_replicas(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    new_projectiles: Query<(Entity, &NetworkProjectile, &NetworkPose), Without<ProjectileReplica>>,
) {
    for (entity, look, pose) in &new_projectiles {
        // A unit shape scaled to the host's size.
        let mesh = match look.shape {
            ProjectileShape::Sphere => meshes.add(Sphere::new(1.0)),
            ProjectileShape::Box => meshes.add(Cuboid::new(2.0, 2.0, 2.0)),
        };
        let [r, g, b, a] = look.base_color;
        let [er, eg, eb, ea] = look.emissive;
        let material = materials.add(StandardMaterial {
            base_color: Color::LinearRgba(LinearRgba::new(r, g, b, a)),
            emissive: LinearRgba::new(er, eg, eb, ea),
            unlit: look.unlit,
            alpha_mode: if a < 1.0 {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            ..default()
        });

        // Replicated entities arrive without a Transform.
        commands.entity(entity).insert((
            ProjectileReplica::default(),
            Name::new("Projectile (replica)"),
            pose.to_transform()
                .with_scale(Vec3::from_array(look.half_extents)),
            Visibility::default(),
            Mesh3d(mesh),
            MeshMaterial3d(material),
        ));
    }
}

fn move_projectile_replicas(
    time: Res<Time>,
    mut replicas: Query<(
        Ref<NetworkPose>,
        Option<&NetworkVelocity>,
        &mut ProjectileReplica,
        &mut Transform,
    )>,
) {
    let delta = time.delta_secs();

    for (pose, velocity, mut replica, mut transform) in &mut replicas {
        if pose.is_changed() {
            replica.seconds_since_pose = 0.0;
        } else {
            replica.seconds_since_pose += delta;
        }

        let ahead = replica.seconds_since_pose.min(MAX_EXTRAPOLATION_SECONDS);
        let velocity = velocity.map_or(Vec3::ZERO, |velocity| Vec3::from_array(velocity.0));
        transform.translation = pose.position() + velocity * ahead;
        transform.rotation = pose.rotation();
    }
}
