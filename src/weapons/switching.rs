use bevy::prelude::*;

use crate::player::{PlayerCharacter, PlayerIntent};

use super::components::{Equipment, PlayerCombat, Weapon};

/// Switches weapon slots (keys 1, 2, 3) for every player character: the
/// local `Player` (its keys arrive through `player::read_local_input`, which
/// ignores them while the cursor is released) and, on a multiplayer host,
/// each joined player's body (through `net::intent`).
pub(crate) fn equipped_weapon_switching(
    mut players: Query<
        (
            &mut PlayerIntent,
            &mut Weapon,
            &mut PlayerCombat,
            &mut Equipment,
        ),
        With<PlayerCharacter>,
    >,
) {
    for (mut intent, mut weapon, mut combat, mut equipment) in &mut players {
        let Some(requested_slot) = intent.take_weapon_slot() else {
            continue;
        };

        let Some(new_weapon) = equipment.weapon_slots[requested_slot] else {
            println!("Weapon slot {} is empty", requested_slot + 1);
            continue;
        };

        if equipment.active_weapon_slot == requested_slot {
            continue;
        }

        equipment.active_weapon_slot = requested_slot;
        *weapon = new_weapon;

        // Cancel unfinished basic attacks when switching weapons.
        combat.attack_timer = 0.0;
        combat.pending_attack = None;
        combat.attack_cooldown = 0.0;

        // Do NOT cancel an active weapon technique.
        // Do NOT reset ability_cooldown.
        //
        // This keeps the currently active weapon ability running
        // and preserves the shared weapon-ability cooldown.
        //
        // For example:
        // Longsword Blade Rush -> swap to Greatsword ->
        // Blade Rush continues, but Greatsword ability stays locked
        // until ability_cooldown reaches 0.

        println!(
            "Equipped slot {}: {:?} {:?}",
            requested_slot + 1,
            weapon.tier,
            weapon.weapon_type,
        );
    }
}
