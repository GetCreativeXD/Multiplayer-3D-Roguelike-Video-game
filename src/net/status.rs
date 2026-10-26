//! The session's connection state and the small on-screen label that shows
//! it. This file is compiled in every build; only the code that *changes*
//! the state lives behind the `multiplayer` feature (see `host.rs` and
//! `client.rs`).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::settings::{NetworkMode, NetworkSettings, NetworkTransport};

/// Where this process is in the connection lifecycle.
///
/// Later milestones add the in-run states (loading the host's level,
/// playing); Milestone 1 only covers getting connected.
#[derive(Debug, Clone, PartialEq, Eq)]
// Without the `multiplayer` feature only the starting states are built.
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
pub enum MultiplayerConnectionState {
    Offline,
    StartingHost,
    /// The host's server is listening and its local player is connected.
    Hosting,
    /// Steam client started without a host ID: waiting for a friend's
    /// invite, a Join Game from Steam, or a Join click in the friends panel.
    WaitingForSteamInvite,
    Connecting,
    Connected,
    /// Was connected, then lost the host.
    Disconnected {
        reason: String,
    },
    /// Never managed to connect (or the host could not open its port).
    ConnectionFailed {
        reason: String,
    },
}

/// Everything the status label needs to know about the session.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct NetworkStatus {
    pub state: MultiplayerConnectionState,
    /// The ID the host assigned to this player. `None` until the host's
    /// welcome message arrives. Clients never choose their own ID.
    pub local_player_id: Option<u64>,
    /// Number of players in the session, including the host. Only
    /// tracked on the host.
    pub connected_players: usize,
    /// This machine's SteamID64, once Steam has started (Steam transport
    /// only). The host shows it so friends know what to join.
    pub steam_id: Option<u64>,
    /// The SteamID64 of the host being joined (Steam clients): from
    /// `--join-steam ID`, or the owner of the lobby joined through an invite.
    pub steam_host: Option<u64>,
    /// The Steam lobby this machine created (host) or entered (client).
    pub steam_lobby: Option<u64>,
    /// Measured round-trip time in whole milliseconds: to the host on a
    /// client, the slowest joined player on the host. `None` until measured
    /// (and on a host with nobody joined).
    pub round_trip_ms: Option<u32>,
}

impl NetworkStatus {
    pub fn starting(settings: &NetworkSettings) -> Self {
        let state = match settings.mode {
            NetworkMode::Offline => MultiplayerConnectionState::Offline,
            NetworkMode::Host => MultiplayerConnectionState::StartingHost,
            NetworkMode::Client
                if settings.transport == NetworkTransport::Steam
                    && settings.steam_host_id.is_none() =>
            {
                MultiplayerConnectionState::WaitingForSteamInvite
            }
            NetworkMode::Client => MultiplayerConnectionState::Connecting,
        };
        Self {
            state,
            local_player_id: None,
            connected_players: 0,
            steam_id: None,
            steam_host: settings.steam_host_id,
            steam_lobby: None,
            round_trip_ms: None,
        }
    }
}

pub struct NetworkStatusUiPlugin;

impl Plugin for NetworkStatusUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (label_window_with_role, spawn_status_label).run_if(is_networked),
        )
        .add_systems(
            Update,
            update_status_label
                .run_if(is_networked)
                .run_if(resource_changed::<NetworkStatus>),
        );
    }
}

fn is_networked(settings: Res<NetworkSettings>) -> bool {
    settings.mode != NetworkMode::Offline
}

#[derive(Component)]
struct NetworkStatusLabel;

/// Makes the host and client windows easy to tell apart when testing two
/// copies of the game on one machine.
fn label_window_with_role(
    settings: Res<NetworkSettings>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    let role = match settings.mode {
        NetworkMode::Offline => return,
        NetworkMode::Host => "Host",
        NetworkMode::Client => "Client",
    };
    let transport = match settings.transport {
        NetworkTransport::Udp => "",
        NetworkTransport::Steam => ", Steam",
    };
    window.title = format!("Ruinbound ({role}{transport})");
}

fn spawn_status_label(mut commands: Commands) {
    commands.spawn((
        Name::new("Network Status Label"),
        NetworkStatusLabel,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            right: Val::Px(12.0),
            ..default()
        },
        // Draw above the run-setup screen and the in-game HUD.
        GlobalZIndex(100),
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.95, 1.0)),
    ));
}

fn update_status_label(
    settings: Res<NetworkSettings>,
    status: Res<NetworkStatus>,
    mut label: Query<(&mut Text, &mut TextColor), With<NetworkStatusLabel>>,
) {
    let (text, color) = describe_status(&settings, &status);
    for (mut label_text, mut label_color) in &mut label {
        label_text.0 = text.clone();
        label_color.0 = color;
    }
}

fn describe_status(settings: &NetworkSettings, status: &NetworkStatus) -> (String, Color) {
    const NORMAL: Color = Color::srgb(0.9, 0.95, 1.0);
    const PENDING: Color = Color::srgb(1.0, 0.85, 0.4);
    const ERROR: Color = Color::srgb(1.0, 0.45, 0.4);

    let player = match status.local_player_id {
        Some(id) => format!("Player {id}"),
        None => "awaiting player ID".to_string(),
    };
    let ping = match status.round_trip_ms {
        Some(ms) => format!(" - ping {ms} ms"),
        None => String::new(),
    };
    // What a host listens on and what a client is joining.
    let (host_where, join_target) = match settings.transport {
        NetworkTransport::Udp => (
            format!("port {}", settings.host_port),
            settings.server_addr.to_string(),
        ),
        NetworkTransport::Steam => (
            match (status.steam_id, status.steam_lobby) {
                (Some(id), Some(_)) => format!("Steam lobby open (Steam ID {id})"),
                (Some(id), None) => format!("Steam ID {id}"),
                (None, _) => "Steam".to_string(),
            },
            match status.steam_host {
                Some(id) => format!("Steam user {id}"),
                None => "a Steam friend".to_string(),
            },
        ),
    };

    match &status.state {
        MultiplayerConnectionState::Offline => (String::new(), NORMAL),
        MultiplayerConnectionState::StartingHost => {
            (format!("HOST - opening {host_where}..."), PENDING)
        }
        MultiplayerConnectionState::Hosting => (
            format!(
                "HOST - {host_where} - {player} - {} player(s) connected{ping}",
                status.connected_players
            ),
            NORMAL,
        ),
        MultiplayerConnectionState::WaitingForSteamInvite => (
            "CLIENT (Steam) - waiting for an invite: accept a friend's invite in Steam,\n\
             right-click them in Steam > Join Game, or press Join in the Steam Friends panel"
                .to_string(),
            PENDING,
        ),
        MultiplayerConnectionState::Connecting => {
            (format!("CLIENT - connecting to {join_target}..."), PENDING)
        }
        MultiplayerConnectionState::Connected => (
            format!("CLIENT - connected to {join_target} - {player}{ping}"),
            NORMAL,
        ),
        MultiplayerConnectionState::Disconnected { reason } => {
            (format!("Disconnected from host: {reason}"), ERROR)
        }
        MultiplayerConnectionState::ConnectionFailed { reason } => {
            (format!("Connection failed: {reason}"), ERROR)
        }
    }
}
