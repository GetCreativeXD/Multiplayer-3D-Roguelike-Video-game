use bevy::prelude::*;

#[derive(Component)]
pub struct Room;

#[derive(Component)]
pub struct FallenGate;

#[derive(Component)]
pub struct SaintArena;

#[derive(Component)]
pub struct TestGrounds;

#[derive(Component)]
pub struct RuinedBarracks;

#[derive(Component)]
pub struct ShatteredJunction;

#[derive(Component)]
pub struct SoldiersPassage;

#[derive(Component)]
pub struct RubbleBridge;

/// Marks the toggleable collider blocking Ruined Barracks' rear
/// gatehouse opening. Despawned by
/// `ruined_barracks::open_rear_gate_when_cleared` once every
/// `BarracksCombatant` has been defeated.
#[derive(Component)]
pub struct RearGate;

/// Marks an enemy as belonging to the Ruined Barracks encounter, so
/// `ruined_barracks::open_rear_gate_when_cleared` can tell when the
/// room is clear without a separate hand-kept counter -- an enemy
/// carrying this despawns on death like any other (see
/// `enemies::handle_enemy_death`), so the query simply empties out.
#[derive(Component)]
pub struct BarracksCombatant;

#[derive(Component)]
pub struct RoomConnector {
    pub kind: ConnectorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorKind {
    Entrance,
    Exit,
}

#[derive(Component)]
pub struct VoidKillZone {
    pub kill_y: f32,
}

#[derive(Component)]
pub struct Breakable;

#[derive(Component)]
pub struct LootSpawn;

#[derive(Component)]
pub struct LoreInteraction;

// =========================================================
// EXPLORATION MAP METADATA
//
// Every room's root entity should carry one `RoomInfo` and one
// `RoomBounds` alongside its `Room` marker (see `fallen_gate.rs`,
// `ruined_barracks.rs`, `saint_arena.rs`, `test_grounds.rs` for the
// pattern). Nothing about the map (`ui::map`) or visit-tracking
// (`rooms::exploration`) needs to change when a new room is added --
// they both just read whatever `Room`/`RoomInfo`/`RoomBounds` entities
// happen to exist.
// =========================================================

/// What a room *is*, for the exploration map and (later) for encounter/
/// reward placement during procedural generation. Purely descriptive --
/// nothing here changes gameplay by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomKind {
    /// The floor's entry point.
    Start,
    /// Ordinary combat/exploration content -- the map shows these as a
    /// plain visited room with no special tag, per the confirmed map
    /// design (Section 6 of the context doc).
    Combat,
    Elite,
    Miniboss,
    Boss,
    Treasure,
    Shrine,
    Event,
    /// Anything else (debug/test zones, or a room nobody's categorized
    /// yet). Renders the same plain way as `Combat`.
    Unknown,
}

/// Static, human-facing metadata for a `Room` entity. Add one alongside
/// `Room` (and the room's own marker component, e.g. `FallenGate`) when
/// spawning a new room.
#[derive(Component, Clone, Copy)]
pub struct RoomInfo {
    pub display_name: &'static str,
    pub kind: RoomKind,
}

impl RoomInfo {
    pub const fn new(display_name: &'static str, kind: RoomKind) -> Self {
        Self { display_name, kind }
    }
}

/// Approximate horizontal (X/Z) footprint of a room's walkable area,
/// measured in the room's own *local* space -- i.e. before its root
/// `Transform` (see `connections::connect_room`) is applied. Only used to
/// tell whether a player is currently standing in this room
/// (`rooms::exploration::track_room_visits`) and to place the room on the
/// exploration map (`ui::map`), so it doesn't need to hug the room's real
/// collision geometry -- a generous box covering the room's playable area
/// is enough. Y is ignored entirely (every room in Ruinbound is placed
/// flat, per `connections.rs`).
#[derive(Component, Clone, Copy)]
pub struct RoomBounds {
    /// Local-space X/Z center of the footprint.
    pub center: Vec2,
    /// Local-space X/Z half-extents around `center`.
    pub half_extents: Vec2,
}

impl RoomBounds {
    pub const fn new(center: Vec2, half_extents: Vec2) -> Self {
        Self {
            center,
            half_extents,
        }
    }

    /// This room's footprint center in world space, given the room root's
    /// own `Transform`. Room roots are always spawned as top-level
    /// entities (never parented), so their `Transform` already *is* their
    /// world transform -- no `GlobalTransform` lookup needed.
    pub fn world_center(&self, room_transform: &Transform) -> Vec3 {
        room_transform.transform_point(Vec3::new(self.center.x, 0.0, self.center.y))
    }

    /// Whether `world_point` (its Y is ignored) falls inside this room's
    /// footprint. Converts the point into the room's local space first,
    /// so a rotated room (e.g. Ruined Barracks, turned 180 degrees by
    /// `connect_room` to face back down the corridor) is still tested
    /// correctly. Assumes unit scale, same as `connections::connect_room`
    /// -- every room root is placed with translation + a Y-axis-only
    /// rotation and nothing else.
    pub fn contains_point(&self, room_transform: &Transform, world_point: Vec3) -> bool {
        let relative = world_point - room_transform.translation;
        let local = room_transform.rotation.inverse() * relative;

        (local.x - self.center.x).abs() <= self.half_extents.x
            && (local.z - self.center.y).abs() <= self.half_extents.y
    }
}

/// Marks a `Room` the party has physically entered at least once this
/// session. Added by `rooms::exploration::track_room_visits`; once
/// present it is never removed -- the exploration map only ever reveals
/// more of the floor, never hides what's already been seen.
#[derive(Component)]
pub struct RoomVisited;
