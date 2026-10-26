# Ruinbound Multiplayer Implementation Plan

## Purpose

Convert Ruinbound from a single-player Rust + Bevy dungeon crawler into a cooperative multiplayer game using **Lightyear**.

This document is an implementation specification for Claude Code. It describes the target architecture, authority rules, data that must be synchronized, the order of implementation, and the acceptance tests for each milestone.

Claude must inspect the existing repository before editing. Do not assume the current Bevy version, module names, entity components, asset loading flow, room-loading flow, or Lightyear API version. Select a Lightyear release compatible with the Bevy version already used by the project and pin compatible dependency versions in `Cargo.toml`.

The first target is cooperative multiplayer for **2–4 players** using a **host-authoritative listen server**:

- One player hosts the run and also plays as a client.
- The host process runs the authoritative simulation.
- Other players connect as clients.
- Clients send input and gameplay requests.
- The host validates actions and owns combat, enemies, loot, progression, and run state.
- Clients receive replicated state and render their local presentation.

Do not begin with a fully distributed peer-to-peer simulation. Every player independently simulating enemies and combat would cause state divergence, duplicated loot, inconsistent boss phases, and cheating problems.

## Existing Ruinbound design constraints

Preserve the established game direction while adding multiplayer:

- Ruinbound is a cooperative dungeon crawler, not a competitive PvP game.
- The dungeon uses authored level types and authored level variants, not runtime assembly of random rooms.
- Each ordinary level has two authored variants.
- The next level is selected by choosing a level type and then one of that type's variants.
- Every level has a boss encounter.
- The final level is fixed by default.
- Future expansions may register additional final bosses that can be selected when unlocked.
- The run must have a deterministic seed or explicit recorded selection state so the host and clients load the same content and runs can be reproduced during debugging.
- Existing combat behavior must remain intact in offline/single-player mode.
- Debug logging must remain behind the existing disabled debug setting or a new explicit network debug setting.

## Core architecture

Separate the game into three conceptual layers:

### 1. Authoritative simulation

The server/host owns the actual game rules:

- Player movement validation
- Player health and death
- Damage and healing
- Dodge invulnerability windows
- Frontal guard and blocking
- Abilities and cooldowns
- Weapon switching
- Projectile creation and collision
- Enemy AI and navigation
- Enemy health and status effects
- Boss phases
- Loot generation
- Chest state
- Revives
- Room completion
- Level selection
- Run seed and run progression

### 2. Network transport and replication

Lightyear owns the connection, transport, input delivery, replication, interpolation, and prediction mechanisms selected for the project.

Create a dedicated network area, using names that fit the existing project structure. A likely organization is:

```text
src/net/
    mod.rs
    protocol.rs
    client.rs
    server.rs
    connections.rs
    network_ids.rs
    replication.rs
    input.rs
```

Do not duplicate existing modules. If the project already has networking or game-state modules, extend them instead of creating competing systems.

### 3. Local presentation

Clients own local-only presentation:

- Camera behavior
- UI
- Animation playback
- Particles
- Audio
- Screen effects
- Local input collection
- Cosmetic feedback

Presentation must react to replicated simulation state. It must not independently decide gameplay outcomes.

## Lightyear integration requirements

1. Inspect `Cargo.toml` and determine the exact Bevy version.
2. Select the Lightyear release compatible with that Bevy version.
3. Read the selected Lightyear examples/documentation for the exact API version before writing integration code.
4. Add only the Lightyear features and transports currently needed.
5. Keep networking behind a project feature if practical, for example:

```toml
[features]
default = []
multiplayer = []
```

Do not require a separate dedicated server binary during the first milestone unless the existing project structure already supports separate binaries.

The first implementation should support three runtime modes:

```rust
enum NetworkMode {
    Offline,
    Host,
    Client,
}
```

Offline mode must preserve the current single-player behavior. Host mode must start the authoritative simulation and a local client. Client mode must connect to a host and wait for the replicated world.

Use Lightyear's current concepts for:

- Server and client plugins
- Protocol registration
- Replicated entities/components
- Input synchronization
- Interpolation
- Local prediction where appropriate
- Reliable and unreliable channels/streams

Do not copy an old Lightyear example if its API does not match the selected version.

## Network authority rules

The following rules are mandatory.

| State or action | Authority |
|---|---|
| Keyboard/controller input | Owning client |
| Player attack request | Owning client sends request; host validates |
| Player movement result | Host |
| Player position correction | Host |
| Player health | Host |
| Dodge invulnerability | Host |
| Ability cooldown | Host |
| Weapon legality and cooldown | Host |
| Hit detection | Host |
| Damage | Host |
| Enemy AI | Host |
| Enemy navigation target | Host |
| Enemy position | Host |
| Enemy health/status effects | Host |
| Boss phase | Host |
| Loot generation | Host |
| Chest opened state | Host |
| Item pickup result | Host |
| Room completion | Host |
| Level selection | Host |
| Run seed | Host |
| Revive result | Host |
| Camera | Local client |
| Animation presentation | Local client based on state |
| Particles/audio/UI | Local client based on events/state |

Never allow a client to send a final damage amount, enemy death command, loot result, boss phase, or level transition as an authority claim.

## Network identity and player ownership

Every connected player needs a stable network identity. Do not rely on a local Bevy `Entity` value as a player identity across machines.

Add or adapt a network-owned player component/resource containing at least:

```rust
struct NetworkPlayer {
    player_id: u64,
    connection_id: u64,
    is_local: bool,
}
```

The exact fields may be adjusted to the Lightyear version and existing codebase.

Track:

- Connection ID
- Player ID
- Character/class selection
- Ready state
- Spawn point
- Alive/downed/dead state
- Current level presence
- Disconnect state

The host must assign player IDs. Clients must not choose or overwrite their authoritative identity.

## Input protocol

Create a compact input command that can be sent from the owning client to the host. It should contain input intent rather than the result of an action.

Conceptually:

```rust
struct PlayerInputCommand {
    tick: u32,
    movement: Vec2,
    aim_direction: Vec2,
    attack_pressed: bool,
    dodge_pressed: bool,
    interact_pressed: bool,
    ability_pressed: Option<u8>,
    switch_weapon_pressed: bool,
}
```

Adapt the fields to the existing input system. Preserve edge-triggered actions such as attack, dodge, interact, ability activation, and weapon switching instead of converting them into uncontrolled repeated requests.

The host must validate each command against:

- The player's current state
- Cooldowns
- Current animation/action lock
- Stamina/resource requirements
- Weapon/class loadout
- Whether the player is downed or dead
- Whether the requested action is allowed in the current room/state

## Player movement and prediction

Implement movement in this order:

### First version

- Client collects movement input.
- Client sends input to the host through Lightyear.
- Host applies movement and collision.
- Host replicates the resulting transform and relevant velocity/state.
- Remote clients interpolate the player.
- The local client may initially use server-authoritative movement without prediction.

### Later version

Add Lightyear client prediction for the locally controlled player only if the first version is stable. Reconcile predicted movement with host snapshots.

Do not add prediction to enemies or every player until the authoritative version is working.

Movement must continue to respect existing collision and navigation constraints. Network synchronization must not reintroduce the old enemy-navigation problems around obstacle clearance, corners, navigation bounds, spawn points, or arena boundaries.

## Combat synchronization

Combat must remain event-driven and authoritative.

The client sends an attack intent. The host then:

1. Confirms that the player owns the weapon.
2. Confirms that the player can currently attack.
3. Starts the attack or rejects it.
4. Performs authoritative hit detection.
5. Applies damage, guard, dodge, barrier, status, and death-prevention rules.
6. Replicates the resulting state changes.
7. Sends combat events needed for presentation.

Explicitly represent state that affects network outcomes:

- Attack phase
- Attack cooldown
- Dodge window
- Invulnerability window
- Guard state
- Barrier amount
- Damage-over-time effects
- Buffs and debuffs
- Stun or interrupt state
- Death-prevention state
- Weapon swap state
- Ability cooldowns

The existing weapon families and effects must remain compatible with this approach, including melee weapons, bows, crossbows, spellbooks, daggers, hammers, sword-and-shield weapons, scorch, damage-over-time, barriers, frontal guard, and future legendary effects.

## Projectiles and abilities

The host creates the authoritative projectile or ability instance. The client may display an immediate predicted visual, but the host decides the actual result.

For each projectile/ability, synchronize only the gameplay state necessary for other clients to see the same result:

- Network entity identity
- Owner/player ID
- Origin and direction
- Spawn tick/time
- Position/velocity when needed
- Lifetime
- Collision result
- Piercing/fragmentation state
- Damage result
- Applied status effect
- Despawn reason

Do not replicate every particle or sound as a gameplay object. Use compact gameplay events and create presentation locally.

## Enemy and boss synchronization

Run enemy AI only on the host. Clients should not independently run authoritative enemy decision-making.

Replicate, as appropriate:

- Enemy network identity
- Enemy type
- Transform and velocity
- Current health/max health
- Current target player ID
- Status effects
- Stun/interrupt state
- Attack/animation state
- Death state
- Boss phase
- Despawn state

The host must be the only authority that:

- Selects enemy targets
- Moves enemies authoritatively
- Applies enemy damage
- Decides enemy death
- Starts boss phases
- Spawns boss adds
- Completes an encounter

Clients may interpolate enemy transforms and play local animations based on replicated state.

## Loot, chests, and rewards

The host owns all reward generation.

The host must determine:

- Whether a chest has already been opened
- Which item or reward is generated
- Rarity and affixes
- Which player receives or may claim the item
- Whether a pickup has already been consumed
- Whether a reward event has already been processed

Use the host's deterministic run seed and a controlled random stream. Do not let each client roll its own loot table.

A recommended initial cooperative rule is:

- Chests are shared world objects.
- The host generates the reward once.
- The reward is replicated to all players.
- Each player receives an individual eligible pickup roll or selection according to the existing loot design.

If the current loot design is not yet individual-per-player, implement the simplest consistent rule and document it instead of creating ambiguous behavior.

## Run setup and level progression

The host controls the run setup screen and progression.

At the start of a run, synchronize:

- Run seed
- Selected classes
- Enabled item-drop categories
- Ruin condition
- Any host-selected modifiers
- Starting level
- Expansion/final-boss unlock state

The host must select levels using explicit data:

1. Select the next `LevelType`.
2. Select one of that type's two authored variants.
3. Replicate the selected level definition to clients.
4. Wait until clients report that the level is loaded and ready.
5. Start gameplay only after the required players are ready.

Every level must include its boss encounter. The ordinary level selector must not accidentally retain the old five-level boss cadence.

The fixed final level must remain outside ordinary random selection. Expansion final bosses should be registered alternatives with explicit unlock and host-selection rules.

Conceptual state:

```rust
struct NetworkRunState {
    run_seed: u64,
    level_index: u32,
    level_type: LevelType,
    level_variant: LevelVariant,
    boss_defeated: bool,
    selected_final_boss: FinalBossId,
}
```

Use the actual project types if they already exist.

## Death, downing, and reviving

Implement a clear initial cooperative rule:

- A player enters a downed state instead of immediately disappearing.
- Other players can interact to revive them.
- The host validates revive range, timing, interruptions, and player state.
- The host completes or cancels the revive.
- If every player is downed or dead, the run ends.

Synchronize:

- Downed state
- Revive target
- Reviving player
- Revive progress
- Interrupt reason
- Death state
- Spectator/respawn state if added later

Do not let the client locally decide that a revive completed.

## Connection lifecycle

Implement explicit states for:

```rust
enum MultiplayerConnectionState {
    Offline,
    StartingHost,
    Connecting,
    Connected,
    LoadingWorld,
    InRun,
    Disconnecting,
    Disconnected,
    ConnectionFailed,
}
```

Handle:

- Host startup failure
- Invalid address/port
- Connection timeout
- Rejected connection
- Client disconnect
- Host shutdown
- Unexpected network loss
- Loading timeout
- Incompatible protocol/version

The first implementation may end the run when the host disconnects. Do not attempt host migration until basic multiplayer is stable.

Joining after a run has started may be disabled initially. Show a clear UI message rather than partially spawning a player into an unknown state.

## Replication categories

Use reliable delivery for important state changes and events such as:

- Player join/leave
- Class selection
- Ready state
- Weapon selection
- Chest opened
- Loot created
- Item pickup
- Enemy death
- Boss phase change
- Revive completion
- Level transition
- Run completion/failure

Use unreliable or snapshot-style delivery for frequently changing state where the newest value replaces older values, such as:

- Player transforms
- Enemy transforms
- Velocity
- Aim direction when needed
- Noncritical animation state

Use the Lightyear mechanism appropriate for the selected version. Do not manually build a second transport layer beside Lightyear.

## Scene and room loading

The host chooses the level and tells clients which authored level to load. Clients must not independently choose a random level.

Each level/room should have a stable content identifier, such as:

```rust
struct LevelDefinitionId {
    level_type: LevelType,
    variant: u8,
}
```

The GLB gameplay markers should remain deterministic and usable by the host:

- Player spawn points
- Enemy spawn points
- Ranged spawn points
- Elite spawn
- Boss spawn
- Reward spawn
- Exit/next-room marker
- Navigation bounds
- Camera target

The host spawns authoritative gameplay entities from these markers. Clients load the same visual level and receive or reconstruct replicated gameplay entities according to the Lightyear design.

## UI requirements

Add or adapt UI for:

- Host game
- Join game
- Connection status
- Player list
- Player IDs/names
- Class selection
- Ready status
- Loading progress
- Other players' health/downed state
- Revive progress
- Host/network error messages
- Optional ping display

The existing run setup UI must not allow clients to silently change host-owned settings. Either restrict controls to the host or implement an explicit voting system later.

## Recommended implementation milestones

### Milestone 0: Repository audit

Before editing:

1. Inspect `Cargo.toml`, Bevy version, executable entry points, app states, plugins, game-state resources, player components, enemy components, combat events, room loading, run setup, loot, and UI.
2. Trace input -> gameplay state -> entity/component changes -> presentation.
3. Identify which existing systems run only on the client and which can run on the host.
4. Record the audit in a short implementation note or Claude's working plan.
5. Run `cargo check` before modifications to establish a baseline.

### Milestone 1: Lightyear bootstrapping

Implement:

- Compatible Lightyear dependencies
- Offline/host/client network mode
- Host startup
- Client connection
- Protocol registration
- Connection state UI/logging behind debug settings

Acceptance test: a host and client can connect without starting gameplay.

### Milestone 2: Replicated player entities

Implement:

- Player network identity
- Host player spawn
- Client player spawn
- Player transforms
- Ownership mapping
- Remote-player interpolation

Acceptance test: two windows connect and both see two players in the same room.

### Milestone 3: Input and movement

Implement:

- Network input command
- Host movement simulation
- Collision validation
- Remote movement replication
- Local player correction

Acceptance test: both players can move, collide with level geometry, and see each other move consistently.

### Milestone 4: Shared combat prototype

Use one weapon and one simple enemy first.

Implement:

- Attack input
- Host hit detection
- Host damage
- Replicated health
- Enemy death
- Combat presentation events

Acceptance test: both players see the same enemy health, hit reactions, and death.

### Milestone 5: Full player combat

Migrate the remaining combat systems incrementally:

- Dodge and invulnerability
- Guard/blocking
- Barriers
- Healing
- Status effects
- Damage-over-time
- Abilities
- Projectiles
- Weapon switching
- Death prevention
- Weapon-specific and legendary effects

After each system, run a focused two-client test.

### Milestone 6: Enemy and boss authority

Implement host-owned enemy AI, navigation, elite behavior, boss phases, boss adds, and encounter completion.

Acceptance test: both clients observe identical enemy deaths, boss phases, and encounter completion.

### Milestone 7: Loot and run progression

Implement host-owned loot, chest state, rewards, deterministic seed, level type/variant selection, boss completion, and final-level selection.

Acceptance test: all clients load the same authored level and progress through the same run.

### Milestone 8: Co-op death/revival and polish

Implement downing, revives, run failure, co-op UI, disconnect handling, loading readiness, and useful network diagnostics.

### Milestone 9: Prediction and online transport

Only after local host/client multiplayer is stable:

- Add local movement prediction if needed.
- Add interpolation tuning.
- Add bandwidth measurement.
- Add relay/platform transport.
- Test across separate machines and real latency.

## Testing strategy

Add tests where practical for pure logic:

- Input command serialization/deserialization
- Authority rejection of invalid actions
- Damage calculation
- Dodge-window validation
- Loot determinism from a run seed
- Level type then variant selection
- Final-level exclusion from ordinary selection
- Boss-per-level progression
- Revive state transitions

Manual multiplayer tests must include:

1. Two local clients joining one host.
2. Players entering and leaving combat areas.
3. Both players attacking the same enemy.
4. One player dodging while another player is hit.
5. Damage-over-time and status effects.
6. Boss phase changes.
7. Chest opening and reward pickup.
8. Player downing and revival.
9. Level completion and next-level loading.
10. Client disconnect.
11. Host shutdown.
12. Artificial latency and packet loss once transport testing is available.

Use `cargo fmt`, `cargo check`, and focused `cargo test` after every coherent milestone. Do not claim multiplayer is working based only on compilation.

## Debugging requirements

Add a disabled-by-default network debug setting. When enabled, it may show:

- Connection state
- Player/network IDs
- Host/client role
- Ping/round-trip time
- Input tick
- Last received snapshot tick
- Replicated entity count
- Server correction count
- Dropped/rejected input count
- Authority rejection reason

Do not leave high-volume per-frame logging enabled by default.

## Performance and bandwidth rules

- Do not send every particle, sound, or UI update over the network.
- Do not replicate full world state every frame if Lightyear snapshots/replication can provide deltas or interpolation.
- Do not run authoritative AI on every client.
- Do not send final damage values from clients.
- Do not send chest or loot events repeatedly without an idempotency check.
- Prefer compact IDs and enums for events.
- Keep network tick rate and snapshot frequency centralized and tunable.
- Measure before optimizing.

## Definition of done for the first multiplayer prototype

The first multiplayer prototype is complete when:

- Offline single-player still works.
- A player can host a session.
- A second player can connect.
- Both players receive stable player IDs.
- Both players see one another in the same authored room.
- Both players can move and collide with the room.
- Both players can attack the same enemy.
- The host validates damage and enemy death.
- Both players see the same enemy health and death.
- Invalid client actions are rejected by the host.
- `cargo fmt`, `cargo check`, and focused tests pass.
- The implementation has a documented manual test procedure.

Do not expand to every weapon, boss, level, item, or online transport until this vertical slice is stable.

## Final implementation instruction to Claude

Implement this incrementally in the existing Ruinbound repository. First inspect the source and establish the baseline. Then make the smallest coherent Lightyear integration that preserves offline mode. After each milestone, format and compile the project, run focused tests, and report:

- Files changed
- Systems added or migrated
- Lightyear/Bevy compatibility assumptions
- Commands run
- Test results
- Manual playtest steps
- Remaining limitations

If the current codebase conflicts with this design, do not silently perform a broad rewrite. Explain the conflict, identify the smallest migration path, and preserve existing working behavior wherever possible.
