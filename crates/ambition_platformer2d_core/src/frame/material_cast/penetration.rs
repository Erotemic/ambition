//! Static contact of polygon material with a clipped obstacle.

use super::*;

pub(crate) struct PolygonAxisContact {
    pub displacement: f64,
    pub minimum_normal: Vec2,
    pub contact: MaterialCastHit,
}

impl<'a> MaterialCastCells<'a> {
    /// Check the occupied material against a physical support half-plane.
    pub(crate) fn in_front_of(
        &self, part: &MaterialCell, time: f64, point: DVec2, outward: DVec2,
    ) -> Result<bool, Refusal> {
        let polygon = self.source_polygon().ok_or_else(|| numeric_at(part, NumericFault::InvalidFrame))?;
        let mut material = std::borrow::Cow::Borrowed(polygon);
        for plane in part.planes_at(time).map_err(|fault| numeric_at(part, fault))?
            .ok_or_else(|| numeric_at(part, NumericFault::InvalidFrame))? {
            material = std::borrow::Cow::Owned(clip(&material, plane.plane));
        }
        let inverse = part.at(time).and_then(|map| map.inverse()).map_err(|fault| numeric_at(part, fault))?;
        let origin = inverse.point_wide(point);
        let normal = inverse.direction_wide(outward);
        Ok(!material.is_empty() && material.iter().all(|point| {
            let error = f64::EPSILON * 64.0 * normal.abs().dot(point.abs() + origin.abs());
            (*point - origin).dot(normal) >= -error
        }))
    }

    pub(crate) fn polygon_axis_contacts(
        &self, part: &MaterialCell, obstacle: &[Vec2], direction: Vec2,
    ) -> Result<Vec<PolygonAxisContact>, Refusal> {
        let mut contacts = Vec::new();
        for scope in self.obstacle_scopes(part, obstacle, Vec2::ZERO, 0.0, Vec2::ZERO)?.iter() {
            if let Some(contact) = self.polygon_axis_contact(scope, obstacle, direction)? {
                contacts.push(contact);
            }
        }
        Ok(contacts)
    }

    pub(crate) fn polygon_axis_probe<'c>(&'c self, part: &'c MaterialCell, direction: Vec2) -> PolygonAxisProbe<'c, 'a> {
        PolygonAxisProbe { cells: self, part, direction, prepared: std::cell::OnceCell::new() }
    }

    pub(crate) fn polygon_axis_contact(
        &self, part: &MaterialCell, obstacle: &[Vec2], direction: Vec2,
    ) -> Result<Option<PolygonAxisContact>, Refusal> {
        self.polygon_axis_probe(part, direction).contact(obstacle)
    }
}

/// Static candidates share the cell's maps and clipped source material.
pub(crate) struct PolygonAxisProbe<'c, 'a> {
    cells: &'c MaterialCastCells<'a>,
    part: &'c MaterialCell,
    direction: Vec2,
    prepared: std::cell::OnceCell<Result<StaticMaterial<'c>, Refusal>>,
}

struct StaticMaterial<'c> {
    body: &'c [DVec2],
    root: Sim2,
    inverse: Sim2,
    direction: DVec2,
    planes: Vec<translation::PlaneMotion>,
    has_area: bool,
}

impl<'c> PolygonAxisProbe<'c, '_> {
    fn prepare(&self) -> Result<StaticMaterial<'c>, Refusal> {
        let part = self.part;
        let body = self.cells.source_polygon().ok_or_else(|| numeric_at(part, NumericFault::InvalidFrame))?;
        let root = self.cells.source.map_at(1.0).map_err(|fault| numeric_at(part, fault))?;
        let inverse = root.inverse().map_err(|fault| numeric_at(part, fault))?;
        let direction = inverse.direction_wide(self.direction.as_dvec2()) * inverse.scale_wide();
        let inverse = part.at(1.0).and_then(|pose| pose.inverse()).map_err(|fault| numeric_at(part, fault))?;
        let planes = part.planes_at(1.0).map_err(|fault| numeric_at(part, fault))?
            .ok_or_else(|| numeric_at(part, NumericFault::InvalidFrame))?;
        let mut material = std::borrow::Cow::Borrowed(body);
        for plane in &planes { material = std::borrow::Cow::Owned(clip(&material, plane.plane)); }
        let has_area = crate::frame::thread::has_area(&material);
        Ok(StaticMaterial { body, root, inverse, direction, planes, has_area })
    }

    /// Find the nearest free position along a root-chart axis at tick end.
    pub(crate) fn contact(&self, obstacle: &[Vec2]) -> Result<Option<PolygonAxisContact>, Refusal> {
        if self.cells.time.start != self.cells.time.end || self.cells.source.delta != DVec2::ZERO || self.cells.source.rotation.is_some() {
            return Err(numeric_at(self.part, NumericFault::InvalidFrame));
        }
        if obstacle.len() >= 3 && !self.cells.collider_may_occupy(self.part, obstacle, Vec2::ZERO) { return Ok(None); }
        let prepared = self.prepared.get_or_init(|| self.prepare()).as_ref().map_err(Clone::clone)?;
        let StaticMaterial { body, root, inverse, direction, planes, has_area } = prepared;
        let projected = PolygonPoints::project(obstacle, |point| inverse.point_wide(point.as_dvec2()));
        let mut region = std::borrow::Cow::Borrowed(projected.as_slice());
        if region.iter().any(|point| !point.is_finite()) { return Err(numeric_at(self.part, NumericFault::NonFinite)); }
        for plane in planes { region = std::borrow::Cow::Owned(clip(&region, plane.plane)); }
        // Boundary strata carry crossing events, not static support.
        if !has_area { return Ok(None); }
        if region.is_empty() || obstacle.len() >= 3 && !crate::frame::thread::has_area(&region) { return Ok(None); }
        let mut lower = (f64::NEG_INFINITY, DVec2::ZERO);
        let mut upper = (f64::INFINITY, DVec2::ZERO);
        let mut minimum = (f64::INFINITY, DVec2::ZERO);
        let mut cardinal_axes = 0;
        for (a, b) in edges(body).chain(edges(&region)) {
            let edge = b - a;
            if edge == DVec2::ZERO { continue; }
            let n = DVec2::new(-edge.y, edge.x).normalize();
            // Parallel cardinal edges share a separation test. Keep the first
            // axis so equal penetration depths retain their contact order.
            let cardinal = if n.x == 0.0 && n.y.abs() == 1.0 { 1 }
                else if n.y == 0.0 && n.x.abs() == 1.0 { 2 } else { 0 };
            if cardinal_axes & cardinal != 0 { continue; }
            cardinal_axes |= cardinal;
            let span = |points: &[DVec2]| points.iter().map(|point| point.dot(n))
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), value| (lo.min(value), hi.max(value)));
            let ((alo, ahi), (blo, bhi)) = (span(body), span(&region));
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
        if !displacement.is_finite() || normal == DVec2::ZERO { return Err(numeric_at(self.part, NumericFault::InvalidFrame)); }
        let moved = PolygonPoints::project(body, |point| point + *direction * displacement);
        let point = closest_polygons(moved.as_slice(), &region);
        let Some(contact) = self.cells.contact(self.part, CastHit { time: 1.0, normal, point })? else { return Ok(None); };
        Ok(Some(PolygonAxisContact { displacement, contact,
            minimum_normal: root.direction(minimum.1.as_vec2()).normalize_or_zero() }))
    }
}
