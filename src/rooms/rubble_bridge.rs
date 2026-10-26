//! The Rubble Bridge -- a pure jump-platforming connector between Soldiers
//! Passage and the Saint of the Broken Siege arena. No enemies spawn here;
//! it's a traversal challenge, not a combat encounter.
//!
//! The GLB (`saint_broken_siege_rubble_bridge.glb`) is 8 discrete floating
//! rubble slabs (`Collision_RubblePlatform_01`..`08`) with real gaps between
//! them and no continuous floor -- crossing means jumping platform to
//! platform, each one sitting a little higher than the last as the route
//! climbs from Soldiers Passage's launch ledge up to the Saint's arena
//! entrance. Like `soldiers_passage.glb`, none of these nodes carry a baked
//! `extras.size_xyz` field, so each collider's size below is that
//! `Collision_*` node's own `scale`, doubled (Blender's Empty-display
//! half-extent convention, same as every other size-less room in this
//! project) -- no axis reordering needed, matching `soldiers_passage.rs`.
//!
//! `NavigationBounds_PlatformRoute`'s own scale is used directly (not
//! doubled) for `RoomBounds`, matching the convention every other room's
//! bounding marker already uses (the marker's scale *is* the half-extent
//! for this purpose, unlike a physical `Collision_*` collider's scale).
//!
//! No perimeter walls, no floor slab under the gaps -- falling off a
//! platform here is meant to drop the player into the void, same as
//! Soldiers Passage's own "floating ruins over a void" design (see that
//! room's module doc comment); the existing single global `VoidKillZone`
//! (Fallen Gate's own) resets them back to the current room entrance.

use avian3d::prelude::{Collider, ColliderConstructor, ColliderConstructorHierarchy, RigidBody};
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

use super::components::*;
use super::connections::{self, RoomSocket};
use super::soldiers_passage;
use crate::collision::camera_occluder_layers;

// =========================================================
// ROOM ASSET + CONNECTION SOCKETS
// =========================================================

pub const RUBBLE_BRIDGE_ROOM_PATH: &str = "rooms/saint_broken_siege_rubble_bridge.glb";

/// This room's own entrance socket (`Socket_Entrance_FromSoldiersPassage`,
/// local (0, 0.18, 0)).
const ENTRANCE_SOCKET: RoomSocket =
    RoomSocket::new(Vec3::new(0.0, 0.18, 0.0), Vec3::new(0.0, 0.0, -1.0));

/// This room's own exit socket, toward the Saint arena
/// (`Socket_Exit_ToSaintBrokenSiege`, local (0, 1.85, -64.7)). Forward
/// continues the route's own direction of travel (-Z) -- every platform in
/// this GLB descends in Z from the entrance towards this point, so the
/// bridge keeps heading the same way it's been heading the whole crossing;
/// not read from the marker's own rotation, which this project has already
/// found unreliable for direction-of-travel (see `shattered_junction.rs`'s
/// module doc comment).
///
/// Follows the corrected pattern established by `ruined_barracks::
/// exit_socket`/`shattered_junction::exit_socket_to_soldiers_passage`/
/// `soldiers_passage::exit_socket`: this room does not sit at the world
/// origin, so its own `room_transform` is recomputed here and used to
/// convert the local socket into true world space before returning it.
pub fn exit_socket() -> RoomSocket {
    let local = RoomSocket::new(Vec3::new(0.0, 1.85, -64.7), Vec3::new(0.0, 0.0, -1.0));
    let room_transform =
        connections::connect_room(soldiers_passage::exit_socket(), ENTRANCE_SOCKET);
    RoomSocket::new(
        room_transform.transform_point(local.position),
        room_transform.rotation * local.forward,
    )
}

// =========================================================
// MAIN ROOM SPAWNER
// =========================================================

pub fn spawn_rubble_bridge(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut room_graph: ResMut<super::graph::RoomGraph>,
    soldiers_passage_room: Query<Entity, With<SoldiersPassage>>,
) {
    let room_transform =
        connections::connect_room(soldiers_passage::exit_socket(), ENTRANCE_SOCKET);

    let room = commands
        .spawn((
            Name::new("ROOM - Rubble Bridge"),
            Room,
            RubbleBridge,
            RoomInfo::new("Rubble Bridge", RoomKind::Unknown),
            // Matches this room's own authored `NavigationBounds_
            // PlatformRoute` marker (center (0, -31), half-extents (6, 32)).
            RoomBounds::new(Vec2::new(0.0, -31.0), Vec2::new(6.0, 32.0)),
            room_transform,
            Visibility::default(),
        ))
        .id();

    if let Ok(passage_entity) = soldiers_passage_room.single() {
        room_graph.connect(passage_entity, room);
    }

    commands.entity(room).with_children(|parent| {
        // -----------------------------------------------------
        // VISIBLE ROOM SCENE
        // -----------------------------------------------------
        let bridge_scene =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(RUBBLE_BRIDGE_ROOM_PATH));

        parent.spawn((
            Name::new("Rubble Bridge Visual Scene"),
            WorldAssetRoot(bridge_scene),
            Transform::default(),
            ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh)
                .with_default_layers(camera_occluder_layers()),
        ));

        // -----------------------------------------------------
        // DETERMINISTIC STATIC COLLISION -- 8 discrete rubble platforms,
        // each its own `Collision_RubblePlatform_NN` node's position and
        // (doubled) scale. Deliberately no floor under the gaps between
        // them -- see the module doc comment.
        // -----------------------------------------------------
        for (name, x, y, z, size) in [
            (
                "Rubble Platform 01",
                0.0,
                0.0,
                -2.3,
                Vec3::new(10.5, 0.8, 6.0),
            ),
            (
                "Rubble Platform 02",
                1.4,
                0.35,
                -10.4,
                Vec3::new(8.8, 0.8, 6.2),
            ),
            (
                "Rubble Platform 03",
                -1.5,
                0.85,
                -18.0,
                Vec3::new(9.0, 0.8, 6.3),
            ),
            (
                "Rubble Platform 04",
                1.8,
                0.35,
                -25.7,
                Vec3::new(10.0, 0.8, 6.8),
            ),
            (
                "Rubble Platform 05",
                -1.0,
                1.25,
                -33.7,
                Vec3::new(8.8, 0.8, 6.5),
            ),
            (
                "Rubble Platform 06",
                1.3,
                0.75,
                -41.7,
                Vec3::new(10.0, 0.8, 7.0),
            ),
            (
                "Rubble Platform 07",
                -0.8,
                1.45,
                -50.0,
                Vec3::new(9.2, 0.8, 7.0),
            ),
            (
                "Rubble Platform 08",
                0.0,
                1.85,
                -58.7,
                Vec3::new(14.0, 1.1, 11.5),
            ),
        ] {
            parent.spawn((
                Name::new(name),
                Transform::from_xyz(x, y, z),
                RigidBody::Static,
                Collider::cuboid(size.x, size.y, size.z),
            ));
        }

        // -----------------------------------------------------
        // ROOM SYSTEM MARKERS -- plain descriptive markers only, same
        // single-global-Entrance/VoidKillZone reasoning as every other
        // connected room (see `ruined_barracks.rs`/`shattered_junction.rs`/
        // `soldiers_passage.rs`).
        // -----------------------------------------------------
        parent.spawn((
            Name::new("Bridge Entrance (from Soldiers Passage)"),
            Transform::from_xyz(0.0, 0.18, 0.0),
            Visibility::default(),
        ));

        parent.spawn((
            Name::new("Bridge Exit (to the Saint's Arena)"),
            RoomConnector {
                kind: ConnectorKind::Exit,
            },
            Transform::from_xyz(0.0, 1.85, -64.7),
            Visibility::default(),
        ));
    });

    println!("Rubble Bridge spawned -- 8 jump platforms toward the Saint's arena.");
}
