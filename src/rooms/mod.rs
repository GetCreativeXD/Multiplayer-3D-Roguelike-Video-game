pub mod components;
pub mod connections;
pub mod exploration;
pub mod fallen_gate;
pub mod geometry;
pub mod graph;
pub mod rubble_bridge;
pub mod ruined_barracks;
pub mod saint_arena;
pub mod shattered_junction;
pub mod soldiers_passage;
pub mod test_grounds;
pub mod void;

use bevy::prelude::*;

use crate::game_state::AppState;

pub struct RoomsPlugin;

impl Plugin for RoomsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<graph::RoomGraph>()
            .init_resource::<exploration::CurrentRoom>()
            .add_systems(
                OnExit(AppState::InGame),
                (
                    crate::game_state::reset_resource::<graph::RoomGraph>,
                    crate::game_state::reset_resource::<exploration::CurrentRoom>,
                ),
            );

        // Active run-start zone: Fallen Gate -> Ruined Barracks ->
        // Shattered Junction -> Soldiers Passage -> Rubble Bridge -> Saint
        // of the Broken Siege Arena, each joined onto the previous one by
        // rooms::connections (see each room's own spawn_* function for its
        // own room-root comment). `.chain()` guarantees each room's geometry
        // (and its exit socket) exists before the next room reads it.
        //
        // Spawns on `OnEnter(AppState::InGame)` rather than `Startup` --
        // the room chain (and the player it's built around, spawned
        // separately by `fallen_gate::FallenGatePlugin`) is the actual run,
        // which now only exists once the run-setup screen's "Begin Run"
        // button fires that state transition (see `ui::run_setup_screen`).
        // Test Grounds and arena::ArenaPlugin (the Saint's standalone
        // player/camera/HUD test harness) are unchanged and still compile
        // -- swap the tuple below to bring either back as the run-start
        // zone instead.
        app.add_systems(
            OnEnter(AppState::InGame),
            (
                fallen_gate::spawn_fallen_gate,
                ruined_barracks::spawn_ruined_barracks,
                shattered_junction::spawn_shattered_junction,
                soldiers_passage::spawn_soldiers_passage,
                rubble_bridge::spawn_rubble_bridge,
                saint_arena::spawn_saint_arena,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                fallen_gate::animate_clouds,
                fallen_gate::animate_fire_flicker,
                fallen_gate::animate_flames,
                // Room completion is the host's decision; a co-op client
                // keeps the gate closed until room state is replicated.
                ruined_barracks::open_rear_gate_when_cleared
                    .run_if(crate::net::has_simulation_authority),
                void::reset_player_from_void,
                exploration::track_room_visits,
            ),
        );
    }
}
