//! Enemy telegraphs and area attacks, seen by every client (Milestone 6).
//!
//! The host decides every hit. This module only lets a joined player *see*
//! the warnings they need to react to, drawn exactly like the host's:
//!
//! - **Telegraph entities** (the Saint's Fist Slam disk and shockwave, the
//!   Black Banner Knight's rush lane and banner disk; anything tagged
//!   `enemies::EnemyTelegraph`). The host replicates each one with a
//!   `NetworkTelegraph` (its kind) and a `NetworkPose`. A client adds the
//!   same meshes (`enemies::attach_telegraph_visual`) and animates them
//!   from the time since they appeared (`enemies::telegraph_scale`): the
//!   disks pulse, the shockwave grows at the host's speed. The host
//!   despawning one removes it everywhere.
//! - **The Gatebow Marksman's aim line.** It follows a moving player, so it
//!   isn't replicated as an entity. The host puts a `NetworkEnemyAim` (who
//!   it is aiming at) on the Marksman, and a client draws the line between
//!   its own smoothed copies of the Marksman and that player, so the line
//!   stays attached to both.
//!
//! A client's copy of a telegraph appears about one round trip plus the
//! replication interval after the host's, like everything else a client
//! sees (no prediction until Milestone 9).

use bevy::prelude::*;
use lightyear::prelude::*;

use crate::enemies::{
    Enemy, EnemyTarget, EnemyTelegraph, RangedEnemyCombat, aim_line_target, aim_line_transform,
    aim_line_visual, attach_telegraph_visual, telegraph_scale,
};
use crate::game_state::AppState;

use super::protocol::{
    NetworkEnemy, NetworkEnemyAim, NetworkPlayer, NetworkPose, NetworkTelegraph,
};

// ---------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------

pub struct HostTelegraphReplicationPlugin;

impl Plugin for HostTelegraphReplicationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (replicate_new_telegraphs, publish_enemy_aim).run_if(in_state(AppState::InGame)),
        );
    }
}

fn replicate_new_telegraphs(
    mut commands: Commands,
    telegraphs: Query<(Entity, &EnemyTelegraph, &Transform), Without<NetworkTelegraph>>,
) {
    for (entity, telegraph, transform) in &telegraphs {
        // `try_insert`: harmless if the telegraph ended this same frame.
        commands.entity(entity).try_insert((
            NetworkTelegraph {
                kind: telegraph.kind,
                spawn_scale: transform.scale.to_array(),
            },
            NetworkPose::from_transform(transform),
            Replicate::to_clients(NetworkTarget::All),
            // The shockwave's rings are children; a client draws its own.
            DisableReplicateHierarchy,
        ));
    }
}

/// Keeps each replicated Marksman's `NetworkEnemyAim` in step with its aim
/// line on the host (`enemies::aim_line_target`). Only writes on change.
fn publish_enemy_aim(
    mut commands: Commands,
    players: Query<&NetworkPlayer>,
    mut marksmen: Query<
        (
            Entity,
            &RangedEnemyCombat,
            &EnemyTarget,
            Option<&mut NetworkEnemyAim>,
        ),
        (With<Enemy>, With<NetworkEnemy>),
    >,
) {
    for (entity, combat, target, aim) in &mut marksmen {
        let aim_target = aim_line_target(combat, target)
            .and_then(|player| players.get(player).ok())
            .map(|player| player.player_id);
        let wanted = NetworkEnemyAim { target: aim_target };

        match aim {
            Some(mut aim) => {
                aim.set_if_neq(wanted);
            }
            None => {
                commands.entity(entity).try_insert(wanted);
            }
        }
    }
}

// ---------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------

pub struct ClientTelegraphReplicaPlugin;

impl Plugin for ClientTelegraphReplicaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                attach_telegraph_replicas,
                animate_telegraph_replicas,
                update_aim_line_replicas,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        );
    }
}

/// A client's visual-only copy of a host telegraph.
#[derive(Component, Default)]
struct TelegraphReplica {
    seconds_alive: f32,
}

fn attach_telegraph_replicas(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    new_telegraphs: Query<(Entity, &NetworkTelegraph, &NetworkPose), Without<TelegraphReplica>>,
) {
    for (entity, telegraph, pose) in &new_telegraphs {
        let spawn_scale = Vec3::from_array(telegraph.spawn_scale);

        // Replicated entities arrive without a Transform.
        commands.entity(entity).insert((
            TelegraphReplica::default(),
            Name::new(format!("{:?} (replica)", telegraph.kind)),
            pose.to_transform()
                .with_scale(telegraph_scale(telegraph.kind, 0.0, spawn_scale)),
            Visibility::default(),
        ));

        attach_telegraph_visual(
            &mut commands,
            entity,
            telegraph.kind,
            &mut meshes,
            &mut materials,
        );
    }
}

fn animate_telegraph_replicas(
    time: Res<Time>,
    mut telegraphs: Query<(&NetworkTelegraph, &mut TelegraphReplica, &mut Transform)>,
) {
    for (telegraph, mut replica, mut transform) in &mut telegraphs {
        replica.seconds_alive += time.delta_secs();
        transform.scale = telegraph_scale(
            telegraph.kind,
            replica.seconds_alive,
            Vec3::from_array(telegraph.spawn_scale),
        );
    }
}

/// A client's aim line, drawn for the enemy `owner`.
#[derive(Component)]
struct AimLineReplica {
    owner: Entity,
}

/// On an enemy copy: the aim line currently drawn for it.
#[derive(Component)]
struct DrawnAimLine(Entity);

fn update_aim_line_replicas(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    shooters: Query<
        (Entity, &NetworkEnemyAim, &Transform, Option<&DrawnAimLine>),
        Without<AimLineReplica>,
    >,
    players: Query<(&NetworkPlayer, &Transform), Without<AimLineReplica>>,
    mut lines: Query<(Entity, &AimLineReplica, &mut Transform), Without<NetworkEnemyAim>>,
) {
    for (shooter, aim, shooter_transform, drawn) in &shooters {
        let line = aim
            .target
            .and_then(|target| {
                players
                    .iter()
                    .find(|(player, _)| player.player_id == target)
            })
            .and_then(|(_, player_transform)| {
                aim_line_transform(shooter_transform.translation, player_transform.translation)
            });

        match (line, drawn) {
            (Some(line), Some(drawn)) => {
                if let Ok((_, _, mut transform)) = lines.get_mut(drawn.0) {
                    *transform = line;
                }
            }
            (Some(line), None) => {
                let line_entity = commands
                    .spawn((
                        Name::new("Aim Line (replica)"),
                        AimLineReplica { owner: shooter },
                        aim_line_visual(&mut meshes, &mut materials),
                        line,
                    ))
                    .id();
                commands.entity(shooter).insert(DrawnAimLine(line_entity));
            }
            (None, Some(drawn)) => {
                commands.entity(drawn.0).try_despawn();
                commands.entity(shooter).remove::<DrawnAimLine>();
            }
            (None, None) => {}
        }
    }

    // A line whose enemy died (despawned by the host) goes with it.
    for (line_entity, line, _) in &lines {
        if shooters.get(line.owner).is_err() {
            commands.entity(line_entity).try_despawn();
        }
    }
}
