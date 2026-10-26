//! Presentation for other players' characters, on every machine.
//!
//! An *avatar* is any `NetworkPlayer` entity that isn't this machine's own
//! gameplay `Player`:
//! - on the host: the `RemotePlayerBody` of each joined player;
//! - on a client: every replicated character, including its own.
//!
//! This module only draws avatars and smooths their movement. It never
//! decides where a character is. That always comes from the host's
//! `NetworkPose`.

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

use crate::camera::CameraTarget;
use crate::characters::animation::{
    CharacterAnimationOwner, SHARED_ANIMATION_PATH, initialize_character_animation_player,
};
use crate::game_state::AppState;
use crate::player::{LocallyDriven, Player};

use super::has_simulation_authority;
use super::protocol::{NetworkEffect, NetworkPlayer, NetworkPose, NetworkProjectile};
use super::status::NetworkStatus;

pub struct AvatarsPlugin;

impl Plugin for AvatarsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                attach_avatar_visual,
                follow_camera_with_own_avatar,
                // On the host, a body's (or enemy's) Transform *is* the
                // authority, so only clients smooth toward the received pose.
                smooth_toward_pose.run_if(not(has_simulation_authority)),
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        );
    }
}

/// Avatars move toward their latest received pose at this rate (per
/// second, exponential). Higher is snappier; lower hides jitter better.
const AVATAR_SMOOTHING_RATE: f32 = 15.0;

/// If an avatar is this far from its pose (in world units, e.g. after a
/// respawn or a lag spike), it jumps there instead of gliding.
const AVATAR_SNAP_DISTANCE: f32 = 5.0;

/// Added once an avatar has its model, so it isn't added twice.
#[derive(Component)]
struct AvatarVisualAttached;

/// Marks the avatar that belongs to this machine's player (clients only).
#[derive(Component)]
pub struct OwnAvatar;

fn attach_avatar_visual(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    avatars: Query<
        (Entity, &NetworkPlayer, &NetworkPose, Has<Transform>),
        (Without<Player>, Without<AvatarVisualAttached>),
    >,
) {
    for (entity, network_player, pose, has_transform) in &avatars {
        // Replicated entities arrive without a Transform. Host-side bodies
        // already have one, and it must not be overwritten.
        if !has_transform {
            commands.entity(entity).insert(pose.to_transform());
        }

        // Every class currently shares this one animated model (see
        // `characters::vanguard`). The avatar idles until animation state
        // is replicated in a later milestone.
        let character_scene =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(SHARED_ANIMATION_PATH));

        commands
            .entity(entity)
            .insert((AvatarVisualAttached, Visibility::default()))
            .with_children(|parent| {
                parent
                    .spawn((
                        Name::new(format!(
                            "Player {} Avatar Visual",
                            network_player.player_id.0
                        )),
                        CharacterAnimationOwner::new(entity),
                        WorldAssetRoot(character_scene),
                        // Same offset as the local player's model: the
                        // character's origin is the capsule's center.
                        Transform::from_xyz(0.0, -1.0, 0.0),
                    ))
                    .observe(initialize_character_animation_player);
            });
    }
}

/// On a client, the camera follows the avatar the host assigned to us.
fn follow_camera_with_own_avatar(
    mut commands: Commands,
    status: Res<NetworkStatus>,
    avatars: Query<(Entity, &NetworkPlayer), (Without<Player>, Without<OwnAvatar>)>,
) {
    let Some(local_player_id) = status.local_player_id else {
        return;
    };
    for (entity, network_player) in &avatars {
        if network_player.player_id.0 == local_player_id {
            commands.entity(entity).insert((OwnAvatar, CameraTarget));
        }
    }
}

/// Smooths every replicated entity, players' avatars and enemies alike.
/// Projectiles are moved by `net::projectiles` instead: they are too fast
/// for smoothing, so they extrapolate along their velocity.
fn smooth_toward_pose(
    time: Res<Time>,
    mut avatars: Query<
        (&NetworkPose, &mut Transform),
        // Projectiles and ability effects move themselves
        // (`net::projectiles`, `net::effect_visuals`); a client's own
        // predicted character moves itself (`net::prediction`).
        (
            Without<Player>,
            Without<LocallyDriven>,
            Without<NetworkProjectile>,
            Without<NetworkEffect>,
        ),
    >,
) {
    let blend = 1.0 - (-AVATAR_SMOOTHING_RATE * time.delta_secs()).exp();

    for (pose, mut transform) in &mut avatars {
        let target_position = pose.position();
        if transform.translation.distance(target_position) > AVATAR_SNAP_DISTANCE {
            *transform = pose.to_transform();
            continue;
        }
        transform.translation = transform.translation.lerp(target_position, blend);
        transform.rotation = transform.rotation.slerp(pose.rotation(), blend);
    }
}
