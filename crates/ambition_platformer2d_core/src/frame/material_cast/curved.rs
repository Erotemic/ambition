//! Continuous rigid curves against canonical material and moving cell cuts.

use std::ops::Range;

use bevy_math::DMat2;

use super::{CastHit, CombatVolume, DVec2, MaterialCell, MaterialPlane, NumericFault};
use crate::frame::thread::RefusalReason;
use crate::{Vec2, frame::Sim2};

const MAX_INTERVALS: usize = 16_384;

#[derive(Clone, Copy)]
struct Vertex {
    point: DVec2,
    velocity: DVec2,
}

struct Rigid<'a> {
    end: &'a [Vec2],
    pivot: DVec2,
    shift: DVec2,
    turn: f64,
    time: Range<f64>,
    cell: &'a MaterialCell,
    inverse: Sim2,
}

impl Rigid<'_> {
    fn span(&self) -> f64 {
        self.time.end - self.time.start
    }

    fn pose(&self, time: f64) -> (DVec2, DMat2) {
        let world = self.time.start + self.span() * time;
        let rotation = crate::frame::rotation_ending_at(self.turn, world);
        (self.pivot - self.shift * (1.0 - world), rotation)
    }

    fn source_vector(&self, v: DVec2) -> DVec2 {
        self.inverse.direction_wide(v) * self.inverse.scale_wide()
    }

    fn rounding(&self, time: f64, source_extent: f64) -> Result<f64, NumericFault> {
        let (pivot, _) = self.pose(time);
        let radius = self
            .end
            .iter()
            .map(|p| (p.as_dvec2() - self.pivot).length())
            .fold(0.0_f64, f64::max);
        let inverse = self.cell.at(time)?.inverse()?;
        Ok(scalar_round(
            (pivot.length() + radius) * inverse.scale_wide()
                + inverse.translation().length()
                + source_extent,
        ) * (self.cell.path.len() + 1) as f64)
    }

    fn contact_rounding(&self, body: &CombatVolume, time: f64) -> Result<f64, NumericFault> {
        let domain = domain_polygon(body);
        let cuts = self.cell.planes_at(time)?.unwrap_or_default();
        let extent = domain.iter().map(|p| p.length()).fold(0.0_f64, f64::max)
            + cuts
                .iter()
                .map(|p| p.plane.offset.abs() + p.offset_rate.abs() + p.normal_rate.length())
                .fold(0.0_f64, f64::max);
        let evaluations = 1 + rigid_planes(self.end, self.pivot).len() + cuts.len() + domain.len();
        Ok(self.rounding(time, extent)? * evaluations as f64)
    }

    fn point(&self, local: DVec2, time: f64) -> Result<Vertex, NumericFault> {
        let (pivot, rotation) = self.pose(time);
        let arm = rotation * local;
        let inverse = self.cell.at(time)?.inverse()?;
        let point = inverse.point_wide(pivot + arm);
        let velocity = (self.shift + perpendicular(arm) * self.turn) * self.span()
            - self.cell.velocity(point, time)?;
        Ok(Vertex {
            point,
            velocity: inverse.direction_wide(velocity) * inverse.scale_wide(),
        })
    }

    fn region(&self, time: f64) -> Result<Vec<Vertex>, NumericFault> {
        let mut points = self
            .end
            .iter()
            .map(|p| self.point(p.as_dvec2() - self.pivot, time))
            .collect::<Result<Vec<_>, _>>()?;
        let Some(cuts) = self.cell.planes_at(time)? else {
            return Ok(Vec::new());
        };
        for cut in cuts {
            points = clip_vertices(&points, cut);
            if points.is_empty() {
                break;
            }
        }
        Ok(points)
    }

    /// Bounds the rotation of each face normal and the relative translation.
    /// The bound retains every winding; it is not an endpoint chord.
    fn relaxed(
        &self,
        body: &CombatVolume,
        interval: &Range<f64>,
        uncertainty: f64,
    ) -> Result<Option<Vec<DVec2>>, NumericFault> {
        let (middle, half) = crate::frame::interval_center(interval);
        let (pivot, rotation) = self.pose(middle);
        let map = self.cell.at(middle)?;
        let domain = domain_polygon(body);
        if self.end.len() >= 3 {
            if let Some(curve) = &self.cell.curve {
                if curve.excludes_rigid(
                    self.end,
                    self.shift.as_vec2(),
                    self.turn as f32,
                    self.pivot.as_vec2(),
                    interval,
                )? {
                    return Ok(None);
                }
            }
        }
        // A solid needs area on this side of every material cut.
        if self.end.len() >= 3 && self.cell.curve.is_none() {
            for cut in &self.cell.planes {
                if self.end.iter().all(|p| {
                    let (upper, rounding) =
                        self.side_bound(p.as_dvec2() - self.pivot, *cut, interval);
                    upper <= rounding
                }) {
                    return Ok(None);
                }
            }
        }
        let reach = domain
            .iter()
            .map(|p| map.point_wide(*p).distance(pivot))
            .fold(0.0_f64, f64::max);
        let angle = crate::frame::rotation_bound(
            self.turn,
            self.time.start + self.span() * middle,
            self.span() * half,
        );
        let relative = self.cell.travel - self.shift * self.span();
        let extent = domain.iter().map(|p| p.length()).fold(0.0_f64, f64::max);
        let travel = if self.cell.curve.is_some() {
            self.cell.variation(middle, half)? + self.shift.length() * self.span() * half
        } else {
            relative.length() * half
        };
        let error = (reach * angle + travel) / map.scale_wide()
            + self.rounding(middle, extent)?
            + uncertainty;
        if !error.is_finite() {
            return Err(NumericFault::Overflow);
        }
        let mut region = domain;
        for (normal, constant) in rigid_planes(self.end, self.pivot) {
            let normal = rotation * normal;
            let inward = map.inverse_direction_wide(normal);
            let offset = (normal.dot(pivot - map.translation()) + constant) / map.scale_wide();
            let rounding = round_bound(inward, offset, &region);
            region = super::clip(
                &region,
                MaterialPlane {
                    inward,
                    offset: offset - error - rounding,
                },
            );
            if region.is_empty() {
                return Ok(None);
            }
        }
        let Some(cuts) = self.cell.relaxed_planes(interval)? else {
            return Ok(None);
        };
        for cut in cuts {
            let rounding = round_bound(cut.inward, cut.offset, &region);
            region = super::clip(
                &region,
                MaterialPlane {
                    offset: cut.offset - rounding - uncertainty * cut.inward.length(),
                    ..cut
                },
            );
            if region.is_empty() {
                return Ok(None);
            }
        }
        Ok(intersection(body, &region).map(|_| region))
    }

    fn side_bound(
        &self,
        local: DVec2,
        cut: super::translation::MovingPlane,
        interval: &Range<f64>,
    ) -> (f64, f64) {
        let axis = self.cell.map.direction_wide(cut.plane.inward) / self.cell.map.scale_wide();
        let pivot = self.pivot - self.shift * (1.0 - self.time.start);
        let arm = crate::frame::rotation_ending_at(self.turn, self.time.start) * local;
        let a = axis.dot(pivot - self.cell.map.translation()) - cut.plane.offset;
        let b = axis.dot(self.shift * self.span() - self.cell.travel) - cut.rate;
        let magnitude = axis.abs().dot((pivot - self.cell.map.translation()).abs())
            + cut.plane.offset.abs()
            + (axis
                .abs()
                .dot((self.shift * self.span()).abs() + self.cell.travel.abs())
                + cut.rate.abs())
                * interval.start.abs().max(interval.end.abs())
            + axis.length() * local.length();
        (
            trig_max(
                a,
                b,
                axis.dot(arm),
                axis.dot(perpendicular(arm)),
                self.turn * self.span(),
                interval,
            ),
            scalar_round(magnitude),
        )
    }

    /// Follow one material point of the collider. If it stays inside both
    /// the body and cell, the initial overlap has not ended in this interval.
    fn overlap_persists(&self, body: &CombatVolume, point: DVec2, interval: &Range<f64>) -> bool {
        let (middle, half) = crate::frame::interval_center(interval);
        let (pivot, rotation) = self.pose(middle);
        let map = match self.cell.at(middle) {
            Ok(map) => map,
            Err(_) => return false,
        };
        let arm = map.point_wide(point) - pivot;
        let local = rotation.transpose() * arm;
        let at = match self.point(local, middle) {
            Ok(point) => point.point,
            Err(_) => return false,
        };
        let radius = crate::frame::rotation_bound(
            self.turn,
            self.time.start + self.span() * middle,
            self.span() * half,
        ) * arm.length();
        let travel = if self.cell.curve.is_some() {
            match self.cell.variation(middle, half) {
                Ok(v) => v + self.shift.length() * self.span() * half,
                Err(_) => return false,
            }
        } else {
            (self.shift * self.span() - self.cell.travel).length() * half
        };
        let reach = (radius + travel) / map.scale_wide();
        // A rejected component includes both the pose and interval error.
        let rounding = match self.contact_rounding(body, middle) {
            Ok(r) => r * 2.0,
            Err(_) => return false,
        };
        if !reach.is_finite() {
            return false;
        }
        let cuts = match self.cell.inner_planes(interval) {
            Ok(Some(cuts)) => cuts,
            _ => return false,
        };
        if !cuts.iter().all(|p| {
            at.dot(p.inward) - p.offset + rounding * p.inward.length()
                > p.inward.length() * reach
                    + scalar_round(at.abs().dot(p.inward.abs()) + p.offset.abs())
        }) {
            return false;
        }
        match body {
            CombatVolume::Circle { center, radius } => {
                at.distance(center.as_dvec2()) + reach < f64::from(*radius) + rounding
            }
            _ => polygon_planes(&domain_polygon(body)).into_iter().all(|p| {
                at.dot(p.inward) - p.offset + rounding * p.inward.length()
                    > reach * p.inward.length()
                        + scalar_round(at.abs().dot(p.inward.abs()) + p.offset.abs())
            }),
        }
    }
}

fn perpendicular(p: DVec2) -> DVec2 {
    DVec2::new(-p.y, p.x)
}

fn scalar_round(magnitude: f64) -> f64 {
    magnitude * f64::EPSILON * 64.0
}

/// Extrema of a linear term and one recorded angular winding.
fn trig_max(a: f64, b: f64, c: f64, d: f64, omega: f64, time: &Range<f64>) -> f64 {
    let value = |t: f64| {
        let (sin, cos) = (omega * t).sin_cos();
        a + b * t + c * cos + d * sin
    };
    let mut upper = value(time.start).max(value(time.end));
    let amplitude = c.hypot(d);
    let envelope = a + (b * time.start).max(b * time.end) + amplitude;
    if omega == 0.0 || amplitude == 0.0 {
        return upper;
    }
    if (omega * time.start).abs().max((omega * time.end).abs()) > 2.0_f64.powi(48) {
        return envelope;
    }
    let slope = -b / (omega * amplitude);
    if slope.abs() > 1.0 {
        return upper;
    }
    let phase = c.atan2(d);
    let lo = (omega * time.start).min(omega * time.end) + phase;
    let hi = (omega * time.start).max(omega * time.end) + phase;
    let tau = std::f64::consts::TAU;
    for base in [slope.acos(), -slope.acos()] {
        let first = ((lo - base) / tau).ceil();
        let last = ((hi - base) / tau).floor();
        if first > last {
            continue;
        }
        // Within one family, successive extrema differ only by the linear
        // term. Its first and last roots give the maximum over all windings.
        for winding in [first, last] {
            let t = ((base - phase) + tau * winding) / omega;
            if t >= time.start && t <= time.end {
                upper = upper.max(value(t));
            }
        }
    }
    // Multiplication of a large angle can lose low bits. The amplitude
    // envelope remains valid even when the local phase cannot be resolved.
    (upper
        + amplitude
            * (omega.abs() * time.end.abs().max(time.start.abs()) * f64::EPSILON * 8.0).min(2.0))
    .min(envelope)
}

fn round_bound(inward: DVec2, offset: f64, points: &[DVec2]) -> f64 {
    scalar_round(
        points
            .iter()
            .map(|p| p.abs().dot(inward.abs()) + offset.abs())
            .fold(0.0_f64, f64::max),
    )
}

pub(super) fn domain_polygon(body: &CombatVolume) -> Vec<DVec2> {
    match body {
        CombatVolume::Circle { center, radius } => {
            let center = center.as_dvec2();
            let radius = f64::from(*radius);
            vec![
                center - DVec2::splat(radius),
                center + DVec2::new(radius, -radius),
                center + DVec2::splat(radius),
                center + DVec2::new(-radius, radius),
            ]
        }
        _ => body
            .polygon()
            .unwrap()
            .iter()
            .map(|p| p.as_dvec2())
            .collect(),
    }
}

pub(super) fn polygon_planes(points: &[DVec2]) -> Vec<MaterialPlane> {
    let area = super::edges(points)
        .map(|(a, b)| a.perp_dot(b))
        .sum::<f64>();
    super::edges(points)
        .filter_map(|(a, b)| {
            let n = perpendicular(b - a) * area.signum();
            (n != DVec2::ZERO).then(|| {
                let inward = n.normalize();
                MaterialPlane {
                    inward,
                    offset: inward.dot(a),
                }
            })
        })
        .collect()
}

fn rigid_planes(points: &[Vec2], pivot: DVec2) -> Vec<(DVec2, f64)> {
    if points.is_empty() {
        return vec![];
    }
    let points = points
        .iter()
        .map(|p| p.as_dvec2() - pivot)
        .collect::<Vec<_>>();
    if points.len() > 2 {
        return polygon_planes(&points)
            .into_iter()
            .map(|p| (p.inward, p.offset))
            .collect();
    }
    let a = points[0];
    let b = *points.last().unwrap();
    let tangent = (b - a).normalize_or_zero();
    if tangent == DVec2::ZERO {
        [DVec2::X, -DVec2::X, DVec2::Y, -DVec2::Y]
            .map(|n| (n, n.dot(a)))
            .to_vec()
    } else {
        let normal = perpendicular(tangent);
        vec![
            (tangent, tangent.dot(a)),
            (-tangent, -tangent.dot(b)),
            (normal, normal.dot(a)),
            (-normal, -normal.dot(a)),
        ]
    }
}

fn clip_vertices(points: &[Vertex], cut: super::translation::PlaneMotion) -> Vec<Vertex> {
    let inward = cut.plane.inward;
    let offset = cut.plane.offset;
    let mut result = Vec::new();
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let side = |p: DVec2| {
            let d = p.dot(inward) - offset;
            if d.abs() <= scalar_round(p.abs().dot(inward.abs()) + offset.abs()) {
                0.0
            } else {
                d
            }
        };
        let da = side(a.point);
        let db = side(b.point);
        if da >= 0.0 {
            result.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let edge = b.point - a.point;
            let relative = b.velocity - a.velocity;
            let along = da / (da - db);
            let speed = (cut.offset_rate
                - inward.dot(a.velocity)
                - along * inward.dot(relative)
                - cut.normal_rate.dot(a.point + edge * along))
                / inward.dot(edge);
            result.push(Vertex {
                point: a.point + edge * along,
                velocity: a.velocity + relative * along + edge * speed,
            });
        }
    }
    result
}

/// A point in the closed intersection, including an isolated tangent.
pub(super) fn intersection(body: &CombatVolume, region: &[DVec2]) -> Option<DVec2> {
    if region.is_empty() {
        return None;
    }
    match body {
        CombatVolume::Circle { center, radius } => {
            let center = center.as_dvec2();
            let planes = polygon_planes(region);
            if !planes.is_empty() && planes.iter().all(|p| p.inward.dot(center) >= p.offset) {
                return Some(center);
            }
            let point = super::edges(region)
                .map(|(a, b)| super::closest_on_segment(center, a, b))
                .min_by(|a, b| {
                    a.distance_squared(center)
                        .total_cmp(&b.distance_squared(center))
                })?;
            let error = scalar_round(center.length() + point.length() + f64::from(*radius));
            (center.distance(point) <= f64::from(*radius) + error).then_some(point)
        }
        _ => {
            let mut overlap = region.to_vec();
            for p in polygon_planes(&domain_polygon(body)) {
                overlap = super::clip(&overlap, p);
                if overlap.is_empty() {
                    return None;
                }
            }
            Some(overlap.iter().copied().sum::<DVec2>() / overlap.len() as f64)
        }
    }
}

struct Witness {
    gap: f64,
    normal: DVec2,
    point: DVec2,
    velocity: DVec2,
}

fn witness(body: &CombatVolume, region: &[Vertex]) -> Option<Witness> {
    if region.is_empty() {
        return None;
    }
    let points = region.iter().map(|p| p.point).collect::<Vec<_>>();
    let (gap, normal, point, velocity) = match body {
        CombatVolume::Circle { center, radius } => {
            let center = center.as_dvec2();
            let inside = polygon_planes(&points);
            let inside =
                !inside.is_empty() && inside.iter().all(|p| p.inward.dot(center) >= p.offset);
            let (a, b, point) = super::edges(&points)
                .enumerate()
                .map(|(i, (a, b))| {
                    (
                        i,
                        (i + 1) % points.len(),
                        super::closest_on_segment(center, a, b),
                    )
                })
                .min_by(|a, b| {
                    a.2.distance_squared(center)
                        .total_cmp(&b.2.distance_squared(center))
                })?;
            let edge = points[b] - points[a];
            let along = if edge == DVec2::ZERO {
                0.0
            } else {
                (point - points[a]).dot(edge) / edge.length_squared()
            };
            let velocity = region[a].velocity + (region[b].velocity - region[a].velocity) * along;
            let offset = if inside {
                point - center
            } else {
                center - point
            };
            let normal = if offset != DVec2::ZERO {
                offset.normalize()
            } else if velocity != DVec2::ZERO {
                -velocity.normalize()
            } else if edge != DVec2::ZERO {
                perpendicular(edge).normalize()
            } else {
                DVec2::X
            };
            (
                (if inside {
                    -offset.length()
                } else {
                    offset.length()
                }) - f64::from(*radius),
                normal,
                point,
                velocity,
            )
        }
        _ => {
            let polygon = domain_polygon(body);
            let mut best = (f64::NEG_INFINITY, DVec2::ZERO);
            for (a, b) in super::edges(&polygon).chain(super::edges(&points)) {
                let edge = b - a;
                if edge == DVec2::ZERO {
                    continue;
                }
                let n = perpendicular(edge).normalize();
                let span = |points: &[DVec2]| {
                    points
                        .iter()
                        .map(|p| p.dot(n))
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
                            (lo.min(x), hi.max(x))
                        })
                };
                let ((alo, ahi), (blo, bhi)) = (span(&polygon), span(&points));
                for (gap, normal) in [(alo - bhi, n), (blo - ahi, -n)] {
                    if gap > best.0 {
                        best = (gap, normal);
                    }
                }
            }
            if best.1 == DVec2::ZERO {
                return None;
            }
            let normal = best.1;
            let support = points
                .iter()
                .map(|p| p.dot(normal))
                .fold(f64::NEG_INFINITY, f64::max);
            let vertex = region
                .iter()
                .filter(|p| {
                    support - p.point.dot(normal) <= scalar_round(support.abs() + p.point.length())
                })
                .max_by(|a, b| a.velocity.dot(normal).total_cmp(&b.velocity.dot(normal)))?;
            let point = super::edges(&points)
                .flat_map(|(a, b)| {
                    polygon
                        .iter()
                        .map(move |p| super::closest_on_segment(*p, a, b))
                })
                .min_by(|a, b| {
                    let distance = |p: &DVec2| {
                        super::edges(&polygon)
                            .map(|(a, b)| p.distance_squared(super::closest_on_segment(*p, a, b)))
                            .fold(f64::INFINITY, f64::min)
                    };
                    distance(a).total_cmp(&distance(b))
                })?;
            (best.0, normal, point, vertex.velocity)
        }
    };
    Some(Witness {
        gap,
        normal,
        point,
        velocity,
    })
}

pub(super) fn cast(
    body: &CombatVolume,
    cell: &MaterialCell,
    obstacle: &[Vec2],
    displacement: Vec2,
    turn: f32,
    pivot: Vec2,
    time: Range<f64>,
    initial_overlap: super::InitialOverlap,
    mut accepts: impl FnMut(CastHit) -> bool,
) -> Result<Option<CastHit>, RefusalReason> {
    let numeric = RefusalReason::Numeric;
    if !displacement.is_finite()
        || !pivot.is_finite()
        || !turn.is_finite()
        || obstacle.iter().any(|p| !p.is_finite())
    {
        return Err(numeric(NumericFault::NonFinite));
    }
    if obstacle.is_empty() {
        return Ok(None);
    }
    if obstacle.len() >= 3
        && !crate::frame::thread::has_area(
            &obstacle.iter().map(|p| p.as_dvec2()).collect::<Vec<_>>(),
        )
    {
        return Err(numeric(NumericFault::InvalidFrame));
    }
    let rigid = Rigid {
        end: obstacle,
        pivot: pivot.as_dvec2(),
        shift: displacement.as_dvec2(),
        turn: f64::from(turn),
        time,
        cell,
        inverse: cell.map.inverse().map_err(numeric)?,
    };
    if rigid.span() == 0.0 && cell.curve.is_none() {
        let points = obstacle
            .iter()
            .map(|p| {
                rigid
                    .point(p.as_dvec2() - rigid.pivot, cell.time.start)
                    .map(|point| point.point)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(numeric)?;
        let duration = cell.time.end - cell.time.start;
        let motion = rigid.source_vector(-cell.travel) * duration;
        let contact = super::translation::cast(
            body,
            DVec2::ZERO,
            &points,
            motion,
            cell.planes.iter().map(|p| p.during(cell.time.clone())),
            initial_overlap,
        )
        .map_err(numeric)?;
        return Ok(contact
            .map(|hit| CastHit {
                time: cell.time.start + duration * hit.time,
                ..hit
            })
            .filter(|hit| accepts(*hit)));
    }
    let initial = rigid.region(cell.time.start).map_err(numeric)?;
    let initial_witness = witness(body, &initial);
    let mut escaping = initial_overlap == super::InitialOverlap::AllowSeparation && initial_witness
        .as_ref()
        .is_some_and(|hit| hit.gap <= 0.0 && hit.velocity.dot(hit.normal) < 0.0);
    let mut stack = vec![cell.time.clone()];
    let mut work = 0;
    // Visit intervals in time order. Reject an overlap component only after
    // separation is proved; a later approach can still supply a contact.
    'search: while let Some(interval) = stack.pop() {
        work += 1;
        if work > MAX_INTERVALS {
            return Err(RefusalReason::WorkLimit);
        }
        let middle = interval.start + (interval.end - interval.start) * 0.5;
        let uncertainty = if escaping {
            rigid.contact_rounding(body, middle).map_err(numeric)?
        } else {
            0.0
        };
        if rigid
            .relaxed(body, &interval, uncertainty)
            .map_err(numeric)?
            .is_none()
        {
            escaping = false;
            continue;
        }
        let region = rigid.region(middle).map_err(numeric)?;
        if region
            .iter()
            .any(|p| !p.point.is_finite() || !p.velocity.is_finite())
        {
            return Err(numeric(NumericFault::Overflow));
        }
        if escaping {
            let points = region.iter().map(|p| p.point).collect::<Vec<_>>();
            let point = intersection(body, &points).or_else(|| {
                witness(body, &region)
                    .filter(|hit| hit.gap <= uncertainty * 2.0)
                    .map(|hit| hit.point)
            });
            if point.is_some_and(|point| rigid.overlap_persists(body, point, &interval)) {
                continue;
            }
        }
        let small = middle == interval.start || middle == interval.end;
        if small {
            if escaping {
                continue;
            }
            let rounding = rigid.contact_rounding(body, middle).map_err(numeric)?;
            for time in [interval.start, middle, interval.end] {
                if let Some(hit) = witness(body, &rigid.region(time).map_err(numeric)?) {
                    if hit.gap <= rounding {
                        let contact = CastHit {
                            time,
                            normal: hit.normal,
                            point: hit.point,
                        };
                        if accepts(contact) {
                            return Ok(Some(contact));
                        }
                        escaping = true;
                        continue 'search;
                    }
                }
            }
            return Err(numeric(NumericFault::Precision));
        }
        if interval.start == cell.time.start && !escaping {
            if let Some(hit) = initial_witness.as_ref() {
                if hit.gap <= 0.0 && (hit.velocity.dot(hit.normal) >= 0.0
                    || initial_overlap == super::InitialOverlap::Report && hit.gap < 0.0) {
                    let contact = CastHit {
                        time: interval.start,
                        normal: hit.normal,
                        point: hit.point,
                    };
                    if accepts(contact) {
                        return Ok(Some(contact));
                    }
                    escaping = true;
                }
            }
        }
        stack.push(middle..interval.end);
        stack.push(interval.start..middle);
    }
    Ok(None)
}
