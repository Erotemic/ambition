//! A sampled target against curved source maps and conditional cuts.

use super::super::curved::{domain_polygon, MAX_INTERVALS};
use super::super::{translation, CastHit, DVec2, MaterialCell, MaterialPlane, NumericFault};
use crate::frame::thread::{RefusalReason, VolumePiece};
use crate::frame::Sim2;

pub(super) fn region(
    part: &MaterialCell,
    polygon: &[DVec2],
    time: f64,
    target: &VolumePiece,
    inverse: Sim2,
) -> Result<Vec<DVec2>, NumericFault> {
    let Some(cuts) = part.planes_at(time)? else {
        return Ok(vec![]);
    };
    let mut region = polygon.to_vec();
    for cut in cuts {
        region = super::super::clip(&region, cut.plane);
    }
    let map = part.at(time)?.then(inverse)?;
    for point in &mut region {
        *point = map.point_wide(*point);
    }
    for cut in target.source_bounds() {
        region = super::super::clip(&region, *cut);
    }
    Ok(region)
}

fn possible(
    part: &MaterialCell,
    polygon: &[DVec2],
    interval: &std::ops::Range<f64>,
    target: &VolumePiece,
    inverse: Sim2,
) -> Result<bool, NumericFault> {
    let Some(cuts) = part.relaxed_planes(interval)? else {
        return Ok(false);
    };
    let mut region = polygon.to_vec();
    for cut in cuts {
        region = super::super::clip(&region, cut);
    }
    if region.is_empty() {
        return Ok(false);
    }
    let (middle, half) = crate::frame::interval_center(interval);
    let map = part.at(middle)?.then(inverse)?;
    for point in &mut region {
        *point = map.point_wide(*point);
    }
    let extent = region
        .iter()
        .map(|point| point.length())
        .fold(0.0_f64, f64::max);
    let error = part.variation(middle, half)? * inverse.scale_wide()
        + (extent + map.translation().length())
            * f64::EPSILON
            * 128.0
            * (part.path.len() + 1) as f64;
    if !error.is_finite() || region.iter().any(|point| !point.is_finite()) {
        return Err(NumericFault::Overflow);
    }
    let mut target_region = domain_polygon(target.source());
    for cut in target.source_bounds() {
        target_region = super::super::clip(&target_region, *cut);
    }
    for cut in translation::boundary(&region) {
        target_region = super::super::clip(
            &target_region,
            MaterialPlane {
                offset: cut.offset - error,
                ..cut
            },
        );
        if target_region.is_empty() {
            return Ok(false);
        }
    }
    Ok(super::super::curved::intersection(target.source(), &target_region).is_some())
}

pub(super) fn cast(
    part: &MaterialCell,
    polygon: &[DVec2],
    target: &VolumePiece,
) -> Result<Option<CastHit>, RefusalReason> {
    let numeric = RefusalReason::Numeric;
    let inverse = target.transform().inverse().map_err(numeric)?;
    let at = |time| -> Result<Option<CastHit>, NumericFault> {
        let region = region(part, polygon, time, target, inverse)?;
        if region.is_empty() {
            return Ok(None);
        }
        let hit = translation::cast(
            target.source(),
            DVec2::ZERO,
            &region,
            DVec2::ZERO,
            std::iter::empty(),
            super::super::InitialOverlap::Report,
        )?;
        Ok(hit.map(|hit| CastHit { time, ..hit }))
    };
    let mut stack = vec![part.time.clone()];
    let mut work = 0;
    while let Some(interval) = stack.pop() {
        work += 1;
        if work > MAX_INTERVALS {
            return Err(RefusalReason::WorkLimit);
        }
        if let Some(hit) = at(interval.start).map_err(numeric)? {
            return Ok(Some(hit));
        }
        if !possible(part, polygon, &interval, target, inverse).map_err(numeric)? {
            continue;
        }
        let middle = interval.start + (interval.end - interval.start) * 0.5;
        if middle == interval.start || middle == interval.end {
            if let Some(hit) = at(interval.end).map_err(numeric)? {
                return Ok(Some(hit));
            }
            let region = region(part, polygon, middle, target, inverse).map_err(numeric)?;
            let map = part
                .at(middle)
                .and_then(|map| map.then(inverse))
                .map_err(numeric)?;
            let extent = region
                .iter()
                .map(|point| point.length())
                .fold(0.0_f64, f64::max);
            let rounding = (extent + map.translation().length())
                * f64::EPSILON
                * 256.0
                * (part.path.len() + 1) as f64;
            if let Some(hit) =
                super::super::curved::static_contact(target.source(), &region, rounding)
            {
                return Ok(Some(CastHit {
                    time: middle,
                    ..hit
                }));
            }
            return Err(numeric(NumericFault::Precision));
        }
        stack.push(middle..interval.end);
        stack.push(interval.start..middle);
    }
    Ok(None)
}
