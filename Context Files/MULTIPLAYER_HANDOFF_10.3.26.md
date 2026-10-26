# Ruinbound Multiplayer: Handoff (2026-10-03)

Read this first in a new chat. It has everything needed to continue the
Lightyear co-op work. Related files in the repo root:
- `RUINBOUND_LIGHTYEAR_MULTIPLAYER_PLAN.md`: the full plan (milestones 0 to 9)
- `MULTIPLAYER_AUDIT.md`: the codebase audit plus notes on Milestones 1 to 4
- `RUINBOUND_CONTEXT_10.03.26.md`: the game's decision log (source of truth;
  Sections 95 to 100 cover the multiplayer work)

## Where we left off (end of 2026-10-03)

**Everything built so far is confirmed working by Ben.** That covers
Milestones 1 to 4, Milestone 5 steps 1 to 6, Milestone 6 steps 1 and 2, and
Milestone 8 steps 1 to 7. The last three steps (Downed -> Lost; run end,
ready system and shared run settings; and the "players lying flat" fix)
were all confirmed at the end of the day. Nothing is half-done. The current
protocol is **15**.

In short, two to four players can now: pick their own class and weapon,
share the drop pool / relics / pacts / Ruin condition, ready up, fight with
every weapon and class kit, see every enemy telegraph and ability effect,
see their own HUD, go down and revive each other, and return to run setup
together after a party wipe.

**Start the next session with Milestone 7, loot and progression for every
player** (recommended; Ben to confirm). It needs:
- drops and chests visible to every machine and pickable by any player;
- a per-player inventory and equipment (today `PlayerInventory`,
  `Equipment` changes, weapon switching and the inventory screen only work
  for the host's `Player`);
- gear defense, Legendary passives and gear procs for joined players
  (armor procs and Legendary passives still filter on `Player`);
- a synced Ruin value (a client's Ruin clock ticks locally and isn't shown).
Other candidates: level transitions with **Returned** (Section 7.3), room
progress on clients, and the Gravemind Embrace aura on clients.

Before writing code: read this file, then the source files involved
(the folder is `/Users/bencoughlin/Desktop/claude ruinbound/Ruinbound`),
back up `src/` to a new `code before multiplayer <step>/` folder, and bump
`PROTOCOL_ID` if the protocol changes. The game's decision log
(`RUINBOUND_CONTEXT_10.03.26.md`) Sections 95 to 100 summarise all of
today's multiplayer work.

## Status

| Milestone | State |
|---|---|
| 0: Repository audit | Done (`MULTIPLAYER_AUDIT.md`) |
| 1: Lightyear bootstrapping | **CONFIRMED by Ben**: compiles, runs, host and client connect |
| 2: Replicated player entities | **CONFIRMED by Ben**: compiles, runs, both windows see both players |
| 3: Input and movement | **CONFIRMED by Ben** ("seems to be working") |
| 4: Shared combat prototype | **CONFIRMED by Ben** |
| 5: Full player combat | **Done for now.** Step 1, replicated animation state (protocol 5): **CONFIRMED by Ben**. Step 2, remote player health + nearest-player enemy targeting (protocol 6): **CONFIRMED by Ben**. Step 3, each player's own weapon + camera aim (protocol 7): **CONFIRMED by Ben**. Step 4, projectiles visible on clients (protocol 8): **CONFIRMED by Ben**. Step 5, weapon abilities for remote players (protocol 9): **CONFIRMED by Ben**. Step 6, joined-player HUD (protocol 11): **runs (Ben)**. (Gear, loot and inventory for joined players move to M7.) |
| 6: Enemy presentation | **Done for now.** Step 1, telegraphs, Marksman aim line, Saint animation and health bar on clients (protocol 10): **CONFIRMED by Ben**. Step 2, players' ability effects on clients (protocol 12): **CONFIRMED by Ben** |
| 7: Loot and run progression | **In progress.** Step 1, shared loot in the world + per-player backpack/weapon slots (protocol 16): **CONFIRMED by Ben**. Step 2, inventory screen for joined players (protocol 17): **CONFIRMED by Ben**. Step 3, Legendary passives + gear procs for everyone (protocol 17 unchanged): **written 2026-10-04, not yet reported tested**. Step 4, shared Ruin on clients (protocol 18): **written 2026-10-04, awaiting Ben's test**. Remaining after this: step 3 Legendary passives + gear procs for everyone; step 4 synced Ruin + Ruin banner/screen on clients |
| 8: Co-op classes, death, polish | **In progress.** Step 1, per-player classes + Vanguard on `PlayerIntent` (protocol 13): **CONFIRMED by Ben**. Step 2, Warden: **CONFIRMED by Ben**. Step 3, Wildshaper: **CONFIRMED by Ben**. Step 4, Runesage: **CONFIRMED by Ben**. Step 5, Gravemind: **runs (Ben)**. Step 6, Downed -> Lost + party wipe (protocol 14): **CONFIRMED by Ben**. Step 7, run end + ready + shared settings (protocol 15): **CONFIRMED by Ben** (after a compile fix and the lying-flat fix). Remaining: Returned (needs level transitions), mid-run join policy |
| 9 | Not started |

## Versions and environment

- Bevy 0.19, Avian3D 0.7, **Lightyear 0.30** (resolves to 0.30.1). Lightyear
  0.30 is built on bevy_replicon internally.
- The API was verified against the Lightyear git tag `0.30.0`
  (`examples/simple_setup`, `examples/common`, `examples/simple_box`).
  Clone the repo from GitHub to read the source; it is reachable from the
  cloud container.
- **Claude can't compile.** The device shell has no cargo or network, and
  the cloud container's crates.io access is blocked. Ben runs
  `cargo fmt`, `cargo check` and `cargo run` on his Mac. Claude uses
  `rustfmt --edition 2024 --check` as a syntax check only.
- Commands:
  - `cargo run`: offline (unchanged single-player)
  - `cargo run --features multiplayer -- --host [PORT]` (default port 5888)
  - `cargo run --features multiplayer -- --join 127.0.0.1[:PORT]`
  - add `--net-debug` for connection logs
  - `cargo test --features multiplayer`: roster, argument-parsing, intent,
    aim-sanitizing and enemy-targeting unit tests
- The repo has no git commits. Backups before each milestone are in
  `code before multiplayer m1/` to `code before multiplayer m4/`, plus
  `code before multiplayer m5a/`, `m5b/`, `m5c/`, `m5d/` and `m5e/` (one backup per Milestone 5 step),
  `code before multiplayer m5f/` (before Milestone 6 step 1) and `m6b/`
  (before Milestone 5 step 6, the joined-player HUD), `m6c/` (before
  Milestone 6 step 2, shared ability effects), `m8a/` (before Milestone 8
  step 1, per-player classes), `m8b/` (before M8 step 2, Warden), `m8c/` (before M8 step 3,
  Wildshaper), `m8d/` (before M8 step 4, Runesage), `m8e/` (before M8 step 5,
  Gravemind), `m8f/` (before M8 step 6, Downed/Lost), `m8g/` (before M8
  step 7, run end + lobby).

## Architecture (as built)

- **Host-authoritative listen server.** Netcode over UDP, max 4 players,
  dev key `[0; 32]`, `PROTOCOL_ID = 15` (bump it on any protocol change).
  The host's own player connects through Lightyear's `HostClient` (the
  local player is also a `LinkOf` the server).
- **Feature-gated.** The `multiplayer` Cargo feature pulls in `lightyear`
  (std, client, server, replication, netcode, udp) and `serde`. Offline
  builds add no Lightyear code.
- **Tick rate.** `NETWORK_TICK_HZ = 64`, which matches Bevy's default fixed
  timestep (Lightyear takes over `Time<Fixed>`, and Avian runs in the fixed
  schedule).
- **Replication interval.** `REPLICATION_INTERVAL = 33ms`, set via
  `ReplicationMetadata` in `host.rs`.
- **`net::has_simulation_authority`** (always compiled) is true offline and
  on the host, false on a client. It gates:
  - `setup_fallen_gate_player` (player, HUD, starting Ruin);
  - Begin Run;
  - `ruined_barracks::open_rear_gate_when_cleared`.
- **Camera.** It follows a `camera::CameraTarget` marker.
  `Player` has `#[require(CameraTarget)]`, and a client puts the marker on
  its own avatar.

### `src/net/` file map

| File | Role |
|---|---|
| `mod.rs` | `NetworkPlugin::from_command_line()`, `has_simulation_authority`, adds the Lightyear plugins by mode |
| `settings.rs` | `NetworkMode {Offline, Host, Client}`, `NetworkSettings`, CLI parsing + tests (always compiled) |
| `status.rs` | `MultiplayerConnectionState`, `NetworkStatus` resource, corner label, window title (always compiled) |
| `protocol.rs` | Channels: `SessionChannel` (ordered reliable, both ways), `IntentChannel` (sequenced unreliable, client to host), `EffectsChannel` (unordered unreliable, host to client). Messages: `WelcomeMessage{player_id}`, `RunStartedMessage`, `PlayerLoadoutMessage{weapon}`, `PlayerIntentMessage`, `MeleeArcMessage`. Replicated: `NetworkPlayer{player_id}`, `NetworkPose{position, rotation}` (players, enemies, projectiles), `NetworkEnemy{kind}`, `CharacterMotion`, `NetworkVitals{current,max}`, `NetworkProjectile`, `NetworkVelocity`. Plain arrays, so Bevy's `serialize` feature isn't needed. `PROTOCOL_ID` history in the doc comment |
| `roster.rs` | Host `PlayerRoster`: IDs start at 1 and are never reused; `host_player_id()`, `contains()` + tests |
| `host.rs` | Starts the server and the host-client; welcome observer assigns the ID, inserts `ReplicationSender` on remote links, and sends `RunStartedMessage` if already in a run; `tell_clients_the_run_started` on `OnEnter(InGame)` |
| `player_bodies.rs` | Host: gives the host `Player` a `NetworkPlayer` + `NetworkPose` + `NetworkVitals` + `Replicate`; spawns a `RemotePlayerBody` for each remote player (2/4/6 units to the host's right) with movement, combat, `PlayerCharacter`, `Health`, `Equipment`; despawns leavers; `publish_host_poses` in `PostUpdate` for every replicated entity (only writes when it moved) |
| `client.rs` | Connects (`ReplicationReceiver` on the client entity); tracks state; `RunStartedMessage` takes the client to `InGame`; removes local `Enemy` entities in `Last`; `receive_welcome` (both modes) |
| `intent.rs` | M3/M4/M5. Client: sends `LocalPlayerInput` as `PlayerIntentMessage` (~60 Hz, immediately on a press) on `IntentChannel`: move/facing, jump/dodge/attack running press totals, camera aim. Host: `apply_remote_intents` (after `PlayerInputSet`, before `PlayerMovementSet`) writes it into the matching `RemotePlayerBody`'s `PlayerIntent`; sanitizes directions and aim (`sanitize_aim`); the first message is a press baseline; stops a body after 0.5 s without messages |
| `enemies.rs` | M4. Host: every `Enemy` gets `NetworkEnemy{kind}` + `NetworkPose` + `NetworkVitals` + `Replicate` (vitals are published by `vitals.rs`). Client: `EnemyReplica` (no `Enemy` marker, so no AI) with `attach_enemy_visual(kind)`, a `Health` from the vitals, and a health bar (not for the Saint) |
| `effects.rs` | M4. Host observer on `weapons::MeleeArcSpawned` sends `MeleeArcMessage` on the unreliable `EffectsChannel` to remote links; clients redraw it with `spawn_melee_arc` |
| `vitals.rs` | M5 step 2. Host: copies `Health` into the replicated `NetworkVitals` (renamed from `EnemyVitals`; players and enemies) in `PostUpdate` when changed. Client: mirrors it into a local `Health` (avatars get one here, enemy replicas in `enemies.rs`). Every machine: a green floating health bar (`ui::spawn_teammate_health_bar`) over every player character that isn't its own `Player` |
| `loadout.rs` | M5 step 3. Client: on entering the run, sends `PlayerLoadoutMessage { weapon }` (its own `RunConfig::weapon`) on `SessionChannel`. Host: stores it per player (`RemoteLoadouts`) and gives that `RemotePlayerBody` the weapon at Common tier (new `Weapon`, `Equipment`, reset `PlayerCombat`) |
| `projectiles.rs` | M5 step 4. Host: every `PlayerProjectile` and `HostileProjectile` gets `NetworkProjectile` (shape, size, colours read from its own mesh AABB and `StandardMaterial`), `NetworkPose` and `NetworkVelocity` (republished in `PostUpdate` when it changes, e.g. homing), and is replicated; host despawn removes it everywhere. Client: draws a sphere/box replica and extrapolates it from its last pose along its velocity (capped at 0.25 s); excluded from `avatars::smooth_toward_pose` |
| `telegraphs.rs` | M6 step 1. Host: every entity tagged `enemies::EnemyTelegraph` gets `NetworkTelegraph{kind, spawn_scale}` (replicated once) + `NetworkPose` + `Replicate` + `DisableReplicateHierarchy`; every replicated Marksman gets `NetworkEnemyAim{target: Option<NetworkPlayerId>}` (set on change from `enemies::aim_line_target`). Client: draws telegraph replicas with `enemies::attach_telegraph_visual` and animates them by age with `enemies::telegraph_scale`; draws each aim line between its own Marksman copy and the target player's avatar (`enemies::aim_line_transform`) |
| `effect_visuals.rs` | M6 step 2. Host: every entity tagged `visual_effects::SharedEffect` (not projectiles) gets `NetworkEffect{shape, colours, unlit}` (replicated once, colours read from its `StandardMaterial`) + `NetworkPose` + `NetworkScale` (republished in `PostUpdate` on change) + `Replicate` + `DisableReplicateHierarchy`. Client: builds the same primitive mesh and material, smooths toward the pose (excluded from `avatars::smooth_toward_pose`) and copies the scale |
| `lobby.rs` | M8 step 7. Shared run settings (`RunSettingsMessage`, both ways; host is the authority and rebroadcasts; client only sends after its first sync), ready (`ReadyMessage` client->host, `LobbyMessage` host->client, fills `ui::LobbyStatus`), run end (`RunEndedMessage` on the host's `OnExit(InGame)`). Marks connection links (host) and `Remote` entities (client) `PersistAcrossRuns` |
| `avatars.rs` | Every machine: an avatar is a `NetworkPlayer` without `Player`. Attaches the shared xbot model (animated from the replicated `CharacterMotion`). A client tags its own avatar with `OwnAvatar` + `CameraTarget`, and smooths players and enemies toward their `NetworkPose` (rate 15, snap beyond 5 units; projectiles are excluded). Smoothing is client-only, because on the host a body's `Transform` is the authority. |

## Milestone 3: input and movement (CONFIRMED 2026-10-03)

What changed:
- **`player::PlayerIntent`** (component, required by `Player`): world-space
  `move_direction`, `facing`, `movement_paused`, and jump/dodge *request
  counters*. Movement systems read only this.
- **`player::LocalPlayerInput`** (resource): this machine's keyboard and
  camera resolved to world space each frame by `read_local_input`
  (cursor-released blocks jump/dodge, inventory-open pauses movement, as
  before). `apply_local_input_to_player` copies it into the local `Player`.
- **System sets:** `PlayerInputSet` then `PlayerMovementSet` (chained, in
  `Update`). `update_dodge_timers`, `start_jump`, `start_dodge`,
  `character_movement` now iterate every entity with `PlayerIntent`
  instead of `single()` on `Player`.
- **`player::player_character_body()`**: shared capsule (0.45/1.0),
  kinematic body, `Player`/`World` layers, `Dodge`, `VerticalMovement`.
  Used by the Fallen Gate player spawn and by `RemotePlayerBody`.
  (`arena` and `test_grounds` still spawn the same components by hand.)
- **Remote bodies** get `PlayerIntent` + `RemoteIntentState` +
  `player_character_body()`. They are still **not** `Player`.
- **Animation** (`characters/animation.rs`) reads `Dodge` + `PlayerIntent`
  of any character instead of the keyboard, so the host sees remote
  players run/jump/dodge. Jump animation now fires on a counted jump
  request (no longer on Space while the cursor is released).
- **Void reset** applies to every `PlayerIntent` character.
- Movement is still in `Update` (FixedUpdate move deferred, offline-tested
  step later).

Ben's acceptance test:
1. `cargo run` offline: walking, turning, jump/double jump, dodge (with and
   without WASD), inventory pausing movement, dash abilities: all as before.
2. Host + client: the client walks, jumps, dodges; walls and floors stop it;
   it falls into the void and resets; both windows see it move. The host
   sees the client's character animate; the client sees avatars idle (M5).
3. `cargo test --features multiplayer` (new `net::intent` tests).

## Milestone 4: shared combat prototype (CONFIRMED 2026-10-03)

What changed:
- **Attack intent.** `PlayerIntent` gained an attack counter
  (`request_attack` / `take_attack`), `LocalPlayerInput.attack_pressed`
  (left click, only while the cursor is captured), and
  `PlayerIntentMessage.attack_presses`.
- **Remote bodies attack.** They get `PlayerCombat` + a Common Longsword.
  `weapons::melee::intent_basic_attack` (after `PlayerMovementSet`, for
  `PlayerIntent` entities that are *not* `Player`) runs the same
  `melee_attack` hit code as the local Longsword, aimed along
  `intent.facing`. Other weapon types are ignored for remote players.
  `melee_attack` now takes an aim direction instead of the camera
  transform; the local call passes the camera's forward.
- **Enemies on clients.** All enemies are replicated (not only Footmen),
  because a client otherwise sees nothing to fight. Footman, Marksman,
  Hound, Knight and Banner use their real visuals; the Saint uses its GLB
  without animation; other kinds use a placeholder capsule. Death =
  host despawn, which despawns everywhere.
- **Health bars.** `ui::update_enemy_health_bars`/`cleanup_enemy_health_bars`
  no longer filter on `Enemy` (bars name their target; cleanup checks
  `Health`), so replicas get bars.
- **Swing arcs.** `spawn_melee_arc` now triggers `MeleeArcSpawned` (also
  used by Greatsword and Hammer swings). The host forwards it to clients.
- No new hit-reaction visual: the game has none offline. Players see hits
  as the health bar dropping, knockback and stagger (via the replicated pose).

Ben's acceptance test:
1. Offline `cargo run`: Longsword, Greatsword, Hammer, bow etc. attack as
   before; enemy health bars still appear and disappear.
2. Host + client, walk to the Ruined Barracks:
   - the client sees the Footmen and Marksmen, their health bars, and their
     movement;
   - the client left-clicks near a Footman: both windows show the swing arc,
     the health bar drops by the same amount, and the enemy is knocked back;
   - the host's swings show on the client too;
   - when an enemy dies it disappears in both windows.
3. `cargo test --features multiplayer`.

## Milestone 5: full player combat (in progress)

Done one step at a time, with a two-player test after each.

**Step 1: replicated animation state** (CONFIRMED 2026-10-03)
- New `characters::animation::CharacterMotion { moving, dodging,
  jump_count }`. It is required by `Player` and given to every
  `RemotePlayerBody`.
- `update_character_motion` fills it from `PlayerIntent` + `Dodge` (only
  writing on change). It runs after `PlayerMovementSet`, chained before
  `update_character_animation`, which now reads only `CharacterMotion`.
- It is replicated, so a client's avatars (including its own) play Run,
  Jump and Dodge.
- There is still no attack animation: the xbot GLB has only Idle, Run, Jump
  and Dodge. Ben can add a clip in Blender later.
- Test: on the joining player's screen, both characters run, jump (both
  jumps of a double jump) and dodge. Offline animation is unchanged.

**Step 2: remote players' health + enemies target the nearest player**
(CONFIRMED 2026-10-03, protocol 6)
- **`player::PlayerCharacter`** marker: required by `Player`, and put on
  every `RemotePlayerBody`. It means "a character enemies can notice, chase
  and damage". `Player` still means "this machine's keyboard/HUD/gear".
- **Remote bodies** now also get `Health` + `BaseHealth` (100 + relic
  bonus, same as the host's player), `Equipment` (starter Longsword, so no
  defense yet) and `NetworkVitals`. `combat::initialize_combat_defenses`
  now runs on `Added<PlayerCharacter>`, so they get `Barriers`,
  `DeathPrevention` and `StatBuffs` too.
- **`enemies::targeting`** (new): `EnemyTarget { entity }` is required by
  `Enemy`. `select_enemy_targets` (in the enemy chain right after
  `update_enemy_activation`) picks the nearest player character by
  horizontal distance, preferring players inside the enemy's aggro area,
  and only switches when another player is 2 units closer
  (`TARGET_SWITCH_MARGIN`). Unit tests for the choice rule.
- **Activation:** an encounter activates when *any* player character enters
  its area and stays engaged while any is inside the aggro area.
- **Every enemy reads its target** instead of `single()` on `Player`:
  Footman, Hound, Marksman (and its aim telegraph), Knight (strikes and
  shield-rush knockback), Saint, and the shared navmesh targeting
  (`navigation_mesh::update_navigation_targets`). The Saint chain runs
  `.after(select_enemy_targets)`.
- **Area attacks hit everyone:** the Saint's direct impact hits every player
  in the disk, and each shockwave tracks `hit_players: Vec<Entity>` so it
  hits each grounded player once. Hostile projectiles hit whichever player
  character they reach.
- **Dodge and jump counterplay work for remote players** for free: their
  bodies already run the shared `Dodge`/`VerticalMovement` systems, and
  every enemy hit checks `Dodge::is_invulnerable()` (and the shockwave
  `VerticalMovement::grounded`) on whoever it hits.
- Offline/single-player: one player character, so it is always the target;
  behaviour should be unchanged.

Ben's acceptance test:
1. Offline `cargo run`: Footmen, Marksmen, Hounds, the Knight and the Saint
   still activate, chase, attack and damage you as before.
2. Host + client, Ruined Barracks:
   - stand the client closest to a Footman: it chases and hits the client;
     the client's green bar drops in both windows (host console prints
     "Player hit for ...");
   - walk the host closer: the Footman switches to the host;
   - client dodges through a swing: no damage ("Player dodged the attack!");
   - Marksman arrows hit whichever player is in the way.
3. Saint arena (if reachable): the slam and shockwave hit both players;
   jumping the wave avoids it for each player independently.
4. `cargo test --features multiplayer` (new `enemies::targeting` tests).

**Step 3: each joined player's own weapon, aimed with their own camera**
(CONFIRMED 2026-10-03, protocol 7)
- **Aim in the intent.** `PlayerIntent` gained `aim_origin` + `aim_rotation`
  (the player's camera) and `aim_transform()`. `LocalPlayerInput` reads
  them from the local camera; `PlayerIntentMessage` carries the client's.
  The host sanitizes them (`intent::sanitize_aim`: rotation finite and
  normalized, origin within 15 units of the body, else a fallback looking
  along `facing`).
- **One basic-attack path for everyone.** `weapons::melee::player_basic_attack`
  and `update_player_combat` now loop over every `PlayerCharacter` and use
  `intent.take_attack()` plus `intent.aim_transform()` in place of the mouse
  and the camera. All six weapon types (Longsword, Daggers, Greatsword and
  Hammer windups, Bow, Crossbow, Spellbook) work for remote players with the
  same code. `intent_basic_attack` (M4, Longsword only) is removed. The
  weapon chain now runs `.after(PlayerInputSet)`.
- **Weapon choice.** `net::loadout` (see the file map). `WeaponType` derives
  serde under the `multiplayer` feature.
- **Projectile ownership.** New `weapons::ProjectileOwner(Entity)`, set by
  `fire_bow`, `fire_crossbow` and `fire_arcane_bolt` (`spawn_bolt` and
  `spawn_arcane_bolt` now return the entity). A Normal projectile's hit
  credits its owner's `BasicAttackHitTracker` (a remote body has none, so it
  no longer credits the host's class). Projectiles without an owner
  (abilities, fragments) credit the local `Player` as before.
- **Friendly projectiles** now pass through every `PlayerCharacter`, not
  just the local `Player` (otherwise a remote player's arrow would hit its
  own body). Same for ricochet arrows.

Ben's acceptance test:
1. Offline `cargo run`: every weapon's basic attack (Longsword, Daggers,
   Greatsword, Hammer, Bow, Crossbow, Spellbook) aims and hits as before,
   including the bow aiming through the crosshair; weapon abilities and
   class abilities still work; no attack while the inventory/cursor is
   released.
2. Host + client: on the client's run-setup screen pick a different weapon
   (e.g. Bow) before the host presses Begin Run. The host log shows
   `Player 2 now wields a Common Bow`.
   - The client's clicks fire that weapon on the host, aimed where the
     client's crosshair points; enemies take damage and health bars drop in
     both windows.
   - Try Greatsword or Hammer on the client: the windup, then the swing arc
     appears in both windows.
   - The client won't see its own arrows/bolts in flight yet (only their
     hits); the host sees them.
3. `cargo test --features multiplayer` (new `sanitize_aim` tests).

**Step 4: projectiles visible on clients**
(CONFIRMED 2026-10-03, protocol 8)
- New `net/projectiles.rs` (see the file map). Generic: no per-kind table,
  so ability projectiles, Spellbook bolts, thrown daggers and Marksman
  arrows all replicate. Projectiles whose mesh is on a child entity (none
  known) fall back to a small orange sphere.
- `enemies::HostileProjectile` is now re-exported from `enemies`.

Ben's acceptance test:
1. Offline `cargo run`: unchanged (nothing in this step runs offline).
2. Host + client, Ruined Barracks:
   - the client (with a Bow, Crossbow or Spellbook) fires: its own shots fly
     from its character on its own screen, about the same size and colour as
     on the host, and vanish when they hit;
   - the host's shots are visible on the client too;
   - Gatebow Marksman arrows are visible in flight on the client;
   - Spellbook bolts curve toward their target on the client.

**Step 5: weapon abilities (right click) for remote players**
(CONFIRMED 2026-10-03, protocol 9)
- **Ability intent.** `PlayerIntent` gained an ability counter
  (`request_ability` / `take_ability`), `LocalPlayerInput.ability_pressed`
  (right click, only while the cursor is captured), and
  `PlayerIntentMessage.ability_presses` (sent immediately on a press, like
  the others). Host-side `apply_message` turns a new total into one request.
- **One ability path for everyone.** `weapons::abilities::weapon_ability_input`
  no longer reads the mouse, cursor or camera. It loops over every
  `PlayerCharacter`, consumes `intent.take_ability()` and aims with
  `intent.aim_transform()`. The cursor gate now lives in `read_local_input`
  (as for attacks). Greatsword Cleave and Hammer Ground Slam windups were
  already resolved per character by `melee::update_player_combat`.
- **In-progress abilities per character.** `daggers::update_dagger_flurry`
  (Rapid Flurry), `update_serrated_rush`, `update_shadowstep_dance` and
  `abilities::update_blade_rush` now iterate every `PlayerCharacter` that
  carries the ability component, instead of `single()` on `Player`.
- **`WeaponAbilityState`** is now given to each `RemotePlayerBody` at spawn
  and reset by `net::loadout` when the weapon changes.
- **Ability projectile ownership.** Crossbow's `spawn_bolt` and Spellbook's
  `spawn_arcane_bolt` take an `owner` and insert `ProjectileOwner` for every
  bolt (basic attack and ability). Bow's Piercing, Splinter and Ricochet
  arrows and Armorbreaker Shot also set it.
  `projectiles::move_projectiles_with_collision` now applies Final Judgment
  and Grace to the owner's `WeaponAbilityState`/`Barriers`, falling back to
  the local `Player` for projectiles without an owner.
- Not migrated (still the local `Player` only): Legendary passives
  (`update_perfect_evasion`, `widows_kiss_spreading_agony`,
  `saints_nail_mark_burst`, `ruinpiercer_cleansing_strike`) and
  `AbilityUseTracker` (remote bodies have no class kit). Remote players only
  hold Common weapons today, so none of these can trigger for them yet.
- Owner not set (no `player_entity` in scope; no owner-based effects):
  Greatsword Phantom Cleave, Hammer Seismic Strike, Widow's Kiss daggers,
  ricochet/splinter fragments.

Ben's acceptance test:
1. `cargo fmt`, `cargo check`, `cargo test --features multiplayer` (new
   `a_new_ability_total_requests_one_ability` test).
2. Offline `cargo run`: every weapon's right-click ability works as before
   (Longsword lunge, Greatsword cleave windup, Bow Piercing Shot, Hammer
   Ground Slam telegraph, Crossbow Hunter's Bolt, Spellbook Focused Cast
   only with an enemy in range, Daggers Rapid Flurry); cooldowns unchanged;
   no ability while the inventory is open / cursor released; class
   abilities unaffected.
3. Host + client (the client picks a weapon on its run-setup screen), Ruined
   Barracks. The client right-clicks:
   - Longsword: lunges forward, swing arc in both windows, enemy damaged;
   - Daggers: three quick arcs (Rapid Flurry); a dodge cancels it;
   - Bow / Crossbow / Spellbook: the ability projectile flies along the
     client's crosshair and is visible in both windows;
   - Greatsword / Hammer: after the windup the cleave arc / slam damages
     enemies around the client's character. (The Hammer slam telegraph
     disk and impact visual only show on the host; see limitations.)
   - pressing again immediately does nothing until the cooldown ends.
4. The host's own abilities still work while a client is connected.

**Step 6: a HUD for joined players; map tracks every player**
(RUNS 2026-10-03, protocol 11)
- **`ui::PlayerHudSnapshot`** (new, always compiled, replicated): barrier
  total, weapon label ("Common Bow"), weapon-ability name and cooldown in
  tenths of a second (rounded up, so it changes at most 10x/s).
  `publish_hud_snapshots` (offline/host only, `has_simulation_authority`)
  fills it for every `PlayerCharacter`, writing only on change.
- **The HUD follows `CameraTarget`** instead of `Player`: the health bar,
  barrier overlay and weapon-ability slot read the character this
  machine's camera follows. On the host/offline that is the `Player` (same
  numbers as before: barrier and cooldown are still read from the real
  `Barriers`/`PlayerCombat` when present); on a client it is its own
  avatar (mirrored `Health` + replicated snapshot).
- **Joined-player HUD** (`ui::spawn_joined_player_hud`, `OnEnter(InGame)`
  when not the simulation authority): health + barrier bar, crosshair,
  weapon indicator ("Current: Common Bow"), ability bar with the class
  slots collapsed (`Display::None`; the bar now centres its slots, which
  changes nothing when all four show). No class resource bar or Ruin banner
  (classes and Ruin aren't per-player yet).
- **Map** (`rooms::exploration::track_room_visits`): rooms are marked
  visited for every character with `CharacterMotion` (every player
  character on every machine), and `CurrentRoom` (the `[YOU]` marker)
  follows `CameraTarget`. This also fixes the host's `[YOU]` marker, which
  could follow a joined player's body.

Ben's acceptance test:
1. `cargo fmt`, `cargo check`, `cargo test --features multiplayer`.
2. Offline `cargo run`: HUD unchanged: health bar, barrier strip (e.g.
   Runebound Robes), crosshair, weapon indicator with slots/inventory,
   class ability slots and RMB slot with cooldowns, Runesage resource pips,
   Ruin banner. Map `[YOU]` and visited rooms as before.
3. Host + client:
   - the client now has a crosshair, a red health bar (top left) that drops
     when its character is hit, "Current: Common <weapon>" bottom left, and
     a single centred RMB slot that counts down after right-clicking;
   - the host's HUD is unchanged;
   - each player's map (M): `[YOU]` is in the room that player stands in,
     and rooms either player entered show as visited.

## Milestone 6: enemy presentation on clients (in progress)

**Step 1: telegraphs, aim line, Saint animation and health bar**
(CONFIRMED 2026-10-03, protocol 10)
- **`enemies::telegraph`** (new): `EnemyTelegraph { kind }` tags every
  telegraph entity; `EnemyTelegraphKind` = `SaintSlamDisk`,
  `SaintShockwave`, `KnightRushLane`, `KnightBannerDisk`.
  `attach_telegraph_visual(kind)` and `telegraph_scale(kind, seconds,
  spawn_scale)` dispatch to per-archetype functions.
- **Shared visuals.** The host's spawn and update code now uses the same
  functions a client does, so they can't drift: `saint::slam_telegraph_visual`,
  `slam_telegraph_scale`, `attach_shockwave_visual`, `shockwave_scale`,
  `shockwave_radius_after`; `knight::rush_telegraph_visual`,
  `banner_telegraph_visual`, `banner_telegraph_scale`;
  `ranged::aim_line_visual`, `aim_line_transform`, `aim_line_target`
  (+ `AIM_ORIGIN_HEIGHT`/`AIM_TARGET_HEIGHT` constants). Behaviour and
  numbers are unchanged offline.
- **Saint animation.** New `saint::SaintAnimationCue(SaintAnimationState)`
  on the boss root. `publish_saint_animation_cue` copies
  `SaintBossCombat::animation` into it (on change), and
  `update_saint_animation` now reads only the cue. It is replicated, and a
  client's Saint copy now gets the animated model
  (`attach_saint_animated_visual`; `attach_saint_static_visual` removed).
- **Saint health bar on clients:** the replica gets the same floating enemy
  bar the host's Saint has (there is no separate boss bar in the game).
- **Timing.** Client telegraphs start when they arrive (about one round
  trip + 33 ms after the host), and the host's despawn ends them. The
  shockwave grows locally at the host's speed from arrival, so on a client
  its visible edge trails the host's damage edge by that same delay.

Ben's acceptance test:
1. `cargo fmt`, `cargo check`, `cargo test --features multiplayer`.
2. Offline `cargo run`: Saint slam disk pulses, the shockwave expands and
   damages grounded players at the edge, jumping it avoids it, and the
   Saint's Idle/Walk/Windup/Impact/Recovery clips play as before. Marksman
   aim lines track you. If reachable, the Knight's rush lane and banner disk
   look as before.
3. Host + client:
   - Ruined Barracks: on the client, a Marksman winding up shows the red
     line to whichever player it targets (client or host), following them,
     and it disappears when the arrow fires or the Marksman dies.
   - Saint arena: on the client, the Saint walks and plays its slam clips,
     has a health bar, the red disk pulses during the windup, and the
     three-ring shockwave expands; jumping it on the client avoids damage
     (the host decides).
   - Knight (if reachable): rush lane and banner disk visible on the client.

**Step 2: players' ability effects on clients**
(CONFIRMED 2026-10-03, protocol 12)
- **`visual_effects`** (new top-level module, always compiled):
  `EffectShape` (`Sphere`, `Cylinder`, `Torus`, `Capsule`, same parameters
  as the Bevy primitives), `SharedEffect { shape }`, and
  `effect_mesh(meshes, shape)`, which spawn sites use instead of
  `Mesh3d(meshes.add(<primitive>))`. Offline it builds the identical mesh.
- **Tagged effects:** Hammer: impact burst (Ground Slam and every
  Earthshatter pulse), Ground Slam telegraph, unstable ground. Wildshaper:
  Bear Slam telegraph and impact, Rhino Charge crash. Gravemind: Consume
  pulse. Runesage: Runic Surge emergence ring, Grand Rune burst. Warden:
  Guardian Otter, Shadow Stag emergence ring, Predator's Spirit strike.
  Vanguard: Afterimage clone. Daggers: shadow copy (Dancer in the Dark).
  (Seismic Strike, Phantom Cleave and other projectiles were already
  visible via `net::projectiles`.)
- Not tagged: Gravemind's Embrace the Ruin aura (a child of the caster) and
  the swing-arc torus (already sent as `MeleeArcMessage`).
- Class abilities are still only used by the host's own character, so in
  practice clients now see the host's class effects; joined players get
  classes in Milestone 8.

Ben's acceptance test:
1. `cargo fmt`, `cargo check`, `cargo test --features multiplayer`.
2. Offline `cargo run`: Hammer Ground Slam disk + burst, Earthshatter's
   three bursts and unstable ground, and the class effects above look and
   time exactly as before.
3. Host + client:
   - a client with a Hammer right-clicks: both windows show the pulsing
     disk, then the burst;
   - the host (e.g. Wildshaper or Warden) uses class abilities: the client
     sees the Bear Slam disk and burst, the Guardian Otter circling its
     target, the Grand Rune burst, etc.;
   - each effect disappears in both windows at about the same time.

## Milestone 8: co-op classes, death and polish (in progress)

**Step 1: each player's own class; Vanguard works for joined players**
(CONFIRMED 2026-10-03, protocol 13)
- **Class choice.** `characters::ClassId` derives serde (multiplayer).
  `PlayerLoadoutMessage` now carries `class` (the client's
  `RunConfig::class`). `net::loadout` gives the body that class once with
  `characters::insert_class` (marked `ClassGiven`).
- **Class-ability intent.** `PlayerIntent` has per-slot class-ability
  counters (`request_class_ability(slot)` / `take_class_ability(slot)`;
  slot 0/1/2 = Q/E/R, `player::CLASS_ABILITY_KEYS`).
  `LocalPlayerInput.class_ability_pressed` (Q/E/R, only with the cursor
  captured and the inventory closed); `PlayerIntentMessage.class_ability_presses`.
- **Shared class state for every player character.**
  `class_abilities::initialize_character_combat_state` now runs on
  `Added<PlayerCharacter>`, so joined bodies get `BasicAttackTracker`,
  `BasicAttackHitTracker`, `CharacterCombatModifiers`, `AbilityUseTracker`.
  `projectiles::move_projectiles_with_collision` credits basic-attack
  projectile hits to the owner's tracker (any `PlayerCharacter`), falling
  back to the local `Player`.
- **Vanguard migrated:** init, timers and HUD/modifier sync run for every
  `PlayerCharacter` with `Vanguard`; Lunge (Q) and Afterimage (R) read
  `intent.take_class_ability(..)` and aim with `intent.aim_transform()`; an
  Afterimage clone aims its echo shots with its owner's camera. No
  keyboard or camera reads left in `vanguard_abilities.rs`.
- **Other classes are not migrated yet.** A joined Warden, Wildshaper,
  Runesage or Gravemind gets the class marker but no kit (their systems
  still filter on `Player` and read the keyboard), so their class slots are
  hidden; the host's own class is unaffected.
- **HUD.** `PlayerHudSnapshot` gained `class_slots` (label + ready, built
  with the HUD's own `class_ability_label`) and `class_resource`. A joined
  player's ability bar shows their class slots when they exist, and their
  HUD now includes the class resource bar.

Ben's acceptance test:
1. `cargo fmt`, `cargo check`, `cargo test --features multiplayer` (new
   `class_ability_presses_request_their_own_slot` test).
2. Offline `cargo run` as Vanguard: Lunge (Q) toward an enemy in front,
   Momentum stacks from hits (faster attacks/movement), Afterimage (R)
   spawns a clone that echoes basic attacks at the crosshair target, R again
   returns to it; no casting with the inventory open. Other classes
   unchanged.
3. Host + client, the client picks **Vanguard** on its setup screen (host
   any class). Host log: `Player 2 is a Vanguard`.
   - The client's HUD shows `[Q] LUNGE`, `[PASSIVE] MOMENTUM 0 / 5`,
     `[R] AFTERIMAGE`; Q lunges at the enemy the client aims at; hits build
     Momentum (counter rises, the client attacks/moves faster); R leaves a
     clone (visible in both windows) that echoes the client's attacks at
     the client's crosshair target; R again teleports back.
   - The host's own class still works.
4. A client picking another class: no class slots on its HUD, nothing
   breaks.

**Step 2: Warden for joined players**
(CONFIRMED 2026-10-03, protocol 13 unchanged)
- Only `characters/warden_abilities.rs` changed. Every system now runs for
  every `PlayerCharacter` with `Warden`; Guardian Otter (Q, slot 0), Shadow
  Stag (E, slot 1) and Predator's Spirit (R, slot 2) read
  `intent.take_class_ability(..)`; Otter targeting, Shadow Stag direction
  and Predator's Spirit strikes aim with `intent.aim_transform()`.
- **Guardian Otter can now bless another player:** its crosshair raycast
  and its target lookup accept any `PlayerCharacter` (host's player or a
  joined body), and the shield/buff go onto that player's `Barriers` /
  `StatBuffs` (every player character has them). The otter visual orbits
  any player character.

Ben's acceptance test:
1. `cargo fmt`, `cargo check`, `cargo test --features multiplayer`.
2. Offline `cargo run` as Warden: Q blesses yourself (blue barrier strip,
   faster movement/damage, otter orbits you), E sends the stag where the
   camera faces, R activates Predator's Spirit (extra strike bursts on hits
   at the crosshair target). Inventory open blocks casting.
3. Host + client, client picks **Warden**: its HUD shows the three Warden
   slots; Q/E/R work from the client as above, aimed with the client's
   camera.
4. Co-op blessing: either Warden puts the crosshair on the other player
   and presses Q: the *other* player gets the barrier strip on their HUD
   and the otter orbits them in both windows. With the crosshair elsewhere,
   Q blesses the caster.

**Step 3: Wildshaper for joined players**
(CONFIRMED 2026-10-03, protocol 13 unchanged)
- Only `characters/wildshaper_abilities.rs` changed. Rhino Charge (Q,
  slot 0), Bear Slam (E, slot 1) and Panther Pounce (R, slot 2) read
  `intent.take_class_ability(..)`; Rhino Charge's direction and Panther
  Pounce's target cone aim with `intent.aim_transform()`. The in-progress
  systems (`update_rhino_charge`, `update_bear_slam_impact`,
  `update_panther_pounce`), init, timers and HUD sync loop over every
  `PlayerCharacter` with `Wildshaper`. Enemy queries now exclude
  `PlayerCharacter` (was `Player`).
- Small behaviour change: Wildshaper abilities used to ignore the
  inventory-open check (only the cursor); they now share the common gate
  in `player::read_local_input` (cursor captured *and* inventory closed),
  like every other class.

Ben's acceptance test:
1. `cargo fmt`, `cargo check`, `cargo test --features multiplayer`.
2. Offline `cargo run` as Wildshaper: Q charges forward (hits enemies,
   crash burst at walls), E shows the Bear Slam disk then slams, R pounces
   at an enemy in front of the camera.
3. Host + client, client picks **Wildshaper**: its HUD shows the three
   slots, and Q/E/R work from the client, aimed with the client's camera;
   the Bear Slam disk and bursts are visible in both windows.

**Step 4: Runesage for joined players**
(CONFIRMED 2026-10-03, protocol 13 unchanged)
- Only `characters/runesage_abilities.rs` changed. Rune Blast (Q, slot 0),
  Runic Surge (E, slot 1) and Grand Rune (R, slot 2) read
  `intent.take_class_ability(..)` and aim with `intent.aim_transform()`
  (Rune Blast's crosshair raycast, Runic Surge's direction, Grand Rune's
  crosshair target). Rune gain, init and HUD sync loop over every
  `PlayerCharacter` with `Runesage`, so a joined Runesage earns Runes from
  their own hits; their HUD shows the RUNES pips via `PlayerHudSnapshot`.
  Enemy queries now exclude `PlayerCharacter` (was `Player`).

Ben's acceptance test:
1. `cargo fmt`, `cargo check`, `cargo test --features multiplayer`.
2. Offline `cargo run` as Runesage: hits build Runes (pips), Q fires a Rune
   Blast at the crosshair, E sends the Runic Surge wave, R detonates a Grand
   Rune on the enemy under the crosshair; each spends Runes.
3. Host + client, client picks **Runesage**: the client's HUD shows the
   three slots and the RUNES pips filling from its own hits; Q/E/R work
   from the client, aimed with its camera.

**Step 5: Gravemind for joined players**
(RUNS 2026-10-03, protocol 13 unchanged)
- Only `characters/gravemind_abilities.rs` changed. Ruin Bolt (Q, slot 0),
  Consume (E, slot 1) and Embrace the Ruin (R, slot 2, press again to
  release) read `intent.take_class_ability(..)`; Ruin Bolt aims with
  `intent.aim_transform()`. Embrace's drain/meter, init, timers and HUD
  sync loop over every `PlayerCharacter` with `Gravemind`. Enemy queries
  exclude `PlayerCharacter`.
- **Ruin is shared:** a joined Gravemind's Embrace the Ruin adds to the
  run's Ruin on the host (`RuinState::add_ruin`), the same run-wide Ruin the
  host's own abilities use. (A client's own Ruin display isn't synced yet,
  M7.)
- The Embrace aura is a child of the caster and is still drawn only on the
  host (known limitation).
- With this, **every class works for joined players.** No class file reads
  the keyboard or camera any more; all go through `PlayerIntent`.

Ben's acceptance test:
1. `cargo fmt`, `cargo check`, `cargo test --features multiplayer`.
2. Offline `cargo run` as Gravemind: Q fires a Ruin Bolt at the crosshair,
   E drains nearby enemies and heals you (pulse visible), R starts Embrace
   the Ruin (aura, health drain, stacking power, adds Ruin), R again
   releases it.
3. Host + client, client picks **Gravemind**: HUD shows the three slots
   (and its resource readout); Q/E/R work from the client; Consume heals the
   client's health bar; the host's Ruin rises while the client embraces.

**Step 6: Downed -> Lost, and party wipe** (context doc section 7)
(CONFIRMED 2026-10-03, protocol 14)
- **`player::life`** (new): `LifeState { Alive, Downed { bleed_out_remaining,
  revive_progress }, Lost }`, required by `PlayerCharacter` and replicated.
  Tuning: `DOWNED_BLEED_OUT_SECONDS` 25, `REVIVE_SECONDS` 3,
  `REVIVE_RADIUS` 2.2, `REVIVE_HEALTH_FRACTION` 0.3.
- **`update_life_states`** (simulation authority only): Alive at 0 health
  -> Downed; any *Alive* player within the radius builds revive progress
  (it decays when they step away; no button); full progress -> Alive at 30%
  health; bleed-out expires -> Lost.
- **Incapacitated:** `PlayerIntent.incapacitated` (set from `LifeState`
  in `PlayerInputSet`) makes every `request_*`/`take_*` a no-op and stops
  walking (`character_movement`, `is_moving`), so a downed/lost character
  can't move, jump, dodge, attack or cast, for every weapon and class.
- **Enemies ignore fallen players:** `enemies::targeting` only picks Alive
  players, and encounters only activate/stay engaged for Alive players.
  (Area attacks can still touch a downed body; health is already 0.)
- **Presentation (every machine):** a Downed player's model lies flat
  (`present_life_states` rotates each model's root node: entities with
  `CharacterAnimationOwner` **and `WorldAssetRoot`**; the GLB's
  animation-player node also carries `CharacterAnimationOwner` and must
  not be rotated: fixed 2026-10-03 after everyone spawned lying flat;
  `CharacterAnimationOwner::owner()` added); a Lost player is hidden.
- **HUD banner** (`ui::LifeStatusBanner`, both HUDs): "DOWNED / A teammate
  can revive you (Ns)", "Being revived... N%", "PLAYER LOST TO THE RUINS",
  teammate prompts ("A teammate is DOWNED - stand beside them to revive",
  "Reviving teammate... N%"), and "THE EXPEDITION HAS FALLEN" when nobody
  is Alive. Unit tests: `ui::life_status_tests`.
- **Not yet:** *Returned* (needs level transitions); a wipe doesn't end
  the run (no run-end flow yet: relaunch to play again); no revive
  animation or sound; a lost player's camera just stays on their hidden
  body.

Ben's acceptance test:
1. `cargo fmt`, `cargo check`, `cargo test --features multiplayer`.
2. Offline: let enemies bring you to 0: you can't move/act, the model lies
   down, enemies leave you, and the banner says THE EXPEDITION HAS FALLEN
   (solo = party wipe). After 25 s the model disappears (Lost).
3. Host + client:
   - get the client downed: both windows show it lying down; the client's
     banner shows DOWNED with a countdown; the host sees "A teammate is
     DOWNED"; enemies switch to the host;
   - the host stands next to the client for 3 s: "Reviving teammate..."
     climbs to 100%, the client stands up with 30% health and can act;
   - let a downed player bleed out: they vanish in both windows and see
     PLAYER LOST TO THE RUINS;
   - both players down: THE EXPEDITION HAS FALLEN on both screens.

**Step 7: run end, ready system, shared run settings**
(CONFIRMED 2026-10-03, protocol 15)
- Fixes during testing: `game_state::reset_resource` needs
  `R: Resource<Mutability = Mutable> + Default` (Bevy 0.19 `ResMut`
  bound); and the downed pose had laid every player flat (see Step 6).
- **Run end.** `player::life::end_run_after_party_wipe` (simulation
  authority): nobody Alive for `PARTY_WIPE_RETURN_SECONDS` (6 s) ->
  `AppState::RunSetup`. The banner reads "THE EXPEDITION HAS FALLEN /
  Returning to run setup...". The host sends `RunEndedMessage`; clients
  return too.
- **Run teardown** (`game_state::RunLifecyclePlugin`, new): records every
  entity at `OnExit(RunSetup)`; at `OnExit(InGame)` despawns every
  top-level entity created since (with children), except `Observer`s,
  registered systems and `PersistAcrossRuns`. Releases the cursor on
  `OnEnter(RunSetup)`. Run-wide resources reset by their owners with the
  generic `game_state::reset_resource::<R>` on `OnExit(InGame)`:
  `RuinState`, `LootDropRequests`, `WorldNavmesh` (navmesh regenerates),
  `EnemyNavigationGrid`, `RoomGraph`, `CurrentRoom`, `MapUiState`,
  `RuinScreenState`, `InventoryUiState`, `LootFeedback`, `DraggedItem`,
  `RunModifiers`, camera `OcclusionFades` (+ `fade_target` cleared). Map,
  Ruin and inventory screens are hidden.
- **Ready.** `ui::LobbyStatus { role: Solo|Host|Guest, local_ready, guests,
  ready_guests }` (always compiled). Guests' Begin Run button reads
  READY/UNREADY and sends a toggle; the host's Begin Run is blocked until
  every guest is ready ("Players ready: N/M"). Readiness clears when a run
  starts/ends and whenever the shared settings change.
- **Shared settings.** `run_setup::SharedRunSettings` (drop pool, relics,
  pacts, Ruin condition) with `RunConfig::shared_settings` /
  `apply_shared_settings`; catalog ids and `GearArchetype` derive serde.
  Any player's change reaches everyone through the host. Class and weapon
  stay per player. A client computes `RunModifiers` from the shared
  relics/pacts when the run starts.
- Unit test: `run_setup::tests` (settings round trip).

Ben's acceptance test:
1. `cargo fmt`, `cargo check`, `cargo test --features multiplayer`.
2. Offline: die -> banner -> after ~6 s the run-setup screen returns with
   your previous choices; Begin Run starts a clean new run (rooms, enemies
   at full health, HUD once, Ruin back to the start value, map empty, no
   leftover loot or effects). Repeat a few times; watch the log for
   warnings or panics.
3. Host + client on the setup screen:
   - the client's button says READY; the host sees "Players ready: 0/1"
     and can't begin; the client presses READY -> "1/1", the host begins;
   - either player toggles a relic / pact / drop-pool category / Ruin
     condition: the other screen updates, and the client is un-readied;
   - class and weapon stay each player's own.
4. Host + client, both go down: both return to run setup ~6 s later, still
   connected; ready up and start again; the client's character, enemies
   and HUD appear normally.

**Next steps (proposed order):** Milestone 7 (loot and progression for
every player); level transitions with Returned; room progress on clients. on clients (the
Barracks rear gate is an invisible collider, so only matters for camera
collision; low priority); per-player classes and class abilities (M8);
gear/barriers/DoT from players' own items (M7).

## Known limitations (current, end of 2026-10-03)

| Limitation | Planned fix |
|---|---|
| ~~Legendary weapon passives and gear procs only run for the host's `Player`~~ | fixed in M7 step 3 (awaiting test) |
| No *Returned* (needs level transitions) | with level transitions |
| Melee enemies only hit their current target, even if another player stands in the swing (area attacks and projectiles hit anyone) | later, if playtests want it |
| ~~Remote players have no gear defense~~ | fixed in M7 step 2 |
| Clients see telegraphs ~one round trip late (no prediction); the Saint's shockwave edge on a client trails the host's damage edge by that delay | M9 (or a time offset) |
| Footman, Hound, Knight and Marksman models have no animation clips anywhere (offline included); only the Saint is animated | when clips exist |
| Gravemind's Embrace the Ruin aura (child of the caster) is host-only | if it matters in play |
| `ProjectileOwner` is not set on Phantom Cleave, Seismic Strike, Widow's Kiss daggers or split fragments (they have no owner-based effects) | if one gains such an effect |
| ~~Loot drops, chests, inventory and weapon switching only work for the host's `Player`~~ | fixed in M7 steps 1-2 |
| No client-side prediction: a client's own movement and swings lag by about the round-trip time plus 33 ms | M9 |
| Remote bodies don't collide with other players (same as `Player`) | by design for now |
| ~~A joined player's HUD has no Ruin banner or inventory~~ | fixed in M7 steps 2 and 4 (awaiting test) |
| ~~A joined Gravemind's Embrace adds to the host's run Ruin (shared Ruin is intended; the client just can't see it yet)~~ | fixed in M7 steps 2 and 4 (awaiting test) |
| ~~A client's Ruin clock ticks locally (it isn't shown)~~ | fixed in M7 steps 2 and 4 (awaiting test) |
| Joining mid-run isn't blocked | M8 decision |
| Movement and combat still run in `Update` | separate offline-tested step |

## Open design decisions (flagged, not decided)

- **Death rules.** Settled: Downed -> Lost implemented per context doc §7
  (2026-10-03); Returned waits for level transitions.
- **Joining mid-run.** Not blocked; a late joiner gets a body beside the
  host. Decide whether to block it or let them wait in the lobby.
- **Boss cadence.** Settled by the context doc (2026-09-09 session summary,
  CONFIRMED): every level has a boss and the every-five-levels cadence is
  SUPERSEDED. Older backlog lines that still say "Levels 5/10/15" are stale.
- **Level selection.** The design is CONFIRMED (level types with two authored
  variants each, a fixed final level), but no `LevelType`/variant data model
  exists in code yet (the room chain is fixed). It's needed before M7.

## Milestone 7: loot and progression for every player (in progress)

**Step 1: shared loot in the world** (CONFIRMED 2026-10-04, protocol 16,
backup `code before multiplayer m7a/`)
- **Intent:** `PlayerIntent` gained an interact counter (`request_interact`,
  `take_interact(InteractionKind::{OpenChest, PickUp})`, one [F] press is
  seen once by each kind) and weapon-slot counters (`request_weapon_slot`,
  `take_weapon_slot`, keys `player::WEAPON_SLOT_KEYS`). `LocalPlayerInput`
  has `interact_pressed` / `weapon_slot_pressed` (cursor captured);
  `PlayerIntentMessage` has `interact_presses` / `weapon_slot_presses`.
- **Every player character:** `weapons::switching`, `weapons::loot::pickup_nearby_weapon`
  and `loot::chests::open_nearby_chest` loop over every `PlayerCharacter`
  (two players can't take the same item in one frame). Chest opening and
  loot rolls run only with simulation authority. `initialize_player_inventory`
  gives every `PlayerCharacter` a backpack; `update_armor_health_bonus`
  runs for every `PlayerCharacter`.
- **`weapons::WorldLoot`** (new, always compiled, replicated): on every
  pickup and chest (spawn helpers add it); mesh/material/rotation helpers
  shared by host and client. Prompts ("[F] Pick up X", "[F] Open Chest")
  read `WorldLoot` near the `CameraTarget` character on every machine.
- **`weapons::PlayerInventorySnapshot`** (new, `weapons/snapshot.rs`,
  replicated): every player's weapon slots, armor, backpack (label +
  tooltip text) and defense totals, published by the authority on change.
  The client's weapon indicator and "Inventory full" prompt read it; the
  inventory screen will (step 2).
- **`LootFeedbackShown`** event: the local `Player`'s messages show
  locally; a joined player's are sent as `LootFeedbackMessage`.
- **`net/loot.rs`** (new): host gives each `WorldLoot` a world-space
  `NetworkPlacement` (replicate_once) and replicates it; client removes its
  rooms' own loot (`WorldLoot` without `NetworkPlacement`, in `Last`) and
  draws the host's copies (bobbing locally).
- **`net::loadout`** now gives the chosen starting weapon once
  (`StartingWeaponGiven`) into weapon slot 1 only, so it no longer resets a
  joined player's weapons.

**Step 2: inventory screen for joined players** (written 2026-10-04,
protocol 17, backup `code before multiplayer m7b/`; CONFIRMED by Ben 2026-10-04)
- The inventory screen (`weapons::inventory_ui`) now reads only the
  `PlayerInventorySnapshot` (+ `Health`) of the `CameraTarget` character, on
  every machine, offline included.
- A drag produces `InventoryMove { from: ItemLocation, to: InventoryTarget::{Slot, Drop} }`
  and triggers `InventoryMoveRequested`. Offline/host: queued on the local
  `Player`'s `InventoryRequests` (`queue_own_inventory_moves`). Client:
  sent as `InventoryMoveMessage` (`net::loot`), queued on that player's
  body (max 16 per frame).
- `apply_inventory_requests` (authority, in the inventory chain, followed by
  `publish_inventory_snapshots` so the screen updates the same frame)
  applies every player's moves with the old drag rules via
  `apply_inventory_move`, which re-checks slot indices and fits. Drops land
  in front of the owner and replicate as loot. Unit tests in
  `inventory_ui::tests`.
- Equipping armor on a joined player raises their max health
  (`update_armor_health_bonus`) and defense (enemy damage already reads the
  target's `Equipment`).

**Step 3: Legendary passives and gear procs for every player** (written
2026-10-04, protocol 17 unchanged, backup `code before multiplayer m7c/`,
NOT yet compiled or tested)
- `weapons::armor_procs` (Runebound Robes, Robes of the Hollow Star, Crown
  of the Unbroken) runs for every `PlayerCharacter`, each with their own
  `Equipment`, `AbilityUseTracker`, `Barriers`, `DeathPrevention` and
  `ArmorProcState`.
- `daggers::update_perfect_evasion` (Dancer in the Dark) loops every
  `PlayerCharacter` with its own `WeaponAbilityState`.
- `daggers::widows_kiss_spreading_agony` is active while *any* player
  character wields Widow's Kiss (DoTs don't record who applied them).
- `crossbow::RuinpiercerMark` now carries `owner` (the bolt's
  `ProjectileOwner`); `ruinpiercer_cleansing_strike` uses that owner's
  cooldown (fallback: local `Player`) and lowers the shared run Ruin.
- Saint's Nail's holy burst already had no player filter. Other
  Legendaries' passives live in the per-character attack/ability paths
  migrated in M5.
- Still `Player`-only by design: class model attach systems
  (`characters::*::attach_*_visual`), since joined players use the shared
  avatar model.

**Step 4: shared Ruin on clients** (written 2026-10-04, protocol 18,
backup `code before multiplayer m7d/`, NOT yet compiled or tested)
- `ruin::RuinSnapshot` (new, always compiled, replicated): Ruin in tenths,
  run clock in whole seconds, warning on/off. `RuinState::snapshot()` /
  `apply_snapshot()`; unit test `ruin::tests`.
- `ruin::advance_run_clock_and_ruin` now runs only with simulation
  authority.
- `net/ruin.rs` (new): host keeps one replicated "Shared Ruin" entity
  (spawned on demand during `InGame`, removed by the run teardown) and
  rewrites its snapshot on change; client copies changed snapshots into
  `RuinState`, so the Tab screen, run timer and warning banner match the
  host.
- The joined-player HUD now includes the Ruin warning banner.
- Still host-only: the Gravemind Embrace aura visual (child of the caster).
