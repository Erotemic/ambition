//! `PathMotion`: waypoint-following used by moving hazards/platforms.
//!
//! Walks a position along a `ambition_platformer2d_core::KinematicPath` by `speed * dt`
//! (`advance`), with the `(segment, dir)` cursor stepped by `lookahead_advance`
//! under Once / Loop / PingPong end-reversal rules. Re-exported via
//! `pub use path_motion::*`.

use super::*;

#[derive(Clone, Debug)]
pub struct PathMotion {
    path: ambition_platformer2d_core::KinematicPath,
    segment: usize,
    dir: i32,
}

impl PathMotion {
    pub fn new(path: ambition_platformer2d_core::KinematicPath) -> Self {
        Self {
            path,
            segment: 0,
            dir: 1,
        }
    }

    /// The mutable half of this component, for `SnapshotState` (netcode.md N3.1).
    ///
    /// `path` is authored content and never changes; `(segment, dir)` is a cursor the
    /// sim advances. A rollback must rewind the cursor and must not re-serialize the
    /// waypoints sixty times a second, so the snapshot carries only this pair.
    pub fn cursor(&self) -> (usize, i32) {
        (self.segment, self.dir)
    }

    /// Rewind the cursor.
    pub fn set_cursor(&mut self, segment: usize, dir: i32) {
        self.segment = segment.min(self.path.points.len().saturating_sub(1));
        self.dir = if dir >= 0 { 1 } else { -1 };
    }

    pub fn start_pos(&self) -> Option<ae::Vec2> {
        self.path.points.first().copied()
    }

    pub fn advance(&mut self, mut pos: ae::Vec2, dt: f32) -> ae::Vec2 {
        if !self.path.is_valid() || dt <= 0.0 {
            return pos;
        }
        let mut remaining = self.path.speed * dt;
        while remaining > 0.0 {
            // `Loop` CLOSES its circuit — the `% len` is the leg back to the first point.
            // Without it the cursor returned to segment 0 while the POSITION was still at the
            // last point, so the mover retraced the final leg backwards and never revisited the
            // first.
            let target_index = if self.dir >= 0 {
                let next = self.segment + 1;
                if matches!(
                    self.path.mode,
                    ambition_platformer2d_core::KinematicPathMode::Loop
                ) {
                    next % self.path.points.len().max(1)
                } else {
                    next
                }
            } else {
                self.segment
            };
            let Some(target) = self.path.points.get(target_index).copied() else {
                break;
            };
            let to_target = target - pos;
            let distance = to_target.length();
            if distance <= 0.001 {
                // A CURSOR THAT DOES NOT MOVE ENDS THE FRAME, or this spins
                // forever. This branch consumes no `remaining`, so it is only
                // safe while every advance changes where the mover is heading. A
                // two-point `Loop` path breaks that: `last_segment` is
                // `len.saturating_sub(2)` = 0, so arriving at the second point
                // "wraps" to segment 0 — whose target is the point the mover is
                // already standing on. Zero distance, cursor unchanged,
                // `continue`, forever.
                let before = (self.segment, self.dir);
                self.advance_segment();
                if (self.segment, self.dir) == before {
                    break;
                }
                continue;
            }
            let step = remaining.min(distance);
            pos += to_target / distance * step;
            remaining -= step;
            if step >= distance - 0.001 {
                self.advance_segment();
            }
        }
        pos
    }

    pub fn advance_segment(&mut self) {
        // `Loop` has one more segment than the open modes: the closing leg.
        let last_segment = match self.path.mode {
            ambition_platformer2d_core::KinematicPathMode::Loop => {
                self.path.points.len().saturating_sub(1)
            }
            _ => self.path.points.len().saturating_sub(2),
        };
        lookahead_advance(
            &mut self.segment,
            &mut self.dir,
            last_segment,
            self.path.mode,
        );
    }
}

/// Advance a (segment, dir) cursor by one waypoint under the given path
/// mode. Returns `true` if the cursor moved, `false` if the path has
/// reached its terminus (only `Once` mode can return `false`). Used by
/// both `advance` (mutating) and `lookahead` (non-mutating).
fn lookahead_advance(
    segment: &mut usize,
    dir: &mut i32,
    last_segment: usize,
    mode: ambition_platformer2d_core::KinematicPathMode,
) -> bool {
    match mode {
        ambition_platformer2d_core::KinematicPathMode::Once => {
            if *dir >= 0 && *segment < last_segment {
                *segment += 1;
                true
            } else {
                false
            }
        }
        ambition_platformer2d_core::KinematicPathMode::Loop => {
            if *dir >= 0 {
                *segment = if *segment >= last_segment {
                    0
                } else {
                    *segment + 1
                };
            } else if *segment == 0 {
                *segment = last_segment;
            } else {
                *segment -= 1;
            }
            true
        }
        ambition_platformer2d_core::KinematicPathMode::PingPong => {
            if *dir >= 0 {
                if *segment >= last_segment {
                    *dir = -1;
                } else {
                    *segment += 1;
                }
            } else if *segment == 0 {
                *dir = 1;
            } else {
                *segment -= 1;
            }
            true
        }
    }
}

#[cfg(test)]
mod path_motion_tests {
    use super::*;
    use ambition_platformer2d_core::{KinematicPath, KinematicPathMode};

    fn path(points: Vec<ae::Vec2>, mode: KinematicPathMode) -> KinematicPath {
        KinematicPath {
            points,
            speed: 1.0,
            mode,
            start_offset_seconds: 0.0,
        }
    }

    fn two_point(mode: KinematicPathMode) -> KinematicPath {
        path(
            vec![ae::Vec2::new(0.0, 0.0), ae::Vec2::new(10.0, 0.0)],
            mode,
        )
    }

    #[test]
    fn advance_moves_toward_the_next_waypoint() {
        let mut m = PathMotion::new(two_point(KinematicPathMode::Once));
        let p = m.advance(ae::Vec2::new(0.0, 0.0), 1.0); // speed 1 * dt 1 = 1 unit
        assert!((p.x - 1.0).abs() < 1e-4 && p.y.abs() < 1e-4, "{p:?}");
    }

    #[test]
    fn advance_is_a_noop_for_invalid_path_or_nonpositive_dt() {
        let mut single =
            PathMotion::new(path(vec![ae::Vec2::new(0.0, 0.0)], KinematicPathMode::Once));
        assert_eq!(
            single.advance(ae::Vec2::new(5.0, 5.0), 1.0),
            ae::Vec2::new(5.0, 5.0)
        );
        let mut valid = PathMotion::new(two_point(KinematicPathMode::Once));
        assert_eq!(
            valid.advance(ae::Vec2::new(3.0, 3.0), 0.0),
            ae::Vec2::new(3.0, 3.0)
        );
    }

    #[test]
    fn start_pos_is_the_first_point() {
        let m = PathMotion::new(path(
            vec![ae::Vec2::new(2.0, 7.0), ae::Vec2::new(10.0, 0.0)],
            KinematicPathMode::Loop,
        ));
        assert_eq!(m.start_pos(), Some(ae::Vec2::new(2.0, 7.0)));
    }

    #[test]
    fn lookahead_once_stops_at_terminus() {
        let (mut seg, mut dir) = (0usize, 1i32);
        let last = 2; // 4-point path
        assert!(lookahead_advance(
            &mut seg,
            &mut dir,
            last,
            KinematicPathMode::Once
        ));
        assert_eq!(seg, 1);
        seg = last;
        assert!(
            !lookahead_advance(&mut seg, &mut dir, last, KinematicPathMode::Once),
            "Once stops at the end"
        );
        assert_eq!(seg, last);
    }

    #[test]
    fn lookahead_loop_wraps_to_zero() {
        let (mut seg, mut dir) = (2usize, 1i32);
        assert!(lookahead_advance(
            &mut seg,
            &mut dir,
            2,
            KinematicPathMode::Loop
        ));
        assert_eq!(seg, 0, "Loop wraps from last back to 0");
    }

    #[test]
    fn lookahead_pingpong_reverses_at_both_ends() {
        let (mut seg, mut dir) = (2usize, 1i32);
        lookahead_advance(&mut seg, &mut dir, 2, KinematicPathMode::PingPong);
        assert_eq!(dir, -1, "forward at the far end flips to reverse");
        seg = 0;
        dir = -1;
        lookahead_advance(&mut seg, &mut dir, 2, KinematicPathMode::PingPong);
        assert_eq!(dir, 1, "reverse at 0 flips to forward");
    }

    /// The same two-point `Loop` spin the platform stepper had, and the same
    /// closed circuit that fixes it.
    ///
    /// Found in the platform copy, where it hung a test binary outright; this asserts that a spike
    /// ball or patrol dummy on a two-waypoint looping path both RETURNS from its frame and goes
    /// round rather than stopping.
    #[test]
    fn a_two_point_looping_path_circulates_instead_of_spinning() {
        let a = ae::Vec2::new(0.0, 0.0);
        let b = ae::Vec2::new(300.0, 0.0);
        let mut motion = PathMotion::new(ambition_platformer2d_core::KinematicPath {
            points: vec![a, b],
            speed: 600.0,
            mode: KinematicPathMode::Loop,
            start_offset_seconds: 0.0,
        });

        let dt = 1.0 / 60.0;
        let step = 600.0 * dt;
        let mut pos = a;
        let mut reached_b = false;
        let mut back_near_a = false;
        // Four traverses of the 300px leg. Without the guard this never returns.
        for _ in 0..120 {
            pos = motion.advance(pos, dt);
            if (pos - b).length() <= step {
                reached_b = true;
            }
            if reached_b && (pos - a).length() <= step {
                back_near_a = true;
            }
        }

        assert!(reached_b, "it should reach the far point: {pos:?}");
        assert!(
            back_near_a,
            "and come back round — a `Loop` that stops at the far end is the bug \
             this test exists for: {pos:?}"
        );
    }
}
