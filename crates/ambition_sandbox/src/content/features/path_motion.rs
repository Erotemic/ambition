use super::*;

#[derive(Clone, Debug)]
pub struct PathMotion {
    path: crate::actor::KinematicPath,
    segment: usize,
    dir: i32,
}

impl PathMotion {
    pub(super) fn new(path: crate::actor::KinematicPath) -> Self {
        Self {
            path,
            segment: 0,
            dir: 1,
        }
    }

    pub(super) fn start_pos(&self) -> Option<ae::Vec2> {
        self.path.points.first().copied()
    }

    pub(super) fn advance(&mut self, mut pos: ae::Vec2, dt: f32) -> ae::Vec2 {
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

    pub(super) fn advance_segment(&mut self) {
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
