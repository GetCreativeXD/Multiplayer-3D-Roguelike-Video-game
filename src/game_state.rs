//! Ruinbound's top-level app state: whether the player is still on the
//! run-setup screen or is actually playing a run.
//!
//! # Why this exists
//!
//! Before this state existed, the playable world (player, camera, rooms,
//! HUD) spawned unconditionally at `Startup` -- there was no "before the
//! run starts" moment to put a menu into. `AppState` gives every other
//! system a single, standard Bevy hook for that distinction:
//!
//! - `OnEnter(AppState::InGame)` is where the actual run gets built --
//!   see `rooms::fallen_gate::FallenGatePlugin` (player + camera-target +
//!   HUD + starting Ruin) and `rooms::RoomsPlugin` (the room chain).
//! - `.run_if(in_state(AppState::InGame))` is how a system opts out of
//!   running before a run has actually started (see the map/inventory/
//!   Ruin screen toggle systems in `ui::UiPlugin` and `weapons::WeaponPlugin`).
//!
//! Everything that doesn't care whether a run is in progress -- asset
//! preloading, spawning the camera itself, building the run-setup screen
//! and every other UI overlay's empty shell -- is unaffected and stays on
//! plain `Startup`/`Update` exactly as before. See
//! `ui::run_setup_screen` for the screen that drives this state's one
//! transition (`RunSetup -> InGame`).
use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Choosing class, starting weapon, item drop pool, starting relic,
    /// run pact, and starting Ruin condition. The default/starting state.
    #[default]
    RunSetup,
    /// A run is in progress: the player, camera, rooms, and HUD exist.
    InGame,
}

// =========================================================
// RUN LIFECYCLE: building up and tearing down a run
// =========================================================

/// Leaving a run (`InGame` -> `RunSetup`, e.g. after a party wipe, see
/// `player::life`) removes everything the run spawned, so the next "Begin
/// Run" builds a fresh world exactly like the first one.
///
/// How: just before a run starts (`OnExit(RunSetup)`) every existing
/// entity is recorded; when the run ends (`OnExit(InGame)`) every top-level
/// entity that wasn't there before is despawned (with its children). That
/// covers rooms, enemies, the player, the HUD, loot and effects without
/// each of them needing a marker. Entities that must outlive a run but may
/// be created during one (network connections, a client's copies of the
/// host's entities, which the host despawns itself) carry
/// `PersistAcrossRuns`. Observers and registered systems are never touched.
///
/// Run-wide *resources* are reset by the module that owns them, each in its
/// own `OnExit(AppState::InGame)` system (Ruin, rooms, map, inventory,
/// navigation, camera fades, ...).
pub struct RunLifecyclePlugin;

impl Plugin for RunLifecyclePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EntitiesBeforeRun>()
            .add_systems(OnExit(AppState::RunSetup), record_entities_before_run)
            .add_systems(OnExit(AppState::InGame), despawn_run_entities)
            .add_systems(OnEnter(AppState::RunSetup), release_cursor_for_menus);
    }
}

/// Keeps an entity alive when a run ends, even if it was created during
/// the run. See `RunLifecyclePlugin`.
#[derive(Component, Default)]
pub struct PersistAcrossRuns;

#[derive(Resource, Default)]
struct EntitiesBeforeRun(std::collections::HashSet<Entity>);

fn record_entities_before_run(world: &mut World) {
    let entities: std::collections::HashSet<Entity> = world.query::<Entity>().iter(world).collect();
    world.resource_mut::<EntitiesBeforeRun>().0 = entities;
}

fn despawn_run_entities(world: &mut World) {
    let before = std::mem::take(&mut world.resource_mut::<EntitiesBeforeRun>().0);

    let run_entities: Vec<Entity> = world
        .query_filtered::<Entity, (
            Without<ChildOf>,
            Without<PersistAcrossRuns>,
            Without<bevy::ecs::observer::Observer>,
            Without<bevy::ecs::system::SystemIdMarker>,
        )>()
        .iter(world)
        .filter(|entity| !before.contains(entity))
        .collect();

    let count = run_entities.len();
    for entity in run_entities {
        // `try_despawn`: a parent despawned earlier in this loop may have
        // already taken this entity with it.
        if let Ok(entity) = world.get_entity_mut(entity) {
            entity.despawn();
        }
    }
    info!("Run ended: despawned {count} run entities");
}

/// The run-setup screen needs a free mouse cursor (a run may have ended
/// with it captured).
fn release_cursor_for_menus(mut cursor_options: Query<&mut bevy::window::CursorOptions>) {
    for mut cursor_options in &mut cursor_options {
        cursor_options.grab_mode = bevy::window::CursorGrabMode::None;
        cursor_options.visible = true;
    }
}

/// Puts a run-wide resource back to its default when a run ends. Each
/// plugin registers this for the resources it owns:
/// `.add_systems(OnExit(AppState::InGame), reset_resource::<MyResource>)`.
pub fn reset_resource<R>(mut resource: ResMut<R>)
where
    R: Resource<Mutability = bevy::ecs::component::Mutable> + Default,
{
    *resource = R::default();
}
