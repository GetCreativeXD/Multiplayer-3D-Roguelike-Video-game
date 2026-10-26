//! Health for every host-simulated character, on every machine
//! (Milestone 5 step 2).
//!
//! - **Host:** any entity with both `Health` and `NetworkVitals` (every
//!   replicated enemy, the host's `Player` and each `RemotePlayerBody`) has
//!   its health copied into `NetworkVitals` after `Update`, only when it
//!   changed, so untouched characters send nothing.
//! - **Client:** a received `NetworkVitals` is mirrored into a local
//!   `Health` that only health bars read. Player avatars get that `Health`
//!   here; enemy replicas get theirs in `net::enemies`.
//! - **Every machine:** each player character that isn't this machine's own
//!   `Player` gets a green floating health bar. So the host sees joined
//!   players' health, and a client sees everyone's, including its own (it
//!   has no HUD yet).

use bevy::prelude::*;

use crate::combat::Health;
use crate::game_state::AppState;
use crate::player::Player;
use crate::ui::spawn_teammate_health_bar;

use super::has_simulation_authority;
use super::protocol::{NetworkPlayer, NetworkVitals};

pub struct VitalsPlugin;

impl Plugin for VitalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (give_avatars_health, mirror_vitals).run_if(not(has_simulation_authority)),
                attach_player_health_bars,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        )
        // After `Update`, so vitals include this frame's damage.
        .add_systems(PostUpdate, publish_vitals.run_if(has_simulation_authority));
    }
}

/// Host: only writes when health changed, because any write marks the
/// component as changed and gets it sent.
fn publish_vitals(mut characters: Query<(&Health, &mut NetworkVitals)>) {
    for (health, mut vitals) in &mut characters {
        if !vitals.matches(health) {
            *vitals = NetworkVitals::from_health(health);
        }
    }
}

/// Client: a replicated player character arrives without `Health`.
fn give_avatars_health(
    mut commands: Commands,
    avatars: Query<(Entity, &NetworkVitals), (With<NetworkPlayer>, Without<Health>)>,
) {
    for (entity, vitals) in &avatars {
        commands.entity(entity).insert(vitals.to_health());
    }
}

/// Client: keeps every replica's `Health` equal to the host's.
fn mirror_vitals(mut replicas: Query<(&NetworkVitals, &mut Health), Changed<NetworkVitals>>) {
    for (vitals, mut health) in &mut replicas {
        if !vitals.matches(&health) {
            *health = vitals.to_health();
        }
    }
}

/// Added once a player character has its floating health bar.
#[derive(Component)]
struct PlayerHealthBarAttached;

fn attach_player_health_bars(
    mut commands: Commands,
    characters: Query<
        Entity,
        (
            With<NetworkPlayer>,
            With<Health>,
            Without<Player>,
            Without<PlayerHealthBarAttached>,
        ),
    >,
) {
    for entity in &characters {
        commands.entity(entity).insert(PlayerHealthBarAttached);
        spawn_teammate_health_bar(&mut commands, entity);
    }
}
