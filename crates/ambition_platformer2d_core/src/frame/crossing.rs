//! Directed root crossings through a recorded finite aperture.

use bevy_math::DVec2;

use super::{ApertureStep, NumericFault, PortalAperture, material_cast::MaterialCast};
use super::thread::RefusalReason;
use crate::CombatVolume;

const MAX_INTERVALS: usize = 16_384;

/// A root on the entry plane. Times refer to the cast and the world tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RootCrossing {
    pub toi: f64,
    pub world_time: f64,
    pub point: crate::Vec2,
    pub aperture: PortalAperture,
}

struct Curve<'a, 'b> {
    cast: &'a MaterialCast<'b>,
    step: ApertureStep,
    end: PortalAperture,
    a: DVec2,
    b: DVec2,
    depth: f64,
    along: f64,
    omega: f64,
    until: f64,
}

fn perpendicular(v: DVec2) -> DVec2 { DVec2::new(-v.y, v.x) }
fn numeric(reason: NumericFault) -> RefusalReason { RefusalReason::Numeric(reason) }
fn rounding(magnitude: f64) -> f64 { magnitude * f64::EPSILON * 64.0 }

impl MaterialCast<'_> {
    /// Follow the complete recorded turn. The primitive keeps its canonical
    /// shape and follows its recorded source motion throughout this cast.
    pub fn crossing(
        &self, step: ApertureStep, end: PortalAperture, until: f64,
    ) -> Result<Option<RootCrossing>, RefusalReason> {
        if !self.root.is_finite() || !self.delta.is_finite()
            || !self.time.start.is_finite() || !self.time.end.is_finite() || !until.is_finite()
        { return Err(numeric(NumericFault::NonFinite)); }
        if self.time.start < 0.0 || self.time.end > 1.0 || self.time.start > self.time.end
            || !(0.0..=1.0).contains(&until) {
            return Err(numeric(NumericFault::InvalidFrame));
        }
        let until = until.min(self.until);
        step.sample_wide(end, self.time.start).map_err(numeric)?;
        super::thread::map_volume(self.material.domain, &self.material.to_world).map_err(numeric)?;
        let normal = end.frame.normal.as_dvec2().normalize();
        let arm = end.frame.origin.as_dvec2() - end.frame.pivot.as_dvec2();
        let span = self.time.end - self.time.start;
        if let CombatVolume::Circle { radius, .. } = self.material.domain {
            if f64::from(*radius) * self.material.to_world.scale_wide() > f64::from(end.half_length) {
                return Ok(None);
            }
        }
        let curve = Curve {
            cast: self, step, end,
            a: self.root.as_dvec2() - end.frame.pivot.as_dvec2()
                + step.displacement.as_dvec2() * (1.0 - self.time.start),
            b: self.delta.as_dvec2() - step.displacement.as_dvec2() * span,
            depth: normal.dot(arm), along: perpendicular(normal).dot(arm),
            omega: f64::from(step.turn) * span, until,
        };
        curve.first()
    }
}

impl Curve<'_, '_> {
    fn world(&self, time: f64) -> f64 {
        self.cast.time.start + (self.cast.time.end - self.cast.time.start) * time
    }
    fn normal(&self, time: f64) -> DVec2 {
        super::rotation_ending_at(f64::from(self.step.turn), self.world(time))
            * self.end.frame.normal.as_dvec2().normalize()
    }
    fn relative(&self, time: f64) -> (DVec2, DVec2) {
        let source = self.cast.source();
        let arm = bevy_math::DMat2::from_angle(source.turn() * time) * source.root_arm();
        (self.a + self.b * time + super::material_cast::rotation_delta(source.turn() * time, source.root_arm()),
            self.b + perpendicular(arm) * source.turn())
    }
    fn sample(&self, time: f64) -> (f64, f64) {
        let n = self.normal(time);
        let (x, rate) = self.relative(time);
        (n.dot(x) - self.depth,
            self.omega * perpendicular(n).dot(x) + n.dot(rate))
    }
    fn fits(&self, time: f64) -> Result<bool, RefusalReason> {
        let tangent = perpendicular(self.normal(time));
        let source = self.cast.source();
        let map = source.map_at(time).map_err(numeric)?;
        let center = self.relative(time).0;
        let root = source.root_at(time);
        let offset = map.translation() - root;
        let project = |point: DVec2| tangent.dot(
            map.direction_wide(point) * map.scale_wide() + offset + center,
        ) - self.along;
        let (lo, hi) = match self.cast.material.domain {
            CombatVolume::Circle { center, radius } => {
                let middle = project(center.as_dvec2());
                let radius = f64::from(*radius) * map.scale_wide();
                (middle - radius, middle + radius)
            }
            shape => shape.polygon().unwrap().iter().map(|p| project(p.as_dvec2()))
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| (lo.min(p), hi.max(p))),
        };
        let half = f64::from(self.end.half_length);
        let error = rounding(lo.abs() + hi.abs() + half + center.length()
            + offset.length() + self.along.abs());
        Ok(lo >= -half - error && hi <= half + error)
    }
    fn hit(&self, time: f64) -> Result<Option<RootCrossing>, RefusalReason> {
        if !self.fits(time)? { return Ok(None); }
        let (distance, _) = self.sample(time);
        let error = rounding(self.a.length() + self.b.length() * time.abs()
            + super::material_cast::rotation_delta(self.cast.source().turn() * time, self.cast.source().root_arm()).length() + self.depth.abs());
        if distance.abs() > error { return Err(numeric(NumericFault::Precision)); }
        let point = super::Sim2::IDENTITY.point_from_wide(
            self.cast.source().root_at(time),
        ).map_err(numeric)?;
        let world_time = self.world(time);
        Ok(Some(RootCrossing {
            toi: time, world_time, point,
            aperture: self.step.sample(self.end, world_time).map_err(numeric)?,
        }))
    }
    fn first(&self) -> Result<Option<RootCrossing>, RefusalReason> {
        let (f0, d0) = self.sample(0.0);
        if f0 == 0.0 && d0 < 0.0 { return self.hit(0.0); }
        if self.until == 0.0 { return Ok(None); }
        let source = self.cast.source();
        let source_arc = source.turn() != 0.0 && source.root_arm() != DVec2::ZERO;
        if self.omega == 0.0 && !source_arc {
            if d0 >= 0.0 || f0 < 0.0 { return Ok(None); }
            let time = -f0 / d0;
            return if (0.0..=self.until).contains(&time) { self.hit(time) } else { Ok(None) };
        }
        if self.b == DVec2::ZERO && source.turn() == 0.0 { return self.periodic(); }
        let mut intervals = vec![0.0..self.until];
        let mut work = 0;
        while let Some(interval) = intervals.pop() {
            work += 1;
            if work > MAX_INTERVALS { return Err(RefusalReason::WorkLimit); }
            if !self.reaches_opening(&interval) { continue; }
            let (middle, radius) = super::interval_center(&interval);
            let (f, derivative) = self.sample(middle);
            let (x, rate) = self.relative(middle);
            let span = self.cast.time.end - self.cast.time.start;
            let angle = super::rotation_bound(f64::from(self.step.turn), self.world(middle), span * radius);
            let error = rounding(x.length() + self.b.length() * radius
                + super::material_cast::rotation_delta(source.turn() * middle, source.root_arm()).length() + self.depth.abs());
            let source_angle = source.angle_bound(middle, radius);
            let x_variation = self.b.length() * radius + source_angle * source.root_arm().length();
            let variation = x_variation + angle * x.length() + error;
            if f - variation > 0.0 || f + variation < 0.0 { continue; }
            let derivative_variation = self.omega.abs() * (x_variation + angle * x.length())
                + source.turn().abs() * source_angle * source.root_arm().length()
                + angle * rate.length() + rounding(self.omega.abs() * x.length() + rate.length());
            if derivative - derivative_variation >= 0.0 { continue; }
            if derivative + derivative_variation < 0.0 {
                let (left, _) = self.sample(interval.start);
                let (right, _) = self.sample(interval.end);
                if left < 0.0 || right > 0.0 { continue; }
                if let Some(hit) = self.hit(self.bisect(interval))? { return Ok(Some(hit)); }
                continue;
            }
            if middle == interval.start || middle == interval.end {
                return Err(numeric(NumericFault::Precision));
            }
            intervals.push(middle..interval.end);
            intervals.push(interval.start..middle);
        }
        Ok(None)
    }
    /// With no relative translation, all directed crossings have the same
    /// pose and fit. Solve the first winding without enumerating later turns.
    fn periodic(&self) -> Result<Option<RootCrossing>, RefusalReason> {
        let n = self.normal(0.0);
        let c = n.dot(self.a);
        let s = perpendicular(n).dot(self.a);
        let radius = c.hypot(s);
        if self.depth.abs() >= radius { return Ok(None); }
        let phase = s.atan2(c);
        let crossing = phase + self.omega.signum() * (self.depth / radius).acos();
        let turns = -crossing / std::f64::consts::TAU;
        let k = if self.omega > 0.0 { turns.ceil() } else { turns.floor() };
        let time = (crossing + k * std::f64::consts::TAU) / self.omega;
        if !(0.0..=self.until).contains(&time) { return Ok(None); }
        self.hit(time)
    }
    fn bisect(&self, interval: std::ops::Range<f64>) -> f64 {
        let mut lo = interval.start;
        let mut hi = interval.end;
        loop {
            let mid = lo + (hi - lo) * 0.5;
            if mid == lo || mid == hi { return hi; }
            if self.sample(mid).0 > 0.0 { lo = mid; } else { hi = mid; }
        }
    }
    /// At a crossing the root must lie in this annulus about the pivot.
    fn reaches_opening(&self, time: &std::ops::Range<f64>) -> bool {
        let source = self.cast.source();
        if source.turn() != 0.0 && source.root_arm() != DVec2::ZERO {
            let (middle, half) = super::interval_center(time);
            let distance = self.relative(middle).0.length();
            let variation = self.b.length() * half + source.angle_bound(middle, half) * source.root_arm().length();
            let opening_half = f64::from(self.end.half_length);
            let inner = self.depth.hypot((self.along.abs() - opening_half).max(0.0));
            let outer = self.depth.hypot(self.along.abs() + opening_half);
            let error = rounding(distance + variation + inner + outer);
            return distance - variation <= outer + error && distance + variation >= inner - error;
        }
        let a = self.a + self.b * time.start;
        let b = self.a + self.b * time.end;
        let delta = b - a;
        let at = if delta.length_squared() == 0.0 { 0.0 }
            else { (-a.dot(delta) / delta.length_squared()).clamp(0.0, 1.0) };
        let min = (a + delta * at).length();
        let max = a.length().max(b.length());
        let half = f64::from(self.end.half_length);
        let along_min = (self.along.abs() - half).max(0.0);
        let along_max = self.along.abs() + half;
        let inner = self.depth.hypot(along_min);
        let outer = self.depth.hypot(along_max);
        let error = rounding(max + inner + outer);
        min <= outer + error && max >= inner - error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Vec2, frame::{PortalFrame, Sim2, thread::MaterialPrimitive}};

    fn rotating(turn: f32) -> (ApertureStep, PortalAperture) {
        let end = PortalAperture { frame: PortalFrame::fixed(Vec2::ZERO, Vec2::X), half_length: 20.0 };
        (ApertureStep::ending_at(end.frame, Vec2::ZERO, turn), end)
    }
    fn cast<'a>(shape: &'a CombatVolume, from: Vec2, delta: Vec2) -> MaterialCast<'a> {
        MaterialCast::new(
            MaterialPrimitive {
            domain: shape, to_world: Sim2::new(1.0, bevy_math::Mat2::IDENTITY, from).unwrap(),
        },
            from,
            delta,
            0.0..1.0,
        )
    }

    #[test]
    fn full_turns_cross_a_stationary_root_between_equal_endpoint_poses() {
        let circle = CombatVolume::circle(Vec2::ZERO, 1.0);
        for turns in [1.0, 2.0, 16_777_216.0, -1.0, -16_777_216.0] {
            let (step, end) = rotating(std::f32::consts::TAU * turns);
            let query = cast(&circle, Vec2::new(10.0, 0.0), Vec2::ZERO);
            let hit = query.crossing(step, end, 1.0).unwrap().unwrap();
            assert!(hit.toi > 0.0 && hit.toi < 1.0, "{turns}: {hit:?}");
            let n = step.sample_wide(end, hit.world_time).unwrap().normal;
            assert!(n.dot(hit.point.as_dvec2()).abs() < 1.0e-10);
            assert!(f64::from(step.turn) * perpendicular(n).dot(hit.point.as_dvec2()) < 0.0);
        }
    }

    #[test]
    fn circles_retain_their_finite_opening_fit() {
        let (step, mut end) = rotating(std::f32::consts::TAU * 16_777_216.0);
        end.half_length = 11.0;
        let circle = CombatVolume::circle(Vec2::ZERO, 1.0);
        let square = CombatVolume::aabb(crate::Aabb::new(Vec2::ZERO, Vec2::splat(1.0)));
        let from = Vec2::splat(10.0 / std::f32::consts::SQRT_2);
        assert!(cast(&circle, from, Vec2::ZERO).crossing(step, end, 1.0).unwrap().is_some());
        assert!(cast(&square, from, Vec2::ZERO).crossing(step, end, 1.0).unwrap().is_none());
    }

    #[test]
    fn a_large_world_origin_does_not_widen_the_opening() {
        let shape = CombatVolume::circle(Vec2::X * 2.0e-8, 1.0);
        let base = Vec2::X * 2.0_f32.powi(20);
        let reach = 31.0;
        let end = PortalAperture { frame: PortalFrame::fixed(base, Vec2::X), half_length: 32.0 };
        let step = ApertureStep::ending_at(end.frame, Vec2::ZERO, std::f32::consts::TAU);
        let query = cast(&shape, base + Vec2::X * reach, Vec2::ZERO);
        assert_eq!(query.crossing(step, end, 1.0), Ok(None));
    }

    #[test]
    fn translating_roots_use_the_curve_and_the_world_subinterval() {
        let shape = CombatVolume::circle(Vec2::ZERO, 0.5);
        let (step, end) = rotating(std::f32::consts::TAU);
        let query = cast(&shape, Vec2::new(10.0, 1.0), Vec2::new(-2.0, 3.0));
        let whole = query.crossing(step, end, 1.0).unwrap().unwrap();
        let mut partial = cast(&shape, query.root + query.delta * 0.125, query.delta * 0.5);
        partial.time = 0.125..0.625;
        let hit = partial.crossing(step, end, 1.0).unwrap().unwrap();
        assert!((hit.world_time - whole.world_time).abs() < 1.0e-14);
        assert!(hit.point.distance(whole.point) < 1.0e-5);
        let boost = Vec2::new(32.0, -16.0);
        let mut moving = end;
        moving.frame.origin += boost;
        moving.frame.pivot += boost;
        let carried = ApertureStep::ending_at(moving.frame, boost, step.turn);
        let boosted = cast(&shape, query.root, query.delta + boost).crossing(carried, moving, 1.0).unwrap().unwrap();
        assert!((boosted.toi - whole.toi).abs() < 1.0e-14);
    }

    #[test]
    fn unresolved_curves_return_a_typed_refusal_or_a_certified_miss() {
        let shape = CombatVolume::circle(Vec2::ZERO, 1.0);
        let (step, end) = rotating(std::f32::consts::TAU * 16_777_216.0);
        let mut late = cast(&shape, Vec2::X * 10.0, Vec2::ZERO);
        late.time = 0.75..1.0;
        assert_eq!(late.crossing(step, end, 1.0), Err(numeric(NumericFault::Precision)));
        let oversized = CombatVolume::circle(Vec2::ZERO, 40.0);
        assert!(matches!(cast(&oversized, Vec2::X * 10.0, Vec2::Y * 0.001).crossing(step, end, 1.0),
            Ok(None) | Err(RefusalReason::WorkLimit)));
    }

    #[test]
    fn motion_intervals_keep_their_bits_in_a_snapshot() {
        use crate::snapshot::{encode_state, decode_state};
        let sample = crate::SweepSample { time: [-0.0, 0.625],
            prev: Vec2::X, curr: Vec2::Y, ..Default::default() };
        let bytes = encode_state(&sample);
        let restored = decode_state::<crate::SweepSample>(&bytes).unwrap();
        assert_eq!(encode_state(&restored), bytes);
        let mut invalid = bytes;
        invalid[..8].copy_from_slice(&f64::NAN.to_bits().to_le_bytes());
        assert!(decode_state::<crate::SweepSample>(&invalid).is_none());
    }

    #[test]
    fn a_closed_prefix_keeps_the_original_crossing_time_and_snapshot() {
        use crate::snapshot::{encode_state, decode_state};
        let shape = CombatVolume::circle(Vec2::ZERO, 0.5);
        let (step, end) = rotating(std::f32::consts::TAU);
        let query = cast(&shape, Vec2::new(10.0, 1.0), Vec2::new(-2.0, 3.0));
        let hit = query.crossing(step, end, 1.0).unwrap().unwrap();
        let mut sample = crate::SweepSample {
            prev: query.root, curr: hit.point,
            travel: crate::LinearTravel::Prefix { delta: query.delta, until: hit.toi },
            ..Default::default()
        };
        let restored = decode_state::<crate::SweepSample>(&encode_state(&sample)).unwrap();
        assert_eq!(encode_state(&restored), encode_state(&sample));
        assert!(restored.ending_at(hit.point).is_some());
        assert_eq!(query.crossing(step, end, restored.travel.line(restored.prev, restored.curr).unwrap().1).unwrap().unwrap(), hit);
        assert!(query.crossing(step, end, hit.toi * 0.5).unwrap().is_none());
        sample.curr.x += 0.25;
        assert!(sample.ending_at(sample.curr).is_none());
    }

    #[test]
    fn a_tangent_and_an_outward_root_are_not_entries() {
        let shape = CombatVolume::circle(Vec2::ZERO, 0.5);
        let (step, end) = rotating(std::f32::consts::TAU);
        assert!(cast(&shape, Vec2::ZERO, Vec2::ZERO).crossing(step, end, 1.0).unwrap().is_none());
        let fixed = ApertureStep::fixed(Vec2::ZERO, Vec2::X);
        assert!(cast(&shape, Vec2::ZERO, Vec2::X).crossing(fixed, end, 1.0).unwrap().is_none());
        assert_eq!(cast(&shape, Vec2::ZERO, -Vec2::X).crossing(fixed, end, 1.0).unwrap().unwrap().toi, 0.0);
    }
}
