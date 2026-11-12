//! Contact between two admitted material paths on the same world clock.

use std::ops::Range;

use super::{CastHit, CombatVolume, DVec2, MaterialCastCells, MaterialCell, NumericFault,
    SampledPieceHit, Sim2, clip, curved, sampled, translation};
use crate::frame::thread::{Refusal, RefusalReason};

struct Pair<'a> {
    source: &'a MaterialCell,
    target: &'a MaterialCell,
    polygon: &'a [DVec2],
    domain: &'a CombatVolume,
    /// Target parameter = offset + rate * source parameter.
    offset: f64,
    rate: f64,
}

impl Pair<'_> {
    fn target_time(&self, time: f64) -> f64 { self.offset + self.rate * time }

    fn region(&self, time: f64) -> Result<Vec<DVec2>, NumericFault> {
        let Some(source_cuts) = self.source.planes_at(time)? else { return Ok(vec![]); };
        let target_time = self.target_time(time);
        let Some(target_cuts) = self.target.planes_at(target_time)? else { return Ok(vec![]); };
        let mut region = self.polygon.to_vec();
        for cut in source_cuts { region = clip(&region, cut.plane); }
        let map = self.source.at(time)?.then(self.target.at(target_time)?.inverse()?)?;
        for point in &mut region { *point = map.point_wide(*point); }
        for cut in target_cuts { region = clip(&region, cut.plane); }
        Ok(region)
    }

    fn at(&self, time: f64) -> Result<Option<CastHit>, NumericFault> {
        let region = self.region(time)?;
        if region.is_empty() { return Ok(None); }
        Ok(translation::cast(self.domain, DVec2::ZERO, &region, DVec2::ZERO,
            std::iter::empty(), super::InitialOverlap::Report)?
            .map(|hit| CastHit { time, ..hit }))
    }

    fn possible(&self, interval: &Range<f64>) -> Result<bool, NumericFault> {
        let Some(source_cuts) = self.source.relaxed_planes(interval)? else { return Ok(false); };
        let target_interval = self.target_time(interval.start)..self.target_time(interval.end);
        let Some(target_cuts) = self.target.relaxed_planes(&target_interval)? else { return Ok(false); };
        let mut region = self.polygon.to_vec();
        for cut in source_cuts { region = clip(&region, cut); }
        if region.is_empty() { return Ok(false); }
        let (middle, half) = crate::frame::interval_center(interval);
        let target_time = self.target_time(middle);
        let inverse = self.target.at(target_time)?.inverse()?;
        let map = self.source.at(middle)?.then(inverse)?;
        for point in &mut region { *point = map.point_wide(*point); }
        // Each clipped set is within its full material's motion envelope.
        // Use both envelopes before a separating plane can reject the pair.
        let error = (self.source.variation(middle, half)?
            + self.target.variation(target_time, half * self.rate)?) * inverse.scale_wide()
            + self.rounding(&region, map)*0.5;
        if !error.is_finite() || region.iter().any(|point| !point.is_finite()) {
            return Err(NumericFault::Overflow);
        }
        let mut target_region = curved::domain_polygon(self.domain);
        for cut in target_cuts { target_region = clip(&target_region, cut); }
        for cut in translation::boundary(&region) {
            target_region = clip(&target_region, super::MaterialPlane {
                offset: cut.offset - error*cut.inward.length(), ..cut
            });
            if target_region.is_empty() { return Ok(false); }
        }
        Ok(curved::intersection(self.domain, &target_region).is_some())
    }

    fn rounding(&self, region: &[DVec2], map: Sim2) -> f64 {
        let extent = region.iter().map(|point| point.length()).fold(0.0_f64, f64::max);
        (extent + map.translation().length()) * f64::EPSILON * 256.0
            * (self.source.path.len() + self.target.path.len() + 1) as f64
    }

    fn cast(&self, interval: Range<f64>, work: &mut usize) -> Result<Option<CastHit>, RefusalReason> {
        let numeric = RefusalReason::Numeric;
        if self.source.curve.is_none() && self.target.curve.is_none() {
            let inverse = self.target.at(self.offset).and_then(Sim2::inverse).map_err(numeric)?;
            let map = self.source.map.then(inverse).map_err(numeric)?;
            let motion = inverse.direction_wide(self.source.travel - self.target.travel*self.rate)
                * inverse.scale_wide();
            let region: Vec<_> = self.polygon.iter().map(|point|
                map.point_wide(*point) + motion*interval.start).collect();
            let mut planes = self.target.planes.iter().map(|plane| translation::MovingPlane {
                plane: super::MaterialPlane { offset: plane.plane.offset + plane.rate*self.offset, ..plane.plane },
                rate: plane.rate*self.rate,
            }).collect::<Vec<_>>();
            for plane in &self.source.planes {
                planes.push(super::carry_plane(*plane, map, motion).map_err(numeric)?);
            }
            return Ok(translation::cast(self.domain, DVec2::ZERO, &region,
                motion*(interval.end-interval.start), planes.into_iter().map(|plane| plane.during(interval.clone())),
                super::InitialOverlap::Report).map_err(numeric)?.map(|hit| CastHit {
                    time: interval.start + (interval.end-interval.start)*hit.time, ..hit
                }));
        }
        let mut stack = vec![interval];
        while let Some(interval) = stack.pop() {
            *work += 1;
            if *work > curved::MAX_INTERVALS { return Err(RefusalReason::WorkLimit); }
            if let Some(hit) = self.at(interval.start).map_err(numeric)? { return Ok(Some(hit)); }
            if !self.possible(&interval).map_err(numeric)? { continue; }
            let (middle, half) = crate::frame::interval_center(&interval);
            if middle == interval.start || middle == interval.end {
                if let Some(hit) = self.at(interval.end).map_err(numeric)? { return Ok(Some(hit)); }
                let region = self.region(middle).map_err(numeric)?;
                let map = self.source.at(middle).and_then(|source|
                    source.then(self.target.at(self.target_time(middle))?.inverse()?)).map_err(numeric)?;
                let inverse = self.target.at(self.target_time(middle)).and_then(Sim2::inverse).map_err(numeric)?;
                let tolerance = self.rounding(&region, map) + (self.source.variation(middle, half)
                    .map_err(numeric)? + self.target.variation(self.target_time(middle), half*self.rate)
                    .map_err(numeric)?) * inverse.scale_wide();
                if let Some(hit) = curved::static_contact(self.domain, &region, tolerance) {
                    return Ok(Some(CastHit { time: middle, ..hit }));
                }
                return Err(numeric(NumericFault::Precision));
            }
            stack.push(middle..interval.end);
            stack.push(interval.start..middle);
        }
        Ok(None)
    }
}

impl MaterialCastCells<'_> {
    /// Sweep against another admitted path. Both clocks are world-tick clocks.
    /// Preserve analytic target circles and each body's conditional portal cuts.
    pub fn against_moving(&self, target: &MaterialCastCells<'_>) -> Result<Option<SampledPieceHit>, Refusal> {
        let refusal = |reason| Refusal { path: vec![], link: None, reason };
        let numeric = |fault| refusal(RefusalReason::Numeric(fault));
        let polygon = sampled::polygon_points(self.material.domain).map_err(refusal)?;
        let source_span = self.time.end - self.time.start;
        let target_span = target.time.end - target.time.start;
        // An instant has no travel outside its recorded world time.
        let (offset, rate) = if target_span == 0.0 {
            (0.0, 0.0)
        } else { ((self.time.start - target.time.start) / target_span, source_span / target_span) };
        let mut first: Option<SampledPieceHit> = None;
        let mut work = 0;
        for (source, source_bounds) in self.bounded_parts() {
            let Some(source_bounds) = source_bounds else { continue; };
            for (recipient, target_bounds) in target.bounded_parts() {
                let Some(target_bounds) = target_bounds else { continue; };
                if (0..2).any(|axis| source_bounds.max[axis] < target_bounds.min[axis].next_down()
                    || target_bounds.max[axis] < source_bounds.min[axis].next_down()) { continue; }
                let mut interval = source.time.clone();
                if source_span == 0.0 {
                    let world_time = self.time.start;
                    let target_time = if target_span == 0.0 { 0.0 } else { (world_time-target.time.start)/target_span };
                    if world_time < target.time.start || world_time > target.time.end
                        || target_time < recipient.time.start || target_time > recipient.time.end { continue; }
                } else if target_span == 0.0 {
                    let time = (target.time.start-self.time.start)/source_span;
                    interval.start = interval.start.max(time);
                    interval.end = interval.end.min(time);
                } else {
                    interval.start = interval.start.max((recipient.time.start-offset)/rate);
                    interval.end = interval.end.min((recipient.time.end-offset)/rate);
                }
                if interval.start > interval.end { continue; }
                let pair = Pair { source, target: recipient, polygon: &polygon,
                    domain: target.material.domain, offset, rate };
                let Some(hit) = pair.cast(interval, &mut work).map_err(|reason| {
                    let mut path = recipient.path.clone();
                    let link = path.pop();
                    Refusal { path, link, reason }
                })? else { continue; };
                let time = hit.time;
                if source.time.start < source.time.end && source.time.end < self.until
                    && time >= source.time.end-f64::EPSILON*64.0 { continue; }
                let target_time = pair.target_time(time);
                if recipient.time.start < recipient.time.end && recipient.time.end < target.until
                    && target_time >= recipient.time.end-f64::EPSILON*64.0 { continue; }
                let target_material = recipient.at(target_time).map_err(numeric)?;
                let source_material = source.at(time).map_err(numeric)?;
                let region = pair.region(time).map_err(numeric)?;
                let point = curved::intersection(target.material.domain, &region).unwrap_or(hit.point);
                let physical_point = target_material.point_from_wide(point).map_err(numeric)?;
                let contact = SampledPieceHit {
                    toi: time, world_time: self.time.start + source_span*time,
                    point: physical_point,
                    normal: -target_material.direction(hit.normal.as_vec2()).normalize_or_zero(),
                    source_path: source.path.clone(), target_path: recipient.path.clone(),
                    target_map: target.source.map_at(target_time).and_then(|map|
                        map.inverse()?.then(target_material)).map_err(numeric)?,
                    source_map: source_material,
                    source_point: source_material.inverse().map_err(numeric)?.point_wide(target_material.point_wide(point)),
                };
                if first.as_ref().is_none_or(|old| contact.toi.total_cmp(&old.toi)
                    .then(contact.source_path.cmp(&old.source_path))
                    .then(contact.target_path.cmp(&old.target_path)).is_lt()) { first = Some(contact); }
            }
        }
        Ok(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Aabb, Vec2};
    use crate::frame::{MapConvention, PortalAperture, PortalFrame};
    use crate::frame::thread::{MaterialPrimitive, MotionAdmission, PieceBudget, PortalLink};
    use super::super::MaterialCast;

    #[test]
    fn a_moving_circle_hits_a_stationary_shot_on_the_shared_clock() {
        let source = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::ONE));
        let target = CombatVolume::circle(Vec2::ZERO, 1.0);
        for start in [0.0, 0.25] {
            let source = MaterialCast::new(MaterialPrimitive::world(&source), Vec2::ZERO,
                Vec2::ZERO, start..1.0).cells(None).unwrap();
            let target = MaterialCast::new(MaterialPrimitive { domain: &target,
                to_world: Sim2::IDENTITY.shifted(DVec2::new(0.0, -10.0)).unwrap() },
                Vec2::new(0.0, -10.0), Vec2::Y*20.0, 0.0..1.0).cells(None).unwrap();
            let hit = source.against_moving(&target).unwrap().unwrap();
            assert!((hit.world_time-0.4).abs() < 1.0e-10, "{hit:?}");
            assert!((hit.toi-(0.4-start)/(1.0-start)).abs() < 1.0e-10, "{hit:?}");
            assert!(hit.normal.dot(Vec2::Y) > 0.99, "{hit:?}");
        }
    }

    #[test]
    fn a_moving_target_keeps_its_scaled_reflected_portal_cut() {
        let entry = PortalAperture { frame: PortalFrame::fixed(Vec2::ZERO, -Vec2::Y), half_length: 40.0 };
        let exit = PortalAperture { frame: PortalFrame::fixed(Vec2::new(500.0, -200.0), Vec2::Y), half_length: 20.0 };
        let links = [PortalLink::new(entry, exit, 0), PortalLink::new(exit, entry, 1)];
        let target = CombatVolume::circle(Vec2::ZERO, 2.0);
        let target = MaterialCast::new(MaterialPrimitive { domain: &target,
            to_world: Sim2::IDENTITY.shifted(DVec2::new(0.0, -10.0)).unwrap() },
            Vec2::new(0.0, -10.0), Vec2::Y*20.0, 0.0..1.0).cells(Some(MotionAdmission {
                links: &links, first: Some(0), budget: PieceBudget::COMPOSED,
                convention: MapConvention::Reflection,
            })).unwrap();
        let source = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::splat(0.25)));
        let shot = |point: Vec2| MaterialCast::new(MaterialPrimitive { domain: &source,
            to_world: Sim2::IDENTITY.shifted(point.as_dvec2()).unwrap() }, point,
            Vec2::ZERO, 0.0..1.0).cells(None).unwrap();
        let hit = shot(Vec2::new(500.0, -197.0)).against_moving(&target).unwrap().unwrap();
        assert_eq!(hit.target_path, vec![0]);
        assert_eq!(hit.target_map.scale(), 0.5);
        assert!(hit.target_map.reflected());
        assert!((hit.world_time-0.675).abs() < 1.0e-9, "{hit:?}");
        assert!(shot(Vec2::new(0.0, 6.0)).against_moving(&target).unwrap().is_none(),
            "the cut-away continuation cannot hit in the entry chart");
    }

    #[test]
    fn a_target_full_turn_hits_between_matching_endpoint_poses() {
        let source = CombatVolume::aabb(Aabb::new(Vec2::new(0.0, 5.0), Vec2::splat(0.2)));
        let source = MaterialCast::new(MaterialPrimitive::world(&source), Vec2::new(0.0, 5.0),
            Vec2::ZERO, 0.0..1.0).cells(None).unwrap();
        let target = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::new(6.0, 0.2)));
        let make = || MaterialCast::new(MaterialPrimitive::world(&target), Vec2::ZERO,
            Vec2::ZERO, 0.0..1.0);
        assert!(source.against_moving(&make().cells(None).unwrap()).unwrap().is_none());
        let target = make().rotating_about(Vec2::ZERO, std::f64::consts::TAU, 1.0).unwrap().cells(None).unwrap();
        let hit = source.against_moving(&target).unwrap().unwrap();
        assert!(hit.world_time > 0.2 && hit.world_time < 0.25, "{hit:?}");
    }
}
