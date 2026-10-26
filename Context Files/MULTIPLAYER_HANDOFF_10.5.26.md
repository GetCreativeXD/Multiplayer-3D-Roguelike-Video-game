# Ruinbound Multiplayer: Handoff (2026-10-05)

Read this first in a new chat. It continues `MULTIPLAYER_HANDOFF_10.4.26.md`
(Milestones 1 to 8, the `src/net/` file map, architecture, and earlier
limitations), which is still accurate for everything not listed here.

## Where we left off (end of 2026-10-05)

Milestone 9 (prediction and online play) started. Ben chose the order:
**Steam first, then fixed tick, then prediction**, with App ID 480
(Spacewar) for now, a Steam lobby + friend invites, and own-character
prediction with host correction (not Lightyear rollback).

| Step | State |
|---|---|
| 9a: Steam transport, join by Steam ID, ping readout (protocol 19) | **CONFIRMED by Ben** (Mac host + Windows PC client over Steam) |
| 9b: Steam lobby, invites, Join Game, Steam Friends panel | **CONFIRMED by Ben** |
| 9c: player movement on the fixed tick + transform interpolation | **CONFIRMED by Ben** |
| 9d: own-character prediction + host correction (protocol 20) | **Written, awaiting Ben's test** |

**Protocol is 20** (19 until 9d). Backups: `code before multiplayer m9a/`, `m9b/`, `m9c/`, `m9d/`.

## Test setup Ben uses

- Mac (host): `/Users/bencoughlin/Desktop/claude ruinbound/Ruinbound`
- Windows PC (client): `D:\Ruinbound\Ruinbound-main\Ruinbound-main`, a
  copy of the Mac project. **Changed files must be copied to the PC**
  (Claude sent a zip of the changed `src/...` files each step). Both
  machines must run the same build (protocol check).
- Two different Steam accounts, Steam friends, Steam running on both.
- Commands (everything after the lone `--` goes to the game):
  - host: `cargo run --features steam -- --host-steam`
  - client, wait for invite: `cargo run --features steam -- --join-steam`
  - client, direct: `cargo run --features steam -- --join-steam 7656119...`
  - PowerShell: don't type `<...>` placeholders literally.
  - UDP modes unchanged: `--host [PORT]`, `--join IP[:PORT]` with
    `--features multiplayer`.

## 9a: Steam transport (CONFIRMED 2026-10-05)

- Cargo feature `steam = ["multiplayer", "lightyear/steam"]` (Lightyear
  0.30 wraps `aeronet_steam` 0.21 / `steamworks` 0.12).
- `net/settings.rs`: `NetworkTransport { Udp, Steam }`, `--host-steam`,
  `--join-steam [ID]` (SteamID64 validated: top 12 bits `0x011`). Steam
  options in a non-steam build exit with a rebuild hint.
- `net/steam.rs` (steam only): `STEAM_APP_ID = 480`,
  `STEAM_VIRTUAL_PORT = 0`, `start_steam` (init, relay network access,
  `SteamworksClient` resource, `SteamIdentity`, callbacks pumped in
  `PreUpdate`). Called in `add_lightyear` **before** Lightyear's plugins;
  failure exits with "Open the Steam app, sign in".
- Host: `SteamServerIo { ListenTarget::Peer { virtual_port } }` instead of
  Netcode + UDP. Client: `SteamClientIo { ConnectTarget::Peer { steam_id,
  virtual_port } }`. Steam links skip Netcode, so
  `WelcomeMessage.protocol` (new) is checked by the client
  (`client::receive_welcome`), which disconnects on a mismatch.
- Client treats Lightyear `Linking` as connecting (Steam has no Netcode
  handshake); Steam attempt limit 30 s, measured per connection
  (`ConnectAttemptStarted`).
- Ping: `PingManager::rtt()` shown in the status label (client: to host;
  host: slowest joined player), refreshed every 0.5 s.

## 9b: Steam lobby and invites (CONFIRMED 2026-10-05)

- `net/steam_lobby.rs` (steam only, Steam transport only):
  - Host opens a friends-only lobby (max `MAX_PLAYERS` = 4, so Steam caps
    the session), sets lobby data `game=ruinbound`, `protocol=19`, and rich
    presence `connect=+connect_lobby <id>` (enables Join Game) and `status`.
  - `--join-steam` without an ID waits (`WaitingForSteamInvite`). Steam
    callbacks `GameLobbyJoinRequested` and `GameRichPresenceJoinRequested`
    queue `SteamEvent::JoinRequested`; the client enters the lobby, checks
    `game` and `protocol`, then `client::connect_over_steam` to the lobby
    owner. A failed lobby step lets the player try again; a failed game
    connection still needs a restart.
  - Steam Friends panel in `ui::RunSetupSideSlot` (new empty slot under
    "YOUR RUN"): friends playing `STEAM_APP_ID`, INVITE (host,
    `invite_user_to_game` with the connect string; works without the Steam
    overlay) or JOIN (waiting client, friend's lobby from
    `game_played().lobby`). Rows rebuild only when they change (0.5 s).
- `settings.rs`: `connect_string_for_lobby` / `lobby_from_connect_string`
  + tests. `status.rs`: `WaitingForSteamInvite`, `steam_host`,
  `steam_lobby`.
- Avoided `Friend::state()`: steamworks 0.12 hits `unreachable!()` on
  unknown persona states.

## 9c: movement on the fixed tick (CONFIRMED 2026-10-05)

Changed files: `src/player/mod.rs`, `src/player/life.rs`,
`src/net/intent.rs`.

- `PlayerMovementSet` (`update_dodge_timers`, `start_jump`, `start_dodge`,
  `character_movement`) now runs in **`FixedUpdate`** (64 Hz, Avian's
  step and `NETWORK_TICK_HZ`). The code is unchanged: `Res<Time>` is the
  fixed clock there.
- `PlayerInputSet` (`read_local_input`, `apply_local_input_to_player`,
  `life::incapacitate_fallen_players`) and the host's
  `net::intent::apply_remote_intents` now run once per frame in
  `RunFixedMainLoop` / `RunFixedMainLoopSystems::BeforeFixedMainLoop`, so a
  press reaches the next tick and this frame's `Update`.
- `PlayerIntent` now `#[require(TransformInterpolation)]` (Avian 0.7,
  `bevy_transform_interpolation` 0.5): every moving player character is
  eased between ticks. Update-frame writes to a player's `Transform`
  (attack rotation snaps and forward steps, Vanguard Afterimage return,
  Knight rush knockback, void reset) count as teleports, which is
  supported.
- `Update` systems still ordered `.after(PlayerInputSet)` /
  `.after(PlayerMovementSet)` (weapons, loot, intent send, animation) are
  unaffected: the fixed loop runs before `Update` each frame.
- **Combat deliberately stays on frame time** for now: it is
  host-authoritative and isn't replayed by own-movement prediction; moving
  ~60 combat/enemy systems would be a large change with regression risk
  and no prediction benefit. Revisit if Ben wants it.

Ben's acceptance test for 9c:
1. `cargo fmt`, `cargo check --features steam`, `cargo test --features multiplayer`.
2. Offline: walk, turn, jump/double jump, dodge (with/without WASD),
   inventory pausing movement, every class dash (Vanguard Lunge, Rhino
   Charge, Panther Pounce, Blade Rush, Daggers dashes), void reset, Saint
   shockwave jump. Movement and camera should look smooth (no stutter) and
   feel as before.
3. Host + client (UDP or Steam): both players move, jump and dodge as before.

## 9d: own-character prediction (WRITTEN, not yet tested)

Changed files: `src/net/prediction.rs` (new), `src/net/intent.rs`,
`src/net/avatars.rs`, `src/net/protocol.rs`, `src/net/mod.rs`,
`src/player/mod.rs`. Protocol 20.

- `player::LocallyDriven` marker: `apply_local_input_to_player` now drives
  `Player` **or** `LocallyDriven`. `player::movement_speed_multiplier`
  (class modifiers x buffs) is shared by `character_movement` and the host;
  `player::PredictedMovementSpeed` multiplies walking speed on a predicted
  avatar.
- Client (`ClientPredictionPlugin`): once `OwnAvatar` has a `Transform`, it
  gets `player_character_body()`, `PlayerIntent`, `LocallyDriven`,
  `OwnPrediction`, so the fixed-tick movement moves it at once (with
  interpolation). `avatars::smooth_toward_pose` skips `LocallyDriven`.
  `incapacitate_own_avatar` mirrors the replicated `LifeState`.
  Animation is now local for the own avatar (`update_character_motion`
  overwrites the replicated `CharacterMotion` before animation reads it).
- `PlayerIntentMessage.sequence`; the client records its position for every
  sent message (`OwnPrediction::record_sent_input`, history 256).
- Host (`HostPredictionPlugin`): `NetworkMovementAck { input_sequence,
  speed_percent }` on every `RemotePlayerBody` (PostUpdate, set on change),
  from `RemoteIntentState::last_sequence()`.
- Reconcile (Update, only when the acked sequence changes): error = host
  pose - recorded expected position. <= 0.35 ignored; > 6 snaps (Update
  write = teleport); otherwise added to `pending` and to every later
  recorded position (so it is corrected once), eased in FixedUpdate before
  movement at rate 10/s. Standing still, the avatar turns toward the host's
  facing (attacks turn the character on the host). Unit tests in
  `prediction::tests`.
- Not predicted (host result eased in): weapon/class dashes
  (`AbilityMovement`), knockback. Tolerated mismatch up to 0.35 units.

Ben's acceptance test for 9d:
1. `cargo fmt`, `cargo check --features steam`,
   `cargo test --features multiplayer` (new `net::prediction` tests).
2. Offline and host: unchanged.
3. Client over Steam (real latency): own walking, turning, jumping,
   double jumping and dodging respond immediately; the camera is smooth;
   walking into walls doesn't jitter; dashes and knockback catch up with a
   short slide; falling into the void resets; Downed stops the client
   moving; a Vanguard with Momentum or a Warden-blessed player doesn't
   rubber-band. `--net-debug` logs each correction.

## Known limitations added today

| Limitation | Planned fix |
|---|---|
| App ID 480: friends see "playing Spacewar"; Steam can't launch Ruinbound from an invite (the game must already be running) | Ruinbound's own App ID |
| A dropped Steam game connection needs a restart to rejoin | later |
| `--join-steam ID` (direct) bypasses the lobby, so no 4-player cap on that path | lobby path is the normal one |
