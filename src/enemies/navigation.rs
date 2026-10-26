use crate::collision::GameLayer;
use avian3d::math::AdjustPrecision;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Navigation diagnostics are retained for future map and enemy debugging,
/// but remain silent during normal development and play. Temporarily set this
/// to `true` when investigating a navigation issue.
const NAVIGATION_DIAGNOSTICS_ENABLED: bool = false;

/// The movement authority that supplied the last locomotion step. This is
/// deliberately shared by every enemy archetype so a future room can be
/// diagnosed without adding room-specific logging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NavigationDecision {
    Landmass,
    Direct,
    Traversal,
    HoldingPlatform,
    AuthoredDetour,
    GridAStar,
    Bypass,
    Blocked,
}
/// Persistent local navigation state shared by every grounded enemy. Keeping
/// a detour side prevents left/right oscillation while following a ledge.
#[derive(Component)]
pub(crate) struct EnemyNavigation {
    detouring: bool,
    authored_detour: bool,
    direct_route: bool,
    waypoints: Vec<Vec3>,
    waypoint_index: usize,
    repath_timer: f32,
    path_target: Vec3,
    search_limit: usize,
    path_cell_size: f32,
    crowd_steering: Vec3,
    last_position: Vec3,
    stuck_timer: f32,
    bypass_direction: Vec3,
    bypass_time_remaining: f32,
    traversal: EnemyTraversalState,
    traversal_safety_time: f32,
    traversal_combat_locked: bool,
    on_traversal_surface: bool,
    traversal_retry_cooldown: f32,
    platform_stuck_timer: f32,
    platform_last_position: Vec3,
    platform_exit_committed: bool,
    debug_decision: Option<NavigationDecision>,
    debug_elapsed: f32,
    debug_last_position: Vec3,
}

#[derive(Clone)]
enum EnemyTraversalState {
    Grounded,
    Following {
        waypoints: Vec<Vec3>,
        waypoint_index: usize,
        last_position: Vec3,
        stalled_time: f32,
        hold_platform: Option<usize>,
    },
    HoldingPlatform {
        platform_index: usize,
    },
}

impl EnemyNavigation {
    pub(crate) fn new(position: Vec3) -> Self {
        Self {
            detouring: false,
            authored_detour: false,
            direct_route: true,
            waypoints: Vec::new(),
            waypoint_index: 0,
            repath_timer: (position.x.abs() * 0.071 + position.z.abs() * 0.037) % 0.8,
            path_target: position,
            search_limit: ENEMY_PATH_INITIAL_VISITED,
            path_cell_size: ENEMY_PATH_COARSE_CELL_SIZE,
            crowd_steering: Vec3::ZERO,
            last_position: position,
            stuck_timer: 0.0,
            bypass_direction: Vec3::ZERO,
            bypass_time_remaining: 0.0,
            traversal: EnemyTraversalState::Grounded,
            traversal_safety_time: 0.0,
            traversal_combat_locked: false,
            on_traversal_surface: false,
            traversal_retry_cooldown: 0.0,
            platform_stuck_timer: 0.0,
            platform_last_position: position,
            platform_exit_committed: false,
            debug_decision: None,
            debug_elapsed: 0.0,
            debug_last_position: position,
        }
    }

    fn debug_step(
        &mut self,
        entity: Entity,
        decision: NavigationDecision,
        position: Vec3,
        target: Vec3,
        step: Vec3,
    ) -> Vec3 {
        let changed = self.debug_decision != Some(decision);
        let made_progress = position.distance(self.debug_last_position) > 0.01;
        let next_waypoint = self.waypoints.get(self.waypoint_index).copied();
        // Log authority changes immediately. If an enemy is repeatedly
        // blocked, emit a heartbeat every half-second so a future map's
        // failure is visible without printing once per frame.
        if NAVIGATION_DIAGNOSTICS_ENABLED
            && (changed || (decision == NavigationDecision::Blocked && self.debug_elapsed >= 0.5))
        {
            println!(
                "[nav-diag] entity={entity:?} decision={decision:?} pos={position:?} target={target:?} step={step:?} next_waypoint={next_waypoint:?} progress={made_progress} stuck_timer={:.2} waypoint={}/{} direct_route={} detouring={} authored_detour={}",
                self.stuck_timer,
                self.waypoint_index,
                self.waypoints.len(),
                self.direct_route,
                self.detouring,
                self.authored_detour,
            );
            self.debug_elapsed = 0.0;
        }
        self.debug_decision = Some(decision);
        self.debug_last_position = position;
        step
    }

    pub(crate) fn set_crowd_steering(&mut self, steering: Vec3) {
        self.crowd_steering = steering;
    }

    fn steer_step(&self, step: Vec3) -> Vec3 {
        let length = step.length();
        // Crowd separation is only a local adjustment to a clear direct
        // pursuit. Applying it to authored/grid waypoints lets two nearby
        // enemies bend opposite sides of the same route and oscillate.
        if !self.direct_route
            || self.detouring
            || self.authored_detour
            || length <= f32::EPSILON
            || self.crowd_steering == Vec3::ZERO
        {
            return step;
        }
        (step.normalize_or_zero() + self.crowd_steering).normalize_or_zero() * length
    }

    pub(crate) fn allows_vertical_traversal(&self) -> bool {
        !matches!(&self.traversal, EnemyTraversalState::Grounded)
            || self.traversal_safety_time > 0.0
    }

    pub(crate) fn traversal_combat_is_locked(&self) -> bool {
        self.traversal_combat_locked
    }

    pub(crate) fn is_on_traversal_surface(&self) -> bool {
        self.on_traversal_surface
    }

    fn reset_ground_route(&mut self, position: Vec3) {
        self.direct_route = true;
        self.detouring = false;
        self.authored_detour = false;
        self.waypoints.clear();
        self.waypoint_index = 0;
        self.repath_timer = 0.0;
        self.path_target = position;
        self.stuck_timer = 0.0;
        self.bypass_direction = Vec3::ZERO;
        self.bypass_time_remaining = 0.0;
    }
}

/// Last position known to have supporting ground. This is a final safety net
/// for every enemy, including future archetypes that forget to use the shared
/// ledge-aware movement helpers.
#[derive(Component)]
pub(crate) struct EnemyGroundSafety {
    pub(crate) last_safe_position: Vec3,
    pub(crate) last_ground_distance: Option<f32>,
    pub(crate) authorized_landing: Option<Vec3>,
}

const ENEMY_GROUND_RAY_HEIGHT: f32 = 0.75;
const ENEMY_GROUND_RAY_DISTANCE: f32 = 4.0;
const ENEMY_NAVIGATION_RADIUS: f32 = 0.42;
pub(crate) const ENEMY_MAX_STEP_DOWN: f32 = 0.75;
const ENEMY_MAX_STEP_UP: f32 = 0.25;
const ENEMY_PATH_COARSE_CELL_SIZE: f32 = 0.75;
const ENEMY_PATH_FINE_CELL_SIZE: f32 = 0.5;
const ENEMY_PATH_RECALCULATE_TIME: f32 = 1.5;
// Do not rebuild an active route every time a fast player moves a small
// amount. Repathing too aggressively makes a moving target flip the selected
// corner/side and produces visible back-and-forth motion.
const ENEMY_PATH_TARGET_MOVED: f32 = 3.0;
const ENEMY_PATH_INITIAL_VISITED: usize = 600;
const ENEMY_PATH_MAX_VISITED: usize = 2400;
const ENEMY_PATH_GOAL_RADIUS: f32 = 1.25;
const ENEMY_DYNAMIC_BLOCK_LIFETIME: f32 = 2.0;

#[derive(Resource, Default)]
pub(crate) struct EnemyNavigationGrid {
    ground_cache: HashMap<(i32, i32, i32), Option<f32>>,
    blocked_world_cells: HashMap<(i32, i32, i32), f32>,
    permanent_obstacle_cells: HashSet<(i32, i32, i32)>,
    authored_obstacles: Vec<AuthoredObstacleBox>,
    traversal_links: Vec<EnemyTraversalLink>,
    step_platforms: Vec<EnemyStepPlatform>,
}

#[derive(Clone)]
struct EnemyTraversalLink {
    /// Complete center-height path from one grounded landing to the other.
    /// The reverse direction is automatically available.
    waypoints: Vec<Vec3>,
}

#[derive(Clone)]
struct EnemyStepPlatform {
    center: Vec2,
    target_half_extents: Vec2,
    target_min_y: f32,
    target_max_y: f32,
    /// Each portal is ground approach, raised outside point, top landing.
    portals: Vec<[Vec3; 3]>,
}

impl EnemyStepPlatform {
    fn contains_surface_target(&self, target: Vec3) -> bool {
        let offset = Vec2::new(target.x, target.z) - self.center;
        offset.x.abs() <= self.target_half_extents.x
            && offset.y.abs() <= self.target_half_extents.y
            && target.y >= self.target_min_y
            && target.y <= self.target_max_y
    }
}

/// Room-authored description of one rectangular surface enemies may step onto.
/// This data can come from Rust room code today or Blender marker nodes later.
pub(crate) struct EnemyStepPlatformConfig {
    center: Vec2,
    obstacle_half_extents: Vec2,
    top_half_extents: Vec2,
    ground_center_y: f32,
    surface_center_y: f32,
    target_half_extents: Vec2,
    target_min_y: f32,
    target_max_y: f32,
    lane_inset: f32,
}

impl EnemyStepPlatformConfig {
    pub(crate) fn new(
        center: Vec2,
        obstacle_half_extents: Vec2,
        top_half_extents: Vec2,
        ground_center_y: f32,
        surface_center_y: f32,
    ) -> Self {
        Self {
            center,
            obstacle_half_extents,
            top_half_extents,
            ground_center_y,
            surface_center_y,
            target_half_extents: top_half_extents,
            target_min_y: surface_center_y - 0.35,
            target_max_y: surface_center_y + 0.35,
            lane_inset: ENEMY_NAVIGATION_RADIUS + 0.12,
        }
    }

    /// Expands target recognition for layered platforms with more than one
    /// valid standing ledge. Heights are actor-center world heights.
    pub(crate) fn with_target_region(
        mut self,
        half_extents: Vec2,
        minimum_y: f32,
        maximum_y: f32,
    ) -> Self {
        self.target_half_extents = half_extents;
        self.target_min_y = minimum_y.min(maximum_y);
        self.target_max_y = maximum_y.max(minimum_y);
        self
    }

    /// Allows unusual platform edges to keep traversal lanes farther inward
    /// while still accounting for the enemy body's navigation radius.
    pub(crate) fn with_lane_inset(mut self, inset: f32) -> Self {
        self.lane_inset = inset.max(ENEMY_NAVIGATION_RADIUS + 0.05);
        self
    }
}

#[derive(Clone, Copy)]
struct AuthoredObstacleBox {
    center: Vec2,
    half_extents: Vec2,
}

impl EnemyNavigationGrid {
    pub(crate) fn clear_dynamic_samples(&mut self) {
        self.ground_cache.clear();
        self.blocked_world_cells.clear();
    }

    /// Registers a room-authored obstacle footprint once. Room builders use
    /// this for large landmarks that enemies must route around as a whole.
    pub(crate) fn register_obstacle_box(&mut self, center: Vec3, half_extents: Vec2) {
        // Inflate authored obstacle footprints by the same radius used by the
        // movement shape cast. A* plans for the enemy's body rather than a
        // zero-width point, so it cannot choose a route that locomotion later
        // rejects as too close to a statue, wall, or other landmark.
        let navigation_half_extents = half_extents + Vec2::splat(ENEMY_NAVIGATION_RADIUS);
        self.authored_obstacles.push(AuthoredObstacleBox {
            center: Vec2::new(center.x, center.z),
            half_extents: navigation_half_extents,
        });
        let min_x =
            ((center.x - navigation_half_extents.x) / ENEMY_PATH_FINE_CELL_SIZE).floor() as i32;
        let max_x =
            ((center.x + navigation_half_extents.x) / ENEMY_PATH_FINE_CELL_SIZE).ceil() as i32;
        let min_z =
            ((center.z - navigation_half_extents.y) / ENEMY_PATH_FINE_CELL_SIZE).floor() as i32;
        let max_z =
            ((center.z + navigation_half_extents.y) / ENEMY_PATH_FINE_CELL_SIZE).ceil() as i32;
        let height_key = (center.y * 2.0).round() as i32;
        for x in min_x..=max_x {
            for z in min_z..=max_z {
                self.permanent_obstacle_cells.insert((x, height_key, z));
            }
        }
    }

    /// Registers a deliberate elevated route. These links are the only way
    /// ordinary enemy locomotion may leave connected ground, so nearby walls
    /// and statue geometry can never be mistaken for climbable surfaces.
    pub(crate) fn register_traversal_link(&mut self, waypoints: Vec<Vec3>) {
        if waypoints.len() >= 4 {
            self.traversal_links.push(EnemyTraversalLink { waypoints });
        }
    }

    /// Adds four reusable bidirectional crossings over a rectangular step or
    /// platform. `ground_center_y` and `surface_center_y` are enemy-root
    /// heights, not mesh heights. Room authors opt specific geometry in;
    /// unregistered walls and scenery remain ordinary obstacles.
    pub(crate) fn register_step_platform(&mut self, config: EnemyStepPlatformConfig) {
        let EnemyStepPlatformConfig {
            center,
            obstacle_half_extents,
            top_half_extents,
            ground_center_y,
            surface_center_y,
            target_half_extents,
            target_min_y,
            target_max_y,
            lane_inset,
        } = config;
        let outside_padding = ENEMY_NAVIGATION_RADIUS + 0.13;
        let outside_x = obstacle_half_extents.x + outside_padding;
        let outside_z = obstacle_half_extents.y + outside_padding;
        let inside_x = (top_half_extents.x - lane_inset).max(0.15);
        let inside_z = (top_half_extents.y - lane_inset).max(0.15);
        // Keep traversal lanes on the platform perimeter. Pulling them toward
        // the center can place every top waypoint beneath a statue, pillar,
        // or other structure mounted on the platform and disconnect the
        // visibility graph even though the outer ledge is walkable.
        let lane_x = inside_x;
        let lane_z = inside_z;

        let mut portals = Vec::new();
        for lane in [-lane_x, lane_x] {
            let forward = [
                Vec3::new(center.x + lane, ground_center_y, center.y + outside_z),
                Vec3::new(center.x + lane, surface_center_y, center.y + outside_z),
                Vec3::new(center.x + lane, surface_center_y, center.y + inside_z),
            ];
            let reverse = [
                Vec3::new(center.x + lane, ground_center_y, center.y - outside_z),
                Vec3::new(center.x + lane, surface_center_y, center.y - outside_z),
                Vec3::new(center.x + lane, surface_center_y, center.y - inside_z),
            ];
            portals.push(forward);
            portals.push(reverse);
            self.register_traversal_link(vec![
                forward[0],
                forward[1],
                forward[2],
                Vec3::new(center.x + lane, surface_center_y, center.y - inside_z),
                reverse[1],
                reverse[0],
            ]);
        }
        for lane in [-lane_z, lane_z] {
            let forward = [
                Vec3::new(center.x - outside_x, ground_center_y, center.y + lane),
                Vec3::new(center.x - outside_x, surface_center_y, center.y + lane),
                Vec3::new(center.x - inside_x, surface_center_y, center.y + lane),
            ];
            let reverse = [
                Vec3::new(center.x + outside_x, ground_center_y, center.y + lane),
                Vec3::new(center.x + outside_x, surface_center_y, center.y + lane),
                Vec3::new(center.x + inside_x, surface_center_y, center.y + lane),
            ];
            portals.push(forward);
            portals.push(reverse);
            self.register_traversal_link(vec![
                forward[0],
                forward[1],
                forward[2],
                Vec3::new(center.x + inside_x, surface_center_y, center.y + lane),
                reverse[1],
                reverse[0],
            ]);
        }
        self.step_platforms.push(EnemyStepPlatform {
            center,
            target_half_extents,
            target_min_y,
            target_max_y,
            portals,
        });
    }

    /// Returns false when any part of a grounded movement segment enters a
    /// room-authored obstacle. This keeps direct pursuit and waypoint
    /// following consistent with the cells A* considers blocked.
    pub(crate) fn segment_avoids_permanent_obstacles(&self, position: Vec3, delta: Vec3) -> bool {
        self.authored_obstacles
            .iter()
            .all(|obstacle| segment_avoids_obstacle(position, delta, *obstacle))
    }

    /// Produces the shortest one- or two-corner route around a room-authored
    /// rectangular obstacle. Unlike grid A*, this route cannot change sides
    /// because of cell rounding and stays close to the real clearance edge.
    fn authored_corner_path(&self, start: Vec3, destination: Vec3) -> Option<Vec<Vec3>> {
        let blocking_obstacle = self
            .authored_obstacles
            .iter()
            .find(|obstacle| !segment_avoids_obstacle(start, destination - start, **obstacle))?;
        let route_padding = 0.20;
        let half = blocking_obstacle.half_extents + Vec2::splat(route_padding);
        let corners = [
            Vec3::new(
                blocking_obstacle.center.x - half.x,
                start.y,
                blocking_obstacle.center.y - half.y,
            ),
            Vec3::new(
                blocking_obstacle.center.x + half.x,
                start.y,
                blocking_obstacle.center.y - half.y,
            ),
            Vec3::new(
                blocking_obstacle.center.x + half.x,
                start.y,
                blocking_obstacle.center.y + half.y,
            ),
            Vec3::new(
                blocking_obstacle.center.x - half.x,
                start.y,
                blocking_obstacle.center.y + half.y,
            ),
        ];

        let mut best_path: Option<(f32, Vec<Vec3>)> = None;
        for first in 0..corners.len() {
            let one_corner = [corners[first]];
            consider_authored_route(self, start, destination, &one_corner, &mut best_path);
            for second in 0..corners.len() {
                if first == second {
                    continue;
                }
                let two_corners = [corners[first], corners[second]];
                consider_authored_route(self, start, destination, &two_corners, &mut best_path);
            }
        }
        best_path.map(|(_, path)| path)
    }

    /// Projects a target out of an authored obstacle's enemy-clearance area.
    /// Players can stand closer to a pedestal than an enemy center can, so
    /// pathfinding must aim for the nearest reachable attack approach instead
    /// of treating the player's exact point as an impossible A* goal.
    fn reachable_destination(&self, start: Vec3, mut destination: Vec3) -> Vec3 {
        for obstacle in &self.authored_obstacles {
            let offset = Vec2::new(destination.x, destination.z) - obstacle.center;
            if offset.x.abs() > obstacle.half_extents.x || offset.y.abs() > obstacle.half_extents.y
            {
                continue;
            }

            // Leave enough room for the Footman's 0.45 collider as well as
            // shape-cast precision. A target only 0.06 outside the shared
            // 0.42 navigation margin could still fail world validation and
            // incorrectly fall back to grid A*.
            let padding = 0.16;
            let min_x = obstacle.center.x - obstacle.half_extents.x - padding;
            let max_x = obstacle.center.x + obstacle.half_extents.x + padding;
            let min_z = obstacle.center.y - obstacle.half_extents.y - padding;
            let max_z = obstacle.center.y + obstacle.half_extents.y + padding;
            let candidates = [
                Vec3::new(min_x, destination.y, destination.z.clamp(min_z, max_z)),
                Vec3::new(max_x, destination.y, destination.z.clamp(min_z, max_z)),
                Vec3::new(destination.x.clamp(min_x, max_x), destination.y, min_z),
                Vec3::new(destination.x.clamp(min_x, max_x), destination.y, max_z),
            ];
            destination = candidates
                .into_iter()
                .min_by(|a, b| {
                    let a_cost = a.distance(destination) + start.distance(*a) * 0.001;
                    let b_cost = b.distance(destination) + start.distance(*b) * 0.001;
                    a_cost.total_cmp(&b_cost)
                })
                .unwrap_or(destination);
        }
        destination
    }

    fn block_world_area(&mut self, position: Vec3) {
        let center_x = (position.x / ENEMY_PATH_FINE_CELL_SIZE).round() as i32;
        let center_y = (position.y * 2.0).round() as i32;
        let center_z = (position.z / ENEMY_PATH_FINE_CELL_SIZE).round() as i32;
        for offset_x in -1..=1 {
            for offset_z in -1..=1 {
                self.blocked_world_cells.insert(
                    (center_x + offset_x, center_y, center_z + offset_z),
                    ENEMY_DYNAMIC_BLOCK_LIFETIME,
                );
            }
        }
    }

    /// Remember the exact cell at the end of a route segment that the real
    /// enemy body could not traverse. Unlike `block_world_area`, this does
    /// not surround the current actor with a 3x3 exclusion zone, so a failed
    /// corner cannot make recovery harder for the same enemy or its neighbors.
    fn block_failed_waypoint(&mut self, position: Vec3) {
        let key = (
            (position.x / ENEMY_PATH_FINE_CELL_SIZE).round() as i32,
            (position.y * 2.0).round() as i32,
            (position.z / ENEMY_PATH_FINE_CELL_SIZE).round() as i32,
        );
        self.blocked_world_cells
            .insert(key, ENEMY_DYNAMIC_BLOCK_LIFETIME);
    }
}

fn segment_avoids_obstacle(position: Vec3, delta: Vec3, obstacle: AuthoredObstacleBox) -> bool {
    let distance = delta.length();
    let sample_count = (distance / 0.15).ceil().max(1.0) as u32;
    let start_offset = Vec2::new(position.x, position.z) - obstacle.center;
    let start_ratio = (start_offset.x.abs() / obstacle.half_extents.x)
        .max(start_offset.y.abs() / obstacle.half_extents.y);
    let mut previous_ratio = start_ratio;
    (1..=sample_count).all(|sample| {
        let point = position + delta * (sample as f32 / sample_count as f32);
        let offset = Vec2::new(point.x, point.z) - obstacle.center;
        let ratio = (offset.x.abs() / obstacle.half_extents.x)
            .max(offset.y.abs() / obstacle.half_extents.y);
        let safe = if start_ratio <= 1.0 {
            // If an enemy is already within the planning margin, permit only
            // movement that maintains or increases its clearance. This lets
            // it escape without allowing a route deeper into the obstacle.
            ratio + 0.001 >= previous_ratio
        } else {
            ratio > 1.0
        };
        previous_ratio = ratio;
        safe
    })
}

fn consider_authored_route(
    grid: &EnemyNavigationGrid,
    start: Vec3,
    destination: Vec3,
    corners: &[Vec3],
    best_path: &mut Option<(f32, Vec<Vec3>)>,
) {
    let mut previous = start;
    let mut length = 0.0;
    for point in corners.iter().copied().chain(std::iter::once(destination)) {
        let delta = point - previous;
        if !grid.segment_avoids_permanent_obstacles(previous, delta) {
            return;
        }
        length += delta.length();
        previous = point;
    }
    if best_path
        .as_ref()
        .is_none_or(|(best_length, _)| length < *best_length)
    {
        *best_path = Some((length, corners.to_vec()));
    }
}

fn path_length(start: Vec3, destination: Vec3, waypoints: &[Vec3]) -> f32 {
    let mut previous = start;
    let mut total = 0.0;
    for point in waypoints
        .iter()
        .copied()
        .chain(std::iter::once(destination))
    {
        total += previous.distance(point);
        previous = point;
    }
    total
}

fn traversal_route_cost(start: Vec3, destination: Vec3, waypoints: &[Vec3]) -> f32 {
    let mut previous = start;
    let mut total = 0.0;
    for point in waypoints
        .iter()
        .copied()
        .chain(std::iter::once(destination))
    {
        let delta = point - previous;
        // Step animation takes time, but it should not make a short crossing
        // look several metres longer than a wide route around the platform.
        total += Vec2::new(delta.x, delta.z).length() + delta.y.abs() * 0.20;
        previous = point;
    }
    total
}

struct EnemyTraversalPlan {
    waypoints: Vec<Vec3>,
    hold_platform: Option<usize>,
}

fn select_traversal_link(
    entity: Entity,
    start: Vec3,
    destination: Vec3,
    spatial_query: &SpatialQuery,
    grid: &EnemyNavigationGrid,
) -> Option<EnemyTraversalPlan> {
    // A target standing on a registered top surface needs an ascent-only
    // route. Do not run the ground-level departure test: the enemy remains in
    // HoldingPlatform after landing and pursues or attacks on that surface.
    let mut best_ascent: Option<(f32, usize, Vec<Vec3>)> = None;
    for (platform_index, platform) in grid.step_platforms.iter().enumerate() {
        if !platform.contains_surface_target(destination) {
            continue;
        }
        for portal in &platform.portals {
            let approach = portal[0] - start;
            if !grid.segment_avoids_permanent_obstacles(start, approach)
                || !enemy_grounded_path_is_safe(entity, start, approach, spatial_query)
                || !enemy_world_path_is_clear(entity, start, approach, spatial_query)
                || !enemy_world_path_is_clear(
                    entity,
                    portal[1],
                    portal[2] - portal[1],
                    spatial_query,
                )
            {
                continue;
            }
            let route = portal.to_vec();
            let cost = traversal_route_cost(start, destination, &route);
            if best_ascent
                .as_ref()
                .is_none_or(|(best_cost, _, _)| cost < *best_cost)
            {
                best_ascent = Some((cost, platform_index, route));
            }
        }
    }
    if let Some((_, platform_index, waypoints)) = best_ascent {
        return Some(EnemyTraversalPlan {
            waypoints,
            hold_platform: Some(platform_index),
        });
    }

    // Traversal is a shortcut, never a fallback for failed corner steering.
    // Requiring a complete authored ground route prevents traversal from
    // changing the behavior of unrelated walls, rubble, and ledges.
    let ground_route = grid.authored_corner_path(start, destination)?;
    let ground_cost = path_length(start, destination, &ground_route);
    let mut best: Option<(f32, Vec<Vec3>)> = None;

    for link in &grid.traversal_links {
        for reversed in [false, true] {
            let route: Vec<Vec3> = if reversed {
                link.waypoints.iter().rev().copied().collect()
            } else {
                link.waypoints.clone()
            };
            let Some(entry) = route.first().copied() else {
                continue;
            };
            let Some(exit) = route.last().copied() else {
                continue;
            };
            let approach = entry - start;
            let departure = destination - exit;
            if !grid.segment_avoids_permanent_obstacles(start, approach)
                || !grid.segment_avoids_permanent_obstacles(exit, departure)
                || !enemy_grounded_path_is_safe(entity, start, approach, spatial_query)
                || !enemy_world_path_is_clear(entity, start, approach, spatial_query)
                || !enemy_grounded_path_is_safe(entity, exit, departure, spatial_query)
                || !enemy_world_path_is_clear(entity, exit, departure, spatial_query)
            {
                continue;
            }

            // Validate every elevated horizontal section against real world
            // geometry. Vertical lift/drop sections are safe because only a
            // room-authored link can create them.
            let mut clear = true;
            for segment in route.windows(2) {
                let delta = segment[1] - segment[0];
                if Vec2::new(delta.x, delta.z).length_squared() > 0.01
                    && !enemy_world_path_is_clear(entity, segment[0], delta, spatial_query)
                {
                    clear = false;
                    break;
                }
            }
            if !clear {
                continue;
            }

            let cost = traversal_route_cost(start, destination, &route);
            // A meaningful saving avoids enemies repeatedly choosing a climb
            // for tiny differences near a corner.
            if cost + 0.35 >= ground_cost {
                continue;
            }
            if best.as_ref().is_none_or(|(best_cost, _)| cost < *best_cost) {
                best = Some((cost, route));
            }
        }
    }

    best.map(|(_, waypoints)| EnemyTraversalPlan {
        waypoints,
        hold_platform: None,
    })
}

fn surface_visibility_next_waypoint(
    entity: Entity,
    start: Vec3,
    destination: Vec3,
    platform: &EnemyStepPlatform,
    spatial_query: &SpatialQuery,
) -> Option<Vec3> {
    let mut nodes = Vec::with_capacity(platform.portals.len() + 2);
    nodes.push(start);
    for portal in &platform.portals {
        let point = portal[2];
        if nodes
            .iter()
            .all(|existing: &Vec3| existing.distance_squared(point) > 0.01)
        {
            nodes.push(point);
        }
    }
    let destination_index = nodes.len();
    nodes.push(destination);

    // This graph stays small and room-authored, so a compact O(n^2) Dijkstra
    // is cheaper and more predictable than invoking the ground A* grid. Edges
    // exist only when the real enemy-sized world sweep is clear.
    let mut distance = vec![f32::INFINITY; nodes.len()];
    let mut previous = vec![usize::MAX; nodes.len()];
    let mut visited = vec![false; nodes.len()];
    distance[0] = 0.0;

    for _ in 0..nodes.len() {
        let mut current: Option<usize> = None;
        for index in 0..nodes.len() {
            if !visited[index] && current.is_none_or(|best| distance[index] < distance[best]) {
                current = Some(index);
            }
        }
        let Some(current) = current else {
            break;
        };
        if !distance[current].is_finite() {
            break;
        }
        if current == destination_index {
            break;
        }
        visited[current] = true;

        for next in 0..nodes.len() {
            if next == current || visited[next] {
                continue;
            }
            let delta = nodes[next] - nodes[current];
            if !enemy_world_path_is_clear(entity, nodes[current], delta, spatial_query) {
                continue;
            }
            let candidate = distance[current] + delta.length();
            if candidate < distance[next] {
                distance[next] = candidate;
                previous[next] = current;
            }
        }
    }

    if !distance[destination_index].is_finite() {
        return None;
    }
    let mut waypoint_index = destination_index;
    while previous[waypoint_index] != 0 {
        waypoint_index = previous[waypoint_index];
        if waypoint_index == usize::MAX {
            return None;
        }
    }
    Some(nodes[waypoint_index])
}

fn platform_holding_step(
    entity: Entity,
    position: Vec3,
    destination: Vec3,
    movement_length: f32,
    navigation: &mut EnemyNavigation,
    spatial_query: &SpatialQuery,
    grid: &EnemyNavigationGrid,
    delta_seconds: f32,
) -> Option<Vec3> {
    let EnemyTraversalState::HoldingPlatform { platform_index } = &navigation.traversal else {
        return None;
    };
    let platform_index = *platform_index;
    let Some(platform) = grid.step_platforms.get(platform_index) else {
        navigation.traversal = EnemyTraversalState::Grounded;
        navigation.on_traversal_surface = false;
        return Some(Vec3::ZERO);
    };

    let platform_progress = position.distance(navigation.platform_last_position);
    // Physics contact around the statue produces a few centimetres of jitter.
    // Count only meaningful displacement as progress so that jitter cannot
    // continually cancel the platform recovery timer.
    if platform_progress >= 0.25 {
        navigation.platform_last_position = position;
        navigation.platform_stuck_timer = 0.0;
        navigation.platform_exit_committed = false;
    } else {
        navigation.platform_stuck_timer += delta_seconds;
    }
    if navigation.platform_stuck_timer >= 0.65 {
        navigation.platform_exit_committed = true;
    }
    let force_exit = navigation.platform_exit_committed;

    if platform.contains_surface_target(destination) {
        navigation.on_traversal_surface = true;
        navigation.traversal_combat_locked = false;
        if !force_exit {
            let mut step = destination - position;
            step.y = 0.0;
            step = step.clamp_length_max(movement_length.max(0.01));
            if enemy_world_path_is_clear(entity, position, step, spatial_query) {
                return Some(step);
            }

            // The target can be on the same layered platform but hidden behind
            // the statue. Build a tiny visibility path through the authored top
            // portals so two or more turns around the statue remain possible.
            let detour_step = surface_visibility_next_waypoint(
                entity,
                position,
                destination,
                platform,
                spatial_query,
            )
            .map_or(Vec3::ZERO, |waypoint| {
                (waypoint - position).clamp_length_max(movement_length.max(0.01))
            });
            if detour_step != Vec3::ZERO {
                return Some(detour_step);
            }
        }

        // If the enemy has already entered the mounted scenery's clearance
        // area, every long visibility edge can be rejected. Take a short,
        // ground-supported step away from the platform center first, then let
        // the visibility graph resume on the following frame.
        let mut outward = Vec3::new(
            position.x - platform.center.x,
            0.0,
            position.z - platform.center.y,
        )
        .normalize_or_zero();
        if outward == Vec3::ZERO {
            outward = Vec3::NEG_Z;
        }
        let tangent = Vec3::new(-outward.z, 0.0, outward.x);
        for direction in [outward, tangent, -tangent] {
            let recovery_step = direction * movement_length.max(0.01);
            if enemy_step_is_safe(entity, position, recovery_step, spatial_query) {
                return Some(recovery_step);
            }
        }

        return Some(Vec3::ZERO);
    }

    let mut best_exit: Option<(f32, [Vec3; 3], Vec3)> = None;
    for portal in &platform.portals {
        if ground_distance(entity, portal[0], spatial_query).is_none() {
            continue;
        }
        let Some(next_surface_waypoint) =
            surface_visibility_next_waypoint(entity, position, portal[2], platform, spatial_query)
        else {
            continue;
        };
        // The descent owns only travel to a safe landing. Requiring the
        // landing to also have a direct line all the way to the player traps
        // every enemy on top whenever the pedestal blocks that long segment.
        // Once grounded, ordinary corner navigation owns the remaining path.
        let cost = position.distance(portal[2])
            + Vec2::new(destination.x - portal[0].x, destination.z - portal[0].z).length();
        if best_exit
            .as_ref()
            .is_none_or(|(best_cost, _, _)| cost < *best_cost)
        {
            best_exit = Some((cost, *portal, next_surface_waypoint));
        }
    }

    if let Some((_, portal, next_surface_waypoint)) = best_exit {
        if position.distance_squared(portal[2]) > 0.0225 {
            return Some(
                (next_surface_waypoint - position).clamp_length_max(movement_length.max(0.01)),
            );
        }
        let waypoints = vec![portal[2], portal[1], portal[0]];
        navigation.traversal = EnemyTraversalState::Following {
            waypoints,
            waypoint_index: 0,
            last_position: position,
            stalled_time: 0.0,
            hold_platform: None,
        };
        navigation.on_traversal_surface = true;
        navigation.traversal_combat_locked = false;
        navigation.platform_stuck_timer = 0.0;
        navigation.traversal_retry_cooldown = 3.0;
        navigation.platform_exit_committed = false;
        return Some(Vec3::ZERO);
    }

    // A forced exit can begin while the enemy is still inside the central
    // scenery's clearance area, leaving every exit visibility edge blocked.
    // Keep stepping toward the outer platform ring until an exit portal
    // becomes visible.
    let mut outward = Vec3::new(
        position.x - platform.center.x,
        0.0,
        position.z - platform.center.y,
    )
    .normalize_or_zero();
    if outward == Vec3::ZERO {
        outward = Vec3::NEG_Z;
    }
    let tangent = Vec3::new(-outward.z, 0.0, outward.x);
    for direction in [outward, tangent, -tangent] {
        let recovery_step = direction * movement_length.max(0.01);
        if enemy_step_is_safe(entity, position, recovery_step, spatial_query) {
            return Some(recovery_step);
        }
    }
    Some(Vec3::ZERO)
}

fn traversal_step(
    position: Vec3,
    movement_length: f32,
    navigation: &mut EnemyNavigation,
    delta_seconds: f32,
) -> Option<Vec3> {
    let EnemyTraversalState::Following {
        waypoints,
        waypoint_index,
        last_position,
        stalled_time,
        hold_platform,
    } = &mut navigation.traversal
    else {
        return None;
    };

    let progress = position.distance(*last_position);
    *last_position = position;
    if progress <= 0.005 {
        *stalled_time += delta_seconds;
    } else {
        *stalled_time = 0.0;
    }

    while *waypoint_index < waypoints.len()
        && position.distance_squared(waypoints[*waypoint_index]) <= 0.0225
    {
        *waypoint_index += 1;
    }

    let stalled = *stalled_time >= 0.5;
    let target = waypoints.get(*waypoint_index).copied();
    if stalled || target.is_none() {
        let completed_platform = if stalled { None } else { *hold_platform };
        navigation.traversal = completed_platform
            .map_or(EnemyTraversalState::Grounded, |platform_index| {
                EnemyTraversalState::HoldingPlatform { platform_index }
            });
        navigation.traversal_safety_time = 0.25;
        navigation.traversal_combat_locked = false;
        navigation.on_traversal_surface = completed_platform.is_some();
        if completed_platform.is_some() {
            navigation.platform_last_position = position;
            navigation.platform_stuck_timer = 0.0;
        }
        navigation.platform_exit_committed = false;
        if stalled {
            navigation.traversal_retry_cooldown = 3.0;
        }
        navigation.reset_ground_route(position);
        return Some(Vec3::ZERO);
    }

    let target = target.unwrap();
    let vertical_transition = (target.y - position.y).abs() > 0.05;
    navigation.on_traversal_surface =
        *waypoint_index >= 3 && *waypoint_index <= waypoints.len().saturating_sub(3);
    navigation.traversal_combat_locked = waypoints.len() <= 3
        || vertical_transition
        || (!navigation.on_traversal_surface
            && *waypoint_index > 0
            && *waypoint_index + 1 < waypoints.len());

    Some((target - position).clamp_length_max(movement_length.max(0.01)))
}

/// Temporary collision samples are hints, not permanent room geometry. Age
/// them once per frame so an imprecise failed-step sample cannot poison the
/// shared A* grid for the rest of the session.
pub(crate) fn age_dynamic_navigation_blocks(
    time: Res<Time>,
    mut grid: ResMut<EnemyNavigationGrid>,
) {
    let delta = time.delta_secs();
    for lifetime in grid.blocked_world_cells.values_mut() {
        *lifetime -= delta;
    }
    grid.blocked_world_cells
        .retain(|_, lifetime| *lifetime > 0.0);
}

pub(crate) fn ground_distance(
    entity: Entity,
    position: Vec3,
    spatial_query: &SpatialQuery,
) -> Option<f32> {
    let direction = Dir3::new(Vec3::NEG_Y.adjust_precision()).ok()?;
    let filter = SpatialQueryFilter::from_excluded_entities([entity]).with_mask(GameLayer::World);
    spatial_query
        .cast_ray(
            (position + Vec3::Y * ENEMY_GROUND_RAY_HEIGHT).adjust_precision(),
            direction,
            ENEMY_GROUND_RAY_DISTANCE,
            true,
            &filter,
        )
        .map(|hit| hit.distance)
}

/// Returns whether a proposed enemy move remains on supported terrain and
/// does not step down farther than enemies can safely traverse.
pub(crate) fn enemy_step_is_safe(
    entity: Entity,
    position: Vec3,
    proposed_delta: Vec3,
    spatial_query: &SpatialQuery,
) -> bool {
    enemy_ground_step_is_safe(entity, position, proposed_delta, spatial_query)
        && enemy_world_path_is_clear(entity, position, proposed_delta, spatial_query)
}

fn enemy_step_is_safe_ignoring(
    entity: Entity,
    ignored_entity: Option<Entity>,
    position: Vec3,
    proposed_delta: Vec3,
    spatial_query: &SpatialQuery,
) -> bool {
    enemy_ground_step_is_safe(entity, position, proposed_delta, spatial_query)
        && enemy_world_path_is_clear_ignoring(
            entity,
            ignored_entity,
            position,
            proposed_delta,
            spatial_query,
        )
}

fn enemy_ground_step_is_safe(
    entity: Entity,
    position: Vec3,
    proposed_delta: Vec3,
    spatial_query: &SpatialQuery,
) -> bool {
    if proposed_delta.length_squared() <= f32::EPSILON {
        return true;
    }
    let Some(current_ground) = ground_distance(entity, position, spatial_query) else {
        return false;
    };
    ground_distance(entity, position + proposed_delta, spatial_query).is_some_and(|next_ground| {
        // A smaller ray distance means the next surface is higher. Enemies
        // move horizontally and cannot climb the vertical side of rubble just
        // because its top was found by the downward ground ray.
        next_ground <= current_ground + ENEMY_MAX_STEP_DOWN
            && next_ground + ENEMY_MAX_STEP_UP >= current_ground
    })
}

/// Sweeps an enemy-sized horizontal volume through world geometry. Character
/// layers are excluded, so enemies can pass one another while statues, walls,
/// pillars, and rubble remain navigation obstacles.
pub(crate) fn enemy_world_path_is_clear(
    entity: Entity,
    position: Vec3,
    proposed_delta: Vec3,
    spatial_query: &SpatialQuery,
) -> bool {
    enemy_world_path_blocker(entity, position, proposed_delta, spatial_query).is_none()
}

/// World sweep used while pursuing a known character target. Character
/// colliders remain solid to gameplay physics, but they are not navigation
/// obstacles; the target is intentionally excluded from this query.
pub(crate) fn enemy_world_path_is_clear_ignoring(
    entity: Entity,
    ignored_entity: Option<Entity>,
    position: Vec3,
    proposed_delta: Vec3,
    spatial_query: &SpatialQuery,
) -> bool {
    enemy_world_path_blocker_ignoring(
        entity,
        ignored_entity,
        position,
        proposed_delta,
        spatial_query,
    )
    .is_none()
}

/// Returns the world collider that blocks an enemy-sized horizontal sweep.
/// Keeping this separate from the boolean check lets navigation diagnostics
/// identify whether a failure came from geometry or from ground validation.
fn enemy_world_path_blocker(
    entity: Entity,
    position: Vec3,
    proposed_delta: Vec3,
    spatial_query: &SpatialQuery,
) -> Option<Entity> {
    enemy_world_path_blocker_ignoring(entity, None, position, proposed_delta, spatial_query)
}

fn enemy_world_path_blocker_ignoring(
    entity: Entity,
    ignored_entity: Option<Entity>,
    position: Vec3,
    proposed_delta: Vec3,
    spatial_query: &SpatialQuery,
) -> Option<Entity> {
    let mut horizontal_delta = proposed_delta;
    horizontal_delta.y = 0.0;
    let distance = horizontal_delta.length();
    if distance <= f32::EPSILON {
        return None;
    }
    let Ok(direction) = Dir3::new(horizontal_delta.normalize().adjust_precision()) else {
        return None;
    };
    let mut excluded_entities = vec![entity];
    if let Some(ignored_entity) = ignored_entity {
        excluded_entities.push(ignored_entity);
    }
    let filter =
        SpatialQueryFilter::from_excluded_entities(excluded_entities).with_mask(GameLayer::World);
    let mut cast_config = ShapeCastConfig::from_max_distance(distance);
    cast_config.ignore_origin_penetration = true;
    // `position` is the enemy's locomotion reference point, not the center
    // of its collision body: the ground probe consistently finds support a
    // small distance below it. Casting a sphere directly from `position`
    // therefore starts it partially inside the floor/platform and reports
    // that support collider as a horizontal blocker. Lift the probe by its
    // radius so its bottom rests at the locomotion reference plane. This
    // keeps real vertical blockers solid without making every walkable floor
    // look like an obstacle to every future enemy type.
    let cast_origin = position + Vec3::Y * ENEMY_NAVIGATION_RADIUS;
    spatial_query
        .cast_shape(
            &Collider::sphere(ENEMY_NAVIGATION_RADIUS),
            cast_origin.adjust_precision(),
            Quat::IDENTITY.adjust_precision(),
            direction,
            &cast_config,
            &filter,
        )
        .map(|hit| hit.entity)
}

/// Checks several points along grounded travel so a narrow gap cannot be
/// mistaken for safe terrain merely because the probe endpoint has ground.
pub(crate) fn enemy_grounded_path_is_safe(
    entity: Entity,
    position: Vec3,
    proposed_delta: Vec3,
    spatial_query: &SpatialQuery,
) -> bool {
    let sample_count = (proposed_delta.length() / 0.3).ceil().max(1.0) as u32;
    (1..=sample_count).all(|sample| {
        let fraction = sample as f32 / sample_count as f32;
        enemy_ground_step_is_safe(entity, position, proposed_delta * fraction, spatial_query)
    })
}

/// Chooses a short, supported escape heading around either a world collider
/// or a ledge. Testing a fan of headings is more robust than a single
/// left/right choice when the enemy is already close to a wide landmark.
fn local_bypass_direction(
    entity: Entity,
    position: Vec3,
    forward: Vec3,
    destination: Vec3,
    spatial_query: &SpatialQuery,
) -> Vec3 {
    let forward = forward.normalize_or_zero();
    if forward == Vec3::ZERO {
        return Vec3::ZERO;
    }
    let left = Vec3::new(-forward.z, 0.0, forward.x);
    let candidates = [
        (forward + left).normalize_or_zero(),
        (forward - left).normalize_or_zero(),
        left,
        -left,
        (-forward + left).normalize_or_zero(),
        (-forward - left).normalize_or_zero(),
        -forward,
    ];
    let probe_distance = 0.9;
    let mut best: Option<(f32, Vec3)> = None;
    for direction in candidates {
        let probe = direction * probe_distance;
        if !enemy_grounded_path_is_safe(entity, position, probe, spatial_query)
            || !enemy_world_path_is_clear(entity, position, probe, spatial_query)
        {
            continue;
        }
        let score = (position + probe).distance_squared(destination);
        if best
            .as_ref()
            .is_none_or(|(best_score, _)| score < *best_score)
        {
            best = Some((score, direction));
        }
    }
    best.map_or(Vec3::ZERO, |(_, direction)| direction)
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct PathNode {
    coordinate: (i32, i32),
    cost: i32,
    estimate: i32,
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        (other.cost + other.estimate)
            .cmp(&(self.cost + self.estimate))
            .then_with(|| other.estimate.cmp(&self.estimate))
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn world_path_coordinate(position: Vec3, cell_size: f32) -> (i32, i32) {
    (
        (position.x / cell_size).round() as i32,
        (position.z / cell_size).round() as i32,
    )
}

fn path_coordinate_world(height: f32, coordinate: (i32, i32), cell_size: f32) -> Vec3 {
    Vec3::new(
        coordinate.0 as f32 * cell_size,
        height,
        coordinate.1 as f32 * cell_size,
    )
}

fn path_heuristic(a: (i32, i32), b: (i32, i32)) -> i32 {
    let dx = (a.0 - b.0).abs();
    let dz = (a.1 - b.1).abs();
    14 * dx.min(dz) + 10 * (dx - dz).abs()
}

fn cached_navigation_ground(
    entity: Entity,
    position: Vec3,
    spatial_query: &SpatialQuery,
    grid: &mut EnemyNavigationGrid,
) -> Option<f32> {
    let key = (
        (position.x / ENEMY_PATH_FINE_CELL_SIZE).round() as i32,
        (position.y * 2.0).round() as i32,
        (position.z / ENEMY_PATH_FINE_CELL_SIZE).round() as i32,
    );
    if grid.blocked_world_cells.contains_key(&key) || grid.permanent_obstacle_cells.contains(&key) {
        return None;
    }
    if let Some(cached) = grid.ground_cache.get(&key) {
        return *cached;
    }
    let ground = ground_distance(entity, position, spatial_query);
    grid.ground_cache.insert(key, ground);
    ground
}

fn build_ground_path(
    entity: Entity,
    start: Vec3,
    destination: Vec3,
    spatial_query: &SpatialQuery,
    grid: &mut EnemyNavigationGrid,
    search_limit: usize,
    cell_size: f32,
) -> (Vec<Vec3>, bool) {
    let start_coordinate = world_path_coordinate(start, cell_size);
    let goal = world_path_coordinate(destination, cell_size);
    if goal == start_coordinate {
        return (Vec::new(), true);
    }

    let mut open = BinaryHeap::new();
    let mut costs = HashMap::new();
    let mut came_from = HashMap::new();
    open.push(PathNode {
        coordinate: start_coordinate,
        cost: 0,
        estimate: path_heuristic(start_coordinate, goal),
    });
    costs.insert(start_coordinate, 0);

    // An enemy can already be inside the navigation clearance margin when a
    // landmark is registered or after it approaches by direct movement. Seed
    // nearby clear cells that the real enemy-sized body can reach directly.
    // This lets A* escape the margin instead of declaring every neighbor
    // blocked and returning an empty path.
    let start_cell_world = path_coordinate_world(start.y, start_coordinate, cell_size);
    if cached_navigation_ground(entity, start_cell_world, spatial_query, grid).is_none() {
        for radius in 1_i32..=2 {
            for offset_x in -radius..=radius {
                for offset_z in -radius..=radius {
                    if offset_x.abs() != radius && offset_z.abs() != radius {
                        continue;
                    }
                    let candidate = (start_coordinate.0 + offset_x, start_coordinate.1 + offset_z);
                    let candidate_world = path_coordinate_world(start.y, candidate, cell_size);
                    if cached_navigation_ground(entity, candidate_world, spatial_query, grid)
                        .is_none()
                    {
                        continue;
                    }
                    let candidate_delta = candidate_world - start;
                    if !enemy_grounded_path_is_safe(entity, start, candidate_delta, spatial_query)
                        || !enemy_world_path_is_clear(entity, start, candidate_delta, spatial_query)
                    {
                        continue;
                    }
                    let seed_cost = path_heuristic(start_coordinate, candidate);
                    costs.insert(candidate, seed_cost);
                    came_from.insert(candidate, start_coordinate);
                    open.push(PathNode {
                        coordinate: candidate,
                        cost: seed_cost,
                        estimate: path_heuristic(candidate, goal),
                    });
                }
            }
            if came_from
                .values()
                .any(|previous| *previous == start_coordinate)
            {
                break;
            }
        }
    }

    let mut reached = None;
    let mut best_coordinate = start_coordinate;
    let mut best_estimate = path_heuristic(start_coordinate, goal);
    let neighbors = [
        (1, 0, 10),
        (-1, 0, 10),
        (0, 1, 10),
        (0, -1, 10),
        (1, 1, 14),
        (1, -1, 14),
        (-1, 1, 14),
        (-1, -1, 14),
    ];
    let mut visited = 0;

    while let Some(node) = open.pop() {
        visited += 1;
        if visited > search_limit {
            break;
        }
        let node_world = path_coordinate_world(start.y, node.coordinate, cell_size);
        if node.estimate < best_estimate {
            best_estimate = node.estimate;
            best_coordinate = node.coordinate;
        }
        let horizontal_goal_distance =
            Vec2::new(destination.x - node_world.x, destination.z - node_world.z).length();
        if horizontal_goal_distance <= ENEMY_PATH_GOAL_RADIUS {
            reached = Some(node.coordinate);
            break;
        }
        if costs
            .get(&node.coordinate)
            .is_some_and(|known| node.cost > *known)
        {
            continue;
        }

        // The coarse grid cell nearest the enemy can overlap an authored
        // obstacle even when the enemy's real position is just outside it.
        // Always seed A* from the real starting position; every neighbor still
        // goes through the normal blocked-cell checks.
        let node_ground = if node.coordinate == start_coordinate {
            ground_distance(entity, start, spatial_query)
        } else {
            cached_navigation_ground(entity, node_world, spatial_query, grid)
        };
        let Some(node_ground) = node_ground else {
            continue;
        };
        for (offset_x, offset_z, movement_cost) in neighbors {
            let next = (node.coordinate.0 + offset_x, node.coordinate.1 + offset_z);
            let next_world = path_coordinate_world(start.y, next, cell_size);
            let Some(next_ground) =
                cached_navigation_ground(entity, next_world, spatial_query, grid)
            else {
                continue;
            };
            if next_ground > node_ground + ENEMY_MAX_STEP_DOWN
                || next_ground + ENEMY_MAX_STEP_UP < node_ground
            {
                continue;
            }
            // Endpoint-only ground checks can bridge a narrow crack: both
            // cells have floor, but the segment between them does not. Use
            // the same sampled transition test as real locomotion so A* can
            // never return a path that waypoint following immediately rejects.
            let edge_start = if node.coordinate == start_coordinate {
                start
            } else {
                node_world
            };
            if !enemy_grounded_path_is_safe(
                entity,
                edge_start,
                next_world - edge_start,
                spatial_query,
            ) {
                continue;
            }
            if offset_x != 0 && offset_z != 0 {
                let side_x = path_coordinate_world(
                    start.y,
                    (node.coordinate.0 + offset_x, node.coordinate.1),
                    cell_size,
                );
                let side_z = path_coordinate_world(
                    start.y,
                    (node.coordinate.0, node.coordinate.1 + offset_z),
                    cell_size,
                );
                if cached_navigation_ground(entity, side_x, spatial_query, grid).is_none()
                    || cached_navigation_ground(entity, side_z, spatial_query, grid).is_none()
                {
                    continue;
                }
            }
            let next_cost = node.cost + movement_cost;
            if costs.get(&next).is_none_or(|known| next_cost < *known) {
                costs.insert(next, next_cost);
                came_from.insert(next, node.coordinate);
                open.push(PathNode {
                    coordinate: next,
                    cost: next_cost,
                    estimate: path_heuristic(next, goal),
                });
            }
        }
    }

    let mut current = reached.unwrap_or(best_coordinate);
    if current == start_coordinate {
        return (Vec::new(), false);
    }
    let mut reversed = vec![path_coordinate_world(start.y, current, cell_size)];
    while current != start_coordinate {
        let Some(previous) = came_from.get(&current).copied() else {
            return (Vec::new(), false);
        };
        current = previous;
        if current != start_coordinate {
            reversed.push(path_coordinate_world(start.y, current, cell_size));
        }
    }
    reversed.reverse();
    (reversed, reached.is_some())
}

/// Reusable ground navigation for chase, retreat, circling, and returning
/// home. It uses direct movement on clear floor and sampled A* waypoints when
/// a ledge separates the enemy from its destination.
pub(crate) fn safe_enemy_locomotion(
    entity: Entity,
    position: Vec3,
    destination: Vec3,
    ignored_entity: Option<Entity>,
    desired_delta: Vec3,
    navmesh_route_available: bool,
    navigation: &mut EnemyNavigation,
    spatial_query: &SpatialQuery,
    grid: &mut EnemyNavigationGrid,
    delta_seconds: f32,
) -> Vec3 {
    navigation.debug_elapsed += delta_seconds;
    navigation.traversal_safety_time = (navigation.traversal_safety_time - delta_seconds).max(0.0);
    // Preserve the full fallback window while an enemy is still returning to
    // ground. The cooldown starts counting only after Grounded is restored,
    // ensuring the same failed shortcut cannot be selected on landing.
    if matches!(&navigation.traversal, EnemyTraversalState::Grounded) {
        navigation.traversal_retry_cooldown =
            (navigation.traversal_retry_cooldown - delta_seconds).max(0.0);
    }

    if desired_delta.length_squared() <= f32::EPSILON {
        return navigation.debug_step(
            entity,
            NavigationDecision::Blocked,
            position,
            destination,
            Vec3::ZERO,
        );
    }

    if let Some(step) = platform_holding_step(
        entity,
        position,
        destination,
        desired_delta.length(),
        navigation,
        spatial_query,
        grid,
        delta_seconds,
    ) {
        return navigation.debug_step(
            entity,
            NavigationDecision::HoldingPlatform,
            position,
            destination,
            step,
        );
    }

    if navigation.traversal_retry_cooldown <= 0.0
        && matches!(&navigation.traversal, EnemyTraversalState::Grounded)
    {
        if let Some(plan) =
            select_traversal_link(entity, position, destination, spatial_query, grid)
        {
            navigation.traversal = EnemyTraversalState::Following {
                waypoints: plan.waypoints,
                waypoint_index: 0,
                last_position: position,
                stalled_time: 0.0,
                hold_platform: plan.hold_platform,
            };
            navigation.traversal_combat_locked = false;
            navigation.on_traversal_surface = false;
            navigation.reset_ground_route(position);
        }
    }

    if let Some(step) = traversal_step(position, desired_delta.length(), navigation, delta_seconds)
    {
        return navigation.debug_step(
            entity,
            NavigationDecision::Traversal,
            position,
            destination,
            step,
        );
    }

    // Landmass owns ordinary pursuit whenever it has produced a route. Keep
    // the proven physics and continuous-ground validation as the final safety
    // gate, but do not replace the navmesh heading with a direct line to the
    // player. That replacement was what sent enemies into the statue and the
    // courtyard gap despite Landmass already steering around them.
    // Landmass is authoritative only while this navigator is still on a
    // stable direct route. Once the local safety layer has selected a detour,
    // do not let a fresh Landmass velocity immediately reclaim control on the
    // next frame. That handoff was causing Landmass -> A* -> bypass ->
    // Landmass oscillation around valid obstacle corners.
    if navmesh_route_available
        && navigation.direct_route
        && !navigation.detouring
        && navigation.waypoints.is_empty()
    {
        let ground_clear =
            enemy_grounded_path_is_safe(entity, position, desired_delta, spatial_query);
        let world_clear = enemy_world_path_is_clear_ignoring(
            entity,
            ignored_entity,
            position,
            desired_delta,
            spatial_query,
        );
        if ground_clear && world_clear {
            navigation.reset_ground_route(position);
            return navigation.debug_step(
                entity,
                NavigationDecision::Landmass,
                position,
                destination,
                desired_delta,
            );
        }
    }

    let requested_destination = destination;
    let projected_destination = grid.reachable_destination(position, requested_destination);
    let destination =
        if navigation.authored_detour && navigation.waypoint_index < navigation.waypoints.len() {
            // Keep the obstacle-edge approach selected when this authored route
            // was built. Reprojecting every frame can flip between adjacent edges
            // as the player moves near a corner, producing a short reversal.
            navigation.path_target
        } else {
            projected_destination
        };
    let movement_length = desired_delta.length();
    let mut navigation_direction = destination - position;
    navigation_direction.y = 0.0;
    let desired_delta = navigation_direction.normalize_or_zero() * movement_length;
    if desired_delta.length_squared() <= f32::EPSILON {
        return Vec3::ZERO;
    }

    // Physics is the source of truth for ordinary grounded pursuit. A route
    // may have been created from an earlier stale sample, a coarse cell
    // boundary, or a temporary dynamic block. Before following that route,
    // retry the short straight move toward the requested target. If the
    // enemy-sized sweep and supported-ground test both pass, the old route is
    // unnecessary and is discarded. This keeps flat floors behaving like
    // flat floors while still preserving A* for a real blocking collider or
    // an actual ledge.
    let direct_to_target = requested_destination - position;
    let mut direct_to_target = direct_to_target;
    direct_to_target.y = 0.0;
    let direct_to_target = direct_to_target.normalize_or_zero() * movement_length;
    let direct_probe = direct_to_target.normalize_or_zero()
        * ENEMY_PATH_COARSE_CELL_SIZE.max(direct_to_target.length());
    let direct_ground_clear = direct_to_target.length_squared() > f32::EPSILON
        && enemy_grounded_path_is_safe(entity, position, direct_probe, spatial_query);
    let direct_blocker = if direct_to_target.length_squared() > f32::EPSILON {
        enemy_world_path_blocker_ignoring(
            entity,
            ignored_entity,
            position,
            direct_probe,
            spatial_query,
        )
    } else {
        None
    };
    if !direct_ground_clear || direct_blocker.is_some() {
        // The decision log previously showed the fallback result but not why
        // the straight probe was rejected. Emit one targeted record when the
        // authority changes so a real collider can be distinguished from a
        // ground-height/sample failure without flooding the terminal.
        if NAVIGATION_DIAGNOSTICS_ENABLED
            && navigation.debug_decision != Some(NavigationDecision::GridAStar)
        {
            let current_ground = ground_distance(entity, position, spatial_query);
            let next_ground = ground_distance(entity, position + direct_probe, spatial_query);
            println!(
                "[nav-reject] entity={entity:?} pos={position:?} probe={direct_probe:?} ground_clear={direct_ground_clear} current_ground={current_ground:?} next_ground={next_ground:?} blocker={direct_blocker:?}"
            );
        }
    }
    if direct_to_target.length_squared() > f32::EPSILON
        && direct_ground_clear
        && direct_blocker.is_none()
    {
        navigation.reset_ground_route(position);
        return navigation.debug_step(
            entity,
            NavigationDecision::Direct,
            position,
            destination,
            direct_to_target,
        );
    }

    let progress = position.distance(navigation.last_position);
    navigation.last_position = position;
    if progress >= 2.0 {
        navigation.direct_route = true;
        navigation.detouring = false;
        navigation.authored_detour = false;
        navigation.waypoints.clear();
        navigation.waypoint_index = 0;
        navigation.repath_timer = 0.0;
        navigation.path_target = position;
        navigation.search_limit = ENEMY_PATH_INITIAL_VISITED;
        navigation.path_cell_size = ENEMY_PATH_COARSE_CELL_SIZE;
        navigation.stuck_timer = 0.0;
        grid.clear_dynamic_samples();
    } else if progress <= 0.01 {
        navigation.stuck_timer += delta_seconds;
    } else {
        navigation.stuck_timer = 0.0;
    }

    if navigation.stuck_timer >= 0.35 {
        // An authored route is still only a plan. If physics reports no real
        // progress, release it and use the same body-sized recovery fan as
        // every future enemy type instead of retrying a blocked corner.
        navigation.direct_route = false;
        navigation.detouring = true;
        navigation.authored_detour = false;
        navigation.waypoints.clear();
        navigation.waypoint_index = 0;
        navigation.repath_timer = 0.0;
        navigation.stuck_timer = 0.0;
        grid.clear_dynamic_samples();

        let bypass_probe_distance = 0.9;
        let remembered_probe = navigation.bypass_direction * bypass_probe_distance;
        let remembered_side_is_safe = navigation.bypass_direction != Vec3::ZERO
            && enemy_grounded_path_is_safe(entity, position, remembered_probe, spatial_query)
            && enemy_world_path_is_clear_ignoring(
                entity,
                ignored_entity,
                position,
                remembered_probe,
                spatial_query,
            );

        if !remembered_side_is_safe {
            // Bypass movement uses real enemy-sized sweeps rather than the
            // expanded authored margin, allowing escape when already close
            // to the statue while still refusing headings over the void.
            navigation.bypass_direction =
                local_bypass_direction(entity, position, desired_delta, destination, spatial_query);
        }
        navigation.bypass_time_remaining = if navigation.bypass_direction == Vec3::ZERO {
            0.0
        } else {
            // Limit recovery by travel distance rather than a fixed long
            // duration. Faster enemies such as the Hound should not sweep far
            // outside the useful A* route during the same recovery window.
            let movement_speed = desired_delta.length() / delta_seconds.max(f32::EPSILON);
            1.5 / movement_speed.max(0.1)
        };
    }

    if navigation.bypass_time_remaining > 0.0 {
        // Do not promote direct/Landmass movement from the middle of a
        // bypass. The bypass is a committed recovery step; once it expires,
        // the fallback route is rebuilt below. Reconnecting here was the
        // remaining source of Bypass -> Direct -> GridAStar oscillation.
        navigation.bypass_time_remaining =
            (navigation.bypass_time_remaining - delta_seconds).max(0.0);
        let bypass_step = navigation.bypass_direction * desired_delta.length();
        if enemy_step_is_safe_ignoring(entity, ignored_entity, position, bypass_step, spatial_query)
        {
            return navigation.debug_step(
                entity,
                NavigationDecision::Bypass,
                position,
                destination,
                bypass_step,
            );
        }
        navigation.bypass_time_remaining = 0.0;
        navigation.repath_timer = 0.0;
    }

    navigation.repath_timer -= delta_seconds;
    let target_moved = !navigation.authored_detour
        && navigation.path_target.distance_squared(destination)
            > ENEMY_PATH_TARGET_MOVED * ENEMY_PATH_TARGET_MOVED;
    if target_moved {
        navigation.repath_timer = 0.0;
    }
    let path_finished = navigation.waypoint_index >= navigation.waypoints.len();

    // Once fallback navigation starts, keep it authoritative until its
    // waypoint route is finished. Reconnecting to direct movement from the
    // middle of a detour lets a moving target promote Landmass again before
    // the obstacle has actually been rounded, creating Direct -> A* ->
    // Bypass -> Direct oscillation. The completed-route check below still
    // returns control to direct/Landmass movement as soon as the route ends.

    let steered_delta = navigation.steer_step(desired_delta);
    let steered_probe =
        steered_delta.normalize_or_zero() * ENEMY_PATH_COARSE_CELL_SIZE.max(steered_delta.length());
    if navigation.direct_route
        && enemy_grounded_path_is_safe(entity, position, steered_probe, spatial_query)
        && enemy_world_path_is_clear_ignoring(
            entity,
            ignored_entity,
            position,
            steered_probe,
            spatial_query,
        )
    {
        return navigation.debug_step(
            entity,
            NavigationDecision::Direct,
            position,
            destination,
            steered_delta,
        );
    }

    // Crowd separation is optional steering. If it bends a clear chase step
    // into an obstacle, prefer the original direct movement instead of
    // abandoning direct pursuit and starting a new detour.
    let direct_probe =
        desired_delta.normalize_or_zero() * ENEMY_PATH_COARSE_CELL_SIZE.max(desired_delta.length());
    if navigation.direct_route
        && enemy_grounded_path_is_safe(entity, position, direct_probe, spatial_query)
        && enemy_world_path_is_clear_ignoring(
            entity,
            ignored_entity,
            position,
            direct_probe,
            spatial_query,
        )
    {
        return navigation.debug_step(
            entity,
            NavigationDecision::Direct,
            position,
            destination,
            desired_delta,
        );
    }
    if navigation.direct_route {
        if !enemy_world_path_is_clear_ignoring(
            entity,
            ignored_entity,
            position,
            direct_probe,
            spatial_query,
        ) {
            // Dynamic memory is based on a real enemy-sized collision, not on
            // a planning footprint. A visual mesh without a collider cannot
            // trigger this and therefore cannot make enemies repath.
            grid.block_world_area(position + direct_probe);
        }
        navigation.direct_route = false;
        navigation.repath_timer = 0.0;
    }

    // Target movement may make the next route request urgent, but it must not
    // replace an active route halfway through its corners. Rebuilding here is
    // what made a moving player repeatedly select different sides of the same
    // obstacle.
    let needs_repath = navigation.repath_timer <= 0.0 && (!navigation.detouring || path_finished);
    if needs_repath {
        let mut to_destination = destination - position;
        to_destination.y = 0.0;
        if enemy_grounded_path_is_safe(entity, position, to_destination, spatial_query)
            && enemy_world_path_is_clear_ignoring(
                entity,
                ignored_entity,
                position,
                to_destination,
                spatial_query,
            )
        {
            navigation.direct_route = true;
            navigation.detouring = false;
            navigation.authored_detour = false;
            navigation.waypoints.clear();
            navigation.bypass_direction = Vec3::ZERO;
            navigation.search_limit = ENEMY_PATH_INITIAL_VISITED;
            navigation.path_cell_size = ENEMY_PATH_COARSE_CELL_SIZE;
            navigation.path_target = destination;
            navigation.repath_timer = ENEMY_PATH_RECALCULATE_TIME;
            return navigation.debug_step(
                entity,
                NavigationDecision::Direct,
                position,
                destination,
                desired_delta,
            );
        }

        navigation.detouring = true;
        let authored_waypoints = grid.authored_corner_path(position, destination);
        let authored_path_is_safe = authored_waypoints.as_ref().is_some_and(|waypoints| {
            let mut previous = position;
            for point in waypoints
                .iter()
                .copied()
                .chain(std::iter::once(destination))
            {
                let delta = point - previous;
                if !enemy_grounded_path_is_safe(entity, previous, delta, spatial_query)
                    || !enemy_world_path_is_clear_ignoring(
                        entity,
                        ignored_entity,
                        previous,
                        delta,
                        spatial_query,
                    )
                {
                    return false;
                }
                previous = point;
            }
            true
        });
        let (waypoints, reached_goal) = if authored_path_is_safe {
            navigation.authored_detour = true;
            (authored_waypoints.unwrap_or_default(), true)
        } else {
            navigation.authored_detour = false;
            build_ground_path(
                entity,
                position,
                destination,
                spatial_query,
                grid,
                navigation.search_limit,
                navigation.path_cell_size,
            )
        };
        navigation.waypoints = waypoints;
        navigation.search_limit = if reached_goal {
            ENEMY_PATH_INITIAL_VISITED
        } else {
            (navigation.search_limit * 2).min(ENEMY_PATH_MAX_VISITED)
        };
        navigation.path_cell_size = if reached_goal {
            ENEMY_PATH_COARSE_CELL_SIZE
        } else {
            ENEMY_PATH_FINE_CELL_SIZE
        };
        navigation.waypoint_index = 0;
        navigation.repath_timer = ENEMY_PATH_RECALCULATE_TIME;
        navigation.path_target = destination;
    }

    let waypoint_reach_squared = if navigation.authored_detour {
        0.04
    } else {
        0.36
    };
    while navigation.waypoint_index < navigation.waypoints.len()
        && position.distance_squared(navigation.waypoints[navigation.waypoint_index])
            < waypoint_reach_squared
    {
        navigation.waypoint_index += 1;
    }

    // Look a few cells ahead and skip the grid's staircase waypoints when the
    // enemy-sized route is clear. This keeps A* safe around corners while
    // producing a shorter, smoother line on open portions of the detour.
    let lookahead_index =
        (navigation.waypoint_index + 3).min(navigation.waypoints.len().saturating_sub(1));
    if lookahead_index > navigation.waypoint_index {
        let lookahead_delta = navigation.waypoints[lookahead_index] - position;
        if enemy_grounded_path_is_safe(entity, position, lookahead_delta, spatial_query)
            && enemy_world_path_is_clear_ignoring(
                entity,
                ignored_entity,
                position,
                lookahead_delta,
                spatial_query,
            )
        {
            navigation.waypoint_index = lookahead_index;
        }
    }

    let Some(waypoint) = navigation.waypoints.get(navigation.waypoint_index).copied() else {
        // Do not stop between the final corner waypoint and direct pursuit.
        // Validate the next chase-sized probe rather than requiring the entire
        // remaining route to be flat and clear in one test.
        let local_probe = desired_delta.normalize_or_zero()
            * ENEMY_PATH_COARSE_CELL_SIZE.max(desired_delta.length());
        let ground_clear =
            enemy_grounded_path_is_safe(entity, position, local_probe, spatial_query);
        let world_clear = enemy_world_path_is_clear_ignoring(
            entity,
            ignored_entity,
            position,
            local_probe,
            spatial_query,
        );
        if ground_clear && world_clear {
            navigation.direct_route = true;
            navigation.detouring = false;
            navigation.authored_detour = false;
            navigation.waypoints.clear();
            navigation.waypoint_index = 0;
            navigation.repath_timer = ENEMY_PATH_RECALCULATE_TIME;
            navigation.path_target = destination;
            navigation.bypass_direction = Vec3::ZERO;
            return navigation.debug_step(
                entity,
                if navigation.authored_detour {
                    NavigationDecision::AuthoredDetour
                } else {
                    NavigationDecision::GridAStar
                },
                position,
                destination,
                desired_delta,
            );
        }

        // Retry soon, but retain a small delay so an unreachable target cannot
        // trigger a full A* search every frame.
        navigation.repath_timer = navigation.repath_timer.min(0.05);
        return navigation.debug_step(
            entity,
            NavigationDecision::Blocked,
            position,
            destination,
            Vec3::ZERO,
        );
    };
    let mut waypoint_direction = waypoint - position;
    waypoint_direction.y = 0.0;
    let direct_step = waypoint_direction.normalize_or_zero() * desired_delta.length();
    let steered_step = navigation.steer_step(direct_step);
    if enemy_step_is_safe_ignoring(
        entity,
        ignored_entity,
        position,
        steered_step,
        spatial_query,
    ) {
        navigation.debug_step(
            entity,
            if navigation.authored_detour {
                NavigationDecision::AuthoredDetour
            } else {
                NavigationDecision::GridAStar
            },
            position,
            destination,
            steered_step,
        )
    } else if enemy_step_is_safe_ignoring(
        entity,
        ignored_entity,
        position,
        direct_step,
        spatial_query,
    ) {
        navigation.debug_step(
            entity,
            if navigation.authored_detour {
                NavigationDecision::AuthoredDetour
            } else {
                NavigationDecision::GridAStar
            },
            position,
            destination,
            direct_step,
        )
    } else {
        // The grid route was valid according to sampled ground data, but the
        // real enemy-sized sweep rejected this segment. Treat the waypoint's
        // cell as a short-lived dynamic obstacle regardless of whether the
        // collision came from authored geometry, imported GLB geometry, or a
        // moving actor. Otherwise recovery backs away and A* immediately
        // selects the same physically unusable waypoint again.
        grid.block_failed_waypoint(waypoint);
        // The enemy can be offset from the centerline of the A* cell it just
        // reached. If that makes the next waypoint approach cross a crack,
        // move along a safe local fan immediately instead of rebuilding the
        // identical unusable route every frame. The same recovery also works
        // around ground-level statue collisions.
        let recovery_direction =
            local_bypass_direction(entity, position, direct_step, destination, spatial_query);
        navigation.waypoints.clear();
        navigation.waypoint_index = 0;
        navigation.repath_timer = 0.0;
        if recovery_direction != Vec3::ZERO {
            navigation.bypass_direction = recovery_direction;
            let movement_speed = desired_delta.length() / delta_seconds.max(f32::EPSILON);
            navigation.bypass_time_remaining = 1.25 / movement_speed.max(0.1);
            navigation.debug_step(
                entity,
                NavigationDecision::Bypass,
                position,
                destination,
                recovery_direction * desired_delta.length(),
            )
        } else {
            navigation.debug_step(
                entity,
                NavigationDecision::Blocked,
                position,
                destination,
                Vec3::ZERO,
            )
        }
    }
}

/// Authorizes a purpose-built leap, lunge, or teleport across unsupported
/// terrain only when its intended landing point has safe supporting ground.
/// Normal chase movement must never call this function.
pub(crate) fn authorize_enemy_edge_traversal(
    entity: Entity,
    landing_position: Vec3,
    safety: &mut EnemyGroundSafety,
    spatial_query: &SpatialQuery,
) -> bool {
    if ground_distance(entity, landing_position, spatial_query).is_none() {
        safety.authorized_landing = None;
        return false;
    }
    safety.authorized_landing = Some(landing_position);
    true
}

pub(crate) fn enemy_landing_is_safe(
    entity: Entity,
    landing_position: Vec3,
    spatial_query: &SpatialQuery,
) -> bool {
    ground_distance(entity, landing_position, spatial_query).is_some()
}

pub(crate) fn finish_enemy_edge_traversal(safety: &mut EnemyGroundSafety) {
    safety.authorized_landing = None;
}
