use avian3d::prelude::*;

/// Shared physics layers. Character bodies pass through one another while
/// remaining solid against room geometry; combat hit detection is handled by
/// the existing spatial-query systems.
#[derive(PhysicsLayer, Default)]
pub enum GameLayer {
    #[default]
    World,
    Player,
    Enemy,
    /// Visual-only geometry that should still block the third-person
    /// camera's spring arm (see `camera::camera_collision`) even though it
    /// never physically blocks players or enemies and is excluded from
    /// navmesh generation (see `navigation_mesh::exclude_camera_occluders_from_navmesh`).
    /// Attached automatically by `rooms::geometry`'s "visual-only" block
    /// helpers (`spawn_block`, `spawn_rotated_block`) and by
    /// `ColliderConstructorHierarchy` on every GLB room scene (see each
    /// room's `spawn_*` function), so any room -- hand-authored or
    /// GLB-imported -- keeps the camera from clipping through walk-through
    /// decoration without extra per-room setup.
    CameraOccluder,
}

/// The single, shared collision-layer configuration for every
/// camera-occlusion-only collider in the game: a member of
/// `GameLayer::CameraOccluder` and nothing else, filtered against
/// `LayerMask::NONE` so it can never physically interact with anything --
/// players and enemies pass through it exactly as if it didn't exist.
///
/// Every place that creates one of these colliders (the hand-placed
/// `rooms::geometry` helpers, and the GLB-scene `ColliderConstructorHierarchy`
/// setup in each room's spawner) calls this one function, so the definition
/// never drifts, and `navigation_mesh::exclude_camera_occluders_from_navmesh`
/// can reliably recognize any collider using it.
pub fn camera_occluder_layers() -> CollisionLayers {
    CollisionLayers::new(GameLayer::CameraOccluder, LayerMask::NONE)
}
