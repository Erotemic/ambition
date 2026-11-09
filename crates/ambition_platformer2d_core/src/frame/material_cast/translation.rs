//! Contact with a translating obstacle inside a fixed spatial cell.

use super::{CastHit, CombatVolume, DVec2, MaterialPlane, clip, edges};
use crate::frame::NumericFault;

#[derive(Clone, Copy)]
struct Plane {
    inward: DVec2,
    offset: f64,
    rate: f64,
    cell: bool,
}

#[derive(Clone, Copy)]
struct Point {
    origin: DVec2,
    delta: DVec2,
}

impl Point {
    fn at(self, time: f64) -> DVec2 {
        self.origin + self.delta * time
    }
}

#[derive(Clone, Copy)]
struct Window {
    lo: f64,
    hi: f64,
    normal: DVec2,
}

impl Window {
    const FULL: Self = Self {
        lo: 0.0,
        hi: 1.0,
        normal: DVec2::ZERO,
    };
}

fn planes(points: &[DVec2], delta: DVec2) -> Vec<Plane> {
    let area: f64 = edges(points).map(|(a, b)| a.perp_dot(b)).sum();
    let mut result = Vec::new();
    let mut push = |inward: DVec2, point: DVec2| {
        if inward != DVec2::ZERO {
            let inward = inward.normalize();
            result.push(Plane {
                inward,
                offset: inward.dot(point),
                rate: inward.dot(delta),
                cell: false,
            });
        }
    };
    if points.len() <= 2 {
        let Some(&a) = points.first() else {
            return result;
        };
        let b = *points.last().unwrap();
        let tangent = (b - a).normalize_or_zero();
        if tangent == DVec2::ZERO {
            for n in [DVec2::X, -DVec2::X, DVec2::Y, -DVec2::Y] {
                push(n, a);
            }
        } else {
            let normal = DVec2::new(-tangent.y, tangent.x);
            push(tangent, a);
            push(-tangent, b);
            push(normal, a);
            push(-normal, a);
        }
    } else {
        for (a, b) in edges(points) {
            let edge = b - a;
            push(DVec2::new(-edge.y, edge.x) * area.signum(), a);
        }
    }
    result
}

fn intersection(a: Plane, b: Plane) -> Result<Option<Point>, NumericFault> {
    let determinant = a.inward.perp_dot(b.inward);
    if determinant == 0.0 {
        return Ok(None);
    }
    let solve = |u: f64, v: f64| {
        DVec2::new(
            (u * b.inward.y - a.inward.y * v) / determinant,
            (a.inward.x * v - u * b.inward.x) / determinant,
        )
    };
    let point = Point {
        origin: solve(a.offset, b.offset),
        delta: solve(a.rate, b.rate),
    };
    if !point.origin.is_finite() || !point.delta.is_finite() {
        return Err(NumericFault::Overflow);
    }
    Ok(Some(point))
}

/// Each half plane gives one linear constraint on time.
fn inside(
    point: Point,
    planes: &[Plane],
    mut window: Window,
    skip: Option<(usize, usize)>,
) -> Result<Option<Window>, NumericFault> {
    for (index, plane) in planes.iter().enumerate() {
        if skip.is_some_and(|(a, b)| index == a || index == b) {
            continue;
        }
        let mut side = point.origin.dot(plane.inward) - plane.offset;
        let error =
            (point.origin.abs().dot(plane.inward.abs()) + plane.offset.abs()) * f64::EPSILON * 8.0;
        if side.abs() <= error {
            side = 0.0;
        }
        let speed = point.delta.dot(plane.inward) - plane.rate;
        if !side.is_finite() || !speed.is_finite() || !error.is_finite() {
            return Err(NumericFault::Overflow);
        }
        if speed == 0.0 {
            if side < 0.0 {
                return Ok(None);
            }
        } else {
            let time = -side / speed;
            if speed > 0.0 && time >= window.lo {
                window.lo = time;
                window.normal = if skip.is_some() && plane.cell {
                    plane.inward
                } else {
                    -plane.inward
                };
            } else if speed < 0.0 {
                window.hi = window.hi.min(time);
            }
            if window.lo > window.hi {
                return Ok(None);
            }
        }
    }
    Ok(Some(window))
}

fn take(best: &mut Option<CastHit>, hit: CastHit) {
    if (0.0..=1.0).contains(&hit.time)
        && hit.normal != DVec2::ZERO
        && best.is_none_or(|best| hit.time < best.time)
    {
        *best = Some(hit);
    }
}

fn hull(mut points: Vec<DVec2>) -> Vec<DVec2> {
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let half = |points: &[DVec2]| {
        let mut result: Vec<DVec2> = Vec::new();
        for &point in points {
            while result.len() > 1 {
                let a = result[result.len() - 2];
                let b = result[result.len() - 1];
                if (b - a).perp_dot(point - b) > 0.0 {
                    break;
                }
                result.pop();
            }
            result.push(point);
        }
        result
    };
    let mut lower = half(&points);
    points.reverse();
    let mut upper = half(&points);
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

pub(super) fn cast(
    body: &CombatVolume,
    delta: DVec2,
    obstacle: &[DVec2],
    motion: DVec2,
    cell: &[MaterialPlane],
) -> Result<Option<CastHit>, NumericFault> {
    if cell.is_empty() {
        return Ok(
            super::primitive_cast(body, delta - motion, obstacle).map(|mut hit| {
                hit.point += motion * hit.time;
                hit
            }),
        );
    }
    let mut constraints = planes(obstacle, motion);
    constraints.extend(cell.iter().map(|plane| Plane {
        inward: plane.inward,
        offset: plane.offset,
        rate: 0.0,
        cell: true,
    }));
    let mut vertices = Vec::new();
    for a in 0..constraints.len() {
        for b in a + 1..constraints.len() {
            if let Some(point) = intersection(constraints[a], constraints[b])? {
                if let Some(window) = inside(point, &constraints, Window::FULL, Some((a, b)))? {
                    vertices.push((point, window));
                }
            }
        }
    }
    let region_at = |time: f64| {
        let error = time.abs() * f64::EPSILON * 16.0;
        hull(
            vertices
                .iter()
                .filter(|(_, window)| window.lo <= time + error && window.hi >= time - error)
                .map(|(point, _)| point.at(time))
                .collect(),
        )
    };
    let initial = region_at(0.0);
    let occupied = |time: f64| {
        if obstacle.len() < 3 || crate::frame::thread::has_area(&region_at(time)) {
            return true;
        }
        let next = vertices
            .iter()
            .flat_map(|(_, window)| [window.lo, window.hi])
            .filter(|bound| *bound > time)
            .min_by(f64::total_cmp)
            .unwrap_or(1.0);
        next > time && crate::frame::thread::has_area(&region_at(time + (next - time) * 0.5))
    };
    let mut best = if obstacle.len() >= 3 && !crate::frame::thread::has_area(&initial) {
        None
    } else {
        super::primitive_cast(body, DVec2::ZERO, &initial)
    };
    if let Some(hit) = best {
        let velocity = vertices
            .iter()
            .filter(|(_, window)| window.lo <= 0.0 && window.hi > 0.0)
            .max_by(|(a, _), (b, _)| {
                a.origin
                    .dot(hit.normal)
                    .total_cmp(&b.origin.dot(hit.normal))
                    .then(a.delta.dot(hit.normal).total_cmp(&b.delta.dot(hit.normal)))
            })
            .map_or(DVec2::ZERO, |(point, _)| point.delta);
        if (delta - velocity).dot(hit.normal) > 0.0 {
            return Ok(None);
        }
    }
    let take = |best: &mut Option<CastHit>, hit: CastHit| {
        if occupied(hit.time) {
            take(best, hit);
        }
    };
    // A solid can first occupy the cell as a line through the body. The
    // line's ends can be outside the body, so vertex contact alone is not enough.
    if obstacle.len() >= 3 {
        for &(_, window) in &vertices {
            if window.normal == DVec2::ZERO || !occupied(window.lo) {
                continue;
            }
            let region = region_at(window.lo);
            if crate::frame::thread::has_area(&region) {
                continue;
            }
            let point = match body {
                CombatVolume::Circle { center, radius } => super::circle_cast(
                    center.as_dvec2() + delta * window.lo,
                    f64::from(*radius),
                    DVec2::ZERO,
                    &region,
                )
                .map(|hit| hit.point),
                body => {
                    let polygon = body
                        .polygon()
                        .unwrap()
                        .iter()
                        .map(|p| p.as_dvec2() + delta * window.lo)
                        .collect::<Vec<_>>();
                    let mut overlap = region;
                    for plane in planes(&polygon, DVec2::ZERO) {
                        overlap = clip(
                            &overlap,
                            MaterialPlane {
                                inward: plane.inward,
                                offset: plane.offset,
                            },
                        );
                    }
                    overlap.first().copied()
                }
            };
            if let Some(point) = point {
                take(
                    &mut best,
                    CastHit {
                        time: window.lo,
                        normal: window.normal.normalize(),
                        point,
                    },
                );
            }
        }
    }
    match body {
        CombatVolume::Circle { center, radius } => {
            let center = center.as_dvec2();
            let radius = f64::from(*radius);
            // A face contact is on its line and inside every other half plane.
            for plane in &constraints {
                let length = plane.inward.length();
                let outward = -plane.inward / length;
                let speed = delta.dot(plane.inward) - plane.rate;
                if speed <= 0.0 {
                    continue;
                }
                let time = (plane.offset - center.dot(plane.inward) - radius * length) / speed;
                if !(0.0..=1.0).contains(&time) {
                    continue;
                }
                let point = Point {
                    origin: center - outward * radius,
                    delta,
                };
                if inside(
                    point,
                    &constraints,
                    Window {
                        lo: time,
                        hi: time,
                        normal: DVec2::ZERO,
                    },
                    None,
                )?
                .is_some()
                {
                    take(
                        &mut best,
                        CastHit {
                            time,
                            normal: outward,
                            point: point.at(time),
                        },
                    );
                }
            }
            // A clipped vertex has an affine path while its constraints hold.
            for &(point, window) in &vertices {
                let offset = center - point.origin;
                let relative = delta - point.delta;
                let aa = relative.length_squared();
                if !aa.is_finite() {
                    return Err(NumericFault::Overflow);
                }
                if aa == 0.0 {
                    continue;
                }
                let bb = 2.0 * offset.dot(relative);
                let cc = offset.length_squared() - radius * radius;
                let discriminant = bb * bb - 4.0 * aa * cc;
                if !bb.is_finite() || !cc.is_finite() || !discriminant.is_finite() {
                    return Err(NumericFault::Overflow);
                }
                if discriminant < 0.0 {
                    continue;
                }
                let q = -0.5 * (bb + discriminant.sqrt().copysign(bb));
                let time = if q == 0.0 {
                    -bb / (2.0 * aa)
                } else {
                    (q / aa).min(cc / q)
                };
                if time < window.lo || time > window.hi {
                    continue;
                }
                let normal = (center + delta * time - point.at(time)).normalize_or_zero();
                if normal == DVec2::ZERO {
                    return Err(NumericFault::Precision);
                }
                if relative.dot(normal) > 0.0 {
                    continue;
                }
                take(
                    &mut best,
                    CastHit {
                        time,
                        normal,
                        point: point.at(time),
                    },
                );
            }
        }
        body => {
            let polygon = body
                .polygon()
                .unwrap()
                .iter()
                .map(|p| p.as_dvec2())
                .collect::<Vec<_>>();
            let body_planes = planes(&polygon, delta);
            // At first polygon contact, a vertex of one region is in the other.
            for origin in polygon {
                let point = Point { origin, delta };
                if let Some(window) = inside(point, &constraints, Window::FULL, None)? {
                    take(
                        &mut best,
                        CastHit {
                            time: window.lo,
                            normal: window.normal.normalize_or_zero(),
                            point: point.at(window.lo),
                        },
                    );
                }
            }
            for &(point, window) in &vertices {
                // The window's first bound can be a cell boundary. Its normal
                // already points toward the source body in that case.
                if let Some(hit) = inside(
                    point,
                    &body_planes,
                    Window {
                        normal: -window.normal,
                        ..window
                    },
                    None,
                )? {
                    take(
                        &mut best,
                        CastHit {
                            time: hit.lo,
                            normal: -hit.normal.normalize_or_zero(),
                            point: point.at(hit.lo),
                        },
                    );
                }
            }
        }
    }
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Aabb, Vec2};

    #[test]
    fn an_initial_overlap_can_separate_while_the_clipped_region_changes() {
        let cell = [MaterialPlane {
            inward: DVec2::X,
            offset: 0.0,
        }];
        let obstacle = [
            DVec2::new(-1.0, -2.0),
            DVec2::new(1.0, -2.0),
            DVec2::new(1.0, 2.0),
            DVec2::new(-1.0, 2.0),
        ];
        for body in [
            CombatVolume::aabb(Aabb::new(Vec2::new(1.0, 0.0), Vec2::new(2.0, 1.0))),
            CombatVolume::Circle {
                center: Vec2::new(1.0, 0.0),
                radius: 2.0,
            },
        ] {
            assert!(
                cast(
                    &body,
                    DVec2::ZERO,
                    &obstacle,
                    DVec2::new(-10.0, -10.0),
                    &cell
                )
                .unwrap()
                .is_none()
            );
        }
    }

    #[test]
    fn translated_contacts_agree_with_rectangle_and_circle_intersections_in_a_cell() {
        let mut state = 1729_u64;
        let mut number = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 32) as u32) as f64 / f64::from(u32::MAX)
        };
        let cell = [
            MaterialPlane {
                inward: DVec2::X,
                offset: 0.0,
            },
            MaterialPlane {
                inward: DVec2::Y,
                offset: -3.0,
            },
        ];
        for _ in 0..1000 {
            let center = Vec2::new((number() * 8.0 - 4.0) as f32, (number() * 8.0 - 4.0) as f32);
            let half = Vec2::new((number() + 0.5) as f32, (number() + 0.5) as f32);
            let delta = DVec2::new(number() * 10.0 - 5.0, number() * 10.0 - 5.0);
            let start = DVec2::new(number() * 16.0 - 8.0, number() * 16.0 - 8.0);
            let motion = DVec2::new(number() * 16.0 - 8.0, number() * 16.0 - 8.0);
            let obstacle = [
                start - DVec2::ONE,
                start + DVec2::new(1.0, -1.0),
                start + DVec2::ONE,
                start + DVec2::new(-1.0, 1.0),
            ];
            for body in [
                CombatVolume::aabb(Aabb::new(center, half)),
                CombatVolume::Circle {
                    center,
                    radius: half.x,
                },
            ] {
                let overlaps = |time: f64, slop: f64| {
                    let min = (start + motion * time - DVec2::ONE).max(DVec2::new(0.0, -3.0));
                    let max = start + motion * time + DVec2::ONE;
                    if min.x >= max.x || min.y >= max.y {
                        return false;
                    }
                    let at = center.as_dvec2() + delta * time;
                    match body {
                        CombatVolume::Circle { radius, .. } => {
                            at.distance(at.clamp(min, max)) < f64::from(radius) + slop
                        }
                        _ => {
                            at.x + f64::from(half.x) + slop > min.x
                                && at.y + f64::from(half.y) + slop > min.y
                                && at.x - f64::from(half.x) - slop < max.x
                                && at.y - f64::from(half.y) - slop < max.y
                        }
                    }
                };
                if overlaps(0.0, 1.0e-8) {
                    continue;
                }
                let hit = cast(&body, delta, &obstacle, motion, &cell).unwrap();
                let sampled = (1..=200)
                    .map(|step| f64::from(step) / 200.0)
                    .find(|time| overlaps(*time, -1.0e-6));
                if let Some(time) = sampled {
                    assert!(
                        hit.is_some_and(|hit| hit.time <= time + 1.0e-8),
                        "missed overlap at {time}: body={body:?}, delta={delta:?}, start={start:?}, motion={motion:?}, hit={:?}",
                        hit.map(|hit| (hit.time, hit.normal, hit.point))
                    );
                }
                if let Some(hit) = hit {
                    assert!(
                        overlaps((hit.time + 1.0e-7).min(1.0), 1.0e-6),
                        "false contact: body={body:?}, delta={delta:?}, start={start:?}, motion={motion:?}, hit={:?}",
                        (hit.time, hit.normal, hit.point)
                    );
                }
            }
        }
    }
}
