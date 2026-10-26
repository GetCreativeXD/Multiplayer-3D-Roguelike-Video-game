//! Striker Gear -- medium armor for weapon-focused, basic-attack builds.
//! Sits between Arcanist and Bulwark on defense; see
//! `RUINBOUND_GEAR_ARCHETYPES.md`. Four rarity sets, five armor slots
//! (Legendary has no Legs piece -- the design doc only defines four
//! Legendary Striker items).

use crate::items::ItemDefinitionId;
use crate::weapons::{Armor, ArmorSlot, GearArchetype, GearEffect, GearTier};

use super::build;

/// Per-slot Common-tier base budget: [Head, Chest, Hands, Legs, Feet].
/// These numbers intentionally match the original placeholder "Iron" set
/// defense values (`armor.head.common` etc. in `loot/catalog.rs`) --
/// Striker's medium-armor identity is exactly what that original
/// generic armor represented, so it inherits the same budget rather than
/// introducing an arbitrary new one. Striker sits at ~74% of Bulwark's
/// defense, within the design doc's 70-80% "Balance Rules" range.
const BASE_DEFENSE: [f32; 5] = [6.0, 12.0, 5.0, 9.0, 5.0];
const BASE_HEALTH: [f32; 5] = [8.0, 18.0, 7.0, 14.0, 7.0];

// =========================================================
// COMMON -- Raider Set
// =========================================================
// Dependable weapon bonuses with reasonable protection.

/// Raider Helm (Head, Common). "Increased basic-attack damage."
pub const RAIDER_HELM: ItemDefinitionId = ItemDefinitionId("armor.striker.head.common");
/// Raider Vest (Chest, Common). "Moderate defense."
pub const RAIDER_VEST: ItemDefinitionId = ItemDefinitionId("armor.striker.chest.common");
/// Raider Gloves (Hands, Common). "Increased attack speed."
pub const RAIDER_GLOVES: ItemDefinitionId = ItemDefinitionId("armor.striker.hands.common");
/// Raider Legguards (Legs, Common). "Increased critical-hit chance."
pub const RAIDER_LEGGUARDS: ItemDefinitionId = ItemDefinitionId("armor.striker.legs.common");
/// Raider Boots (Feet, Common). "Increased movement speed while
/// attacking."
pub const RAIDER_BOOTS: ItemDefinitionId = ItemDefinitionId("armor.striker.feet.common");

// =========================================================
// RARE -- Duelist Set
// =========================================================
// Rewards positioning, mobility, and continued attacking.

/// Duelist Helm (Head, Rare). "Basic attacks against undamaged enemies
/// deal bonus damage."
pub const DUELIST_HELM: ItemDefinitionId = ItemDefinitionId("armor.striker.head.rare");
/// Duelist Vest (Chest, Rare). "Attacking shortly after dodging grants
/// temporary defense."
pub const DUELIST_VEST: ItemDefinitionId = ItemDefinitionId("armor.striker.chest.rare");
/// Duelist Gloves (Hands, Rare). "Consecutive basic attacks gradually
/// increase attack speed."
pub const DUELIST_GLOVES: ItemDefinitionId = ItemDefinitionId("armor.striker.hands.rare");
/// Duelist Legguards (Legs, Rare). "Critical basic attacks briefly
/// increase movement speed."
pub const DUELIST_LEGGUARDS: ItemDefinitionId = ItemDefinitionId("armor.striker.legs.rare");
/// Duelist Boots (Feet, Rare). "Basic attacks made shortly after
/// sprinting deal bonus damage."
pub const DUELIST_BOOTS: ItemDefinitionId = ItemDefinitionId("armor.striker.feet.rare");

// =========================================================
// EPIC -- Executioner Set
// =========================================================
// Powerful attack sequences that enemies can interrupt.

/// Executioner Visor -- "Expose Weakness" (Head, Epic). "Every fifth
/// basic attack makes the target temporarily take increased basic-attack
/// damage."
pub const EXECUTIONER_VISOR: ItemDefinitionId = ItemDefinitionId("armor.striker.head.epic");
/// Executioner Harness -- "Battle Momentum" (Chest, Epic). "Sustained
/// basic attacks grant stacking damage resistance. The stacks quickly
/// disappear when attacking stops."
pub const EXECUTIONER_HARNESS: ItemDefinitionId = ItemDefinitionId("armor.striker.chest.epic");
/// Executioner Grips -- "Relentless Hands" (Hands, Epic). "Critical basic
/// attacks accelerate the next several attacks."
pub const EXECUTIONER_GRIPS: ItemDefinitionId = ItemDefinitionId("armor.striker.hands.epic");
/// Executioner Legguards -- "Predator's Patience" (Legs, Epic). "Avoiding
/// damage for a short period increases the damage of the next basic
/// attack."
pub const EXECUTIONER_LEGGUARDS: ItemDefinitionId = ItemDefinitionId("armor.striker.legs.epic");
/// Executioner Treads -- "Pursuit" (Feet, Epic). "Moving toward an
/// injured enemy increases movement speed and basic-attack damage."
pub const EXECUTIONER_TREADS: ItemDefinitionId = ItemDefinitionId("armor.striker.feet.epic");

// =========================================================
// LEGENDARY -- four unique items (no Legs piece in the design doc)
// =========================================================

/// Helm of the Perfect Hunt -- "Marked for Death" (Head, Legendary). "The
/// first basic attack marks an enemy. Continued basic attacks build
/// toward a powerful finishing strike."
pub const HELM_OF_THE_PERFECT_HUNT: ItemDefinitionId =
    ItemDefinitionId("armor.striker.head.legendary");
/// Bloodrush Harness -- "Unbroken Assault" (Chest, Legendary). "Basic
/// attacks build Momentum, increasing attack speed and damage. Taking a
/// heavy hit removes some Momentum instead of all of it."
pub const BLOODRUSH_HARNESS: ItemDefinitionId = ItemDefinitionId("armor.striker.chest.legendary");
/// Gauntlets of the Twin Fang -- "Phantom Strike" (Hands, Legendary).
/// "Every few basic attacks produces a spectral follow-up attack. Slow
/// weapons trigger it less often but create a stronger follow-up."
pub const GAUNTLETS_OF_THE_TWIN_FANG: ItemDefinitionId =
    ItemDefinitionId("armor.striker.hands.legendary");
/// Wayfarer's Killing Boots -- "Drive-By" (Feet, Legendary). "Dodging
/// through or past an enemy empowers the next basic attack with bonus
/// damage and increased reach."
pub const WAYFARERS_KILLING_BOOTS: ItemDefinitionId =
    ItemDefinitionId("armor.striker.feet.legendary");

/// Item ids grouped by tier, used by `loot::tables` to build weighted
/// loot-table entries without listing all 19 ids by hand in every table.
pub const COMMON: [ItemDefinitionId; 5] = [
    RAIDER_HELM,
    RAIDER_VEST,
    RAIDER_GLOVES,
    RAIDER_LEGGUARDS,
    RAIDER_BOOTS,
];
pub const RARE: [ItemDefinitionId; 5] = [
    DUELIST_HELM,
    DUELIST_VEST,
    DUELIST_GLOVES,
    DUELIST_LEGGUARDS,
    DUELIST_BOOTS,
];
pub const EPIC: [ItemDefinitionId; 5] = [
    EXECUTIONER_VISOR,
    EXECUTIONER_HARNESS,
    EXECUTIONER_GRIPS,
    EXECUTIONER_LEGGUARDS,
    EXECUTIONER_TREADS,
];
pub const LEGENDARY: [ItemDefinitionId; 4] = [
    HELM_OF_THE_PERFECT_HUNT,
    BLOODRUSH_HARNESS,
    GAUNTLETS_OF_THE_TWIN_FANG,
    WAYFARERS_KILLING_BOOTS,
];

/// Builds the `Armor` for one Striker item definition id, or `None` if
/// `id` does not belong to this archetype (used by `loot::catalog` as one
/// link in the create_item lookup chain).
pub fn create(id: ItemDefinitionId) -> Option<Armor> {
    Some(match id {
        RAIDER_HELM => build(
            id.0,
            "Raider Helm",
            ArmorSlot::Head,
            GearTier::Common,
            GearArchetype::Striker,
            BASE_DEFENSE[0],
            BASE_HEALTH[0],
            GearEffect::new("", "Increased basic-attack damage."),
        ),
        RAIDER_VEST => build(
            id.0,
            "Raider Vest",
            ArmorSlot::Chest,
            GearTier::Common,
            GearArchetype::Striker,
            BASE_DEFENSE[1],
            BASE_HEALTH[1],
            GearEffect::new("", "Moderate defense."),
        ),
        RAIDER_GLOVES => build(
            id.0,
            "Raider Gloves",
            ArmorSlot::Hands,
            GearTier::Common,
            GearArchetype::Striker,
            BASE_DEFENSE[2],
            BASE_HEALTH[2],
            GearEffect::new("", "Increased attack speed."),
        ),
        RAIDER_LEGGUARDS => build(
            id.0,
            "Raider Legguards",
            ArmorSlot::Legs,
            GearTier::Common,
            GearArchetype::Striker,
            BASE_DEFENSE[3],
            BASE_HEALTH[3],
            GearEffect::new("", "Increased critical-hit chance."),
        ),
        RAIDER_BOOTS => build(
            id.0,
            "Raider Boots",
            ArmorSlot::Feet,
            GearTier::Common,
            GearArchetype::Striker,
            BASE_DEFENSE[4],
            BASE_HEALTH[4],
            GearEffect::new("", "Increased movement speed while attacking."),
        ),

        DUELIST_HELM => build(
            id.0,
            "Duelist Helm",
            ArmorSlot::Head,
            GearTier::Rare,
            GearArchetype::Striker,
            BASE_DEFENSE[0],
            BASE_HEALTH[0],
            GearEffect::new(
                "",
                "Basic attacks against undamaged enemies deal bonus damage.",
            ),
        ),
        DUELIST_VEST => build(
            id.0,
            "Duelist Vest",
            ArmorSlot::Chest,
            GearTier::Rare,
            GearArchetype::Striker,
            BASE_DEFENSE[1],
            BASE_HEALTH[1],
            GearEffect::new(
                "",
                "Attacking shortly after dodging grants temporary defense.",
            ),
        ),
        DUELIST_GLOVES => build(
            id.0,
            "Duelist Gloves",
            ArmorSlot::Hands,
            GearTier::Rare,
            GearArchetype::Striker,
            BASE_DEFENSE[2],
            BASE_HEALTH[2],
            GearEffect::new(
                "",
                "Consecutive basic attacks gradually increase attack speed.",
            ),
        ),
        DUELIST_LEGGUARDS => build(
            id.0,
            "Duelist Legguards",
            ArmorSlot::Legs,
            GearTier::Rare,
            GearArchetype::Striker,
            BASE_DEFENSE[3],
            BASE_HEALTH[3],
            GearEffect::new(
                "",
                "Critical basic attacks briefly increase movement speed.",
            ),
        ),
        DUELIST_BOOTS => build(
            id.0,
            "Duelist Boots",
            ArmorSlot::Feet,
            GearTier::Rare,
            GearArchetype::Striker,
            BASE_DEFENSE[4],
            BASE_HEALTH[4],
            GearEffect::new(
                "",
                "Basic attacks made shortly after sprinting deal bonus damage.",
            ),
        ),

        EXECUTIONER_VISOR => build(
            id.0,
            "Executioner Visor",
            ArmorSlot::Head,
            GearTier::Epic,
            GearArchetype::Striker,
            BASE_DEFENSE[0],
            BASE_HEALTH[0],
            GearEffect::new(
                "Expose Weakness",
                "Every fifth basic attack makes the target temporarily take increased basic-attack damage.",
            ),
        ),
        EXECUTIONER_HARNESS => build(
            id.0,
            "Executioner Harness",
            ArmorSlot::Chest,
            GearTier::Epic,
            GearArchetype::Striker,
            BASE_DEFENSE[1],
            BASE_HEALTH[1],
            GearEffect::new(
                "Battle Momentum",
                "Sustained basic attacks grant stacking damage resistance. The stacks quickly disappear when attacking stops.",
            ),
        ),
        EXECUTIONER_GRIPS => build(
            id.0,
            "Executioner Grips",
            ArmorSlot::Hands,
            GearTier::Epic,
            GearArchetype::Striker,
            BASE_DEFENSE[2],
            BASE_HEALTH[2],
            GearEffect::new(
                "Relentless Hands",
                "Critical basic attacks accelerate the next several attacks.",
            ),
        ),
        EXECUTIONER_LEGGUARDS => build(
            id.0,
            "Executioner Legguards",
            ArmorSlot::Legs,
            GearTier::Epic,
            GearArchetype::Striker,
            BASE_DEFENSE[3],
            BASE_HEALTH[3],
            GearEffect::new(
                "Predator's Patience",
                "Avoiding damage for a short period increases the damage of the next basic attack.",
            ),
        ),
        EXECUTIONER_TREADS => build(
            id.0,
            "Executioner Treads",
            ArmorSlot::Feet,
            GearTier::Epic,
            GearArchetype::Striker,
            BASE_DEFENSE[4],
            BASE_HEALTH[4],
            GearEffect::new(
                "Pursuit",
                "Moving toward an injured enemy increases movement speed and basic-attack damage.",
            ),
        ),

        HELM_OF_THE_PERFECT_HUNT => build(
            id.0,
            "Helm of the Perfect Hunt",
            ArmorSlot::Head,
            GearTier::Legendary,
            GearArchetype::Striker,
            BASE_DEFENSE[0],
            BASE_HEALTH[0],
            GearEffect::new(
                "Marked for Death",
                "The first basic attack marks an enemy. Continued basic attacks build toward a powerful finishing strike.",
            ),
        ),
        BLOODRUSH_HARNESS => build(
            id.0,
            "Bloodrush Harness",
            ArmorSlot::Chest,
            GearTier::Legendary,
            GearArchetype::Striker,
            BASE_DEFENSE[1],
            BASE_HEALTH[1],
            GearEffect::new(
                "Unbroken Assault",
                "Basic attacks build Momentum, increasing attack speed and damage. Taking a heavy hit removes some Momentum instead of all of it.",
            ),
        ),
        GAUNTLETS_OF_THE_TWIN_FANG => build(
            id.0,
            "Gauntlets of the Twin Fang",
            ArmorSlot::Hands,
            GearTier::Legendary,
            GearArchetype::Striker,
            BASE_DEFENSE[2],
            BASE_HEALTH[2],
            GearEffect::new(
                "Phantom Strike",
                "Every few basic attacks produces a spectral follow-up attack. Slow weapons trigger it less often but create a stronger follow-up.",
            ),
        ),
        WAYFARERS_KILLING_BOOTS => build(
            id.0,
            "Wayfarer's Killing Boots",
            ArmorSlot::Feet,
            GearTier::Legendary,
            GearArchetype::Striker,
            BASE_DEFENSE[4],
            BASE_HEALTH[4],
            GearEffect::new(
                "Drive-By",
                "Dodging through or past an enemy empowers the next basic attack with bonus damage and increased reach.",
            ),
        ),

        _ => return None,
    })
}
