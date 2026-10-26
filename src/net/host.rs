//! Host mode: opens a server for remote players (UDP, or Steam's relay
//! network with `--host-steam`) and connects the
//! host's own local player to it ("listen server"). The host assigns every
//! player's `NetworkPlayerId`.
//!
//! Lightyear models each connection as an entity:
//! - the *server* entity (`Server`, `NetcodeServer`, `ServerUdpIo`) listens;
//! - every remote player gets a *link* entity (`LinkOf` + `ClientOf`) that
//!   Lightyear spawns when they connect;
//! - the host's own player is a `Client` entity that is also a `LinkOf` the
//!   server, which Lightyear turns into a `HostClient` (no sockets involved).
//!
//! Remote links get a `ReplicationSender`, which lets entities with a
//! `Replicate` component (see `player_bodies.rs`) be sent to them.

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use bevy::prelude::*;
use bevy::time::common_conditions::on_real_timer;
use lightyear::connection::client_of::ClientOf;
use lightyear::connection::host::HostClient;
use lightyear::prelude::server::{
    NetcodeConfig, NetcodeServer, ServerUdpIo, Start, Started, Stopped,
};
use lightyear::prelude::*;

use crate::game_state::AppState;

use super::protocol::{
    DEV_PRIVATE_KEY, PROTOCOL_ID, RunStartedMessage, SessionChannel, WelcomeMessage,
};
use super::roster::{MAX_PLAYERS, PlayerRoster};
use super::settings::{NetworkSettings, NetworkTransport};
use super::status::{MultiplayerConnectionState, NetworkStatus};

/// How often the host sends replicated state (player poses, and later
/// enemies) to clients. Clients smooth between updates, so this doesn't
/// need to match the frame rate.
const REPLICATION_INTERVAL: Duration = Duration::from_millis(33);

pub struct HostSessionPlugin;

impl Plugin for HostSessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerRoster>()
            .insert_resource(ReplicationMetadata::new(REPLICATION_INTERVAL))
            .add_systems(Startup, start_hosting)
            .add_systems(Update, track_server_state)
            .add_systems(
                Update,
                track_slowest_round_trip.run_if(on_real_timer(super::ROUND_TRIP_REFRESH)),
            )
            .add_systems(OnEnter(AppState::InGame), tell_clients_the_run_started)
            .add_observer(welcome_connected_player)
            .add_observer(forget_disconnected_player);
    }
}

/// Marks the server entity this plugin spawned.
#[derive(Component)]
struct RuinboundServer;

/// Spawns the listening server, then connects the host's local player to
/// it. Lightyear holds the local connection as `Connecting` until the
/// server reports `Started`, so the order of the two triggers is safe.
/// (Steam opens its listen socket asynchronously, so this matters there.)
fn start_hosting(mut commands: Commands, settings: Res<NetworkSettings>) {
    let mut server = commands.spawn((Name::new("Ruinbound Server"), RuinboundServer));
    match settings.transport {
        NetworkTransport::Udp => {
            // Listen on every network interface so LAN players can join.
            let bind_address = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), settings.host_port);
            server.insert((
                Server::default(),
                NetcodeServer::new(NetcodeConfig {
                    protocol_id: PROTOCOL_ID,
                    private_key: DEV_PRIVATE_KEY,
                    // The host's own player doesn't use a Netcode slot.
                    max_clients: MAX_PLAYERS - 1,
                    // Remote players address the host by its LAN/public IP
                    // while the socket is bound to 0.0.0.0, so the connect
                    // token's address won't match the bind address. Skip
                    // that check.
                    server_addr_check: false,
                    ..default()
                }),
                LocalAddr(bind_address),
                ServerUdpIo::default(),
            ));
        }
        NetworkTransport::Steam => insert_steam_server_io(&mut server),
    }
    let server = server.id();
    commands.trigger(Start { entity: server });

    let local_player = commands
        .spawn((
            Name::new("Ruinbound Host Player"),
            Client,
            LinkOf { server },
        ))
        .id();
    commands.trigger(Connect {
        entity: local_player,
    });
}

/// Listens for Steam peers on `STEAM_VIRTUAL_PORT` through Steam's relay
/// network. Steam connections skip Netcode (Steam authenticates them), so
/// the player cap is enforced in `welcome_connected_player` instead.
#[cfg(feature = "steam")]
fn insert_steam_server_io(server: &mut EntityCommands) {
    use lightyear::steam::prelude::SessionConfig;
    use lightyear::steam::prelude::server::{ListenTarget, SteamServerIo};

    server.insert(SteamServerIo {
        target: ListenTarget::Peer {
            virtual_port: super::steam::STEAM_VIRTUAL_PORT,
        },
        config: SessionConfig::default(),
    });
}

/// `NetworkPlugin::from_command_line` exits before this can be reached.
#[cfg(not(feature = "steam"))]
fn insert_steam_server_io(_server: &mut EntityCommands) {
    unreachable!("Steam hosting requested in a build without the `steam` feature");
}

/// Reflects the server's Started/Stopped state in `NetworkStatus`.
fn track_server_state(
    server: Query<(Has<Started>, Has<Stopped>), With<RuinboundServer>>,
    roster: Res<PlayerRoster>,
    settings: Res<NetworkSettings>,
    mut status: ResMut<NetworkStatus>,
) {
    let Ok((started, stopped)) = server.single() else {
        return;
    };

    let next_state = if started {
        MultiplayerConnectionState::Hosting
    } else if stopped && status.state == MultiplayerConnectionState::StartingHost {
        MultiplayerConnectionState::ConnectionFailed {
            reason: match settings.transport {
                NetworkTransport::Udp => {
                    "could not open the host port (is another copy already hosting?)".into()
                }
                NetworkTransport::Steam => {
                    "could not open a Steam listen socket (is Steam signed in?)".into()
                }
            },
        }
    } else {
        return;
    };

    // Only write when something changed, so the status label isn't
    // rebuilt every frame.
    if status.state != next_state {
        if next_state != MultiplayerConnectionState::Hosting {
            warn!("[net] host failed to start");
        }
        status.state = next_state;
    }
    if status.connected_players != roster.len() {
        status.connected_players = roster.len();
    }
}

/// When any player (including the host's own) finishes connecting, give
/// them a player ID and tell them what it is. Remote players are also
/// enrolled in replication, and told to load the run if it already started.
fn welcome_connected_player(
    trigger: On<Add, Connected>,
    mut commands: Commands,
    mut connections: Query<
        (
            Has<HostClient>,
            &mut MessageSender<WelcomeMessage>,
            &mut MessageSender<RunStartedMessage>,
        ),
        With<ClientOf>,
    >,
    mut roster: ResMut<PlayerRoster>,
    app_state: Res<State<AppState>>,
    settings: Res<NetworkSettings>,
) {
    let Ok((is_host, mut welcome_sender, mut run_started_sender)) =
        connections.get_mut(trigger.entity)
    else {
        return;
    };

    let player_id = roster.add_player(trigger.entity, is_host);
    welcome_sender.send::<SessionChannel>(WelcomeMessage {
        player_id,
        protocol: PROTOCOL_ID,
    });

    // The host's own player shares this world, so it needs neither.
    if !is_host {
        commands.entity(trigger.entity).insert(ReplicationSender);
        if *app_state.get() == AppState::InGame {
            run_started_sender.send::<SessionChannel>(RunStartedMessage);
        }
    }

    if settings.debug {
        info!(
            "[net] player {} joined (host: {is_host}, connection {:?}); {} in session",
            player_id.0,
            trigger.entity,
            roster.len()
        );
    }
}

/// Shows the slowest joined player's round trip in the status label.
fn track_slowest_round_trip(
    remote_players: Query<&PingManager, (With<ClientOf>, With<Connected>, Without<HostClient>)>,
    mut status: ResMut<NetworkStatus>,
) {
    let slowest = remote_players
        .iter()
        .filter_map(|ping| super::measured_round_trip_ms(ping.rtt()))
        .max();
    if status.round_trip_ms != slowest {
        status.round_trip_ms = slowest;
    }
}

/// Removes a player from the roster when their connection drops or is
/// despawned.
fn forget_disconnected_player(
    trigger: On<Remove, Connected>,
    mut roster: ResMut<PlayerRoster>,
    settings: Res<NetworkSettings>,
) {
    let Some(player) = roster.remove_connection(trigger.entity) else {
        return;
    };
    if settings.debug {
        let role = if player.is_host { "host" } else { "remote" };
        info!(
            "[net] {role} player {} left; {} in session",
            player.player_id.0,
            roster.len()
        );
    }
}

/// The host pressed Begin Run: every connected remote player loads the run.
fn tell_clients_the_run_started(
    mut remote_players: Query<
        &mut MessageSender<RunStartedMessage>,
        (With<ClientOf>, With<Connected>, Without<HostClient>),
    >,
) {
    for mut sender in &mut remote_players {
        sender.send::<SessionChannel>(RunStartedMessage);
    }
}
