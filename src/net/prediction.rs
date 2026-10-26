//! Own-character prediction (Milestone 9, step 4).
//!
//! Without prediction a joined player's own walking, jumping and dodging
//! showed up one round trip late: input went to the host, the host moved
//! the body, and the pose came back. Now the client moves its own avatar
//! immediately with the same fixed-tick movement code the host uses, and
//! the host stays the authority: the client checks its prediction against
//! the host and corrects it.
//!
//! How it works:
//! - **Client:** its `OwnAvatar` gets the movement body
//!   (`player::player_character_body`), a `PlayerIntent` and
//!   `player::LocallyDriven`, so local input drives it and
//!   `player::PlayerMovementSet` moves it every tick, exactly like the host's
//!   `Player`. `avatars::smooth_toward_pose` leaves it alone.
//! - Every `PlayerIntentMessage` carries a `sequence`. When the client
//!   sends one, it records where it predicted itself to be
//!   (`OwnPrediction::record_sent_input`).
//! - **Host:** reports, on each joined player's body, the last sequence it
//!   applied and the body's walking-speed multiplier
//!   (`NetworkMovementAck`, replicated beside `NetworkPose`).
//! - **Reconcile:** when a new sequence is acknowledged, the client compares
//!   the host's position with its own recorded one. Small differences
//!   (timing noise between two machines' frames) are ignored; larger ones
//!   are eased out over a few ticks; huge ones (a void reset, a long
//!   knockback) snap. Later recorded positions are shifted by the same
//!   correction, so one disagreement is corrected exactly once.
//!
//! Not predicted (the host's result arrives and is eased in): weapon and
//! class dashes (`AbilityMovement`), knockback, and anything else only the
//! host simulates. Combat itself is unchanged: the host decides every hit.

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::characters::class_abilities::CharacterCombatModifiers;
use crate::combat::StatBuffs;
use crate::game_state::AppState;
use crate::player::{
    LifeState, LocallyDriven, PlayerInputSet, PlayerIntent, PlayerMovementSet,
    PredictedMovementSpeed, VerticalMovement, movement_speed_multiplier, player_character_body,
};

use super::avatars::OwnAvatar;
use super::intent::RemoteIntentState;
use super::player_bodies::RemotePlayerBody;
use super::protocol::{NetworkMovementAck, NetworkPose};
use super::settings::NetworkSettings;

/// Differences smaller than this (world units) are left alone. Two machines
/// run their ticks at slightly different moments, so the host's position for
/// an input is naturally off by up to a tick or two of movement (walking is
/// 0.1 units per tick, a dodge 0.22).
const IGNORED_ERROR: f32 = 0.35;

/// Differences larger than this snap instead of easing: the host teleported
/// the character (void reset) or the prediction is hopelessly off.
const SNAP_ERROR: f32 = 6.0;

/// How fast a correction is eased out (per second, exponential): about 90%
/// of it within a quarter of a second.
const CORRECTION_RATE: f32 = 10.0;

/// How fast a standing character turns to the host's facing (attacks and
/// abilities turn the character on the host).
const ROTATION_FOLLOW_RATE: f32 = 12.0;

/// Sent inputs remembered (about 4 seconds at 60 messages a second).
const HISTORY_LENGTH: usize = 256;

// ---------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------

/// Host: publishes `NetworkMovementAck` for every joined player's body.
pub struct HostPredictionPlugin;

impl Plugin for HostPredictionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            publish_movement_acks.run_if(in_state(AppState::InGame)),
        );
    }
}

fn publish_movement_acks(
    mut commands: Commands,
    mut bodies: Query<
        (
            Entity,
            &RemoteIntentState,
            Option<&CharacterCombatModifiers>,
            Option<&StatBuffs>,
            Option<&mut NetworkMovementAck>,
        ),
        With<RemotePlayerBody>,
    >,
) {
    for (entity, intent_state, modifiers, stat_buffs, ack) in &mut bodies {
        let wanted = NetworkMovementAck {
            input_sequence: intent_state.last_sequence(),
            speed_percent: speed_to_percent(movement_speed_multiplier(modifiers, stat_buffs)),
        };
        match ack {
            Some(mut ack) => {
                ack.set_if_neq(wanted);
            }
            None => {
                commands.entity(entity).insert(wanted);
            }
        }
    }
}

/// A speed multiplier in whole percent (so tiny float changes aren't sent).
fn speed_to_percent(multiplier: f32) -> u16 {
    if !multiplier.is_finite() {
        return 100;
    }
    (multiplier * 100.0).round().clamp(0.0, u16::MAX as f32) as u16
}

// ---------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------

/// Client: predicts and corrects its own avatar.
pub struct ClientPredictionPlugin;

impl Plugin for ClientPredictionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (begin_predicting_own_avatar, reconcile_own_avatar)
                .chain()
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            RunFixedMainLoop,
            // After this frame's input reaches the avatar's intent, before
            // the ticks that move it.
            incapacitate_own_avatar
                .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop)
                .after(PlayerInputSet),
        )
        .add_systems(
            FixedUpdate,
            ease_own_avatar_toward_host.before(PlayerMovementSet),
        );
    }
}

/// The client's record of its own predictions and pending correction.
#[derive(Component, Default, Debug)]
pub struct OwnPrediction {
    /// Inputs sent and not yet acknowledged, oldest first.
    sent: VecDeque<SentInput>,
    /// Correction still to be eased into the avatar's position.
    pending: Vec3,
    /// The last acknowledged sequence already compared.
    last_checked: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SentInput {
    sequence: u32,
    /// Where the avatar was when this input was sent, plus any correction
    /// still pending then: where it *should* have been.
    expected: Vec3,
}

/// What `OwnPrediction::reconcile` decided.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Reconciliation {
    /// Nothing to do (close enough, already checked, or not a recorded
    /// input).
    None,
    /// The difference is being eased out.
    Ease(Vec3),
    /// Too far apart: jump to the host's position.
    Snap,
}

impl OwnPrediction {
    /// Called by `net::intent` for every message sent.
    pub fn record_sent_input(&mut self, sequence: u32, position: Vec3) {
        self.sent.push_back(SentInput {
            sequence,
            expected: position + self.pending,
        });
        while self.sent.len() > HISTORY_LENGTH {
            self.sent.pop_front();
        }
    }

    /// Compares the host's position after input `sequence` with ours.
    fn reconcile(&mut self, sequence: u32, host_position: Vec3) -> Reconciliation {
        if self.last_checked == Some(sequence) {
            return Reconciliation::None;
        }
        self.last_checked = Some(sequence);

        let Some(index) = self.sent.iter().position(|sent| sent.sequence == sequence) else {
            return Reconciliation::None;
        };
        let error = host_position - self.sent[index].expected;
        // This input and everything before it are settled.
        self.sent.drain(..=index);

        let distance = error.length();
        if !distance.is_finite() || distance <= IGNORED_ERROR {
            return Reconciliation::None;
        }
        if distance > SNAP_ERROR {
            self.sent.clear();
            self.pending = Vec3::ZERO;
            return Reconciliation::Snap;
        }
        // Inputs sent since were predicted from the uncorrected position, so
        // they carry the same error; shift them, so it isn't corrected twice.
        for later in &mut self.sent {
            later.expected += error;
        }
        self.pending += error;
        Reconciliation::Ease(error)
    }

    /// The part of the pending correction to apply this tick.
    fn take_correction_step(&mut self, delta_seconds: f32) -> Vec3 {
        if self.pending.length_squared() < 1e-8 {
            self.pending = Vec3::ZERO;
            return Vec3::ZERO;
        }
        let blend = 1.0 - (-CORRECTION_RATE * delta_seconds).exp();
        let step = self.pending * blend;
        self.pending -= step;
        step
    }
}

/// Turns this client's own avatar into a predicted character once it has a
/// `Transform` (from `avatars::attach_avatar_visual`).
fn begin_predicting_own_avatar(
    mut commands: Commands,
    avatars: Query<Entity, (With<OwnAvatar>, With<Transform>, Without<OwnPrediction>)>,
    settings: Res<NetworkSettings>,
) {
    for avatar in &avatars {
        commands.entity(avatar).insert((
            player_character_body(),
            PlayerIntent::default(),
            LocallyDriven,
            OwnPrediction::default(),
        ));
        if settings.debug {
            info!("[net] predicting our own character locally");
        }
    }
}

/// Copies the host's speed multiplier, and checks each newly acknowledged
/// input against the host's position.
fn reconcile_own_avatar(
    mut commands: Commands,
    mut avatars: Query<
        (
            Entity,
            &NetworkPose,
            &NetworkMovementAck,
            &mut OwnPrediction,
            &mut Transform,
            &mut VerticalMovement,
            Option<&PredictedMovementSpeed>,
        ),
        With<LocallyDriven>,
    >,
    settings: Res<NetworkSettings>,
) {
    for (entity, pose, ack, mut prediction, mut transform, mut vertical, speed) in &mut avatars {
        let wanted_speed = PredictedMovementSpeed(f32::from(ack.speed_percent) / 100.0);
        if speed != Some(&wanted_speed) {
            commands.entity(entity).insert(wanted_speed);
        }

        match prediction.reconcile(ack.input_sequence, pose.position()) {
            Reconciliation::None => {}
            Reconciliation::Ease(error) => {
                if settings.debug {
                    info!(
                        "[net] prediction off by {:.2} units; easing toward the host",
                        error.length()
                    );
                }
            }
            Reconciliation::Snap => {
                // Written outside the fixed schedule, so Avian's
                // interpolation treats it as a teleport.
                *transform = pose.to_transform();
                *vertical = VerticalMovement::default();
                if settings.debug {
                    info!("[net] prediction too far from the host; snapped");
                }
            }
        }
    }
}

/// A downed or lost character can't move; the host decides when, the
/// client's avatar follows (`LifeState` is replicated).
fn incapacitate_own_avatar(
    mut avatars: Query<(&LifeState, &mut PlayerIntent), With<LocallyDriven>>,
) {
    for (life, mut intent) in &mut avatars {
        let incapacitated = !life.is_alive();
        if intent.incapacitated != incapacitated {
            intent.incapacitated = incapacitated;
        }
    }
}

/// Each tick, before movement: eases in part of any pending correction and,
/// while standing still, turns toward the host's facing. Done in the fixed
/// schedule so Avian's interpolation keeps it smooth on screen.
fn ease_own_avatar_toward_host(
    time: Res<Time>,
    mut avatars: Query<
        (
            &mut OwnPrediction,
            &mut Transform,
            &PlayerIntent,
            &NetworkPose,
        ),
        With<LocallyDriven>,
    >,
) {
    let delta = time.delta_secs();
    for (mut prediction, mut transform, intent, pose) in &mut avatars {
        let step = prediction.take_correction_step(delta);
        if step != Vec3::ZERO {
            transform.translation += step;
        }

        if !intent.is_moving() {
            let host_rotation = pose.rotation();
            if transform.rotation.dot(host_rotation).abs() < 0.9999 {
                let blend = 1.0 - (-ROTATION_FOLLOW_RATE * delta).exp();
                transform.rotation = transform.rotation.slerp(host_rotation, blend);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prediction_with(inputs: &[(u32, Vec3)]) -> OwnPrediction {
        let mut prediction = OwnPrediction::default();
        for (sequence, position) in inputs {
            prediction.record_sent_input(*sequence, *position);
        }
        prediction
    }

    #[test]
    fn small_differences_are_ignored() {
        let mut prediction = prediction_with(&[(1, Vec3::ZERO), (2, Vec3::X)]);
        assert_eq!(
            prediction.reconcile(1, Vec3::new(0.2, 0.0, 0.0)),
            Reconciliation::None
        );
        assert_eq!(prediction.pending, Vec3::ZERO);
        // Input 1 is settled; input 2 is still waiting.
        assert_eq!(prediction.sent.len(), 1);
    }

    #[test]
    fn a_difference_is_corrected_once() {
        let mut prediction = prediction_with(&[(1, Vec3::ZERO), (2, Vec3::X), (3, Vec3::X * 2.0)]);
        // The host is 1 unit to the side at input 1...
        let offset = Vec3::new(0.0, 0.0, 1.0);
        assert_eq!(
            prediction.reconcile(1, offset),
            Reconciliation::Ease(offset)
        );
        assert_eq!(prediction.pending, offset);
        // ...and still 1 unit to the side at input 2: already being corrected.
        assert_eq!(
            prediction.reconcile(2, Vec3::X + offset),
            Reconciliation::None
        );
        assert_eq!(prediction.pending, offset);
    }

    #[test]
    fn inputs_sent_during_a_correction_expect_the_corrected_position() {
        let mut prediction = prediction_with(&[(1, Vec3::ZERO)]);
        let offset = Vec3::new(0.0, 0.0, 1.0);
        prediction.reconcile(1, offset);
        // One tick's worth of the correction applied so far.
        let avatar = prediction.take_correction_step(1.0 / 64.0);
        assert!(avatar.length() > 0.0 && avatar.length() < offset.length());
        prediction.record_sent_input(2, avatar);
        // Host agrees with the fully corrected position: nothing more to do.
        assert_eq!(prediction.reconcile(2, offset), Reconciliation::None);
    }

    #[test]
    fn corrections_ease_out_completely() {
        let mut prediction = prediction_with(&[(1, Vec3::ZERO)]);
        let offset = Vec3::new(2.0, 0.0, 0.0);
        prediction.reconcile(1, offset);
        let mut applied = Vec3::ZERO;
        for _ in 0..200 {
            applied += prediction.take_correction_step(1.0 / 64.0);
        }
        assert!(applied.distance(offset) < 1e-3);
        assert!(prediction.pending.length() < 1e-3);
    }

    #[test]
    fn huge_differences_snap() {
        let mut prediction = prediction_with(&[(1, Vec3::ZERO), (2, Vec3::X)]);
        assert_eq!(
            prediction.reconcile(1, Vec3::new(0.0, -20.0, 0.0)),
            Reconciliation::Snap
        );
        assert!(prediction.sent.is_empty());
        assert_eq!(prediction.pending, Vec3::ZERO);
    }

    #[test]
    fn unknown_or_repeated_sequences_are_ignored() {
        let mut prediction = prediction_with(&[(5, Vec3::ZERO)]);
        assert_eq!(prediction.reconcile(4, Vec3::X * 3.0), Reconciliation::None);
        assert_eq!(prediction.sent.len(), 1);
        assert_eq!(
            prediction.reconcile(5, Vec3::X * 3.0),
            Reconciliation::Ease(Vec3::X * 3.0)
        );
        assert_eq!(prediction.reconcile(5, Vec3::X * 3.0), Reconciliation::None);
    }

    #[test]
    fn speed_is_sent_in_whole_percent() {
        assert_eq!(speed_to_percent(1.0), 100);
        assert_eq!(speed_to_percent(1.234), 123);
        assert_eq!(speed_to_percent(f32::NAN), 100);
        assert_eq!(speed_to_percent(-1.0), 0);
    }
}
