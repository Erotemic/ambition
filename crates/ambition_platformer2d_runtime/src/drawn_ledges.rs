//! A mount's ledges and slope that follow its art (`CharacterMount::drawn_ledges`,
//! `CharacterMount::slope_follows`) read the row and frame the mount is drawn in.
//!
//! The row is the body's [`PinnedRow`]: a conductor's, or its rest row's.
//! A pin is not rollback state (its writers derive it again each tick), so
//! the ledges read it once a tick AFTER every writer and keep what they read
//! in the [`BackLedge`] (rollback state): the ground of the next tick is a
//! function of that, and a resimulated tick reads the same.
//!
//! Here, in the composition, because it joins two owners that do not know each
//! other: the mount owns the ledge, the sprite sheet owns the pin.

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
/// The ledges are placed where the owner's clock stands NOW: the phase it
/// pinned this tick, beside the mount's own accepted step (its
/// `SweepSample`), so the support's shape and place belong to one cut. The
/// reader does not forecast the owner's next phase: an owner's clock is its
/// own (GNU's walk advances by the distance it covered, a held row not at
/// all), and a fixed tick added to it fabricated a tick of shape motion for
/// a phase that was held.
///
/// A seat on a drawn ledge (`CharacterMount::saddle_on`) goes with it.
pub fn follow_drawn_ledges(mut mounts: Query<(&mut BackLedge, Option<&mut Mountable>, Option<&PinnedRow>)>) {
    for (mut back, mountable, pinned) in &mut mounts {
        if !back.follows_drawing() || back.surfaces.is_none() {
            continue;
        }
        match pinned.filter(|pinned| pinned.is_pinned()) {
            Some(pinned) => back.follow_drawing(&pinned.rows, pinned.elapsed, pinned.looping),
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


#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::{MountBack, SurfaceRow, SurfaceTable};
    use std::collections::BTreeMap;

    /// SUP1: a drawn ledge is where the owner's pinned phase draws it, not a
    /// fixed tick past it: the owner's clock is its own (GNU's walk counts
    /// distance), and the forecast put the ground where no drawing was.
    #[test]
    fn a_drawn_ledge_is_where_the_pinned_phase_draws_it() {
        let frame = |y: f32| BTreeMap::from([("deck".to_string(), [Vec2::new(0.0, y), Vec2::new(10.0, y)])]);
        let table = SurfaceTable {
            rows: BTreeMap::from([("walk".to_string(), SurfaceRow { frame_duration_s: 0.1, frames: vec![frame(0.0), frame(-10.0), frame(-20.0)] })]),
            rest_row: Some("walk".to_string()),
        };
        let mut back = BackLedge::new(MountBack { left: -10.0, right: 10.0, top: 0.0, thickness: 4.0 });
        back.drawn.push(ambition_mount::DrawnLedge::new("deck"));
        back.surfaces = Some(std::sync::Arc::new(table.clone()));
        let mut pinned = PinnedRow::default();
        pinned.pin(&["walk"], 0.1, true);
        let mut world = World::new();
        let mount = world.spawn((back, pinned.clone())).id();
        let mut system = IntoSystem::into_system(follow_drawn_ledges);
        system.initialize(&mut world);
        let _ = system.run((), &mut world);
        let drawn = world.get::<BackLedge>(mount).unwrap().drawn[0].now.expect("drawn");
        let at_pin = table.segment(&pinned.rows, 0.1, true, "deck").unwrap();
        assert_eq!(drawn, at_pin, "the ledge is the drawing at the pinned phase (y {}), not a tick past it", at_pin[0].y);
        // Held: the next tick reads the same phase, so the drawing did not change.
        let _ = system.run((), &mut world);
        assert!(!world.get::<BackLedge>(mount).unwrap().drawn[0].changed(), "a held phase moves no shape");
    }
}
