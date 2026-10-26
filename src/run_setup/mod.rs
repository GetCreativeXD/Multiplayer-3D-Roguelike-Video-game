//! The player's confirmed run-setup choices: class, starting weapon, item
//! drop pool, starting relics, run pacts, and starting Ruin condition. See
//! `catalog` for the menu-facing data (names/descriptions) behind each
//! choice, `effects` for how relics/pacts turn into real numbers, and
//! `ui::run_setup_screen` for the screen that lets the player make them.
//!
//! `RunConfig` is the single source of truth `OnEnter(AppState::InGame)`
//! setup reads from (see
//! `rooms::fallen_gate::setup_fallen_gate_player`) -- nothing
//! about it is Fallen-Gate-specific, so a future alternate run-start room
//! can read the exact same resource.

pub mod catalog;
pub mod effects;

// `ui::run_setup_screen` reaches the catalog's types and lookup
// functions through `catalog::` directly rather than through re-exports
// here -- only the identifiers `RunConfig` itself is built from
// (`DropPoolCategoryId`, `PactId`, `RelicId`, `RuinConditionId`) need to
// be nameable from outside this module.
pub use catalog::{DropPoolCategoryId, PactId, RelicId, RuinConditionId};
pub use effects::{RunEffect, RunModifiers};

use bevy::prelude::*;

use crate::characters::ClassId;
use crate::weapons::WeaponType;

pub struct RunSetupPlugin;

impl Plugin for RunSetupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RunConfig>()
            .init_resource::<RunModifiers>()
            // Recomputed from `RunConfig` when the next run begins.
            .add_systems(
                OnExit(crate::game_state::AppState::InGame),
                crate::game_state::reset_resource::<RunModifiers>,
            );
    }
}

/// A run-setup category where the player can select any number of
/// options at once -- item-drop-pool categories, starting relics, and run
/// pacts all use this same shape (see `RunConfig` below). `T` is a small
/// `Copy` identifier enum (`DropPoolCategoryId`, `RelicId`, `PactId`), so
/// adding a new selectable option to any of the three menus is a new
/// catalog entry, never a new selection type or a change to this one.
#[derive(Clone, Debug)]
pub struct MultiSelect<T> {
    selected: Vec<T>,
}

impl<T: Copy + PartialEq> MultiSelect<T> {
    /// Nothing selected -- the default a fresh relic/pact choice starts
    /// with.
    pub fn none() -> Self {
        Self {
            selected: Vec::new(),
        }
    }

    /// Every option in `options` selected -- used for the item-drop-pool
    /// selector, which starts with every category enabled.
    pub fn all(options: impl IntoIterator<Item = T>) -> Self {
        Self {
            selected: options.into_iter().collect(),
        }
    }

    pub fn is_selected(&self, id: T) -> bool {
        self.selected.contains(&id)
    }

    /// Flips one option on or off.
    pub fn toggle(&mut self, id: T) {
        if let Some(index) = self.selected.iter().position(|selected| *selected == id) {
            self.selected.remove(index);
        } else {
            self.selected.push(id);
        }
    }

    pub fn count(&self) -> usize {
        self.selected.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.selected.iter()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.selected
    }

    /// Exactly `selected`, in that order.
    #[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
    pub fn from_slice(selected: &[T]) -> Self {
        Self {
            selected: selected.to_vec(),
        }
    }
}

impl<T: Copy + PartialEq> Default for MultiSelect<T> {
    fn default() -> Self {
        Self::none()
    }
}

/// The player's confirmed (or currently-being-edited -- the run-setup
/// screen writes into this resource live as the player clicks options,
/// rather than staging changes elsewhere) run-setup choices.
#[derive(Resource, Clone)]
pub struct RunConfig {
    pub class: ClassId,
    pub weapon: WeaponType,
    pub drop_pool: MultiSelect<DropPoolCategoryId>,
    /// Any number of relics may be active at once -- see `effects` for
    /// how multiple relics combine.
    pub relics: MultiSelect<RelicId>,
    /// Any number of pacts may be active at once, including none (a run
    /// pact is optional -- starting_menu.md).
    pub pacts: MultiSelect<PactId>,
    pub ruin_condition: RuinConditionId,
}

/// The run-setup choices the whole co-op party shares: item drop pool,
/// relics, pacts and Ruin condition. (Class and weapon stay per player.)
/// Any player can change them; `net::lobby` keeps every machine's copy the
/// same.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
pub struct SharedRunSettings {
    pub drop_pool: Vec<DropPoolCategoryId>,
    pub relics: Vec<RelicId>,
    pub pacts: Vec<PactId>,
    pub ruin_condition: RuinConditionId,
}

impl RunConfig {
    /// This config's party-wide choices.
    #[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
    pub fn shared_settings(&self) -> SharedRunSettings {
        SharedRunSettings {
            drop_pool: self.drop_pool.as_slice().to_vec(),
            relics: self.relics.as_slice().to_vec(),
            pacts: self.pacts.as_slice().to_vec(),
            ruin_condition: self.ruin_condition,
        }
    }

    /// Replaces this config's party-wide choices (class and weapon are
    /// left alone).
    #[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
    pub fn apply_shared_settings(&mut self, settings: &SharedRunSettings) {
        self.drop_pool = MultiSelect::from_slice(&settings.drop_pool);
        self.relics = MultiSelect::from_slice(&settings.relics);
        self.pacts = MultiSelect::from_slice(&settings.pacts);
        self.ruin_condition = settings.ruin_condition;
    }

    /// Whether the current drop-pool selection satisfies
    /// `catalog::MIN_ENABLED_DROP_POOL_CATEGORIES` -- the run-setup
    /// screen's "Begin Run" button is disabled while this is `false`.
    pub fn drop_pool_meets_minimum(&self) -> bool {
        self.drop_pool.count() >= catalog::MIN_ENABLED_DROP_POOL_CATEGORIES
    }
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            class: ClassId::Vanguard,
            weapon: WeaponType::Longsword,
            drop_pool: MultiSelect::all(catalog::DROP_POOL_CATEGORIES.iter().map(|c| c.id)),
            relics: MultiSelect::none(),
            pacts: MultiSelect::none(),
            ruin_condition: RuinConditionId::Stable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_settings_round_trip_and_leave_class_and_weapon_alone() {
        let mut host = RunConfig::default();
        host.relics.toggle(catalog::RELICS[0].id);
        host.ruin_condition = catalog::RUIN_CONDITIONS
            .iter()
            .find(|entry| entry.id != host.ruin_condition)
            .map_or(host.ruin_condition, |entry| entry.id);

        let mut guest = RunConfig {
            class: ClassId::Warden,
            weapon: WeaponType::Bow,
            ..RunConfig::default()
        };
        guest.apply_shared_settings(&host.shared_settings());

        assert_eq!(guest.shared_settings(), host.shared_settings());
        assert_eq!(guest.class, ClassId::Warden);
        assert_eq!(guest.weapon, WeaponType::Bow);
    }
}
