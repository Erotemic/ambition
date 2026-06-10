use super::*;

#[derive(Clone, Debug)]
pub struct PathMotion {
    path: crate::actor::KinematicPath,
    segment: usize,
    dir: i32,
}

impl PathMotion {
    pub(crate) fn new(path: crate::actor::KinematicPath) -> Self {
        Self {
            path,
            segment: 0,
            dir: 1,
        }
    }

    pub(crate) fn start_pos(&self) -> Option<ae::Vec2> {
        self.path.points.first().copied()
    }

    pub(crate) fn advance(&mut self, mut pos: ae::Vec2, dt: f32) -> ae::Vec2 {
        if !self.path.is_valid() || dt <= 0.0 {
            return pos;
        }
        let mut remaining = self.path.speed * dt;
        while remaining > 0.0 {
            let target_index = if self.dir >= 0 {
                self.segment + 1
            } else {
                self.segment
            };
            let Some(target) = self.path.points.get(target_index).copied() else {
                break;
            };
            let to_target = target - pos;
            let distance = to_target.length();
            if distance <= 0.001 {
                self.advance_segment();
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

    pub(crate) fn advance_segment(&mut self) {
        let last_segment = self.path.points.len().saturating_sub(2);
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
    mode: crate::actor::KinematicPathMode,
) -> bool {
    match mode {
        crate::actor::KinematicPathMode::Once => {
            if *dir >= 0 && *segment < last_segment {
                *segment += 1;
                true
            } else {
                false
            }
        }
        crate::actor::KinematicPathMode::Loop => {
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
        crate::actor::KinematicPathMode::PingPong => {
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
    //! Moving-platform path following. advance walks the position toward
    //! the next waypoint by speed*dt; lookahead_advance is the (segment,
    //! dir) cursor whose Once/Loop/PingPong reversal logic is the
    //! bug-prone part (off-by-one at the ends flips a platform's travel).
    use super::*;
    use crate::actor::{KinematicPath, KinematicPathMode};

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
}
