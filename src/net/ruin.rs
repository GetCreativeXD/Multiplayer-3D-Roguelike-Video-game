//! The run's shared Ruin, seen by every player (Milestone 7 step 4).
//!
//! Ruin is one run-wide value for the whole party (`ruin::RuinState`).
//! Only the host changes it: passive growth, the starting Ruin condition,
//! any player's Gravemind Embrace, The Ruinpiercer's Cleansing Strike.
//!
//! - **Host:** keeps one replicated entity carrying a `ruin::RuinSnapshot`
//!   (Ruin to a tenth, the run clock in whole seconds, whether the "The
//!   Ruin is spreading..." warning is up), rewritten only when it changes.
//!   The run teardown removes it when the run ends.
//! - **Client:** doesn't run its own Ruin clock (`ruin::RuinPlugin`) and
//!   copies each new snapshot into its `RuinState`, so the warning banner,
//!   the Tab screen and the run timer match the host.

use bevy::prelude::*;
use lightyear::prelude::*;

use crate::game_state::AppState;
use crate::ruin::{RuinSnapshot, RuinState};

// ---------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------

pub struct HostRuinPlugin;

impl Plugin for HostRuinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, publish_ruin.run_if(in_state(AppState::InGame)));
    }
}

fn publish_ruin(
    mut commands: Commands,
    ruin_state: Res<RuinState>,
    mut published: Query<&mut RuinSnapshot>,
) {
    let wanted = ruin_state.snapshot();
    if published.is_empty() {
        commands.spawn((
            Name::new("Shared Ruin"),
            wanted,
            Replicate::to_clients(NetworkTarget::All),
        ));
        return;
    }
    for mut snapshot in &mut published {
        snapshot.set_if_neq(wanted);
    }
}

// ---------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------

pub struct ClientRuinPlugin;

impl Plugin for ClientRuinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            apply_ruin_from_host.run_if(in_state(AppState::InGame)),
        );
    }
}

fn apply_ruin_from_host(
    snapshots: Query<&RuinSnapshot, Changed<RuinSnapshot>>,
    mut ruin_state: ResMut<RuinState>,
) {
    for snapshot in &snapshots {
        ruin_state.apply_snapshot(snapshot);
    }
}
