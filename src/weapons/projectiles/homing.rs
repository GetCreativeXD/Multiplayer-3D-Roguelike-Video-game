//! Generic homing-projectile steering, shared by every projectile kind
//! that reports `true` from `ProjectileKind::homes` -- Splinter fragments
//! and the Spellbook's Arcane Bolt family (plain, Scorch, and Grace).
//! Originally Splinter-specific (`weapons::projectiles::splinter`); any
//! future homing projectile just adds itself to `ProjectileKind::homes`
//! and gets this steering for free.

use bevy::prelude::*;

use crate::enemies::Enemy;

use crate::weapons::components::{PlayerProjectile, Projectile};

/// Speed a homing projectile steers at once it has a live target. Kept
/// separate from `Projectile::velocity`'s magnitude at spawn so a slower-
/// or faster-launched homing bolt still homes at a consistent speed --
/// today every homing kind launches at this same speed already, but
/// future ones don't have to.
pub(crate) const HOMING_STEER_SPEED_FALLBACK: f32 = 20.0;

pub(crate) fn home_locked_projectiles(
    enemy_query: Query<&Transform, (With<Enemy>, Without<PlayerProjectile>)>,

    mut projectile_query: Query<
        (&Transform, &mut Projectile),
        (With<PlayerProjectile>, Without<Enemy>),
    >,
) {
    for (projectile_transform, mut projectile) in &mut projectile_query {
        if !projectile.kind.homes() {
            continue;
        }

        let Some(target) = projectile.target else {
            continue;
        };

        let Ok(target_transform) = enemy_query.get(target) else {
            projectile.target = None;
            continue;
        };

        let direction =
            (target_transform.translation - projectile_transform.translation).normalize_or_zero();

        if direction == Vec3::ZERO {
            continue;
        }

        let speed = projectile.velocity.length();
        let speed = if speed > 0.0 {
            speed
        } else {
            HOMING_STEER_SPEED_FALLBACK
        };

        projectile.velocity = direction * speed;
    }
}
