//! Motion-input gesture recognition: a rolling directional buffer, a generic
//! ordered-subsequence matcher ([`MotionInputBuffer::detect_sequence`]), and an
//! **open, content-owned** [`MotionTechniqueCatalog`] of named techniques.
//!
//! The reusable input crate owns no named technique. A game registers its own
//! motion techniques (a quarter-circle, a half-circle, a dragon-punch, …) from
//! its content crate via [`MotionTechniqueAppExt::register_motion_technique`],
//! each a set of direction patterns; the fire/action systems ask the catalog
//! whether a registered technique fired this frame.

use std::collections::{BTreeMap, VecDeque};

use bevy::prelude::{App, Resource};

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

    /// Detect an ordered subsequence in the recent samples. Returns
    /// `Some(facing)` based on the final direction (`+1.0` for right,
    /// `-1.0` for left, `+1.0` for up/down ambiguity).
    ///
    /// We don't require strict adjacency; intermediate Neutral or extra cardinal
    /// samples are tolerated as long as the expected directions appear in order
    /// within the buffer window. This is the generic substrate every named
    /// [`MotionTechnique`] is built from — the reusable input crate names no
    /// specific gesture.
    pub fn detect_sequence(&self, expected: &[MotionDirection]) -> Option<f32> {
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

/// A named motion-input technique: any one of its `patterns` matching the recent
/// buffer counts as the technique firing. `invert_facing` flips the reported
/// facing — a half-circle conventionally fires the direction opposite the one it
/// ends on, so its authored patterns request the flip.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionTechnique {
    /// Alternative direction sequences (e.g. the right-facing and left-facing
    /// forms of the same gesture). Matched by [`MotionInputBuffer::detect_sequence`].
    pub patterns: Vec<Vec<MotionDirection>>,
    /// Negate the facing the matched pattern reports.
    pub invert_facing: bool,
}

impl MotionTechnique {
    /// Build a technique from its alternative patterns (no facing inversion).
    pub fn new(patterns: Vec<Vec<MotionDirection>>) -> Self {
        Self {
            patterns,
            invert_facing: false,
        }
    }

    /// Report `Some(facing)` if any of this technique's patterns is present in
    /// `buffer`, with `invert_facing` applied.
    pub fn detect(&self, buffer: &MotionInputBuffer) -> Option<f32> {
        for pattern in &self.patterns {
            if let Some(facing) = buffer.detect_sequence(pattern) {
                return Some(if self.invert_facing { -facing } else { facing });
            }
        }
        None
    }
}

/// Open, content-populated registry of named motion techniques.
///
/// Empty by default; a game registers its own via
/// [`MotionTechniqueAppExt::register_motion_technique`]. The reusable input crate
/// names none — the fire/action systems query techniques by id, so a second game
/// adds a gesture without editing this crate.
#[derive(Resource, Clone, Debug, Default)]
pub struct MotionTechniqueCatalog {
    techniques: BTreeMap<String, MotionTechnique>,
}

impl MotionTechniqueCatalog {
    /// The registered technique for `id`, or `None`.
    pub fn get(&self, id: &str) -> Option<&MotionTechnique> {
        self.techniques.get(id)
    }

    /// Detect technique `id` against `buffer`. `None` if the id is unregistered
    /// or no pattern matched.
    pub fn detect(&self, id: &str, buffer: &MotionInputBuffer) -> Option<f32> {
        self.techniques.get(id).and_then(|t| t.detect(buffer))
    }

    /// Register a named technique. Idempotent for an identical (id, technique);
    /// panics on a conflicting re-registration so an authoring mistake is loud at
    /// startup, matching the other content catalogs.
    fn register(&mut self, id: impl Into<String>, technique: MotionTechnique) {
        let id = id.into();
        match self.techniques.get(&id) {
            Some(existing) if *existing == technique => {}
            Some(_) => {
                panic!("motion technique {id:?} already registered with a different pattern set")
            }
            None => {
                self.techniques.insert(id, technique);
            }
        }
    }
}

/// Composition-time sugar for registering a named motion technique from a content
/// crate, mirroring the other `register_*` content seams.
pub trait MotionTechniqueAppExt {
    fn register_motion_technique(
        &mut self,
        id: impl Into<String>,
        technique: MotionTechnique,
    ) -> &mut Self;
}

impl MotionTechniqueAppExt for App {
    fn register_motion_technique(
        &mut self,
        id: impl Into<String>,
        technique: MotionTechnique,
    ) -> &mut Self {
        self.init_resource::<MotionTechniqueCatalog>();
        self.world_mut()
            .resource_mut::<MotionTechniqueCatalog>()
            .register(id, technique);
        self
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
