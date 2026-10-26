use std::collections::HashMap;

use bevy::prelude::*;

use crate::items::ItemDefinitionId;
use crate::weapons::{
    BELLRINGER_ID, DANCER_IN_THE_DARK_ID, HOLLOW_GOSPEL_ID, KINGSLAYER_ID, LIVING_CODEX_ID,
    RUINPIERCER_ID, SAINTS_NAIL_ID, SAINTS_TESTAMENT_ID, WIDOWS_KISS_ID,
};

use super::catalog::*;
use super::gear::{arcanist, bulwark, striker};

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LootTableId(pub &'static str);

impl LootTableId {
    pub const NORMAL_ENEMY: Self = Self("enemy.normal");
    pub const ELITE_ENEMY: Self = Self("enemy.elite");
    pub const MINIBOSS: Self = Self("enemy.miniboss");
    pub const BOSS: Self = Self("enemy.boss");
    pub const COMMON_CHEST: Self = Self("chest.common");
    pub const RARE_CHEST: Self = Self("chest.rare");
    pub const BOSS_CHEST: Self = Self("chest.boss");
}

#[derive(Clone, Debug)]
pub struct WeightedLootEntry {
    pub item: ItemDefinitionId,
    pub weight: u32,
    pub min_floor: u32,
    pub max_floor: Option<u32>,
}

impl WeightedLootEntry {
    const fn new(item: ItemDefinitionId, weight: u32, min_floor: u32) -> Self {
        Self {
            item,
            weight,
            min_floor,
            max_floor: None,
        }
    }
}

/// Builds one `WeightedLootEntry` per item in `ids` (an archetype's
/// per-tier id array from `loot::gear::{arcanist,striker,bulwark}`), all
/// sharing the same weight and minimum floor. Used to add the Arcanist /
/// Striker / Bulwark gear sets to a table without hand-writing all 57
/// entries across every table.
fn archetype_entries(
    ids: &[ItemDefinitionId],
    weight: u32,
    min_floor: u32,
) -> Vec<WeightedLootEntry> {
    ids.iter()
        .map(|id| WeightedLootEntry::new(*id, weight, min_floor))
        .collect()
}

#[derive(Clone, Debug)]
pub struct LootTable {
    pub drop_chance: f32,
    pub min_rolls: u32,
    pub max_rolls: u32,
    pub entries: Vec<WeightedLootEntry>,
}

#[derive(Resource)]
pub struct LootTableRegistry {
    pub tables: HashMap<LootTableId, LootTable>,
}

impl FromWorld for LootTableRegistry {
    fn from_world(_world: &mut World) -> Self {
        let mut tables = HashMap::new();

        // ---------------------------------------------------------------
        // Normal enemy (Ruinbound Footman, Gatebow Marksman, Cinder Hound):
        // 45% drop chance, exactly one roll. Common archetype armor is the
        // main armor reward here; a small chance at Rare archetype armor
        // opens up from floor 3 onward, mirroring the existing rare-weapon
        // gating (RARE_LONGSWORD etc. also unlock at floor 3).
        // ---------------------------------------------------------------
        let mut normal_entries = vec![
            WeightedLootEntry::new(COMMON_LONGSWORD, 12, 1),
            WeightedLootEntry::new(COMMON_GREATSWORD, 10, 1),
            WeightedLootEntry::new(COMMON_BOW, 10, 1),
            WeightedLootEntry::new(COMMON_HAMMER, 10, 1),
            WeightedLootEntry::new(COMMON_CROSSBOW, 10, 1),
            WeightedLootEntry::new(COMMON_SPELLBOOK, 10, 1),
            WeightedLootEntry::new(COMMON_DAGGERS, 10, 1),
            WeightedLootEntry::new(COMMON_HEAD, 8, 1),
            WeightedLootEntry::new(COMMON_CHEST, 8, 1),
            WeightedLootEntry::new(COMMON_HANDS, 8, 1),
            WeightedLootEntry::new(COMMON_LEGS, 8, 1),
            WeightedLootEntry::new(COMMON_FEET, 8, 1),
            WeightedLootEntry::new(RARE_LONGSWORD, 2, 3),
            WeightedLootEntry::new(RARE_GREATSWORD, 2, 3),
            WeightedLootEntry::new(RARE_BOW, 2, 3),
            WeightedLootEntry::new(RARE_HAMMER, 2, 3),
            WeightedLootEntry::new(RARE_CROSSBOW, 2, 3),
            WeightedLootEntry::new(RARE_SPELLBOOK, 2, 3),
            WeightedLootEntry::new(RARE_DAGGERS, 2, 3),
        ];
        normal_entries.extend(archetype_entries(&arcanist::COMMON, 3, 1));
        normal_entries.extend(archetype_entries(&striker::COMMON, 3, 1));
        normal_entries.extend(archetype_entries(&bulwark::COMMON, 3, 1));
        normal_entries.extend(archetype_entries(&arcanist::RARE, 1, 3));
        normal_entries.extend(archetype_entries(&striker::RARE, 1, 3));
        normal_entries.extend(archetype_entries(&bulwark::RARE, 1, 3));

        tables.insert(
            LootTableId::NORMAL_ENEMY,
            LootTable {
                drop_chance: 0.45,
                min_rolls: 1,
                max_rolls: 1,
                entries: normal_entries,
            },
        );

        // ---------------------------------------------------------------
        // Elite enemy: 85% drop chance, 1-2 rolls. Rare archetype armor is
        // the featured reward; a modest Epic chance opens at floor 4.
        // ---------------------------------------------------------------
        let mut elite_entries = vec![
            WeightedLootEntry::new(RARE_LONGSWORD, 10, 1),
            WeightedLootEntry::new(RARE_GREATSWORD, 10, 1),
            WeightedLootEntry::new(RARE_BOW, 10, 1),
            WeightedLootEntry::new(RARE_HAMMER, 10, 1),
            WeightedLootEntry::new(RARE_CROSSBOW, 10, 1),
            WeightedLootEntry::new(RARE_SPELLBOOK, 10, 1),
            WeightedLootEntry::new(RARE_DAGGERS, 10, 1),
            WeightedLootEntry::new(RARE_HEAD, 8, 1),
            WeightedLootEntry::new(RARE_CHEST, 8, 1),
            WeightedLootEntry::new(COMMON_CHEST, 5, 1),
        ];
        elite_entries.extend(archetype_entries(&arcanist::RARE, 2, 1));
        elite_entries.extend(archetype_entries(&striker::RARE, 2, 1));
        elite_entries.extend(archetype_entries(&bulwark::RARE, 2, 1));
        elite_entries.extend(archetype_entries(&arcanist::EPIC, 1, 4));
        elite_entries.extend(archetype_entries(&striker::EPIC, 1, 4));
        elite_entries.extend(archetype_entries(&bulwark::EPIC, 1, 4));

        tables.insert(
            LootTableId::ELITE_ENEMY,
            LootTable {
                drop_chance: 0.85,
                min_rolls: 1,
                max_rolls: 2,
                entries: elite_entries,
            },
        );

        // ---------------------------------------------------------------
        // Miniboss: guaranteed drop, 2-3 rolls. Rare archetype armor plus
        // an Epic archetype chance from floor 5, matching EPIC_BOW's
        // existing floor gate.
        // ---------------------------------------------------------------
        let mut miniboss_entries = vec![
            WeightedLootEntry::new(RARE_LONGSWORD, 10, 1),
            WeightedLootEntry::new(RARE_GREATSWORD, 10, 1),
            WeightedLootEntry::new(RARE_BOW, 10, 1),
            WeightedLootEntry::new(RARE_HAMMER, 10, 1),
            WeightedLootEntry::new(RARE_CROSSBOW, 10, 1),
            WeightedLootEntry::new(RARE_SPELLBOOK, 10, 1),
            WeightedLootEntry::new(RARE_DAGGERS, 10, 1),
            WeightedLootEntry::new(RARE_HEAD, 8, 1),
            WeightedLootEntry::new(RARE_CHEST, 8, 1),
            WeightedLootEntry::new(EPIC_BOW, 2, 5),
            WeightedLootEntry::new(EPIC_CROSSBOW, 2, 5),
            WeightedLootEntry::new(EPIC_SPELLBOOK, 2, 5),
            WeightedLootEntry::new(EPIC_DAGGERS, 2, 5),
        ];
        miniboss_entries.extend(archetype_entries(&arcanist::RARE, 2, 1));
        miniboss_entries.extend(archetype_entries(&striker::RARE, 2, 1));
        miniboss_entries.extend(archetype_entries(&bulwark::RARE, 2, 1));
        miniboss_entries.extend(archetype_entries(&arcanist::EPIC, 2, 5));
        miniboss_entries.extend(archetype_entries(&striker::EPIC, 2, 5));
        miniboss_entries.extend(archetype_entries(&bulwark::EPIC, 2, 5));

        tables.insert(
            LootTableId::MINIBOSS,
            LootTable {
                drop_chance: 1.0,
                min_rolls: 2,
                max_rolls: 3,
                entries: miniboss_entries,
            },
        );

        // ---------------------------------------------------------------
        // Boss (the Saint of the Broken Siege and future bosses):
        // guaranteed drop, 3-4 rolls. Rare + Epic archetype armor, plus a
        // first taste of Legendary archetype gear from floor 8.
        // ---------------------------------------------------------------
        let mut boss_entries = vec![
            WeightedLootEntry::new(RARE_LONGSWORD, 10, 1),
            WeightedLootEntry::new(RARE_GREATSWORD, 10, 1),
            WeightedLootEntry::new(RARE_BOW, 10, 1),
            WeightedLootEntry::new(RARE_HAMMER, 10, 1),
            WeightedLootEntry::new(RARE_CROSSBOW, 10, 1),
            WeightedLootEntry::new(RARE_SPELLBOOK, 10, 1),
            WeightedLootEntry::new(RARE_DAGGERS, 10, 1),
            WeightedLootEntry::new(RARE_HEAD, 8, 1),
            WeightedLootEntry::new(RARE_CHEST, 8, 1),
            WeightedLootEntry::new(EPIC_BOW, 5, 5),
            WeightedLootEntry::new(EPIC_CROSSBOW, 5, 5),
            WeightedLootEntry::new(EPIC_SPELLBOOK, 5, 5),
            WeightedLootEntry::new(EPIC_DAGGERS, 5, 5),
            // The six named Legendary Crossbows/Spellbooks, plus the
            // three named Legendary Daggers, gated to floor 8 alongside
            // the archetype gear's own Legendary tier just below.
            WeightedLootEntry::new(SAINTS_NAIL_ID, 1, 8),
            WeightedLootEntry::new(RUINPIERCER_ID, 1, 8),
            WeightedLootEntry::new(BELLRINGER_ID, 1, 8),
            WeightedLootEntry::new(HOLLOW_GOSPEL_ID, 1, 8),
            WeightedLootEntry::new(SAINTS_TESTAMENT_ID, 1, 8),
            WeightedLootEntry::new(LIVING_CODEX_ID, 1, 8),
            WeightedLootEntry::new(KINGSLAYER_ID, 1, 8),
            WeightedLootEntry::new(WIDOWS_KISS_ID, 1, 8),
            WeightedLootEntry::new(DANCER_IN_THE_DARK_ID, 1, 8),
        ];
        boss_entries.extend(archetype_entries(&arcanist::RARE, 2, 1));
        boss_entries.extend(archetype_entries(&striker::RARE, 2, 1));
        boss_entries.extend(archetype_entries(&bulwark::RARE, 2, 1));
        boss_entries.extend(archetype_entries(&arcanist::EPIC, 3, 5));
        boss_entries.extend(archetype_entries(&striker::EPIC, 3, 5));
        boss_entries.extend(archetype_entries(&bulwark::EPIC, 3, 5));
        boss_entries.extend(archetype_entries(&arcanist::LEGENDARY, 1, 8));
        boss_entries.extend(archetype_entries(&striker::LEGENDARY, 1, 8));
        boss_entries.extend(archetype_entries(&bulwark::LEGENDARY, 1, 8));

        tables.insert(
            LootTableId::BOSS,
            LootTable {
                drop_chance: 1.0,
                min_rolls: 3,
                max_rolls: 4,
                entries: boss_entries,
            },
        );

        // ---------------------------------------------------------------
        // Common chest: guaranteed, 1-2 rolls. Common archetype armor
        // alongside the existing common gear.
        // ---------------------------------------------------------------
        let mut common_chest_entries = vec![
            WeightedLootEntry::new(COMMON_LONGSWORD, 10, 1),
            WeightedLootEntry::new(COMMON_GREATSWORD, 10, 1),
            WeightedLootEntry::new(COMMON_BOW, 10, 1),
            WeightedLootEntry::new(COMMON_HAMMER, 10, 1),
            WeightedLootEntry::new(COMMON_CROSSBOW, 10, 1),
            WeightedLootEntry::new(COMMON_SPELLBOOK, 10, 1),
            WeightedLootEntry::new(COMMON_DAGGERS, 10, 1),
            WeightedLootEntry::new(COMMON_HEAD, 8, 1),
            WeightedLootEntry::new(COMMON_CHEST, 8, 1),
            WeightedLootEntry::new(COMMON_HANDS, 8, 1),
            WeightedLootEntry::new(COMMON_LEGS, 8, 1),
            WeightedLootEntry::new(COMMON_FEET, 8, 1),
        ];
        common_chest_entries.extend(archetype_entries(&arcanist::COMMON, 3, 1));
        common_chest_entries.extend(archetype_entries(&striker::COMMON, 3, 1));
        common_chest_entries.extend(archetype_entries(&bulwark::COMMON, 3, 1));

        tables.insert(
            LootTableId::COMMON_CHEST,
            LootTable {
                drop_chance: 1.0,
                min_rolls: 1,
                max_rolls: 2,
                entries: common_chest_entries,
            },
        );

        // ---------------------------------------------------------------
        // Rare chest: guaranteed, 2-3 rolls. Rare archetype armor is the
        // headline reward, with a small Epic archetype chance from floor 4.
        // ---------------------------------------------------------------
        let mut rare_chest_entries = vec![
            WeightedLootEntry::new(RARE_LONGSWORD, 10, 1),
            WeightedLootEntry::new(RARE_GREATSWORD, 10, 1),
            WeightedLootEntry::new(RARE_BOW, 10, 1),
            WeightedLootEntry::new(RARE_HAMMER, 10, 1),
            WeightedLootEntry::new(RARE_CROSSBOW, 10, 1),
            WeightedLootEntry::new(RARE_SPELLBOOK, 10, 1),
            WeightedLootEntry::new(RARE_DAGGERS, 10, 1),
            WeightedLootEntry::new(RARE_HEAD, 8, 1),
            WeightedLootEntry::new(RARE_CHEST, 8, 1),
        ];
        rare_chest_entries.extend(archetype_entries(&arcanist::RARE, 3, 1));
        rare_chest_entries.extend(archetype_entries(&striker::RARE, 3, 1));
        rare_chest_entries.extend(archetype_entries(&bulwark::RARE, 3, 1));
        rare_chest_entries.extend(archetype_entries(&arcanist::EPIC, 1, 4));
        rare_chest_entries.extend(archetype_entries(&striker::EPIC, 1, 4));
        rare_chest_entries.extend(archetype_entries(&bulwark::EPIC, 1, 4));

        tables.insert(
            LootTableId::RARE_CHEST,
            LootTable {
                drop_chance: 1.0,
                min_rolls: 2,
                max_rolls: 3,
                entries: rare_chest_entries,
            },
        );

        // ---------------------------------------------------------------
        // Boss chest: guaranteed, 3-5 rolls. The best chest in the game --
        // Rare, Epic, and (from floor 8) Legendary archetype armor all
        // available, alongside the existing best-in-slot weapon reward.
        // ---------------------------------------------------------------
        let mut boss_chest_entries = vec![
            WeightedLootEntry::new(RARE_LONGSWORD, 10, 1),
            WeightedLootEntry::new(RARE_GREATSWORD, 10, 1),
            WeightedLootEntry::new(RARE_BOW, 10, 1),
            WeightedLootEntry::new(RARE_HAMMER, 10, 1),
            WeightedLootEntry::new(RARE_CROSSBOW, 10, 1),
            WeightedLootEntry::new(RARE_SPELLBOOK, 10, 1),
            WeightedLootEntry::new(RARE_DAGGERS, 10, 1),
            WeightedLootEntry::new(RARE_HEAD, 8, 1),
            WeightedLootEntry::new(RARE_CHEST, 8, 1),
            WeightedLootEntry::new(EPIC_BOW, 7, 5),
            WeightedLootEntry::new(EPIC_CROSSBOW, 7, 5),
            WeightedLootEntry::new(EPIC_SPELLBOOK, 7, 5),
            WeightedLootEntry::new(EPIC_DAGGERS, 7, 5),
            // The best chest in the game also carries the best odds at
            // the six named Legendary Crossbows/Spellbooks, plus the
            // three named Legendary Daggers.
            WeightedLootEntry::new(SAINTS_NAIL_ID, 2, 8),
            WeightedLootEntry::new(RUINPIERCER_ID, 2, 8),
            WeightedLootEntry::new(BELLRINGER_ID, 2, 8),
            WeightedLootEntry::new(HOLLOW_GOSPEL_ID, 2, 8),
            WeightedLootEntry::new(SAINTS_TESTAMENT_ID, 2, 8),
            WeightedLootEntry::new(LIVING_CODEX_ID, 2, 8),
            WeightedLootEntry::new(KINGSLAYER_ID, 2, 8),
            WeightedLootEntry::new(WIDOWS_KISS_ID, 2, 8),
            WeightedLootEntry::new(DANCER_IN_THE_DARK_ID, 2, 8),
        ];
        boss_chest_entries.extend(archetype_entries(&arcanist::RARE, 2, 1));
        boss_chest_entries.extend(archetype_entries(&striker::RARE, 2, 1));
        boss_chest_entries.extend(archetype_entries(&bulwark::RARE, 2, 1));
        boss_chest_entries.extend(archetype_entries(&arcanist::EPIC, 4, 5));
        boss_chest_entries.extend(archetype_entries(&striker::EPIC, 4, 5));
        boss_chest_entries.extend(archetype_entries(&bulwark::EPIC, 4, 5));
        boss_chest_entries.extend(archetype_entries(&arcanist::LEGENDARY, 2, 8));
        boss_chest_entries.extend(archetype_entries(&striker::LEGENDARY, 2, 8));
        boss_chest_entries.extend(archetype_entries(&bulwark::LEGENDARY, 2, 8));

        tables.insert(
            LootTableId::BOSS_CHEST,
            LootTable {
                drop_chance: 1.0,
                min_rolls: 3,
                max_rolls: 5,
                entries: boss_chest_entries,
            },
        );

        Self { tables }
    }
}
