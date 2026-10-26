//! Everything the host and clients must agree on byte-for-byte: channels,
//! messages, replicated components, and the protocol version. Both sides
//! register exactly this plugin, so a mismatch is caught at connect time by
//! Lightyear's protocol check and by `PROTOCOL_ID` below.

use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use crate::characters::ClassId;
use crate::characters::animation::CharacterMotion;
use crate::combat::Health;
use crate::enemies::{EnemyKind, EnemyTelegraphKind, SaintAnimationCue};
use crate::player::LifeState;
use crate::ruin::RuinSnapshot;
use crate::run_setup::SharedRunSettings;
use crate::ui::PlayerHudSnapshot;
use crate::visual_effects::EffectShape;
use crate::weapons::{InventoryMove, PlayerInventorySnapshot, WeaponType, WorldLoot};

/// Bump this whenever a channel, message, or replicated component is
/// added, removed, or changed. Netcode refuses connections whose protocol
/// ID differs, so an outdated build gets a clean rejection instead of
/// undefined behaviour.
///
/// History: 1 = connection + welcome (M1). 2 = player entities + run start (M2).
/// 3 = client -> host player intent (M3). 4 = attack intent, replicated
/// enemies, swing-arc effects (M4). 5 = replicated `CharacterMotion` (M5).
/// 6 = `EnemyVitals` renamed `NetworkVitals` and also sent for players (M5).
/// 7 = camera aim in `PlayerIntentMessage`, `PlayerLoadoutMessage` (M5).
/// 8 = replicated projectiles: `NetworkProjectile`, `NetworkVelocity` (M5).
/// 9 = weapon-ability presses in `PlayerIntentMessage` (M5).
/// 10 = enemy presentation: `NetworkTelegraph`, `NetworkEnemyAim`,
/// `SaintAnimationCue` (M6).
/// 11 = `ui::PlayerHudSnapshot` for joined players' HUD (M5).
/// 12 = shared ability effects: `NetworkEffect`, `NetworkScale` (M6).
/// 13 = class in `PlayerLoadoutMessage`, class-ability presses in
/// `PlayerIntentMessage`, class slots in `ui::PlayerHudSnapshot` (M8).
/// 14 = replicated `player::LifeState` (Downed / Lost) (M8).
/// 15 = lobby: `RunSettingsMessage`, `ReadyMessage`, `LobbyMessage`,
/// `RunEndedMessage` (M8).
/// 16 = shared loot: interact and weapon-slot presses in
/// `PlayerIntentMessage`, replicated `weapons::WorldLoot` +
/// `NetworkPlacement` and `weapons::PlayerInventorySnapshot`,
/// `LootFeedbackMessage` (M7).
/// 17 = `InventoryMoveMessage`: joined players' inventory screen (M7).
/// 18 = replicated `ruin::RuinSnapshot`: shared Ruin on clients (M7).
/// 19 = `WelcomeMessage::protocol`: build check for Steam links (M9).
/// 20 = `PlayerIntentMessage::sequence`, replicated `NetworkMovementAck`:
/// own-character prediction (M9).
pub const PROTOCOL_ID: u64 = 20;

/// Development-only Netcode key, shared by host and client so clients can
/// build their own connect tokens (`Authentication::Manual`). Before any
/// public release this must be replaced by tokens issued by a backend.
pub const DEV_PRIVATE_KEY: [u8; 32] = [0; 32];

// ---------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------

/// The host-assigned identity of a player. Stable for the whole session
/// and meaningful on every machine, unlike a Bevy `Entity`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NetworkPlayerId(pub u64);

// ---------------------------------------------------------------------
// Replicated components (host -> every client)
// ---------------------------------------------------------------------

/// Marks an entity as one player's character and says whose it is. The
/// host puts it on every player's character; clients receive it.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct NetworkPlayer {
    pub player_id: NetworkPlayerId,
}

/// Where a host-simulated entity (a player's character or an enemy) stands
/// and which way it faces. Only the host writes it; clients smooth their
/// `Transform` toward it (`net::avatars`).
///
/// On a joined player's body: the newest `PlayerIntentMessage::sequence`
/// the host has applied, and the body's walking-speed multiplier in percent
/// (class modifiers and buffs only the host simulates). Replicated beside
/// `NetworkPose`, so a client knows which of its inputs the pose reflects
/// (`net::prediction`).
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetworkMovementAck {
    pub input_sequence: u32,
    pub speed_percent: u16,
}

/// Stored as plain arrays rather than `Vec3`/`Quat` so it serializes
/// without enabling Bevy's optional `serialize` feature.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct NetworkPose {
    position: [f32; 3],
    rotation: [f32; 4],
}

impl NetworkPose {
    pub fn from_transform(transform: &Transform) -> Self {
        Self {
            position: transform.translation.to_array(),
            rotation: transform.rotation.to_array(),
        }
    }

    pub fn position(&self) -> Vec3 {
        Vec3::from_array(self.position)
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_array(self.rotation)
    }

    pub fn to_transform(self) -> Transform {
        Transform::from_translation(self.position()).with_rotation(self.rotation())
    }

    /// Whether `transform` has moved or turned enough to be worth sending.
    /// Skipping tiny changes keeps an idle player from using bandwidth.
    pub fn differs_from(&self, transform: &Transform) -> bool {
        const POSITION_EPSILON: f32 = 0.001;
        const ROTATION_EPSILON: f32 = 0.0001;
        self.position().distance_squared(transform.translation)
            > POSITION_EPSILON * POSITION_EPSILON
            || self.rotation().dot(transform.rotation).abs() < 1.0 - ROTATION_EPSILON
    }
}

/// Marks a host-owned enemy and says which kind it is, so a client can draw
/// the right model. A client's copy never runs AI and never gets the
/// `Enemy` marker (`net::enemies`).
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct NetworkEnemy {
    pub kind: EnemyKind,
}

/// A host-simulated character's health (an enemy or a player), for
/// clients' health bars. Only the host writes it (`net::vitals`).
/// Despawning an enemy on the host (death) despawns it on every client.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct NetworkVitals {
    pub current: f32,
    pub max: f32,
}

impl NetworkVitals {
    pub fn from_health(health: &Health) -> Self {
        Self {
            current: health.current,
            max: health.max,
        }
    }

    pub fn to_health(self) -> Health {
        Health {
            current: self.current,
            max: self.max,
        }
    }

    pub fn matches(&self, health: &Health) -> bool {
        self.current == health.current && self.max == health.max
    }
}

/// Rough shape a client draws a projectile replica with.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileShape {
    Sphere,
    Box,
}

/// Marks a host-owned projectile (a player's arrow, bolt or spell, or an
/// enemy's arrow) and says how to draw it: its size and colours are read
/// from the host's own mesh and material, so every current and future
/// projectile looks right without a per-kind table (`net::projectiles`).
/// Replicas are visual only; hits are resolved on the host.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct NetworkProjectile {
    pub shape: ProjectileShape,
    /// Half the projectile's size along each local axis, in world units.
    pub half_extents: [f32; 3],
    /// Linear RGBA.
    pub base_color: [f32; 4],
    /// Linear RGBA.
    pub emissive: [f32; 4],
    pub unlit: bool,
}

/// Marks a host-owned enemy telegraph or area-attack visual (the Saint's
/// slam disk and shockwave, the Knight's rush lane and banner disk) and
/// says which, so a client can draw the same thing (`net::telegraphs`).
/// Where it is comes from its `NetworkPose`. Purely visual on clients: the
/// host alone decides who gets hit. The host despawning it (windup over,
/// wave finished) removes it everywhere.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct NetworkTelegraph {
    pub kind: EnemyTelegraphKind,
    /// The telegraph's scale when it appeared (the rush lane's length).
    pub spawn_scale: [f32; 3],
}

/// Marks a host-owned visual effect a player's ability spawned (Hammer's
/// Ground Slam disk, a class ability's burst or summon; anything tagged
/// `visual_effects::SharedEffect`) and says how to draw it: its primitive
/// shape and its material's colours (`net::effect_visuals`). Where it is
/// comes from `NetworkPose` and `NetworkScale`. Visual only on clients.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct NetworkEffect {
    pub shape: EffectShape,
    /// Linear RGBA.
    pub base_color: [f32; 4],
    /// Linear RGBA.
    pub emissive: [f32; 4],
    pub unlit: bool,
}

/// A host-owned visual's scale (pulsing disks), republished on change.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct NetworkScale(pub [f32; 3]);

/// On a host enemy that shows an aim line (the Gatebow Marksman): which
/// player it is aiming at right now, or `None`. A client draws the line
/// between its own copies of the enemy and that player (`net::telegraphs`).
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct NetworkEnemyAim {
    pub target: Option<NetworkPlayerId>,
}

/// Where a host-owned pickup or chest (`weapons::WorldLoot`) rests, in
/// world space. Sent once: loot doesn't move (a client bobs and spins its
/// copy itself, `net::loot`). Not a `NetworkPose`, which is republished
/// from the local `Transform` every time it changes.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct NetworkPlacement {
    position: [f32; 3],
    rotation: [f32; 4],
}

impl NetworkPlacement {
    pub fn new(position: Vec3, rotation: Quat) -> Self {
        Self {
            position: position.to_array(),
            rotation: rotation.to_array(),
        }
    }

    pub fn position(&self) -> Vec3 {
        Vec3::from_array(self.position)
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_array(self.rotation)
    }
}

/// A host-owned projectile's velocity (world units per second), so a
/// client can move its replica smoothly between pose updates.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct NetworkVelocity(pub [f32; 3]);

// ---------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------

/// Reliable, ordered channel for low-frequency session events: joining,
/// leaving, run start, and later ready-state, level transitions, and loot.
pub struct SessionChannel;

/// Host -> client, sent once when the client connects: "you are player N".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WelcomeMessage {
    pub player_id: NetworkPlayerId,
    /// The host's `PROTOCOL_ID`. Netcode (UDP) already refuses a different
    /// build, but Steam connections skip Netcode, so the client compares
    /// this with its own and leaves on a mismatch.
    pub protocol: u64,
}

/// Host -> client: the host pressed Begin Run, so load the run's world.
/// Also sent to a player who joins after the run has already started.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RunStartedMessage;

/// Both ways, on `SessionChannel`: the party's shared run-setup choices
/// (drop pool, relics, pacts, Ruin condition). A client sends it when its
/// player changes one; the host applies it and sends the result to every
/// client, and also sends it when it changes them itself or someone joins
/// (`net::lobby`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RunSettingsMessage(pub SharedRunSettings);

/// Client -> host: "mark me ready / not ready" on the run-setup screen.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReadyMessage {
    pub ready: bool,
}

/// Host -> client: who is ready on the run-setup screen.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct LobbyMessage {
    /// Joined players (not the host) currently connected.
    pub guests: u32,
    /// The ones marked ready.
    pub ready_players: Vec<NetworkPlayerId>,
}

/// Host -> client: the run is over (party wipe); go back to run setup.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RunEndedMessage;

/// Host -> one client: a loot message for that player ("Equipped X in
/// weapon slot 2", "Inventory full ..."), shown like the host's own.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LootFeedbackMessage {
    pub text: String,
}

/// Client -> host: one drag-and-drop in this player's inventory screen
/// (move, equip, swap, or drop on the floor). The host checks it and
/// applies it to that player's own gear (`net::loot`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InventoryMoveMessage(pub InventoryMove);

/// Unreliable channel for each client's continuous movement intent. Only
/// the newest message matters, so older ones that arrive late are dropped
/// rather than replayed.
pub struct IntentChannel;

/// Client -> host, about 60 times a second during a run: what this
/// client's player wants their character to do. See `player::PlayerIntent`.
///
/// Directions are already world space: the client resolves its own
/// camera, because the host has no camera for a remote player.
///
/// Jump and dodge are running totals of presses rather than "pressed this
/// frame" flags, so a lost packet never loses a press; the host acts on
/// the difference from the last total it saw (`net::intent`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerIntentMessage {
    /// Counts up by one per message from this client. The host reports the
    /// last one it applied (`NetworkMovementAck`), so the client can compare
    /// the host's position with where it predicted itself at that input
    /// (`net::prediction`).
    pub sequence: u32,
    pub move_direction: [f32; 3],
    pub facing: [f32; 3],
    pub movement_paused: bool,
    pub jump_presses: u32,
    pub dodge_presses: u32,
    pub attack_presses: u32,
    /// Weapon-ability (right click) presses, a running total like the others.
    pub ability_presses: u32,
    /// Class-ability presses per slot (Q, E, R), running totals.
    pub class_ability_presses: [u32; 3],
    /// Interact ([F]: open chest / pick up) presses, a running total.
    pub interact_presses: u32,
    /// Weapon-slot key presses per slot (1, 2, 3), running totals.
    pub weapon_slot_presses: [u32; 3],
    /// The client's camera position and rotation: weapons aim from it
    /// through the centre of the client's view (`PlayerIntent::aim_origin`).
    pub aim_origin: [f32; 3],
    pub aim_rotation: [f32; 4],
}

/// Client -> host, once at the start of a run: the weapon and class this
/// player chose on their own run-setup screen. The host gives their body
/// that weapon at Common tier and that class (`net::loadout`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerLoadoutMessage {
    pub weapon: WeaponType,
    pub class: ClassId,
}

/// Unreliable host -> client channel for cosmetic combat effects. A lost
/// effect is only a missing flash, never a gameplay difference.
pub struct EffectsChannel;

/// Host -> client: a melee swing arc was drawn on the host (any player's
/// swing). Mirrors `weapons::MeleeArcSpawned`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MeleeArcMessage {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
    pub radius: f32,
    pub duration: f32,
}

// ---------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------

/// Registers the shared protocol. Must be added after Lightyear's
/// `ClientPlugins` / `ServerPlugins`.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_channel::<SessionChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::Bidirectional);

        app.register_message::<WelcomeMessage>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<RunStartedMessage>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<PlayerLoadoutMessage>()
            .add_direction(NetworkDirection::ClientToServer);
        // Lobby (run-setup screen) and run end.
        app.register_message::<RunSettingsMessage>()
            .add_direction(NetworkDirection::Bidirectional);
        app.register_message::<ReadyMessage>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<LobbyMessage>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<RunEndedMessage>()
            .add_direction(NetworkDirection::ServerToClient);
        // Loot (M7).
        app.register_message::<LootFeedbackMessage>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<InventoryMoveMessage>()
            .add_direction(NetworkDirection::ClientToServer);

        app.add_channel::<IntentChannel>(ChannelSettings {
            mode: ChannelMode::SequencedUnreliable,
            ..default()
        })
        .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<PlayerIntentMessage>()
            .add_direction(NetworkDirection::ClientToServer);

        app.add_channel::<EffectsChannel>(ChannelSettings {
            mode: ChannelMode::UnorderedUnreliable,
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<MeleeArcMessage>()
            .add_direction(NetworkDirection::ServerToClient);

        app.component::<NetworkPlayer>().replicate();
        app.component::<NetworkPose>().replicate();
        app.component::<NetworkEnemy>().replicate();
        // Players' animation state: walking / dodging / jump count.
        app.component::<CharacterMotion>().replicate();
        app.component::<NetworkVitals>().replicate();
        app.component::<NetworkProjectile>().replicate();
        app.component::<NetworkVelocity>().replicate();
        // Enemy presentation (M6): telegraphs, aim lines, the Saint's clip.
        app.component::<NetworkTelegraph>().replicate_once();
        app.component::<NetworkEnemyAim>().replicate();
        app.component::<SaintAnimationCue>().replicate();
        // Barrier and weapon-ability state for a joined player's own HUD.
        app.component::<PlayerHudSnapshot>().replicate();
        // Players' ability effects (M6).
        app.component::<NetworkEffect>().replicate_once();
        app.component::<NetworkScale>().replicate();
        // Downed / Lost state of every player (M8).
        app.component::<LifeState>().replicate();
        // Loot in the world and each player's gear, for display (M7).
        app.component::<WorldLoot>().replicate();
        app.component::<NetworkPlacement>().replicate_once();
        app.component::<PlayerInventorySnapshot>().replicate();
        // The run's shared Ruin and clock (M7).
        app.component::<RuinSnapshot>().replicate();
        // Own-character prediction (M9).
        app.component::<NetworkMovementAck>().replicate();
    }
}
