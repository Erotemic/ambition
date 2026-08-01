//! Generic body-transit velocity math for platformer mechanics.
//!
//! This is the proto-runtime home for the pure velocity transform that maps a
//! body's velocity from one oriented surface to another. It is portal-shaped in
//! origin but is plain reflect/rotate-between-two-normals math with no portal
//! dependency, so non-portal mechanics can reuse it.

use bevy::prelude::*;

use crate::math::portal_map_vec;

/// Transform a velocity from an entry surface (outward normal `n_in`) to an
/// exit surface (outward normal `n_out`) via the IDEAL tangent-preserving map
/// ([`portal_map_vec`](crate::math::portal_map_vec)): the component
/// into the entry emerges out of the exit, and the along-surface component is
/// carried over. So
/// momentum is preserved Portal-style AND the along-surface direction is kept
/// (two floor surfaces don't mirror your horizontal velocity).
pub fn rotate_velocity_between_normals(v: Vec2, n_in: Vec2, n_out: Vec2) -> Vec2 {
    portal_map_vec(v, n_in, n_out)
}
