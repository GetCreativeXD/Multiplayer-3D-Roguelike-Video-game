//! Player intent over the network (Milestone 3).
//!
//! Every machine reads its own keyboard and camera into
//! `player::LocalPlayerInput` (world space). Then:
//! - **Client:** `send_local_intent_to_host` sends it to the host about 60
//!   times a second on the unreliable `IntentChannel`.
//! - **Host:** `apply_remote_intents` copies each client's latest message
//!   into the `PlayerIntent` of that player's `RemotePlayerBody`. The
//!   ordinary `player` movement systems then move the body exactly as they
//!   move the host's own `Player`, with collision, gravity, jumping and
//!   dodging, and the body's new pose is replicated back to everyone.
//!
//! The host never trusts a client's numbers blindly: directions are
//! flattened, normalized and checked for NaN before use.
//!
//! Each message carries a `sequence`; the host reports the last one it
//! applied (`net::prediction`), which is what lets the client predict its
//! own movement and correct it against the host (Milestone 9).
//! `send_local_intent_to_host` records the client's predicted position for
//! every message it sends.

use bevy::prelude::*;
use lightyear::connection::client_of::ClientOf;
use lightyear::prelude::*;

use crate::game_state::AppState;
use crate::player::{LocalPlayerInput, PlayerInputSet, PlayerIntent};

use super::player_bodies::RemotePlayerBody;
use super::prediction::OwnPrediction;
use super::protocol::{IntentChannel, NetworkPlayer, PlayerIntentMessage};
use super::roster::PlayerRoster;

/// How often a client sends its intent, in seconds. A jump or dodge press
/// is sent immediately regardless.
const INTENT_SEND_INTERVAL: f32 = 1.0 / 60.0;

/// If the host hears nothing from a client for this long (lag spike,
/// stalled window), that player's character stops walking instead of
/// running off in its last direction.
const INTENT_STALE_AFTER: f32 = 0.5;

/// Client mode: sends this player's intent to the host.
pub struct ClientIntentPlugin;

impl Plugin for ClientIntentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            send_local_intent_to_host
                .after(PlayerInputSet)
                .run_if(in_state(AppState::InGame)),
        );
    }
}

/// Host mode: drives remote players' bodies from their clients' intents.
pub struct HostIntentPlugin;

impl Plugin for HostIntentPlugin {
    fn build(&self, app: &mut App) {
        // Beside the local input, just before the fixed-tick loop, so a
        // remote press reaches the next tick's movement (Milestone 9).
        app.add_systems(
            RunFixedMainLoop,
            apply_remote_intents
                .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop)
                .after(PlayerInputSet)
                .run_if(in_state(AppState::InGame)),
        );
    }
}

// ---------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------

/// Running totals of this client's presses (see `PlayerIntentMessage`).
#[derive(Default)]
struct PressTotals {
    jump: u32,
    dodge: u32,
    attack: u32,
    ability: u32,
    class_ability: [u32; 3],
    interact: u32,
    weapon_slot: [u32; 3],
}

fn send_local_intent_to_host(
    time: Res<Time>,
    local_input: Res<LocalPlayerInput>,
    mut connection: Query<&mut MessageSender<PlayerIntentMessage>, (With<Client>, With<Connected>)>,
    mut own_prediction: Query<(&Transform, &mut OwnPrediction)>,
    mut totals: Local<PressTotals>,
    mut since_last_send: Local<f32>,
    mut sequence: Local<u32>,
) {
    if local_input.jump_pressed {
        totals.jump = totals.jump.wrapping_add(1);
    }
    if local_input.dodge_pressed {
        totals.dodge = totals.dodge.wrapping_add(1);
    }
    if local_input.attack_pressed {
        totals.attack = totals.attack.wrapping_add(1);
    }
    if local_input.ability_pressed {
        totals.ability = totals.ability.wrapping_add(1);
    }
    for (total, pressed) in totals
        .class_ability
        .iter_mut()
        .zip(local_input.class_ability_pressed)
    {
        if pressed {
            *total = total.wrapping_add(1);
        }
    }
    if local_input.interact_pressed {
        totals.interact = totals.interact.wrapping_add(1);
    }
    for (total, pressed) in totals
        .weapon_slot
        .iter_mut()
        .zip(local_input.weapon_slot_pressed)
    {
        if pressed {
            *total = total.wrapping_add(1);
        }
    }

    *since_last_send += time.delta_secs();
    let pressed_something = local_input.jump_pressed
        || local_input.dodge_pressed
        || local_input.attack_pressed
        || local_input.ability_pressed
        || local_input.class_ability_pressed.contains(&true)
        || local_input.interact_pressed
        || local_input.weapon_slot_pressed.contains(&true);
    if *since_last_send < INTENT_SEND_INTERVAL && !pressed_something {
        return;
    }
    *since_last_send = 0.0;
    *sequence = sequence.wrapping_add(1);

    // Where this client's own character is as it sends this input, so the
    // host's position at this input can be checked against it.
    for (transform, mut prediction) in &mut own_prediction {
        prediction.record_sent_input(*sequence, transform.translation);
    }

    let message = PlayerIntentMessage {
        sequence: *sequence,
        move_direction: local_input.move_direction.to_array(),
        facing: local_input.facing.to_array(),
        movement_paused: local_input.movement_paused,
        jump_presses: totals.jump,
        dodge_presses: totals.dodge,
        attack_presses: totals.attack,
        ability_presses: totals.ability,
        class_ability_presses: totals.class_ability,
        interact_presses: totals.interact,
        weapon_slot_presses: totals.weapon_slot,
        aim_origin: local_input.aim_origin.to_array(),
        aim_rotation: local_input.aim_rotation.to_array(),
    };

    for mut sender in &mut connection {
        sender.send::<IntentChannel>(message.clone());
    }
}

// ---------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------

#[derive(Clone, Copy)]
struct PressTotalsSeen {
    jump: u32,
    dodge: u32,
    attack: u32,
    ability: u32,
    class_ability: [u32; 3],
    interact: u32,
    weapon_slot: [u32; 3],
}

/// Host-side bookkeeping for one remote player's incoming intent.
#[derive(Component, Default)]
pub struct RemoteIntentState {
    /// The press totals from the last message applied. `None` until the
    /// first message, which only sets the baseline (presses made before
    /// this body existed are not replayed).
    last_presses: Option<PressTotalsSeen>,
    seconds_since_message: f32,
    /// `PlayerIntentMessage::sequence` of the last message applied
    /// (reported back to the client by `net::prediction`).
    last_sequence: u32,
}

impl RemoteIntentState {
    pub fn last_sequence(&self) -> u32 {
        self.last_sequence
    }
}

fn apply_remote_intents(
    time: Res<Time>,
    roster: Res<PlayerRoster>,
    mut links: Query<(Entity, &mut MessageReceiver<PlayerIntentMessage>), With<ClientOf>>,
    mut bodies: Query<
        (
            &NetworkPlayer,
            &Transform,
            &mut PlayerIntent,
            &mut RemoteIntentState,
        ),
        With<RemotePlayerBody>,
    >,
) {
    let delta = time.delta_secs();
    for (_, _, _, mut state) in &mut bodies {
        state.seconds_since_message += delta;
    }

    for (link, mut receiver) in &mut links {
        // Only the newest message matters: presses are running totals.
        let Some(message) = receiver.receive().last() else {
            continue;
        };
        let Some(sender) = roster.player_for_connection(link) else {
            continue;
        };
        if sender.is_host {
            continue;
        }

        let Some((_, transform, mut intent, mut state)) = bodies
            .iter_mut()
            .find(|(player, _, _, _)| player.player_id == sender.player_id)
        else {
            // The body is spawned a frame after the player joins the run.
            continue;
        };

        apply_message(&message, transform.translation, &mut intent, &mut state);
    }

    for (_, _, mut intent, state) in &mut bodies {
        if state.seconds_since_message > INTENT_STALE_AFTER {
            intent.move_direction = Vec3::ZERO;
        }
    }
}

fn apply_message(
    message: &PlayerIntentMessage,
    body_position: Vec3,
    intent: &mut PlayerIntent,
    state: &mut RemoteIntentState,
) {
    intent.move_direction = sanitize_direction(message.move_direction);
    intent.facing = sanitize_direction(message.facing);
    intent.movement_paused = message.movement_paused;
    (intent.aim_origin, intent.aim_rotation) = sanitize_aim(
        message.aim_origin,
        message.aim_rotation,
        body_position,
        intent.facing,
    );

    let presses = PressTotalsSeen {
        jump: message.jump_presses,
        dodge: message.dodge_presses,
        attack: message.attack_presses,
        ability: message.ability_presses,
        class_ability: message.class_ability_presses,
        interact: message.interact_presses,
        weapon_slot: message.weapon_slot_presses,
    };
    if let Some(last) = state.last_presses {
        if presses.jump != last.jump {
            intent.request_jump();
        }
        if presses.dodge != last.dodge {
            intent.request_dodge();
        }
        if presses.attack != last.attack {
            intent.request_attack();
        }
        if presses.ability != last.ability {
            intent.request_ability();
        }
        for slot in 0..presses.class_ability.len() {
            if presses.class_ability[slot] != last.class_ability[slot] {
                intent.request_class_ability(slot);
            }
        }
        if presses.interact != last.interact {
            intent.request_interact();
        }
        for slot in 0..presses.weapon_slot.len() {
            if presses.weapon_slot[slot] != last.weapon_slot[slot] {
                intent.request_weapon_slot(slot);
            }
        }
    }
    state.last_presses = Some(presses);
    state.seconds_since_message = 0.0;
    state.last_sequence = message.sequence;
}

/// A horizontal unit vector, or zero. Anything else a client sends (NaN,
/// a vertical component, a speed hack length) is corrected here, so a
/// client can never move faster than `PLAYER_SPEED` or fly.
fn sanitize_direction(raw: [f32; 3]) -> Vec3 {
    let flat = Vec3::new(raw[0], 0.0, raw[2]);
    if !flat.is_finite() || flat.length_squared() < 0.01 * 0.01 {
        return Vec3::ZERO;
    }
    flat.normalize_or_zero()
}

/// How far from their character a client's camera may be, in world units.
/// The real camera sits about 8 units behind and above the character
/// (`camera::CAMERA_DISTANCE`); this leaves room for lag, and stops a client
/// from aiming from somewhere else in the level.
const MAX_AIM_ORIGIN_DISTANCE: f32 = 15.0;

/// A camera position near the body and a valid rotation, or a fallback:
/// looking along `facing` from just above the character.
fn sanitize_aim(
    raw_origin: [f32; 3],
    raw_rotation: [f32; 4],
    body_position: Vec3,
    facing: Vec3,
) -> (Vec3, Quat) {
    let fallback_origin = body_position + Vec3::Y * 1.0;
    let fallback_rotation = if facing == Vec3::ZERO {
        Quat::IDENTITY
    } else {
        Transform::default().looking_to(facing, Vec3::Y).rotation
    };

    let origin = Vec3::from_array(raw_origin);
    let origin = if origin.is_finite() && origin.distance(body_position) <= MAX_AIM_ORIGIN_DISTANCE
    {
        origin
    } else {
        fallback_origin
    };

    let rotation = Quat::from_array(raw_rotation);
    let rotation = if rotation.is_finite() && rotation.length_squared() > 0.25 {
        rotation.normalize()
    } else {
        fallback_rotation
    };

    (origin, rotation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(jump: u32, dodge: u32) -> PlayerIntentMessage {
        PlayerIntentMessage {
            sequence: 1,
            move_direction: [0.0, 0.0, 1.0],
            facing: [0.0, 0.0, 1.0],
            movement_paused: false,
            jump_presses: jump,
            dodge_presses: dodge,
            attack_presses: 0,
            ability_presses: 0,
            class_ability_presses: [0; 3],
            interact_presses: 0,
            weapon_slot_presses: [0; 3],
            aim_origin: [0.0, 3.0, 8.0],
            aim_rotation: [0.0, 0.0, 0.0, 1.0],
        }
    }

    fn apply_message_at_origin(
        message: &PlayerIntentMessage,
        intent: &mut PlayerIntent,
        state: &mut RemoteIntentState,
    ) {
        apply_message(message, Vec3::ZERO, intent, state);
    }

    #[test]
    fn aim_near_the_body_is_kept() {
        let (origin, rotation) =
            sanitize_aim([0.0, 3.0, 8.0], [0.0, 0.0, 0.0, 2.0], Vec3::ZERO, Vec3::Z);
        assert_eq!(origin, Vec3::new(0.0, 3.0, 8.0));
        assert!((rotation.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn aim_far_away_or_broken_falls_back_to_the_body() {
        let (origin, _) =
            sanitize_aim([500.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], Vec3::ZERO, Vec3::Z);
        assert_eq!(origin, Vec3::Y);
        let (_, rotation) = sanitize_aim(
            [0.0, 0.0, 0.0],
            [f32::NAN, 0.0, 0.0, 1.0],
            Vec3::ZERO,
            Vec3::Z,
        );
        assert!(rotation.is_finite());
        assert!((Transform::from_rotation(rotation).forward().as_vec3() - Vec3::Z).length() < 1e-4);
    }

    #[test]
    fn directions_are_flattened_and_normalized() {
        assert_eq!(sanitize_direction([3.0, 5.0, 0.0]), Vec3::X);
        assert_eq!(sanitize_direction([0.0, 1.0, 0.0]), Vec3::ZERO);
        assert_eq!(sanitize_direction([f32::NAN, 0.0, 1.0]), Vec3::ZERO);
        assert_eq!(sanitize_direction([0.0, 0.0, 0.0]), Vec3::ZERO);
    }

    #[test]
    fn first_message_sets_a_baseline_without_jumping() {
        let mut intent = PlayerIntent::default();
        let mut state = RemoteIntentState::default();

        apply_message_at_origin(&message(7, 3), &mut intent, &mut state);
        assert_eq!(intent.jump_requests(), 0);

        apply_message_at_origin(&message(8, 3), &mut intent, &mut state);
        assert_eq!(intent.jump_requests(), 1);

        // A repeated total (no new press) does nothing.
        apply_message_at_origin(&message(8, 3), &mut intent, &mut state);
        assert_eq!(intent.jump_requests(), 1);
    }

    #[test]
    fn totals_survive_wrapping() {
        let mut intent = PlayerIntent::default();
        let mut state = RemoteIntentState::default();

        apply_message_at_origin(&message(u32::MAX, 0), &mut intent, &mut state);
        apply_message_at_origin(&message(0, 0), &mut intent, &mut state);
        assert_eq!(intent.jump_requests(), 1);
    }

    #[test]
    fn a_new_attack_total_requests_one_attack() {
        let mut intent = PlayerIntent::default();
        let mut state = RemoteIntentState::default();

        apply_message_at_origin(&message(0, 0), &mut intent, &mut state);
        let mut attacking = message(0, 0);
        attacking.attack_presses = 1;
        apply_message_at_origin(&attacking, &mut intent, &mut state);

        assert!(intent.take_attack());
        assert!(!intent.take_attack());
    }

    #[test]
    fn a_new_ability_total_requests_one_ability() {
        let mut intent = PlayerIntent::default();
        let mut state = RemoteIntentState::default();

        apply_message_at_origin(&message(0, 0), &mut intent, &mut state);
        let mut casting = message(0, 0);
        casting.ability_presses = 1;
        apply_message_at_origin(&casting, &mut intent, &mut state);

        assert!(intent.take_ability());
        assert!(!intent.take_ability());
        // An ability press is not an attack press.
        assert!(!intent.take_attack());
    }

    #[test]
    fn class_ability_presses_request_their_own_slot() {
        let mut intent = PlayerIntent::default();
        let mut state = RemoteIntentState::default();

        apply_message_at_origin(&message(0, 0), &mut intent, &mut state);
        let mut casting = message(0, 0);
        casting.class_ability_presses = [0, 0, 1];
        apply_message_at_origin(&casting, &mut intent, &mut state);

        assert!(!intent.take_class_ability(0));
        assert!(!intent.take_class_ability(1));
        assert!(intent.take_class_ability(2));
        assert!(!intent.take_class_ability(2));
        // Out-of-range slots are ignored.
        assert!(!intent.take_class_ability(7));
    }

    #[test]
    fn interact_and_weapon_slot_presses_reach_the_body() {
        use crate::player::InteractionKind;

        let mut intent = PlayerIntent::default();
        let mut state = RemoteIntentState::default();

        apply_message_at_origin(&message(0, 0), &mut intent, &mut state);
        let mut pressing = message(0, 0);
        pressing.interact_presses = 1;
        pressing.weapon_slot_presses = [0, 1, 0];
        apply_message_at_origin(&pressing, &mut intent, &mut state);

        // One [F] press is seen once by each kind of interaction.
        assert!(intent.take_interact(InteractionKind::OpenChest));
        assert!(intent.take_interact(InteractionKind::PickUp));
        assert!(!intent.take_interact(InteractionKind::PickUp));
        assert_eq!(intent.take_weapon_slot(), Some(1));
        assert_eq!(intent.take_weapon_slot(), None);
    }
}
