//! Continuous contact in the spatial cells of a material partition.

use bevy_math::DVec2;

use super::thread::{
    partition, MaterialPlane, MaterialPrimitive, MotionAdmission, Refusal, RefusalReason,
    VolumePiece,
};
use super::{NumericFault, Sim2};
use crate::{Aabb, CombatVolume, Vec2};

mod translation;

/// A primitive's translation in its owner's world chart.
pub struct MaterialCast<'a> {
    pub material: MaterialPrimitive<'a>,
    pub root: Vec2,
    pub delta: Vec2,
    /// The attempted motion's interval within the world's current tick.
    pub time: std::ops::Range<f64>,
}

/// One contact, with both the owner-chart and physical-world witnesses.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialCastHit {
    pub toi: f64,
    pub normal: Vec2,
    pub point: Vec2,
    pub world_point: Vec2,
    pub path: Vec<u32>,
    pub map: Sim2,
}

/// The spatial cells stay fixed while the actual primitive moves through them.
pub struct MaterialCastCells<'a> {
    material: MaterialPrimitive<'a>,
    delta: DVec2,
    parts: Vec<VolumePiece>,
    time: std::ops::Range<f64>,
}

fn numeric(reason: NumericFault) -> Refusal {
    Refusal {
        path: Vec::new(),
        link: None,
        reason: RefusalReason::Numeric(reason),
    }
}

fn numeric_at(part: &VolumePiece, reason: NumericFault) -> Refusal {
    let mut path = part.path.clone();
    let link = path.pop();
    Refusal {
        path,
        link,
        reason: RefusalReason::Numeric(reason),
    }
}

impl<'a> MaterialCast<'a> {
    pub fn cells(
        self,
        admission: Option<MotionAdmission<'_>>,
    ) -> Result<MaterialCastCells<'a>, Refusal> {
        if !self.time.start.is_finite() || !self.time.end.is_finite() {
            return Err(numeric(NumericFault::NonFinite));
        }
        if self.time.start<0.0 || self.time.end>1.0 || self.time.start>self.time.end {
            return Err(numeric(NumericFault::InvalidFrame));
        }
        super::thread::map_volume(self.material.domain, &self.material.to_world)
            .map_err(numeric)?;
        let inverse = self.material.to_world.inverse().map_err(numeric)?;
        let delta = inverse.vector(self.delta).map_err(numeric)?;
        let bounds = self.material.domain.bounds();
        if !bounds.min.is_finite() || !bounds.max.is_finite() {
            return Err(numeric(NumericFault::NonFinite));
        }
        let mut points = corners(bounds)
            .into_iter()
            .flat_map(|p| [p, p + delta])
            .collect::<Vec<_>>();
        if points.iter().any(|p| !p.is_finite()) {
            return Err(numeric(NumericFault::Overflow));
        }
        points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        points.dedup();
        let half_hull = |points: &[Vec2]| {
            let mut hull: Vec<Vec2> = Vec::new();
            for point in points {
                while hull.len() > 1 {
                    let (a, b) = (hull[hull.len() - 2], hull[hull.len() - 1]);
                    if (b.as_dvec2() - a.as_dvec2()).perp_dot(point.as_dvec2() - b.as_dvec2()) > 0.0
                    {
                        break;
                    }
                    hull.pop();
                }
                hull.push(*point);
            }
            hull
        };
        let mut hull = half_hull(&points);
        points.reverse();
        let mut upper = half_hull(&points);
        hull.pop();
        upper.pop();
        hull.extend(upper);
        if hull.len() < 3 {
            return Err(numeric(NumericFault::InvalidScale));
        }
        let swept = CombatVolume::convex(hull);
        let spatial = MaterialPrimitive {
            domain: &swept,
            to_world: self.material.to_world,
        };
        let parts = if let Some(admission) = admission {
            partition(
                spatial,
                self.root,
                admission.links,
                admission.first,
                admission.budget,
                admission.convention,
            )?
        } else {
            vec![VolumePiece::from_material(spatial).map_err(numeric)?]
        };
        Ok(MaterialCastCells {
            material: self.material,
            delta: delta.as_dvec2(),
            parts,
            time:self.time,
        })
    }
}

impl MaterialCastCells<'_> {
    pub fn parts(&self) -> &[VolumePiece] {
        &self.parts
    }

    /// Broad phase bounds in each part's physical chart.
    pub fn bounds(&self, part: &VolumePiece) -> Result<Aabb, Refusal> {
        let vertices = part
            .source_region_wide()
            .ok_or_else(|| numeric_at(part, NumericFault::InvalidFrame))?;
        let mut lo = Vec2::splat(f32::INFINITY);
        let mut hi = Vec2::splat(f32::NEG_INFINITY);
        for vertex in vertices {
            let point = part
                .transform()
                .point_from_wide(vertex)
                .map_err(|reason| numeric_at(part, reason))?;
            lo = lo.min(point);
            hi = hi.max(point);
        }
        Ok(Aabb { min: lo, max: hi })
    }

    /// The obstacle is at the end of the world tick. Its displacement covers
    /// that tick. Its shape can be a convex polygon, a segment or a point.
    /// The spatial cell stays fixed during the cast's time interval.
    pub fn against(
        &self,
        part: &VolumePiece,
        obstacle: &[Vec2],
        displacement: Vec2,
    ) -> Result<Option<MaterialCastHit>, Refusal> {
        let inverse = part
            .transform()
            .inverse()
            .map_err(|reason| numeric_at(part, reason))?;
        let mut region = obstacle
            .iter()
            .map(|point| {
                if !point.is_finite() {
                    return Err(NumericFault::NonFinite);
                }
                Ok(inverse.point_wide(point.as_dvec2()))
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|reason| numeric_at(part, reason))?;
        if !displacement.is_finite() {
            return Err(numeric_at(part, NumericFault::NonFinite));
        }
        let full_motion=inverse.direction_wide(displacement.as_dvec2())*inverse.scale_wide();
        for point in &mut region { *point-=full_motion*(1.0-self.time.end); }
        let motion=full_motion*(self.time.end-self.time.start);
        let contact = if motion != DVec2::ZERO {
            for point in &mut region { *point -= motion; }
            translation::cast(self.material.domain, self.delta, &region, motion, part.source_bounds())
                .map_err(|fault|numeric_at(part,fault))?
        } else {
            for plane in part.source_bounds() {
                region = clip(&region, *plane);
                if region.is_empty() { return Ok(None); }
            }
            region.dedup();
            // A solid that only touches the cell boundary occupies no area in it.
            if obstacle.len() >= 3 && !super::thread::has_area(&region) { return Ok(None); }
            primitive_cast(self.material.domain,self.delta,&region)
        };
        let Some(contact) = contact else {
            return Ok(None);
        };
        let point = self
            .material
            .to_world
            .point_from_wide(contact.point)
            .map_err(|reason| numeric_at(part, reason))?;
        let world_point = part
            .transform()
            .point_from_wide(contact.point)
            .map_err(|reason| numeric_at(part, reason))?;
        let normal = self
            .material
            .to_world
            .direction(contact.normal.as_vec2())
            .normalize_or_zero();
        Ok(Some(MaterialCastHit {
            toi: contact.time,
            normal,
            point,
            world_point,
            path: part.path.clone(),
            map: part
                .path_transform()
                .map_err(|reason| numeric_at(part, reason))?,
        }))
    }
}

#[derive(Clone, Copy)]
struct CastHit {
    time: f64,
    normal: DVec2,
    point: DVec2,
}

fn primitive_cast(body:&CombatVolume,delta:DVec2,obstacle:&[DVec2]) -> Option<CastHit> {
    match body {
        CombatVolume::Circle {center,radius}=>circle_cast(center.as_dvec2(),f64::from(*radius),delta,obstacle),
        body=>polygon_cast(&body.polygon().unwrap().iter().map(|p|p.as_dvec2()).collect::<Vec<_>>(),delta,obstacle),
    }
}

fn corners(bounds: Aabb) -> [Vec2; 4] {
    [
        bounds.min,
        Vec2::new(bounds.max.x, bounds.min.y),
        bounds.max,
        Vec2::new(bounds.min.x, bounds.max.y),
    ]
}

fn clip(polygon: &[DVec2], plane: MaterialPlane) -> Vec<DVec2> {
    let mut result = Vec::new();
    for i in 0..polygon.len() {
        let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
        let (da, db) = (
            a.dot(plane.inward) - plane.offset,
            b.dot(plane.inward) - plane.offset,
        );
        if da >= 0.0 {
            result.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            result.push(a + (b - a) * (da / (da - db)));
        }
    }
    result
}

fn edges(polygon: &[DVec2]) -> impl Iterator<Item = (DVec2, DVec2)> + '_ {
    (0..polygon.len()).map(|i| (polygon[i], polygon[(i + 1) % polygon.len()]))
}

fn polygon_cast(body: &[DVec2], delta: DVec2, obstacle: &[DVec2]) -> Option<CastHit> {
    if obstacle.is_empty() {
        return None;
    }
    let mut enter = f64::NEG_INFINITY;
    let mut exit = f64::INFINITY;
    let mut normal = DVec2::ZERO;
    let mut penetrating = true;
    let mut shallowest = (f64::INFINITY, DVec2::ZERO);
    for (a, b) in edges(body).chain(edges(obstacle)) {
        let edge = b - a;
        if edge == DVec2::ZERO {
            continue;
        }
        let n = DVec2::new(-edge.y, edge.x).normalize();
        let span = |points: &[DVec2]| {
            points
                .iter()
                .map(|p| p.dot(n))
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
                    (lo.min(x), hi.max(x))
                })
        };
        let ((alo, ahi), (blo, bhi)) = (span(body), span(obstacle));
        let forward = bhi - alo;
        let backward = ahi - blo;
        let (depth, out) = if forward < backward {
            (forward, n)
        } else {
            (backward, -n)
        };
        if depth < shallowest.0 {
            shallowest = (depth, out);
        }
        if forward <= 0.0 || backward <= 0.0 {
            penetrating = false;
        }
        let speed = delta.dot(n);
        if speed == 0.0 {
            if ahi <= blo || bhi <= alo {
                return None;
            }
            continue;
        }
        let (t0, t1) = ((blo - ahi) / speed, (bhi - alo) / speed);
        let near = t0.min(t1);
        if near > enter {
            enter = near;
            normal = if speed > 0.0 { -n } else { n };
        }
        exit = exit.min(t0.max(t1));
        if enter > exit {
            return None;
        }
    }
    if penetrating {
        normal = shallowest.1;
        if delta.dot(normal) > 0.0 {
            return None;
        }
        enter = 0.0;
    }
    if !penetrating && enter < 0.0 { return None; }
    let time = enter.max(0.0);
    if time > 1.0
        || exit < time
        || normal == DVec2::ZERO
        || if penetrating {
            delta.dot(normal) > 0.0
        } else {
            delta.dot(normal) >= 0.0
        }
    {
        return None;
    }
    let moved = body.iter().map(|p| *p + delta * time).collect::<Vec<_>>();
    let point = closest_polygons(&moved, obstacle);
    Some(CastHit {
        time,
        normal,
        point,
    })
}

fn closest_on_segment(p: DVec2, a: DVec2, b: DVec2) -> DVec2 {
    let d = b - a;
    if d == DVec2::ZERO {
        a
    } else {
        a + d * ((p - a).dot(d) / d.length_squared()).clamp(0.0, 1.0)
    }
}

fn closest_polygons(a: &[DVec2], b: &[DVec2]) -> DVec2 {
    let mut best = (f64::INFINITY, DVec2::ZERO);
    for (points, other) in [(a, b), (b, a)] {
        for p in points {
            for (u, v) in edges(other) {
                let q = closest_on_segment(*p, u, v);
                let distance = p.distance_squared(q);
                if distance < best.0 {
                    best = (distance, (*p + q) * 0.5);
                }
            }
        }
    }
    best.1
}

fn circle_cast(center: DVec2, radius: f64, delta: DVec2, obstacle: &[DVec2]) -> Option<CastHit> {
    if obstacle.is_empty() || radius < 0.0 {
        return None;
    }
    let area: f64 = edges(obstacle).map(|(a, b)| a.perp_dot(b)).sum();
    let inside = area != 0.0 && obstacle.len() >= 3
        && edges(obstacle).all(|(a, b)| (b - a).perp_dot(center - a) * area.signum() >= 0.0);
    let (a, b, closest) = edges(obstacle)
        .map(|(a, b)| (a, b, closest_on_segment(center, a, b)))
        .min_by(|(_, _, a), (_, _, b)| {
            center
                .distance_squared(*a)
                .total_cmp(&center.distance_squared(*b))
        })?;
    let distance = center.distance(closest);
    if inside || distance < radius {
        let offset = if inside {
            closest - center
        } else {
            center - closest
        };
        let normal = if offset == DVec2::ZERO {
            let edge = b - a;
            DVec2::new(edge.y, -edge.x).normalize_or_zero() * if area < 0.0 { -1.0 } else { 1.0 }
        } else {
            offset.normalize()
        };
        if normal == DVec2::ZERO || delta.dot(normal) > 0.0 {
            return None;
        }
        return Some(CastHit {
            time: 0.0,
            normal,
            point: closest,
        });
    }
    if delta == DVec2::ZERO {
        return None;
    }
    let mut best: Option<CastHit> = None;
    let mut take = |time: f64, point: DVec2| {
        if !(0.0..=1.0).contains(&time) {
            return;
        }
        let normal = (center + delta * time - point).normalize_or_zero();
        if normal == DVec2::ZERO || delta.dot(normal) > 0.0 {
            return;
        }
        if best.is_none_or(|hit| time < hit.time) {
            best = Some(CastHit {
                time,
                normal,
                point,
            });
        }
    };
    for (a, b) in edges(obstacle) {
        let edge = b - a;
        if edge != DVec2::ZERO {
            let tangent = edge.normalize();
            let normal = DVec2::new(-tangent.y, tangent.x);
            for side in [-1.0, 1.0] {
                let n = normal * side;
                let speed = delta.dot(n);
                if speed >= 0.0 {
                    continue;
                }
                let time = (radius - (center - a).dot(n)) / speed;
                let at = center + delta * time;
                let along = (at - a).dot(tangent);
                if along >= 0.0 && along <= edge.length() {
                    take(time, at - n * radius);
                }
            }
        }
        let offset = center - a;
        let aa = delta.length_squared();
        let bb = 2.0 * offset.dot(delta);
        let cc = offset.length_squared() - radius * radius;
        let discriminant = bb * bb - 4.0 * aa * cc;
        if discriminant >= 0.0 {
            let root = discriminant.sqrt();
            let q = -0.5 * (bb + root.copysign(bb));
            if q != 0.0 {
                take((q / aa).min(cc / q), a);
            } else {
                take(-bb / (2.0 * aa), a);
            }
        }
    }
    best
}

#[cfg(test)]
mod tests;
