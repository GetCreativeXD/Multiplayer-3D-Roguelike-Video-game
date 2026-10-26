//! Steam as a transport (Milestone 9, step 1). Only compiled with the
//! `steam` feature.
//!
//! Players reach each other through Steam's peer-to-peer relay network
//! (Steam Datagram Relay), addressed by SteamID64 instead of an IP address,
//! so nobody needs port forwarding and nobody's IP is revealed.
//!
//! Lightyear 0.30's `steam` feature wraps `aeronet_steam`: the host's server
//! entity gets `SteamServerIo` (listening on a Steam "virtual port") and a
//! client's connection entity gets `SteamClientIo` (connecting to the host's
//! SteamID64). Steam links authenticate through Steam itself, so they skip
//! Netcode; the build check Netcode used to do (`PROTOCOL_ID`) is done by
//! the welcome message instead (see `client::receive_welcome`).
//!
//! The Steam API must be started before Lightyear's plugins are added, and
//! its callbacks must be pumped every frame (`run_steam_callbacks`).

use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use lightyear::steam::prelude::{SteamworksClient, steamworks};

use super::settings::lobby_from_connect_string;

/// Valve's public test app ("Spacewar"). Every Steam account can use it, so
/// it works before Ruinbound has its own Steamworks app. Friends will see
/// you as "playing Spacewar". Replace with Ruinbound's App ID once it exists.
pub const STEAM_APP_ID: u32 = 480;

/// Steam's equivalent of a port number for peer-to-peer sockets. Host and
/// client must agree. Anything Ruinbound-specific is fine; 0 is Steam's
/// conventional default.
pub const STEAM_VIRTUAL_PORT: i32 = 0;

/// Who this machine is on Steam. Shown in the status label so the host can
/// tell friends what to join.
#[derive(Resource, Clone, Debug)]
pub struct SteamIdentity {
    pub steam_id: u64,
    pub persona_name: String,
}

/// Something Steam told us through a callback. Steam runs callbacks inside
/// `run_callbacks`, outside any system's borrows, so they only queue an
/// event here; `net::steam_lobby` acts on the queue in a normal system.
#[derive(Debug, Clone)]
pub enum SteamEvent {
    /// Our `create_lobby` request finished (host).
    LobbyCreated(Result<u64, String>),
    /// This player accepted an invite or chose Join Game in Steam (or in
    /// Ruinbound's friends panel): enter that lobby and join its owner.
    JoinRequested { lobby: u64 },
    /// Our `join_lobby` request finished (client). `None` means Steam
    /// refused (lobby full, closed, or not visible to us).
    LobbyEntered(Option<u64>),
}

/// Thread-safe queue of `SteamEvent`s.
#[derive(Resource, Clone, Default)]
pub struct SteamEvents(Arc<Mutex<Vec<SteamEvent>>>);

impl SteamEvents {
    pub fn push(&self, event: SteamEvent) {
        if let Ok(mut events) = self.0.lock() {
            events.push(event);
        }
    }

    pub fn drain(&self) -> Vec<SteamEvent> {
        self.0
            .lock()
            .map(|mut events| std::mem::take(&mut *events))
            .unwrap_or_default()
    }
}

/// Keeps Steam callback registrations alive (dropping a handle unregisters
/// the callback).
#[derive(Resource)]
struct SteamCallbacks(#[allow(dead_code)] Vec<steamworks::CallbackHandle>);

/// Starts the Steam API for `STEAM_APP_ID` and inserts the
/// `SteamworksClient` resource Lightyear's Steam IO needs. Call before
/// adding Lightyear's plugins.
///
/// Fails if Steam isn't running, isn't logged in, or runs as a different OS
/// user than the game.
pub fn start_steam(app: &mut App) -> Result<SteamIdentity, String> {
    let client = steamworks::Client::init_app(STEAM_APP_ID).map_err(|error| error.to_string())?;

    // Starts the relay network handshake now rather than on first connect,
    // so the first connection attempt doesn't pay for it.
    client.networking_utils().init_relay_network_access();

    let identity = SteamIdentity {
        steam_id: client.user().steam_id().raw(),
        persona_name: client.friends().name(),
    };

    // Join requests can arrive two ways, depending on how the friend
    // reached us; both end up as `JoinRequested`.
    let events = SteamEvents::default();
    let lobby_requests = events.clone();
    let presence_requests = events.clone();
    let callbacks = vec![
        // "Join Game" on a friend who is in a lobby, or an accepted lobby
        // invite.
        client.register_callback(move |request: steamworks::GameLobbyJoinRequested| {
            lobby_requests.push(SteamEvent::JoinRequested {
                lobby: request.lobby_steam_id.raw(),
            });
        }),
        // An accepted `invite_user_to_game`, or "Join Game" on a friend
        // whose rich presence carries a `connect` string.
        client.register_callback(move |request: steamworks::GameRichPresenceJoinRequested| {
            match lobby_from_connect_string(&request.connect) {
                Some(lobby) => presence_requests.push(SteamEvent::JoinRequested { lobby }),
                None => warn!(
                    "[net] ignored a Steam join request that isn't a Ruinbound lobby ({:?})",
                    request.connect
                ),
            }
        }),
    ];

    app.insert_resource(SteamworksClient(client))
        .insert_resource(identity.clone())
        .insert_resource(events)
        .insert_resource(SteamCallbacks(callbacks))
        .add_systems(PreUpdate, run_steam_callbacks);
    Ok(identity)
}

/// Steam delivers connection events (and later lobby/invite events) through
/// callbacks that only fire when pumped.
fn run_steam_callbacks(steam: Res<SteamworksClient>) {
    steam.0.run_callbacks();
}
