mod abilities;
// Gear effects (Runebound Robes, Robes of the Hollow Star, Crown of the
// Unbroken) wired into the shared barrier / death-prevention systems from
// `crate::combat`. See armor_procs.rs for what is and isn't wired yet.
mod armor_procs;
mod bow;
mod components;
mod crossbow;
mod daggers;
mod inventory;
mod inventory_ui;
mod loot;
mod melee;
mod projectiles;
mod snapshot;
mod spellbook;
mod switching;

pub(crate) use components::InventoryItem;
pub use components::{Equipment, GearTier, PlayerCombat, PlayerInventory, Weapon, WeaponType};
pub(crate) use components::{PlayerProjectile, Projectile, ProjectileKind, WeaponAbilityState};

// Legendary item-definition ids, re-exported so UI display code (and
// anything else outside `weapons`) can identify which specific legendary
// a Crossbow/Spellbook is without needing to reach into these otherwise
// private submodules directly.
pub(crate) use crossbow::{
    BELLRINGER_ID, BELLRINGER_NAME, RUINPIERCER_ID, RUINPIERCER_NAME, SAINTS_NAIL_ID,
    SAINTS_NAIL_NAME,
};
pub(crate) use daggers::{
    DANCER_IN_THE_DARK_ID, DANCER_IN_THE_DARK_NAME, KINGSLAYER_ID, KINGSLAYER_NAME, WIDOWS_KISS_ID,
    WIDOWS_KISS_NAME,
};
pub use inventory_ui::InventoryUiState;
// A joined player's inventory moves travel to the host (`net::loot`).
pub use inventory_ui::{
    InventoryMove, InventoryMoveRequested, InventoryRequests, InventoryTarget, ItemLocation,
    MAX_QUEUED_INVENTORY_MOVES,
};
// Used by multiplayer to forward and redraw swing arcs (`net::effects`).
#[cfg(feature = "multiplayer")]
pub use melee::{MeleeArcSpawned, spawn_melee_arc};
pub(crate) use spellbook::{
    HOLLOW_GOSPEL_ID, HOLLOW_GOSPEL_NAME, LIVING_CODEX_ID, LIVING_CODEX_NAME, SAINTS_TESTAMENT_ID,
    SAINTS_TESTAMENT_NAME,
};

pub use components::{Armor, ArmorSlot};
// GearArchetype tags a piece of gear as Arcanist/Striker/Bulwark;
// GearEffect carries a unique effect's display name + rules text.
// Both are used by the archetype gear catalog in src/loot/gear/.
pub use components::{GearArchetype, GearEffect};
pub(crate) use loot::spawn_dropped_item;
// Shared by every machine's loot prompt; multiplayer replicates it
// (`net::loot`).
#[cfg(feature = "multiplayer")]
pub(crate) use loot::{LootFeedback, PickupAnimation};
pub use loot::{LootFeedbackShown, WorldLoot, spawn_armor_pickup, spawn_weapon_pickup};
pub use snapshot::{ItemSnapshot, ItemSnapshotKind, PlayerInventorySnapshot};

use bevy::prelude::*;

use crate::game_state::AppState;

// =========================================================
// CONSTANTS
// =========================================================
pub(crate) const BOW_PIERCING_DAMAGE_MULTIPLIER: f32 = 1.6;

pub(crate) const GREATSWORD_WINDUP: f32 = 0.28;

pub(crate) const ARROW_SPEED: f32 = 40.0;
pub(crate) const ARROW_LIFETIME: f32 = 3.0;
pub(crate) const ARROW_RADIUS: f32 = 0.18;

// Warden of Shades' Shadow Stag (ProjectileKind::ShadowStag). Lives here,
// not in characters::warden_abilities, so both the ability's spawn code
// and weapons::projectiles::movement's collider selection can read the
// same number -- the visible stag and its actual hitbox can never drift
// apart. See characters::warden_abilities for the ability itself.
pub(crate) const SHADOW_STAG_SIZE: Vec3 = Vec3::new(2.4, 1.8, 3.0);

// Runesage's Runic Surge (ProjectileKind::RunicSurge). Lives here for the
// same reason SHADOW_STAG_SIZE does -- both the ability's spawn code
// (characters::runesage_abilities) and move_projectiles_with_collision's
// collider selection read this one number, so the visible wave and its
// actual hitbox can never drift apart.
pub(crate) const RUNIC_SURGE_SIZE: Vec3 = Vec3::new(3.4, 1.6, 1.3);

pub(crate) const LONGSWORD_VISUAL_DURATION: f32 = 0.12;
pub(crate) const GREATSWORD_VISUAL_DURATION: f32 = 0.18;

pub(crate) const HAMMER_WINDUP: f32 = 0.42;
pub(crate) const HAMMER_VISUAL_DURATION: f32 = 0.22;
pub(crate) const HAMMER_ATTACK_FORWARD_STEP: f32 = 0.15;
pub(crate) const HAMMER_ATTACK_ANGLE_DOT: f32 = 0.15;

// Hammer's Rare ability, Seismic Strike (weapons::abilities::hammer) -- a
// low, wide ground fissure. Lives here, not in abilities::hammer, for the
// same reason SHADOW_STAG_SIZE and RUNIC_SURGE_SIZE do: both the spawn
// code and move_projectiles_with_collision's collider selection read the
// same number, so the visible crack and its actual hitbox can never
// drift apart.
pub(crate) const SEISMIC_FISSURE_SIZE: Vec3 = Vec3::new(1.6, 0.4, 2.2);

// Spellbook's "Basic Arcana" (WEAPONS.md's Apprentice's Grimoire) --
// baked into every Spellbook tier's base stats (`Weapon::spellbook`)
// rather than only Common, see that constructor's doc comment.
pub(crate) const SPELLBOOK_BASE_ABILITY_POWER: f32 = 6.0;
pub(crate) const SPELLBOOK_ABILITY_COOLDOWN_MULTIPLIER: f32 = 0.92;

// =========================================================
// PLUGIN
// =========================================================

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryUiState>();
        app.init_resource::<crate::items::ItemIdGenerator>();
        app.init_resource::<loot::LootFeedback>();
        app.add_observer(loot::show_own_loot_feedback);
        app.add_observer(inventory_ui::queue_own_inventory_moves);
        app.add_systems(
            OnExit(AppState::InGame),
            (
                crate::game_state::reset_resource::<InventoryUiState>,
                crate::game_state::reset_resource::<loot::LootFeedback>,
                crate::game_state::reset_resource::<inventory_ui::DraggedItem>,
                inventory_ui::hide_inventory_screen,
            ),
        );

        app.add_systems(
            Update,
            (
                loot::assign_missing_item_ids,
                inventory::initialize_player_inventory,
                switching::equipped_weapon_switching,
                loot::pickup_nearby_weapon,
                abilities::weapon_ability_input,
                abilities::update_blade_rush,
                daggers::update_dagger_flurry,
                daggers::update_serrated_rush,
                daggers::update_shadowstep_dance,
                melee::update_player_combat,
                melee::player_basic_attack,
                projectiles::home_locked_projectiles,
                projectiles::move_projectiles_with_collision,
                melee::update_attack_visuals,
            )
                .chain()
                // Basic attacks read each player's `PlayerIntent` (presses
                // and camera aim), which `PlayerInputSet` fills.
                .after(crate::player::PlayerInputSet),
        );

        app.add_systems(
            Update,
            (
                loot::animate_weapon_pickups,
                loot::update_interaction_prompt,
                loot::update_loot_feedback,
                // Every player character's weapons/armor/backpack for
                // display (a joined player's HUD reads its replicated copy).
                snapshot::publish_inventory_snapshots.run_if(crate::net::has_simulation_authority),
            ),
        );

        // Hammer's ability follow-up systems -- ground-slam telegraph,
        // Earthshatter's pulse sequence, its unstable-ground collapse, and
        // the shared ground-impact burst visual. Order among these four
        // doesn't matter; none of them read another's output.
        app.add_systems(
            Update,
            (
                abilities::update_ground_slam_telegraphs,
                abilities::update_earthshatter_aftershocks,
                abilities::update_unstable_ground,
                abilities::update_hammer_impact_visuals,
            ),
        );

        // Per-weapon-ability runtime state that isn't tied to a specific
        // projectile's collision -- Crossbow/Spellbook Legendary cooldowns
        // and stacks (`components::tick_weapon_ability_state`), Bellringer's
        // per-enemy Resonance decay, and the two on-death/on-kill hooks
        // (Saint's Nail's holy burst, Ruinpiercer's Cleansing Strike).
        // Order among these doesn't matter; none of them read another's
        // output.
        app.add_systems(
            Update,
            (
                components::tick_weapon_ability_state,
                crossbow::tick_resonance,
                crossbow::saints_nail_mark_burst,
                crossbow::ruinpiercer_cleansing_strike,
                daggers::tick_dagger_marks,
                daggers::update_shadow_copies,
                daggers::widows_kiss_spreading_agony,
                daggers::update_perfect_evasion,
            ),
        );

        app.add_systems(Startup, loot::spawn_interaction_prompt);
        app.add_systems(Startup, loot::spawn_loot_feedback);
        app.add_systems(Startup, inventory_ui::spawn_inventory_ui);

        // In-run inventory screen -- nothing here has a Player/Equipment
        // to act on before a run exists, so it's gated the same way the
        // map/Ruin overlays are (see `ui::UiPlugin`).
        app.add_systems(
            Update,
            (
                inventory_ui::toggle_inventory,
                inventory_ui::close_inventory_when_map_opens,
                inventory_ui::close_inventory_when_ruin_opens,
                inventory_ui::begin_item_drag,
                inventory_ui::update_drag_visual,
                inventory_ui::finish_item_drag,
                // Simulation: every player's queued moves, then every
                // player's display snapshot, which the screen reads.
                inventory_ui::apply_inventory_requests.run_if(crate::net::has_simulation_authority),
                snapshot::publish_inventory_snapshots.run_if(crate::net::has_simulation_authority),
                inventory_ui::refresh_inventory_ui,
                inventory_ui::update_item_tooltip,
                inventory::update_armor_health_bonus,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        );

        // Gear-specific barrier / death-prevention procs. Kept in their
        // own system set so armor_procs.rs stays the single place that
        // knows about these items by name.
        app.add_systems(
            Update,
            (
                armor_procs::initialize_armor_proc_state,
                armor_procs::tick_armor_proc_cooldowns,
                armor_procs::ability_use_barrier_procs,
                armor_procs::sync_unbroken_death_prevention,
            )
                .chain(),
        );
    }
}
