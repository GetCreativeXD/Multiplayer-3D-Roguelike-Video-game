use avian_rerecast::ExcludeColliderFromNavmesh;
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::collision::camera_occluder_layers;

// =========================================================
// CAMERA OCCLUSION FOR VISUAL-ONLY GEOMETRY
// =========================================================

/// A collider used only so the third-person camera's spring arm (see
/// `camera::camera_collision`) treats this piece as something to duck
/// around -- even though it never physically blocks players or enemies and
/// is excluded from navmesh generation. Matches the visual mesh's own box
/// size and transform exactly, so the camera reacts to what the player can
/// actually see, not just to what happens to block movement.
///
/// Used by the "visual-only" block helpers below (`spawn_block`,
/// `spawn_rotated_block`) so every decorative, walk-through piece built
/// with them -- in any room, present or future -- automatically keeps the
/// camera from clipping through it, with no per-room setup required.
fn camera_occluder(size: Vec3) -> impl Bundle {
    (
        RigidBody::Static,
        Collider::cuboid(size.x, size.y, size.z),
        Sensor,
        camera_occluder_layers(),
        ExcludeColliderFromNavmesh,
    )
}

// =========================================================
// VISUAL BLOCK
// =========================================================

pub fn spawn_block(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
    name: &'static str,
) {
    parent.spawn((
        Name::new(name),
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(material),
        Transform::from_translation(position),
        camera_occluder(size),
    ));
}

// =========================================================
// VISUAL ROTATED BLOCK
// =========================================================

pub fn spawn_rotated_block(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
    rotation: Vec3,
    name: &'static str,
) {
    parent.spawn((
        Name::new(name),
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(material),
        Transform {
            translation: position,

            rotation: Quat::from_euler(EulerRot::XYZ, rotation.x, rotation.y, rotation.z),

            ..default()
        },
        camera_occluder(size),
    ));
}

// =========================================================
// SOLID BLOCK
// =========================================================

pub fn spawn_solid_block(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
    name: &'static str,
) {
    parent.spawn((
        Name::new(name),
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(material),
        Transform::from_translation(position),
        RigidBody::Static,
        Collider::cuboid(size.x, size.y, size.z),
    ));
}

// =========================================================
// SOLID ROTATED BLOCK
// =========================================================

pub fn spawn_solid_rotated_block(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
    rotation: Vec3,
    name: &'static str,
) {
    parent.spawn((
        Name::new(name),
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(material),
        Transform {
            translation: position,

            rotation: Quat::from_euler(EulerRot::XYZ, rotation.x, rotation.y, rotation.z),

            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(size.x, size.y, size.z),
    ));
}

// =========================================================
// ROTATED VISUAL WITH FLAT WALKABLE COLLIDER
// =========================================================

/// Draws a fractured, rotated floor piece while keeping its walkable collision
/// level with neighboring floor sections. This prevents narrow navmesh seams
/// where decorative edge fragments overlap a flat connector or floor core.
pub fn spawn_rotated_visual_with_flat_collider(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
    rotation: Vec3,
    name: &'static str,
) {
    spawn_rotated_block(parent, meshes, material, position, size, rotation, name);

    parent.spawn((
        Name::new("Flat Walkable Floor Collider"),
        Transform::from_translation(position),
        RigidBody::Static,
        Collider::cuboid(size.x, size.y, size.z),
    ));
}

pub fn spawn_rotated_visual_with_solid_collider(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
    rotation: Vec3,
    name: &'static str,
) {
    parent.spawn((
        Name::new(name),
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(material),
        Transform {
            translation: position,

            rotation: Quat::from_euler(EulerRot::XYZ, rotation.x, rotation.y, rotation.z),

            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(size.x, size.y, size.z),
    ));
}

pub fn spawn_rotated_visual_with_box_collider(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
    rotation: Vec3,
    name: &'static str,
) {
    let entity = parent
        .spawn((
            Name::new(name),
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(material),
            Transform {
                translation: position,

                rotation: Quat::from_euler(EulerRot::XYZ, rotation.x, rotation.y, rotation.z),

                ..default()
            },
        ))
        .id();

    parent.commands().entity(entity).with_children(|child| {
        child.spawn((
            Name::new("Solid Collider"),
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            Transform {
                rotation: Quat::from_euler(EulerRot::XYZ, -rotation.x, -rotation.y, -rotation.z),

                ..default()
            },
        ));
    });
}
