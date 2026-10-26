use avian3d::math::AdjustPrecision as _;
use avian3d::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::collision::GameLayer;
use crate::game_state::AppState;
use crate::weapons::InventoryUiState;

mod fade;

/// Baseline (fully unobstructed) camera framing: `CAMERA_HEIGHT` units above
/// the pivot and `CAMERA_DISTANCE` units behind it, rotated by the current
/// yaw/pitch. Think of this as the *resting length* of a spring arm — every
/// room shares this same rig, so nothing here is specific to any one map.
pub const CAMERA_HEIGHT: f32 = 3.0;
pub const CAMERA_DISTANCE: f32 = 8.0;

/// The absolute shortest the spring arm may ever collapse to -- just far
/// enough outside the player capsule (radius 0.45) that the camera doesn't
/// clip into the character model. This is a hard physical floor, not a
/// preferred "comfortable" distance: when *real, solid* geometry
/// (`GameLayer::World`) is closer than this, the arm still shortens all the
/// way down to it. An earlier version of this clamped up to a larger, more
/// comfortable minimum (2.0) whenever anything was hit -- which felt fine
/// from a few units away, but the moment the player stood closer to a wall
/// than that comfortable minimum, it forced the camera to sit farther out
/// than the wall actually allowed, clipping straight through it. Never
/// raise this back up without also re-deriving the target length as a
/// fraction of distance-to-obstruction instead of a flat floor, or the same
/// bug comes back.
const CAMERA_ARM_ABSOLUTE_MIN_LENGTH: f32 = 0.6;

/// The farthest the spring arm will pull in just because *decorative*,
/// walk-through geometry (`GameLayer::CameraOccluder`) is in the way. Real
/// architecture can still push the arm all the way down to
/// `CAMERA_ARM_ABSOLUTE_MIN_LENGTH`, but decorative clutter -- statue
/// rubble, siege wreckage, GLB set-dressing with no gameplay collider --
/// stops here and leans on `fade` instead, so standing near a piece of
/// decoration doesn't shove the camera into an uncomfortably tight
/// close-up. See `fade::fade_camera_occluders`.
const CAMERA_ARM_COMFORT_MIN_LENGTH: f32 = 2.4;

/// Radius of the sphere swept along the spring arm to look for obstructions.
/// A sphere (rather than a plain raycast) keeps the whole lens clear of
/// corners and thin geometry, not just the arm's exact center line.
const CAMERA_COLLISION_RADIUS: f32 = 0.3;

/// Extra pull-in past the hit point so the near clip plane doesn't poke
/// through whatever stopped the arm.
const CAMERA_COLLISION_SKIN: f32 = 0.2;

/// Exponential-decay smoothing rates (per second) applied to the arm length.
/// Pulling in is fast and decisive, so geometry never visibly pokes into
/// frame even for a moment; releasing back out is slower and gentler, which
/// is what keeps the camera from swinging/whipping around in small rooms.
const CAMERA_ZOOM_IN_RATE: f32 = 20.0;
const CAMERA_ZOOM_OUT_RATE: f32 = 6.0;

pub const MOUSE_SENSITIVITY: f32 = 0.003;

#[derive(Component)]
pub struct ThirdPersonCamera {
    pub yaw: f32,
    pub pitch: f32,
    /// Current smoothed spring-arm length. Eases toward whatever length
    /// `camera_collision` finds clear this frame; `camera_follow` is the
    /// only other place that reads it, so the two systems always agree on
    /// where the camera actually ends up.
    current_arm_length: f32,
    /// The single decorative occluder (`GameLayer::CameraOccluder`)
    /// currently between the camera and the player that `fade` should be
    /// fading toward transparent, if any. Only ever set when nothing on
    /// `GameLayer::World` is closer -- see `camera_collision`.
    fade_target: Option<Entity>,
}

/// The entity the third-person camera orbits and follows. Every `Player`
/// gets this automatically (see `player::Player`). In co-op, a joined
/// client has no local `Player`; instead its own replicated character is
/// given this marker (see `net::avatars`).
#[derive(Component, Default)]
pub struct CameraTarget;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        // The camera itself spawns unconditionally at `Startup`, not
        // `OnEnter(AppState::InGame)` -- bevy_ui needs a camera present
        // to render anything at all, including the run-setup screen
        // shown before a run exists (see `ui::run_setup_screen`). Every
        // other Camera-related system here still only has real work to
        // do once a `CameraTarget` (normally the player) exists.
        app.add_systems(Startup, spawn_camera_system);

        app.init_resource::<fade::OcclusionFades>().add_systems(
            Update,
            (
                camera_mouse_look,
                camera_collision,
                fade::fade_camera_occluders,
                camera_follow,
            )
                .chain(),
        );

        // Split out of the chain above and gated to `AppState::InGame`:
        // toggling cursor grab on Escape is only appropriate once a run
        // (and its locked/hidden gameplay cursor) exists -- during
        // run-setup it would yank the cursor away from the menu the
        // player is trying to click. No other system in this plugin
        // depends on this one's ordering.
        app.add_systems(Update, toggle_cursor.run_if(in_state(AppState::InGame)));

        // The run's world is despawned when it ends; forget anything faded.
        app.add_systems(
            OnExit(AppState::InGame),
            (
                crate::game_state::reset_resource::<fade::OcclusionFades>,
                forget_fade_target,
            ),
        );
    }
}

fn forget_fade_target(mut cameras: Query<&mut ThirdPersonCamera>) {
    for mut camera in &mut cameras {
        camera.fade_target = None;
    }
}

fn spawn_camera_system(mut commands: Commands) {
    spawn_camera(&mut commands);
}

pub fn spawn_camera(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, CAMERA_HEIGHT, CAMERA_DISTANCE).looking_at(Vec3::Y, Vec3::Y),
        ThirdPersonCamera {
            yaw: 0.0,
            pitch: -0.25,
            current_arm_length: base_arm_vector().length(),
            fade_target: None,
        },
    ));
}

/// The point the camera orbits and looks at — roughly the player's chest.
/// Collision probing and final placement both anchor on this so they never
/// drift apart.
fn camera_pivot(player_transform: &Transform) -> Vec3 {
    player_transform.translation + Vec3::Y
}

fn camera_rotation(camera: &ThirdPersonCamera) -> Quat {
    Quat::from_euler(EulerRot::YXZ, camera.yaw, camera.pitch, 0.0)
}

/// The camera's resting offset from the pivot, before yaw/pitch rotation and
/// before any collision shortening is applied.
fn base_arm_vector() -> Vec3 {
    Vec3::new(0.0, CAMERA_HEIGHT, CAMERA_DISTANCE)
}

fn camera_mouse_look(
    mut mouse_motion: MessageReader<MouseMotion>,
    cursor_options: Single<&CursorOptions>,
    mut camera_query: Query<&mut ThirdPersonCamera>,
) {
    if cursor_options.grab_mode == CursorGrabMode::None {
        return;
    }

    let Ok(mut camera) = camera_query.single_mut() else {
        return;
    };

    for motion in mouse_motion.read() {
        camera.yaw -= motion.delta.x * MOUSE_SENSITIVITY;
        camera.pitch -= motion.delta.y * MOUSE_SENSITIVITY;
    }

    camera.pitch = camera.pitch.clamp(-1.2, 1.2);
}

/// Dynamic zoom-in: shortens the camera's spring arm when room geometry
/// (a wall, a pillar, a tight alcove) would otherwise sit between the camera
/// and the player, so the view snaps forward instead of clipping through the
/// structure or hiding the character behind it.
///
/// This only depends on two shared collision layers -- `GameLayer::World`
/// (ordinary solid geometry) and `GameLayer::CameraOccluder` (visual-only,
/// walk-through decoration that still wants the camera to duck around it,
/// see `rooms::geometry::camera_occluder`) -- so it works unmodified in any
/// room or map, no per-level setup required.
fn camera_collision(
    time: Res<Time>,
    player_query: Query<(Entity, &Transform), (With<CameraTarget>, Without<Camera3d>)>,
    mut camera_query: Query<&mut ThirdPersonCamera, With<Camera3d>>,
    spatial_query: SpatialQuery,
) {
    let Ok((player_entity, player_transform)) = player_query.single() else {
        return;
    };

    let Ok(mut camera) = camera_query.single_mut() else {
        return;
    };

    let pivot = camera_pivot(player_transform);
    let desired_offset = camera_rotation(&camera) * base_arm_vector();
    let max_length = desired_offset.length();

    let mut fade_target = None;

    let target_length = match Dir3::new(desired_offset.adjust_precision()) {
        Ok(direction) => {
            let sweep = |mask: GameLayer| {
                let filter =
                    SpatialQueryFilter::from_excluded_entities([player_entity]).with_mask(mask);

                spatial_query.cast_shape(
                    &Collider::sphere(CAMERA_COLLISION_RADIUS),
                    pivot.adjust_precision(),
                    Quat::IDENTITY.adjust_precision(),
                    direction,
                    &ShapeCastConfig::from_max_distance(max_length),
                    &filter,
                )
            };

            // Real, solid architecture always wins: if it's in the way, the
            // arm must physically avoid it, full stop. Only when nothing
            // solid is blocking do we even consider decorative geometry.
            //
            // Querying both layers together in one combined-mask cast (the
            // previous version of this function) let a deterministic
            // gameplay collider -- e.g. a pillar's hand-placed proxy box --
            // and that same pillar's own auto-generated decorative trimesh
            // compete for "closest hit" against each other every frame.
            // Wherever their two slightly different surfaces crossed over,
            // a tiny change in view angle flipped which one "won", which is
            // exactly what made the camera flicker/snap right at pillars
            // and walls that carry both kinds of collider. Two separate,
            // priority-ordered sweeps remove that competition entirely.
            if let Some(hit) = sweep(GameLayer::World) {
                (hit.distance - CAMERA_COLLISION_SKIN).max(CAMERA_ARM_ABSOLUTE_MIN_LENGTH)
            } else if let Some(hit) = sweep(GameLayer::CameraOccluder) {
                let comfortable =
                    (hit.distance - CAMERA_COLLISION_SKIN).max(CAMERA_ARM_COMFORT_MIN_LENGTH);

                // The decorative piece sits nearer than we're willing to
                // physically zoom in for -- fade it out instead of pushing
                // the camera the rest of the way in past it.
                if hit.distance < CAMERA_ARM_COMFORT_MIN_LENGTH {
                    fade_target = Some(hit.entity);
                }

                comfortable
            } else {
                max_length
            }
        }
        // The arm has zero length (camera exactly on top of the pivot) —
        // nothing to sweep, so just fall back to the resting length.
        Err(_) => max_length,
    };

    camera.fade_target = fade_target;

    // Frame-rate independent exponential smoothing, deliberately asymmetric:
    // fast when the target is closer than where we are (zooming in), slower
    // when it's farther (zooming back out). See the rate constants above.
    let rate = if target_length < camera.current_arm_length {
        CAMERA_ZOOM_IN_RATE
    } else {
        CAMERA_ZOOM_OUT_RATE
    };
    let t = 1.0 - (-rate * time.delta_secs()).exp();
    camera.current_arm_length += (target_length - camera.current_arm_length) * t;
}

fn camera_follow(
    player_query: Query<&Transform, (With<CameraTarget>, Without<Camera3d>)>,
    mut camera_query: Query<
        (&ThirdPersonCamera, &mut Transform),
        (With<Camera3d>, Without<CameraTarget>),
    >,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let Ok((camera, mut camera_transform)) = camera_query.single_mut() else {
        return;
    };

    let pivot = camera_pivot(player_transform);
    let arm_direction = base_arm_vector().normalize_or_zero();
    let offset = camera_rotation(camera) * (arm_direction * camera.current_arm_length);

    camera_transform.translation = pivot + offset;

    camera_transform.look_at(pivot, Vec3::Y);
}

fn toggle_cursor(
    keyboard: Res<ButtonInput<KeyCode>>,
    inventory_ui: Res<InventoryUiState>,
    mut cursor_options: Single<&mut CursorOptions>,
) {
    if inventory_ui.open {
        return;
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        if cursor_options.grab_mode == CursorGrabMode::None {
            cursor_options.grab_mode = CursorGrabMode::Locked;

            cursor_options.visible = false;
        } else {
            cursor_options.grab_mode = CursorGrabMode::None;

            cursor_options.visible = true;
        }
    }
}
