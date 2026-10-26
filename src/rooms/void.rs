use bevy::prelude::*;

use crate::player::{PlayerIntent, VerticalMovement};

use super::components::{ConnectorKind, RoomConnector, VoidKillZone};

// =========================================================
// RESET PLAYER AFTER FALLING INTO VOID
// =========================================================

pub fn reset_player_from_void(
    entrances: Query<(&RoomConnector, &GlobalTransform)>,

    void_zones: Query<&VoidKillZone>,

    // Every player character: the local `Player` and, on the host, remote
    // players' bodies.
    mut players: Query<(&mut Transform, &mut VerticalMovement), With<PlayerIntent>>,
) {
    // -----------------------------------------------------
    // FIND THE CURRENT ROOM ENTRANCE
    // -----------------------------------------------------

    let spawn_position = entrances.iter().find_map(|(connector, transform)| {
        if connector.kind == ConnectorKind::Entrance {
            Some(transform.translation())
        } else {
            None
        }
    });

    let Some(spawn_position) = spawn_position else {
        return;
    };

    let Some(void_zone) = void_zones.iter().next() else {
        return;
    };

    let kill_y = void_zone.kill_y;

    // -----------------------------------------------------
    // RESET PLAYERS WHO FELL INTO THE VOID
    // -----------------------------------------------------

    for (mut transform, mut vertical_movement) in &mut players {
        if transform.translation.y < kill_y {
            println!("Player fell into the void - resetting to entrance");

            // Return to the room entrance.
            transform.translation = spawn_position;

            // Reset falling / jumping state so the player
            // doesn't keep their old downward movement.
            *vertical_movement = VerticalMovement::default();
        }
    }
}
