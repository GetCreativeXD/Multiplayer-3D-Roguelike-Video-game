//! Host only: makes sure every player in the session has exactly one
//! replicated character, and keeps each character's `NetworkPose` current.
//!
//! - **The host's own player** is the normal gameplay `Player` entity spawned by
//!   the run-start room. It just gains `NetworkPlayer` + `NetworkPose` +
//!   `Replicate`, so nothing about local play changes.
//! - **Each remote player** gets a `RemotePlayerBody`: a replicated character
//!   placed beside the host. It has the same capsule, collision, dodge and
//!   jump state as `Player`, plus a `PlayerIntent` that `net::intent` fills
//!   from that player's client, so the shared `player` movement systems move
//!   it. It is a `PlayerCharacter`, so enemies notice, chase and damage it
//!   exactly like the host's own player, with the same `Health`, defenses
//!   and dodge invulnerability. It is deliberately *not* a `Player`: many
//!   gameplay systems (HUD, gear, abilities, loot) still mean "this
//!   machine's player", and are migrated in later milestones.
//!
//! Both kinds of character also carry `NetworkVitals`, so every machine can
//! show their health (`net::vitals`).
//!
//! All systems here are idempotent "make it so" checks that run every
//! frame during a run. Players who join mid-run and players who leave are
//! handled without any special cases.

use bevy::prelude::*;
use lightyear::prelude::*;

use crate::characters::animation::CharacterMotion;
use crate::combat::{BaseHealth, Health};
use crate::game_state::AppState;
use crate::player::{Player, PlayerCharacter, PlayerIntent, player_character_body};
use crate::run_setup::RunModifiers;
use crate::weapons::{Equipment, GearTier, PlayerCombat, Weapon, WeaponAbilityState, WeaponType};

use super::intent::RemoteIntentState;
use super::protocol::{NetworkPlayer, NetworkPlayerId, NetworkPose, NetworkVitals};
use super::roster::PlayerRoster;

pub struct PlayerBodiesPlugin;

impl Plugin for PlayerBodiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                give_host_player_a_network_identity,
                spawn_bodies_for_remote_players,
                despawn_bodies_of_departed_players,
            )
                .run_if(in_state(AppState::InGame)),
        )
        // After `Update`, so the pose reflects this frame's movement.
        .add_systems(PostUpdate, publish_host_poses);
    }
}

/// A remote player's character as it exists on the host.
#[derive(Component)]
pub struct RemotePlayerBody;

/// How far apart remote players stand when they first appear beside the
/// host, in world units.
const REMOTE_SPAWN_SPACING: f32 = 2.0;

fn give_host_player_a_network_identity(
    mut commands: Commands,
    roster: Res<PlayerRoster>,
    host_player: Query<
        (Entity, &Transform, Option<&Health>),
        (With<Player>, Without<NetworkPlayer>),
    >,
) {
    let Some(player_id) = roster.host_player_id() else {
        return;
    };
    for (entity, transform, health) in &host_player {
        let mut host_player = commands.entity(entity);
        host_player.insert((
            NetworkPlayer { player_id },
            NetworkPose::from_transform(transform),
            Replicate::to_clients(NetworkTarget::All),
        ));
        if let Some(health) = health {
            host_player.insert(NetworkVitals::from_health(health));
        }
    }
}

fn spawn_bodies_for_remote_players(
    mut commands: Commands,
    roster: Res<PlayerRoster>,
    existing_bodies: Query<&NetworkPlayer, With<RemotePlayerBody>>,
    host_player: Query<&Transform, With<Player>>,
    run_modifiers: Res<RunModifiers>,
) {
    // Remote players appear relative to the host, so wait for the host's
    // own character to exist.
    let Ok(host_transform) = host_player.single() else {
        return;
    };

    for member in roster.players().filter(|member| !member.is_host) {
        let already_spawned = existing_bodies
            .iter()
            .any(|body| body.player_id == member.player_id);
        if already_spawned {
            continue;
        }

        let mut transform = *host_transform;
        transform.translation += remote_spawn_offset(member.player_id);

        // Same starting health as the host's player
        // (`rooms::fallen_gate::setup_fallen_gate_player`), including any
        // relic bonus. Joined players have no gear of their own yet, so
        // their `Equipment` is just the starter Longsword (no defense).
        let starting_max_health = 100.0 + run_modifiers.bonus_max_health;
        let weapon = Weapon::of_type(WeaponType::Longsword, GearTier::Common);
        let health = Health::new(starting_max_health);

        let body = commands
            .spawn((
                Name::new(format!("Remote Player {}", member.player_id.0)),
                RemotePlayerBody,
                NetworkPlayer {
                    player_id: member.player_id,
                },
                NetworkPose::from_transform(&transform),
                transform,
                Replicate::to_clients(NetworkTarget::All),
                // Movement: driven by the client's intent via `net::intent`.
                PlayerIntent::default(),
                RemoteIntentState::default(),
                // Filled from intent + dodge by `characters::animation`, and
                // replicated so clients animate this player.
                CharacterMotion::default(),
                player_character_body(),
                // Starts with a Common Longsword; `net::loadout` swaps in the
                // weapon this player chose once their client reports it. Attacks
                // go through `weapons::melee::player_basic_attack`.
                PlayerCombat::default(),
                weapon,
            ))
            .id();
        // Milestone 5: enemies target, chase and damage this character
        // (`enemies::targeting`). `PlayerCharacter` also gives it barriers,
        // death prevention and stat buffs (`combat::initialize_combat_defenses`).
        // A second insert keeps the spawn tuple under Bevy's bundle size limit.
        commands.entity(body).insert((
            PlayerCharacter,
            BaseHealth(starting_max_health),
            NetworkVitals::from_health(&health),
            health,
            Equipment::with_starter_weapon(weapon, run_modifiers.bonus_ability_power),
            // Per-weapon ability stacks and cooldowns (Echoing Cast,
            // Phantom Step, ...), as `weapons::inventory` gives the host's
            // `Player`. Weapon abilities (right click) go through
            // `weapons::abilities::weapon_ability_input`.
            WeaponAbilityState::default(),
        ));
    }
}

/// Spreads remote players out to the host's right: 2, 4, then 6 units.
fn remote_spawn_offset(player_id: NetworkPlayerId) -> Vec3 {
    let slot = (player_id.0.saturating_sub(1) % 3) + 1;
    Vec3::X * REMOTE_SPAWN_SPACING * slot as f32
}

/// Removing the entity on the host also removes it on every client.
fn despawn_bodies_of_departed_players(
    mut commands: Commands,
    roster: Res<PlayerRoster>,
    bodies: Query<(Entity, &NetworkPlayer), With<RemotePlayerBody>>,
) {
    for (entity, body) in &bodies {
        if !roster.contains(body.player_id) {
            commands.entity(entity).despawn();
        }
    }
}

/// Copies every host-simulated entity's `Transform` (player characters and
/// enemies) into its replicated `NetworkPose`. Only writes when it actually
/// moved, because any write marks the component as changed and gets it sent.
fn publish_host_poses(mut replicated: Query<(&Transform, &mut NetworkPose)>) {
    for (transform, mut pose) in &mut replicated {
        if pose.differs_from(transform) {
            *pose = NetworkPose::from_transform(transform);
        }
    }
}
