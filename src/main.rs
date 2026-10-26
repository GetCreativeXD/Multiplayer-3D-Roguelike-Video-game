mod arena;
mod camera;
mod characters;
mod collision;
mod combat;
mod enemies;
mod game_state;
mod items;
mod loot;
mod navigation_mesh;
mod net;
mod player;
mod rooms;
mod ruin;
mod run_setup;
mod ui;
mod visual_effects;
mod weapons;

use avian3d::prelude::*;
use bevy::prelude::*;
use characters::CharactersPlugin;
use game_state::AppState;
use loot::LootPlugin;
use rooms::RoomsPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ruinbound".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        // Tears a run's world down when it ends (see game_state).
        .add_plugins(game_state::RunLifecyclePlugin)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(navigation_mesh::WorldNavigationPlugin)
        .add_plugins(camera::CameraPlugin)
        // The player's confirmed run-setup choices (class, starting
        // weapon, item drop pool, starting relic, run pact, starting
        // Ruin condition) -- see run_setup::RunConfig. Registered before
        // UiPlugin (which owns the screen that edits it) and RoomsPlugin/
        // FallenGatePlugin (which read it once AppState::InGame starts).
        .add_plugins(run_setup::RunSetupPlugin)
        .add_plugins(ui::UiPlugin)
        .add_plugins(weapons::WeaponPlugin)
        // Ruin: the run-based corruption resource (see
        // RUIN_CORRUPTION_DESIGN.md and src/ruin/mod.rs). Its clock runs
        // unconditionally (opening the map, inventory, or Tab screen
        // does not pause it -- see src/ruin/mod.rs), so it has no
        // ordering dependency on UiPlugin/WeaponPlugin; kept here for
        // readability, alongside the other UI-adjacent plugins.
        .add_plugins(ruin::RuinPlugin)
        .add_plugins(LootPlugin)
        .add_plugins(player::PlayerPlugin)
        // Barriers + death-prevention: shared, gear-agnostic combat state
        // (see src/combat/barrier.rs, src/combat/death_prevention.rs).
        .add_plugins(combat::CombatPlugin)
        .add_plugins(enemies::EnemyPlugin)
        // Fallen Gate is the active run-start zone (player + HUD spawn;
        // the camera itself spawns separately and unconditionally, see
        // camera::CameraPlugin), connected straight through into Ruined
        // Barracks -> Shattered Junction -> Soldiers Passage -> Rubble
        // Bridge -> Saint of the Broken Siege Arena by RoomsPlugin's own
        // OnEnter(AppState::InGame) chain (see rooms::RoomsPlugin). Both
        // this plugin and RoomsPlugin only actually spawn anything once
        // the run-setup screen's "Begin Run" button fires that state
        // transition (see ui::run_setup_screen). Test Grounds and
        // arena::ArenaPlugin (the Saint's standalone player/camera/HUD
        // test harness) are unchanged and still compile -- swap this line
        // to bring either back.
        .add_plugins(rooms::fallen_gate::FallenGatePlugin)
        .add_plugins(RoomsPlugin)
        .add_plugins(CharactersPlugin)
        // Co-op networking. Reads `--host` / `--join` from the command
        // line; with neither (or without the `multiplayer` feature) it only
        // records Offline mode and adds nothing else -- see src/net/mod.rs.
        .add_plugins(net::NetworkPlugin::from_command_line())
        .run();
}
