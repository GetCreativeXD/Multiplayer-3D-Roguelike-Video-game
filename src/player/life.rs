//! Downed -> Lost (context doc section 7, CONFIRMED).
//!
//! - A player character at 0 health becomes **Downed**: it can't move,
//!   attack or cast, and enemies stop targeting it. A teammate who stays
//!   within `REVIVE_RADIUS` for `REVIVE_SECONDS` revives it with
//!   `REVIVE_HEALTH_FRACTION` of its max health. Reviving takes standing
//!   still beside them, so doing it mid-fight exposes the rescuer.
//! - If nobody revives it within `DOWNED_BLEED_OUT_SECONDS` it becomes
//!   **Lost** for the rest of the floor: hidden and out of play, but it
//!   keeps its character, gear and build.
//! - **Party wipe:** when no player character is Alive, nobody can revive
//!   anyone and the expedition has fallen. Solo play is a party wipe the
//!   moment the player goes down (section 7.4).
//!
//! A party wipe ends the run: after `PARTY_WIPE_RETURN_SECONDS` (so the
//! banner can be read) everyone goes back to the run-setup screen
//! (`AppState::RunSetup`), which tears the run's world down (see
//! `game_state::RunLifecyclePlugin`). In co-op the host decides this and
//! tells every client (`net::lobby`).
//!
//! Not yet: **Returned** (lost players come back at the start of the next
//! level) needs level transitions.
//!
//! The simulating machine (offline or the host) decides every state change
//! (`update_life_states`). In multiplayer `LifeState` is replicated, so
//! every machine can show it (`present_life_states`, the HUD banner).

use bevy::prelude::*;

use crate::characters::animation::CharacterAnimationOwner;
use crate::combat::Health;
use crate::game_state::AppState;

use super::{PlayerCharacter, PlayerInputSet, PlayerIntent};

/// How long a downed player can wait for a revive before being lost.
/// The context doc's early idea was 20-30 s; tuning value.
pub const DOWNED_BLEED_OUT_SECONDS: f32 = 25.0;
/// How long a teammate must stay beside a downed player to revive them.
pub const REVIVE_SECONDS: f32 = 3.0;
/// How close (horizontally) a teammate must stand to revive.
pub const REVIVE_RADIUS: f32 = 2.2;
/// Revived players come back with this fraction of their max health.
pub const REVIVE_HEALTH_FRACTION: f32 = 0.3;
/// After a party wipe, how long "THE EXPEDITION HAS FALLEN" stays up
/// before everyone returns to the run-setup screen.
pub const PARTY_WIPE_RETURN_SECONDS: f32 = 6.0;

/// Where a player character is in the Downed -> Lost flow. Required by
/// `PlayerCharacter`, so every player character has one.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum LifeState {
    #[default]
    Alive,
    Downed {
        /// Seconds until this player is lost.
        bleed_out_remaining: f32,
        /// Seconds of revive accumulated so far (out of `REVIVE_SECONDS`).
        revive_progress: f32,
    },
    /// Out of play for the rest of the floor.
    Lost,
}

impl LifeState {
    pub fn is_alive(&self) -> bool {
        matches!(self, LifeState::Alive)
    }
}

pub struct PlayerLifePlugin;

impl Plugin for PlayerLifePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            RunFixedMainLoop,
            // In the input set (before the fixed-tick movement), so a downed
            // character's presses and movement are ignored from the next
            // frame's input on.
            incapacitate_fallen_players
                .in_set(PlayerInputSet)
                .run_if(crate::net::has_simulation_authority),
        )
        .add_systems(
            Update,
            (
                update_life_states.run_if(crate::net::has_simulation_authority),
                end_run_after_party_wipe
                    .run_if(crate::net::has_simulation_authority)
                    .run_if(in_state(AppState::InGame)),
                present_life_states,
            ),
        );
    }
}

/// A character that isn't Alive can't move, jump, dodge, attack or cast
/// (see `PlayerIntent::incapacitated`).
fn incapacitate_fallen_players(
    mut players: Query<(&LifeState, &mut PlayerIntent), With<PlayerCharacter>>,
) {
    for (life, mut intent) in &mut players {
        let incapacitated = !life.is_alive();
        if intent.incapacitated != incapacitated {
            intent.incapacitated = incapacitated;
        }
    }
}

fn update_life_states(
    time: Res<Time>,
    mut players: Query<(Entity, &Transform, &mut Health, &mut LifeState), With<PlayerCharacter>>,
) {
    let delta = time.delta_secs();

    // Health reaching zero downs a player.
    for (_, _, health, mut life) in &mut players {
        if life.is_alive() && health.current <= 0.0 {
            *life = LifeState::Downed {
                bleed_out_remaining: DOWNED_BLEED_OUT_SECONDS,
                revive_progress: 0.0,
            };
            println!("A player is DOWNED!");
        }
    }

    // Who can revive: every player still standing.
    let rescuers: Vec<(Entity, Vec3)> = players
        .iter()
        .filter(|(_, _, _, life)| life.is_alive())
        .map(|(entity, transform, _, _)| (entity, transform.translation))
        .collect();

    for (entity, transform, mut health, mut life) in &mut players {
        let LifeState::Downed {
            bleed_out_remaining,
            revive_progress,
        } = *life
        else {
            continue;
        };

        let being_revived = rescuers.iter().any(|(rescuer, position)| {
            let mut offset = *position - transform.translation;
            offset.y = 0.0;
            *rescuer != entity && offset.length() <= REVIVE_RADIUS
        });

        let revive_progress = if being_revived {
            revive_progress + delta
        } else {
            // Stepping away loses progress, but not all at once.
            (revive_progress - delta).max(0.0)
        };

        if revive_progress >= REVIVE_SECONDS {
            health.current = (health.max * REVIVE_HEALTH_FRACTION).max(1.0);
            *life = LifeState::Alive;
            println!("A player was REVIVED!");
            continue;
        }

        let bleed_out_remaining = bleed_out_remaining - delta;
        if bleed_out_remaining <= 0.0 {
            *life = LifeState::Lost;
            println!("PLAYER LOST TO THE RUINS");
            continue;
        }

        *life = LifeState::Downed {
            bleed_out_remaining,
            revive_progress,
        };
    }
}

/// Nobody standing: wait `PARTY_WIPE_RETURN_SECONDS`, then end the run.
fn end_run_after_party_wipe(
    time: Res<Time>,
    players: Query<&LifeState, With<PlayerCharacter>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut seconds_wiped: Local<f32>,
) {
    let wiped = !players.is_empty() && !players.iter().any(LifeState::is_alive);
    if !wiped {
        *seconds_wiped = 0.0;
        return;
    }

    *seconds_wiped += time.delta_secs();
    if *seconds_wiped >= PARTY_WIPE_RETURN_SECONDS {
        *seconds_wiped = 0.0;
        println!("The expedition has fallen. Returning to run setup.");
        next_state.set(AppState::RunSetup);
    }
}

/// On every machine: a downed player's model lies on the ground, and a
/// lost player is hidden.
fn present_life_states(
    mut characters: Query<(&LifeState, Option<&mut Visibility>), Changed<LifeState>>,
    all_states: Query<&LifeState>,
    // Only the model's root node (the one holding the scene). The model's
    // animation-player node inside the GLB also carries
    // `CharacterAnimationOwner`, and its own rotation must not be touched.
    mut models: Query<(&CharacterAnimationOwner, &mut Transform), With<WorldAssetRoot>>,
) {
    for (life, visibility) in &mut characters {
        if let Some(mut visibility) = visibility {
            let wanted = if matches!(life, LifeState::Lost) {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            visibility.set_if_neq(wanted);
        }
    }

    // Cheap: one model per player.
    for (owner, mut transform) in &mut models {
        let downed = all_states
            .get(owner.owner())
            .is_ok_and(|life| matches!(life, LifeState::Downed { .. }));
        let rotation = if downed {
            // Pivot at the feet: the model lies flat on the floor.
            Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
        } else {
            Quat::IDENTITY
        };
        if transform.rotation != rotation {
            transform.rotation = rotation;
        }
    }
}
