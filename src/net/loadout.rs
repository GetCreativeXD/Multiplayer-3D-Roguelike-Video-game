//! Each joined player's own weapon (Milestone 5 step 3) and class
//! (Milestone 8 step 1).
//!
//! - **Client:** when the run starts, sends the weapon and class chosen on
//!   its own run-setup screen (`RunConfig::weapon`, `RunConfig::class`) to
//!   the host.
//! - **Host:** remembers each player's choice and gives their
//!   `RemotePlayerBody` that weapon at Common tier, the same tier the
//!   host's own starting weapon has, and that class
//!   (`characters::insert_class`). Their basic attacks then go through the
//!   shared `weapons::melee::player_basic_attack`, aimed with their camera;
//!   their class abilities through each class's systems, for the classes
//!   migrated to `PlayerIntent` so far.
//!
//! Both are given once per run. The chosen weapon goes into weapon slot 1;
//! after that, the player's weapons are theirs to change (loot, switching,
//! Milestone 7) and this module leaves them alone.

use std::collections::HashMap;

use bevy::prelude::*;
use lightyear::connection::client_of::ClientOf;
use lightyear::prelude::*;

use crate::characters::{ClassId, insert_class};
use crate::game_state::AppState;
use crate::run_setup::RunConfig;
use crate::weapons::{Equipment, GearTier, PlayerCombat, Weapon, WeaponAbilityState, WeaponType};

use super::player_bodies::RemotePlayerBody;
use super::protocol::{NetworkPlayer, NetworkPlayerId, PlayerLoadoutMessage, SessionChannel};
use super::roster::PlayerRoster;

// ---------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------

pub struct ClientLoadoutPlugin;

impl Plugin for ClientLoadoutPlugin {
    fn build(&self, app: &mut App) {
        // A client only enters the run after the host's `RunStartedMessage`,
        // so it is connected at this point.
        app.add_systems(OnEnter(AppState::InGame), send_loadout_to_host);
    }
}

fn send_loadout_to_host(
    run_config: Res<RunConfig>,
    mut connection: Query<
        &mut MessageSender<PlayerLoadoutMessage>,
        (With<Client>, With<Connected>),
    >,
) {
    let message = PlayerLoadoutMessage {
        weapon: run_config.weapon,
        class: run_config.class,
    };
    for mut sender in &mut connection {
        sender.send::<SessionChannel>(message.clone());
        info!(
            "[net] Sent loadout to host: {:?}, {:?}",
            message.weapon, message.class
        );
    }
}

// ---------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------

pub struct HostLoadoutPlugin;

impl Plugin for HostLoadoutPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RemoteLoadouts>().add_systems(
            Update,
            (
                receive_loadouts,
                apply_loadouts_to_bodies.run_if(in_state(AppState::InGame)),
            )
                .chain(),
        );
    }
}

/// The weapon and class each joined player asked for. Player IDs are never
/// reused, so entries for players who left are simply never read again.
#[derive(Resource, Default)]
struct RemoteLoadouts(HashMap<NetworkPlayerId, (WeaponType, ClassId)>);

/// On a `RemotePlayerBody` once its class has been given.
#[derive(Component)]
struct ClassGiven;

/// On a `RemotePlayerBody` once its chosen starting weapon has been given.
#[derive(Component)]
struct StartingWeaponGiven;

fn receive_loadouts(
    roster: Res<PlayerRoster>,
    mut links: Query<(Entity, &mut MessageReceiver<PlayerLoadoutMessage>), With<ClientOf>>,
    mut loadouts: ResMut<RemoteLoadouts>,
) {
    for (link, mut receiver) in &mut links {
        let Some(message) = receiver.receive().last() else {
            continue;
        };
        let Some(sender) = roster.player_for_connection(link) else {
            continue;
        };
        if sender.is_host {
            continue;
        }
        info!(
            "[net] Player {} chose {:?}, {:?}",
            sender.player_id.0, message.weapon, message.class
        );
        loadouts
            .0
            .insert(sender.player_id, (message.weapon, message.class));
    }
}

/// Gives each body its player's class and starting weapon, once each.
/// Also covers a body spawned after the message arrived.
fn apply_loadouts_to_bodies(
    mut commands: Commands,
    loadouts: Res<RemoteLoadouts>,
    mut bodies: Query<
        (
            Entity,
            &NetworkPlayer,
            Has<ClassGiven>,
            Has<StartingWeaponGiven>,
            &mut Weapon,
            &mut Equipment,
            &mut PlayerCombat,
            Option<&mut WeaponAbilityState>,
        ),
        With<RemotePlayerBody>,
    >,
) {
    for (
        body,
        network_player,
        given_class,
        given_weapon,
        mut weapon,
        mut equipment,
        mut combat,
        ability_state,
    ) in &mut bodies
    {
        let Some(&(chosen, class)) = loadouts.0.get(&network_player.player_id) else {
            continue;
        };

        if !given_class {
            insert_class(&mut commands, body, class);
            commands.entity(body).insert(ClassGiven);
            info!(
                "[net] Player {} is a {:?}",
                network_player.player_id.0, class
            );
        }

        if given_weapon {
            continue;
        }
        commands.entity(body).insert(StartingWeaponGiven);

        // Replace only the starter in weapon slot 1, so nothing the player
        // may already have picked up is lost.
        let starting_weapon = Weapon::of_type(chosen, GearTier::Common);
        equipment.weapon_slots[0] = Some(starting_weapon);
        if equipment.active_weapon_slot == 0 {
            *weapon = starting_weapon;
            // Drop any windup or cooldown from the old weapon.
            *combat = PlayerCombat::default();
            if let Some(mut ability_state) = ability_state {
                *ability_state = WeaponAbilityState::default();
            }
        }
        info!(
            "[net] Player {} now wields a Common {:?}",
            network_player.player_id.0, chosen
        );
    }
}
