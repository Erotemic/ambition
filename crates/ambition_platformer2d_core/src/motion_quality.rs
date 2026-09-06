//! Numeric diagnostics for the shape of an authoritative per-tick trajectory.
//!
//! [`measure_motion`] summarizes step distance, changes in step (jerk), reversals, path
//! length, and net displacement. Ratio metrics normalize jerk against mean movement so
//! thresholds can be compared across slow and fast bodies. Feed simulation positions,
//! not presentation-smoothed positions, so the measurement identifies simulation motion
//! rather than camera/render behavior.

use crate::Vec2;

/// Steps below this (in px) are treated as "not moving" for reversal counting,
/// so floating-point dither in a resting body is not reported as oscillation.
const STILL_PX: f32 = 1e-3;

/// A trajectory's shape, summarized. Build one with [`measure_motion`].
///
/// Every length is in world pixels. Every "per tick" figure is per SIM TICK, the
/// unit the track was sampled at — deliberately not per second, because the
/// quantity that matters is what changes between two drawn states.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionQuality {
    /// Positions in the track.
    pub samples: usize,
    /// Mean distance travelled per tick.
    pub mean_step: f32,
    /// Largest single-tick travel.
    pub max_step: f32,
    /// Mean change in per-tick travel — the average jerk.
    pub mean_jerk: f32,
    /// The headline. Largest change in per-tick travel across the track: the
    /// worst single discontinuity in the motion.
    pub max_jerk: f32,
    /// Tick index (into the track) where [`Self::max_jerk`] occurred, so a
    /// failure can point at WHEN rather than only how much.
    pub max_jerk_at: usize,
    /// Consecutive steps that pointed in opposing directions (dot < 0), ignoring
    /// steps below [`STILL_PX`].
    pub reversals: usize,
    /// Straight-line distance from the first sample to the last.
    pub net_displacement: f32,
    /// Total distance travelled along the path.
    pub path_length: f32,
    /// Ticks whose travel was below [`STILL_PX`] — a body that stopped.
    pub stalled_ticks: usize,
}

impl MotionQuality {
    /// Jerk as a multiple of ordinary travel — the scale-free reading.
    ///
    /// `0.0` for a body that never moved and never jerked.
    pub fn jerk_ratio(&self) -> f32 {
        if self.mean_step <= STILL_PX {
            if self.max_jerk <= STILL_PX {
                0.0
            } else {
                f32::INFINITY
            }
        } else {
            self.max_jerk / self.mean_step
        }
    }

    /// Fraction of steps that reversed direction. A turn or two is normal; a
    /// sustained rate is an oscillation.
    pub fn reversal_rate(&self) -> f32 {
        let steps = self.samples.saturating_sub(2);
        if steps == 0 {
            0.0
        } else {
            self.reversals as f32 / steps as f32
        }
    }

    /// How directly the path got where it ended up, in `[0, 1]`. A body that
    /// crawled 200 px along a wall scores ~1; one that shuffled back and forth
    /// over the same corner for 200 px of path and ended up where it started
    /// scores ~0. `1.0` for a body that never moved (vacuously direct).
    pub fn straightness(&self) -> f32 {
        if self.path_length <= STILL_PX {
            1.0
        } else {
            (self.net_displacement / self.path_length).clamp(0.0, 1.0)
        }
    }

    /// One line, for a test failure message or a probe dump.
    pub fn summary(&self) -> String {
        format!(
            "{} samples | step mean {:.3} max {:.3} px | jerk mean {:.3} max {:.3} px \
             (x{:.1} step, at tick {}) | {} reversals ({:.0}%) | {} stalled | \
             straightness {:.2} | path {:.1} px net {:.1} px",
            self.samples,
            self.mean_step,
            self.max_step,
            self.mean_jerk,
            self.max_jerk,
            self.jerk_ratio(),
            self.max_jerk_at,
            self.reversals,
            self.reversal_rate() * 100.0,
            self.stalled_ticks,
            self.straightness(),
            self.path_length,
            self.net_displacement,
        )
    }
}

/// Measure the shape of a per-tick position track.
///
/// Fewer than three samples cannot have a second difference, so the jerk figures
/// come back zero rather than pretending — a caller measuring a two-tick window
/// gets honest emptiness instead of a false pass.
pub fn measure_motion(track: &[Vec2]) -> MotionQuality {
    let steps: Vec<Vec2> = track.windows(2).map(|pair| pair[1] - pair[0]).collect();
    let lengths: Vec<f32> = steps.iter().map(|step| step.length()).collect();
    let mut quality = MotionQuality {
        samples: track.len(),
        mean_step: 0.0,
        max_step: 0.0,
        mean_jerk: 0.0,
        max_jerk: 0.0,
        max_jerk_at: 0,
        reversals: 0,
        net_displacement: 0.0,
        path_length: lengths.iter().sum(),
        stalled_ticks: lengths.iter().filter(|len| **len <= STILL_PX).count(),
    };
    if let (Some(first), Some(last)) = (track.first(), track.last()) {
        quality.net_displacement = first.distance(*last);
    }
    if !lengths.is_empty() {
        quality.mean_step = quality.path_length / lengths.len() as f32;
        quality.max_step = lengths.iter().copied().fold(0.0, f32::max);
    }
    // The second difference, and the sign flips, both live between CONSECUTIVE
    // steps — so both loops are the same walk over pairs of steps.
    let mut jerk_total = 0.0;
    for (index, pair) in steps.windows(2).enumerate() {
        let jerk = (pair[1] - pair[0]).length();
        jerk_total += jerk;
        if jerk > quality.max_jerk {
            quality.max_jerk = jerk;
            // +1 so the index names the sample BETWEEN the two steps: the tick
            // at which the motion changed.
            quality.max_jerk_at = index + 1;
        }
        if lengths[index] > STILL_PX && lengths[index + 1] > STILL_PX && pair[0].dot(pair[1]) < 0.0
        {
            quality.reversals += 1;
        }
    }
    if steps.len() >= 2 {
        quality.mean_jerk = jerk_total / (steps.len() - 1) as f32;
    }
    quality
}

/// Named bounds on a trajectory's shape, so a test states its expectation in
/// prose-shaped fields rather than a wall of bare comparisons.
///
/// Construct with [`Self::CRAWLING`] or a literal, then [`Self::violations`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionBudget {
    /// Absolute ceiling on the worst single-tick change in travel.
    pub max_jerk: f32,
    /// Ceiling on the worst jerk as a multiple of ordinary travel.
    pub max_jerk_ratio: f32,
    /// Ceiling on the fraction of steps that reverse direction.
    pub max_reversal_rate: f32,
    /// Floor on how directly the body got where it ended up.
    pub min_straightness: f32,
}

impl MotionBudget {
    /// What a body crawling a continuous surface should look like: it may turn a
    /// corner (one reversal in a short window is a large *rate*, so this is not
    /// tight), but it must never lurch, and it must make progress.
    pub const CRAWLING: Self = Self {
        max_jerk: 4.0,
        max_jerk_ratio: 6.0,
        max_reversal_rate: 0.10,
        min_straightness: 0.25,
    };

    /// Every bound this quality breaks, as readable clauses. Empty means it fits.
    pub fn violations(&self, quality: &MotionQuality) -> Vec<String> {
        let mut broken = Vec::new();
        if quality.max_jerk > self.max_jerk {
            broken.push(format!(
                "worst jerk {:.3} px > {:.3} px allowed (at tick {})",
                quality.max_jerk, self.max_jerk, quality.max_jerk_at
            ));
        }
        if quality.jerk_ratio() > self.max_jerk_ratio {
            broken.push(format!(
                "worst jerk is {:.1}x the mean step, > {:.1}x allowed",
                quality.jerk_ratio(),
                self.max_jerk_ratio
            ));
        }
        if quality.reversal_rate() > self.max_reversal_rate {
            broken.push(format!(
                "{:.0}% of steps reversed direction, > {:.0}% allowed ({} reversals)",
                quality.reversal_rate() * 100.0,
                self.max_reversal_rate * 100.0,
                quality.reversals
            ));
        }
        if quality.straightness() < self.min_straightness {
            broken.push(format!(
                "straightness {:.2} < {:.2} required (path {:.1} px, net {:.1} px)",
                quality.straightness(),
                self.min_straightness,
                quality.path_length,
                quality.net_displacement
            ));
        }
        broken
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A body moving at a constant rate has zero jerk — the calibration case.
    #[test]
    fn constant_motion_has_no_jerk() {
        let track: Vec<Vec2> = (0..60).map(|i| Vec2::new(i as f32 * 0.67, 10.0)).collect();
        let quality = measure_motion(&track);
        assert!(quality.max_jerk < 1e-4, "{}", quality.summary());
        assert_eq!(quality.reversals, 0);
        assert!((quality.straightness() - 1.0).abs() < 1e-4);
        assert!(MotionBudget::CRAWLING.violations(&quality).is_empty());
    }

    /// A body oscillating over one spot: tiny steps, but every one reverses and
    /// it ends where it started. Absolute jerk is small — the RATIO and the
    /// straightness are what name it, which is the reason both exist.
    #[test]
    fn an_oscillation_is_named_by_its_reversals_not_its_size() {
        let track: Vec<Vec2> = (0..60)
            .map(|i| Vec2::new(if i % 2 == 0 { 0.0 } else { 0.7 }, 0.0))
            .collect();
        let quality = measure_motion(&track);
        assert!(quality.max_jerk < 2.0, "the lurch itself is small");
        assert!(
            quality.reversal_rate() > 0.9,
            "but nearly every step reverses: {}",
            quality.summary()
        );
        assert!(quality.straightness() < 0.05, "and it goes nowhere");
        let broken = MotionBudget::CRAWLING.violations(&quality);
        assert_eq!(
            broken.len(),
            2,
            "reversal rate and straightness: {broken:?}"
        );
    }

    /// One snap in an otherwise steady crawl. The absolute jerk catches it even
    /// though the path is straight and never reverses.
    #[test]
    fn a_single_snap_is_caught_by_absolute_jerk() {
        let mut track: Vec<Vec2> = (0..30).map(|i| Vec2::new(i as f32 * 0.67, 0.0)).collect();
        let jump = track.last().copied().expect("non-empty") + Vec2::new(24.0, 0.0);
        track.push(jump);
        track.extend((1..30).map(|i| jump + Vec2::new(i as f32 * 0.67, 0.0)));
        let quality = measure_motion(&track);
        assert_eq!(quality.reversals, 0, "a snap is not an oscillation");
        assert!(quality.straightness() > 0.99, "nor a failure to progress");
        assert!(quality.max_jerk > 20.0, "{}", quality.summary());
        assert_eq!(
            quality.max_jerk_at, 30,
            "and it names the tick it happened on"
        );
        assert!(!MotionBudget::CRAWLING.violations(&quality).is_empty());
    }

    /// A body that never moves is vacuously fine, and a track too short to have
    /// a second difference reports zero rather than guessing.
    #[test]
    fn degenerate_tracks_report_honestly() {
        let still = measure_motion(&[Vec2::ZERO; 10]);
        assert_eq!(still.max_jerk, 0.0);
        assert_eq!(still.jerk_ratio(), 0.0);
        assert_eq!(still.stalled_ticks, 9);
        assert!(MotionBudget::CRAWLING.violations(&still).is_empty());

        let pair = measure_motion(&[Vec2::ZERO, Vec2::new(5.0, 0.0)]);
        assert_eq!(pair.max_jerk, 0.0, "two samples cannot have a jerk");
        assert_eq!(pair.mean_step, 5.0, "but they do have a step");
    }
}
