use bevy::prelude::*;

use crate::player::PlayerCharacter;

pub struct ClassAbilitiesPlugin;

impl Plugin for ClassAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, initialize_character_combat_state);
    }
}

/// Every player character (the local `Player` and, on a multiplayer host,
/// each joined player's body) gets the shared class-facing combat state.
fn initialize_character_combat_state(
    mut commands: Commands,
    players: Query<Entity, Added<PlayerCharacter>>,
) {
    for player in &players {
        commands.entity(player).insert((
            BasicAttackTracker::default(),
            BasicAttackHitTracker::default(),
            CharacterCombatModifiers::default(),
            AbilityUseTracker::default(),
        ));
    }
}

/// Counts basic attacks that actually connect with at least one enemy.
#[derive(Component, Default)]
pub struct BasicAttackHitTracker {
    total_confirmed: u64,
}

impl BasicAttackHitTracker {
    pub fn record_confirmed_hit(&mut self) {
        self.total_confirmed = self.total_confirmed.saturating_add(1);
    }

    pub fn total_confirmed(&self) -> u64 {
        self.total_confirmed
    }
}

#[derive(Component, Default)]
pub struct BasicAttackTracker {
    total_started: u64,
}

impl BasicAttackTracker {
    pub fn record_started_attack(&mut self) {
        self.total_started = self.total_started.saturating_add(1);
    }

    pub fn total_started(&self) -> u64 {
        self.total_started
    }
}

/// Counts abilities the player has successfully used: weapon abilities
/// (right-click, see `weapons::abilities::weapon_ability_input`) and class
/// abilities (e.g. Vanguard's Lunge and Afterimage in
/// `characters::vanguard_abilities`). Any current or future system that
/// reacts to "the player used an ability" -- gear effects chief among them
/// (see `weapons::armor_procs`) -- watches this counter the same way
/// Vanguard's Momentum watches `BasicAttackHitTracker`: record the
/// observed count, then compare each frame.
#[derive(Component, Default)]
pub struct AbilityUseTracker {
    total_used: u64,
}

impl AbilityUseTracker {
    pub fn record_ability_used(&mut self) {
        self.total_used = self.total_used.saturating_add(1);
    }

    pub fn total_used(&self) -> u64 {
        self.total_used
    }
}

/// Multipliers supplied by the currently selected class.
/// Shared movement and weapon code consume these without knowing the class.
#[derive(Component)]
pub struct CharacterCombatModifiers {
    pub attack_speed: f32,
    pub movement_speed: f32,
}

impl Default for CharacterCombatModifiers {
    fn default() -> Self {
        Self {
            attack_speed: 1.0,
            movement_speed: 1.0,
        }
    }
}

/// UI-facing state for one class ability.
///
/// Each character class owns its gameplay components and systems, then writes
/// their current display state into `ClassAbilityHud`. This keeps the shared
/// HUD independent of Vanguard, Runesage, Warden, or any future class.
#[derive(Clone, Copy)]
pub enum ClassAbilityDisplay {
    Ready,
    Cooldown(f32),
    Counter {
        current: u8,
        max: u8,
    },
    Active(&'static str),
    /// Generic "not usable right now" state with a short reason label,
    /// for abilities gated by something other than a cooldown timer --
    /// e.g. Runesage abilities gated by available Runes rather than time
    /// (see `characters::runesage_abilities`).
    Locked(&'static str),
}

#[derive(Clone, Copy)]
pub struct ClassAbilityHudSlot {
    pub input: &'static str,
    pub name: &'static str,
    pub display: ClassAbilityDisplay,
}

impl ClassAbilityHudSlot {
    pub const fn new(
        input: &'static str,
        name: &'static str,
        display: ClassAbilityDisplay,
    ) -> Self {
        Self {
            input,
            name,
            display,
        }
    }
}

/// Optional class-resource display (e.g. Runesage's Runes) shown
/// beneath the ability bar. Not every class has one -- add this
/// component only when the class has a discrete spendable resource to
/// show, the same way `ClassAbilityHud` is added once a class is
/// assigned. `ui::update_class_resource_bar` hides the bar entirely for
/// any player without this component.
#[derive(Component)]
pub struct ClassResourceHud {
    pub label: &'static str,
    pub current: u8,
    pub max: u8,
}

impl ClassResourceHud {
    pub const fn new(label: &'static str, current: u8, max: u8) -> Self {
        Self {
            label,
            current,
            max,
        }
    }
}

/// The three class-ability slots displayed by the shared bottom HUD.
///
/// Add this component when a class is assigned to a player. The class's own
/// systems are responsible for keeping the three slots current.
#[derive(Component)]
pub struct ClassAbilityHud {
    pub slots: [ClassAbilityHudSlot; 3],
}

impl ClassAbilityHud {
    pub const fn new(slots: [ClassAbilityHudSlot; 3]) -> Self {
        Self { slots }
    }
}
