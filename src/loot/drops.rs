use bevy::prelude::*;

use crate::items::{ItemDefinitionId, ItemIdGenerator};
use crate::run_setup::{DropPoolCategoryId, MultiSelect, RunConfig, RunModifiers};
use crate::weapons::{GearTier, spawn_dropped_item};

use super::catalog::{create_item, item_drop_pool_category, item_tier};
use super::tables::{LootTable, LootTableId, LootTableRegistry, WeightedLootEntry};

#[derive(Clone, Copy, Debug)]
pub struct LootDropRequest {
    pub table_id: LootTableId,
    pub position: Vec3,
    pub floor: u32,
}

#[derive(Resource, Default)]
pub struct LootDropRequests(pub Vec<LootDropRequest>);

/// Small deterministic generator. Later, seed this from the run seed so loot
/// remains reproducible for networking, replays, and bug reports.
#[derive(Resource)]
pub struct LootRng {
    state: u64,
}

impl Default for LootRng {
    fn default() -> Self {
        Self {
            state: 0x4d59_5df4_d0f3_3173,
        }
    }
}

impl LootRng {
    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn range_inclusive(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            min
        } else {
            min + self.next_u32() % (max - min + 1)
        }
    }
}

pub(crate) fn process_loot_drop_requests(
    mut commands: Commands,
    mut requests: ResMut<LootDropRequests>,
    registry: Res<LootTableRegistry>,
    run_config: Res<RunConfig>,
    run_modifiers: Res<RunModifiers>,
    mut rng: ResMut<LootRng>,
    mut id_generator: ResMut<ItemIdGenerator>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let pending = std::mem::take(&mut requests.0);

    for request in pending {
        let Some(table) = registry.tables.get(&request.table_id) else {
            warn!("Missing loot table: {:?}", request.table_id);
            continue;
        };

        if rng.next_f32() > table.drop_chance {
            continue;
        }

        let rolls = rng.range_inclusive(table.min_rolls, table.max_rolls);
        for roll in 0..rolls {
            let Some(entry) = choose_entry(
                table,
                request.floor,
                &run_config.drop_pool,
                run_modifiers.rare_drop_chance_bonus,
                &mut rng,
            ) else {
                continue;
            };
            let Some(item) = create_item(entry.item, &mut id_generator) else {
                warn!("Missing item definition: {:?}", entry.item);
                continue;
            };

            let angle = roll as f32 * 2.399_963_1;
            let offset = Vec3::new(angle.cos(), 0.0, angle.sin()) * (0.45 + roll as f32 * 0.12);
            spawn_dropped_item(
                &mut commands,
                &mut meshes,
                &mut materials,
                request.position + offset + Vec3::Y * 0.65,
                item,
            );
        }
    }
}

/// Picks one eligible entry from `table`, honoring `drop_pool`
/// (`RunConfig::drop_pool` -- see `item_allowed_by_drop_pool`) and, with
/// probability `rare_drop_chance_bonus`, re-rolling a Common-tier result
/// against the table's Rare-or-better entries instead (the "rare item
/// chance" relic/pact effect -- see
/// `run_setup::RunModifiers::rare_drop_chance_bonus`). Falls back to the
/// original Common result if the table has no Rare-or-better entries
/// eligible at this floor, so the bonus never causes a table with no
/// eligible entries to drop nothing when it otherwise would have dropped
/// something.
fn choose_entry<'a>(
    table: &'a LootTable,
    floor: u32,
    drop_pool: &MultiSelect<DropPoolCategoryId>,
    rare_drop_chance_bonus: f32,
    rng: &mut LootRng,
) -> Option<&'a WeightedLootEntry> {
    let eligible: Vec<_> = table
        .entries
        .iter()
        .filter(|entry| {
            floor >= entry.min_floor
                && entry.max_floor.map_or(true, |max_floor| floor <= max_floor)
                && entry.weight > 0
                && item_allowed_by_drop_pool(entry.item, drop_pool)
        })
        .collect();

    let chosen = weighted_pick(&eligible, rng)?;

    if rare_drop_chance_bonus > 0.0
        && item_tier(chosen.item) == Some(GearTier::Common)
        && rng.next_f32() < rare_drop_chance_bonus
    {
        let upgraded: Vec<_> = eligible
            .iter()
            .copied()
            .filter(|entry| item_tier(entry.item).is_some_and(|tier| tier != GearTier::Common))
            .collect();

        if let Some(upgraded_entry) = weighted_pick(&upgraded, rng) {
            return Some(upgraded_entry);
        }
    }

    Some(chosen)
}

/// True if `item` is either outside the drop-pool taxonomy entirely (see
/// `loot::catalog::item_drop_pool_category`) or its category is
/// currently enabled in `drop_pool`.
fn item_allowed_by_drop_pool(
    item: ItemDefinitionId,
    drop_pool: &MultiSelect<DropPoolCategoryId>,
) -> bool {
    item_drop_pool_category(item).map_or(true, |category| drop_pool.is_selected(category))
}

/// Weighted-random pick among already-eligible entries. Shared by
/// `choose_entry`'s normal pick and its rare-upgrade re-roll.
fn weighted_pick<'a>(
    entries: &[&'a WeightedLootEntry],
    rng: &mut LootRng,
) -> Option<&'a WeightedLootEntry> {
    let total_weight: u32 = entries.iter().map(|entry| entry.weight).sum();
    if total_weight == 0 {
        return None;
    }

    let mut roll = rng.next_u32() % total_weight;
    for entry in entries {
        if roll < entry.weight {
            return Some(entry);
        }
        roll -= entry.weight;
    }

    None
}
