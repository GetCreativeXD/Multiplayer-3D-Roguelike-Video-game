//! Ruined Barracks -- the second connected room. The player reaches it by
//! walking through Fallen Gate's rear gatehouse arch (`rooms::fallen_gate`).
//! The two rooms' geometry is joined by `rooms::connections::connect_room`
//! -- see that module's doc comment for the general pattern, and the
//! room-root comment in `spawn_ruined_barracks` below for exactly how this
//! room uses it. Future rooms should follow the same pattern: expose an
//! `exit_socket()` from the room being connected *from* (see
//! `fallen_gate::exit_socket`), define the new room's own local entrance
//! socket, and call `connect_room` once at the top of the new room's
//! spawner -- no hand-derived rotation/offset math needed per room pair.
//!
//! Every position/size/rotation in this file was measured directly from
//! the exported `ruined_barracks.glb` -- its Collision_*, EnemySpawn_*,
//! RangedSpawn_*, Entrance_*, Exit_* and RewardSpawn nodes were authored
//! specifically for this room (each carries `gameplay_marker`/
//! `collision_role` metadata confirming its purpose), not approximated by
//! hand. None of them carry actual mesh geometry -- they're plain empty
//! transform nodes -- so unlike `saint_arena.rs` there is nothing to hide
//! on load; the visible scene already contains no reference-only meshes.
//!
//! Stage 1 scope only, per the requested build-out order: normal combat
//! (four ground-floor melee enemies, two platform ranged enemies, a
//! locked rear gate that opens on clear, one reward chest) with the two
//! signal bells and the bell-triggered reinforcement wave
//! (EnemySpawn_Wave2_*, EliteSpawn) deliberately left unused. Those
//! markers already exist in the GLB and are ready for a follow-up pass
//! once this normal-combat loop is confirmed working.

use avian3d::prelude::{Collider, ColliderConstructor, ColliderConstructorHierarchy, RigidBody};
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

use super::components::*;
use super::connections::{self, RoomSocket};
use super::fallen_gate;
use crate::collision::camera_occluder_layers;
use crate::enemies::{self, EnemyEncounterConfig, EnemyNavigationGrid, EnemyStepPlatformConfig};
use crate::loot::{LootTableId, spawn_loot_chest};
use crate::ui::spawn_enemy_health_bar;

// =========================================================
// ROOM ASSET + CONNECTION SOCKET
// =========================================================

pub const BARRACKS_ROOM_PATH: &str = "rooms/ruined_barracks.glb";

/// This room's own entrance socket, measured directly in the GLB's local
/// space. `Entrance_FromFallenGate` sits at local Z=+21 (the front wall,
/// facing Fallen Gate); walking deeper into the room means local Z
/// *decreasing* (the rear gate at local Z=-20.3 is the far end), so the
/// entrance's direction of travel, continuing further into the room, is
/// local -Z. Y=0.0 is `Collision_Floor`'s own measured top surface
/// (center Y=-0.18, half-thickness 0.18 -- see that collider below) --
/// `connect_room` uses this to line this room's floor up flush with
/// whatever floor height the room being connected from has, even though
/// this room was authored independently at its own local ground level.
const ENTRANCE_SOCKET: RoomSocket =
    RoomSocket::new(Vec3::new(0.0, 0.0, 21.0), Vec3::new(0.0, 0.0, -1.0));

/// This room's own exit socket, for whatever connects onward from here
/// (Shattered Junction, see `shattered_junction.rs`). Matches the GLB's own
/// authored `Exit_NextRoom` marker (local (0.0, 2.43, -19.0)) -- already
/// recorded in the scene as the "Barracks Exit (future room)"
/// `RoomConnector::Exit` marker below, now given a real socket function.
/// Forward continues in the same -Z direction as this room's own entrance
/// (deeper into the level, not back toward Fallen Gate).
///
/// UNLIKE `fallen_gate::exit_socket`, this can't just return the raw local
/// marker value: Fallen Gate is spawned with `Transform::default()`
/// (identity), so its local space happens to already be world space --
/// this room is not (it's placed by `connect_room` below, with a real
/// rotation/translation), so its exit socket has to be run through that
/// same placement before it's actually in world space. Recomputing
/// `room_transform` here is cheap and pure (no ECS access needed) and
/// keeps this a simple no-argument function the next room can call, same
/// as `fallen_gate::exit_socket`'s calling convention.
pub fn exit_socket() -> RoomSocket {
    let local = RoomSocket::new(Vec3::new(0.0, 2.43, -19.0), Vec3::new(0.0, 0.0, -1.0));
    let room_transform = connections::connect_room(fallen_gate::exit_socket(), ENTRANCE_SOCKET);
    RoomSocket::new(
        room_transform.transform_point(local.position),
        room_transform.rotation * local.forward,
    )
}

// =========================================================
// MAIN ROOM SPAWNER
// =========================================================

pub fn spawn_ruined_barracks(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut navigation_grid: ResMut<EnemyNavigationGrid>,
    mut room_graph: ResMut<super::graph::RoomGraph>,
    fallen_gate_room: Query<Entity, With<FallenGate>>,
) {
    // -----------------------------------------------------
    // ROOM ROOT -- WORLD PLACEMENT
    //
    // `connect_room` (rooms::connections) joins this room's entrance
    // socket onto Fallen Gate's exit socket -- the back face of its
    // gatehouse pillars, facing further into the level (+Z), at the
    // gatehouse's own floor-top height (see `fallen_gate::exit_socket`).
    // It works out the yaw (here, 180 degrees: Fallen Gate progresses
    // along +Z, while this room's own "deeper" direction is local -Z, so
    // the room has to be turned to face the opposite way) and the
    // translation (including the Y lift that reconciles this room's own
    // Y=0 floor-top convention with Fallen Gate's elevated courtyard
    // floor) with no numbers hand-derived here -- see `connections.rs`'s
    // doc comment for the full math. Every other local coordinate in this
    // file, including enemy spawn points, is converted to world space
    // through this same `room_transform` (via `.transform_point`) or, for
    // anything spawned as a child of `room`, inherited automatically
    // through ordinary transform propagation.
    let room_transform = connections::connect_room(fallen_gate::exit_socket(), ENTRANCE_SOCKET);

    // -----------------------------------------------------
    // NAVIGATION GRID -- RAISED PLATFORM
    //
    // Registered in world space (via `room_transform.transform_point`),
    // same as everything else spawned as an independent entity rather than
    // a child of `room`. Numbers below are `room_transform.transform_point`
    // applied by hand to the platform collider's own local placement
    // (Vec3::new(0.0, 2.05, -15.9), half-extents (10.6, 4.3) from
    // Collider::cuboid(21.2, 0.4, 8.6) below) plus the ground/platform
    // enemy-root heights already used for this room's own melee (Y=0.18
    // local) and ranged (Y=2.43 local) spawn points, each with the same
    // +0.75 room lift. `top_half_extents` and the `with_target_region`
    // margins mirror the proportions fallen_gate.rs uses for its own
    // statue pedestal (Section see spawn_fallen_gate) rather than being
    // invented fresh. `with_lane_inset` pulls the two climb lanes in from
    // the platform's outer edge to roughly X=+-6.6 (world), matching where
    // this room's own visual stairs actually sit, so enemies climbing the
    // platform at least approximately track the stairs rather than
    // appearing to climb at the platform's bare corner. First-pass
    // numbers -- confirm melee enemies can actually reach a player on the
    // platform in playtesting and retune if not.
    // -----------------------------------------------------
    navigation_grid.register_obstacle_box(Vec3::new(0.0, 0.93, 91.9), Vec2::new(10.6, 4.3));
    navigation_grid.register_step_platform(
        EnemyStepPlatformConfig::new(
            Vec2::new(0.0, 91.9),
            Vec2::new(10.6, 4.3),
            Vec2::new(10.35, 4.05),
            0.93,
            3.18,
        )
        .with_target_region(Vec2::new(10.35, 4.05), 2.80, 4.33)
        .with_lane_inset(3.75),
    );

    let room = commands
        .spawn((
            Name::new("ROOM - Ruined Barracks"),
            Room,
            RuinedBarracks,
            RoomInfo::new("Ruined Barracks", RoomKind::Combat),
            // Covers the floor (32x42, see "Barracks Floor Collision"
            // below) plus the entrance-threshold overlap past its front
            // wall -- see `RoomBounds`'s doc comment for why this doesn't
            // need to be exact.
            RoomBounds::new(Vec2::new(0.0, 4.0), Vec2::new(16.0, 25.0)),
            room_transform,
            Visibility::default(),
        ))
        .id();

    // Record this room's connection to Fallen Gate for the exploration
    // map (`ui::map`) -- see `rooms::graph::RoomGraph`'s doc comment.
    // Geometric placement above (`room_transform`) already joined the two
    // rooms; this just tells the map they're linked. `.chain()` in
    // `RoomsPlugin` guarantees Fallen Gate already exists by this point.
    if let Ok(fallen_gate_entity) = fallen_gate_room.single() {
        room_graph.connect(fallen_gate_entity, room);
    }

    commands.entity(room).with_children(|parent| {
        // -----------------------------------------------------
        // VISIBLE ROOM SCENE
        // -----------------------------------------------------
        let barracks_scene =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(BARRACKS_ROOM_PATH));

        parent.spawn((
            Name::new("Ruined Barracks Visual Scene"),
            WorldAssetRoot(barracks_scene),
            Transform::default(),
            ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh)
                .with_default_layers(camera_occluder_layers()),
        ));

        // -----------------------------------------------------
        // DETERMINISTIC STATIC COLLISION
        //
        // Matches the confirmed pattern from fallen_gate.rs/saint_arena.rs:
        // one clean convex box per structural piece rather than collision
        // derived from the decorative visual meshes. Every Collision_*
        // node in the GLB is an Empty (no mesh) whose `scale` is a
        // half-extent by Blender's own Empty-display convention; doubled
        // below because Avian's `Collider::cuboid` here takes full side
        // lengths, matching every other room in this project (see
        // `rooms::geometry::spawn_solid_block`, which passes the same
        // `size` to both the visual `Cuboid` mesh and `Collider::cuboid`).
        // Spawned as children of the room root so they inherit its
        // placement above automatically.
        // -----------------------------------------------------

        parent.spawn((
            Name::new("Barracks Floor Collision"),
            Transform::from_xyz(0.0, -0.18, 0.0),
            RigidBody::Static,
            Collider::cuboid(32.0, 0.36, 42.0),
        ));

        // Threshold patch. Fallen Gate's own courtyard floor stops short
        // of the gatehouse's back face (its "Courtyard Rear Fragment"
        // pieces end at world Z=52.5, while the gatehouse structure runs
        // to Z=55 -- confirmed by reading fallen_gate.rs directly), so a
        // roughly 2.5-unit strip under the archway itself has no floor
        // from Fallen Gate's side. Rather than edit that already-built,
        // untested room to add it there, this room brings its own floor
        // forward past its entrance wall to cover the gap.
        //
        // This patch's local Z range (19.0..25.5) deliberately overlaps
        // BOTH neighbors instead of just touching them: its far edge
        // (world Z=57) overlaps this room's own "Barracks Floor Collision"
        // (which starts at world Z=55) by 2 units, and its near edge
        // (world Z=50.5) overlaps Fallen Gate's rear courtyard fragment
        // (which ends at world Z=52.5) by 2 units. An earlier version
        // matched the Barracks floor's edge exactly (touching at world
        // Z=55 with zero overlap) and the player fell through that seam in
        // playtesting -- two separate static colliders meeting at exactly
        // the same coordinate is a classic case for a kinematic character
        // controller to catch and fall through. Widened slightly past the
        // gatehouse's own walkable opening (X -4.5..4.5) for the same
        // reason -- margin, not a precisely-fitted edge.
        parent.spawn((
            Name::new("Barracks Entrance Threshold"),
            Transform::from_xyz(0.0, -0.18, 22.25),
            RigidBody::Static,
            Collider::cuboid(10.0, 0.36, 6.5),
        ));

        // Outer perimeter walls. The front wall (local Z=21, facing
        // Fallen Gate) has an 8-wide gap at X=-4..4 for
        // Entrance_FromFallenGate; the rear wall (local Z=-21) has a
        // 6-wide gap at X=-3..3 filled by the toggleable rear gate below.
        for (name, x, z, size) in [
            (
                "Barracks Wall Front Left",
                -10.0,
                21.0,
                Vec3::new(12.0, 5.5, 0.9),
            ),
            (
                "Barracks Wall Front Right",
                10.0,
                21.0,
                Vec3::new(12.0, 5.5, 0.9),
            ),
            ("Barracks Wall Left", -16.0, 0.0, Vec3::new(0.9, 5.5, 42.0)),
            ("Barracks Wall Right", 16.0, 0.0, Vec3::new(0.9, 5.5, 42.0)),
            (
                "Barracks Wall Rear Left",
                -9.5,
                -21.0,
                Vec3::new(13.0, 5.5, 0.9),
            ),
            (
                "Barracks Wall Rear Right",
                9.5,
                -21.0,
                Vec3::new(13.0, 5.5, 0.9),
            ),
        ] {
            parent.spawn((
                Name::new(name),
                Transform::from_xyz(x, size.y / 2.0, z),
                RigidBody::Static,
                Collider::cuboid(size.x, size.y, size.z),
            ));
        }

        // Raised rear platform -- ranged enemies and the reward sit on
        // top of it (see RangedSpawn_Left/Right and RewardSpawn below).
        parent.spawn((
            Name::new("Barracks Platform Collision"),
            Transform::from_xyz(0.0, 2.05, -15.9),
            RigidBody::Static,
            Collider::cuboid(21.2, 0.4, 8.6),
        ));

        // Stairs up to the platform. Rotation copied directly from the
        // GLB's own Collision_StairsLeft/Right Empties -- Blender already
        // solved the "walkable ramp face" rotation the same way
        // saint_arena.rs derives by hand for its own stairs, so it's
        // reused here instead of re-deriving that geometry a second time.
        let stairs_rotation = Quat::from_xyzw(0.211_328_45, 0.0, 0.0, 0.977_415_1);

        for (name, x) in [
            ("Barracks Stairs Left Collision", -6.6_f32),
            ("Barracks Stairs Right Collision", 6.6_f32),
        ] {
            parent.spawn((
                Name::new(name),
                Transform {
                    translation: Vec3::new(x, 1.125, -9.12),
                    rotation: stairs_rotation,
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(4.15, 0.3, 5.446),
            ));
        }

        // Rear gate. Starts closed (locked) and is despawned by
        // `open_rear_gate_when_cleared` below once every
        // `BarracksCombatant` is defeated -- the GLB's own node metadata
        // tags this collider `collision_role: dynamic_gate`.
        parent.spawn((
            Name::new("Barracks Rear Gate"),
            RearGate,
            Transform::from_xyz(0.0, 3.35, -20.3),
            RigidBody::Static,
            Collider::cuboid(6.0, 6.1, 0.48),
        ));

        // -----------------------------------------------------
        // ROOM SYSTEM MARKERS
        //
        // Only one `RoomConnector::Entrance` and one `VoidKillZone` exist
        // across the whole combined Fallen Gate + Ruined Barracks level
        // right now -- both still Fallen Gate's own. `void::reset_player_
        // from_void` takes the first match of each with no per-room
        // awareness, so adding a second Entrance/VoidKillZone here would
        // make that lookup ambiguous depending on query iteration order --
        // a pre-existing single-room assumption this task doesn't change.
        // Ruined Barracks' own entrance/exit points are still recorded
        // below as plain named markers (no RoomConnector) so they're
        // visible in the scene tree and ready for a future room-aware
        // void-reset pass.
        // -----------------------------------------------------

        parent.spawn((
            Name::new("Barracks Entrance (from Fallen Gate)"),
            Transform::from_xyz(0.0, 0.18, 20.0),
            Visibility::default(),
        ));

        parent.spawn((
            Name::new("Barracks Player Spawn (unused -- see comment above)"),
            Transform::from_xyz(0.0, 0.18, 18.3),
            Visibility::default(),
        ));

        parent.spawn((
            Name::new("Barracks Exit (future room)"),
            RoomConnector {
                kind: ConnectorKind::Exit,
            },
            Transform::from_xyz(0.0, 2.43, -19.0),
            Visibility::default(),
        ));

        // -----------------------------------------------------
        // REWARD
        //
        // One guaranteed chest on the platform, "before the exit" per the
        // requested encounter flow. Always present rather than gated on
        // clearing the room -- the platform sits mid-room, not behind the
        // rear gate, so nothing stops the player from reaching it early.
        // Gating its spawn on the gate opening is an easy later
        // refinement once bell/reinforcement behavior is added.
        // -----------------------------------------------------
        spawn_loot_chest(
            parent,
            &mut meshes,
            &mut materials,
            Vec3::new(0.0, 2.43, -16.0),
            LootTableId::COMMON_CHEST,
            1,
        );
    });

    // -----------------------------------------------------
    // ENEMIES
    //
    // Spawned as independent top-level entities -- spawn_ruinbound_footman
    // / spawn_gatebow_marksman are shared across every room, not scoped to
    // this one -- so their GLB-measured local positions are converted to
    // world space explicitly via `room_transform.transform_point` rather
    // than inheriting the room root's placement the way a child entity
    // would. Because `room_transform` folds in the full connection
    // (rotation, X/Z offset, and the floor-height Y lift), this is the
    // same conversion any future room's enemy spawns can reuse verbatim.
    //
    // All six share one `EnemyEncounterConfig` (`encounter_id: 1`)
    // covering the whole room, so they activate together the moment the
    // player enters -- and, via the existing `activate_damaged_encounters`
    // system, damaging any one of them force-activates the rest too. Stage
    // 1 scope only: the four ground-floor melee spawns
    // (EnemySpawn_Wave1_*) and the two platform ranged spawns
    // (RangedSpawn_Left/Right). EliteSpawn and EnemySpawn_Wave2_* (the
    // bell-triggered reinforcement wave) exist in the GLB but are
    // deliberately unused this pass.
    // -----------------------------------------------------

    let encounter = EnemyEncounterConfig {
        encounter_id: 1,
        activation_center: room_transform.translation,
        activation_half_extents: Vec2::new(16.0, 21.0),
        aggro_extension: 25.0,
    };

    const MELEE_SPAWNS: [Vec3; 4] = [
        Vec3::new(-6.2, 0.18, 6.5),
        Vec3::new(6.2, 0.18, 6.5),
        Vec3::new(-5.0, 0.18, -0.2),
        Vec3::new(5.0, 0.18, -0.2),
    ];

    for local_position in MELEE_SPAWNS {
        let enemy = enemies::spawn_ruinbound_footman(
            &mut commands,
            &mut meshes,
            &mut materials,
            room_transform.transform_point(local_position),
            encounter,
        );
        commands.entity(enemy).insert(BarracksCombatant);
        spawn_enemy_health_bar(&mut commands, enemy);
    }

    const RANGED_SPAWNS: [Vec3; 2] = [Vec3::new(-6.3, 2.43, -16.2), Vec3::new(6.3, 2.43, -16.2)];

    for local_position in RANGED_SPAWNS {
        let enemy = enemies::spawn_gatebow_marksman(
            &mut commands,
            &mut meshes,
            &mut materials,
            room_transform.transform_point(local_position),
            encounter,
        );
        commands.entity(enemy).insert(BarracksCombatant);
        spawn_enemy_health_bar(&mut commands, enemy);
    }

    println!("Ruined Barracks spawned -- 4 melee + 2 ranged, rear gate locked.");
}

// =========================================================
// REAR GATE
// =========================================================

/// Despawns the rear gate's blocking collider once every
/// `BarracksCombatant` has been defeated. Enemies carrying that marker
/// despawn on death like any other (`enemies::handle_enemy_death`), so an
/// empty query here already means the room is genuinely clear -- no
/// separate "enemies remaining" counter needs to be kept in sync with it.
pub fn open_rear_gate_when_cleared(
    mut commands: Commands,
    combatants: Query<(), With<BarracksCombatant>>,
    gate: Query<Entity, With<RearGate>>,
) {
    if !combatants.is_empty() {
        return;
    }

    for gate_entity in &gate {
        commands.entity(gate_entity).despawn();
        println!("Ruined Barracks cleared -- rear gate opens!");
    }
}
