//! Cosmetic combat effects the host forwards to clients (Milestone 4).
//!
//! Gameplay effects never travel this way. The host decides every hit, and
//! clients see the result through replicated state (enemy health, poses,
//! despawns). These messages only let clients draw the same flashes, on the
//! unreliable `EffectsChannel`.
//!
//! Today: melee swing arcs (`weapons::MeleeArcSpawned`), for every player's
//! swing, including the host's own.

use bevy::prelude::*;
use lightyear::connection::client_of::ClientOf;
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;

use crate::weapons::{MeleeArcSpawned, spawn_melee_arc};

use super::protocol::{EffectsChannel, MeleeArcMessage};

/// Host mode: forwards effects to every remote player.
pub struct HostEffectsPlugin;

impl Plugin for HostEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(forward_melee_arcs);
    }
}

/// Client mode: draws effects the host forwarded.
pub struct ClientEffectsPlugin;

impl Plugin for ClientEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_forwarded_melee_arcs);
    }
}

fn forward_melee_arcs(
    arc: On<MeleeArcSpawned>,
    mut remote_players: Query<
        &mut MessageSender<MeleeArcMessage>,
        (With<ClientOf>, With<Connected>, Without<HostClient>),
    >,
) {
    let message = MeleeArcMessage {
        origin: arc.origin.to_array(),
        direction: arc.direction.to_array(),
        radius: arc.radius,
        duration: arc.duration,
    };
    for mut sender in &mut remote_players {
        sender.send::<EffectsChannel>(message.clone());
    }
}

fn draw_forwarded_melee_arcs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut receivers: Query<&mut MessageReceiver<MeleeArcMessage>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        for message in receiver.receive() {
            let direction = Vec3::from_array(message.direction);
            if !direction.is_finite() || !message.radius.is_finite() {
                continue;
            }
            spawn_melee_arc(
                &mut commands,
                &mut meshes,
                &mut materials,
                &Transform::from_translation(Vec3::from_array(message.origin)),
                direction,
                message.radius,
                message.duration,
            );
        }
    }
}
