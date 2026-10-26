# Ruinbound Multiplayer Audit (Milestone 0)

Written 2026-10-03 against the source in `src/` (34.6k lines) and
`RUINBOUND_CONTEXT_9.11.26.md`, before any multiplayer code existed.
Companion to `RUINBOUND_LIGHTYEAR_MULTIPLAYER_PLAN.md`.

## Versions

| Item | Value |
|---|---|
| Bevy | 0.19 |
| Avian3D | 0.7 |
| Lightyear chosen | **0.30** (0.30.1 on crates.io, 2026-09-16). Its workspace pins `bevy = "0.19"` and `avian3d = "0.7"`, the same as Ruinbound. |
| API reference used | Lightyear tag `0.30.0`: `examples/simple_setup`, `examples/common` (HostClient mode, Netcode, UDP) |

Lightyear 0.30 is built on `bevy_replicon` internally. Connections are entities
(`Server`, `Client`, `LinkOf`, `ClientOf`, `HostClient`) and messages use the
`MessageSender<M>` / `MessageReceiver<M>` components.

## Baseline build

**Not established by Claude.** Neither sandbox can build the project: the
device shell has no Rust toolchain or network, and the cloud container has
`cargo` but crates.io is blocked by the egress policy. Run `cargo check` on
your Mac before and after these changes to confirm the baseline.

## App structure

- **Entry point:** a single binary (`src/main.rs`) with one `App` and no CLI.
- **States:** `AppState { RunSetup (default), InGame }`. The only transition
  is the "Begin Run" button (`ui::run_setup_screen::handle_begin_run_click`).
- **Run start:** `OnEnter(AppState::InGame)` →
  `rooms::fallen_gate::setup_fallen_gate_player_and_scene` (spawns the
  player and HUD) and `rooms::RoomsPlugin` (the authored room chain Fallen
  Gate → Ruined Barracks → Shattered Junction → Soldiers Passage → Rubble
  Bridge → Saint Arena).
- **Run config:** `run_setup::RunConfig` (class, weapon, drop pool, relics,
  pacts, Ruin condition), frozen into `RunModifiers` at Begin Run. This is
  host-owned settings data that can be sent to clients as a message later.

## Single-player assumptions that multiplayer must undo

| Finding | Evidence | Impact | Smallest migration path |
|---|---|---|---|
| Gameplay reads the keyboard and mouse directly | `ButtonInput<KeyCode/MouseButton>` is read in 17 files (player movement, every class ability file, melee, weapons, switching, loot pickup, chests, camera, UI) | The host can't drive a remote player's character from the host's keyboard state | **M3:** add a per-player `PlayerIntent` component. One local system fills it from the keyboard; gameplay systems read `PlayerIntent` instead of `ButtonInput`. Lightyear input replication fills the same component for remote players. Migrate one system at a time. |
| Movement is camera-relative | `player::camera_relative_direction` uses the local camera transform | The host has no camera for a remote player | Resolve camera-relative to world-space on the owning client, and send a world-space move vector in the intent |
| Many systems assume one player | 157 `Single<>` / `.single()` call sites; 54 `Query<..With<Player>..>` | With two players these systems will skip (`Single` fails) or apply to the wrong player | **M2/M3:** audit each one. Most become per-player iteration or "the local player" (a `LocalPlayer` marker) for HUD and camera. |
| All gameplay runs in `Update` | 0 `FixedUpdate` systems | Lightyear ticks inputs and prediction on the fixed schedule (Avian already runs fixed). Variable-rate `Update` simulation gives different results on host and client. | **M3:** move player movement, then combat, into `FixedUpdate`. This changes feel slightly, so test it offline first. |
| Loot RNG has a constant seed | `loot::drops::LootRng` starts at `0x4d59_5df4_d0f3_3173` | Deterministic, but not run-seeded | **M7:** seed it from a host-chosen run seed, as the context doc already requires (§3158) |
| The run-setup screen is local | `RunConfig` is edited live by the screen | A client could change settings the host owns | **Done in M1:** a joined client can't press Begin Run. Later, the host sends its config. |

## Systems by role

- **Host-only (authoritative) once networked:** enemy AI and navigation
  (`enemies/navigation.rs` and the shared Landmass navmesh), damage and
  defense (`combat/*`), Saint mechanics (`enemies/saint.rs`), loot and chests
  (`loot/*`), Ruin (`ruin/`), room progression (`rooms/*` exits and
  connections), dodge and invulnerability timers (`player::Dodge`).
- **Client-local presentation:** `camera/*`, `ui/*` (HUD, map, inventory
  UI, Ruin screen), `characters/animation.rs`, enemy visuals
  (`enemies/visuals.rs`), shockwave and telegraph meshes, fades.
- **Mixed (needs splitting):** weapon and ability input files, which both
  read input *and* apply gameplay.

## Conflicts with the plan

1. **Death rules.** The context doc (§7, CONFIRMED) defines **Downed → Lost
   → Returned**: a player who isn't revived is *Lost* for the rest of that
   floor and returns at the next floor if a teammate survives. The plan's
   M8 text says "if every player is downed or dead, the run ends", which
   matches the party-wipe rule but leaves out *Lost/Returned*. **Follow the
   context doc at M8.**
2. **Boss cadence.** The context backlog still mentions "boss pools for
   Levels 5/10/15". The plan and the engineering skill require a boss in
   every level. Settle this before M7.
3. **Run/level selection.** No `LevelType` or variant selector exists yet.
   The room chain is a fixed authored sequence. M7 needs that data model
   before network level selection can be built.

## Milestone 1 implemented here

See `src/net/mod.rs`. Summary:

- `multiplayer` Cargo feature (off by default). The offline build adds no
  Lightyear code.
- `--host [PORT]`, `--join ADDR[:PORT]` and `--net-debug` flags, parsed in
  `net/settings.rs` (with unit tests).
- The host runs a listen server (Netcode over UDP, 4 players max) and
  connects its own player through Lightyear's HostClient.
- The host assigns player IDs (`net/roster.rs`, with unit tests) and sends
  each player a `WelcomeMessage` on a reliable `SessionChannel`.
- The connection state is shown in a corner label, and windows are titled
  "Ruinbound (Host)" or "Ruinbound (Client)".
- The protocol version is checked through the Netcode `PROTOCOL_ID` plus
  Lightyear's own protocol-hash check.

**Not yet:** shared gameplay. A client stays on the setup screen, and the
host's run is still local. Joining after the host has started a run is not
blocked yet (it is harmless, because nothing is replicated). The Netcode
key is a development key.

## Milestone 2 implemented: replicated player entities

**Protocol version 2.** Host and client must both be rebuilt.

- **Run start follows the host.** Begin Run on the host sends
  `RunStartedMessage`, and each client enters `AppState::InGame` and builds
  the same room chain. A player who joins mid-run gets the message on
  connect.
- **Authority split.** The new `net::has_simulation_authority` run
  condition is true offline and on the host, and false on a joined client.
  It currently gates:
  - spawning the gameplay player, HUD and starting Ruin (the fallen-gate
    setup is now split into `setup_fallen_gate_scene`, which runs everywhere,
    and `setup_fallen_gate_player`, which only runs where there is authority);
  - the Begin Run button;
  - opening the Ruined Barracks rear gate.
- **Players.** The host gives its own `Player` a `NetworkPlayer` and a
  `PlayerPose` and replicates it. Each remote player gets a
  `RemotePlayerBody` beside the host (`net/player_bodies.rs`). Poses are
  sent about 30 times a second, and only when a character actually moves.
- **Avatars** (`net/avatars.rs`). Every machine draws other players with the
  shared character model. Clients smooth each avatar toward the host's
  pose.
- **Camera.** The camera now follows a `CameraTarget` marker instead of
  `Player`. Every `Player` gets the marker automatically, so offline play
  is unchanged. A client puts it on its own avatar.
- **Enemies on clients.** A client removes any enemies its rooms spawn,
  because enemies are host-owned.

**Known limitations (later milestones):**

| Limitation | Fixed in |
|---|---|
| Remote characters can't move yet | M3 |
| No collision on remote bodies | M3 |
| Avatars always play the idle animation | M3/M5 |
| Clients see no enemies | M6 |
| Clients have no HUD | M5/M8 |
| The client's Ruin clock still ticks locally (it isn't shown) | M7 |
| Every player uses the host's class choice for gameplay; avatars all share one model | M8 class selection |

## Milestone 3 implemented: input and movement

**Protocol version 3.** Host and client must both be rebuilt.

- Gameplay movement reads a per-character `player::PlayerIntent` instead
  of `ButtonInput`/the camera. The local keyboard and camera are read once
  into `player::LocalPlayerInput` (world space).
- A client sends its intent to the host (`net/intent.rs`, unreliable
  sequenced channel, ~60 Hz). The host applies it to that player's
  `RemotePlayerBody`, which now has the same capsule, collision, dodge and
  jump state as `Player`, and the shared movement systems move it.
- Only movement (walk, jump, dodge) is migrated. The other 16 files that
  read `ButtonInput` (weapons, abilities, loot, UI) are untouched and still
  only affect the local `Player`; they move to intent in M4/M5.
- Details and the acceptance test are in `MULTIPLAYER_HANDOFF_10.3.26.md`.

## Milestone 4 implemented: shared combat prototype

**Protocol version 4.** Host and client must both be rebuilt.

- Clients send attack presses as part of their intent. The host swings each
  remote player's Common Longsword with the same hit code as the local
  Longsword (`weapons::melee::intent_basic_attack`).
- Every host enemy is replicated (kind, pose, health). Clients draw a
  visual-only replica with a health bar; host despawns propagate deaths.
- Melee swing arcs are forwarded to clients as cosmetic messages.
- Details, acceptance test and limitations: `MULTIPLAYER_HANDOFF_10.3.26.md`.
