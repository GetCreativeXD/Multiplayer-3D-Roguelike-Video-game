mod aura;
mod crowd;
mod hostile_projectiles;
mod hound;
mod knight;
mod melee;
mod navigation;
mod ranged;
mod saint;
mod targeting;
mod telegraph;
mod traversal;
mod visuals;

pub use hound::spawn_cinder_hound;
pub use knight::spawn_black_banner_knight;
pub use ranged::spawn_gatebow_marksman;
pub use saint::spawn_saint_of_the_broken_siege;
// Multiplayer replicates enemy projectiles to clients (`net::projectiles`).
#[cfg_attr(not(feature = "multiplayer"), allow(unused_imports))]
pub use hostile_projectiles::HostileProjectile;
pub use targeting::EnemyTarget;
// Multiplayer shows enemy presentation on clients: telegraphs
// (`net::telegraphs`), the Marksman's aim line and the Saint's animation.
#[cfg(feature = "multiplayer")]
pub use ranged::{RangedEnemyCombat, aim_line_target, aim_line_transform, aim_line_visual};
#[cfg(feature = "multiplayer")]
pub use saint::SaintAnimationCue;
#[cfg(feature = "multiplayer")]
pub use telegraph::{EnemyTelegraph, EnemyTelegraphKind, attach_telegraph_visual, telegraph_scale};

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::collision::GameLayer;
use crate::combat::{EnemyBuffs, EnemyDebuffs, Health};
use crate::loot::{LootDropRequest, LootDropRequests, LootTableId};
use crate::navigation_mesh::EnemyNavAgentProfile;
use crate::player::{LifeState, PlayerCharacter};
use crate::run_setup::RunModifiers;

const ENEMY_ATTACK_DURATION: f32 = 0.18;
const ENEMY_RECOVERY_DURATION: f32 = 0.7;

const ENEMY_ATTACK_SPEED: f32 = 8.0;
pub const ENEMY_ATTACK_RANGE: f32 = 2.0;

const ENEMY_ATTACK_DAMAGE: f32 = 10.0;
const ENEMY_HIT_RANGE: f32 = 1.6;

/// Every enemy has an `EnemyTarget`: the player it is after, chosen by
/// `targeting::select_enemy_targets`.
#[derive(Component)]
#[require(EnemyTarget)]
pub struct Enemy;

#[derive(Component)]
pub struct MeleeEnemy;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
// Multiplayer sends the kind to clients so they can draw the right model
// (`net::enemies`).
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum EnemyKind {
    RuinboundFootman,
    GatebowMarksman,
    CinderHound,
    ChainWretch,
    BellHusk,
    StoneboundLaborer,
    BlackBannerKnight,
    /// Not a roster enemy -- the destructible object planted by the Black
    /// Banner Knight's Banner Plant (see `enemies::knight`). Given its own
    /// EnemyKind so it's identifiable in debugging/tooling rather than
    /// silently reusing BlackBannerKnight's.
    CorruptedBanner,
    RiftboundExecutioner,
    RuinCantor,
    GatebreakerVuld,
    ForsakenDuelist,
    HollowCastellan,
    SaintOfTheBrokenSiege,
    LastBellkeeper,
}

#[derive(Clone, Copy)]
pub struct EnemyEncounterConfig {
    pub encounter_id: u32,
    pub activation_center: Vec3,
    pub activation_half_extents: Vec2,
    /// Extra distance beyond every edge of the encounter area in which an
    /// activated enemy remains aggroed and may continue pursuing a player.
    pub aggro_extension: f32,
}

impl EnemyEncounterConfig {
    pub fn around(position: Vec3) -> Self {
        Self {
            encounter_id: 0,
            activation_center: position,
            activation_half_extents: Vec2::splat(18.0),
            aggro_extension: 50.0,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct EnemyBaseConfig {
    pub(super) kind: EnemyKind,
    pub(super) name: &'static str,
    pub(super) health: f32,
    pub(super) movement_speed: f32,
    pub(super) stop_distance: f32,
    pub(super) stagger_resistance: u32,
    pub(super) loot_table: LootTableId,
    pub(super) collider_radius: f32,
    pub(super) collider_height: f32,
}

/// Shared room/encounter awareness used by every enemy archetype.
/// Enemies remain dormant until a player enters their configured area.
#[derive(Component)]
pub struct EnemyAwareness {
    pub encounter_id: u32,
    pub home_position: Vec3,
    pub activation_center: Vec3,
    pub activation_half_extents: Vec2,
    pub aggro_extension: f32,
    pub activated: bool,
    pub engaged: bool,
    pub forced_aggro_remaining: f32,
}

impl EnemyAwareness {
    pub fn new(
        encounter_id: u32,
        home_position: Vec3,
        activation_center: Vec3,
        activation_half_extents: Vec2,
        aggro_extension: f32,
    ) -> Self {
        Self {
            encounter_id,
            home_position,
            activation_center,
            activation_half_extents,
            aggro_extension,
            activated: false,
            engaged: false,
            forced_aggro_remaining: 0.0,
        }
    }

    fn contains_player(&self, player_position: Vec3) -> bool {
        let offset = player_position - self.activation_center;

        offset.x.abs() <= self.activation_half_extents.x
            && offset.z.abs() <= self.activation_half_extents.y
    }

    fn contains_aggroed_player(&self, player_position: Vec3) -> bool {
        let offset = player_position - self.activation_center;
        let extended_half_extents =
            self.activation_half_extents + Vec2::splat(self.aggro_extension);

        offset.x.abs() <= extended_half_extents.x && offset.z.abs() <= extended_half_extents.y
    }
}

#[derive(Component)]
struct EnemyDamageTracker {
    encounter_id: u32,
    previous_health: f32,
}

#[derive(Component)]
pub struct EnemyMovement {
    pub speed: f32,
    pub stop_distance: f32,
}

pub(super) use navigation::{
    ENEMY_MAX_STEP_DOWN, EnemyGroundSafety, EnemyNavigation, EnemyNavigationGrid,
    EnemyStepPlatformConfig, authorize_enemy_edge_traversal, enemy_grounded_path_is_safe,
    enemy_landing_is_safe, enemy_step_is_safe, enemy_world_path_is_clear,
    enemy_world_path_is_clear_ignoring, finish_enemy_edge_traversal, ground_distance,
    safe_enemy_locomotion,
};
pub(super) use traversal::{CommittedTraversalPolicy, enemy_committed_step_is_safe};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EnemyState {
    Chase,
    Attack,
    Recovery,
}

#[derive(Component)]
pub struct EnemyCombat {
    pub state: EnemyState,
    pub timer: f32,
    pub attack_direction: Vec3,
    pub has_hit: bool,
    pub last_committed_position: Vec3,
    pub committed_stall_timer: f32,
}

#[derive(Component)]
pub struct Stagger {
    pub timer: f32,
    pub resistance: u32,
}

impl Stagger {
    pub fn new(resistance: u32) -> Self {
        Self {
            timer: 0.0,
            resistance,
        }
    }

    pub fn is_staggered(&self) -> bool {
        self.timer > 0.0
    }
}

/// Optional per-enemy override for the floor number used when rolling this
/// enemy's loot table on death (`handle_enemy_death` below). Absent means
/// floor 1 -- the behavior every existing enemy already had before this
/// component existed.
///
/// Ruinbound doesn't have a real floor-progression system yet, so a normal
/// dungeon enemy has no way to roll the higher-tier entries in its loot
/// table (Rare/Epic/Legendary archetype gear is gated to min_floor 3/5/8 --
/// see RUINBOUND_GEAR_ARCHETYPES.md and loot/tables.rs). This lets a room
/// (currently just `rooms::test_grounds`) spawn an enemy that rolls against
/// a specific floor for testing, without inventing that whole system now.
#[derive(Component, Clone, Copy, Debug)]
pub struct LootFloor(pub u32);

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyNavigationGrid>()
            .add_systems(
                OnExit(crate::game_state::AppState::InGame),
                crate::game_state::reset_resource::<EnemyNavigationGrid>,
            )
            .add_systems(Startup, saint::setup_saint_animations)
            .add_systems(Update, apply_enemy_strength_modifier)
            .add_systems(
                Update,
                (
                    navigation::age_dynamic_navigation_blocks,
                    activate_damaged_encounters,
                    update_enemy_activation,
                    targeting::select_enemy_targets,
                    crowd::update_enemy_crowd_steering,
                    return_disengaged_enemies,
                    update_stagger,
                    handle_enemy_death,
                    aura::apply_enemy_auras,
                    melee::enemy_combat,
                    hound::flanking_lunge_combat,
                    ranged::ranged_enemy_combat,
                    knight::black_banner_knight_combat,
                    enforce_enemy_ground_safety,
                    ranged::update_aim_telegraphs,
                    knight::update_knight_rush_telegraphs,
                    knight::update_knight_banner_channel_telegraphs,
                    hostile_projectiles::move_hostile_projectiles,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    saint::saint_fist_slam_combat,
                    saint::update_saint_slam_telegraphs,
                    saint::update_saint_shockwaves,
                    saint::publish_saint_animation_cue,
                    saint::update_saint_animation,
                )
                    .chain()
                    .after(targeting::select_enemy_targets),
            );
    }
}

/// Scales every enemy's max health the moment it spawns
/// (`Added<Enemy>`) by the run's `RunModifiers::enemy_strength_multiplier`
/// -- the "harder enemies" side of pacts like Grim Bargain. Every enemy
/// archetype (including the Saint boss) is spawned through
/// `spawn_enemy_root`, which always adds both `Enemy` and `Health`, so
/// this one generic system covers every current and future enemy type
/// without any spawn function needing to know `RunModifiers` exists.
///
/// Only health is scaled today -- no pact or relic changes enemy damage
/// output yet (see `run_setup::effects`'s module doc comment for why).
fn apply_enemy_strength_modifier(
    modifiers: Res<RunModifiers>,
    mut spawned: Query<&mut Health, Added<Enemy>>,
) {
    if modifiers.enemy_strength_multiplier == 1.0 {
        return;
    }

    for mut health in &mut spawned {
        health.max *= modifiers.enemy_strength_multiplier;
        health.current = health.max;
    }
}

/// An encounter activates when *any* player character enters its area, and
/// stays engaged while any player is inside its extended aggro area.
fn update_enemy_activation(
    time: Res<Time>,
    all_players: Query<(&Transform, &LifeState), With<PlayerCharacter>>,
    mut enemies: Query<&mut EnemyAwareness, With<Enemy>>,
) {
    if all_players.is_empty() {
        return;
    }
    // Downed or lost players don't activate or hold an encounter.
    let player_query: Vec<&Transform> = all_players
        .iter()
        .filter(|(_, life)| life.is_alive())
        .map(|(transform, _)| transform)
        .collect();

    for mut awareness in &mut enemies {
        awareness.forced_aggro_remaining =
            (awareness.forced_aggro_remaining - time.delta_secs()).max(0.0);
        if !awareness.activated
            && player_query
                .iter()
                .any(|player| awareness.contains_player(player.translation))
        {
            awareness.activated = true;
            println!("Enemy encounter activated!");
        }

        awareness.engaged = awareness.activated
            && (awareness.forced_aggro_remaining > 0.0
                || player_query
                    .iter()
                    .any(|player| awareness.contains_aggroed_player(player.translation)));
    }
}

fn activate_damaged_encounters(
    mut damaged_enemies: Query<(&Health, &mut EnemyDamageTracker), With<Enemy>>,
    mut awareness_query: Query<&mut EnemyAwareness, With<Enemy>>,
) {
    let mut damaged_encounters = Vec::new();

    for (health, mut tracker) in &mut damaged_enemies {
        if health.current < tracker.previous_health {
            damaged_encounters.push(tracker.encounter_id);
        }
        tracker.previous_health = health.current;
    }

    for encounter_id in damaged_encounters {
        for mut awareness in &mut awareness_query {
            if awareness.encounter_id == encounter_id {
                awareness.activated = true;
                awareness.engaged = true;
                awareness.forced_aggro_remaining = 10.0;
            }
        }
    }
}

fn return_disengaged_enemies(
    time: Res<Time>,
    spatial_query: SpatialQuery,
    mut navigation_grid: ResMut<EnemyNavigationGrid>,
    mut enemies: Query<
        (
            Entity,
            &mut Transform,
            &EnemyMovement,
            &mut EnemyNavigation,
            &mut EnemyCombat,
            &EnemyAwareness,
        ),
        (With<Enemy>, Without<PlayerCharacter>),
    >,
) {
    let delta = time.delta_secs();

    for (entity, mut transform, movement, mut navigation, mut combat, awareness) in &mut enemies {
        if !awareness.activated || awareness.engaged {
            continue;
        }

        let mut to_home = awareness.home_position - transform.translation;
        to_home.y = 0.0;
        let direction = to_home.normalize_or_zero();

        if to_home.length() > 0.25 {
            let desired = direction * movement.speed * delta;
            let step = safe_enemy_locomotion(
                entity,
                transform.translation,
                awareness.home_position,
                None,
                desired,
                false,
                &mut navigation,
                &spatial_query,
                &mut navigation_grid,
                delta,
            );
            transform.translation += step;
        }

        if direction != Vec3::ZERO {
            transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
        }

        combat.state = EnemyState::Chase;
        combat.timer = 0.0;
        combat.has_hit = false;
    }
}

fn handle_enemy_death(
    mut commands: Commands,
    mut requests: ResMut<LootDropRequests>,
    enemy_query: Query<
        (
            Entity,
            &Health,
            &GlobalTransform,
            Option<&LootTableId>,
            Option<&LootFloor>,
        ),
        With<Enemy>,
    >,
) {
    for (entity, health, transform, loot_table, loot_floor) in &enemy_query {
        if health.current <= 0.0 {
            println!("Enemy defeated!");

            if let Some(loot_table) = loot_table {
                requests.0.push(LootDropRequest {
                    table_id: *loot_table,
                    position: transform.translation(),
                    floor: loot_floor.map_or(1, |floor| floor.0),
                });
            }

            commands.entity(entity).despawn();
        }
    }
}

fn update_stagger(time: Res<Time>, mut enemy_query: Query<&mut Stagger, With<Enemy>>) {
    for mut stagger in &mut enemy_query {
        stagger.timer = (stagger.timer - time.delta_secs()).max(0.0);
    }
}

pub fn spawn_basic_enemy(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
) -> Entity {
    spawn_ruinbound_footman(
        commands,
        meshes,
        materials,
        position,
        EnemyEncounterConfig::around(position),
    )
}

/// Spawns the current prototype melee enemy with explicit encounter bounds.
/// Handcrafted rooms should use this form so every enemy in a room can share
/// the same activation center and half-extents.
pub fn spawn_basic_enemy_in_area(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    activation_center: Vec3,
    activation_half_extents: Vec2,
    aggro_extension: f32,
) -> Entity {
    spawn_ruinbound_footman(
        commands,
        meshes,
        materials,
        position,
        EnemyEncounterConfig {
            encounter_id: 0,
            activation_center,
            activation_half_extents,
            aggro_extension,
        },
    )
}

pub fn spawn_ruinbound_footman(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    encounter: EnemyEncounterConfig,
) -> Entity {
    let enemy = spawn_enemy_root(
        commands,
        position,
        encounter,
        EnemyBaseConfig {
            kind: EnemyKind::RuinboundFootman,
            name: "Ruinbound Footman",
            health: 60.0,
            movement_speed: 3.0,
            stop_distance: ENEMY_ATTACK_RANGE,
            stagger_resistance: 0,
            loot_table: LootTableId::NORMAL_ENEMY,
            collider_radius: 0.45,
            collider_height: 1.0,
        },
    );

    commands.entity(enemy).insert(MeleeEnemy);

    visuals::attach_footman_visual(commands, enemy, meshes, materials);
    enemy
}

pub(super) fn spawn_enemy_root(
    commands: &mut Commands,
    position: Vec3,
    encounter: EnemyEncounterConfig,
    config: EnemyBaseConfig,
) -> Entity {
    let enemy = commands
        .spawn((
            Enemy,
            config.kind,
            EnemyAwareness::new(
                encounter.encounter_id,
                position,
                encounter.activation_center,
                encounter.activation_half_extents,
                encounter.aggro_extension,
            ),
            Health::new(config.health),
            EnemyDamageTracker {
                encounter_id: encounter.encounter_id,
                previous_health: config.health,
            },
            EnemyMovement {
                speed: config.movement_speed,
                stop_distance: config.stop_distance,
            },
            EnemyNavAgentProfile::ground(
                config.collider_radius,
                config.collider_height + config.collider_radius * 2.0,
                config.movement_speed,
            ),
            EnemyNavigation::new(position),
            EnemyGroundSafety {
                last_safe_position: position,
                last_ground_distance: None,
                authorized_landing: None,
            },
            EnemyCombat {
                state: EnemyState::Chase,
                timer: 0.0,
                attack_direction: Vec3::ZERO,
                has_hit: false,
                last_committed_position: position,
                committed_stall_timer: 0.0,
            },
            Stagger::new(config.stagger_resistance),
            config.loot_table,
            Name::new(config.name),
            Transform::from_translation(position),
            RigidBody::Kinematic,
        ))
        .id();

    commands.entity(enemy).insert((
        Visibility::default(),
        Collider::capsule(config.collider_radius, config.collider_height),
        CollisionLayers::new(GameLayer::Enemy, GameLayer::World),
        EnemyDebuffs::default(),
        EnemyBuffs::default(),
    ));

    enemy
}

/// Attaches the model for `kind` to `entity` with no gameplay components.
/// Used for a joined client's copy of a host-owned enemy (`net::enemies`);
/// the host's own enemies get their visuals from their spawn functions.
/// Kinds without a dedicated model yet get a plain placeholder.
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
pub fn attach_enemy_visual(
    commands: &mut Commands,
    entity: Entity,
    kind: EnemyKind,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &AssetServer,
) {
    match kind {
        EnemyKind::RuinboundFootman => {
            visuals::attach_footman_visual(commands, entity, meshes, materials);
        }
        EnemyKind::GatebowMarksman => {
            visuals::attach_marksman_visual(commands, entity, meshes, materials);
        }
        EnemyKind::CinderHound => {
            visuals::attach_cinder_hound_visual(commands, entity, meshes, materials);
        }
        EnemyKind::BlackBannerKnight => {
            visuals::attach_black_banner_knight_visual(commands, entity, meshes, materials);
        }
        EnemyKind::CorruptedBanner => {
            visuals::attach_corrupted_banner_visual(
                commands,
                entity,
                meshes,
                materials,
                knight::BANNER_AURA_RADIUS,
            );
        }
        EnemyKind::SaintOfTheBrokenSiege => {
            saint::attach_saint_animated_visual(commands, entity, asset_server);
        }
        _ => {
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    visuals::EnemyPlaceholderVisual,
                    Name::new("Enemy Placeholder Visual"),
                    Mesh3d(meshes.add(Capsule3d::new(0.45, 1.0))),
                    MeshMaterial3d(materials.add(Color::srgb(0.45, 0.12, 0.10))),
                ));
            });
        }
    }
}

/// Universal fail-safe: no enemy may finish a frame without nearby ground.
/// Archetype movement should still use the helpers above so it can steer or
/// cancel gracefully; this catches every other action automatically.
fn enforce_enemy_ground_safety(
    spatial_query: SpatialQuery,
    mut enemies: Query<
        (
            Entity,
            &mut Transform,
            &EnemyNavigation,
            &mut EnemyGroundSafety,
        ),
        With<Enemy>,
    >,
) {
    for (entity, mut transform, navigation, mut safety) in &mut enemies {
        let current_ground = ground_distance(entity, transform.translation, &spatial_query);
        let is_navigation_traversal = navigation.allows_vertical_traversal();
        let is_authorized = safety
            .authorized_landing
            .is_some_and(|landing| ground_distance(entity, landing, &spatial_query).is_some());
        let is_grounded = current_ground.is_some_and(|distance| {
            safety
                .last_ground_distance
                .is_none_or(|previous| distance <= previous + ENEMY_MAX_STEP_DOWN)
        });

        if is_grounded || ((is_authorized || is_navigation_traversal) && current_ground.is_some()) {
            safety.last_safe_position = transform.translation;
            safety.last_ground_distance = current_ground;
            if safety
                .authorized_landing
                .is_some_and(|landing| transform.translation.distance_squared(landing) <= 0.25)
            {
                safety.authorized_landing = None;
            }
        } else if is_authorized {
            // A validated ability is currently crossing a gap. Its archetype
            // remains responsible for ending at the authorized landing point.
        } else {
            transform.translation = safety.last_safe_position;
            safety.authorized_landing = None;
        }
    }
}
