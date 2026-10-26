//! Wall/decoration fading for the third-person camera.
//!
//! When decorative, walk-through geometry (`GameLayer::CameraOccluder` --
//! see `rooms::geometry::camera_occluder` and every GLB room's
//! `ColliderConstructorHierarchy`) gets close enough that `camera_collision`
//! chooses not to physically zoom the camera in any further (see
//! `CAMERA_ARM_COMFORT_MIN_LENGTH`), this module fades that specific piece
//! of geometry toward transparent instead, so the player stays visible
//! without an uncomfortably tight close-up.
//!
//! This is driven entirely by whichever entity the camera's own collision
//! sweep reports as the blocker (`ThirdPersonCamera::fade_target`, set by
//! `camera_collision`), read generically off its `MeshMaterial3d<StandardMaterial>`
//! component. Nothing here names a specific room, object, or material, so
//! it applies automatically to any decorative geometry -- hand-placed or
//! GLB-imported -- in any room, present or future, exactly like the
//! occlusion system it rides on.

use std::collections::HashMap;

use bevy::prelude::*;

use super::ThirdPersonCamera;

/// How transparent a faded piece of geometry gets. Not fully invisible
/// (0.0) -- a faint silhouette stays, so the player still reads the room's
/// shape instead of decoration randomly vanishing.
const FADE_MIN_ALPHA: f32 = 0.12;

/// Once a fading-back-in entity's alpha is at least this close to fully
/// opaque, treat it as done and restore its original material.
const FADE_RESTORED_ALPHA: f32 = 0.99;

/// Exponential-decay smoothing rates (per second), asymmetric like the
/// camera's own arm-length smoothing: fading out (becoming transparent) is
/// quicker than fading back in, so decoration doesn't visibly linger
/// blocking the view for long after it stops being the blocker.
const FADE_OUT_RATE: f32 = 10.0;
const FADE_IN_RATE: f32 = 5.0;

struct FadingMaterial {
    /// The entity's original material, restored once it's fully faded back
    /// in (or if the entity disappears while still fading).
    original: Handle<StandardMaterial>,
    /// A private clone of that material this entity alone uses while
    /// fading, so animating its alpha can never bleed into every other
    /// piece of geometry that happens to share the same original material
    /// asset -- common in GLB rooms, where a handful of materials are
    /// reused across dozens of mesh nodes.
    faded: Handle<StandardMaterial>,
    /// Current alpha, eased each frame toward 1.0 (not the blocker) or
    /// `FADE_MIN_ALPHA` (is the blocker).
    alpha: f32,
}

/// Every entity currently faded (or fading back in) by the camera-occlusion
/// system, keyed by entity so each one is only ever cloned/restored once.
#[derive(Resource, Default)]
pub(super) struct OcclusionFades(HashMap<Entity, FadingMaterial>);

/// Fades `ThirdPersonCamera::fade_target` toward transparent, and eases
/// every other tracked entity back toward fully opaque. See the module doc
/// comment for the full picture.
pub(super) fn fade_camera_occluders(
    mut fades: ResMut<OcclusionFades>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut material_handles: Query<&mut MeshMaterial3d<StandardMaterial>>,
    camera_query: Query<&ThirdPersonCamera>,
    time: Res<Time>,
) {
    let Ok(camera) = camera_query.single() else {
        return;
    };
    let active_target = camera.fade_target;

    // Start tracking a newly-blocking entity that isn't already faded.
    if let Some(entity) = active_target {
        if !fades.0.contains_key(&entity) {
            let original_handle = material_handles
                .get(entity)
                .ok()
                .map(|handle| handle.0.clone());

            if let Some(original_handle) = original_handle {
                if let Some(original_material) = materials.get(&original_handle) {
                    let mut faded_material = original_material.clone();
                    faded_material.alpha_mode = AlphaMode::Blend;
                    let starting_alpha = faded_material.base_color.alpha();
                    let faded_handle = materials.add(faded_material);

                    if let Ok(mut current_handle) = material_handles.get_mut(entity) {
                        current_handle.0 = faded_handle.clone();

                        fades.0.insert(
                            entity,
                            FadingMaterial {
                                original: original_handle,
                                faded: faded_handle,
                                alpha: starting_alpha,
                            },
                        );
                    }
                }
            }
        }
    }

    let delta = time.delta_secs();
    let mut finished = Vec::new();

    for (&entity, fade) in fades.0.iter_mut() {
        let is_active = active_target == Some(entity);
        let target_alpha = if is_active { FADE_MIN_ALPHA } else { 1.0 };
        let rate = if target_alpha < fade.alpha {
            FADE_OUT_RATE
        } else {
            FADE_IN_RATE
        };
        let t = 1.0 - (-rate * delta).exp();
        fade.alpha += (target_alpha - fade.alpha) * t;

        if let Some(mut material) = materials.get_mut(&fade.faded) {
            material.base_color.set_alpha(fade.alpha);
        }

        // The entity might have been despawned entirely while still
        // fading (e.g. destructible/corruption geometry) -- either way,
        // once it's fully restored or simply gone, stop tracking it and
        // free the private material clone.
        let entity_gone = material_handles.get(entity).is_err();
        let fully_restored = !is_active && fade.alpha >= FADE_RESTORED_ALPHA;

        if fully_restored || entity_gone {
            if !entity_gone {
                if let Ok(mut current_handle) = material_handles.get_mut(entity) {
                    current_handle.0 = fade.original.clone();
                }
            }

            materials.remove(&fade.faded);
            finished.push(entity);
        }
    }

    for entity in finished {
        fades.0.remove(&entity);
    }
}
