use super::*;

#[derive(Clone, Debug)]
pub struct PathMotion {
    path: ae::KinematicPath,
    segment: usize,
    dir: i32,
}

impl PathMotion {
    pub(super) fn new(path: ae::KinematicPath) -> Self {
        Self {
            path,
            segment: 0,
            dir: 1,
        }
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
        match self.path.mode {
            ae::KinematicPathMode::Once => {
                if self.dir >= 0 && self.segment < last_segment {
                    self.segment += 1;
                }
            }
            ae::KinematicPathMode::Loop => {
                if self.dir >= 0 {
                    self.segment = if self.segment >= last_segment {
                        0
                    } else {
                        self.segment + 1
                    };
                } else if self.segment == 0 {
                    self.segment = last_segment;
                } else {
                    self.segment -= 1;
                }
            }
            ae::KinematicPathMode::PingPong => {
                if self.dir >= 0 {
                    if self.segment >= last_segment {
                        self.dir = -1;
                    } else {
                        self.segment += 1;
                    }
                } else if self.segment == 0 {
                    self.dir = 1;
                } else {
                    self.segment -= 1;
                }
            }
        }
    }
}
