//! Host-owned enemies, seen by every client (Milestone 4).
//!
//! - **Host:** every `Enemy` gets a `NetworkEnemy` (its kind), a
//!   `NetworkPose` and `NetworkVitals`, and is replicated. The host keeps
//!   running all enemy AI, navigation, damage and death exactly as offline;
//!   this module only *publishes* the result. Poses are published by
//!   `player_bodies::publish_host_poses`, vitals by `vitals::publish_vitals`. When an enemy dies the
//!   host despawns it, which despawns it on every client.
//! - **Client:** a replicated enemy becomes a *replica*: the right model
//!   for its kind (`enemies::attach_enemy_visual`), a `Health` mirrored from
//!   `NetworkVitals` (by `vitals::mirror_vitals`), and a health bar. Its `Transform` is smoothed toward the
//!   host's pose by `avatars::smooth_toward_pose`.
//!
//! A replica deliberately never gets the `Enemy` marker: every AI and
//! combat system filters on `Enemy`, so a replica can't think, attack, or
//! be "killed" locally. (The client also still removes the enemies its own
//! rooms spawn; see `client::remove_locally_spawned_enemies`.)
//!
//! The Saint's model is animated on clients from its replicated
//! `SaintAnimationCue`; telegraphs and aim lines are in `net::telegraphs`,
//! projectiles in `net::projectiles`.

use bevy::prelude::*;
use lightyear::prelude::*;

use crate::combat::Health;
use crate::enemies::{Enemy, EnemyKind, attach_enemy_visual};
use crate::game_state::AppState;
use crate::ui::spawn_enemy_health_bar;

use super::protocol::{NetworkEnemy, NetworkPose, NetworkVitals};

// ---------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------

pub struct HostEnemyReplicationPlugin;

impl Plugin for HostEnemyReplicationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            replicate_new_enemies.run_if(in_state(AppState::InGame)),
        );
    }
}

fn replicate_new_enemies(
    mut commands: Commands,
    enemies: Query<(Entity, &EnemyKind, &Transform, &Health), (With<Enemy>, Without<NetworkEnemy>)>,
) {
    for (entity, kind, transform, health) in &enemies {
        // `try_insert`: harmless if the enemy died this same frame.
        commands.entity(entity).try_insert((
            NetworkEnemy { kind: *kind },
            NetworkPose::from_transform(transform),
            NetworkVitals::from_health(health),
            Replicate::to_clients(NetworkTarget::All),
        ));
    }
}

// ---------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------

pub struct ClientEnemyReplicaPlugin;

impl Plugin for ClientEnemyReplicaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            attach_enemy_replica.run_if(in_state(AppState::InGame)),
        );
    }
}

/// A client's visual-only copy of a host enemy.
#[derive(Component)]
pub struct EnemyReplica;

fn attach_enemy_replica(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    new_enemies: Query<
        (Entity, &NetworkEnemy, &NetworkPose, &NetworkVitals),
        Without<EnemyReplica>,
    >,
) {
    for (entity, network_enemy, pose, vitals) in &new_enemies {
        let kind = network_enemy.kind;

        // Replicated entities arrive without a Transform.
        commands.entity(entity).insert((
            EnemyReplica,
            Name::new(format!("{kind:?} (replica)")),
            pose.to_transform(),
            Visibility::default(),
            vitals.to_health(),
        ));

        attach_enemy_visual(
            &mut commands,
            entity,
            kind,
            &mut meshes,
            &mut materials,
            &asset_server,
        );

        // Every enemy gets the same floating bar as on the host, the Saint
        // included (`rooms::saint_arena` gives the host's Saint this bar).
        spawn_enemy_health_bar(&mut commands, entity);
    }
}
