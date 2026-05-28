//! Quarter-circle / half-circle motion-input recognition. Used by
//! the sandbox to upgrade a plain Fireball press into Hadouken /
//! HadoukenSuper when the player buffered the right gesture.

use std::collections::VecDeque;

/// Snapshot of a single recorded directional sample, captured by
/// [`MotionInputBuffer`] for motion-input recognition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionSample {
    /// Discrete direction; quantized to one of the 8 cardinals so
    /// recognition is robust against noisy analog input.
    pub dir: MotionDirection,
    /// Time when this sample was recorded, in arbitrary monotonic
    /// seconds. The buffer prunes samples older than its window.
    pub time: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MotionDirection {
    Neutral,
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl MotionDirection {
    /// Quantize an axis vector to a cardinal direction. `threshold`
    /// is the magnitude below which the direction is `Neutral`.
    pub fn from_axis(x: f32, y: f32, threshold: f32) -> Self {
        let mag = (x * x + y * y).sqrt();
        if mag < threshold {
            return Self::Neutral;
        }
        let xs = x.abs() > threshold * 0.5;
        let ys = y.abs() > threshold * 0.5;
        match (xs, ys, x.signum(), y.signum()) {
            (true, true, sx, sy) if sx > 0.0 && sy < 0.0 => Self::UpRight,
            (true, true, sx, sy) if sx < 0.0 && sy < 0.0 => Self::UpLeft,
            (true, true, sx, sy) if sx > 0.0 && sy > 0.0 => Self::DownRight,
            (true, true, sx, sy) if sx < 0.0 && sy > 0.0 => Self::DownLeft,
            (true, _, sx, _) if sx > 0.0 => Self::Right,
            (true, _, _, _) => Self::Left,
            (_, _, _, sy) if sy < 0.0 => Self::Up,
            _ => Self::Down,
        }
    }
}

/// Rolling buffer of recent directional samples. Used by motion-input
/// recognizers to test for quarter-circle / half-circle gestures.
///
/// Records samples even when the direction is `Neutral`; that lets the
/// recognizer require a Neutral pause between distinct gestures so a
/// constant Right hold is not interpreted as repeated half-circles.
#[derive(Clone, Debug)]
pub struct MotionInputBuffer {
    pub samples: VecDeque<MotionSample>,
    /// Maximum age in seconds for samples to be retained.
    pub window: f32,
}

impl MotionInputBuffer {
    pub fn new(window: f32) -> Self {
        Self {
            samples: VecDeque::with_capacity(64),
            window,
        }
    }

    /// Record one sample at `now`. Prunes anything older than
    /// `now - window`. Collapses repeats so a held direction does
    /// not flood the buffer.
    pub fn push(&mut self, dir: MotionDirection, now: f32) {
        match self.samples.back() {
            Some(prev) if prev.dir == dir => {
                // Same direction continues — update only the time of
                // the most recent occurrence so the window math sees
                // a fresh sample.
                let last = self.samples.back_mut().unwrap();
                last.time = now;
            }
            _ => {
                self.samples.push_back(MotionSample { dir, time: now });
            }
        }
        let cutoff = now - self.window;
        while let Some(front) = self.samples.front() {
            if front.time < cutoff {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    /// Iterator over recent (oldest-first) directions, ignoring time.
    /// Exposed as part of the public motion-input API even though the
    /// in-tree QCF recognizer reaches into `samples` directly; future
    /// gesture matchers (e.g. half-circle, dragon-punch) are expected
    /// to consume this iterator. Tested in the bottom-of-file
    /// `tests` mod so the API stays callable even though no
    /// production code calls it today.
    #[allow(dead_code)]
    pub fn directions(&self) -> impl Iterator<Item = MotionDirection> + '_ {
        self.samples.iter().map(|s| s.dir)
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    /// Recognize a `Down → DownRight → Right` quarter-circle (or its
    /// mirror image) finishing recently. Returns `Some(facing)` where
    /// facing is +1 (right) or -1 (left) to match the player's
    /// `facing` field.
    ///
    /// We don't require strict adjacency; intermediate Neutral or
    /// extra cardinal samples are tolerated as long as the three key
    /// directions appear in order within the buffer window.
    pub fn detect_quarter_circle(&self) -> Option<f32> {
        if let Some(facing) = self.detect_sequence(&[
            MotionDirection::Down,
            MotionDirection::DownRight,
            MotionDirection::Right,
        ]) {
            return Some(facing);
        }
        if let Some(facing) = self.detect_sequence(&[
            MotionDirection::Down,
            MotionDirection::DownLeft,
            MotionDirection::Left,
        ]) {
            return Some(facing);
        }
        None
    }

    /// Recognize a *grace* quarter-circle: just `Down → Right` (or
    /// its mirror), without requiring the diagonal `DownRight`
    /// midpoint. Hitting the diagonal is awkward on a keyboard with
    /// 4 cardinal arrow keys, so the grace shape is the easy-mode
    /// path to a Hadouken; the full 3-step
    /// (`detect_quarter_circle`) gates the stronger projectile.
    ///
    /// IMPORTANT: this MUST be checked AFTER `detect_quarter_circle`
    /// because a 3-step Down → DownRight → Right also satisfies
    /// "Down somewhere before Right" and would match the grace form
    /// — caller decides which gate fires first.
    pub fn detect_quarter_circle_grace(&self) -> Option<f32> {
        if let Some(facing) = self.detect_sequence(&[MotionDirection::Down, MotionDirection::Right])
        {
            return Some(facing);
        }
        if let Some(facing) = self.detect_sequence(&[MotionDirection::Down, MotionDirection::Left])
        {
            return Some(facing);
        }
        None
    }

    /// Recognize a half-circle: `Right → DownRight → Down → DownLeft → Left`
    /// (or mirror). Treated as a stronger gesture than the quarter
    /// circle and used in the sandbox to upgrade `Fireball` to
    /// `Hadouken`. The mirror form returns `-1.0`.
    pub fn detect_half_circle(&self) -> Option<f32> {
        if let Some(facing) = self.detect_sequence(&[
            MotionDirection::Right,
            MotionDirection::DownRight,
            MotionDirection::Down,
            MotionDirection::DownLeft,
            MotionDirection::Left,
        ]) {
            return Some(-facing);
        }
        if let Some(facing) = self.detect_sequence(&[
            MotionDirection::Left,
            MotionDirection::DownLeft,
            MotionDirection::Down,
            MotionDirection::DownRight,
            MotionDirection::Right,
        ]) {
            return Some(-facing);
        }
        None
    }

    /// Detect an ordered subsequence in the recent samples. Returns
    /// `Some(facing)` based on the final direction (`+1.0` for right,
    /// `-1.0` for left, `+1.0` for up/down ambiguity).
    fn detect_sequence(&self, expected: &[MotionDirection]) -> Option<f32> {
        if expected.is_empty() {
            return None;
        }
        let mut idx = 0;
        for sample in self.samples.iter() {
            if sample.dir == expected[idx] {
                idx += 1;
                if idx == expected.len() {
                    let last = expected[expected.len() - 1];
                    return Some(match last {
                        MotionDirection::Right
                        | MotionDirection::UpRight
                        | MotionDirection::DownRight => 1.0,
                        MotionDirection::Left
                        | MotionDirection::UpLeft
                        | MotionDirection::DownLeft => -1.0,
                        _ => 1.0,
                    });
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push(buffer: &mut MotionInputBuffer, dir: MotionDirection, at: f32) {
        buffer.samples.push_back(MotionSample { dir, time: at });
    }

    #[test]
    fn directions_yields_samples_in_oldest_first_order() {
        let mut buffer = MotionInputBuffer::new(0.5);
        push(&mut buffer, MotionDirection::Down, 0.0);
        push(&mut buffer, MotionDirection::DownRight, 0.05);
        push(&mut buffer, MotionDirection::Right, 0.10);
        let dirs: Vec<MotionDirection> = buffer.directions().collect();
        assert_eq!(
            dirs,
            vec![
                MotionDirection::Down,
                MotionDirection::DownRight,
                MotionDirection::Right,
            ]
        );
    }

    #[test]
    fn directions_is_empty_when_buffer_is_empty() {
        let buffer = MotionInputBuffer::new(0.5);
        assert_eq!(buffer.directions().count(), 0);
    }
}
