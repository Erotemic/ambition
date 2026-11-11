//! Static contact of polygon material with a clipped obstacle.

use super::*;

pub(crate) struct PolygonAxisContact {
    pub displacement: f64,
    pub minimum_normal: Vec2,
    pub contact: MaterialCastHit,
}

impl MaterialCastCells<'_> {
    /// Find the nearest free position along a root-chart axis at tick end.
    pub(crate) fn polygon_axis_contact(
        &self, part: &MaterialCell, obstacle: &[Vec2], direction: Vec2,
    ) -> Result<Option<PolygonAxisContact>, Refusal> {
        if self.time.start != self.time.end || self.source.delta != DVec2::ZERO || self.source.rotation.is_some() {
            return Err(numeric_at(part, NumericFault::InvalidFrame));
        }
        if obstacle.len() >= 3 && !self.solid_may_occupy(part, obstacle, Vec2::ZERO) { return Ok(None); }
        let body = self.material.domain.polygon().ok_or_else(|| numeric_at(part, NumericFault::InvalidFrame))?;
        let body: Vec<_> = body.iter().map(|point| point.as_dvec2()).collect();
        let root = self.source.map_at(1.0).map_err(|fault| numeric_at(part, fault))?;
        let inverse = root.inverse().map_err(|fault| numeric_at(part, fault))?;
        let direction = inverse.direction_wide(direction.as_dvec2()) * inverse.scale_wide();
        let pose = part.at(1.0).map_err(|fault| numeric_at(part, fault))?;
        let inverse = pose.inverse().map_err(|fault| numeric_at(part, fault))?;
        let mut region: Vec<_> = obstacle.iter().map(|point| inverse.point_wide(point.as_dvec2())).collect();
        if region.iter().any(|point| !point.is_finite()) { return Err(numeric_at(part, NumericFault::NonFinite)); }
        let planes = part.planes_at(1.0).map_err(|fault| numeric_at(part, fault))?
            .ok_or_else(|| numeric_at(part, NumericFault::InvalidFrame))?;
        let mut material = body.clone();
        for plane in planes {
            region = clip(&region, plane.plane);
            material = clip(&material, plane.plane);
        }
        // Boundary strata carry crossing events, not static support.
        if !crate::frame::thread::has_area(&material) { return Ok(None); }
        if region.is_empty() || obstacle.len() >= 3 && !crate::frame::thread::has_area(&region) { return Ok(None); }
        let mut lower = (f64::NEG_INFINITY, DVec2::ZERO);
        let mut upper = (f64::INFINITY, DVec2::ZERO);
        let mut minimum = (f64::INFINITY, DVec2::ZERO);
        for (a, b) in edges(&body).chain(edges(&region)) {
            let edge = b - a;
            if edge == DVec2::ZERO { continue; }
            let n = DVec2::new(-edge.y, edge.x).normalize();
            let span = |points: &[DVec2]| points.iter().map(|point| point.dot(n))
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), value| (lo.min(value), hi.max(value)));
            let ((alo, ahi), (blo, bhi)) = (span(&body), span(&region));
            let forward = bhi - alo;
            let backward = ahi - blo;
            let nearest = if forward < backward { (forward, n) } else { (backward, -n) };
            if nearest.0 < minimum.0 { minimum = nearest; }
            let speed = direction.dot(n);
            if speed == 0.0 {
                if ahi <= blo || bhi <= alo { return Ok(None); }
                continue;
            }
            let (a, b) = ((blo - ahi) / speed, (bhi - alo) / speed);
            let (lo, hi, normal) = if speed > 0.0 { (a, b, -n) } else { (b, a, n) };
            if lo > lower.0 { lower = (lo, normal); }
            if hi < upper.0 { upper = (hi, -normal); }
            if lower.0 > upper.0 { return Ok(None); }
        }
        if lower.0 > 0.0 || upper.0 < 0.0 { return Ok(None); }
        let (displacement, normal) = if upper.0.abs() < lower.0.abs() { upper } else { lower };
        if !displacement.is_finite() || normal == DVec2::ZERO { return Err(numeric_at(part, NumericFault::InvalidFrame)); }
        let moved: Vec<_> = body.iter().map(|point| *point + direction * displacement).collect();
        let point = closest_polygons(&moved, &region);
        let Some(contact) = self.contact(part, CastHit { time: 1.0, normal, point })? else { return Ok(None); };
        Ok(Some(PolygonAxisContact { displacement, contact,
            minimum_normal: root.direction(minimum.1.as_vec2()).normalize_or_zero() }))
    }
}
