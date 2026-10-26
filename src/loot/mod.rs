mod catalog;
mod chests;
mod drops;
// Data definitions for the Arcanist / Striker / Bulwark gear archetypes
// (RUINBOUND_GEAR_ARCHETYPES.md), split into their own subfolder so
// catalog.rs doesn't have to hold all ~57 item definitions by hand.
mod gear;
mod tables;

use bevy::prelude::*;

pub use chests::{spawn_loot_chest, spawn_repeatable_loot_chest};
pub use drops::{LootDropRequest, LootDropRequests};
pub use tables::LootTableId;

// Re-exports so `weapons::armor_procs` (and any future gear-proc code
// outside this module) can identify specific gear pieces by their
// `ItemDefinitionId` without needing `loot::gear` itself to be public --
// `gear` stays a private implementation detail of the loot catalog.
pub use gear::arcanist::{ROBES_OF_THE_HOLLOW_STAR, RUNEBOUND_ROBES};
pub use gear::bulwark::CROWN_OF_THE_UNBROKEN;

pub struct LootPlugin;

impl Plugin for LootPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<tables::LootTableRegistry>()
            .init_resource::<drops::LootRng>()
            .init_resource::<LootDropRequests>()
            .add_systems(
                OnExit(crate::game_state::AppState::InGame),
                crate::game_state::reset_resource::<LootDropRequests>,
            )
            .add_systems(Startup, chests::spawn_chest_prompt)
            .add_systems(
                Update,
                (
                    chests::update_chest_prompt,
                    // Opening chests and rolling loot are simulation: a
                    // joined co-op client sees the host's results instead.
                    (chests::open_nearby_chest, drops::process_loot_drop_requests)
                        .chain()
                        .run_if(crate::net::has_simulation_authority),
                )
                    .chain()
                    // [F] presses arrive in `PlayerIntent`.
                    .after(crate::player::PlayerInputSet),
            );
    }
}
