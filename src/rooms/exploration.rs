//! Tracks which `Room`s the party has physically entered, and which one
//! each player currently stands in. Shared, room-count-independent state
//! that anything can read -- today just the exploration map (`ui::map`),
//! but future systems (e.g. "activate this encounter when the party
//! enters the room") can reuse the same `RoomVisited`/`CurrentRoom` data
//! instead of building their own room-membership check.

use bevy::prelude::*;

use crate::camera::CameraTarget;
use crate::characters::animation::CharacterMotion;

use super::components::{Room, RoomBounds, RoomVisited};

/// The room (if any) a player currently stands inside, per
/// `track_room_visits`. `None` covers both "not spawned yet" and
/// "between/outside every registered `RoomBounds`" -- both are rare in
/// practice since `RoomBounds` is meant to generously cover a room's
/// whole playable footprint.
#[derive(Resource, Default)]
pub struct CurrentRoom(pub Option<Entity>);

/// Marks any room a player is standing in as `RoomVisited`, and records
/// the room *this machine's* player stands in as `CurrentRoom`. Runs
/// continuously (not just while the map is open) so the map has accurate
/// history the instant it's opened.
///
/// Co-op: every player character carries `CharacterMotion` on every
/// machine (the local `Player`, the host's bodies for joined players, and
/// a client's copies of everyone), so each machine's map marks every room
/// any party member has stood in. `CurrentRoom` ("where does the map's
/// `[YOU]` marker go") follows `CameraTarget`, the character this machine's
/// camera follows: the local `Player`, or a joined player's own character.
pub fn track_room_visits(
    mut commands: Commands,
    mut current_room: ResMut<CurrentRoom>,
    players: Query<(&Transform, Has<CameraTarget>), With<CharacterMotion>>,
    rooms: Query<(Entity, &Transform, &RoomBounds, Option<&RoomVisited>), With<Room>>,
) {
    let mut found_current = None;

    for (player_transform, is_this_machines_player) in &players {
        let player_position = player_transform.translation;

        for (room_entity, room_transform, bounds, already_visited) in &rooms {
            if !bounds.contains_point(room_transform, player_position) {
                continue;
            }

            if is_this_machines_player && found_current.is_none() {
                found_current = Some(room_entity);
            }

            if already_visited.is_none() {
                commands.entity(room_entity).insert(RoomVisited);
            }
        }
    }

    current_room.0 = found_current;
}
