//! Co-op multiplayer, built on Lightyear (host-authoritative listen server).
//!
//! See `RUINBOUND_LIGHTYEAR_MULTIPLAYER_PLAN.md` for the full design and
//! `MULTIPLAYER_AUDIT.md` for how it maps onto the current codebase.
//!
//! # Layout
//!
//! | File                | Compiled           | Purpose                                          |
//! |---------------------|--------------------|--------------------------------------------------|
//! | `settings.rs`       | always             | `--host` / `--join` parsing, `NetworkMode`       |
//! | `status.rs`         | always             | connection state + on-screen status label       |
//! | `protocol.rs`       | `multiplayer` only | channels, messages, replicated components        |
//! | `roster.rs`         | `multiplayer` only | host's player list and player-ID assignment      |
//! | `host.rs`           | `multiplayer` only | opens the server, welcomes players, starts runs  |
//! | `player_bodies.rs`  | `multiplayer` only | host: one replicated character per player        |
//! | `intent.rs`         | `multiplayer` only | client sends input; host moves remote bodies     |
//! | `loadout.rs`        | `multiplayer` only | client sends its weapon choice; host equips it   |
//! | `enemies.rs`        | `multiplayer` only | host replicates enemies; clients draw replicas   |
//! | `effects.rs`        | `multiplayer` only | host forwards cosmetic effects (swing arcs)      |
//! | `projectiles.rs`    | `multiplayer` only | host replicates projectiles; clients draw them   |
//! | `client.rs`         | `multiplayer` only | connects to a host, follows the host into a run  |
//! | `avatars.rs`        | `multiplayer` only | every machine: draws/smooths other characters    |
//! | `vitals.rs`         | `multiplayer` only | replicated health + players' floating health bars |
//! | `loot.rs`           | `multiplayer` only | host replicates pickups/chests; per-player loot messages |
//! | `ruin.rs`           | `multiplayer` only | host publishes the shared Ruin; clients mirror it  |
//! | `prediction.rs`     | `multiplayer` only | client predicts its own movement; host acks inputs |
//! | `steam.rs`          | `steam` only       | starts the Steam API for Steam-relay connections |
//! | `steam_lobby.rs`    | `steam` only       | Steam lobby, invites, Join Game, friends panel   |
//!
//! Offline play never adds a single Lightyear plugin, so it behaves exactly
//! as it did before this module existed.
//!
//! # Authority
//!
//! The host (and offline play) *has simulation authority*: it runs player
//! movement, enemies, combat, loot, and room progression. A joined client
//! does not; it loads the same rooms for collision and visuals but leaves
//! every gameplay decision to the host. Gameplay systems that must never run
//! on a client use `.run_if(net::has_simulation_authority)`.
//!
//! # Current milestone (9: prediction and online play, in progress)
//!
//! Milestones 1 to 8 give two to four players a full shared run over UDP
//! (LAN or port forwarding). Milestone 9 step 1 adds Steam's relay network
//! as a second transport (`--host-steam` / `--join-steam`, `steam`
//! feature) and shows the measured ping in the status label. Step 2 adds a
//! friends-only Steam lobby, invites, Steam's Join Game and a Steam Friends
//! panel on the run-setup screen (`--join-steam` without an ID waits for an
//! invite). Step 3 moves player movement onto the fixed tick; step 4
//! (`prediction.rs`) predicts a joined player's own movement.

mod settings;
mod status;

#[cfg(feature = "multiplayer")]
mod avatars;
#[cfg(feature = "multiplayer")]
mod client;
#[cfg(feature = "multiplayer")]
mod effect_visuals;
#[cfg(feature = "multiplayer")]
mod effects;
#[cfg(feature = "multiplayer")]
mod enemies;
#[cfg(feature = "multiplayer")]
mod host;
#[cfg(feature = "multiplayer")]
mod intent;
#[cfg(feature = "multiplayer")]
mod loadout;
#[cfg(feature = "multiplayer")]
mod lobby;
#[cfg(feature = "multiplayer")]
mod loot;
#[cfg(feature = "multiplayer")]
mod player_bodies;
#[cfg(feature = "multiplayer")]
mod prediction;
#[cfg(feature = "multiplayer")]
mod projectiles;
#[cfg(feature = "multiplayer")]
mod protocol;
#[cfg(feature = "multiplayer")]
mod roster;
#[cfg(feature = "multiplayer")]
mod ruin;
#[cfg(feature = "steam")]
mod steam;
#[cfg(feature = "steam")]
mod steam_lobby;
#[cfg(feature = "multiplayer")]
mod telegraphs;
#[cfg(feature = "multiplayer")]
mod vitals;

use bevy::prelude::*;

use settings::{NetworkMode, NetworkSettings, NetworkTransport};
use status::{NetworkStatus, NetworkStatusUiPlugin};

pub struct NetworkPlugin {
    settings: NetworkSettings,
}

impl NetworkPlugin {
    /// Reads `--host` / `--join` from the process arguments. Prints usage
    /// and exits if they can't be understood, rather than silently starting
    /// in the wrong mode.
    pub fn from_command_line() -> Self {
        match NetworkSettings::from_args(std::env::args().skip(1)) {
            Ok(settings)
                if settings.transport == NetworkTransport::Steam && !cfg!(feature = "steam") =>
            {
                eprintln!(
                    "Ruinbound: the Steam options need a Steam build. Run with\n  \
                     cargo run --features steam -- --host-steam\nor\n  \
                     cargo run --features steam -- --join-steam <host Steam ID>"
                );
                std::process::exit(2);
            }
            Ok(settings) => Self { settings },
            Err(error) => {
                eprintln!("Ruinbound: {error}\n\n{}", settings::USAGE);
                std::process::exit(2);
            }
        }
    }
}

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        let settings = settings_supported_by_this_build(self.settings.clone());

        app.insert_resource(NetworkStatus::starting(&settings))
            .insert_resource(settings.clone())
            .add_plugins(NetworkStatusUiPlugin);

        #[cfg(feature = "multiplayer")]
        add_lightyear(app, &settings);
    }
}

/// Run condition: does this process own the game simulation? True offline
/// and on the host; false on a joined client, which waits for the host to
/// start the run and never makes gameplay decisions itself.
pub fn has_simulation_authority(settings: Res<NetworkSettings>) -> bool {
    settings.mode != NetworkMode::Client
}

/// Falls back to offline (with a warning) if multiplayer was requested
/// from a build that doesn't include it.
fn settings_supported_by_this_build(settings: NetworkSettings) -> NetworkSettings {
    if cfg!(feature = "multiplayer") || settings.mode == NetworkMode::Offline {
        return settings;
    }
    warn!(
        "[net] {:?} mode requested, but this build has no multiplayer support. \
         Rebuild with `cargo run --features multiplayer -- ...`. Starting offline.",
        settings.mode
    );
    NetworkSettings {
        mode: NetworkMode::Offline,
        ..settings
    }
}

/// Lightyear's simulation tick rate. 64 Hz deliberately matches Bevy's
/// default fixed timestep, which Lightyear takes over: Avian physics runs
/// in the fixed schedule, so any other value would change physics timing
/// in networked sessions.
#[cfg(feature = "multiplayer")]
const NETWORK_TICK_HZ: f64 = 64.0;

/// How often the status label's ping is refreshed.
#[cfg(feature = "multiplayer")]
const ROUND_TRIP_REFRESH: std::time::Duration = std::time::Duration::from_millis(500);

/// Lightyear's round-trip estimate in whole milliseconds, or `None` before
/// the first ping/pong has been measured (the estimate starts at zero).
#[cfg(feature = "multiplayer")]
fn measured_round_trip_ms(rtt: std::time::Duration) -> Option<u32> {
    (!rtt.is_zero()).then(|| rtt.as_millis().min(u32::MAX as u128) as u32)
}

#[cfg(feature = "multiplayer")]
fn add_lightyear(app: &mut App, settings: &NetworkSettings) {
    use lightyear::prelude::client::ClientPlugins;
    use lightyear::prelude::server::ServerPlugins;
    use std::time::Duration;

    if settings.mode == NetworkMode::Offline {
        return;
    }

    // Steam must be running before Lightyear's Steam IO exists. A failure
    // here is a setup problem on this machine (Steam closed or logged out),
    // so explain it and stop, like a bad command-line option.
    #[cfg(feature = "steam")]
    if settings.transport == NetworkTransport::Steam {
        match steam::start_steam(app) {
            Ok(identity) => {
                info!(
                    "[net] Steam ready: signed in as {} (Steam ID {})",
                    identity.persona_name, identity.steam_id
                );
                if settings.mode == NetworkMode::Host {
                    // Printed unconditionally (once) so it can be copied
                    // from the terminal and sent to a friend.
                    info!(
                        "[net] Friends join with: cargo run --features steam -- --join-steam {}",
                        identity.steam_id
                    );
                }
                app.world_mut().resource_mut::<NetworkStatus>().steam_id = Some(identity.steam_id);
            }
            Err(error) => {
                eprintln!(
                    "Ruinbound: couldn't start Steam ({error}).\n\
                     Open the Steam app, sign in, and try again."
                );
                std::process::exit(1);
            }
        }
    }

    let tick_duration = Duration::from_secs_f64(1.0 / NETWORK_TICK_HZ);

    match settings.mode {
        NetworkMode::Offline => return,
        // A listen server is a server and a client in one app.
        NetworkMode::Host => {
            app.add_plugins((
                ServerPlugins { tick_duration },
                ClientPlugins { tick_duration },
            ));
            app.add_plugins((
                host::HostSessionPlugin,
                player_bodies::PlayerBodiesPlugin,
                intent::HostIntentPlugin,
                enemies::HostEnemyReplicationPlugin,
                effects::HostEffectsPlugin,
                loadout::HostLoadoutPlugin,
                projectiles::HostProjectileReplicationPlugin,
                telegraphs::HostTelegraphReplicationPlugin,
                effect_visuals::HostEffectVisualsPlugin,
                lobby::HostLobbyPlugin,
                loot::HostLootPlugin,
                ruin::HostRuinPlugin,
                prediction::HostPredictionPlugin,
            ));
        }
        NetworkMode::Client => {
            app.add_plugins(ClientPlugins { tick_duration });
            app.add_plugins((
                client::JoinSessionPlugin,
                intent::ClientIntentPlugin,
                enemies::ClientEnemyReplicaPlugin,
                effects::ClientEffectsPlugin,
                loadout::ClientLoadoutPlugin,
                projectiles::ClientProjectileReplicaPlugin,
                telegraphs::ClientTelegraphReplicaPlugin,
                effect_visuals::ClientEffectVisualsPlugin,
                lobby::ClientLobbyPlugin,
                loot::ClientLootPlugin,
                ruin::ClientRuinPlugin,
                prediction::ClientPredictionPlugin,
            ));
        }
    }

    // Lightyear requires the protocol to be registered after its plugins.
    app.add_plugins((
        protocol::ProtocolPlugin,
        avatars::AvatarsPlugin,
        vitals::VitalsPlugin,
    ))
    .add_systems(Update, client::receive_welcome);

    #[cfg(feature = "steam")]
    if settings.transport == NetworkTransport::Steam {
        app.add_plugins(steam_lobby::SteamLobbyPlugin);
    }
}
