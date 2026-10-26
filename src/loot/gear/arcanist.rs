//! Arcanist Gear -- light armor for ability-heavy builds. Lowest defense
//! of the three archetypes; see `RUINBOUND_GEAR_ARCHETYPES.md` for design
//! rationale. Four rarity sets, five armor slots (Legendary has no Legs
//! piece -- the design doc only defines four Legendary Arcanist items).

use crate::items::ItemDefinitionId;
use crate::weapons::{Armor, ArmorSlot, GearArchetype, GearEffect, GearTier};

use super::build;

/// Per-slot Common-tier base budget: [Head, Chest, Hands, Legs, Feet].
/// Arcanist sits at ~65% of Striker's defense, which itself sits at ~74%
/// of Bulwark's -- both within the "Balance Rules" ranges in the design
/// doc (Arcanist 60-70% of Striker; Striker 70-80% of Bulwark).
const BASE_DEFENSE: [f32; 5] = [4.0, 8.0, 3.0, 6.0, 3.0];
const BASE_HEALTH: [f32; 5] = [5.0, 12.0, 4.0, 9.0, 4.0];

/// Common-tier Ability Power granted by the two Apprentice pieces whose
/// `GearEffect` text already reads "Increased ability damage" (Hood,
/// Leggings -- see `Equipment::ability_power_multiplier` in
/// `weapons/components.rs`). The rest of the Arcanist set keeps its
/// effect descriptive-only for now, same as every other archetype item;
/// this is a first real, working number for the two pieces whose flavor
/// text already promised it, not a full itemization pass.
const APPRENTICE_ABILITY_POWER: f32 = 8.0;

// =========================================================
// COMMON -- Apprentice Set
// =========================================================
// Straightforward ability-stat bonuses with very little protection.

/// Apprentice Hood (Head, Common). "Increased ability damage."
pub const APPRENTICE_HOOD: ItemDefinitionId = ItemDefinitionId("armor.arcanist.head.common");
/// Apprentice Robes (Chest, Common). "A small amount of defense and
/// increased maximum ability resource."
pub const APPRENTICE_ROBES: ItemDefinitionId = ItemDefinitionId("armor.arcanist.chest.common");
/// Apprentice Gloves (Hands, Common). "Increased ability-resource
/// generation."
pub const APPRENTICE_GLOVES: ItemDefinitionId = ItemDefinitionId("armor.arcanist.hands.common");
/// Apprentice Leggings (Legs, Common). "Increased ability damage."
pub const APPRENTICE_LEGGINGS: ItemDefinitionId = ItemDefinitionId("armor.arcanist.legs.common");
/// Apprentice Shoes (Feet, Common). "Increased cooldown recovery."
pub const APPRENTICE_SHOES: ItemDefinitionId = ItemDefinitionId("armor.arcanist.feet.common");

// =========================================================
// RARE -- Runebound Set
// =========================================================
// Bonuses that trigger through active ability use.

/// Runebound Hood (Head, Rare). "Ability critical hits deal increased
/// damage."
pub const RUNEBOUND_HOOD: ItemDefinitionId = ItemDefinitionId("armor.arcanist.head.rare");
/// Runebound Robes (Chest, Rare). "Using an ability grants a small
/// temporary barrier."
pub const RUNEBOUND_ROBES: ItemDefinitionId = ItemDefinitionId("armor.arcanist.chest.rare");
/// Runebound Gloves (Hands, Rare). "Damaging an enemy with an ability
/// restores ability resource."
pub const RUNEBOUND_GLOVES: ItemDefinitionId = ItemDefinitionId("armor.arcanist.hands.rare");
/// Runebound Leggings (Legs, Rare). "Consecutive ability hits against the
/// same enemy gain damage."
pub const RUNEBOUND_LEGGINGS: ItemDefinitionId = ItemDefinitionId("armor.arcanist.legs.rare");
/// Runebound Shoes (Feet, Rare). "Using an ability briefly increases
/// movement speed."
pub const RUNEBOUND_SHOES: ItemDefinitionId = ItemDefinitionId("armor.arcanist.feet.rare");

// =========================================================
// EPIC -- Spellweaver Set
// =========================================================
// Abilities interact with one another instead of only giving numbers.

/// Spellweaver Crown -- "Arcane Rhythm" (Head, Epic). "Every third ability
/// deals bonus damage."
pub const SPELLWEAVER_CROWN: ItemDefinitionId = ItemDefinitionId("armor.arcanist.head.epic");
/// Spellweaver Mantle -- "Warding Invocation" (Chest, Epic). "Using a
/// defensive ability strengthens the next offensive ability."
pub const SPELLWEAVER_MANTLE: ItemDefinitionId = ItemDefinitionId("armor.arcanist.chest.epic");
/// Spellweaver Grips -- "Overflowing Power" (Hands, Epic). "Ability
/// resource gained above its maximum becomes a temporary barrier."
pub const SPELLWEAVER_GRIPS: ItemDefinitionId = ItemDefinitionId("armor.arcanist.hands.epic");
/// Spellweaver Legwraps -- "Lingering Sorcery" (Legs, Epic). "Ability
/// damage leaves a short damage-over-time effect."
pub const SPELLWEAVER_LEGWRAPS: ItemDefinitionId = ItemDefinitionId("armor.arcanist.legs.epic");
/// Spellweaver Steps -- "Quickened Casting" (Feet, Epic). "Defeating an
/// enemy with an ability reduces active cooldowns."
pub const SPELLWEAVER_STEPS: ItemDefinitionId = ItemDefinitionId("armor.arcanist.feet.epic");

// =========================================================
// LEGENDARY -- four unique items (no Legs piece in the design doc)
// =========================================================

/// Crown of the Final Word -- "Echoed Invocation" (Head, Legendary).
/// "After using three different abilities, the next ability is cast a
/// second time at reduced power."
pub const CROWN_OF_THE_FINAL_WORD: ItemDefinitionId =
    ItemDefinitionId("armor.arcanist.head.legendary");
/// Robes of the Hollow Star -- "Desperate Conduit" (Chest, Legendary).
/// "Ability damage increases as health decreases. Using an ability while
/// critically wounded grants a barrier, subject to a cooldown."
pub const ROBES_OF_THE_HOLLOW_STAR: ItemDefinitionId =
    ItemDefinitionId("armor.arcanist.chest.legendary");
/// Hands of the Spellthief -- "Stolen Power" (Hands, Legendary). "Hitting
/// an elite or boss with an ability temporarily copies a portion of its
/// offensive strength. Repeated hits refresh the effect but do not stack
/// it."
pub const HANDS_OF_THE_SPELLTHIEF: ItemDefinitionId =
    ItemDefinitionId("armor.arcanist.hands.legendary");
/// Boots of the Riftwalker -- "Rift Step" (Feet, Legendary). "Dodging
/// leaves behind a rift that erupts after a short delay and deals ability
/// damage."
pub const BOOTS_OF_THE_RIFTWALKER: ItemDefinitionId =
    ItemDefinitionId("armor.arcanist.feet.legendary");

/// Item ids grouped by tier, used by `loot::tables` to build weighted
/// loot-table entries without listing all 19 ids by hand in every table.
pub const COMMON: [ItemDefinitionId; 5] = [
    APPRENTICE_HOOD,
    APPRENTICE_ROBES,
    APPRENTICE_GLOVES,
    APPRENTICE_LEGGINGS,
    APPRENTICE_SHOES,
];
pub const RARE: [ItemDefinitionId; 5] = [
    RUNEBOUND_HOOD,
    RUNEBOUND_ROBES,
    RUNEBOUND_GLOVES,
    RUNEBOUND_LEGGINGS,
    RUNEBOUND_SHOES,
];
pub const EPIC: [ItemDefinitionId; 5] = [
    SPELLWEAVER_CROWN,
    SPELLWEAVER_MANTLE,
    SPELLWEAVER_GRIPS,
    SPELLWEAVER_LEGWRAPS,
    SPELLWEAVER_STEPS,
];
pub const LEGENDARY: [ItemDefinitionId; 4] = [
    CROWN_OF_THE_FINAL_WORD,
    ROBES_OF_THE_HOLLOW_STAR,
    HANDS_OF_THE_SPELLTHIEF,
    BOOTS_OF_THE_RIFTWALKER,
];

/// Builds the `Armor` for one Arcanist item definition id, or `None` if
/// `id` does not belong to this archetype (used by `loot::catalog` as one
/// link in the create_item lookup chain).
pub fn create(id: ItemDefinitionId) -> Option<Armor> {
    Some(match id {
        APPRENTICE_HOOD => build(
            id.0,
            "Apprentice Hood",
            ArmorSlot::Head,
            GearTier::Common,
            GearArchetype::Arcanist,
            BASE_DEFENSE[0],
            BASE_HEALTH[0],
            GearEffect::new("", "Increased ability damage."),
        )
        .with_ability_power_bonus(APPRENTICE_ABILITY_POWER),
        APPRENTICE_ROBES => build(
            id.0,
            "Apprentice Robes",
            ArmorSlot::Chest,
            GearTier::Common,
            GearArchetype::Arcanist,
            BASE_DEFENSE[1],
            BASE_HEALTH[1],
            GearEffect::new(
                "",
                "A small amount of defense and increased maximum ability resource.",
            ),
        ),
        APPRENTICE_GLOVES => build(
            id.0,
            "Apprentice Gloves",
            ArmorSlot::Hands,
            GearTier::Common,
            GearArchetype::Arcanist,
            BASE_DEFENSE[2],
            BASE_HEALTH[2],
            GearEffect::new("", "Increased ability-resource generation."),
        ),
        APPRENTICE_LEGGINGS => build(
            id.0,
            "Apprentice Leggings",
            ArmorSlot::Legs,
            GearTier::Common,
            GearArchetype::Arcanist,
            BASE_DEFENSE[3],
            BASE_HEALTH[3],
            GearEffect::new("", "Increased ability damage."),
        )
        .with_ability_power_bonus(APPRENTICE_ABILITY_POWER),
        APPRENTICE_SHOES => build(
            id.0,
            "Apprentice Shoes",
            ArmorSlot::Feet,
            GearTier::Common,
            GearArchetype::Arcanist,
            BASE_DEFENSE[4],
            BASE_HEALTH[4],
            GearEffect::new("", "Increased cooldown recovery."),
        ),

        RUNEBOUND_HOOD => build(
            id.0,
            "Runebound Hood",
            ArmorSlot::Head,
            GearTier::Rare,
            GearArchetype::Arcanist,
            BASE_DEFENSE[0],
            BASE_HEALTH[0],
            GearEffect::new("", "Ability critical hits deal increased damage."),
        ),
        RUNEBOUND_ROBES => build(
            id.0,
            "Runebound Robes",
            ArmorSlot::Chest,
            GearTier::Rare,
            GearArchetype::Arcanist,
            BASE_DEFENSE[1],
            BASE_HEALTH[1],
            GearEffect::new("", "Using an ability grants a small temporary barrier."),
        ),
        RUNEBOUND_GLOVES => build(
            id.0,
            "Runebound Gloves",
            ArmorSlot::Hands,
            GearTier::Rare,
            GearArchetype::Arcanist,
            BASE_DEFENSE[2],
            BASE_HEALTH[2],
            GearEffect::new(
                "",
                "Damaging an enemy with an ability restores ability resource.",
            ),
        ),
        RUNEBOUND_LEGGINGS => build(
            id.0,
            "Runebound Leggings",
            ArmorSlot::Legs,
            GearTier::Rare,
            GearArchetype::Arcanist,
            BASE_DEFENSE[3],
            BASE_HEALTH[3],
            GearEffect::new(
                "",
                "Consecutive ability hits against the same enemy gain damage.",
            ),
        ),
        RUNEBOUND_SHOES => build(
            id.0,
            "Runebound Shoes",
            ArmorSlot::Feet,
            GearTier::Rare,
            GearArchetype::Arcanist,
            BASE_DEFENSE[4],
            BASE_HEALTH[4],
            GearEffect::new("", "Using an ability briefly increases movement speed."),
        ),

        SPELLWEAVER_CROWN => build(
            id.0,
            "Spellweaver Crown",
            ArmorSlot::Head,
            GearTier::Epic,
            GearArchetype::Arcanist,
            BASE_DEFENSE[0],
            BASE_HEALTH[0],
            GearEffect::new("Arcane Rhythm", "Every third ability deals bonus damage."),
        ),
        SPELLWEAVER_MANTLE => build(
            id.0,
            "Spellweaver Mantle",
            ArmorSlot::Chest,
            GearTier::Epic,
            GearArchetype::Arcanist,
            BASE_DEFENSE[1],
            BASE_HEALTH[1],
            GearEffect::new(
                "Warding Invocation",
                "Using a defensive ability strengthens the next offensive ability.",
            ),
        ),
        SPELLWEAVER_GRIPS => build(
            id.0,
            "Spellweaver Grips",
            ArmorSlot::Hands,
            GearTier::Epic,
            GearArchetype::Arcanist,
            BASE_DEFENSE[2],
            BASE_HEALTH[2],
            GearEffect::new(
                "Overflowing Power",
                "Ability resource gained above its maximum becomes a temporary barrier.",
            ),
        ),
        SPELLWEAVER_LEGWRAPS => build(
            id.0,
            "Spellweaver Legwraps",
            ArmorSlot::Legs,
            GearTier::Epic,
            GearArchetype::Arcanist,
            BASE_DEFENSE[3],
            BASE_HEALTH[3],
            GearEffect::new(
                "Lingering Sorcery",
                "Ability damage leaves a short damage-over-time effect.",
            ),
        ),
        SPELLWEAVER_STEPS => build(
            id.0,
            "Spellweaver Steps",
            ArmorSlot::Feet,
            GearTier::Epic,
            GearArchetype::Arcanist,
            BASE_DEFENSE[4],
            BASE_HEALTH[4],
            GearEffect::new(
                "Quickened Casting",
                "Defeating an enemy with an ability reduces active cooldowns.",
            ),
        ),

        CROWN_OF_THE_FINAL_WORD => build(
            id.0,
            "Crown of the Final Word",
            ArmorSlot::Head,
            GearTier::Legendary,
            GearArchetype::Arcanist,
            BASE_DEFENSE[0],
            BASE_HEALTH[0],
            GearEffect::new(
                "Echoed Invocation",
                "After using three different abilities, the next ability is cast a second time at reduced power.",
            ),
        ),
        ROBES_OF_THE_HOLLOW_STAR => build(
            id.0,
            "Robes of the Hollow Star",
            ArmorSlot::Chest,
            GearTier::Legendary,
            GearArchetype::Arcanist,
            BASE_DEFENSE[1],
            BASE_HEALTH[1],
            GearEffect::new(
                "Desperate Conduit",
                "Ability damage increases as health decreases. Using an ability while critically wounded grants a barrier, subject to a cooldown.",
            ),
        ),
        HANDS_OF_THE_SPELLTHIEF => build(
            id.0,
            "Hands of the Spellthief",
            ArmorSlot::Hands,
            GearTier::Legendary,
            GearArchetype::Arcanist,
            BASE_DEFENSE[2],
            BASE_HEALTH[2],
            GearEffect::new(
                "Stolen Power",
                "Hitting an elite or boss with an ability temporarily copies a portion of its offensive strength. Repeated hits refresh the effect but do not stack it.",
            ),
        ),
        BOOTS_OF_THE_RIFTWALKER => build(
            id.0,
            "Boots of the Riftwalker",
            ArmorSlot::Feet,
            GearTier::Legendary,
            GearArchetype::Arcanist,
            BASE_DEFENSE[4],
            BASE_HEALTH[4],
            GearEffect::new(
                "Rift Step",
                "Dodging leaves behind a rift that erupts after a short delay and deals ability damage.",
            ),
        ),

        _ => return None,
    })
}
