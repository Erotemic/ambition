//! Ray reach through directed portal links.

use bevy_math::DVec2;

use super::SolidWorldQuery;
use crate::frame::thread::{PieceBudget, PortalLink, Refusal, RefusalReason};
use crate::frame::{MapConvention, NumericFault, Sim2};
use crate::Vec2;

/// The ray starts in the world chart. Reach is measured in that chart.
#[derive(Clone, Copy, Debug)]
pub struct PortalRay {
    pub origin: Vec2,
    pub direction: Vec2,
    pub reach: f32,
    /// Full band width in the initial chart. Zero is a point ray.
    pub width: f32,
}

/// Limits for one ray path. Scale limits use the material policy.
#[derive(Clone, Copy, Debug)]
pub struct RayBudget {
    pub max_hops: u32,
    pub max_scale_exponent: u8,
    pub max_pieces: u16,
}

impl From<PieceBudget> for RayBudget {
    fn from(value: PieceBudget) -> Self {
        Self {
            max_hops: u32::from(value.max_hops),
            max_scale_exponent: value.max_scale_exponent,
            max_pieces: value.max_pieces,
        }
    }
}

/// The final-chart hit and the distance in the ray's initial chart.
#[derive(Clone, Debug, PartialEq)]
pub struct ApertureTrace {
    pub hit: Option<(Vec2, Vec2)>,
    pub distance: f32,
    pub path: Vec<u32>,
}

/// Trace against solid surfaces and each directed aperture. An aperture wins
/// a tie with its host. A refusal publishes no partial ray.
pub fn trace_through_apertures<W: SolidWorldQuery + ?Sized>(
    world: &W,
    links: &[PortalLink],
    ray: PortalRay,
    include_one_way: bool,
    budget: RayBudget,
    convention: MapConvention,
) -> Result<ApertureTrace, Refusal> {
    let mut path = Vec::new();
    let refusal = |path: &[u32], link, reason| Refusal {
        path: path.to_vec(),
        link,
        reason,
    };
    if !ray.origin.is_finite() || !ray.direction.is_finite() || !ray.reach.is_finite() || !ray.width.is_finite() {
        return Err(refusal(
            &path,
            None,
            RefusalReason::Numeric(NumericFault::NonFinite),
        ));
    }
    if ray.reach < 0.0 || ray.width < 0.0 {
        return Err(refusal(
            &path,
            None,
            RefusalReason::Numeric(NumericFault::InvalidScale),
        ));
    }
    let mut direction = ray.direction.as_dvec2().normalize_or_zero();
    if direction == DVec2::ZERO || ray.reach == 0.0 {
        return Ok(ApertureTrace {
            hit: None,
            distance: 0.0,
            path,
        });
    }
    let mut origin = ray.origin.as_dvec2();
    let mut map = Sim2::IDENTITY;
    let mut travelled = 0.0;
    let mut first = None;
    loop {
        let remaining = (f64::from(ray.reach) - travelled).max(0.0);
        let reach = remaining * map.scale_wide();
        let numeric = |reason| refusal(&path, None, RefusalReason::Numeric(reason));
        let end = origin + direction * reach;
        let end_narrow = Sim2::IDENTITY.point_from_wide(end).map_err(numeric)?;
        let origin_narrow = Sim2::IDENTITY.point_from_wide(origin).map_err(numeric)?;
        if reach > 0.0 && end_narrow == origin_narrow {
            return Err(numeric(NumericFault::Precision));
        }
        let mut nearest: Option<(f64, &PortalLink)> = None;
        for link in links {
            let enter = link.enter;
            let normal = enter.frame.normal.as_dvec2().normalize_or_zero();
            let facing = direction.dot(normal);
            if facing >= 0.0 {
                continue;
            }
            let t = (enter.frame.origin.as_dvec2() - origin).dot(normal) / facing;
            if !t.is_finite() || t < 0.0 || t > reach {
                continue;
            }
            let at = origin + direction * t;
            let tangent = DVec2::new(-normal.y, normal.x);
            if (at - enter.frame.origin.as_dvec2()).dot(tangent).abs()
                > f64::from(enter.half_length)
            {
                continue;
            }
            if nearest.is_none_or(|(best, other)| {
                t.total_cmp(&best).then(link.key.cmp(&other.key)).is_lt()
            }) {
                nearest = Some((t, link));
            }
        }
        let leg_end = nearest.map_or(reach, |(distance, _)| distance);
        let section = crate::CombatVolume::aabb(crate::Aabb::new(Vec2::ZERO, Vec2::new(0.0, ray.width * 0.5)));
        let axes = bevy_math::Mat2::from_cols(direction.as_vec2(), DVec2::new(-direction.y, direction.x).as_vec2());
        let source = Sim2::new(map.scale(), axes, origin_narrow).map_err(numeric)?;
        let delta = Sim2::IDENTITY.vector_from_wide(direction * reach).map_err(numeric)?;
        let cast = crate::frame::material_cast::MaterialCast::new(
            crate::frame::thread::MaterialPrimitive { domain: &section, to_world: source },
            origin_narrow, delta, 1.0..1.0,
        ).with_initial_overlap(crate::frame::material_cast::InitialOverlap::Report)
            .prefix(if reach == 0.0 { 0.0 } else { leg_end / reach })?;
        let cells = cast.cells(Some(crate::frame::thread::MotionAdmission {
            links, first,
            budget: PieceBudget {
                max_hops: budget.max_hops.saturating_add(path.len() as u32).min(u32::from(u8::MAX)) as u8,
                max_pieces: budget.max_pieces, max_scale_exponent: budget.max_scale_exponent,
            }, convention,
        })).map_err(|mut failure| { failure.path.splice(0..0, path.iter().copied()); failure })?;
        for part in cells.parts() {
            let mut carried = path.clone();
            for key in &part.path {
                let inverse = carried.last().and_then(|last| links.iter().find(|link| link.key == *last))
                    .is_some_and(|last| links.iter().any(|link| link.key == *key && link.enter == last.exit && link.exit == last.enter));
                if inverse { carried.pop(); }
                else {
                    if carried.len() as u64 >= u64::from(budget.max_hops) {
                        return Err(refusal(&carried, Some(*key), RefusalReason::HopLimit));
                    }
                    carried.push(*key);
                }
            }
        }
        let mut solid: Option<crate::frame::material_cast::MaterialCastHit> = None;
        let mut failed = None;
        for part in cells.parts() {
            let mut consider = |result: Result<Option<crate::frame::material_cast::MaterialCastHit>, Refusal>| {
                match result {
                    Ok(Some(hit)) if solid.as_ref().is_none_or(|old| hit.toi.total_cmp(&old.toi)
                        .then(hit.path.cmp(&old.path)).is_lt()) => solid = Some(hit),
                    Err(error) if failed.is_none() => failed = Some(error),
                    _ => {}
                }
            };
            world.for_each_solid_aabb(include_one_way, &mut |bounds| {
                let obstacle = [bounds.min, Vec2::new(bounds.max.x, bounds.min.y), bounds.max, Vec2::new(bounds.min.x, bounds.max.y)];
                consider(cells.against(part, &obstacle, Vec2::ZERO));
            });
            world.for_each_line(&mut |a, b| {
                let edge = (b-a).as_dvec2();
                let normal = DVec2::new(edge.y, -edge.x).normalize_or_zero();
                consider(cells.against_rigid(part, &[a,b], Vec2::ZERO, 0.0, Vec2::ZERO)
                    .map(|hit| hit.filter(|hit| hit.map.direction_wide(hit.normal.as_dvec2()).dot(normal) > 0.5)));
            });
        }
        if let Some(mut failure) = failed {
            failure.path.splice(0..0, path.iter().copied());
            return Err(failure);
        }
        if let Some(hit) = solid {
            let distance = (travelled + hit.toi * reach / map.scale_wide()).min(f64::from(ray.reach));
            let rounded = distance as f32;
            return Ok(ApertureTrace {
                hit: Some((hit.world_point, hit.map.direction(hit.normal).normalize_or_zero())),
                distance: if f64::from(rounded) > distance { rounded.next_down() } else { rounded },
                path: path.iter().copied().chain(hit.path).collect(),
            });
        }
        let Some((distance, link)) = nearest else {
            return Ok(ApertureTrace { hit: None, distance: ray.reach, path });
        };
        let at_link = |reason| refusal(&path, Some(link.key), reason);
        if path.len() as u64 >= u64::from(budget.max_hops) {
            return Err(at_link(RefusalReason::HopLimit));
        }
        let step = link
            .map(convention)
            .map_err(|reason| at_link(RefusalReason::Numeric(reason)))?;
        if !map.admits_scale_product(step, budget.max_scale_exponent) {
            return Err(at_link(RefusalReason::ScaleLimit));
        }
        let next_map = map
            .then(step)
            .map_err(|reason| at_link(RefusalReason::Numeric(reason)))?;
        let entry = origin + direction * distance;
        let mapped = step.point_wide(entry);
        let exit_normal = link.exit.frame.normal.as_dvec2().normalize();
        // The crossing lies on the face. Keep round-off out of the host.
        let next_origin =
            mapped - exit_normal * (mapped - link.exit.frame.origin.as_dvec2()).dot(exit_normal);
        Sim2::IDENTITY
            .point_from_wide(next_origin)
            .map_err(|reason| at_link(RefusalReason::Numeric(reason)))?;
        travelled += distance / map.scale_wide();
        origin = next_origin;
        direction = step.direction_wide(direction).normalize();
        map = next_map;
        path.push(link.key);
        first = links.iter().find(|other| other.enter == link.exit && other.exit == link.enter).map(|other| other.key);
        if travelled >= f64::from(ray.reach) {
            return Ok(ApertureTrace {
                hit: None,
                distance: ray.reach,
                path,
            });
        }
    }
}

#[cfg(test)]
mod tests;
