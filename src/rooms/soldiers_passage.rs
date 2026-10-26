//! Soldiers Passage -- the elite encounter room reached through Shattered
//! Junction. The GLB (`soldiers_passage.glb`) is a single long corridor/hall
//! running from the entrance (local Z ~0) down to a broken ledge at local
//! Z=-68.3 (`Collision_FloatingRubble_LaunchLedge`, right before
//! `Exit_ToSaintBrokenSiege`/`Socket_Exit_ToFloatingRubble`, this room's own
//! `exit_socket()` -- connected onward to `rubble_bridge`), passing through
//! a normal-combat muster hall, a side loot room, and finally the Black
//! Banner Knight's own elite arena.
//!
//! Every position/size below was read directly out of the exported
//! `soldiers_passage.glb`. Unlike `ruined_barracks.glb`/`shattered_junction.
//! glb`, this GLB's `Collision_*` nodes carry no baked `extras.size_xyz` --
//! only a bare `ruinbound_marker`/`marker_type` tag -- so sizes here are the
//! node's own `scale`, doubled (Blender's Empty-display half-extent
//! convention, same doubling every room in this project already uses; no
//! axis reordering needed here since there's no `size_xyz` field reordering
//! it, confirmed by cross-checking `shattered_junction.glb`'s own
//! `Collision_Floor` scale against its `size_xyz`, which agreed exactly once
//! doubled).
//!
//! NO PERIMETER WALLS: every `Collision_*` node in this GLB is a floor
//! slab -- there is no wall/rail collision at all outside the muster hall's
//! decorative masonry (which, like every other purely visual mesh in this
//! project, carries no collider unless a matching `Collision_*` node backs
//! it, and none does here). That reads as deliberate: this passage and the
//! Elite Arena are floating/broken platforms over a void, matching the
//! project's stated "floating ruins and dangerous voids" art direction, and
//! the existing single global `VoidKillZone`/`reset_player_from_void`
//! (Fallen Gate's own) already catches a player who walks off an edge. If
//! that reads as an accidental gap in playtesting rather than the intended
//! feel, the fix is adding wall colliders here, not touching the void
//! system.
//!
//! SCOPE THIS PASS: only `EnemySpawn_Wave1_*` (4 melee) + `RangedSpawn_
//! Left/Right` (2 ranged) are spawned as the muster hall's encounter,
//! exactly mirroring Ruined Barracks' own "Stage 1 scope only" precedent --
//! `EnemySpawn_Wave2_*` exists in the GLB and is deliberately left unused
//! for a follow-up pass. The Black Banner Knight is spawned once, at the
//! `EliteSpawn` center marker; `EliteSpawn_01..04` (flanking positions)
//! exist in the GLB but aren't used yet -- ready for a future added/adds
//! pass, not this one.

use avian3d::prelude::{Collider, ColliderConstructor, ColliderConstructorHierarchy, RigidBody};
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

use super::components::*;
use super::connections::{self, RoomSocket};
use super::shattered_junction;
use crate::collision::camera_occluder_layers;
use crate::enemies::{self, EnemyEncounterConfig, LootFloor};
use crate::loot::{LootTableId, spawn_loot_chest};
use crate::ui::spawn_enemy_health_bar;

// =========================================================
// ROOM ASSET + CONNECTION SOCKET
// =========================================================

pub const SOLDIERS_PASSAGE_ROOM_PATH: &str = "rooms/soldiers_passage.glb";

/// This room's own entrance socket (`Socket_Entrance_FromShatteredJunction`,
/// local (0, 0.18, 0) -- the precise connection point; `Entrance_
/// FromShatteredJunction` a couple of units further in at Z=-1 is a separate,
/// coarser threshold marker, not the socket). The room's content runs all
/// the way to local Z=-70, so walking deeper means local Z decreasing, same
/// as every other room.
const ENTRANCE_SOCKET: RoomSocket =
    RoomSocket::new(Vec3::new(0.0, 0.18, 0.0), Vec3::new(0.0, 0.0, -1.0));

/// Distinct from Ruined Barracks' `encounter_id: 1` and the default `0`
/// (see `EnemyEncounterConfig::around`) so damaging an enemy in one room
/// can't force-activate an unrelated one elsewhere --
/// `activate_damaged_encounters` (`enemies::mod`) matches purely by this
/// numeric id, with no per-room/spatial scoping.
const MUSTER_HALL_ENCOUNTER_ID: u32 = 2;
const ELITE_ENCOUNTER_ID: u32 = 3;

/// Elite loot floor override, matching the same EXPERIMENTAL value
/// `test_grounds.rs`'s F2 debug spawn already uses -- without this, the
/// Knight's `LootTableId::ELITE_ENEMY` table defaults to floor 1
/// (`enemies::handle_enemy_death`) and its higher-tier (Epic+) entries,
/// gated at a higher `min_floor`, could never actually drop from killing
/// him for real.
const ELITE_LOOT_FLOOR: u32 = 5;

/// This room's own exit socket, toward the rubble bridge
/// (`Socket_Exit_ToFloatingRubble`, local (0, 0.18, -69.0) -- the coarser
/// `Exit_ToSaintBrokenSiege` marker a unit further out at Z=-70 is the same
/// kind of secondary threshold marker as every other room's `Entrance_*`/
/// `Socket_Entrance_*` pair, not the socket itself). Forward continues the
/// corridor's own direction of travel (-Z), same reasoning as every other
/// exit socket in this project -- not read from the marker's own rotation,
/// which is not reliable (see `shattered_junction.rs`'s module doc comment).
///
/// Follows the corrected pattern established by `ruined_barracks::
/// exit_socket`/`shattered_junction::exit_socket_to_soldiers_passage`: this
/// room does NOT sit at the world origin, so its own `room_transform` must
/// be recomputed here and used to convert the local socket into true world
/// space before returning it -- returning the raw local value directly (as
/// Fallen Gate's `exit_socket` alone is allowed to do, since only it sits at
/// `Transform::default()`) was the exact bug that caused the "spawned in
/// the floor" report this pattern was introduced to fix.
pub fn exit_socket() -> RoomSocket {
    let local = RoomSocket::new(Vec3::new(0.0, 0.18, -69.0), Vec3::new(0.0, 0.0, -1.0));
    let room_transform = connections::connect_room(
        shattered_junction::exit_socket_to_soldiers_passage(),
        ENTRANCE_SOCKET,
    );
    RoomSocket::new(
        room_transform.transform_point(local.position),
        room_transform.rotation * local.forward,
    )
}

// =========================================================
// MAIN ROOM SPAWNER
// =========================================================

pub fn spawn_soldiers_passage(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut room_graph: ResMut<super::graph::RoomGraph>,
    shattered_junction_room: Query<Entity, With<ShatteredJunction>>,
) {
    let room_transform = connections::connect_room(
        shattered_junction::exit_socket_to_soldiers_passage(),
        ENTRANCE_SOCKET,
    );

    let room = commands
        .spawn((
            Name::new("ROOM - Soldiers Passage"),
            Room,
            SoldiersPassage,
            RoomInfo::new("Soldiers Passage", RoomKind::Elite),
            // Matches this room's own authored `NavigationBounds` marker
            // (world center (-1.5, -34), half-extents (12, 34)).
            RoomBounds::new(Vec2::new(-1.5, -34.0), Vec2::new(12.0, 34.0)),
            room_transform,
            Visibility::default(),
        ))
        .id();

    if let Ok(junction_entity) = shattered_junction_room.single() {
        room_graph.connect(junction_entity, room);
    }

    commands.entity(room).with_children(|parent| {
        // -----------------------------------------------------
        // VISIBLE ROOM SCENE
        // -----------------------------------------------------
        let passage_scene =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(SOLDIERS_PASSAGE_ROOM_PATH));

        parent.spawn((
            Name::new("Soldiers Passage Visual Scene"),
            WorldAssetRoot(passage_scene),
            Transform::default(),
            ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh)
                .with_default_layers(camera_occluder_layers()),
        ));

        // -----------------------------------------------------
        // DETERMINISTIC STATIC COLLISION -- floor slabs only, see the
        // module doc comment for why there's no perimeter wall collision.
        // Sizes are each `Collision_*` node's own `scale`, doubled.
        // -----------------------------------------------------
        for (name, x, y, z, size) in [
            (
                "Passage Floor Entrance",
                0.0,
                -0.18,
                -7.0,
                Vec3::new(6.0, 0.36, 14.0),
            ),
            (
                "Passage Floor Muster Hall",
                0.0,
                -0.18,
                -31.0,
                Vec3::new(18.0, 0.36, 20.0),
            ),
            (
                "Passage Floor Main",
                0.0,
                -0.18,
                -31.0,
                Vec3::new(18.0, 0.36, 62.0),
            ),
            (
                "Passage Floor Loot Room",
                -12.5,
                -0.18,
                -31.0,
                Vec3::new(8.0, 0.36, 14.0),
            ),
            (
                "Passage Floor Elite Approach",
                0.0,
                -0.18,
                -43.5,
                Vec3::new(18.0, 0.36, 5.0),
            ),
            (
                "Passage Floor Elite Arena",
                0.0,
                -0.18,
                -56.0,
                Vec3::new(22.0, 0.36, 20.0),
            ),
            (
                "Passage Floor Floating Rubble Launch Ledge",
                0.0,
                -0.18,
                -68.3,
                Vec3::new(11.0, 0.36, 3.0),
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
        // connected room (see `ruined_barracks.rs`/`shattered_junction.rs`).
        // -----------------------------------------------------
        parent.spawn((
            Name::new("Passage Entrance (from Shattered Junction)"),
            Transform::from_xyz(0.0, 0.18, 0.0),
            Visibility::default(),
        ));

        parent.spawn((
            Name::new("Passage Exit (to the Rubble Bridge)"),
            RoomConnector {
                kind: ConnectorKind::Exit,
            },
            Transform::from_xyz(0.0, 0.18, -69.0),
            Visibility::default(),
        ));

        // -----------------------------------------------------
        // REWARD -- loot room chest, matching Ruined Barracks' single
        // guaranteed COMMON_CHEST precedent (no floor-progression system
        // exists yet to justify escalating this).
        // -----------------------------------------------------
        spawn_loot_chest(
            parent,
            &mut meshes,
            &mut materials,
            Vec3::new(-12.5, 0.35, -35.0),
            LootTableId::COMMON_CHEST,
            1,
        );
    });

    // -----------------------------------------------------
    // MUSTER HALL -- Wave 1 melee + ranged, one shared encounter so
    // damaging any one force-activates the rest (see
    // `enemies::activate_damaged_encounters`). Positions converted to
    // world space via `room_transform.transform_point` since these are
    // independent top-level entities, not children of `room` -- same
    // pattern `ruined_barracks.rs` already established.
    //
    // `activation_half_extents` is a WORLD-space axis-aligned box
    // (`EnemyAwareness::is_in_activation_area`, enemies::mod), but this
    // room sits at a genuine 45-degree yaw once connected (Shattered
    // Junction's Soldiers Passage exit is a diagonal corridor) -- unlike
    // Ruined Barracks, whose own rotation (180 degrees) keeps world axes
    // parallel to local ones. Sized generously (16x16 rather than the
    // room's tighter local 9x12 half-extents) so the box still fully
    // covers the rotated footprint's corners rather than clipping them.
    // -----------------------------------------------------
    let muster_hall_encounter = EnemyEncounterConfig {
        encounter_id: MUSTER_HALL_ENCOUNTER_ID,
        activation_center: room_transform.transform_point(Vec3::new(0.0, 0.0, -22.0)),
        activation_half_extents: Vec2::new(16.0, 16.0),
        aggro_extension: 25.0,
    };

    const WAVE1_SPAWNS: [Vec3; 4] = [
        Vec3::new(-2.0, 0.25, -14.0),
        Vec3::new(2.0, 0.25, -14.0),
        Vec3::new(-2.2, 0.25, -17.0),
        Vec3::new(2.2, 0.25, -17.0),
    ];

    for local_position in WAVE1_SPAWNS {
        let enemy = enemies::spawn_ruinbound_footman(
            &mut commands,
            &mut meshes,
            &mut materials,
            room_transform.transform_point(local_position),
            muster_hall_encounter,
        );
        spawn_enemy_health_bar(&mut commands, enemy);
    }

    const RANGED_SPAWNS: [Vec3; 2] = [Vec3::new(-4.8, 0.25, -31.0), Vec3::new(4.8, 0.25, -31.0)];

    for local_position in RANGED_SPAWNS {
        let enemy = enemies::spawn_gatebow_marksman(
            &mut commands,
            &mut meshes,
            &mut materials,
            room_transform.transform_point(local_position),
            muster_hall_encounter,
        );
        spawn_enemy_health_bar(&mut commands, enemy);
    }

    // -----------------------------------------------------
    // ELITE ARENA -- the Black Banner Knight. Own encounter id and a
    // tighter activation box centered on the approach + arena (local Z
    // -36..-64ish) so clearing/engaging Muster Hall doesn't pull him in
    // early, and so he doesn't chase all the way back into it either.
    // Sized generously (18x18, same rotated-footprint reasoning as Muster
    // Hall's own box above) rather than the room's tighter local 11x14.
    //
    // KNOWN CAVEAT: these two boxes are close enough together (centers 28
    // units apart along the corridor, each needing >15 units of reach to
    // cover its own rotated footprint) that they unavoidably overlap by a
    // margin near the Elite Approach -- a player lingering there may cause
    // both encounters to activate together rather than strictly Muster
    // Hall-then-Knight. `EnemyAwareness::is_in_activation_area` (enemies::
    // mod) is a plain world-axis-aligned box with no room-rotation
    // awareness, and Soldiers Passage is the first room connected at a
    // genuine (non-90-degree-multiple) yaw, so this isn't fixable by
    // resizing alone -- flagging for playtesting rather than widening the
    // encounter system's math unprompted.
    // -----------------------------------------------------
    let knight_encounter = EnemyEncounterConfig {
        encounter_id: ELITE_ENCOUNTER_ID,
        activation_center: room_transform.transform_point(Vec3::new(0.0, 0.0, -50.0)),
        activation_half_extents: Vec2::new(18.0, 18.0),
        aggro_extension: 25.0,
    };

    let knight = enemies::spawn_black_banner_knight(
        &mut commands,
        &mut meshes,
        &mut materials,
        room_transform.transform_point(Vec3::new(0.0, 0.25, -56.0)),
        knight_encounter,
    );
    commands.entity(knight).insert(LootFloor(ELITE_LOOT_FLOOR));
    spawn_enemy_health_bar(&mut commands, knight);

    println!(
        "Soldiers Passage spawned -- 4 melee + 2 ranged, Black Banner Knight in the Elite Arena."
    );
}
