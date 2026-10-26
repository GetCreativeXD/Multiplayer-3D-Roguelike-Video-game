//! How this process takes part in a session (offline, hosting, or joining),
//! read once from the command line at startup.
//!
//! ```text
//! cargo run                                            # offline (default)
//! cargo run --features multiplayer -- --host           # host on port 5888
//! cargo run --features multiplayer -- --host 6000      # host on port 6000
//! cargo run --features multiplayer -- --join 127.0.0.1 # join on port 5888
//! cargo run --features multiplayer -- --join 192.168.1.20:6000 --net-debug
//! cargo run --features steam -- --host-steam           # host over Steam
//! cargo run --features steam -- --join-steam 76561198000000000
//! cargo run --features steam -- --join-steam           # wait for a Steam invite
//! ```
//!
//! Parsing is plain `std` (no CLI crate) and is kept free of Bevy and
//! Lightyear types so it can be unit tested in the default build.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bevy::prelude::*;

/// UDP port a host listens on when `--host` is given without a port.
pub const DEFAULT_PORT: u16 = 5888;

/// Which role this process plays in a session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkMode {
    /// Single-player, exactly as before multiplayer existed. No networking
    /// plugins are added at all.
    Offline,
    /// Runs the authoritative simulation and also plays as a local client
    /// (a "listen server").
    Host,
    /// Connects to a host and waits for the host's world.
    Client,
}

/// How the players' machines reach each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkTransport {
    /// Direct UDP to an IP address: same machine, LAN, or port forwarding.
    Udp,
    /// Steam's peer-to-peer relay network: players connect by Steam ID, no
    /// port forwarding needed. Requires the `steam` build feature and a
    /// running, logged-in Steam client (Milestone 9).
    Steam,
}

/// The startup choice for this process. Inserted as a resource by
/// `NetworkPlugin` and never changed afterwards.
#[derive(Resource, Clone, Debug, PartialEq)]
// The address/port/debug fields are only read by the Lightyear-backed code
// behind the `multiplayer` feature.
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
pub struct NetworkSettings {
    pub mode: NetworkMode,
    /// UDP (`--host` / `--join`) or Steam (`--host-steam` / `--join-steam`).
    pub transport: NetworkTransport,
    /// The host's SteamID64 (Client mode over Steam only). `None` means
    /// "wait for a friend's Steam invite or Join Game" (Milestone 9 step 2).
    pub steam_host_id: Option<u64>,
    /// Port the host listens on (Host mode only).
    pub host_port: u16,
    /// Host address to connect to (Client mode only).
    pub server_addr: SocketAddr,
    /// Enables extra connection logging. Off by default (`--net-debug`).
    pub debug: bool,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            mode: NetworkMode::Offline,
            transport: NetworkTransport::Udp,
            steam_host_id: None,
            host_port: DEFAULT_PORT,
            server_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_PORT),
            debug: false,
        }
    }
}

/// Why the multiplayer command-line arguments could not be understood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkArgsError {
    HostAndJoinTogether,
    MissingJoinAddress,
    InvalidPort(String),
    InvalidAddress(String),
    InvalidSteamId(String),
    UdpAndSteamTogether,
}

impl fmt::Display for NetworkArgsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HostAndJoinTogether => write!(f, "use either --host or --join, not both"),
            Self::MissingJoinAddress => write!(f, "--join needs an address, e.g. --join 127.0.0.1"),
            Self::InvalidPort(value) => write!(f, "'{value}' is not a valid port (1-65535)"),
            Self::InvalidAddress(value) => write!(
                f,
                "'{value}' is not a valid address (expected IP or IP:PORT, e.g. 192.168.1.20:5888)"
            ),
            Self::InvalidSteamId(value) => write!(
                f,
                "'{value}' is not a SteamID64 (a 17-digit number starting 7656119...; \
                 the host's window and log show it)"
            ),
            Self::UdpAndSteamTogether => {
                write!(
                    f,
                    "use either the IP options or the Steam options, not both"
                )
            }
        }
    }
}

pub const USAGE: &str = "\
Multiplayer options (requires building with --features multiplayer):
  --host [PORT]       host a session (default port 5888)
  --join ADDR[:PORT]  join a host, e.g. --join 127.0.0.1 or --join 192.168.1.20:5888
  --host-steam        host over Steam (needs --features steam and Steam running)
  --join-steam [ID]   join over Steam: with the host's SteamID64, connect directly;
                      without it, wait for a friend's invite (or use Join in the
                      Steam Friends panel / Join Game in Steam)
  --net-debug         log extra connection details";

impl NetworkSettings {
    /// Parses the arguments after the program name. Unknown arguments are
    /// ignored so other tools (e.g. `cargo` or Bevy flags) are unaffected.
    pub fn from_args<I, S>(args: I) -> Result<Self, NetworkArgsError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let args: Vec<String> = args.into_iter().map(Into::into).collect();
        let mut settings = Self::default();
        let mut hosting = false;
        let mut joining = false;
        let mut udp_flag = false;
        let mut steam_flag = false;

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--host" => {
                    hosting = true;
                    udp_flag = true;
                    // The port is optional: only consume the next argument
                    // if it isn't another flag.
                    if let Some(next) = args.get(index + 1).filter(|next| !next.starts_with("--")) {
                        settings.host_port = parse_port(next)?;
                        index += 1;
                    }
                }
                "--join" => {
                    joining = true;
                    udp_flag = true;
                    let address = args
                        .get(index + 1)
                        .filter(|next| !next.starts_with("--"))
                        .ok_or(NetworkArgsError::MissingJoinAddress)?;
                    settings.server_addr = parse_server_address(address)?;
                    index += 1;
                }
                "--host-steam" => {
                    hosting = true;
                    steam_flag = true;
                }
                "--join-steam" => {
                    joining = true;
                    steam_flag = true;
                    // The ID is optional: without one, wait for an invite.
                    if let Some(id) = args.get(index + 1).filter(|next| !next.starts_with("--")) {
                        settings.steam_host_id = Some(parse_steam_id(id)?);
                        index += 1;
                    }
                }
                "--net-debug" => settings.debug = true,
                _ => {}
            }
            index += 1;
        }

        if udp_flag && steam_flag {
            return Err(NetworkArgsError::UdpAndSteamTogether);
        }
        if steam_flag {
            settings.transport = NetworkTransport::Steam;
        }
        settings.mode = match (hosting, joining) {
            (true, true) => return Err(NetworkArgsError::HostAndJoinTogether),
            (true, false) => NetworkMode::Host,
            (false, true) => NetworkMode::Client,
            (false, false) => NetworkMode::Offline,
        };
        Ok(settings)
    }
}

fn parse_port(value: &str) -> Result<u16, NetworkArgsError> {
    match value.parse::<u16>() {
        Ok(port) if port != 0 => Ok(port),
        _ => Err(NetworkArgsError::InvalidPort(value.to_string())),
    }
}

/// A SteamID64 for an individual account: universe 1 ("public"), account
/// type 1 ("individual"), i.e. the top 12 bits are `0x011`. Rejecting
/// anything else catches typos and pasted account names early.
fn parse_steam_id(value: &str) -> Result<u64, NetworkArgsError> {
    match value.trim().parse::<u64>() {
        Ok(id) if id >> 52 == 0x011 => Ok(id),
        _ => Err(NetworkArgsError::InvalidSteamId(value.to_string())),
    }
}

/// Prefix of the Steam "connect string" Ruinbound puts in rich presence and
/// invites. Steam hands the whole string back to the invited player's game
/// (`GameRichPresenceJoinRequested`), and would also append it to the command
/// line if Steam had to launch the game.
const CONNECT_LOBBY_PREFIX: &str = "+connect_lobby ";

/// The connect string that leads a friend into `lobby`.
#[cfg_attr(not(feature = "steam"), allow(dead_code))]
pub fn connect_string_for_lobby(lobby: u64) -> String {
    format!("{CONNECT_LOBBY_PREFIX}{lobby}")
}

/// The lobby named by a connect string, if it is one of Ruinbound's.
#[cfg_attr(not(feature = "steam"), allow(dead_code))]
pub fn lobby_from_connect_string(connect: &str) -> Option<u64> {
    connect
        .trim()
        .strip_prefix(CONNECT_LOBBY_PREFIX)?
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|lobby| *lobby != 0)
}

/// Accepts `IP`, `IP:PORT`, `localhost`, or `localhost:PORT`.
fn parse_server_address(value: &str) -> Result<SocketAddr, NetworkArgsError> {
    if let Ok(address) = value.parse::<SocketAddr>() {
        return Ok(address);
    }
    if let Ok(ip) = value.parse::<IpAddr>() {
        return Ok(SocketAddr::new(ip, DEFAULT_PORT));
    }

    let (host, port) = match value.split_once(':') {
        Some((host, port)) => (host, parse_port(port)?),
        None => (value, DEFAULT_PORT),
    };
    if host.eq_ignore_ascii_case("localhost") {
        return Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port));
    }
    Err(NetworkArgsError::InvalidAddress(value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<NetworkSettings, NetworkArgsError> {
        NetworkSettings::from_args(args.iter().copied())
    }

    #[test]
    fn no_arguments_means_offline() {
        assert_eq!(parse(&[]).unwrap().mode, NetworkMode::Offline);
    }

    #[test]
    fn host_uses_default_port() {
        let settings = parse(&["--host"]).unwrap();
        assert_eq!(settings.mode, NetworkMode::Host);
        assert_eq!(settings.host_port, DEFAULT_PORT);
    }

    #[test]
    fn host_accepts_custom_port_and_later_flags() {
        let settings = parse(&["--host", "6000", "--net-debug"]).unwrap();
        assert_eq!(settings.host_port, 6000);
        assert!(settings.debug);
    }

    #[test]
    fn host_does_not_swallow_a_following_flag() {
        let settings = parse(&["--host", "--net-debug"]).unwrap();
        assert_eq!(settings.host_port, DEFAULT_PORT);
        assert!(settings.debug);
    }

    #[test]
    fn join_accepts_ip_with_and_without_port() {
        let with_port = parse(&["--join", "192.168.1.20:6000"]).unwrap();
        assert_eq!(with_port.mode, NetworkMode::Client);
        assert_eq!(with_port.server_addr, "192.168.1.20:6000".parse().unwrap());

        let without_port = parse(&["--join", "10.0.0.5"]).unwrap();
        assert_eq!(without_port.server_addr.port(), DEFAULT_PORT);
    }

    #[test]
    fn join_accepts_localhost() {
        let settings = parse(&["--join", "localhost:7000"]).unwrap();
        assert_eq!(settings.server_addr, "127.0.0.1:7000".parse().unwrap());
    }

    #[test]
    fn invalid_input_is_rejected() {
        assert_eq!(
            parse(&["--join"]),
            Err(NetworkArgsError::MissingJoinAddress)
        );
        assert_eq!(
            parse(&["--host", "0"]),
            Err(NetworkArgsError::InvalidPort("0".into()))
        );
        assert_eq!(
            parse(&["--join", "not an address"]),
            Err(NetworkArgsError::InvalidAddress("not an address".into()))
        );
        assert_eq!(
            parse(&["--host", "--join", "127.0.0.1"]),
            Err(NetworkArgsError::HostAndJoinTogether)
        );
    }

    #[test]
    fn ip_options_use_udp() {
        assert_eq!(parse(&["--host"]).unwrap().transport, NetworkTransport::Udp);
        assert_eq!(
            parse(&["--join", "127.0.0.1"]).unwrap().transport,
            NetworkTransport::Udp
        );
    }

    #[test]
    fn steam_host_and_join() {
        let host = parse(&["--host-steam", "--net-debug"]).unwrap();
        assert_eq!(host.mode, NetworkMode::Host);
        assert_eq!(host.transport, NetworkTransport::Steam);
        assert!(host.debug);

        let join = parse(&["--join-steam", "76561198000000042"]).unwrap();
        assert_eq!(join.mode, NetworkMode::Client);
        assert_eq!(join.transport, NetworkTransport::Steam);
        assert_eq!(join.steam_host_id, Some(76561198000000042));
    }

    #[test]
    fn join_steam_without_an_id_waits_for_an_invite() {
        let join = parse(&["--join-steam", "--net-debug"]).unwrap();
        assert_eq!(join.mode, NetworkMode::Client);
        assert_eq!(join.transport, NetworkTransport::Steam);
        assert_eq!(join.steam_host_id, None);
        assert!(join.debug);
    }

    #[test]
    fn invalid_steam_input_is_rejected() {
        assert_eq!(
            parse(&["--join-steam", "12345"]),
            Err(NetworkArgsError::InvalidSteamId("12345".into()))
        );
        assert_eq!(
            parse(&["--join-steam", "ben"]),
            Err(NetworkArgsError::InvalidSteamId("ben".into()))
        );
        assert_eq!(
            parse(&["--host-steam", "--join-steam", "76561198000000042"]),
            Err(NetworkArgsError::HostAndJoinTogether)
        );
        assert_eq!(
            parse(&["--host", "--host-steam"]),
            Err(NetworkArgsError::UdpAndSteamTogether)
        );
    }

    #[test]
    fn connect_strings_round_trip() {
        let connect = connect_string_for_lobby(109775241000000001);
        assert_eq!(
            lobby_from_connect_string(&connect),
            Some(109775241000000001)
        );
        assert_eq!(lobby_from_connect_string("  +connect_lobby 42 "), Some(42));
        assert_eq!(lobby_from_connect_string("+connect 1.2.3.4:27015"), None);
        assert_eq!(lobby_from_connect_string("+connect_lobby 0"), None);
        assert_eq!(lobby_from_connect_string("+connect_lobby abc"), None);
        assert_eq!(lobby_from_connect_string(""), None);
    }

    #[test]
    fn unknown_arguments_are_ignored() {
        assert_eq!(
            parse(&["--something-else"]).unwrap().mode,
            NetworkMode::Offline
        );
    }
}
