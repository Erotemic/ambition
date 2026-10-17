//! The geometry of lightning: jagged strands in a local frame, and the
//! triangles that draw them. Pure functions of a seed, so a test can hold a
//! path still.
//!
//! A bolt's frame runs from its start (0, 0) to its end (1, 0), so a path
//! stretches with its ends as they move between new paths. An aura's frame is
//! its body's ellipse as the unit circle.

use bevy::math::Vec2;

/// A small, fast generator; presentation only, so any good spread will do.
#[derive(Clone, Copy, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    pub fn next_u64(&mut self) -> u64 {
        // SplitMix64.
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform in `0..1`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform in `lo..hi`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }

    /// Uniform in `-1..1`.
    pub fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}

/// One jagged line of a bolt or an aura, in its frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Strand {
    pub points: Vec<Vec2>,
    /// The width at each point, over the look's width.
    pub widths: Vec<f32>,
    /// How far along its bolt (0..1) the growing head must be before this
    /// strand shows. The trunk grows; a fork shows whole once its root has.
    pub appears_at: f32,
    /// True for the trunk, the strand a growing bolt draws a prefix of.
    pub trunk: bool,
}

/// A jagged path from `a` to `b` of `segments` (a power of two) pieces, by
/// midpoint displacement: each level moves each middle across its piece by
/// up to `jag` of the whole length, halving as the pieces halve.
pub fn jagged(rng: &mut Rng, a: Vec2, b: Vec2, segments: usize, jag: f32) -> Vec<Vec2> {
    let segments = segments.max(1).next_power_of_two();
    let mut points = vec![a; segments + 1];
    points[segments] = b;
    let across = (b - a).perp().normalize_or_zero();
    let mut amp = jag * (b - a).length();
    let mut step = segments;
    while step > 1 {
        let half = step / 2;
        for i in (0..segments).step_by(step) {
            let mid = (points[i] + points[i + step]) * 0.5;
            points[i + half] = mid + across * (rng.signed() * amp);
        }
        amp *= 0.55;
        step = half;
    }
    points
}

/// How many pieces a bolt `length` world units long is cut into.
pub fn segments_for(length: f32) -> usize {
    ((length / 14.0).clamp(4.0, 64.0) as usize).next_power_of_two()
}

/// A bolt from (0, 0) to (1, 0) in its frame: the trunk, and `forks` side
/// strands off it. `length` is its length in the world, which sets how
/// finely it is cut.
pub fn bolt(rng: &mut Rng, length: f32, jag: f32, forks: usize) -> Vec<Strand> {
    let n = segments_for(length);
    let trunk = jagged(rng, Vec2::ZERO, Vec2::X, n, jag);
    let mut strands = Vec::with_capacity(1 + forks);
    for _ in 0..forks {
        let k = rng.range(0.15, 0.8) * n as f32;
        let k = (k as usize).clamp(1, n - 1);
        let root = trunk[k];
        let along = (trunk[k + 1] - trunk[k - 1]).normalize_or(Vec2::X);
        let turn = rng.range(0.35, 0.95) * if rng.unit() < 0.5 { -1.0 } else { 1.0 };
        let reach = rng.range(0.1, 0.28) * (1.0 - k as f32 / n as f32) + 0.04;
        let end = root + Vec2::from_angle(turn).rotate(along) * reach;
        let points = jagged(rng, root, end, (n / 4).max(4), jag * 1.2);
        let m = points.len() - 1;
        let widths = (0..=m).map(|j| 0.55 - 0.5 * j as f32 / m as f32).collect();
        strands.push(Strand { points, widths, appears_at: k as f32 / n as f32, trunk: false });
    }
    let widths = vec![1.0; trunk.len()];
    strands.insert(0, Strand { points: trunk, widths, appears_at: 0.0, trunk: true });
    strands
}

/// The arcs of an aura in the unit circle of its body: some crawl the
/// outline, some leap off it, a few cross the body. Each of `arcs` is drawn
/// this flicker with a chance, so the crackle comes and goes.
pub fn aura(rng: &mut Rng, jag: f32, arcs: usize) -> Vec<Strand> {
    let mut strands = Vec::with_capacity(arcs);
    let on_ring = |angle: f32, r: f32| Vec2::from_angle(angle) * r;
    for _ in 0..arcs {
        if rng.unit() < 0.2 {
            continue;
        }
        let kind = rng.unit();
        let theta = rng.range(0.0, std::f32::consts::TAU);
        let (a, b, bulge) = if kind < 0.55 {
            // Crawls the outline.
            let sweep = rng.range(0.35, 0.95) * if rng.unit() < 0.5 { -1.0 } else { 1.0 };
            (on_ring(theta, rng.range(0.8, 1.0)), on_ring(theta + sweep, rng.range(0.8, 1.0)), 0.16)
        } else if kind < 0.85 {
            // Leaps off it.
            let out = rng.range(1.2, 1.55);
            (on_ring(theta, 0.92), on_ring(theta + rng.signed() * 0.25, out), 0.0)
        } else {
            // Crosses the body.
            let a = on_ring(theta, rng.range(0.2, 0.75));
            (a, a + on_ring(theta + rng.range(1.2, 2.0), rng.range(0.35, 0.6)), 0.0)
        };
        let mut points = jagged(rng, a, b, 8, jag);
        let m = points.len() - 1;
        for (j, p) in points.iter_mut().enumerate() {
            let s = j as f32 / m as f32;
            *p += p.normalize_or_zero() * bulge * (std::f32::consts::PI * s).sin();
        }
        let widths = (0..=m).map(|j| 0.95 - 0.6 * j as f32 / m as f32).collect();
        strands.push(Strand { points, widths, appears_at: 0.0, trunk: false });
    }
    strands
}

/// Triangles with a colour at each corner: one mesh of a whole effect.
#[derive(Clone, Debug, Default)]
pub struct Tris {
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl Tris {
    fn vertex(&mut self, at: Vec2, color: [f32; 4]) -> u32 {
        self.positions.push([at.x, at.y, 0.0]);
        self.colors.push(color);
        (self.positions.len() - 1) as u32
    }

    /// A soft band along `points` (in mesh space), `widths` across: `center`
    /// down its middle, fading to `edge` at its sides, with soft caps.
    pub fn ribbon(&mut self, points: &[Vec2], widths: &[f32], center: [f32; 4], edge: [f32; 4]) {
        if points.len() < 2 {
            return;
        }
        let n = points.len();
        let mut rows = Vec::with_capacity(n);
        for i in 0..n {
            let ahead = points[(i + 1).min(n - 1)] - points[i.saturating_sub(1)];
            let across = ahead.perp().normalize_or(Vec2::Y) * (widths[i] * 0.5);
            let l = self.vertex(points[i] + across, edge);
            let c = self.vertex(points[i], center);
            let r = self.vertex(points[i] - across, edge);
            rows.push([l, c, r]);
        }
        for w in rows.windows(2) {
            let ([l0, c0, r0], [l1, c1, r1]) = (w[0], w[1]);
            self.indices.extend_from_slice(&[l0, c0, l1, c0, c1, l1, c0, r0, c1, r0, r1, c1]);
        }
        // A cap past each end, so a strand ends in a point of light, not a cut.
        for (end, inner, row) in [(0, 1, rows[0]), (n - 1, n - 2, rows[n - 1])] {
            let out = (points[end] - points[inner]).normalize_or(Vec2::X) * (widths[end] * 0.5);
            let tip = self.vertex(points[end] + out, edge);
            let [l, c, r] = row;
            self.indices.extend_from_slice(&[l, c, tip, c, r, tip]);
        }
    }

    /// A disc of light at `at`, `center` in the middle fading to `edge`.
    pub fn glow(&mut self, at: Vec2, radius: f32, center: [f32; 4], edge: [f32; 4]) {
        if radius <= 0.0 {
            return;
        }
        let hub = self.vertex(at, center);
        let sides = 16;
        let first = self.positions.len() as u32;
        for i in 0..sides {
            let angle = i as f32 / sides as f32 * std::f32::consts::TAU;
            self.vertex(at + Vec2::from_angle(angle) * radius, edge);
        }
        for i in 0..sides {
            self.indices.extend_from_slice(&[hub, first + i, first + (i + 1) % sides]);
        }
    }
}

/// The part of a trunk shown when its head is `head` (0..1) of the way out.
pub fn grown(points: &[Vec2], head: f32) -> Vec<Vec2> {
    if head >= 1.0 || points.len() < 2 {
        return points.to_vec();
    }
    let span = head.max(0.0) * (points.len() - 1) as f32;
    let whole = span.floor() as usize;
    let mut shown: Vec<Vec2> = points[..=whole].to_vec();
    let rest = span - whole as f32;
    if rest > 0.0 {
        shown.push(points[whole].lerp(points[whole + 1], rest));
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_jagged_path_keeps_its_ends_and_strays_from_its_line() {
        let mut rng = Rng::new(3);
        let path = jagged(&mut rng, Vec2::ZERO, Vec2::X, 16, 0.1);
        assert_eq!(path.len(), 17);
        assert_eq!(path[0], Vec2::ZERO);
        assert_eq!(path[16], Vec2::X);
        assert!(path.iter().any(|p| p.y.abs() > 0.01), "a straight line is not lightning");
        // Bounded: the halving amplitudes sum to under 2.25 x the first.
        assert!(path.iter().all(|p| p.y.abs() < 0.1 * 2.25));
    }

    #[test]
    fn the_same_seed_draws_the_same_bolt_and_another_seed_another() {
        let a = bolt(&mut Rng::new(9), 300.0, 0.1, 3);
        assert_eq!(a, bolt(&mut Rng::new(9), 300.0, 0.1, 3));
        assert_ne!(a, bolt(&mut Rng::new(10), 300.0, 0.1, 3));
    }

    #[test]
    fn a_bolt_is_a_trunk_and_its_forks_and_a_fork_starts_on_the_trunk() {
        let strands = bolt(&mut Rng::new(1), 400.0, 0.1, 4);
        assert_eq!(strands.len(), 5);
        assert!(strands[0].trunk);
        for fork in &strands[1..] {
            assert!(!fork.trunk);
            assert!(strands[0].points.contains(&fork.points[0]), "a fork grows from the trunk");
            assert!(fork.widths.last() < fork.widths.first(), "a fork thins");
        }
    }

    #[test]
    fn a_longer_bolt_is_cut_finer() {
        assert!(segments_for(800.0) > segments_for(60.0));
        assert!(segments_for(1.0) >= 4);
        assert!(segments_for(1.0e6) <= 64);
    }

    #[test]
    fn an_aura_stays_near_its_body() {
        let mut rng = Rng::new(5);
        for _ in 0..50 {
            for strand in aura(&mut rng, 0.2, 6) {
                for p in strand.points {
                    assert!(p.length() < 2.0, "{p:?} is far off the body");
                }
            }
        }
    }

    #[test]
    fn a_growing_trunk_shows_a_prefix_and_all_of_it_when_grown() {
        let points: Vec<Vec2> = (0..=4).map(|i| Vec2::new(i as f32, 0.0)).collect();
        assert_eq!(grown(&points, 1.0), points);
        let half = grown(&points, 0.5);
        assert_eq!(half.last(), Some(&Vec2::new(2.0, 0.0)));
        assert_eq!(grown(&points, 0.375).last(), Some(&Vec2::new(1.5, 0.0)));
    }

    #[test]
    fn a_ribbon_has_three_corners_a_point_and_two_caps() {
        let mut tris = Tris::default();
        let points = [Vec2::ZERO, Vec2::X, Vec2::new(2.0, 0.0)];
        tris.ribbon(&points, &[1.0; 3], [1.0; 4], [0.0; 4]);
        assert_eq!(tris.positions.len(), 3 * 3 + 2);
        assert_eq!(tris.indices.len(), 2 * 12 + 2 * 6);
        assert!(tris.indices.iter().all(|&i| (i as usize) < tris.positions.len()));
    }
}
