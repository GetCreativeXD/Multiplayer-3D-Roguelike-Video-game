//! What a player character carries, in a form any machine can display.
//!
//! The simulating machine (offline or the multiplayer host) owns every
//! player's real `Equipment` and `PlayerInventory`. It also publishes a
//! `PlayerInventorySnapshot` for every player character
//! (`publish_inventory_snapshots`), rebuilt only when either changes. In
//! multiplayer the snapshot is replicated, so a joined player's client can
//! show their own weapon slots, backpack count, pickup prompt and inventory
//! screen without simulating anything. The inventory screen reads it on
//! every machine, offline included (`weapons::inventory_ui`).
//!
//! Items are sent as display text (name, tier, the tooltip the inventory
//! screen already shows), not as `Weapon`/`Armor` values: a client never
//! uses an item's stats, and the text needs no item catalog on the client.

use bevy::prelude::*;

use crate::player::PlayerCharacter;

use super::components::{
    ArmorSlot, Equipment, GearTier, INVENTORY_CAPACITY, InventoryItem, PlayerInventory,
    WEAPON_SLOT_COUNT, WeaponType,
};
use super::inventory_ui::{item_label, item_tooltip};

/// The armor slots in the order `PlayerInventorySnapshot::armor` uses.
pub const ARMOR_SLOT_ORDER: [ArmorSlot; 5] = [
    ArmorSlot::Head,
    ArmorSlot::Chest,
    ArmorSlot::Hands,
    ArmorSlot::Legs,
    ArmorSlot::Feet,
];

/// Which kind of slot an item fits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum ItemSnapshotKind {
    Weapon(WeaponType),
    Armor(ArmorSlot),
}

/// One item as the HUD and inventory screen show it.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub struct ItemSnapshot {
    pub kind: ItemSnapshotKind,
    pub tier: GearTier,
    /// Two lines: tier, then name (the inventory slot label).
    pub label: String,
    /// The inventory screen's hover tooltip.
    pub tooltip: String,
}

impl ItemSnapshot {
    pub fn of(item: InventoryItem) -> Self {
        let (kind, tier) = match item {
            InventoryItem::Weapon(weapon) => {
                (ItemSnapshotKind::Weapon(weapon.weapon_type), weapon.tier)
            }
            InventoryItem::Armor(armor) => (ItemSnapshotKind::Armor(armor.slot), armor.tier),
        };
        Self {
            kind,
            tier,
            label: item_label(item),
            tooltip: item_tooltip(item),
        }
    }
}

/// A player character's weapons, armor and backpack, for display. See the
/// module documentation.
#[derive(Component, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub struct PlayerInventorySnapshot {
    pub weapon_slots: [Option<ItemSnapshot>; WEAPON_SLOT_COUNT],
    pub active_weapon_slot: u8,
    /// In `ARMOR_SLOT_ORDER`.
    pub armor: [Option<ItemSnapshot>; 5],
    pub backpack: [Option<ItemSnapshot>; INVENTORY_CAPACITY],
    pub total_health_bonus: f32,
    pub total_defense: f32,
    pub damage_reduction_percent: f32,
}

impl PlayerInventorySnapshot {
    pub fn of(equipment: &Equipment, inventory: &PlayerInventory) -> Self {
        let mut armor_equipment = equipment.armor;
        Self {
            weapon_slots: equipment
                .weapon_slots
                .map(|weapon| weapon.map(|weapon| ItemSnapshot::of(InventoryItem::Weapon(weapon)))),
            active_weapon_slot: equipment.active_weapon_slot as u8,
            armor: ARMOR_SLOT_ORDER.map(|slot| {
                (*armor_equipment.get_mut(slot))
                    .map(|armor| ItemSnapshot::of(InventoryItem::Armor(armor)))
            }),
            backpack: inventory.slots.map(|item| item.map(ItemSnapshot::of)),
            total_health_bonus: equipment.total_health_bonus(),
            total_defense: equipment.total_defense(),
            damage_reduction_percent: equipment.damage_reduction_percent(),
        }
    }

    pub fn has_free_weapon_slot(&self) -> bool {
        self.weapon_slots.iter().any(Option::is_none)
    }

    pub fn has_backpack_space(&self) -> bool {
        self.backpack.iter().any(Option::is_none)
    }

    pub fn occupied_backpack_slots(&self) -> usize {
        self.backpack.iter().flatten().count()
    }
}

/// Simulating machine only (offline or the host): keeps every player
/// character's snapshot current. Only rebuilds when their gear changed.
pub(crate) fn publish_inventory_snapshots(
    mut commands: Commands,
    mut characters: Query<
        (
            Entity,
            &Equipment,
            &PlayerInventory,
            Option<&mut PlayerInventorySnapshot>,
        ),
        (
            With<PlayerCharacter>,
            Or<(Changed<Equipment>, Changed<PlayerInventory>)>,
        ),
    >,
) {
    for (entity, equipment, inventory, snapshot) in &mut characters {
        let wanted = PlayerInventorySnapshot::of(equipment, inventory);
        match snapshot {
            Some(mut snapshot) => {
                snapshot.set_if_neq(wanted);
            }
            None => {
                commands.entity(entity).insert(wanted);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapons::components::{Armor, Weapon};

    #[test]
    fn snapshot_lists_slots_and_space() {
        let sword = Weapon::of_type(WeaponType::Longsword, GearTier::Common);
        let mut equipment = Equipment::with_starter_weapon(sword, 0.0);
        equipment.equip_armor(Armor::new(ArmorSlot::Chest, GearTier::Common, 12.0, 18.0));
        let mut inventory = PlayerInventory::default();
        inventory
            .add(InventoryItem::Weapon(Weapon::of_type(
                WeaponType::Bow,
                GearTier::Rare,
            )))
            .unwrap();

        let snapshot = PlayerInventorySnapshot::of(&equipment, &inventory);

        assert_eq!(
            snapshot.weapon_slots[0].as_ref().map(|item| item.kind),
            Some(ItemSnapshotKind::Weapon(WeaponType::Longsword))
        );
        assert!(snapshot.has_free_weapon_slot());
        assert_eq!(
            snapshot.armor[1].as_ref().map(|item| item.kind),
            Some(ItemSnapshotKind::Armor(ArmorSlot::Chest))
        );
        assert!(snapshot.armor[0].is_none());
        assert_eq!(snapshot.occupied_backpack_slots(), 1);
        assert!(snapshot.has_backpack_space());
        assert_eq!(snapshot.total_defense, 12.0);
    }
}
