//! Shattered Junction -- a pure connector room between Ruined Barracks and
//! Soldiers Passage. No enemies of its own: the GLB (`shattered_junction.glb`)
//! has no `EnemySpawn_*`/`RangedSpawn_*`/`EliteSpawn` nodes at all, just a
//! diamond-shaped hub floor with three routes leading off it. Joined onto
//! Ruined Barracks the same way Ruined Barracks joined onto Fallen Gate --
//! see `rooms::connections`'s doc comment for the general pattern, and
//! `ruined_barracks::exit_socket` for the socket this room connects from.
//!
//! Every position/size/rotation below was read directly out of the exported
//! `shattered_junction.glb` (node translation/rotation/scale, and each
//! `Collision_*` node's own baked `extras.size_xyz`), not approximated by
//! hand -- same measurement discipline `ruined_barracks.rs` established.
//! None of the `Collision_*` nodes carry mesh geometry (plain Empties, same
//! as Ruined Barracks), so unlike `saint_arena.rs` there is nothing to hide
//! on load.
//!
//! ONE IMPORTANT WRINKLE: this hub has three exits, and their own node
//! rotations in the GLB do **not** reliably encode which way each one
//! continues (Ruined Barracks' `Entrance_FromFallenGate` node is likewise
//! rotation-identity despite the room clearly continuing in local -Z --
//! these Empties' rotations are Blender placement artifacts, not authored
//! travel directions). The forward vectors below were instead derived by
//! comparing each exit socket's position against the corridor floor it
//! leads to (e.g. `Exit_SoldiersPassage` at world (-13.18, -14.18) points
//! toward `Collision_SoldiersCorridorFloor` at (-14.66, -15.66), i.e.
//! continuing further in the -X/-Z direction) -- real geometry, not a
//! rotation value that turned out not to mean what it looked like it meant.
//!
//! SCOPE THIS PASS: only the Soldiers Passage route is opened (no blocking
//! collider spawned for `Collision_SoldiersGate`, so the passage is simply
//! walkable). The Forgotten Passage route and the Sealed Boss Route both
//! have no room built behind them yet, so their gates (`Collision_
//! ForgottenGate`, `Collision_NorthGate`) are spawned as permanently closed
//! static collision -- dead ends for now, not a toggleable mechanic. Their
//! corridor floor/rail pieces are still spawned (cheap, and correct if a
//! future pass opens either route), just currently unreachable.

use avian3d::prelude::{Collider, ColliderConstructor, ColliderConstructorHierarchy, RigidBody};
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

use super::components::*;
use super::connections::{self, RoomSocket};
use super::ruined_barracks;
use crate::collision::camera_occluder_layers;

// =========================================================
// ROOM ASSET + CONNECTION SOCKETS
// =========================================================

pub const SHATTERED_JUNCTION_ROOM_PATH: &str = "rooms/shattered_junction.glb";

/// This room's own entrance socket (`Entrance_FromBarracks`, world-measured
/// local position (0, 0.18, 15.2)). The hub's content (all three exits) sits
/// at local Z roughly -14..-16, so walking deeper means local Z decreasing --
/// same -Z "deeper" convention every room so far uses.
const ENTRANCE_SOCKET: RoomSocket =
    RoomSocket::new(Vec3::new(0.0, 0.18, 15.2), Vec3::new(0.0, 0.0, -1.0));

/// This room's exit toward Soldiers Passage (`Exit_SoldiersPassage`,
/// local (-13.1768, 1.3, -14.1768)). Forward derived from geometry -- see
/// the module doc comment -- rather than the node's own (misleading)
/// rotation: it points toward `Collision_SoldiersCorridorFloor`, further in
/// the -X/-Z direction.
///
/// Same fix as `ruined_barracks::exit_socket`: this room is not at the
/// world origin (it's placed by `connect_room` below), so the local socket
/// has to be run through this room's own `room_transform` before it's
/// actually in world space -- returning the raw local value here caused
/// Soldiers Passage (and, before this room existed, this exact bug in
/// `ruined_barracks::exit_socket` placed Shattered Junction itself) to
/// spawn near whatever room sits at the origin instead of past this one.
pub fn exit_socket_to_soldiers_passage() -> RoomSocket {
    let local = RoomSocket::new(
        Vec3::new(-13.1768, 1.3, -14.1768),
        Vec3::new(-1.0, 0.0, -1.0),
    );
    let room_transform = connections::connect_room(ruined_barracks::exit_socket(), ENTRANCE_SOCKET);
    RoomSocket::new(
        room_transform.transform_point(local.position),
        room_transform.rotation * local.forward,
    )
}

// =========================================================
// MAIN ROOM SPAWNER
// =========================================================

pub fn spawn_shattered_junction(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut room_graph: ResMut<super::graph::RoomGraph>,
    ruined_barracks_room: Query<Entity, With<RuinedBarracks>>,
) {
    // See `ruined_barracks::spawn_ruined_barracks`'s room-root comment for
    // the full explanation of what `connect_room` does; this is the same
    // pattern one room pair further down the level.
    let room_transform = connections::connect_room(ruined_barracks::exit_socket(), ENTRANCE_SOCKET);

    let room = commands
        .spawn((
            Name::new("ROOM - Shattered Junction"),
            Room,
            ShatteredJunction,
            // Pure connector room, no combat of its own -- `Unknown` renders
            // the same plain way `Combat` does on the exploration map (see
            // `RoomKind`'s doc comment), and unlike `Combat` doesn't promise
            // a fight that isn't here.
            RoomInfo::new("Shattered Junction", RoomKind::Unknown),
            // Matches this room's own authored `NavigationBounds` marker
            // (world center (0,0), half-extents (16.5, 17.5)) rather than a
            // hand-guessed box.
            RoomBounds::new(Vec2::new(0.0, 0.0), Vec2::new(16.5, 17.5)),
            room_transform,
            Visibility::default(),
        ))
        .id();

    if let Ok(barracks_entity) = ruined_barracks_room.single() {
        room_graph.connect(barracks_entity, room);
    }

    commands.entity(room).with_children(|parent| {
        // -----------------------------------------------------
        // VISIBLE ROOM SCENE
        // -----------------------------------------------------
        let junction_scene =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(SHATTERED_JUNCTION_ROOM_PATH));

        parent.spawn((
            Name::new("Shattered Junction Visual Scene"),
            WorldAssetRoot(junction_scene),
            Transform::default(),
            ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh)
                .with_default_layers(camera_occluder_layers()),
        ));

        // -----------------------------------------------------
        // DETERMINISTIC STATIC COLLISION
        //
        // Every box below is `Collider::cuboid(size_xyz[0], size_xyz[2],
        // size_xyz[1])` from the matching `Collision_*` node's own baked
        // `extras.size_xyz` -- that field is authored as [width, depth,
        // height], not engine [X, Y, Z], confirmed by cross-checking it
        // against Ruined Barracks' own already-working collider calls
        // before trusting it here.
        // -----------------------------------------------------

        parent.spawn((
            Name::new("Junction Floor Collision"),
            Transform::from_xyz(0.0, -0.17, 0.0),
            RigidBody::Static,
            Collider::cuboid(30.0, 0.34, 32.0),
        ));

        // Outer diamond walls (axis-aligned pieces + two diagonal corner
        // pieces where the room's silhouette cuts the corners).
        for (name, x, y, z, rotation, size) in [
            (
                "Junction Wall East",
                15.0,
                2.9,
                0.0,
                Quat::IDENTITY,
                Vec3::new(0.9, 5.8, 24.0),
            ),
            (
                "Junction Wall West",
                -15.0,
                2.9,
                0.0,
                Quat::IDENTITY,
                Vec3::new(0.9, 5.8, 24.0),
            ),
            (
                "Junction Wall North Left",
                -7.3,
                2.9,
                -16.0,
                Quat::IDENTITY,
                Vec3::new(7.4, 5.8, 0.9),
            ),
            (
                "Junction Wall North Right",
                7.3,
                2.9,
                -16.0,
                Quat::IDENTITY,
                Vec3::new(7.4, 5.8, 0.9),
            ),
            (
                "Junction Wall South Left",
                -6.9,
                2.9,
                16.0,
                Quat::IDENTITY,
                Vec3::new(8.2, 5.8, 0.9),
            ),
            (
                "Junction Wall South Right",
                6.9,
                2.9,
                16.0,
                Quat::IDENTITY,
                Vec3::new(8.2, 5.8, 0.9),
            ),
            (
                "Junction Wall Southeast Diagonal",
                13.0,
                2.9,
                14.0,
                Quat::from_xyzw(0.0, -0.382_683, 0.0, 0.923_88),
                Vec3::new(5.656_854, 5.8, 0.9),
            ),
            (
                "Junction Wall Southwest Diagonal",
                -13.0,
                2.9,
                14.0,
                Quat::from_xyzw(0.0, 0.382_683, 0.0, 0.923_88),
                Vec3::new(5.656_854, 5.8, 0.9),
            ),
        ] {
            parent.spawn((
                Name::new(name),
                Transform {
                    translation: Vec3::new(x, y, z),
                    rotation,
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(size.x, size.y, size.z),
            ));
        }

        // Four corner pillars.
        for (name, x, z) in [
            ("Junction Pillar NE", 4.6_f32, -7.2_f32),
            ("Junction Pillar NW", -4.6, -7.2),
            ("Junction Pillar SE", 7.0, 7.0),
            ("Junction Pillar SW", -7.0, 7.0),
        ] {
            parent.spawn((
                Name::new(name),
                Transform::from_xyz(x, 3.0, z),
                RigidBody::Static,
                Collider::cuboid(1.65, 6.0, 1.65),
            ));
        }

        // North Gate -- blocks Exit_SealedBossRoute. No room built behind
        // it yet; stays permanently closed (future work, not a toggle).
        parent.spawn((
            Name::new("Junction North Gate (sealed -- future boss route)"),
            Transform::from_xyz(0.0, 3.45, -15.05),
            RigidBody::Static,
            Collider::cuboid(6.4, 6.3, 0.45),
        ));

        // Forgotten Passage route -- corridor geometry spawned (cheap, and
        // ready if this route opens later), but its gate stays closed since
        // nothing is built past it yet.
        parent.spawn((
            Name::new("Junction Forgotten Gate (sealed -- future route)"),
            Transform {
                translation: Vec3::new(13.0, 3.2, -14.0),
                rotation: Quat::from_xyzw(0.0, -0.382_683, 0.0, 0.923_88),
                ..default()
            },
            RigidBody::Static,
            Collider::cuboid(5.8, 6.4, 0.45),
        ));
        parent.spawn((
            Name::new("Junction Forgotten Corridor Floor"),
            Transform {
                translation: Vec3::new(14.661_7, 0.92, -15.661_7),
                rotation: Quat::from_xyzw(0.0, -0.382_683, 0.0, 0.923_88),
                ..default()
            },
            RigidBody::Static,
            Collider::cuboid(5.6, 0.3, 4.8),
        ));
        for (name, x, z) in [
            ("Junction Forgotten Corridor Rail Left", 12.540_4, -17.783),
            ("Junction Forgotten Corridor Rail Right", 16.783, -13.540_4),
        ] {
            parent.spawn((
                Name::new(name),
                Transform {
                    translation: Vec3::new(x, 3.35, z),
                    rotation: Quat::from_xyzw(0.0, -0.382_683, 0.0, 0.923_88),
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(0.7, 4.9, 4.8),
            ));
        }
        parent.spawn((
            Name::new("Junction Forgotten Stairs"),
            Transform {
                translation: Vec3::new(11.939_3, 0.55, -12.939_3),
                rotation: Quat::from_xyzw(0.066_901, 0.376_79, 0.161_512, 0.909_652),
                ..default()
            },
            RigidBody::Static,
            Collider::cuboid(3.195_309, 0.3, 5.2),
        ));

        // Soldiers Passage route -- open this pass. Corridor floor + rails
        // + stairs spawned as real collision; deliberately NO collider for
        // `Collision_SoldiersGate` itself, so the passage is simply
        // walkable rather than needing a clear/unlock condition this room
        // has no encounter of its own to gate on.
        parent.spawn((
            Name::new("Junction Soldiers Corridor Floor"),
            Transform {
                translation: Vec3::new(-14.661_7, 0.92, -15.661_7),
                rotation: Quat::from_xyzw(0.0, 0.382_683, 0.0, 0.923_88),
                ..default()
            },
            RigidBody::Static,
            Collider::cuboid(5.6, 0.3, 4.8),
        ));
        for (name, x, z) in [
            ("Junction Soldiers Corridor Rail Left", -16.783, -13.540_4),
            ("Junction Soldiers Corridor Rail Right", -12.540_4, -17.783),
        ] {
            parent.spawn((
                Name::new(name),
                Transform {
                    translation: Vec3::new(x, 3.35, z),
                    rotation: Quat::from_xyzw(0.0, 0.382_683, 0.0, 0.923_88),
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(0.7, 4.9, 4.8),
            ));
        }
        parent.spawn((
            Name::new("Junction Soldiers Stairs"),
            Transform {
                translation: Vec3::new(-11.939_3, 0.55, -12.939_3),
                rotation: Quat::from_xyzw(0.161_512, 0.909_652, 0.066_901, 0.376_79),
                ..default()
            },
            RigidBody::Static,
            Collider::cuboid(3.195_309, 0.3, 5.2),
        ));

        // -----------------------------------------------------
        // ROOM SYSTEM MARKERS
        //
        // No RoomConnector::Entrance/VoidKillZone here -- same
        // single-global-Entrance reasoning `ruined_barracks.rs` already
        // documents (`void::reset_player_from_void` takes the first
        // Entrance/VoidKillZone match with no per-room awareness). Exit
        // connectors aren't consumed ambiguously anywhere, so, like Ruined
        // Barracks' own exit, these are safe to add as plain descriptive
        // markers.
        // -----------------------------------------------------

        parent.spawn((
            Name::new("Junction Entrance (from Ruined Barracks)"),
            Transform::from_xyz(0.0, 0.18, 15.2),
            Visibility::default(),
        ));

        parent.spawn((
            Name::new("Junction Exit (to Soldiers Passage)"),
            RoomConnector {
                kind: ConnectorKind::Exit,
            },
            Transform::from_xyz(-13.176_8, 1.3, -14.176_8),
            Visibility::default(),
        ));
    });

    println!("Shattered Junction spawned -- Soldiers Passage route open.");
}
