//! Ruin: the run-based corruption resource described in
//! `RUIN_CORRUPTION_DESIGN.md`. This module owns the number itself --
//! its 0-100 clamp, its passive growth over time, and the threshold
//! bands the rest of the game will react to -- plus the run clock that
//! passive growth (and the Tab screen's timer readout, `ui::ruin_screen`)
//! are both driven by.
//!
//! # What's implemented now vs. later
//!
//! This first pass wires up the resource, its passive time-based growth,
//! and the Ruinbreak reset at 100 -- enough for the Tab screen to show
//! something real and for future systems to have a stable API to build
//! on. Everything else the design doc describes (corrupted enemies and
//! rooms, shrines, cursed rewards, choice events, the full Ruinbreak
//! event) reads and writes this same `RuinState` through `add_ruin` /
//! `remove_ruin`, but is intentionally not built yet -- each is its own
//! follow-up mechanic, per the project's "confirm one mechanic before
//! moving to the next" workflow.
//!
//! Any future system that grants or costs Ruin should go through
//! `RuinState::add_ruin` / `RuinState::remove_ruin` rather than touching
//! the underlying number directly, so the 0-100 clamp and the Ruinbreak
//! trigger stay in one place.

use bevy::prelude::*;

use crate::game_state::AppState;
use crate::run_setup::RunModifiers;

// =========================================================
// CONSTANTS
// =========================================================

pub const RUIN_MIN: f32 = 0.0;
pub const RUIN_MAX: f32 = 100.0;

/// Where Ruin settles after a Ruinbreak (design doc: "Ruin drops to
/// approximately 60-70").
const RUINBREAK_RESET_RUIN: f32 = 65.0;

/// Shown on-screen whenever passive growth ticks, per the design doc's
/// "Before each passive increase, the game should display a warning".
const PASSIVE_GROWTH_WARNING: &str = "The Ruin is spreading...";
const PASSIVE_GROWTH_WARNING_SECONDS: f32 = 3.0;

/// How much Ruin one passive tick adds. Matches every row of the design
/// doc's time-scaling table ("+1 Ruin every ...") -- only the interval
/// between ticks changes as the run goes on, not the amount.
const PASSIVE_GROWTH_AMOUNT: f32 = 1.0;

// =========================================================
// RUIN STATE
// =========================================================

/// The player's current Ruin and the run clock it's tied to.
///
/// Ruin itself is always in `RUIN_MIN..=RUIN_MAX`; nothing outside this
/// module should need to clamp it again. `run_elapsed_seconds` runs
/// continuously in real time -- see `advance_run_clock_and_ruin` --
/// regardless of whether the map, inventory, or Tab screen is open. The
/// design doc mentions pausing the timer in menus/shops/safe rooms, but
/// the confirmed behavior for this project is that opening any current
/// menu does not pause Ruin or the run clock; revisit this if a real
/// shop or safe-room mechanic is added later and needs it.
#[derive(Resource)]
pub struct RuinState {
    ruin: f32,
    run_elapsed_seconds: f32,
    /// Seconds accumulated toward the next passive Ruin tick. Reset (with
    /// leftover carried forward) every time a tick fires.
    passive_timer: f32,
    /// Counts down while the passive-growth warning banner is visible;
    /// zero means nothing to show. See `ui::update_ruin_warning_banner`.
    warning_remaining: f32,
}

impl Default for RuinState {
    fn default() -> Self {
        Self {
            ruin: RUIN_MIN,
            run_elapsed_seconds: 0.0,
            passive_timer: 0.0,
            warning_remaining: 0.0,
        }
    }
}

impl RuinState {
    /// Current Ruin, always within `RUIN_MIN..=RUIN_MAX`.
    pub fn ruin(&self) -> f32 {
        self.ruin
    }

    /// Current Ruin rounded to the nearest whole point, for display.
    pub fn ruin_rounded(&self) -> i32 {
        self.ruin.round() as i32
    }

    /// Total in-run seconds the clock has ticked (excludes time spent
    /// with a menu open). See `RuinState` docs.
    pub fn run_elapsed_seconds(&self) -> f32 {
        self.run_elapsed_seconds
    }

    /// The threshold band the current Ruin falls into (design doc
    /// section "Ruin Thresholds").
    pub fn threshold(&self) -> RuinThreshold {
        RuinThreshold::for_ruin(self.ruin)
    }

    /// The passive-growth warning text, if the banner should currently
    /// be showing.
    pub fn warning_message(&self) -> Option<&'static str> {
        (self.warning_remaining > 0.0).then_some(PASSIVE_GROWTH_WARNING)
    }

    /// Raises Ruin by `amount`, clamped to `RUIN_MAX`. Reaching `RUIN_MAX`
    /// triggers a Ruinbreak (see `trigger_ruinbreak`). This is the
    /// intended entry point for every future Ruin-granting mechanic --
    /// exploration choices, cursed rewards, corrupted abilities, and so
    /// on -- not just passive growth.
    pub fn add_ruin(&mut self, amount: f32) {
        if amount <= 0.0 {
            return;
        }

        self.ruin = (self.ruin + amount).min(RUIN_MAX);

        if self.ruin >= RUIN_MAX {
            self.trigger_ruinbreak();
        }
    }

    /// Lowers Ruin by `amount`, clamped to `RUIN_MIN`. The intended entry
    /// point for every future cleansing mechanic (shrines, cleansing
    /// altars, purification rooms, and so on).
    pub fn remove_ruin(&mut self, amount: f32) {
        if amount <= 0.0 {
            return;
        }

        self.ruin = (self.ruin - amount).max(RUIN_MIN);
    }

    /// Ruin cannot directly kill the player (design doc, "Core Rules"):
    /// reaching 100 instead triggers a dramatic event and resets Ruin
    /// down to the 60-70 band so the run continues.
    ///
    /// The full Ruinbreak event (a Corrupted Elite invasion, a survival
    /// encounter, a pocket-realm detour, and the temporary "Ruinbreak
    /// Scar" that follows it) is a future mechanic -- this only applies
    /// the documented aftermath so hitting 100 doesn't get the player
    /// stuck at the cap.
    fn trigger_ruinbreak(&mut self) {
        self.ruin = RUINBREAK_RESET_RUIN;
        info!(
            "Ruinbreak triggered (full event not yet implemented) -- Ruin reset to {}",
            RUINBREAK_RESET_RUIN
        );
    }
}

// =========================================================
// SHARED SNAPSHOT (co-op)
// =========================================================

/// What a player who doesn't simulate the run needs to know about Ruin:
/// the shared value, the run clock and whether the passive-growth warning
/// is up. Ruin is one run-wide value for the whole party. In multiplayer
/// the host publishes this on a replicated entity whenever it changes, and
/// a joined client copies it into its own `RuinState` (`net::ruin`)
/// instead of running its own clock, so every HUD, the Tab screen and
/// anything that reads Ruin agree with the host.
///
/// Ruin is kept to a tenth of a point and the clock to whole seconds, so
/// continuous gains (a Gravemind's Embrace) don't resend it every frame.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
pub struct RuinSnapshot {
    pub ruin_tenths: u16,
    pub run_seconds: u32,
    pub warning: bool,
}

// Only the multiplayer build sends and receives snapshots.
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
impl RuinState {
    /// This state as a `RuinSnapshot`.
    pub fn snapshot(&self) -> RuinSnapshot {
        RuinSnapshot {
            ruin_tenths: (self.ruin.clamp(RUIN_MIN, RUIN_MAX) * 10.0).round() as u16,
            run_seconds: self.run_elapsed_seconds.max(0.0) as u32,
            warning: self.warning_remaining > 0.0,
        }
    }

    /// Takes the simulating machine's values. Bypasses `add_ruin` on
    /// purpose: the host already applied the clamp and any Ruinbreak.
    pub fn apply_snapshot(&mut self, snapshot: &RuinSnapshot) {
        self.ruin = (snapshot.ruin_tenths as f32 / 10.0).clamp(RUIN_MIN, RUIN_MAX);
        self.run_elapsed_seconds = snapshot.run_seconds as f32;
        self.warning_remaining = if snapshot.warning {
            PASSIVE_GROWTH_WARNING_SECONDS
        } else {
            0.0
        };
    }
}

// =========================================================
// THRESHOLDS
// =========================================================

/// One of the six Ruin bands from the design doc's "Ruin Thresholds"
/// table. Purely descriptive on its own -- nothing here changes gameplay
/// by itself, the same way `rooms::components::RoomKind` doesn't. Future
/// systems (corrupted enemy spawn rates, room mutation odds, reward
/// tables) should match on this rather than re-deriving their own bands
/// from the raw Ruin number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuinThreshold {
    /// 0-19.
    Unmarked,
    /// 20-39.
    Touched,
    /// 40-59.
    Stained,
    /// 60-79.
    Corrupted,
    /// 80-99.
    Doomed,
    /// 100.
    Ruinbreak,
}

impl RuinThreshold {
    pub fn for_ruin(ruin: f32) -> Self {
        match ruin.round() as i32 {
            i32::MIN..=19 => Self::Unmarked,
            20..=39 => Self::Touched,
            40..=59 => Self::Stained,
            60..=79 => Self::Corrupted,
            80..=99 => Self::Doomed,
            _ => Self::Ruinbreak,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Unmarked => "Unmarked",
            Self::Touched => "Touched",
            Self::Stained => "Stained",
            Self::Corrupted => "Corrupted",
            Self::Doomed => "Doomed",
            Self::Ruinbreak => "Ruinbreak",
        }
    }

    /// Accent color for this threshold -- used by the Tab screen today
    /// and available for any future HUD element that wants to
    /// color-code current Ruin severity, mirroring
    /// `ui::map::kind_accent_color`'s per-category palette.
    pub fn accent_color(self) -> Color {
        match self {
            Self::Unmarked => Color::srgb(0.72, 0.72, 0.72),
            Self::Touched => Color::srgb(0.62, 0.78, 0.55),
            Self::Stained => Color::srgb(0.85, 0.75, 0.25),
            Self::Corrupted => Color::srgb(0.85, 0.55, 0.20),
            Self::Doomed => Color::srgb(0.85, 0.25, 0.25),
            Self::Ruinbreak => Color::srgb(0.75, 0.15, 0.85),
        }
    }
}

// =========================================================
// PASSIVE GROWTH + RUN CLOCK
// =========================================================

/// Seconds between passive Ruin ticks for a given amount of run time so
/// far. Matches the design doc's "Suggested time scaling" table -- the
/// dungeon becomes unstable faster the longer the run goes on.
fn passive_interval_seconds(run_elapsed_seconds: f32) -> f32 {
    const TEN_MINUTES: f32 = 10.0 * 60.0;
    const TWENTY_MINUTES: f32 = 20.0 * 60.0;

    if run_elapsed_seconds < TEN_MINUTES {
        90.0
    } else if run_elapsed_seconds < TWENTY_MINUTES {
        75.0
    } else {
        60.0
    }
}

/// Advances the run clock and applies passive Ruin growth (design doc
/// "Passive Ruin Growth"). Runs unconditionally every frame -- see
/// `RuinState`'s docs for why opening the map, inventory, or Tab screen
/// doesn't pause this.
pub(crate) fn advance_run_clock_and_ruin(
    time: Res<Time>,
    run_modifiers: Res<RunModifiers>,
    mut ruin_state: ResMut<RuinState>,
) {
    let delta = time.delta_secs();

    if ruin_state.warning_remaining > 0.0 {
        ruin_state.warning_remaining = (ruin_state.warning_remaining - delta).max(0.0);
    }

    ruin_state.run_elapsed_seconds += delta;
    ruin_state.passive_timer += delta;

    let interval = passive_interval_seconds(ruin_state.run_elapsed_seconds);
    if ruin_state.passive_timer >= interval {
        ruin_state.passive_timer -= interval;
        ruin_state.warning_remaining = PASSIVE_GROWTH_WARNING_SECONDS;
        // Scaled by the run's Ruin-gain relic/pact effects (e.g. Cleansed
        // Fragment, Corruption's Price) -- see
        // `run_setup::RunModifiers::ruin_gain_multiplier`.
        ruin_state.add_ruin(PASSIVE_GROWTH_AMOUNT * run_modifiers.ruin_gain_multiplier);
    }
}

// =========================================================
// PLUGIN
// =========================================================

pub struct RuinPlugin;

impl Plugin for RuinPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RuinState>().add_systems(
            OnExit(AppState::InGame),
            crate::game_state::reset_resource::<RuinState>,
        );
        // Unconditional across every in-run menu (map/inventory/Ruin
        // screen -- see the module doc comment above), but *not* before
        // a run exists: without this gate the passive-growth clock would
        // start counting on the run-setup screen, before the player has
        // even confirmed a starting Ruin condition
        // (run_setup::catalog::RUIN_CONDITIONS) for it to add onto.
        //
        // A joined co-op client doesn't run the clock: it shows the host's
        // Ruin (`RuinSnapshot`, `net::ruin`).
        app.add_systems(
            Update,
            advance_run_clock_and_ruin
                .run_if(in_state(AppState::InGame))
                .run_if(crate::net::has_simulation_authority),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snapshot_round_trips_ruin_clock_and_warning() {
        let mut host = RuinState::default();
        host.add_ruin(42.34);
        host.run_elapsed_seconds = 125.7;
        host.warning_remaining = 1.0;

        let mut client = RuinState::default();
        client.apply_snapshot(&host.snapshot());

        assert!((client.ruin() - 42.3).abs() < 1e-4);
        assert_eq!(client.ruin_rounded(), 42);
        assert_eq!(client.run_elapsed_seconds(), 125.0);
        assert!(client.warning_message().is_some());

        host.warning_remaining = 0.0;
        client.apply_snapshot(&host.snapshot());
        assert!(client.warning_message().is_none());
    }
}
