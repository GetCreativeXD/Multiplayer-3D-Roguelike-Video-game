//! Loot in the world, shared by every player (Milestone 7 step 1).
//!
//! - **Host:** every pickup and chest (anything with `weapons::WorldLoot`,
//!   whether a room placed it, an enemy dropped it, a chest rolled it or a
//!   player dropped it from their inventory) gets a `NetworkPlacement` (its
//!   world position, worked out through the room it belongs to) and is
//!   replicated. Picking it up or opening it despawns it on the host, which
//!   removes it everywhere. Loot messages meant for a joined player
//!   ("Stored Iron Helm in backpack slot 2") are sent to that player's
//!   client.
//! - **Client:** the rooms a client loads still place their own pickups and
//!   chests; those are removed (like enemies, see `net::client`) and the
//!   host's copies are drawn instead, bobbing and spinning locally. [F]
//!   reaches the host as part of the player's intent (`net::intent`), so
//!   the host decides who gets what. The prompts read `WorldLoot` and the
//!   player's replicated `weapons::PlayerInventorySnapshot`.
//! - **Inventory screen (step 2):** a joined player's screen shows their
//!   replicated snapshot. Each drag-and-drop is sent to the host as an
//!   `InventoryMoveMessage` and queued on their body's
//!   `weapons::InventoryRequests`; the host applies it with the same rules
//!   as its own (`weapons::inventory_ui::apply_inventory_requests`), so
//!   equipping armor gives that player its defense and health.

use bevy::prelude::*;
use lightyear::connection::client_of::ClientOf;
use lightyear::prelude::*;

use crate::game_state::AppState;
use crate::weapons::{
    InventoryMoveRequested, InventoryRequests, LootFeedback, LootFeedbackShown,
    MAX_QUEUED_INVENTORY_MOVES, PickupAnimation, WorldLoot,
};

use super::player_bodies::RemotePlayerBody;
use super::protocol::{
    InventoryMoveMessage, LootFeedbackMessage, NetworkPlacement, NetworkPlayer, SessionChannel,
};
use super::roster::PlayerRoster;

// ---------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------

pub struct HostLootPlugin;

impl Plugin for HostLootPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (replicate_world_loot, receive_inventory_moves).run_if(in_state(AppState::InGame)),
        )
        .add_observer(send_loot_feedback_to_joined_players);
    }
}

fn replicate_world_loot(
    mut commands: Commands,
    new_loot: Query<
        (
            Entity,
            &Transform,
            Option<&ChildOf>,
            Option<&PickupAnimation>,
        ),
        (With<WorldLoot>, Without<NetworkPlacement>),
    >,
    parents: Query<(&Transform, Option<&ChildOf>)>,
) {
    for (entity, transform, child_of, animation) in &new_loot {
        let world = world_transform(*transform, child_of, &parents);
        // A pickup bobs around its resting height; send that height, not
        // wherever the bob happens to be this frame.
        let bob = animation.map_or(0.0, |animation| {
            transform.translation.y - animation.base_y()
        });
        let resting = world.translation - Vec3::Y * bob;

        // `try_insert`: harmless if it was picked up this same frame.
        commands.entity(entity).try_insert((
            NetworkPlacement::new(resting, world.rotation),
            Replicate::to_clients(NetworkTarget::All),
            DisableReplicateHierarchy,
        ));
    }
}

/// A room places its loot as its own children, so the world position is
/// the room's transform applied to the loot's. (`GlobalTransform` isn't
/// computed yet for loot spawned this frame.)
fn world_transform<'a>(
    local: Transform,
    mut child_of: Option<&'a ChildOf>,
    parents: &'a Query<(&Transform, Option<&ChildOf>)>,
) -> Transform {
    let mut world = local;
    while let Some(parent) = child_of {
        let Ok((parent_transform, grandparent)) = parents.get(parent.parent()) else {
            break;
        };
        world = parent_transform.mul_transform(world);
        child_of = grandparent;
    }
    world
}

fn send_loot_feedback_to_joined_players(
    shown: On<LootFeedbackShown>,
    bodies: Query<&NetworkPlayer, With<RemotePlayerBody>>,
    roster: Res<PlayerRoster>,
    mut links: Query<&mut MessageSender<LootFeedbackMessage>, (With<ClientOf>, With<Connected>)>,
) {
    let Ok(body) = bodies.get(shown.character) else {
        return;
    };
    let Some(member) = roster
        .players()
        .find(|member| member.player_id == body.player_id)
    else {
        return;
    };
    if let Ok(mut sender) = links.get_mut(member.connection) {
        sender.send::<SessionChannel>(LootFeedbackMessage {
            text: shown.message.clone(),
        });
    }
}

/// Queues each joined player's inventory moves on their own body. A
/// player never reaches anyone else's gear: the body is found from the
/// connection the message arrived on.
fn receive_inventory_moves(
    roster: Res<PlayerRoster>,
    mut links: Query<(Entity, &mut MessageReceiver<InventoryMoveMessage>), With<ClientOf>>,
    mut bodies: Query<(&NetworkPlayer, &mut InventoryRequests), With<RemotePlayerBody>>,
) {
    for (link, mut receiver) in &mut links {
        let moves: Vec<_> = receiver.receive().collect();
        if moves.is_empty() {
            continue;
        }
        let Some(sender) = roster.player_for_connection(link) else {
            continue;
        };
        if sender.is_host {
            continue;
        }
        let Some((_, mut requests)) = bodies
            .iter_mut()
            .find(|(player, _)| player.player_id == sender.player_id)
        else {
            continue;
        };
        for InventoryMoveMessage(inventory_move) in moves {
            if requests.0.len() < MAX_QUEUED_INVENTORY_MOVES {
                requests.0.push(inventory_move);
            }
        }
    }
}

// ---------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------

pub struct ClientLootPlugin;

impl Plugin for ClientLootPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (attach_loot_replicas, show_loot_feedback_from_host).run_if(in_state(AppState::InGame)),
        )
        // `Last`, so every room's spawn commands have been applied.
        .add_systems(Last, remove_locally_spawned_loot)
        .add_observer(send_inventory_moves_to_host);
    }
}

/// A client's visual-only copy of a host pickup or chest.
#[derive(Component)]
struct LootReplica;

/// The client's own rooms placed these; the host's copies (which arrive
/// with a `NetworkPlacement`) replace them.
fn remove_locally_spawned_loot(
    mut commands: Commands,
    local_loot: Query<Entity, (With<WorldLoot>, Without<NetworkPlacement>)>,
) {
    for loot in &local_loot {
        commands.entity(loot).despawn();
    }
}

fn attach_loot_replicas(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    new_loot: Query<(Entity, &WorldLoot, &NetworkPlacement), Without<LootReplica>>,
) {
    for (entity, loot, placement) in &new_loot {
        let position = placement.position();
        let mut replica = commands.entity(entity);
        // Replicated entities arrive without a Transform.
        replica.insert((
            LootReplica,
            Name::new("Loot (replica)"),
            Transform::from_translation(position).with_rotation(placement.rotation()),
            Visibility::default(),
            Mesh3d(meshes.add(loot.mesh())),
            MeshMaterial3d(materials.add(loot.material())),
        ));
        if loot.animates() {
            replica.insert(PickupAnimation::new(
                position.y,
                position.x * 0.37 + position.z * 0.19,
            ));
        }
    }
}

fn show_loot_feedback_from_host(
    mut receivers: Query<&mut MessageReceiver<LootFeedbackMessage>, With<Client>>,
    mut feedback: ResMut<LootFeedback>,
) {
    for mut receiver in &mut receivers {
        for message in receiver.receive() {
            feedback.show(message.text);
        }
    }
}

/// This player's inventory screen asked for a move: the host makes it.
fn send_inventory_moves_to_host(
    requested: On<InventoryMoveRequested>,
    mut connection: Query<
        &mut MessageSender<InventoryMoveMessage>,
        (With<Client>, With<Connected>),
    >,
) {
    for mut sender in &mut connection {
        sender.send::<SessionChannel>(InventoryMoveMessage(requested.0));
    }
}
