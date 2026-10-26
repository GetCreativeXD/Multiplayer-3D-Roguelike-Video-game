//! The co-op lobby (the run-setup screen) and the end of a run
//! (Milestone 8).
//!
//! - **Shared run settings.** The item drop pool, relics, pacts and Ruin
//!   condition are the same for the whole party
//!   (`run_setup::SharedRunSettings`); class and weapon stay per player.
//!   Any player may change them. A client sends its change to the host; the
//!   host applies it and sends the result to every client, and also sends
//!   its own changes and the current settings to anyone who joins. The host's
//!   copy wins if two players change something at the same moment.
//! - **Ready.** A client's Begin Run button is a READY toggle
//!   (`ui::LobbyStatus`). The host keeps who is ready and shows "Players
//!   ready: N/M"; Begin Run only works once every joined player is ready.
//!   Any change to the shared settings un-readies everyone, so nobody starts
//!   a run with settings they haven't seen.
//! - **Run end.** When the host's run ends (party wipe, see
//!   `player::life`), every client is told to go back to run setup too.
//!   On a client, everything the host replicated is despawned by the host
//!   (so it is kept out of the client's own run teardown via
//!   `PersistAcrossRuns`); connection entities are kept the same way.

use std::collections::HashSet;

use bevy::prelude::*;
use lightyear::connection::client_of::ClientOf;
use lightyear::connection::host::HostClient;
use lightyear::prelude::client::Remote;
use lightyear::prelude::*;

use crate::game_state::{AppState, PersistAcrossRuns};
use crate::run_setup::{RunConfig, SharedRunSettings};
use crate::ui::{LobbyRole, LobbyStatus};

use super::protocol::{
    LobbyMessage, NetworkPlayerId, ReadyMessage, RunEndedMessage, RunSettingsMessage,
    SessionChannel,
};
use super::roster::PlayerRoster;
use super::status::NetworkStatus;

// ---------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------

pub struct HostLobbyPlugin;

impl Plugin for HostLobbyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReadyPlayers>()
            .init_resource::<LastSentSettings>()
            .add_systems(
                Update,
                (
                    host_receive_settings,
                    host_share_settings,
                    host_receive_ready,
                    host_publish_lobby,
                )
                    .chain(),
            )
            .add_systems(OnEnter(AppState::RunSetup), clear_ready_players)
            .add_systems(OnExit(AppState::InGame), tell_clients_the_run_ended)
            .add_observer(keep_links_across_runs)
            .add_observer(send_settings_to_new_player);
    }
}

/// Joined players the host has marked ready.
#[derive(Resource, Default)]
struct ReadyPlayers(HashSet<NetworkPlayerId>);

/// The shared settings the host last sent to everyone.
#[derive(Resource, Default)]
struct LastSentSettings(Option<SharedRunSettings>);

/// Connections are created whenever a player joins, possibly mid-run, and
/// must survive the run's teardown.
fn keep_links_across_runs(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands.entity(trigger.entity).insert(PersistAcrossRuns);
}

fn send_settings_to_new_player(
    trigger: On<Add, Connected>,
    run_config: Res<RunConfig>,
    mut links: Query<&mut MessageSender<RunSettingsMessage>, (With<ClientOf>, Without<HostClient>)>,
) {
    if let Ok(mut sender) = links.get_mut(trigger.entity) {
        sender.send::<SessionChannel>(RunSettingsMessage(run_config.shared_settings()));
    }
}

/// A client changed a shared setting: take it (the host then shares the
/// result with everyone in `host_share_settings`).
fn host_receive_settings(
    mut links: Query<&mut MessageReceiver<RunSettingsMessage>, With<ClientOf>>,
    mut run_config: ResMut<RunConfig>,
) {
    for mut receiver in &mut links {
        if let Some(RunSettingsMessage(settings)) = receiver.receive().last() {
            if run_config.shared_settings() != settings {
                run_config.apply_shared_settings(&settings);
            }
        }
    }
}

/// Whenever the shared settings differ from what clients last got (the
/// host's own click, or a client's change just applied), send them to
/// every client and un-ready everyone.
fn host_share_settings(
    run_config: Res<RunConfig>,
    mut last_sent: ResMut<LastSentSettings>,
    mut ready: ResMut<ReadyPlayers>,
    mut links: Query<
        &mut MessageSender<RunSettingsMessage>,
        (With<ClientOf>, With<Connected>, Without<HostClient>),
    >,
) {
    let current = run_config.shared_settings();
    if last_sent.0.as_ref() == Some(&current) {
        return;
    }
    // The first snapshot after start-up isn't a change anyone made.
    if last_sent.0.is_some() && !ready.0.is_empty() {
        ready.0.clear();
        info!("[net] Shared run settings changed: everyone is un-readied");
    }
    for mut sender in &mut links {
        sender.send::<SessionChannel>(RunSettingsMessage(current.clone()));
    }
    last_sent.0 = Some(current);
}

fn host_receive_ready(
    roster: Res<PlayerRoster>,
    app_state: Res<State<AppState>>,
    mut links: Query<(Entity, &mut MessageReceiver<ReadyMessage>), With<ClientOf>>,
    mut ready: ResMut<ReadyPlayers>,
) {
    for (link, mut receiver) in &mut links {
        let Some(message) = receiver.receive().last() else {
            continue;
        };
        // Readiness only means something on the run-setup screen.
        if *app_state.get() != AppState::RunSetup {
            continue;
        }
        let Some(sender) = roster.player_for_connection(link) else {
            continue;
        };
        if message.ready {
            ready.0.insert(sender.player_id);
        } else {
            ready.0.remove(&sender.player_id);
        }
    }

    // Forget players who left.
    ready.0.retain(|player| roster.contains(*player));
}

/// Keeps the host's own screen (`LobbyStatus`) and every client's copy of
/// who is ready up to date. Only sends when something changed.
fn host_publish_lobby(
    roster: Res<PlayerRoster>,
    ready: Res<ReadyPlayers>,
    mut lobby: ResMut<LobbyStatus>,
    mut links: Query<
        &mut MessageSender<LobbyMessage>,
        (With<ClientOf>, With<Connected>, Without<HostClient>),
    >,
    mut last_sent: Local<Option<LobbyMessage>>,
) {
    let guests: Vec<NetworkPlayerId> = roster
        .players()
        .filter(|player| !player.is_host)
        .map(|player| player.player_id)
        .collect();
    let mut ready_players: Vec<NetworkPlayerId> = guests
        .iter()
        .copied()
        .filter(|player| ready.0.contains(player))
        .collect();
    ready_players.sort_by_key(|player| player.0);

    let message = LobbyMessage {
        guests: guests.len() as u32,
        ready_players,
    };

    let status = LobbyStatus {
        role: LobbyRole::Host,
        local_ready: true,
        guests: guests.len(),
        ready_guests: message.ready_players.len(),
        ready_toggle_requested: false,
    };
    if lobby.role != status.role
        || lobby.guests != status.guests
        || lobby.ready_guests != status.ready_guests
    {
        *lobby = status;
    }

    if last_sent.as_ref() == Some(&message) {
        return;
    }
    for mut sender in &mut links {
        sender.send::<SessionChannel>(message.clone());
    }
    *last_sent = Some(message);
}

fn clear_ready_players(mut ready: ResMut<ReadyPlayers>) {
    ready.0.clear();
}

fn tell_clients_the_run_ended(
    mut links: Query<
        &mut MessageSender<RunEndedMessage>,
        (With<ClientOf>, With<Connected>, Without<HostClient>),
    >,
) {
    for mut sender in &mut links {
        sender.send::<SessionChannel>(RunEndedMessage);
    }
}

// ---------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------

pub struct ClientLobbyPlugin;

impl Plugin for ClientLobbyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SettingsSyncedWithHost>()
            .add_systems(
                Update,
                (
                    client_receive_settings,
                    client_share_settings,
                    client_receive_lobby,
                    client_send_ready,
                    client_receive_run_ended,
                )
                    .chain(),
            )
            .add_observer(keep_host_entities_for_the_host);
    }
}

/// The shared settings as last agreed with the host. `None` until the host
/// sends them, so a client never pushes its own defaults over the host's.
#[derive(Resource, Default)]
struct SettingsSyncedWithHost(Option<SharedRunSettings>);

/// The host despawns what it replicated when a run ends; the client must
/// not despawn those entities itself.
fn keep_host_entities_for_the_host(trigger: On<Add, Remote>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .try_insert(PersistAcrossRuns);
}

fn client_receive_settings(
    mut connection: Query<&mut MessageReceiver<RunSettingsMessage>, With<Client>>,
    mut run_config: ResMut<RunConfig>,
    mut synced: ResMut<SettingsSyncedWithHost>,
) {
    for mut receiver in &mut connection {
        if let Some(RunSettingsMessage(settings)) = receiver.receive().last() {
            if run_config.shared_settings() != settings {
                run_config.apply_shared_settings(&settings);
            }
            synced.0 = Some(settings);
        }
    }
}

/// This player changed a shared setting: send it to the host.
fn client_share_settings(
    run_config: Res<RunConfig>,
    mut synced: ResMut<SettingsSyncedWithHost>,
    mut connection: Query<&mut MessageSender<RunSettingsMessage>, (With<Client>, With<Connected>)>,
) {
    let Some(agreed) = synced.0.as_ref() else {
        return;
    };
    let current = run_config.shared_settings();
    if *agreed == current {
        return;
    }
    for mut sender in &mut connection {
        sender.send::<SessionChannel>(RunSettingsMessage(current.clone()));
    }
    synced.0 = Some(current);
}

fn client_receive_lobby(
    status: Res<NetworkStatus>,
    mut connection: Query<&mut MessageReceiver<LobbyMessage>, With<Client>>,
    mut lobby: ResMut<LobbyStatus>,
) {
    if lobby.role != LobbyRole::Guest {
        lobby.role = LobbyRole::Guest;
    }
    for mut receiver in &mut connection {
        let Some(message) = receiver.receive().last() else {
            continue;
        };
        lobby.guests = message.guests as usize;
        lobby.ready_guests = message.ready_players.len();
        lobby.local_ready = status
            .local_player_id
            .is_some_and(|own| message.ready_players.iter().any(|player| player.0 == own));
    }
}

fn client_send_ready(
    mut lobby: ResMut<LobbyStatus>,
    mut connection: Query<&mut MessageSender<ReadyMessage>, (With<Client>, With<Connected>)>,
) {
    if !lobby.ready_toggle_requested {
        return;
    }
    lobby.ready_toggle_requested = false;
    let ready = !lobby.local_ready;
    for mut sender in &mut connection {
        sender.send::<SessionChannel>(ReadyMessage { ready });
    }
}

fn client_receive_run_ended(
    mut connection: Query<&mut MessageReceiver<RunEndedMessage>, With<Client>>,
    app_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut lobby: ResMut<LobbyStatus>,
) {
    for mut receiver in &mut connection {
        if receiver.receive().count() > 0 && *app_state.get() == AppState::InGame {
            info!("[net] The host ended the run; returning to run setup");
            lobby.local_ready = false;
            next_state.set(AppState::RunSetup);
        }
    }
}
