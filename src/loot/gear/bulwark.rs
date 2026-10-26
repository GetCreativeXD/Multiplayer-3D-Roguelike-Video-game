//! Bulwark Gear -- heavy armor with the highest defense of the three
//! archetypes. Its offense is meant to scale from Armor, max health,
//! blocking, barriers, and other defensive actions (see
//! `RUINBOUND_GEAR_ARCHETYPES.md`). Four rarity sets, five armor slots
//! (Legendary has no Legs piece -- the design doc only defines four
//! Legendary Bulwark items).

use crate::items::ItemDefinitionId;
use crate::weapons::{Armor, ArmorSlot, GearArchetype, GearEffect, GearTier};

use super::build;

/// Per-slot Common-tier base budget: [Head, Chest, Hands, Legs, Feet].
/// Bulwark is the tankiest archetype -- these are the highest raw
/// defense/health numbers of the three files. Striker sits at ~74% of
/// this budget, and Arcanist at ~65% of Striker's, matching the design
/// doc's "Balance Rules" ranges.
const BASE_DEFENSE: [f32; 5] = [8.0, 16.0, 7.0, 12.0, 7.0];
const BASE_HEALTH: [f32; 5] = [11.0, 24.0, 10.0, 18.0, 10.0];

// =========================================================
// COMMON -- Ironwall Set
// =========================================================
// Straightforward survivability and minor defense-based offense.

/// Ironwall Helm (Head, Common). "Increased Armor."
pub const IRONWALL_HELM: ItemDefinitionId = ItemDefinitionId("armor.bulwark.head.common");
/// Ironwall Plate (Chest, Common). "Increased maximum health."
pub const IRONWALL_PLATE: ItemDefinitionId = ItemDefinitionId("armor.bulwark.chest.common");
/// Ironwall Gauntlets (Hands, Common). "Basic attacks gain a small damage
/// bonus based on Armor."
pub const IRONWALL_GAUNTLETS: ItemDefinitionId = ItemDefinitionId("armor.bulwark.hands.common");
/// Ironwall Legplates (Legs, Common). "Increased stagger resistance."
pub const IRONWALL_LEGPLATES: ItemDefinitionId = ItemDefinitionId("armor.bulwark.legs.common");
/// Ironwall Greaves (Feet, Common). "Reduced knockback and improved
/// stability."
pub const IRONWALL_GREAVES: ItemDefinitionId = ItemDefinitionId("armor.bulwark.feet.common");

// =========================================================
// RARE -- Bastion Set
// =========================================================
// Turns enemy pressure into defensive and offensive opportunities.

/// Bastion Helm (Head, Rare). "Taking a heavy hit briefly increases
/// Armor."
pub const BASTION_HELM: ItemDefinitionId = ItemDefinitionId("armor.bulwark.head.rare");
/// Bastion Plate (Chest, Rare). "Healing received is increased while
/// below half health."
pub const BASTION_PLATE: ItemDefinitionId = ItemDefinitionId("armor.bulwark.chest.rare");
/// Bastion Gauntlets (Hands, Rare). "Blocking empowers the next basic
/// attack."
pub const BASTION_GAUNTLETS: ItemDefinitionId = ItemDefinitionId("armor.bulwark.hands.rare");
/// Bastion Legplates (Legs, Rare). "Remaining near multiple enemies
/// grants damage resistance."
pub const BASTION_LEGPLATES: ItemDefinitionId = ItemDefinitionId("armor.bulwark.legs.rare");
/// Bastion Greaves (Feet, Rare). "Resisting a stagger releases a small
/// damaging shockwave."
pub const BASTION_GREAVES: ItemDefinitionId = ItemDefinitionId("armor.bulwark.feet.rare");

// =========================================================
// EPIC -- Juggernaut Set
// =========================================================
// Actively converts defense into pressure, stagger, and retaliation.

/// Juggernaut Greathelm -- "Immovable" (Head, Epic). "Remaining in place
/// for a short time builds Armor and stagger resistance. Moving
/// gradually removes the effect."
pub const JUGGERNAUT_GREATHELM: ItemDefinitionId = ItemDefinitionId("armor.bulwark.head.epic");
/// Juggernaut Plate -- "Retaliating Steel" (Chest, Epic). "A portion of
/// damage mitigated by Armor charges a damaging pulse."
pub const JUGGERNAUT_PLATE: ItemDefinitionId = ItemDefinitionId("armor.bulwark.chest.epic");
/// Juggernaut Fists -- "Crushing Weight" (Hands, Epic). "Heavy attacks
/// gain damage based on Armor."
pub const JUGGERNAUT_FISTS: ItemDefinitionId = ItemDefinitionId("armor.bulwark.hands.epic");
/// Juggernaut Legplates -- "Hold the Line" (Legs, Epic). "Nearby enemies
/// increase defense and the size of retaliatory effects."
pub const JUGGERNAUT_LEGPLATES: ItemDefinitionId = ItemDefinitionId("armor.bulwark.legs.epic");
/// Juggernaut Sabatons -- "Unstoppable Advance" (Feet, Epic). "Moving
/// toward enemies reduces slows and builds stagger power for the next
/// attack."
pub const JUGGERNAUT_SABATONS: ItemDefinitionId = ItemDefinitionId("armor.bulwark.feet.epic");

// =========================================================
// LEGENDARY -- four unique items (no Legs piece in the design doc)
// =========================================================

/// Crown of the Unbroken -- "Refuse to Fall" (Head, Legendary). "Fatal
/// damage instead leaves the player at one health and creates a barrier
/// based on maximum health. This effect has a long cooldown."
pub const CROWN_OF_THE_UNBROKEN: ItemDefinitionId =
    ItemDefinitionId("armor.bulwark.head.legendary");
/// Worldbearer Plate -- "Living Fortress" (Chest, Legendary). "A portion
/// of bonus Armor also increases attack damage. The conversion has a cap
/// to prevent extreme Armor stacking from producing excessive damage."
pub const WORLDBEARER_PLATE: ItemDefinitionId = ItemDefinitionId("armor.bulwark.chest.legendary");
/// Gauntlets of Reprisal -- "Answer the Blow" (Hands, Legendary).
/// "Blocking or heavily mitigating an attack stores part of the
/// prevented damage. The next heavy attack releases the stored power."
pub const GAUNTLETS_OF_REPRISAL: ItemDefinitionId =
    ItemDefinitionId("armor.bulwark.hands.legendary");
/// Graveshaker Sabatons -- "Weight of War" (Feet, Legendary). "After
/// traveling a certain distance, the next heavy attack creates a
/// shockwave. Maximum health increases its area, while Armor increases
/// its stagger."
pub const GRAVESHAKER_SABATONS: ItemDefinitionId = ItemDefinitionId("armor.bulwark.feet.legendary");

/// Item ids grouped by tier, used by `loot::tables` to build weighted
/// loot-table entries without listing all 19 ids by hand in every table.
pub const COMMON: [ItemDefinitionId; 5] = [
    IRONWALL_HELM,
    IRONWALL_PLATE,
    IRONWALL_GAUNTLETS,
    IRONWALL_LEGPLATES,
    IRONWALL_GREAVES,
];
pub const RARE: [ItemDefinitionId; 5] = [
    BASTION_HELM,
    BASTION_PLATE,
    BASTION_GAUNTLETS,
    BASTION_LEGPLATES,
    BASTION_GREAVES,
];
pub const EPIC: [ItemDefinitionId; 5] = [
    JUGGERNAUT_GREATHELM,
    JUGGERNAUT_PLATE,
    JUGGERNAUT_FISTS,
    JUGGERNAUT_LEGPLATES,
    JUGGERNAUT_SABATONS,
];
pub const LEGENDARY: [ItemDefinitionId; 4] = [
    CROWN_OF_THE_UNBROKEN,
    WORLDBEARER_PLATE,
    GAUNTLETS_OF_REPRISAL,
    GRAVESHAKER_SABATONS,
];

/// Builds the `Armor` for one Bulwark item definition id, or `None` if
/// `id` does not belong to this archetype (used by `loot::catalog` as one
/// link in the create_item lookup chain).
pub fn create(id: ItemDefinitionId) -> Option<Armor> {
    Some(match id {
        IRONWALL_HELM => build(
            id.0,
            "Ironwall Helm",
            ArmorSlot::Head,
            GearTier::Common,
            GearArchetype::Bulwark,
            BASE_DEFENSE[0],
            BASE_HEALTH[0],
            GearEffect::new("", "Increased Armor."),
        ),
        IRONWALL_PLATE => build(
            id.0,
            "Ironwall Plate",
            ArmorSlot::Chest,
            GearTier::Common,
            GearArchetype::Bulwark,
            BASE_DEFENSE[1],
            BASE_HEALTH[1],
            GearEffect::new("", "Increased maximum health."),
        ),
        IRONWALL_GAUNTLETS => build(
            id.0,
            "Ironwall Gauntlets",
            ArmorSlot::Hands,
            GearTier::Common,
            GearArchetype::Bulwark,
            BASE_DEFENSE[2],
            BASE_HEALTH[2],
            GearEffect::new(
                "",
                "Basic attacks gain a small damage bonus based on Armor.",
            ),
        ),
        IRONWALL_LEGPLATES => build(
            id.0,
            "Ironwall Legplates",
            ArmorSlot::Legs,
            GearTier::Common,
            GearArchetype::Bulwark,
            BASE_DEFENSE[3],
            BASE_HEALTH[3],
            GearEffect::new("", "Increased stagger resistance."),
        ),
        IRONWALL_GREAVES => build(
            id.0,
            "Ironwall Greaves",
            ArmorSlot::Feet,
            GearTier::Common,
            GearArchetype::Bulwark,
            BASE_DEFENSE[4],
            BASE_HEALTH[4],
            GearEffect::new("", "Reduced knockback and improved stability."),
        ),

        BASTION_HELM => build(
            id.0,
            "Bastion Helm",
            ArmorSlot::Head,
            GearTier::Rare,
            GearArchetype::Bulwark,
            BASE_DEFENSE[0],
            BASE_HEALTH[0],
            GearEffect::new("", "Taking a heavy hit briefly increases Armor."),
        ),
        BASTION_PLATE => build(
            id.0,
            "Bastion Plate",
            ArmorSlot::Chest,
            GearTier::Rare,
            GearArchetype::Bulwark,
            BASE_DEFENSE[1],
            BASE_HEALTH[1],
            GearEffect::new("", "Healing received is increased while below half health."),
        ),
        BASTION_GAUNTLETS => build(
            id.0,
            "Bastion Gauntlets",
            ArmorSlot::Hands,
            GearTier::Rare,
            GearArchetype::Bulwark,
            BASE_DEFENSE[2],
            BASE_HEALTH[2],
            GearEffect::new("", "Blocking empowers the next basic attack."),
        ),
        BASTION_LEGPLATES => build(
            id.0,
            "Bastion Legplates",
            ArmorSlot::Legs,
            GearTier::Rare,
            GearArchetype::Bulwark,
            BASE_DEFENSE[3],
            BASE_HEALTH[3],
            GearEffect::new(
                "",
                "Remaining near multiple enemies grants damage resistance.",
            ),
        ),
        BASTION_GREAVES => build(
            id.0,
            "Bastion Greaves",
            ArmorSlot::Feet,
            GearTier::Rare,
            GearArchetype::Bulwark,
            BASE_DEFENSE[4],
            BASE_HEALTH[4],
            GearEffect::new(
                "",
                "Resisting a stagger releases a small damaging shockwave.",
            ),
        ),

        JUGGERNAUT_GREATHELM => build(
            id.0,
            "Juggernaut Greathelm",
            ArmorSlot::Head,
            GearTier::Epic,
            GearArchetype::Bulwark,
            BASE_DEFENSE[0],
            BASE_HEALTH[0],
            GearEffect::new(
                "Immovable",
                "Remaining in place for a short time builds Armor and stagger resistance. Moving gradually removes the effect.",
            ),
        ),
        JUGGERNAUT_PLATE => build(
            id.0,
            "Juggernaut Plate",
            ArmorSlot::Chest,
            GearTier::Epic,
            GearArchetype::Bulwark,
            BASE_DEFENSE[1],
            BASE_HEALTH[1],
            GearEffect::new(
                "Retaliating Steel",
                "A portion of damage mitigated by Armor charges a damaging pulse.",
            ),
        ),
        JUGGERNAUT_FISTS => build(
            id.0,
            "Juggernaut Fists",
            ArmorSlot::Hands,
            GearTier::Epic,
            GearArchetype::Bulwark,
            BASE_DEFENSE[2],
            BASE_HEALTH[2],
            GearEffect::new(
                "Crushing Weight",
                "Heavy attacks gain damage based on Armor.",
            ),
        ),
        JUGGERNAUT_LEGPLATES => build(
            id.0,
            "Juggernaut Legplates",
            ArmorSlot::Legs,
            GearTier::Epic,
            GearArchetype::Bulwark,
            BASE_DEFENSE[3],
            BASE_HEALTH[3],
            GearEffect::new(
                "Hold the Line",
                "Nearby enemies increase defense and the size of retaliatory effects.",
            ),
        ),
        JUGGERNAUT_SABATONS => build(
            id.0,
            "Juggernaut Sabatons",
            ArmorSlot::Feet,
            GearTier::Epic,
            GearArchetype::Bulwark,
            BASE_DEFENSE[4],
            BASE_HEALTH[4],
            GearEffect::new(
                "Unstoppable Advance",
                "Moving toward enemies reduces slows and builds stagger power for the next attack.",
            ),
        ),

        CROWN_OF_THE_UNBROKEN => build(
            id.0,
            "Crown of the Unbroken",
            ArmorSlot::Head,
            GearTier::Legendary,
            GearArchetype::Bulwark,
            BASE_DEFENSE[0],
            BASE_HEALTH[0],
            GearEffect::new(
                "Refuse to Fall",
                "Fatal damage instead leaves the player at one health and creates a barrier based on maximum health. This effect has a long cooldown.",
            ),
        ),
        WORLDBEARER_PLATE => build(
            id.0,
            "Worldbearer Plate",
            ArmorSlot::Chest,
            GearTier::Legendary,
            GearArchetype::Bulwark,
            BASE_DEFENSE[1],
            BASE_HEALTH[1],
            GearEffect::new(
                "Living Fortress",
                "A portion of bonus Armor also increases attack damage. The conversion has a cap to prevent extreme Armor stacking from producing excessive damage.",
            ),
        ),
        GAUNTLETS_OF_REPRISAL => build(
            id.0,
            "Gauntlets of Reprisal",
            ArmorSlot::Hands,
            GearTier::Legendary,
            GearArchetype::Bulwark,
            BASE_DEFENSE[2],
            BASE_HEALTH[2],
            GearEffect::new(
                "Answer the Blow",
                "Blocking or heavily mitigating an attack stores part of the prevented damage. The next heavy attack releases the stored power.",
            ),
        ),
        GRAVESHAKER_SABATONS => build(
            id.0,
            "Graveshaker Sabatons",
            ArmorSlot::Feet,
            GearTier::Legendary,
            GearArchetype::Bulwark,
            BASE_DEFENSE[4],
            BASE_HEALTH[4],
            GearEffect::new(
                "Weight of War",
                "After traveling a certain distance, the next heavy attack creates a shockwave. Maximum health increases its area, while Armor increases its stagger.",
            ),
        ),

        _ => return None,
    })
}
