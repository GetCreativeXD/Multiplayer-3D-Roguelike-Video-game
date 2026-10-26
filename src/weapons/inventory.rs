use bevy::prelude::*;

use crate::combat::{BaseHealth, Health};
use crate::player::PlayerCharacter;
use crate::run_setup::RunModifiers;

use super::components::{Equipment, PlayerInventory, Weapon, WeaponAbilityState};
use super::inventory_ui::InventoryRequests;

/// Adds the reusable inventory/equipment state to every player character:
/// this machine's `Player` and, on a multiplayer host, each joined
/// player's body. The character's current Weapon becomes weapon slot 1.
/// Also bakes in any flat Ability Power granted by the run's active relics
/// (`RunModifiers::bonus_ability_power`) -- see
/// `Equipment::bonus_ability_power`.
///
/// A joined player's body arrives with its `Equipment` and
/// `WeaponAbilityState` already (`net::player_bodies`), so it only gains
/// the backpack here.
pub(crate) fn initialize_player_inventory(
    mut commands: Commands,
    run_modifiers: Res<RunModifiers>,
    players: Query<
        (Entity, &Weapon, Has<Equipment>, Has<WeaponAbilityState>),
        (With<PlayerCharacter>, Without<PlayerInventory>),
    >,
) {
    for (entity, starter_weapon, has_equipment, has_ability_state) in &players {
        let mut character = commands.entity(entity);
        character.insert((PlayerInventory::default(), InventoryRequests::default()));
        if !has_equipment {
            character.insert(Equipment::with_starter_weapon(
                *starter_weapon,
                run_modifiers.bonus_ability_power,
            ));
        }
        if !has_ability_state {
            character.insert(WeaponAbilityState::default());
        }
    }
}

/// Recalculates max health from equipped armor, for every player
/// character. The current-health percentage is preserved, so armor
/// swapping cannot be used as a free heal.
pub(crate) fn update_armor_health_bonus(
    mut players: Query<
        (&Equipment, &BaseHealth, &mut Health),
        (With<PlayerCharacter>, Changed<Equipment>),
    >,
) {
    for (equipment, base_health, mut health) in &mut players {
        let old_max = health.max.max(1.0);
        let health_percent = (health.current / old_max).clamp(0.0, 1.0);
        let new_max = (base_health.0 + equipment.total_health_bonus()).max(1.0);

        health.max = new_max;
        health.current = health_percent * new_max;
    }
}
