//! Ray reach through directed portal links.

use bevy_math::DVec2;

use super::{raycast_surfaces_wide, SolidWorldQuery};
use crate::frame::thread::{PieceBudget, PortalLink, Refusal, RefusalReason};
use crate::frame::{MapConvention, NumericFault, Sim2};
use crate::Vec2;

/// The ray starts in the world chart. Reach is measured in that chart.
#[derive(Clone, Copy, Debug)]
pub struct PortalRay {
    pub origin: Vec2,
    pub direction: Vec2,
    pub reach: f32,
}

/// Limits for one ray path. Scale limits use the material policy.
#[derive(Clone, Copy, Debug)]
pub struct RayBudget {
    pub max_hops: u32,
    pub max_scale_exponent: u8,
}

impl From<PieceBudget> for RayBudget {
    fn from(value: PieceBudget) -> Self {
        Self {
            max_hops: u32::from(value.max_hops),
            max_scale_exponent: value.max_scale_exponent,
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
    if !ray.origin.is_finite() || !ray.direction.is_finite() || !ray.reach.is_finite() {
        return Err(refusal(
            &path,
            None,
            RefusalReason::Numeric(NumericFault::NonFinite),
        ));
    }
    if ray.reach < 0.0 {
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
        let solid = raycast_surfaces_wide(world, origin, direction, reach, include_one_way, true);
        let solid_distance = solid.as_ref().map_or(reach, |hit| hit.distance);
        let mut nearest: Option<(f64, &PortalLink)> = None;
        for link in links {
            let enter = link.enter;
            let normal = enter.frame.normal.as_dvec2().normalize_or_zero();
            let facing = direction.dot(normal);
            if facing >= 0.0 {
                continue;
            }
            let t = (enter.frame.origin.as_dvec2() - origin).dot(normal) / facing;
            if !t.is_finite() || t < 0.0 || t > solid_distance {
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
        let Some((distance, link)) = nearest else {
            return Ok(ApertureTrace {
                hit: solid
                    .map(|hit| {
                        Sim2::IDENTITY
                            .point_from_wide(origin + direction * hit.distance)
                            .map(|point| (point, hit.normal))
                    })
                    .transpose()
                    .map_err(numeric)?,
                distance: (travelled + solid_distance / map.scale_wide()).min(f64::from(ray.reach))
                    as f32,
                path,
            });
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
