use crate::items::{ItemDefinitionId, ItemIdGenerator};
use crate::run_setup::DropPoolCategoryId;
use crate::weapons::{
    Armor, ArmorSlot, BELLRINGER_ID, BELLRINGER_NAME, DANCER_IN_THE_DARK_ID,
    DANCER_IN_THE_DARK_NAME, GearTier, HOLLOW_GOSPEL_ID, HOLLOW_GOSPEL_NAME, InventoryItem,
    KINGSLAYER_ID, KINGSLAYER_NAME, LIVING_CODEX_ID, LIVING_CODEX_NAME, RUINPIERCER_ID,
    RUINPIERCER_NAME, SAINTS_NAIL_ID, SAINTS_NAIL_NAME, SAINTS_TESTAMENT_ID, SAINTS_TESTAMENT_NAME,
    WIDOWS_KISS_ID, WIDOWS_KISS_NAME, Weapon, WeaponType,
};

// The Arcanist / Striker / Bulwark gear archetypes
// (RUINBOUND_GEAR_ARCHETYPES.md). Each module owns its own item
// definition ids and stat tables; see src/loot/gear/mod.rs.
use super::gear;

pub const COMMON_LONGSWORD: ItemDefinitionId = ItemDefinitionId("weapon.longsword.common");
pub const RARE_LONGSWORD: ItemDefinitionId = ItemDefinitionId("weapon.longsword.rare");
pub const COMMON_GREATSWORD: ItemDefinitionId = ItemDefinitionId("weapon.greatsword.common");
pub const RARE_GREATSWORD: ItemDefinitionId = ItemDefinitionId("weapon.greatsword.rare");
pub const COMMON_BOW: ItemDefinitionId = ItemDefinitionId("weapon.bow.common");
pub const RARE_BOW: ItemDefinitionId = ItemDefinitionId("weapon.bow.rare");
pub const EPIC_BOW: ItemDefinitionId = ItemDefinitionId("weapon.bow.epic");
pub const COMMON_HAMMER: ItemDefinitionId = ItemDefinitionId("weapon.hammer.common");
pub const RARE_HAMMER: ItemDefinitionId = ItemDefinitionId("weapon.hammer.rare");

pub const COMMON_CROSSBOW: ItemDefinitionId = ItemDefinitionId("weapon.crossbow.common");
pub const RARE_CROSSBOW: ItemDefinitionId = ItemDefinitionId("weapon.crossbow.rare");
pub const EPIC_CROSSBOW: ItemDefinitionId = ItemDefinitionId("weapon.crossbow.epic");

pub const COMMON_SPELLBOOK: ItemDefinitionId = ItemDefinitionId("weapon.spellbook.common");
pub const RARE_SPELLBOOK: ItemDefinitionId = ItemDefinitionId("weapon.spellbook.rare");
pub const EPIC_SPELLBOOK: ItemDefinitionId = ItemDefinitionId("weapon.spellbook.epic");

pub const COMMON_DAGGERS: ItemDefinitionId = ItemDefinitionId("weapon.daggers.common");
pub const RARE_DAGGERS: ItemDefinitionId = ItemDefinitionId("weapon.daggers.rare");
pub const EPIC_DAGGERS: ItemDefinitionId = ItemDefinitionId("weapon.daggers.epic");

// The six named Legendary Crossbows/Spellbooks, plus the three named
// Legendary Daggers -- see `weapons::crossbow`, `weapons::spellbook`, and
// `weapons::daggers` for their unique abilities/passives. `SAINTS_NAIL_ID`
// etc. are re-exported from `weapons` (the ids the weapon's own
// `metadata.definition_id` is set to via `Weapon::legendary` at
// construction below), so ability dispatch in
// `weapons::abilities::weapon_ability_input` and item creation here can
// never drift out of sync on what identifies which legendary.

pub const COMMON_HEAD: ItemDefinitionId = ItemDefinitionId("armor.head.common");
pub const COMMON_CHEST: ItemDefinitionId = ItemDefinitionId("armor.chest.common");
pub const COMMON_HANDS: ItemDefinitionId = ItemDefinitionId("armor.hands.common");
pub const COMMON_LEGS: ItemDefinitionId = ItemDefinitionId("armor.legs.common");
pub const COMMON_FEET: ItemDefinitionId = ItemDefinitionId("armor.feet.common");
pub const RARE_HEAD: ItemDefinitionId = ItemDefinitionId("armor.head.rare");
pub const RARE_CHEST: ItemDefinitionId = ItemDefinitionId("armor.chest.rare");

/// Central item factory. Add future weapons/armor here, then reference their
/// definition IDs from any loot table without changing enemy or chest code.
pub fn create_item(
    definition_id: ItemDefinitionId,
    id_generator: &mut ItemIdGenerator,
) -> Option<InventoryItem> {
    let mut item = match definition_id {
        COMMON_LONGSWORD => InventoryItem::Weapon(Weapon::longsword(GearTier::Common)),
        RARE_LONGSWORD => InventoryItem::Weapon(Weapon::longsword(GearTier::Rare)),
        COMMON_GREATSWORD => InventoryItem::Weapon(Weapon::greatsword(GearTier::Common)),
        RARE_GREATSWORD => InventoryItem::Weapon(Weapon::greatsword(GearTier::Rare)),
        COMMON_BOW => InventoryItem::Weapon(Weapon::bow(GearTier::Common)),
        RARE_BOW => InventoryItem::Weapon(Weapon::bow(GearTier::Rare)),
        EPIC_BOW => InventoryItem::Weapon(Weapon::bow(GearTier::Epic)),
        COMMON_HAMMER => InventoryItem::Weapon(Weapon::hammer(GearTier::Common)),
        RARE_HAMMER => InventoryItem::Weapon(Weapon::hammer(GearTier::Rare)),

        COMMON_CROSSBOW => InventoryItem::Weapon(Weapon::crossbow(GearTier::Common)),
        RARE_CROSSBOW => InventoryItem::Weapon(Weapon::crossbow(GearTier::Rare)),
        EPIC_CROSSBOW => InventoryItem::Weapon(Weapon::crossbow(GearTier::Epic)),

        COMMON_SPELLBOOK => InventoryItem::Weapon(Weapon::spellbook(GearTier::Common)),
        RARE_SPELLBOOK => InventoryItem::Weapon(Weapon::spellbook(GearTier::Rare)),
        EPIC_SPELLBOOK => InventoryItem::Weapon(Weapon::spellbook(GearTier::Epic)),

        COMMON_DAGGERS => InventoryItem::Weapon(Weapon::daggers(GearTier::Common)),
        RARE_DAGGERS => InventoryItem::Weapon(Weapon::daggers(GearTier::Rare)),
        EPIC_DAGGERS => InventoryItem::Weapon(Weapon::daggers(GearTier::Epic)),

        SAINTS_NAIL_ID => InventoryItem::Weapon(Weapon::legendary(
            WeaponType::Crossbow,
            SAINTS_NAIL_ID.0,
            SAINTS_NAIL_NAME,
        )),
        RUINPIERCER_ID => InventoryItem::Weapon(Weapon::legendary(
            WeaponType::Crossbow,
            RUINPIERCER_ID.0,
            RUINPIERCER_NAME,
        )),
        BELLRINGER_ID => InventoryItem::Weapon(Weapon::legendary(
            WeaponType::Crossbow,
            BELLRINGER_ID.0,
            BELLRINGER_NAME,
        )),

        HOLLOW_GOSPEL_ID => InventoryItem::Weapon(Weapon::legendary(
            WeaponType::Spellbook,
            HOLLOW_GOSPEL_ID.0,
            HOLLOW_GOSPEL_NAME,
        )),
        SAINTS_TESTAMENT_ID => InventoryItem::Weapon(Weapon::legendary(
            WeaponType::Spellbook,
            SAINTS_TESTAMENT_ID.0,
            SAINTS_TESTAMENT_NAME,
        )),
        LIVING_CODEX_ID => InventoryItem::Weapon(Weapon::legendary(
            WeaponType::Spellbook,
            LIVING_CODEX_ID.0,
            LIVING_CODEX_NAME,
        )),

        KINGSLAYER_ID => InventoryItem::Weapon(Weapon::legendary(
            WeaponType::Daggers,
            KINGSLAYER_ID.0,
            KINGSLAYER_NAME,
        )),
        WIDOWS_KISS_ID => InventoryItem::Weapon(Weapon::legendary(
            WeaponType::Daggers,
            WIDOWS_KISS_ID.0,
            WIDOWS_KISS_NAME,
        )),
        DANCER_IN_THE_DARK_ID => InventoryItem::Weapon(Weapon::legendary(
            WeaponType::Daggers,
            DANCER_IN_THE_DARK_ID.0,
            DANCER_IN_THE_DARK_NAME,
        )),
        COMMON_HEAD => {
            InventoryItem::Armor(Armor::new(ArmorSlot::Head, GearTier::Common, 6.0, 8.0))
        }
        COMMON_CHEST => {
            InventoryItem::Armor(Armor::new(ArmorSlot::Chest, GearTier::Common, 12.0, 18.0))
        }
        COMMON_HANDS => {
            InventoryItem::Armor(Armor::new(ArmorSlot::Hands, GearTier::Common, 5.0, 7.0))
        }
        COMMON_LEGS => {
            InventoryItem::Armor(Armor::new(ArmorSlot::Legs, GearTier::Common, 9.0, 14.0))
        }
        COMMON_FEET => {
            InventoryItem::Armor(Armor::new(ArmorSlot::Feet, GearTier::Common, 5.0, 7.0))
        }
        RARE_HEAD => InventoryItem::Armor(Armor::new(ArmorSlot::Head, GearTier::Rare, 12.0, 16.0)),
        RARE_CHEST => {
            InventoryItem::Armor(Armor::new(ArmorSlot::Chest, GearTier::Rare, 22.0, 32.0))
        }

        // Fall through to the archetype gear sets. Each `create` function
        // only recognizes its own archetype's definition ids and returns
        // `None` for anything else, so trying all three in sequence is
        // safe and cheap (57 total items, checked once per drop/pickup).
        _ => {
            if let Some(armor) = gear::create_arcanist(definition_id) {
                InventoryItem::Armor(armor)
            } else if let Some(armor) = gear::create_striker(definition_id) {
                InventoryItem::Armor(armor)
            } else if let Some(armor) = gear::create_bulwark(definition_id) {
                InventoryItem::Armor(armor)
            } else {
                return None;
            }
        }
    };

    match &mut item {
        InventoryItem::Weapon(weapon) => weapon.metadata.instance_id = id_generator.next(),
        InventoryItem::Armor(armor) => armor.metadata.instance_id = id_generator.next(),
    }

    Some(item)
}

/// Which item-drop-pool category (`run_setup::DropPoolCategoryId`) an
/// item definition belongs to, if any. Used by `loot::drops` to honor
/// `RunConfig::drop_pool`. Legacy/placeholder armor created via
/// `Armor::new` (the original Head/Chest/Hands/Legs/Feet sets, both
/// Common and Rare tier) predates the drop-pool system and has no
/// archetype, so this returns `None` for it -- an item with no category
/// is never filtered out, since the player has no toggle for it.
///
/// Builds a throwaway item via `create_item` rather than duplicating its
/// id-to-category matching here, so this can never drift out of sync
/// with what actually drops. The scratch `ItemIdGenerator` is local and
/// discarded, so this never consumes a real item-instance id.
pub fn item_drop_pool_category(id: ItemDefinitionId) -> Option<DropPoolCategoryId> {
    let mut scratch_ids = ItemIdGenerator::default();
    match create_item(id, &mut scratch_ids)? {
        InventoryItem::Weapon(weapon) => Some(DropPoolCategoryId::Weapon(weapon.weapon_type)),
        InventoryItem::Armor(armor) => armor.archetype.map(DropPoolCategoryId::Gear),
    }
}

/// The gear tier of an item definition, if it exists. Used by
/// `loot::drops` for the "rare item chance" relic/pact bonus
/// (`run_setup::RunModifiers::rare_drop_chance_bonus`). See
/// `item_drop_pool_category` for why this goes through `create_item`
/// rather than its own id matching.
pub fn item_tier(id: ItemDefinitionId) -> Option<GearTier> {
    let mut scratch_ids = ItemIdGenerator::default();
    match create_item(id, &mut scratch_ids)? {
        InventoryItem::Weapon(weapon) => Some(weapon.tier),
        InventoryItem::Armor(armor) => Some(armor.tier),
    }
}
