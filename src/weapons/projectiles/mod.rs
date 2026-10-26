mod homing;
mod movement;
mod ricochet;
mod splinter;

pub(crate) use homing::home_locked_projectiles;
pub(crate) use movement::move_projectiles_with_collision;
