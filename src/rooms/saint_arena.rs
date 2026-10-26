use avian3d::prelude::{Collider, ColliderConstructor, ColliderConstructorHierarchy, RigidBody};
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use super::components::*;
use super::connections::{self, RoomSocket};
use super::rubble_bridge;
use crate::collision::camera_occluder_layers;
use crate::enemies::{self, EnemyEncounterConfig};
use crate::ui::spawn_enemy_health_bar;

// =========================================================
// ARENA ASSET + CONNECTION SOCKET
// =========================================================

pub const ARENA_ROOM_PATH: &str = "rooms/saint_broken_siege_room.glb";

/// This room's own entrance socket, matching the GLB's authored
/// `SPAWN_PLAYER_Entrance` marker (local (0, 1.3, 22.32) -- see "Player
/// Spawn" below). Forward continues into the arena (-Z), same as every
/// other room's entrance.
const ENTRANCE_SOCKET: RoomSocket =
    RoomSocket::new(Vec3::new(0.0, 1.3, 22.32), Vec3::new(0.0, 0.0, -1.0));

/// Distinct from every other currently-active room's encounter id (Ruined
/// Barracks=1, Soldiers Passage's Muster Hall=2 / Elite Arena=3) so
/// damaging the Saint can't cross-activate an unrelated encounter --
/// `activate_damaged_encounters` (`enemies::mod`) matches purely by this
/// numeric id, with no per-room/spatial scoping. Distinct also from
/// `arena::ArenaPlugin`'s own standalone `encounter_id: 1` -- that plugin
/// is a separate, never-simultaneously-active Startup path (see its own
/// module comment), so no collision there either, but a fresh id costs
/// nothing and rules it out for good.
const BOSS_ENCOUNTER_ID: u32 = 4;

/// Node-name prefixes exported by Blender that are reference-only and must
/// never be visible in gameplay: `COLLIDER_*` boxes measure our deterministic
/// Avian colliders below, and `TRIGGER_Void_Kill_Zone` is a large flat plane
/// used only to author the void-kill height.
const HIDDEN_REFERENCE_PREFIXES: [&str; 2] = ["COLLIDER_", "TRIGGER_"];

// =========================================================
// MAIN ROOM SPAWNER
// =========================================================

/// Spawns the Saint of the Broken Siege boss arena, connected onto the end
/// of the Rubble Bridge.
///
/// All positions/sizes below were measured directly from the exported
/// `saint_broken_siege_room.glb` (node transforms and POSITION accessor
/// bounds), not approximated. Every collider node in the room is a root-level
/// node with no rotation or scale, so a plain translated cuboid is an exact
/// match for each one -- these are all local-space measurements, spawned as
/// children of the room root below, so they inherit `room_transform`
/// automatically through ordinary Bevy transform propagation and need no
/// manual conversion themselves. Only the independently-spawned boss (which
/// does not inherit the room root's transform) is converted by hand, via
/// `room_transform.transform_point`.
///
/// Previously spawned at `Transform::default()` (the world origin) as a
/// standalone, unconnected room -- safe only because it was never spawned
/// alongside Fallen Gate at the same time. Now that it joins the connected
/// chain (Fallen Gate -> ... -> Rubble Bridge -> here), it needs a real
/// `room_transform` like every other connected room, following the corrected
/// pattern established by `ruined_barracks::exit_socket`/
/// `shattered_junction::exit_socket_to_soldiers_passage`/
/// `soldiers_passage::exit_socket`/`rubble_bridge::exit_socket`.
pub fn spawn_saint_arena(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut room_graph: ResMut<super::graph::RoomGraph>,
    rubble_bridge_room: Query<Entity, With<RubbleBridge>>,
) {
    let room_transform = connections::connect_room(rubble_bridge::exit_socket(), ENTRANCE_SOCKET);

    let room = commands
        .spawn((
            Name::new("ROOM - Saint of the Broken Siege Arena"),
            Room,
            SaintArena,
            RoomInfo::new("Saint of the Broken Siege", RoomKind::Boss),
            // Covers the main circular floor (radius ~26.1, see "Arena
            // Floor Collision" below) plus the raised rear platform's
            // extension past it -- see `RoomBounds`'s doc comment for why
            // this doesn't need to be exact.
            RoomBounds::new(Vec2::new(0.0, 0.0), Vec2::new(28.0, 28.0)),
            room_transform,
            Visibility::default(),
        ))
        .id();

    if let Ok(bridge_entity) = rubble_bridge_room.single() {
        room_graph.connect(bridge_entity, room);
    }

    commands.entity(room).with_children(|parent| {
        // -----------------------------------------------------
        // VISIBLE ROOM SCENE
        // -----------------------------------------------------
        // The GLB's root nodes carry no rotation/scale and are already
        // authored in engine-space units, so no correcting transform is
        // needed here.

        let arena_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(ARENA_ROOM_PATH));

        parent
            .spawn((
                Name::new("Saint Arena Visual Scene"),
                WorldAssetRoot(arena_scene),
                Transform::default(),
                ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh)
                    .with_default_layers(camera_occluder_layers()),
            ))
            .observe(hide_reference_only_meshes);

        // -----------------------------------------------------
        // DETERMINISTIC STATIC COLLISION
        //
        // Matches the confirmed statue-blocker pattern (fallen_gate.rs):
        // one clean convex box per structural piece instead of collision
        // derived from the decorative/faceted visual meshes.
        // -----------------------------------------------------

        // Main circular arena floor.
        //
        // Re-measured 2026-08-30 from the resized saint_broken_siege_room.glb
        // (COLLIDER_Arena_Floor node, world-space translation/size). The room
        // was scaled down by 0.72 on X/Z only in Blender; every collider below
        // is read directly off the new export rather than hand-scaled, per the
        // project rule of measuring from the GLB instead of guessing.
        // The floor isn't one flat mesh -- it's ~157 individual
        // FloorStone_* tiles (a broken/weathered ruin floor), each sitting
        // at a slightly different height. Sampling all of them puts their
        // top surface at 0.057-0.093, median 0.076; the flat
        // COLLIDER_Arena_Floor reference box's own top (0.02) sits below
        // every single tile, so the player's feet visually clipped into
        // the floor everywhere they stood. Raised the collider (same 1.2
        // thickness) so its top matches the tiles' median instead.
        parent.spawn((
            Name::new("Arena Floor Collision"),
            Transform::from_xyz(0.0, -0.524, 0.0),
            RigidBody::Static,
            Collider::cuboid(52.272, 1.2, 52.272),
        ));

        // Raised rear boss platform.
        parent.spawn((
            Name::new("Boss Platform Collision"),
            Transform::from_xyz(0.0, 1.5, -17.784),
            RigidBody::Static,
            Collider::cuboid(19.008, 3.0, 7.632),
        ));

        // Four large pillars.
        let pillar_positions = [
            Vec3::new(14.458_92, 7.5, -14.458_92),
            Vec3::new(-14.458_92, 7.5, -14.458_92),
            Vec3::new(-14.458_92, 7.5, 14.458_92),
            Vec3::new(14.458_92, 7.5, 14.458_92),
        ];

        for (index, position) in pillar_positions.into_iter().enumerate() {
            parent.spawn((
                Name::new(format!("Arena Pillar {index} Collision")),
                Transform::from_translation(position),
                RigidBody::Static,
                Collider::cuboid(3.8916, 15.0, 3.8916),
            ));
        }

        // Stairs connecting the main floor to the raised platform. Not yet
        // used by the Saint (he stays on the main floor for this pass), but
        // the player can walk up there, so they need real collision now
        // rather than leaving an uncollided gap in the floor.
        //
        // These were originally a single axis-aligned box spanning the full
        // floor-to-platform height, which made the stairs' front face
        // perfectly vertical -- unclimbable, since `character_movement`
        // (src/player/mod.rs) moves the player through Avian's
        // `MoveAndSlide` controller, which slides along whatever surface it
        // hits but has no discrete step-up/stair-climb logic.
        //
        // The first fix tilted that same box into a ramp by rotating it
        // about its centerline, but a box's thickness is centered on that
        // line -- rotating it swings the actual walkable top face up and
        // off to the side of where the centerline points, leaving a lip at
        // the floor seam of roughly half the thickness. That lip was still
        // enough to block `MoveAndSlide` outright and only jumping cleared
        // it.
        //
        // Solved properly below: `bottom` and `top` are the exact points
        // the WALKABLE SURFACE must pass through (floor height matches the
        // corrected Arena Floor Collision above; platform height matches
        // COLLIDER_Boss_Platform / Boss_Platform_Faceted's confirmed top at
        // 3.0). The rotation and the box's center are then solved so its
        // actual +Y face -- not its centerline -- lands exactly on that
        // line, with no residual lip at either end. The resulting incline
        // is about 23.6 degrees, comfortably under the ~53 degree slope
        // `character_movement` already treats as walkable ground (it
        // accepts any hit normal with normal.y > 0.6), so the player walks
        // straight up it like any other sloped floor.
        let stairs_width = 6.624;
        let stairs_thickness = 1.2;
        let stairs_floor_y = 0.076; // matches the corrected floor collider top
        let stairs_platform_y = 3.0;
        let stairs_near_z = -10.44; // floor-side edge of the stairs footprint
        let stairs_far_z = -17.208; // platform-side edge of the stairs footprint

        for (name, x) in [
            ("Arena Stairs Left Collision", -8.64_f32),
            ("Arena Stairs Right Collision", 8.64_f32),
        ] {
            let bottom = Vec3::new(x, stairs_floor_y, stairs_near_z);
            let top = Vec3::new(x, stairs_platform_y, stairs_far_z);
            let along = top - bottom;
            let slope_length = along.length();
            // Rotation about X that maps local -Z onto `along` (both lie in
            // the Y/Z plane since bottom/top share the same X).
            let rotation = Quat::from_rotation_x(along.y.atan2(-along.z));
            // The box's local +Y face, rotated into world space: this is the
            // walkable top face's outward normal.
            let normal = rotation * Vec3::Y;
            // Centering the box on the bottom/top midpoint would put that
            // point on the CENTERLINE, half a thickness below the actual top
            // face; shift the center back along the normal to compensate so
            // the top face itself passes through `bottom` and `top`.
            let center = (bottom + top) * 0.5 - normal * (stairs_thickness * 0.5);

            parent.spawn((
                Name::new(name),
                Transform::from_translation(center).with_rotation(rotation),
                RigidBody::Static,
                Collider::cuboid(stairs_width, stairs_thickness, slope_length),
            ));
        }

        // -----------------------------------------------------
        // ROOM SYSTEM MARKERS -- plain descriptive markers only. Both of
        // these carried real `RoomConnector::Entrance`/`VoidKillZone`
        // components in the room's original standalone form (safe then,
        // since it was never spawned alongside Fallen Gate at the same
        // time); now that this room joins the connected chain, only Fallen
        // Gate's own originals may carry those components --
        // `rooms::void::reset_player_from_void` matches an `Entrance` by
        // `.find_map` (first match, not `.single()`) and a `VoidKillZone` by
        // `.next()` on an unordered query, so a second real one anywhere
        // else makes behavior ambiguous. Same reasoning already applied to
        // every other connected room (see `ruined_barracks.rs`/
        // `shattered_junction.rs`/`soldiers_passage.rs`/`rubble_bridge.rs`).
        // -----------------------------------------------------

        // Matches the GLB's authored SPAWN_PLAYER_Entrance (x=0, z=22.32). The
        // marker's own Y (0.2) sits at floor level; raised here to clear the
        // player capsule (radius 0.45, half-height 0.95) instead of spawning
        // it intersecting the floor.
        parent.spawn((
            Name::new("Player Spawn"),
            Transform::from_xyz(0.0, 1.3, 22.32),
            Visibility::default(),
        ));

        // Matches the GLB's authored MARKER_Void_Kill_Height, corroborated by
        // TRIGGER_Void_Kill_Zone's top face (plane centered at y=-14.5,
        // half-thickness 0.5) landing on the same y=-14.0 value. No longer a
        // real `VoidKillZone` (see the marker block comment above) -- the
        // single global one is Fallen Gate's own, which by this point in the
        // chain sits at a world Y well below every downstream room's floor
        // (each room's floor Y only ever climbs, per `connections::
        // connect_room`'s Y-carries-through behavior), so it still correctly
        // catches a player who falls off this arena's edge; just later than
        // an arena-local kill plane would. Already the same accepted
        // simplification Soldiers Passage's own module doc comment
        // discloses, not something newly introduced here.
        parent.spawn((
            Name::new("Void Kill Zone (reference only -- see comment)"),
            Transform::from_xyz(0.0, -20.0, 0.0),
            Visibility::default(),
        ));
    });

    // -----------------------------------------------------
    // THE SAINT OF THE BROKEN SIEGE -- spawned as an independent top-level
    // entity (not a child of `room`), same as every other room's own
    // enemies, since `EnemyEncounterConfig`'s activation box and the boss's
    // own movement/combat systems all expect a world-space position rather
    // than one that rides along with a parent transform. Converted through
    // `room_transform.transform_point` from the GLB's authored SPAWN_BOSS-
    // adjacent main-floor position (see `arena::ArenaPlugin`'s own prior
    // placement of this same boss for the values reused here) since this
    // room no longer sits at the world origin.
    // -----------------------------------------------------
    let boss_encounter = EnemyEncounterConfig {
        encounter_id: BOSS_ENCOUNTER_ID,
        activation_center: room_transform.transform_point(Vec3::new(0.0, 0.0, 0.0)),
        activation_half_extents: Vec2::new(26.136, 26.136),
        aggro_extension: 20.0,
    };

    let boss_entity = enemies::spawn_saint_of_the_broken_siege(
        &mut commands,
        &asset_server,
        room_transform.transform_point(Vec3::new(0.0, 2.7, -5.76)),
        boss_encounter,
    );

    spawn_enemy_health_bar(&mut commands, boss_entity);

    println!("Saint of the Broken Siege arena spawned -- boss active on the main floor.");
}

// =========================================================
// REFERENCE-MESH VISIBILITY
// =========================================================

fn hide_reference_only_meshes(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    for descendant in children.iter_descendants(scene_ready.entity) {
        let Ok(name) = names.get(descendant) else {
            continue;
        };

        let is_reference_only = HIDDEN_REFERENCE_PREFIXES
            .iter()
            .any(|prefix| name.as_str().starts_with(prefix));

        if is_reference_only {
            commands.entity(descendant).insert(Visibility::Hidden);
        }
    }
}
