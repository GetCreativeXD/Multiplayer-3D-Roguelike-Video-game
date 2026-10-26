//! Steam lobby and friend invites (Milestone 9, step 2). Only compiled with
//! the `steam` feature, and only added for the Steam transport.
//!
//! The lobby is how friends *find* a session; the game itself still runs
//! over the Steam peer-to-peer link from step 1 (`SteamServerIo` /
//! `SteamClientIo`), connecting to the lobby's owner.
//!
//! Host (`--host-steam`): opens a friends-only lobby capped at
//! `MAX_PLAYERS`, tags it as a Ruinbound lobby with this build's
//! `PROTOCOL_ID`, and advertises it in rich presence (`connect`), so a
//! friend can right-click the host in Steam > Join Game, or accept an
//! invite. Steam itself refuses a fifth member, which caps the session.
//!
//! Client (`--join-steam` without an ID): waits. An accepted invite, Join
//! Game in Steam, or the Join button in the run-setup screen's Steam Friends
//! panel becomes `SteamEvent::JoinRequested`; the client enters the lobby,
//! checks it is a Ruinbound lobby of the same build, then connects to its
//! owner with `client::connect_over_steam`.
//!
//! Both: the Steam Friends panel (in `ui::RunSetupSideSlot`) lists friends
//! currently running a Steam build of Ruinbound (Steam reports them as
//! playing `STEAM_APP_ID`), with Invite (host) or Join (waiting client)
//! buttons. Invites work without the Steam overlay, which usually isn't
//! available when the game is started with `cargo run`.

use std::collections::HashSet;

use bevy::prelude::*;
use bevy::time::common_conditions::on_real_timer;
use lightyear::steam::prelude::SteamworksClient;
use lightyear::steam::prelude::steamworks::{FriendFlags, LobbyId, LobbyType, SteamId};

use crate::game_state::AppState;
use crate::ui::RunSetupSideSlot;

use super::protocol::PROTOCOL_ID;
use super::roster::MAX_PLAYERS;
use super::settings::{NetworkMode, NetworkSettings, connect_string_for_lobby};
use super::status::{MultiplayerConnectionState, NetworkStatus};
use super::steam::{STEAM_APP_ID, SteamEvent, SteamEvents, SteamIdentity};

/// Lobby data keys. Spacewar (App ID 480) is shared by many developers, so
/// a client only joins lobbies that say they are Ruinbound's.
const LOBBY_KEY_GAME: &str = "game";
const LOBBY_GAME_NAME: &str = "ruinbound";
const LOBBY_KEY_PROTOCOL: &str = "protocol";

/// How often the friends panel re-reads Steam's (locally cached) friend
/// list. Cheap, and quick enough that a clicked Invite updates promptly.
const FRIENDS_PANEL_REFRESH: std::time::Duration = std::time::Duration::from_millis(500);

pub struct SteamLobbyPlugin;

impl Plugin for SteamLobbyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SteamJoinProgress>()
            .init_resource::<SteamInvitesSent>()
            .add_systems(Startup, open_lobby_when_hosting)
            .add_systems(Update, handle_steam_events)
            .add_systems(
                Update,
                (
                    spawn_friends_panel,
                    refresh_friends_panel.run_if(on_real_timer(FRIENDS_PANEL_REFRESH)),
                    handle_friend_buttons,
                )
                    .chain()
                    .run_if(in_state(AppState::RunSetup)),
            );
    }
}

/// Where a Steam client is in joining someone. A join can only start while
/// no game connection has been opened yet; a failed lobby step (full,
/// closed, wrong build) leaves it free to try again.
#[derive(Resource, Default)]
struct SteamJoinProgress {
    /// Waiting for Steam's answer to `join_lobby`.
    entering_lobby: bool,
    /// The Lightyear connection to the host exists (from `--join-steam ID`
    /// at startup, or opened here after entering a lobby).
    connection_opened: bool,
}

/// Friends the host has invited this session (shown as "Invited").
#[derive(Resource, Default)]
struct SteamInvitesSent(HashSet<u64>);

// ---------------------------------------------------------------------
// Lobby
// ---------------------------------------------------------------------

/// Host: ask Steam for a friends-only lobby. The answer arrives as
/// `SteamEvent::LobbyCreated`.
fn open_lobby_when_hosting(
    settings: Res<NetworkSettings>,
    steam: Res<SteamworksClient>,
    events: Res<SteamEvents>,
    mut progress: ResMut<SteamJoinProgress>,
) {
    if settings.mode != NetworkMode::Host {
        // `--join-steam ID` connects at startup (`client::connect_to_host`).
        progress.connection_opened = settings.steam_host_id.is_some();
        return;
    }
    let events = events.clone();
    steam
        .0
        .matchmaking()
        .create_lobby(LobbyType::FriendsOnly, MAX_PLAYERS as u32, move |result| {
            events.push(SteamEvent::LobbyCreated(
                result
                    .map(|lobby| lobby.raw())
                    .map_err(|error| error.to_string()),
            ));
        });
}

/// Acts on everything Steam queued since last frame.
fn handle_steam_events(
    mut commands: Commands,
    steam: Res<SteamworksClient>,
    events: Res<SteamEvents>,
    identity: Res<SteamIdentity>,
    settings: Res<NetworkSettings>,
    time: Res<Time<Real>>,
    mut status: ResMut<NetworkStatus>,
    mut progress: ResMut<SteamJoinProgress>,
) {
    for event in events.drain() {
        match event {
            SteamEvent::LobbyCreated(Ok(lobby)) => {
                advertise_lobby(&steam, lobby);
                status.steam_lobby = Some(lobby);
                info!("[net] Steam lobby open: friends can join from Steam or be invited");
            }
            SteamEvent::LobbyCreated(Err(error)) => warn!(
                "[net] couldn't open a Steam lobby ({error}); friends can still join with \
                 --join-steam {}",
                identity.steam_id
            ),
            SteamEvent::JoinRequested { lobby } => {
                if settings.mode == NetworkMode::Host {
                    info!("[net] ignored a Steam join request: this game is hosting");
                } else if progress.connection_opened || progress.entering_lobby {
                    info!("[net] ignored a Steam join request: already joining a game");
                } else {
                    progress.entering_lobby = true;
                    status.state = MultiplayerConnectionState::Connecting;
                    let events = events.clone();
                    steam
                        .0
                        .matchmaking()
                        .join_lobby(LobbyId::from_raw(lobby), move |result| {
                            events.push(SteamEvent::LobbyEntered(
                                result.ok().map(|lobby| lobby.raw()),
                            ));
                        });
                    if settings.debug {
                        info!("[net] entering Steam lobby {lobby}");
                    }
                }
            }
            SteamEvent::LobbyEntered(entered) => {
                progress.entering_lobby = false;
                let Some(lobby) = entered else {
                    status.state = MultiplayerConnectionState::ConnectionFailed {
                        reason: "Steam couldn't enter that lobby (it may be full, or the host \
                                 closed the game). Waiting for another invite."
                            .into(),
                    };
                    continue;
                };
                match lobby_host(&steam, lobby, identity.steam_id) {
                    Ok(host) => {
                        status.steam_lobby = Some(lobby);
                        status.steam_host = Some(host);
                        progress.connection_opened = true;
                        super::client::connect_over_steam(&mut commands, host, &settings, &time);
                        set_rich_presence_status(&steam, "In a Ruinbound run");
                    }
                    Err(reason) => {
                        steam.0.matchmaking().leave_lobby(LobbyId::from_raw(lobby));
                        status.state = MultiplayerConnectionState::ConnectionFailed {
                            reason: format!("{reason} Waiting for another invite."),
                        };
                    }
                }
            }
        }
    }
}

/// Host: tag the lobby as Ruinbound's (game + build) and put its connect
/// string in rich presence, which is what makes "Join Game" appear on the
/// host in friends' Steam lists.
fn advertise_lobby(steam: &SteamworksClient, lobby: u64) {
    let matchmaking = steam.0.matchmaking();
    let lobby_id = LobbyId::from_raw(lobby);
    matchmaking.set_lobby_data(lobby_id, LOBBY_KEY_GAME, LOBBY_GAME_NAME);
    matchmaking.set_lobby_data(lobby_id, LOBBY_KEY_PROTOCOL, &PROTOCOL_ID.to_string());
    steam
        .0
        .friends()
        .set_rich_presence("connect", Some(&connect_string_for_lobby(lobby)));
    set_rich_presence_status(steam, "Hosting a Ruinbound run");
}

fn set_rich_presence_status(steam: &SteamworksClient, text: &str) {
    steam.0.friends().set_rich_presence("status", Some(text));
}

/// Client, after entering a lobby: the SteamID64 to connect to (the lobby's
/// owner), or why not.
fn lobby_host(steam: &SteamworksClient, lobby: u64, own_steam_id: u64) -> Result<u64, String> {
    let matchmaking = steam.0.matchmaking();
    let lobby_id = LobbyId::from_raw(lobby);

    if matchmaking.lobby_data(lobby_id, LOBBY_KEY_GAME).as_deref() != Some(LOBBY_GAME_NAME) {
        return Err("That Steam lobby isn't a Ruinbound game.".into());
    }
    let their_protocol = matchmaking.lobby_data(lobby_id, LOBBY_KEY_PROTOCOL);
    if their_protocol.as_deref() != Some(PROTOCOL_ID.to_string().as_str()) {
        return Err(format!(
            "The host is running a different build (protocol {} vs ours {PROTOCOL_ID}); \
             both players need the same version.",
            their_protocol.as_deref().unwrap_or("unknown")
        ));
    }
    let host = matchmaking.lobby_owner(lobby_id).raw();
    if host == own_steam_id {
        return Err("That is your own lobby.".into());
    }
    Ok(host)
}

// ---------------------------------------------------------------------
// Steam Friends panel (run-setup screen)
// ---------------------------------------------------------------------

#[derive(Component)]
struct SteamFriendsPanel;

#[derive(Component)]
struct SteamFriendsHint;

#[derive(Component)]
struct SteamFriendsList;

#[derive(Component, Clone, Copy)]
enum SteamFriendButton {
    /// Host: invite this friend into our lobby.
    Invite { friend: u64 },
    /// Waiting client: join the lobby this friend is in.
    Join { lobby: u64 },
}

/// What one row of the panel shows. Compared between refreshes so the rows
/// are only rebuilt when something changed.
#[derive(Clone, Debug, PartialEq)]
struct FriendRow {
    steam_id: u64,
    name: String,
    action: RowAction,
}

#[derive(Clone, Debug, PartialEq)]
enum RowAction {
    Invite,
    Invited,
    InLobby,
    Join {
        lobby: u64,
    },
    /// Running Ruinbound but not in a lobby we can join.
    NothingToJoin,
    /// Nothing to offer (this client is already joining or in a game).
    None,
}

const PANEL_TEXT: Color = Color::srgb(0.82, 0.82, 0.82);
const PANEL_MUTED: Color = Color::srgb(0.55, 0.58, 0.62);
const PANEL_TITLE: Color = Color::srgb(0.95, 0.82, 0.52);
const BUTTON_BG: Color = Color::srgb(0.2, 0.42, 0.24);
const BUTTON_HOVER_BG: Color = Color::srgb(0.27, 0.52, 0.31);
const BUTTON_BORDER: Color = Color::srgb(0.95, 0.82, 0.52);

/// Puts the panel into the run-setup screen's side slot (once).
fn spawn_friends_panel(
    mut commands: Commands,
    slots: Query<Entity, With<RunSetupSideSlot>>,
    existing: Query<(), With<SteamFriendsPanel>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Ok(slot) = slots.single() else {
        return;
    };
    commands.entity(slot).with_children(|slot| {
        slot.spawn((
            SteamFriendsPanel,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(6.0),
                ..default()
            },
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("STEAM FRIENDS"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(PANEL_TITLE),
            ));
            panel.spawn((
                SteamFriendsHint,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(PANEL_MUTED),
            ));
            panel.spawn((
                SteamFriendsList,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4.0),
                    ..default()
                },
            ));
        });
    });
}

/// Re-reads Steam's friend list and lobby state and rebuilds the rows when
/// anything shown changed.
#[allow(clippy::too_many_arguments)]
fn refresh_friends_panel(
    mut commands: Commands,
    steam: Res<SteamworksClient>,
    settings: Res<NetworkSettings>,
    status: Res<NetworkStatus>,
    progress: Res<SteamJoinProgress>,
    invites: Res<SteamInvitesSent>,
    mut hints: Query<&mut Text, With<SteamFriendsHint>>,
    lists: Query<Entity, With<SteamFriendsList>>,
    mut shown: Local<Option<(String, Vec<FriendRow>)>>,
) {
    let Ok(list) = lists.single() else {
        return;
    };
    let hosting = settings.mode == NetworkMode::Host;
    let can_join = !hosting && !progress.connection_opened && !progress.entering_lobby;

    let lobby_members: HashSet<u64> = match (hosting, status.steam_lobby) {
        (true, Some(lobby)) => steam
            .0
            .matchmaking()
            .lobby_members(LobbyId::from_raw(lobby))
            .iter()
            .map(|member| member.raw())
            .collect(),
        _ => HashSet::new(),
    };

    let mut rows: Vec<FriendRow> = steam
        .0
        .friends()
        .get_friends(FriendFlags::IMMEDIATE)
        .into_iter()
        .filter_map(|friend| {
            // Only friends running a Steam build of Ruinbound (Steam sees
            // them as playing `STEAM_APP_ID`).
            let game = friend.game_played()?;
            if game.game.app_id().0 != STEAM_APP_ID {
                return None;
            }
            let steam_id = friend.id().raw();
            let their_lobby = game.lobby.raw();
            let action = if hosting {
                if lobby_members.contains(&steam_id) {
                    RowAction::InLobby
                } else if status.steam_lobby.is_none() {
                    RowAction::None
                } else if invites.0.contains(&steam_id) {
                    RowAction::Invited
                } else {
                    RowAction::Invite
                }
            } else if !can_join {
                RowAction::None
            } else if their_lobby != 0 {
                RowAction::Join { lobby: their_lobby }
            } else {
                RowAction::NothingToJoin
            };
            Some(FriendRow {
                steam_id,
                name: friend.name(),
                action,
            })
        })
        .collect();
    rows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let hint = panel_hint(
        &steam,
        &settings,
        &status,
        hosting,
        can_join,
        rows.is_empty(),
    );
    let model = (hint, rows);
    if shown.as_ref() == Some(&model) {
        return;
    }

    for mut text in &mut hints {
        text.0 = model.0.clone();
    }
    commands.entity(list).despawn_related::<Children>();
    commands.entity(list).with_children(|list| {
        for row in &model.1 {
            spawn_friend_row(list, row);
        }
    });
    *shown = Some(model);
}

fn panel_hint(
    steam: &SteamworksClient,
    settings: &NetworkSettings,
    status: &NetworkStatus,
    hosting: bool,
    can_join: bool,
    no_friends: bool,
) -> String {
    const EMPTY: &str = "No friends are running the Steam build right now. To join you, \
                         they run:\ncargo run --features steam -- --join-steam";
    if hosting {
        return match (status.steam_lobby, no_friends) {
            (None, _) => "Opening a friends-only Steam lobby...".into(),
            (Some(_), true) => format!(
                "Lobby open (up to {MAX_PLAYERS} players). Friends can also right-click you \
                 in Steam > Join Game.\n\n{EMPTY}"
            ),
            (Some(_), false) => format!(
                "Lobby open (up to {MAX_PLAYERS} players). Invite a friend, or they can \
                 right-click you in Steam > Join Game."
            ),
        };
    }
    if can_join {
        return if no_friends {
            format!("Waiting for an invite.\n\n{EMPTY}")
        } else {
            "Waiting for an invite, or join a friend's lobby:".into()
        };
    }
    let host_name = status
        .steam_host
        .map(|host| steam.0.friends().get_friend(SteamId::from_raw(host)).name())
        .unwrap_or_else(|| "the host".into());
    match status.state {
        MultiplayerConnectionState::Connected => format!("Joined {host_name}'s game."),
        _ if settings.debug => format!("Joining {host_name} ({:?})...", status.state),
        _ => format!("Joining {host_name}..."),
    }
}

fn spawn_friend_row(list: &mut ChildSpawnerCommands, row: &FriendRow) {
    list.spawn(Node {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        column_gap: px(6.0),
        ..default()
    })
    .with_children(|line| {
        line.spawn((
            Text::new(row.name.clone()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(PANEL_TEXT),
        ));

        let button = match row.action {
            RowAction::Invite => Some((
                SteamFriendButton::Invite {
                    friend: row.steam_id,
                },
                "INVITE",
            )),
            RowAction::Join { lobby } => Some((SteamFriendButton::Join { lobby }, "JOIN")),
            _ => None,
        };
        if let Some((button, label)) = button {
            line.spawn((
                Button,
                button,
                Node {
                    padding: UiRect::axes(px(8.0), px(3.0)),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(BUTTON_BG),
                BorderColor::all(BUTTON_BORDER),
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        } else {
            let note = match row.action {
                RowAction::Invited => "invited",
                RowAction::InLobby => "in lobby",
                RowAction::NothingToJoin => "not hosting",
                _ => "",
            };
            line.spawn((
                Text::new(note),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(PANEL_MUTED),
            ));
        }
    });
}

fn handle_friend_buttons(
    mut buttons: Query<
        (&Interaction, &SteamFriendButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    steam: Res<SteamworksClient>,
    events: Res<SteamEvents>,
    status: Res<NetworkStatus>,
    mut invites: ResMut<SteamInvitesSent>,
) {
    for (interaction, button, mut background) in &mut buttons {
        *background = BackgroundColor(match interaction {
            Interaction::Hovered | Interaction::Pressed => BUTTON_HOVER_BG,
            Interaction::None => BUTTON_BG,
        });
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *button {
            SteamFriendButton::Invite { friend } => {
                let Some(lobby) = status.steam_lobby else {
                    continue;
                };
                // A game invite carrying our connect string: Steam shows it
                // to the friend, and accepting it reaches their running game
                // as `GameRichPresenceJoinRequested`.
                steam
                    .0
                    .friends()
                    .get_friend(SteamId::from_raw(friend))
                    .invite_user_to_game(&connect_string_for_lobby(lobby));
                invites.0.insert(friend);
                info!("[net] sent a Steam invite");
            }
            SteamFriendButton::Join { lobby } => {
                events.push(SteamEvent::JoinRequested { lobby });
            }
        }
    }
}
