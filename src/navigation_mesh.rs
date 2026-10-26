//! Shared, map-independent navigation-mesh infrastructure.
//!
//! This layer deliberately owns world representation and path-query scheduling,
//! while enemy archetypes continue to own combat and locomotion. Keeping those
//! responsibilities separate prevents traversal experiments from changing normal
//! corner movement or attack state.

use avian_rerecast::ExcludeColliderFromNavmesh;
use avian_rerecast::prelude::*;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_landmass::{Agent3d, AgentDesiredVelocity3d, PointSampleDistance3d, prelude::*};
use bevy_rerecast::prelude::*;
use landmass_rerecast::{Island3dBundle, LandmassRerecastPlugin, NavMeshHandle3d};

use crate::collision::camera_occluder_layers;
use crate::enemies::{Enemy, EnemyAwareness, EnemyTarget, ground_distance};
use crate::player::PlayerCharacter;

/// Radius and standing height used to bake routes for the largest ordinary
/// ground enemy. Smaller enemies may use the same conservative mesh safely.
const NAV_AGENT_RADIUS: f32 = 0.62;
const NAV_AGENT_HEIGHT: f32 = 1.9;

/// Wait a few update frames so room commands, child colliders, and imported
/// scene transforms have all propagated before taking the static-world snapshot.
const WORLD_SETTLE_FRAMES: u8 = 3;

/// Persistent handle to the asynchronously generated world navmesh.
/// Holding the handle keeps the generated asset alive.
#[derive(Resource, Default)]
pub struct WorldNavmesh {
    pub handle: Option<Handle<Navmesh>>,
    archipelago: Option<Entity>,
    settle_frames: u8,
}

/// Per-archetype dimensions and capabilities consumed by the shared navigator.
/// New enemy types opt in by receiving this component; they do not need their
/// own pathfinder.
#[derive(Component, Clone, Copy)]
pub struct EnemyNavAgentProfile {
    pub radius: f32,
    pub height: f32,
    pub desired_speed: f32,
    pub max_speed: f32,
    pub route_priority: u8,
}

impl EnemyNavAgentProfile {
    pub fn ground(radius: f32, height: f32, speed: f32) -> Self {
        Self {
            radius,
            height,
            desired_speed: speed,
            max_speed: speed * 1.25,
            route_priority: 1,
        }
    }
}

/// Central performance policy for all future enemy path consumers.
///
/// Pathfinding systems must take a permit before starting a query. The small
/// fixed budget spreads expensive work across frames instead of allowing an
/// encounter activation to create a frame-time spike.
#[derive(Resource)]
pub struct NavigationQueryBudget {
    remaining_this_frame: u8,
    pub max_queries_per_frame: u8,
    pub minimum_repath_interval: f32,
    pub target_move_threshold: f32,
}

impl Default for NavigationQueryBudget {
    fn default() -> Self {
        Self {
            remaining_this_frame: 2,
            max_queries_per_frame: 2,
            minimum_repath_interval: 0.25,
            target_move_threshold: 0.8,
        }
    }
}

impl NavigationQueryBudget {
    /// Returns `true` only when this frame still has pathfinding capacity.
    pub fn try_take(&mut self) -> bool {
        if self.remaining_this_frame == 0 {
            return false;
        }
        self.remaining_this_frame -= 1;
        true
    }

    fn reset(&mut self) {
        self.remaining_this_frame = self.max_queries_per_frame;
    }
}

/// Cached route state shared by every future enemy navigator.
///
/// The target threshold and cooldown prevent small player movements from
/// invalidating every enemy path on the same frame.
#[derive(Component, Default)]
pub struct NavmeshRouteCache {
    pub requested_target: Vec3,
    pub seconds_since_request: f32,
    pub consecutive_failures: u8,
    pub has_target: bool,
    pub last_on_mesh_position: Vec3,
    pub has_on_mesh_position: bool,
    anchor_age: f32,
}

impl NavmeshRouteCache {
    pub fn should_repath(&self, target: Vec3, policy: &NavigationQueryBudget) -> bool {
        !self.has_target
            || (self.seconds_since_request >= policy.minimum_repath_interval
                && self.requested_target.distance_squared(target)
                    >= policy.target_move_threshold * policy.target_move_threshold)
    }
}

pub struct WorldNavigationPlugin;

impl Plugin for WorldNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NavmeshPlugins::default())
            .add_plugins(AvianBackendPlugin::default())
            .add_plugins(Landmass3dPlugin::default())
            .add_plugins(LandmassRerecastPlugin::default())
            .init_resource::<WorldNavmesh>()
            // A new run regenerates the navmesh for its fresh world (the
            // old archipelago entity is despawned with the run).
            .add_systems(
                OnExit(crate::game_state::AppState::InGame),
                crate::game_state::reset_resource::<WorldNavmesh>,
            )
            .init_resource::<NavigationQueryBudget>()
            .add_systems(
                First,
                (reset_navigation_query_budget, track_last_on_mesh_positions).chain(),
            )
            .add_systems(
                Update,
                (
                    exclude_camera_occluders_from_navmesh,
                    generate_static_world_navmesh_once,
                    attach_shared_navigation_agents,
                    update_navigation_targets,
                )
                    .chain(),
            );
    }
}

/// Any collider that exists purely so the third-person camera can duck
/// around decorative, walk-through geometry (see
/// `collision::camera_occluder_layers`) must never feed navmesh generation
/// -- it describes nothing that blocks movement, only what the camera
/// should treat as visually solid. Centralizing the exclusion here means
/// nothing that creates one of these colliders -- the hand-placed helpers
/// in `rooms::geometry`, or the `ColliderConstructorHierarchy` attached to
/// every GLB room scene -- has to remember to opt out itself.
fn exclude_camera_occluders_from_navmesh(
    mut commands: Commands,
    occluders: Query<
        (Entity, &CollisionLayers),
        (With<Collider>, Without<ExcludeColliderFromNavmesh>),
    >,
) {
    let occluder_layers = camera_occluder_layers();

    for (entity, layers) in &occluders {
        if *layers == occluder_layers {
            commands.entity(entity).insert(ExcludeColliderFromNavmesh);
        }
    }
}

fn track_last_on_mesh_positions(
    time: Res<Time>,
    mut agents: Query<(&Transform, &AgentState, &mut NavmeshRouteCache), With<Enemy>>,
) {
    for (transform, state, mut cache) in &mut agents {
        if !matches!(*state, AgentState::Moving) {
            continue;
        }
        cache.anchor_age += time.delta_secs();
        // Keep the anchor meaningfully inside the last navigable region rather
        // than updating it on the exact polygon boundary every frame.
        if !cache.has_on_mesh_position || cache.anchor_age >= 0.25 {
            cache.last_on_mesh_position = transform.translation;
            cache.has_on_mesh_position = true;
            cache.anchor_age = 0.0;
        }
    }
}

fn reset_navigation_query_budget(mut budget: ResMut<NavigationQueryBudget>) {
    budget.reset();
}

fn generate_static_world_navmesh_once(
    mut commands: Commands,
    mut world_navmesh: ResMut<WorldNavmesh>,
    mut generator: NavmeshGenerator,
    static_colliders: Query<(), (With<Collider>, With<RigidBody>)>,
) {
    if world_navmesh.handle.is_some() || static_colliders.is_empty() {
        return;
    }

    if world_navmesh.settle_frames < WORLD_SETTLE_FRAMES {
        world_navmesh.settle_frames += 1;
        return;
    }

    // Generation is queued once and runs in the background. Static room
    // geometry therefore never causes per-frame navmesh rebuilding.
    let settings = NavmeshSettings::from_agent_3d(NAV_AGENT_RADIUS, NAV_AGENT_HEIGHT);
    let handle = generator.generate(settings);
    let archipelago = commands
        .spawn(Archipelago3d::new(ArchipelagoOptions {
            point_sample_distance: PointSampleDistance3d {
                distance_above: 0.65,
                distance_below: 1.25,
                ..PointSampleDistance3d::from_agent_radius(NAV_AGENT_RADIUS)
            },
            ..ArchipelagoOptions::from_agent_radius(NAV_AGENT_RADIUS)
        }))
        .id();

    commands.spawn(Island3dBundle {
        island: Island,
        archipelago_ref: ArchipelagoRef3d::new(archipelago),
        nav_mesh: NavMeshHandle3d(handle.clone()),
    });

    world_navmesh.handle = Some(handle);
    world_navmesh.archipelago = Some(archipelago);
}

fn attach_shared_navigation_agents(
    mut commands: Commands,
    world_navmesh: Res<WorldNavmesh>,
    enemies: Query<(Entity, &EnemyNavAgentProfile), (With<Enemy>, Without<Agent3d>)>,
) {
    let Some(archipelago) = world_navmesh.archipelago else {
        return;
    };

    for (entity, profile) in &enemies {
        commands.entity(entity).insert((
            Agent3dBundle {
                agent: Agent3d::default(),
                archipelago_ref: ArchipelagoRef3d::new(archipelago),
                settings: AgentSettings {
                    desired_speed: profile.desired_speed,
                    max_speed: profile.max_speed,
                    radius: profile.radius,
                },
            },
            NavmeshRouteCache::default(),
        ));
    }
}

fn update_navigation_targets(
    time: Res<Time>,
    mut commands: Commands,
    mut budget: ResMut<NavigationQueryBudget>,
    spatial_query: SpatialQuery,
    player: Query<&Transform, With<PlayerCharacter>>,
    mut enemies: Query<
        (
            Entity,
            &EnemyAwareness,
            &EnemyNavAgentProfile,
            &mut NavmeshRouteCache,
            &EnemyTarget,
        ),
        With<Agent3d>,
    >,
) {
    let delta = time.delta_secs();

    // Higher-priority archetypes can later be processed in a dedicated queue.
    // All archetypes still share this hard global request ceiling.
    for (entity, awareness, _profile, mut cache, enemy_target) in &mut enemies {
        cache.seconds_since_request += delta;

        if !awareness.engaged {
            continue;
        }

        // Path toward whichever player this enemy is after
        // (`enemies::targeting`).
        let Some(player_target) = enemy_target
            .entity
            .and_then(|target| player.get(target).ok())
            .map(|transform| transform.translation)
        else {
            continue;
        };

        // Cheap time-gate first. `project_target_onto_supported_floor` below
        // walks up to 4 fallback radii x 16 samples, each doing up to 5
        // ground raycasts (as many as ~320 spatial queries) -- it used to
        // run for every engaged agent on every single frame regardless of
        // whether the result would even be used. This duplicates, cheaply,
        // the time-since-last-request half of `should_repath`'s condition
        // before paying for that projection; `should_repath`'s full check
        // (including the distance-moved half, which does need the
        // projected target) still runs below before anything is actually
        // requested, so the final repath decision is unchanged -- this only
        // skips computing a target that would have been thrown away anyway.
        // Confirmed as the primary cause of a reported frame-rate drop with
        // several engaged agents pathing near Ruined Barracks' stairs and
        // platform, where the projection's fallback loop is far more likely
        // to run to exhaustion against that uneven, multi-level geometry
        // than on flat ground (see RUINBOUND_CONTEXT, Section 60).
        if cache.has_target && cache.seconds_since_request < budget.minimum_repath_interval {
            continue;
        }

        let target = project_target_onto_supported_floor(entity, player_target, &spatial_query);
        if !cache.should_repath(target, &budget) {
            continue;
        }
        if !budget.try_take() {
            break;
        }

        commands.entity(entity).insert(AgentTarget3d::Point(target));
        cache.requested_target = target;
        cache.seconds_since_request = 0.0;
        cache.has_target = true;
    }
}

fn project_target_onto_supported_floor(
    entity: Entity,
    target: Vec3,
    spatial_query: &SpatialQuery,
) -> Vec3 {
    // A player may stand closer to a ledge than the center of an enemy can.
    // Landmass correctly rejects that exact point as TargetNotOnNavMesh, so
    // select the closest nearby point with a full enemy-sized footprint.
    for radius in [0.0_f32, 0.8, 1.2, 1.6] {
        let samples = if radius == 0.0 { 1 } else { 16 };
        for index in 0..samples {
            let angle = index as f32 * std::f32::consts::TAU / samples as f32;
            let (sin, cos) = angle.sin_cos();
            let candidate = target + Vec3::new(sin * radius, 0.0, cos * radius);
            let footprint = [
                Vec3::ZERO,
                Vec3::X * NAV_AGENT_RADIUS,
                Vec3::NEG_X * NAV_AGENT_RADIUS,
                Vec3::Z * NAV_AGENT_RADIUS,
                Vec3::NEG_Z * NAV_AGENT_RADIUS,
            ];
            let mut minimum_ground = f32::INFINITY;
            let mut maximum_ground = f32::NEG_INFINITY;
            let mut supported = true;
            for offset in footprint {
                let Some(distance) = ground_distance(entity, candidate + offset, spatial_query)
                else {
                    supported = false;
                    break;
                };
                minimum_ground = minimum_ground.min(distance);
                maximum_ground = maximum_ground.max(distance);
            }
            if supported && maximum_ground - minimum_ground <= 0.8 {
                return candidate;
            }
        }
    }
    target
}

/// Keeps the shared navigator attached for route validation while the proven
/// legacy locomotion remains authoritative.
pub fn navmesh_pursuit_step(
    desired_velocity: Option<&AgentDesiredVelocity3d>,
    agent_state: Option<&AgentState>,
    route_cache: Option<&NavmeshRouteCache>,
    position: Vec3,
    direct_direction: Vec3,
    speed: f32,
    delta_seconds: f32,
) -> (Vec3, bool) {
    if agent_state.is_some_and(|state| matches!(*state, AgentState::AgentNotOnNavMesh)) {
        if let Some(cache) = route_cache.filter(|cache| cache.has_on_mesh_position) {
            let mut to_anchor = cache.last_on_mesh_position - position;
            to_anchor.y = 0.0;
            if to_anchor.length_squared() > 0.01 {
                return (to_anchor.normalize_or_zero() * speed * delta_seconds, true);
            }
        }
    }
    if let Some(desired_velocity) = desired_velocity {
        let mut route_velocity = desired_velocity.velocity();
        route_velocity.y = 0.0;
        if route_velocity.length_squared() > 0.0001 {
            return (
                route_velocity.normalize_or_zero() * speed * delta_seconds,
                true,
            );
        }
    }
    (direct_direction * speed * delta_seconds, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_budget_is_strict_and_resets() {
        let mut budget = NavigationQueryBudget::default();
        assert!(budget.try_take());
        assert!(budget.try_take());
        assert!(!budget.try_take());
        budget.reset();
        assert!(budget.try_take());
    }

    #[test]
    fn cache_ignores_tiny_target_movements() {
        let policy = NavigationQueryBudget::default();
        let cache = NavmeshRouteCache {
            requested_target: Vec3::ZERO,
            seconds_since_request: 1.0,
            has_target: true,
            ..default()
        };
        assert!(!cache.should_repath(Vec3::new(0.2, 0.0, 0.2), &policy));
        assert!(cache.should_repath(Vec3::new(1.0, 0.0, 0.0), &policy));
    }
}
