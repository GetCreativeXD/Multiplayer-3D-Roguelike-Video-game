//! Static data for every choice on the run-setup screen (see
//! `ui::run_setup_screen`): classes, weapons, item-drop-pool categories,
//! starting relics, run pacts, and starting Ruin conditions.
//!
//! # Why this is a separate, data-only file
//!
//! `ui::run_setup_screen` never hard-codes an option's name or
//! description -- it only iterates these catalogs and reads
//! `RunConfig` (`run_setup::RunConfig`) to know what's currently chosen.
//! Adding a new relic, pact, Ruin condition, or (once it's implemented)
//! weapon family is one new entry in the matching `const` slice below;
//! nothing about the screen's layout, click handling, or tooltip code
//! needs to change. The same is true of `characters::ClassId` for
//! classes and `weapons::WeaponType` for weapons -- this file only adds
//! the *menu-facing* text on top of identifiers those modules already
//! own.
//!
//! Relic and pact effects are real, numeric, and applied for the whole
//! run -- see `effects::RunModifiers` for exactly what each one does and
//! `RelicEntry`/`PactEntry`'s own `effects` field for which relic/pact
//! grants what. Ruin-condition effects are only partially wired (see
//! `RuinConditionEntry`'s doc comment); `RunConfig` (`run_setup::mod`)
//! still records every choice regardless, so a future Ruin-condition
//! system (or a new relic/pact effect kind) has a stable,
//! already-built place to read the player's choice from.

use crate::characters::ClassId;
use crate::weapons::{GearArchetype, WeaponType};

use super::effects::RunEffect;

// =========================================================
// CLASSES
// =========================================================

/// One class ability or passive, shown in the run-setup screen's class
/// tab when the player hovers the class preview (see
/// `ui::run_setup_screen`'s `ClassAbilityPreview`). `key_hint` is a short
/// label like `"Q"`, `"E"`, `"R"`, or `"Passive"` -- not an actual input
/// binding, just what's shown next to the ability's name.
pub struct AbilityBlurb {
    pub key_hint: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

/// One selectable class, with the flavor text the run-setup screen shows.
/// The gameplay side of each class (its marker component and its three
/// abilities) lives in `characters::<class>` / `characters::<class>_abilities`
/// -- this only adds the menu-facing name/description on top of the
/// `ClassId` those modules already key off of.
pub struct ClassEntry {
    pub id: ClassId,
    pub name: &'static str,
    /// One-line playstyle summary shown on the class's own option card.
    pub tagline: &'static str,
    /// Longer blurb shown in the tooltip/description strip when the
    /// class's option card is hovered.
    pub description: &'static str,
    pub abilities: &'static [AbilityBlurb],
    /// Gravemind is an unlockable class (context doc's "Permanent-
    /// progression rule") -- nothing about that unlock system exists yet,
    /// so this is a hard-coded placeholder rather than a real save-data
    /// check. Locked classes still appear on the screen (as "Locked") so
    /// the roster reads as complete, but cannot be selected.
    pub unlocked: bool,
}

pub const CLASSES: &[ClassEntry] = &[
    ClassEntry {
        id: ClassId::Vanguard,
        name: "Vanguard",
        tagline: "Attack-focused duelist -- closes distance and snowballs off landed hits.",
        description: "Vanguard lives at melee range: Lunge closes gaps fast, and every basic \
            attack that connects builds Momentum, stacking up attack speed and movement speed \
            the longer a fight goes well.",
        abilities: &[
            AbilityBlurb {
                key_hint: "Q",
                name: "Lunge",
                description: "Dash quickly toward the nearest enemy in front of you, closing distance fast.",
            },
            AbilityBlurb {
                key_hint: "Passive",
                name: "Momentum",
                description: "Landing basic attacks builds stacks of Momentum, increasing attack speed and movement speed.",
            },
            AbilityBlurb {
                key_hint: "R",
                name: "Afterimage",
                description: "Leave behind a spectral duplicate that mirrors your attacks against nearby enemies for a short time.",
            },
        ],
        unlocked: true,
    },
    ClassEntry {
        id: ClassId::Warden,
        name: "Warden of Shades",
        tagline: "Tank/support -- shields allies and calls down spectral beasts.",
        description: "Warden of Shades protects the party with spirit animals: shielding and \
            empowering an ally, charging through enemies as a spectral stag, or striking a \
            target with a spectral predator.",
        abilities: &[
            AbilityBlurb {
                key_hint: "Q",
                name: "Guardian Otter",
                description: "Summon a spectral otter that shields a targeted ally and boosts their damage and movement speed.",
            },
            AbilityBlurb {
                key_hint: "E",
                name: "Shadow Stag",
                description: "Charge forward as a spectral stag, damaging and knocking back enemies in your path.",
            },
            AbilityBlurb {
                key_hint: "R",
                name: "Predator's Spirit",
                description: "Unleash a spectral predator that strikes a targeted enemy.",
            },
        ],
        unlocked: true,
    },
    ClassEntry {
        id: ClassId::Wildshaper,
        name: "Wildshaper",
        tagline: "Attack-focused shapeshifter -- one fast animal technique at a time.",
        description: "Wildshaper briefly transforms into an animal to execute a single fast \
            attack or movement technique, then immediately returns to human form -- a charge, \
            a slam, or a pounce, never a slow multi-beat sequence.",
        abilities: &[
            AbilityBlurb {
                key_hint: "Q",
                name: "Rhino Charge",
                description: "Transform into a rhino and bulldoze forward, damaging and knocking back enemies in your path.",
            },
            AbilityBlurb {
                key_hint: "E",
                name: "Bear Slam",
                description: "Transform into a bear and slam the ground after a brief windup, dealing heavy damage and stagger in a radius.",
            },
            AbilityBlurb {
                key_hint: "R",
                name: "Panther Pounce",
                description: "Transform into a panther and leap through the nearest enemy, striking it and landing beyond.",
            },
        ],
        unlocked: true,
    },
    ClassEntry {
        id: ClassId::Runesage,
        name: "Runesage",
        tagline: "Spell-focused -- Runes are earned by landing basic attacks, not cooldowns.",
        description: "Runesage builds Runes by landing basic-attack hits, then spends them \
            instantly on ranged rune magic -- a fast rhythm of weapon swings and spellcasting \
            with no ability cooldowns to wait out.",
        abilities: &[
            AbilityBlurb {
                key_hint: "Q -- 1 Rune",
                name: "Rune Blast",
                description: "Fire a fast rune projectile that damages and staggers whatever it hits.",
            },
            AbilityBlurb {
                key_hint: "E -- 2 Runes",
                name: "Runic Surge",
                description: "Release a short-range rune wave in front of you.",
            },
            AbilityBlurb {
                key_hint: "R -- 4 Runes",
                name: "Grand Rune",
                description: "Detonate a powerful rune burst around a targeted enemy.",
            },
        ],
        unlocked: true,
    },
    ClassEntry {
        id: ClassId::Gravemind,
        name: "Gravemind",
        tagline: "Unlockable -- aggressive corruption-user, trades safety for rising power.",
        description: "Gravemind channels the dungeon's own corruption: draining nearby enemies \
            to heal, firing corruption bolts, and embracing a dangerous empowered state that \
            drains health and raises Ruin in exchange for stacking bonus damage.",
        abilities: &[
            AbilityBlurb {
                key_hint: "Q",
                name: "Ruin Bolt",
                description: "Fire a heavy corruption bolt that splashes lighter damage onto nearby enemies on impact.",
            },
            AbilityBlurb {
                key_hint: "E",
                name: "Consume",
                description: "Drain corruption from nearby enemies, healing yourself for more the more enemies affected.",
            },
            AbilityBlurb {
                key_hint: "R",
                name: "Embrace the Ruin",
                description: "Toggle a high-risk state: health drains continuously while attacking builds a meter that grants stacking bonus damage and raises Ruin when it fills.",
            },
        ],
        unlocked: false,
    },
];

// =========================================================
// WEAPONS
// =========================================================

/// One selectable starting weapon. Classes do not restrict which
/// weapons a player can use (see `WEAPONS.md`'s "Weapon Philosophy"), so
/// every class sees the same weapon roster here -- only whether the
/// family is implemented in the prototype yet gates it.
pub struct WeaponEntry {
    pub weapon_type: WeaponType,
    pub name: &'static str,
    pub description: &'static str,
}

/// Only weapon families with a working basic attack and weapon ability
/// today (see `WEAPONS.md`'s "Development Status") appear as starting
/// choices. A new family becomes selectable by adding it here once it's
/// actually implemented -- not before, since choosing an unimplemented
/// weapon would leave the player unable to attack.
pub const WEAPONS: &[WeaponEntry] = &[
    WeaponEntry {
        weapon_type: WeaponType::Longsword,
        name: "Longsword",
        description: "Mobile, versatile melee blade -- fast basic attacks and repositioning techniques like Crosscut.",
    },
    WeaponEntry {
        weapon_type: WeaponType::Greatsword,
        name: "Greatsword",
        description: "Huge blade arcs, cleaves, and heavy slicing attacks that reward committing to a swing, like Executioner's Sweep.",
    },
    WeaponEntry {
        weapon_type: WeaponType::Bow,
        name: "Bow",
        description: "Precision ranged attacks, from piercing shots to Splinter Shot's homing fragments.",
    },
    WeaponEntry {
        weapon_type: WeaponType::Hammer,
        name: "Hammer",
        description: "Ground slams, shockwaves, and the heaviest stagger of any weapon in the roster.",
    },
    WeaponEntry {
        weapon_type: WeaponType::Crossbow,
        name: "Crossbow",
        description: "Slower, heavier-hitting ranged bolts than the Bow -- deliberate shots that reward positioning and commitment.",
    },
    WeaponEntry {
        weapon_type: WeaponType::Spellbook,
        name: "Spellbook",
        description: "Ability-focused caster -- every cast fires a homing Arcane Bolt that locks onto whatever you click on.",
    },
    WeaponEntry {
        weapon_type: WeaponType::Daggers,
        name: "Daggers",
        description: "Extremely fast, short-range melee -- built around mobility, backstab bonus damage, and Bleed/Venom rather than raw stagger.",
    },
];

// =========================================================
// ITEM DROP POOL
// =========================================================

/// One toggleable item-drop-pool category: either a weapon family or a
/// gear archetype. The run-setup screen lets the player disable
/// categories they don't want appearing as drops for the run -- see
/// `RunConfig::drop_pool` (`run_setup::mod`).
///
/// This directly reuses `WeaponType` and `GearArchetype` rather than a
/// separate menu-only identifier, so a category can never drift out of
/// sync with the identifier the loot system actually rolls against.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
// Shared by the whole co-op party (`run_setup::SharedRunSettings`).
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum DropPoolCategoryId {
    Weapon(WeaponType),
    Gear(GearArchetype),
}

pub struct DropPoolCategoryEntry {
    pub id: DropPoolCategoryId,
    pub name: &'static str,
    pub description: &'static str,
}

/// The full set of toggleable categories. Grows automatically as
/// `WEAPONS` above gains new implemented weapon families -- add the new
/// family to both lists at once.
pub const DROP_POOL_CATEGORIES: &[DropPoolCategoryEntry] = &[
    DropPoolCategoryEntry {
        id: DropPoolCategoryId::Weapon(WeaponType::Longsword),
        name: "Longswords",
        description: "Allow Longswords to appear as weapon drops and pickups this run.",
    },
    DropPoolCategoryEntry {
        id: DropPoolCategoryId::Weapon(WeaponType::Greatsword),
        name: "Greatswords",
        description: "Allow Greatswords to appear as weapon drops and pickups this run.",
    },
    DropPoolCategoryEntry {
        id: DropPoolCategoryId::Weapon(WeaponType::Bow),
        name: "Bows",
        description: "Allow Bows to appear as weapon drops and pickups this run.",
    },
    DropPoolCategoryEntry {
        id: DropPoolCategoryId::Weapon(WeaponType::Hammer),
        name: "Hammers",
        description: "Allow Hammers to appear as weapon drops and pickups this run.",
    },
    DropPoolCategoryEntry {
        id: DropPoolCategoryId::Weapon(WeaponType::Crossbow),
        name: "Crossbows",
        description: "Allow Crossbows to appear as weapon drops and pickups this run.",
    },
    DropPoolCategoryEntry {
        id: DropPoolCategoryId::Weapon(WeaponType::Spellbook),
        name: "Spellbooks",
        description: "Allow Spellbooks to appear as weapon drops and pickups this run.",
    },
    DropPoolCategoryEntry {
        id: DropPoolCategoryId::Weapon(WeaponType::Daggers),
        name: "Daggers",
        description: "Allow Daggers to appear as weapon drops and pickups this run.",
    },
    DropPoolCategoryEntry {
        id: DropPoolCategoryId::Gear(GearArchetype::Arcanist),
        name: "Arcanist Gear",
        description: "Allow Arcanist (ability-damage) armor to appear as drops this run.",
    },
    DropPoolCategoryEntry {
        id: DropPoolCategoryId::Gear(GearArchetype::Striker),
        name: "Striker Gear",
        description: "Allow Striker (basic-attack) armor to appear as drops this run.",
    },
    DropPoolCategoryEntry {
        id: DropPoolCategoryId::Gear(GearArchetype::Bulwark),
        name: "Bulwark Gear",
        description: "Allow Bulwark (defensive) armor to appear as drops this run.",
    },
];

/// The item-drop-pool selector requires at least this many enabled
/// categories, so a player can't guarantee an overly narrow build by
/// disabling everything but one weapon family (starting_menu.md's "The
/// game should probably require a minimum number of categories").
/// Experimental placeholder -- tune through playtesting like every other
/// number in the project.
pub const MIN_ENABLED_DROP_POOL_CATEGORIES: usize = 4;

// =========================================================
// STARTING RELIC / BLESSING
// =========================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
// Shared by the whole co-op party (`run_setup::SharedRunSettings`).
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum RelicId {
    AbilityDamage,
    MoreHealth,
    DodgeRecovery,
    RareItemChance,
    SlowerRuinGain,
}

pub struct RelicEntry {
    pub id: RelicId,
    pub name: &'static str,
    pub description: &'static str,
    /// The real, numeric effect(s) this relic grants for the whole run --
    /// see `effects::RunModifiers` for exactly what each `RunEffect`
    /// variant is wired into. The player may pick any number of relics at
    /// once (`RunConfig::relics`); `RunModifiers::from_selections` folds
    /// every active relic's (and pact's) effects together.
    pub effects: &'static [RunEffect],
}

/// Every relic's bonus is a small, permanent-for-the-run edge -- see
/// `effects::RunModifiers` for how these numbers are applied, and each
/// entry's own values below for the tuning (experimental, like every
/// other number in the project).
pub const RELICS: &[RelicEntry] = &[
    RelicEntry {
        id: RelicId::AbilityDamage,
        name: "Ember Sigil",
        description: "Increased ability damage for the rest of the run.",
        effects: &[RunEffect::BonusAbilityPower(25.0)],
    },
    RelicEntry {
        id: RelicId::MoreHealth,
        name: "Warden's Heart",
        description: "Start the run with additional maximum health.",
        effects: &[RunEffect::BonusMaxHealth(25.0)],
    },
    RelicEntry {
        id: RelicId::DodgeRecovery,
        name: "Swiftstep Charm",
        description: "Your dodge recovers faster, letting you reposition more often.",
        effects: &[RunEffect::DodgeCooldownMultiplier(0.6)],
    },
    RelicEntry {
        id: RelicId::RareItemChance,
        name: "Gilded Reliquary",
        description: "Increased chance for rare items to drop this run.",
        effects: &[RunEffect::RareDropChanceBonus(0.15)],
    },
    RelicEntry {
        id: RelicId::SlowerRuinGain,
        name: "Cleansed Fragment",
        description: "Ruin accumulates slightly more slowly for the rest of the run.",
        effects: &[RunEffect::RuinGainMultiplier(0.75)],
    },
];

// =========================================================
// RUN PACT
// =========================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
// Shared by the whole co-op party (`run_setup::SharedRunSettings`).
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum PactId {
    HarderEnemiesBetterRewards,
    RisingCorruption,
    FragileSanctuary,
    ScarceFortune,
}

pub struct PactEntry {
    pub id: PactId,
    pub name: &'static str,
    pub description: &'static str,
    /// This pact's real, numeric effect(s) -- see `effects::RunModifiers`.
    /// A run pact is optional (starting_menu.md: "Optional risk/reward
    /// modifiers"); `RunConfig::pacts` starts empty, and the player may
    /// take any number of pacts at once.
    pub effects: &'static [RunEffect],
}

/// Every pact trades a real downside for a real upside -- except the two
/// noted below, where the upside names a system (a shop, an item-choice
/// offer screen) that doesn't exist in the game yet. Rather than invent
/// one to fully realize the pact, only the implementable half is wired;
/// see each entry's own comment.
pub const PACTS: &[PactEntry] = &[
    PactEntry {
        id: PactId::HarderEnemiesBetterRewards,
        name: "Grim Bargain",
        description: "Enemies are stronger, but rewards improve.",
        effects: &[
            RunEffect::EnemyStrengthMultiplier(1.35),
            RunEffect::RareDropChanceBonus(0.20),
        ],
    },
    PactEntry {
        id: PactId::RisingCorruption,
        name: "Corruption's Price",
        description: "Ruin rises faster, but elite enemies drop better loot.",
        // The bonus applies to every loot roll, not elite encounters
        // specifically -- `loot::drops` doesn't distinguish "this roll
        // came from an elite" from any other roll, and adding that
        // distinction for one pact was judged a larger change than this
        // pass should make unprompted.
        effects: &[
            RunEffect::RuinGainMultiplier(1.5),
            RunEffect::RareDropChanceBonus(0.15),
        ],
    },
    PactEntry {
        id: PactId::FragileSanctuary,
        name: "Fragile Sanctuary",
        description: "Healing is weaker, but shops are cheaper.",
        // Only the downside is implemented -- there is no shop system in
        // the game yet for "cheaper" to apply to. Revisit once one exists.
        effects: &[RunEffect::HealingMultiplier(0.5)],
    },
    PactEntry {
        id: PactId::ScarceFortune,
        name: "Scarce Fortune",
        description: "Fewer item choices are offered, but they are of higher rarity.",
        // Only the rarity upside is implemented -- there is no
        // reward-choice-offer system in the game yet for "fewer choices"
        // to apply to. Revisit once one exists.
        effects: &[RunEffect::RareDropChanceBonus(0.25)],
    },
];

// =========================================================
// STARTING RUIN CONDITION
// =========================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
// Shared by the whole co-op party (`run_setup::SharedRunSettings`).
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum RuinConditionId {
    Uncorrupted,
    Stable,
    Tainted,
    Purified,
}

pub struct RuinConditionEntry {
    pub id: RuinConditionId,
    pub name: &'static str,
    pub description: &'static str,
    /// Ruin the player begins the run with. This is the only part of
    /// each condition currently wired into gameplay -- applied once via
    /// `ruin::RuinState::add_ruin` when the run starts (see
    /// `rooms::fallen_gate::setup_fallen_gate_player`). Any
    /// difference in Ruin *gain rate* or *reward quality* the
    /// description mentions is not implemented yet.
    pub starting_ruin: f32,
    /// Tainted and Purified are meant to be unlocked through later
    /// meta-progression ("Once unlocked, players could begin with...");
    /// no such unlock-tracking exists yet, so this is a hard-coded
    /// placeholder, same as `ClassEntry::unlocked`.
    pub unlocked: bool,
}

pub const RUIN_CONDITIONS: &[RuinConditionEntry] = &[
    RuinConditionEntry {
        id: RuinConditionId::Uncorrupted,
        name: "Uncorrupted",
        description: "Begin the run with no Ruin.",
        starting_ruin: 0.0,
        unlocked: true,
    },
    RuinConditionEntry {
        id: RuinConditionId::Stable,
        name: "Stable",
        description: "Begin the run with normal Ruin gain.",
        starting_ruin: 0.0,
        unlocked: true,
    },
    RuinConditionEntry {
        id: RuinConditionId::Tainted,
        name: "Tainted",
        description: "Begin the run with some Ruin, but find better rewards.",
        starting_ruin: 25.0,
        unlocked: false,
    },
    RuinConditionEntry {
        id: RuinConditionId::Purified,
        name: "Purified",
        description: "Begin with less starting Ruin than Tainted, but fewer high-rarity opportunities.",
        starting_ruin: 0.0,
        unlocked: false,
    },
];

// =========================================================
// LOOKUPS
// =========================================================
//
// Small helpers so the run-setup screen never has to write its own
// `.iter().find(...)` -- every catalog slice above is small (a handful
// of entries), so a linear scan is plenty fast for something that only
// runs when a menu is drawn or a click lands.

pub fn class_entry(id: ClassId) -> &'static ClassEntry {
    CLASSES
        .iter()
        .find(|entry| entry.id == id)
        .expect("every ClassId has a CLASSES entry")
}

pub fn weapon_entry(weapon_type: WeaponType) -> &'static WeaponEntry {
    WEAPONS
        .iter()
        .find(|entry| entry.weapon_type == weapon_type)
        .expect("every selectable WeaponType has a WEAPONS entry")
}

pub fn drop_pool_entry(id: DropPoolCategoryId) -> &'static DropPoolCategoryEntry {
    DROP_POOL_CATEGORIES
        .iter()
        .find(|entry| entry.id == id)
        .expect("every DropPoolCategoryId has a DROP_POOL_CATEGORIES entry")
}

pub fn relic_entry(id: RelicId) -> &'static RelicEntry {
    RELICS
        .iter()
        .find(|entry| entry.id == id)
        .expect("every RelicId has a RELICS entry")
}

pub fn pact_entry(id: PactId) -> &'static PactEntry {
    PACTS
        .iter()
        .find(|entry| entry.id == id)
        .expect("every PactId has a PACTS entry")
}

pub fn ruin_condition_entry(id: RuinConditionId) -> &'static RuinConditionEntry {
    RUIN_CONDITIONS
        .iter()
        .find(|entry| entry.id == id)
        .expect("every RuinConditionId has a RUIN_CONDITIONS entry")
}
