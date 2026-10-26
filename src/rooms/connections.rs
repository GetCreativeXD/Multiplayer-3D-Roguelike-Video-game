//! Reusable room-to-room connection math.
//!
//! Ruinbound has no level-streaming/teleport system: two connected rooms
//! simply coexist in one persistent world, and the player walks from one
//! straight into the other (see `ruined_barracks::spawn_ruined_barracks`
//! for the first real use of this, joining Ruined Barracks onto Fallen
//! Gate). Every room is spawned as a root entity carrying a `Transform`;
//! everything else in the room (visual scene, colliders, markers) is
//! spawned as a *child* of that root and inherits its placement for free
//! through ordinary Bevy transform propagation. This module computes that
//! root `Transform` so a new room's authored geometry (measured in its own
//! local/GLB space) lines up seamlessly -- no gap, no overlap, no floor
//! step -- with the room it's connecting to, and continues in the same
//! direction of travel.
//!
//! A connection is described by a matched pair of `RoomSocket`s: the
//! previous room's *exit* socket, already in world space, and the next
//! room's *entrance* socket, measured in the next room's own local space
//! (before that room has been placed anywhere). Both use the same shape --
//! a position (X/Z matter for the seam, Y should be the floor's *top
//! surface* height so two rooms with different floor thicknesses still
//! line up flush) plus the horizontal direction of travel through that
//! point, continuing further along the route (not the direction facing
//! back at the room just left). `connect_room` finds the yaw rotation and
//! translation that place the entrance socket exactly on the exit socket
//! while keeping the direction of travel continuous across the seam.
//!
//! Usage pattern for a new room (see `ruined_barracks.rs` for the full
//! version): give the room being connected *from* its own `exit_socket()`
//! function (or a `const`) describing where its content ends in world
//! space once it's spawned -- `fallen_gate::exit_socket` is the first
//! example -- and give the *new* room a local entrance socket measured
//! from its own GLB or hand-authored layout, then call:
//! ```ignore
//! let room_transform = connections::connect_room(
//!     previous_room::exit_socket(),
//!     RoomSocket::new(local_entrance_position, local_entrance_forward),
//! );
//! ```
//! and spawn the new room's root entity with that `Transform`. Anything
//! that needs to convert one of the room's other local-space measurements
//! into world space by hand -- independently-spawned enemies, chiefly,
//! since they don't inherit the room root's transform the way a child
//! entity does -- can reuse the same `Transform` via
//! `room_transform.transform_point(local)`, which folds in the same
//! rotation, translation, and Y adjustment automatically.
//!
//! A room with more than one exit, or one reachable from more than one
//! direction, can expose more than one `RoomSocket`-returning function;
//! nothing here assumes exactly one of each.

use bevy::prelude::*;

/// One end of a room-to-room connection: a position (X/Z for the seam, Y
/// for the floor's top-surface height so floors of different thickness
/// still line up flush) plus the horizontal direction of travel through
/// it, continuing further along the route. Only the horizontal (X/Z)
/// component of `forward` matters -- Y is ignored, since every room in
/// Ruinbound is placed flat (no pitch/roll). `forward` does not need to be
/// pre-normalized.
#[derive(Debug, Clone, Copy)]
pub struct RoomSocket {
    pub position: Vec3,
    pub forward: Vec3,
}

impl RoomSocket {
    pub const fn new(position: Vec3, forward: Vec3) -> Self {
        Self { position, forward }
    }

    /// Horizontal heading of `forward`, in the same convention
    /// `Quat::from_rotation_y` rotates by (see `connect_room`'s doc
    /// comment for the derivation).
    fn heading(&self) -> f32 {
        f32::atan2(self.forward.x, self.forward.z)
    }
}

/// Computes the room-root `Transform` (translation plus a Y-axis-only
/// rotation -- no pitch/roll/scale, matching every room's flat placement)
/// that joins `entrance` (measured in the new room's own local space) onto
/// `exit` (an already-placed, world-space socket in the room being
/// connected from), continuing in the same direction of travel across the
/// seam rather than doubling back into the room just left.
///
/// Derivation: `Quat::from_rotation_y(yaw)` rotates a horizontal vector's
/// heading (`atan2(x, z)`) by `+yaw` (confirmed against Bevy's rotation
/// convention: a positive Y rotation carries +Z towards +X). Solving
/// `heading(entrance.forward) + yaw == heading(exit.forward)` for `yaw`
/// gives the rotation below; the translation is then whatever's needed so
/// the rotated entrance position lands exactly on the exit position. A
/// pure Y-axis rotation never changes a vector's Y component, so the
/// entrance socket's local Y (its floor's top surface) simply shifts by a
/// flat amount to match the exit socket's Y -- which is what gives every
/// other local Y coordinate in the new room a correct, uniform lift for
/// free once it's converted through the same `Transform`.
pub fn connect_room(exit: RoomSocket, entrance: RoomSocket) -> Transform {
    let yaw = exit.heading() - entrance.heading();
    let rotation = Quat::from_rotation_y(yaw);
    let translation = exit.position - rotation * entrance.position;
    Transform {
        translation,
        rotation,
        ..default()
    }
}
