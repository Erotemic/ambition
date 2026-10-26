//! A mount's ledges that follow its art (`CharacterMount::drawn_ledges`) read
//! the row and frame the mount is drawn in.
//!
//! The row is the body's [`PinnedRow`]: a conductor's, or its rest row's.
//! A pin is not rollback state (its writers derive it again each tick), so
//! the ledges read it once a tick AFTER every writer and keep what they read
//! in the [`BackLedge`] (rollback state): the ground of the next tick is a
//! function of that, and a resimulated tick reads the same.

use ambition_mount::{BackLedge, Mountable};
use ambition_sprite_sheet::character::PinnedRow;
use bevy::prelude::*;

/// Read each drawn ledge in the row its mount is drawn in this tick. (sim)
///
/// Run it after every system that pins a row (the boss conductors, the rest
/// and fade rows). A mount that nothing pins is drawn in its rest row; the
/// ledges read its first frame, as the simulation has no clock of its own
/// for an unpinned body's rest.
///
/// The ledges are placed where the drawing will be on the NEXT tick (its row
/// one tick further on), as that is the tick whose bodies stand on them.
///
/// A seat on a drawn ledge (`CharacterMount::saddle_on`) goes with it.
pub fn follow_drawn_ledges(mut mounts: Query<(&mut BackLedge, Option<&mut Mountable>, Option<&PinnedRow>)>) {
    for (mut back, mountable, pinned) in &mut mounts {
        if back.drawn.is_empty() || back.surfaces.is_none() {
            continue;
        }
        match pinned.filter(|pinned| pinned.is_pinned()) {
            Some(pinned) => back.follow_drawing(&pinned.rows, pinned.elapsed + TICK_S, pinned.looping),
            None => back.follow_drawing(&[], 0.0, true),
        }
        if let Some(mut mountable) = mountable {
            if let Some(y) = back.seat_y(mountable.seat.x) {
                if mountable.seat.y != y {
                    mountable.seat.y = y;
                }
            }
        }
    }
}

/// The length of a simulation tick (s): the clock of a pinned row goes on by
/// this much a tick.
const TICK_S: f32 = 1.0 / 60.0;
