//! Client mode: connects to a host (over UDP, or to a SteamID64 over
//! Steam's relay network with `--join-steam`), loads the run's world when
//! the host starts it, and leaves all simulation to the host. Also home to
//! `receive_welcome`, which both roles use because the host's own player is
//! a Lightyear client too.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy::time::common_conditions::on_real_timer;
use lightyear::prelude::client::{NetcodeClient, NetcodeConfig};
use lightyear::prelude::*;

use crate::enemies::Enemy;
use crate::game_state::AppState;
use crate::run_setup::{RunConfig, RunModifiers};

use super::protocol::{DEV_PRIVATE_KEY, PROTOCOL_ID, RunStartedMessage, WelcomeMessage};
use super::settings::{NetworkSettings, NetworkTransport};
use super::status::{MultiplayerConnectionState, NetworkStatus};

/// How long the host may stay silent before the client gives up, both
/// while connecting and once connected.
const CONNECTION_TIMEOUT_SECS: i32 = 5;

/// If the connection hasn't even started handshaking by now, report a
/// failure rather than waiting forever.
const CONNECT_ATTEMPT_LIMIT: Duration = Duration::from_secs(10);

/// Steam may need several seconds to find a relay route to the host, and a
/// failed attempt reports itself, so it gets a longer limit.
const STEAM_CONNECT_ATTEMPT_LIMIT: Duration = Duration::from_secs(30);

pub struct JoinSessionPlugin;

impl Plugin for JoinSessionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, connect_to_host)
            .add_systems(
                Update,
                (track_client_connection, enter_run_when_host_starts),
            )
            .add_systems(
                Update,
                track_round_trip.run_if(on_real_timer(super::ROUND_TRIP_REFRESH)),
            )
            // `Last`, so every room's spawn commands have been applied.
            .add_systems(Last, remove_locally_spawned_enemies);
    }
}

/// Marks the connection entity this plugin spawned.
#[derive(Component)]
struct RuinboundClientConnection;

/// When (real time since startup) this connection was opened. A Steam
/// client may wait minutes for an invite before connecting, so the attempt
/// limit is measured from here rather than from startup.
#[derive(Component)]
struct ConnectAttemptStarted(Duration);

fn connect_to_host(
    mut commands: Commands,
    settings: Res<NetworkSettings>,
    time: Res<Time<Real>>,
) -> Result {
    if settings.transport == NetworkTransport::Steam {
        // Without a host ID, `net::steam_lobby` connects once an invite or
        // Join arrives.
        if let Some(host) = settings.steam_host_id {
            connect_over_steam(&mut commands, host, &settings, &time);
        }
        return Ok(());
    }

    let authentication = Authentication::Manual {
        server_addr: settings.server_addr,
        client_id: random_netcode_client_id(),
        private_key: DEV_PRIVATE_KEY,
        protocol_id: PROTOCOL_ID,
    };
    let netcode = NetcodeClient::new(
        authentication,
        NetcodeConfig {
            client_timeout_secs: CONNECTION_TIMEOUT_SECS,
            // Self-made tokens never need to expire.
            token_expire_secs: -1,
            ..default()
        },
    )?;

    // Port 0 lets the OS pick a free local port, so several clients can
    // run on one machine.
    let local_address = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0);

    let connection = commands
        .spawn((
            Name::new("Ruinbound Client Connection"),
            RuinboundClientConnection,
            Client,
            // Accept the entities the host replicates (player characters).
            ReplicationReceiver,
            netcode,
            ConnectAttemptStarted(time.elapsed()),
            LocalAddr(local_address),
            PeerAddr(settings.server_addr),
            UdpIo::default(),
        ))
        .id();
    commands.trigger(Connect { entity: connection });

    if settings.debug {
        info!("[net] connecting to {}", settings.server_addr);
    }
    Ok(())
}

/// Connects to the host's SteamID64 through Steam's relay network. Steam
/// authenticates both ends itself, so there is no Netcode layer, no
/// addresses and no port forwarding. Used at startup with `--join-steam ID`
/// and by `net::steam_lobby` once a lobby invite resolves to its host.
#[cfg(feature = "steam")]
pub(super) fn connect_over_steam(
    commands: &mut Commands,
    host_steam_id: u64,
    settings: &NetworkSettings,
    time: &Time<Real>,
) {
    use lightyear::steam::prelude::client::{ConnectTarget, SteamClientIo};
    use lightyear::steam::prelude::{SessionConfig, SteamId};

    let connection = commands
        .spawn((
            Name::new("Ruinbound Client Connection (Steam)"),
            RuinboundClientConnection,
            Client,
            ReplicationReceiver,
            ConnectAttemptStarted(time.elapsed()),
            SteamClientIo {
                target: ConnectTarget::Peer {
                    steam_id: SteamId::from_raw(host_steam_id),
                    virtual_port: super::steam::STEAM_VIRTUAL_PORT,
                },
                config: SessionConfig::default(),
            },
        ))
        .id();
    commands.trigger(Connect { entity: connection });

    if settings.debug {
        info!("[net] connecting to Steam user {host_steam_id}");
    }
}

/// `NetworkPlugin::from_command_line` exits before this can be reached.
#[cfg(not(feature = "steam"))]
fn connect_over_steam(
    _commands: &mut Commands,
    _host_steam_id: u64,
    _settings: &NetworkSettings,
    _time: &Time<Real>,
) {
    unreachable!("Steam joining requested in a build without the `steam` feature");
}

/// Netcode needs a per-connection client number. It is *not* the player's
/// identity (the host assigns that); it only has to differ between the
/// clients currently connected, so a randomly seeded hash is plenty.
fn random_netcode_client_id() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u32(std::process::id());
    if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
        hasher.write_u128(now.as_nanos());
    }
    hasher.finish()
}

/// Reflects the client connection's Lightyear state in `NetworkStatus`.
fn track_client_connection(
    connection: Query<
        (
            Has<Connecting>,
            Has<Linking>,
            Has<Connected>,
            Option<&Disconnected>,
            &ConnectAttemptStarted,
        ),
        With<RuinboundClientConnection>,
    >,
    time: Res<Time<Real>>,
    settings: Res<NetworkSettings>,
    mut status: ResMut<NetworkStatus>,
    // Whether the connection has ever reached `Connecting` or `Connected`;
    // before that, a `Disconnected` marker just means "not started yet".
    mut handshake_started: Local<bool>,
) {
    let Ok((connecting, linking, connected, disconnected, attempt_started)) = connection.single()
    else {
        return;
    };
    // Steam has no Netcode handshake: its link goes straight from Linking
    // to Connected, so Linking counts as connecting.
    let connecting = connecting || linking;
    let attempt_limit = match settings.transport {
        NetworkTransport::Udp => CONNECT_ATTEMPT_LIMIT,
        NetworkTransport::Steam => STEAM_CONNECT_ATTEMPT_LIMIT,
    };

    let next_state = if connected {
        *handshake_started = true;
        MultiplayerConnectionState::Connected
    } else if connecting {
        *handshake_started = true;
        MultiplayerConnectionState::Connecting
    } else if let (true, Some(disconnected)) = (*handshake_started, disconnected) {
        let reason = disconnected.reason.to_string();
        match status.state {
            // We were in: the host went away or the network dropped.
            MultiplayerConnectionState::Connected => {
                MultiplayerConnectionState::Disconnected { reason }
            }
            // Already reported; keep the first reason on screen.
            MultiplayerConnectionState::Disconnected { .. }
            | MultiplayerConnectionState::ConnectionFailed { .. } => return,
            // We never got in: wrong address, host not running, or a
            // different game version (protocol mismatch).
            _ => MultiplayerConnectionState::ConnectionFailed {
                reason: match settings.transport {
                    NetworkTransport::Udp => format!(
                        "{reason} - check the address and that the host is running the same build"
                    ),
                    NetworkTransport::Steam => format!(
                        "{reason} - check the host's Steam ID and that the host is running the same build"
                    ),
                },
            },
        }
    } else if !*handshake_started && time.elapsed() > attempt_started.0 + attempt_limit {
        MultiplayerConnectionState::ConnectionFailed {
            reason: "timed out before the connection started".into(),
        }
    } else {
        return;
    };

    if status.state != next_state {
        if settings.debug {
            info!("[net] client connection: {next_state:?}");
        }
        status.state = next_state;
    }
}

/// Stores the player ID the host assigned. Runs in Host and Client mode.
/// Leaves the session if the host runs a different build (Steam links skip
/// Netcode's own protocol check).
pub fn receive_welcome(
    mut commands: Commands,
    mut receivers: Query<(Entity, &mut MessageReceiver<WelcomeMessage>), With<Client>>,
    settings: Res<NetworkSettings>,
    mut status: ResMut<NetworkStatus>,
) {
    for (connection, mut receiver) in &mut receivers {
        for welcome in receiver.receive() {
            if welcome.protocol != PROTOCOL_ID {
                warn!(
                    "[net] host runs protocol {}, this build runs {PROTOCOL_ID}; disconnecting",
                    welcome.protocol
                );
                status.state = MultiplayerConnectionState::ConnectionFailed {
                    reason: format!(
                        "the host is running a different build (protocol {} vs ours {PROTOCOL_ID}); \
                         both players need the same version",
                        welcome.protocol
                    ),
                };
                commands.trigger(Disconnect { entity: connection });
                continue;
            }
            if settings.debug {
                info!("[net] host assigned us player {}", welcome.player_id.0);
            }
            status.local_player_id = Some(welcome.player_id.0);
        }
    }
}

/// Shows the round trip to the host in the status label.
fn track_round_trip(
    connection: Query<&PingManager, (With<RuinboundClientConnection>, With<Connected>)>,
    mut status: ResMut<NetworkStatus>,
) {
    let round_trip = connection
        .single()
        .ok()
        .and_then(|ping| super::measured_round_trip_ms(ping.rtt()));
    if status.round_trip_ms != round_trip {
        status.round_trip_ms = round_trip;
    }
}

/// The host started the run: load the same world. Rooms build as normal,
/// but the client's own `Player` is skipped (see
/// `rooms::fallen_gate::FallenGatePlugin`); its character comes from the
/// host instead.
fn enter_run_when_host_starts(
    mut receivers: Query<&mut MessageReceiver<RunStartedMessage>, With<Client>>,
    app_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    run_config: Res<RunConfig>,
    mut run_modifiers: ResMut<RunModifiers>,
) {
    for mut receiver in &mut receivers {
        // Drain every message, but only act on the first.
        if receiver.receive().count() > 0 && *app_state.get() == AppState::RunSetup {
            // Same snapshot the host takes when it presses Begin Run
            // (the relics and pacts are shared, see `net::lobby`).
            *run_modifiers = RunModifiers::from_selections(
                run_config.relics.as_slice(),
                run_config.pacts.as_slice(),
            );
            next_state.set(AppState::InGame);
        }
    }
}

/// Enemies belong to the host (plan: "Run enemy AI only on the host"). The
/// rooms a client loads still spawn their own enemies, so remove them here.
/// The client sees the host's enemies instead, as replicas without the
/// `Enemy` marker (`net::enemies`), which this query therefore never
/// touches.
fn remove_locally_spawned_enemies(
    mut commands: Commands,
    local_enemies: Query<Entity, With<Enemy>>,
) {
    for enemy in &local_enemies {
        commands.entity(enemy).despawn();
    }
}
