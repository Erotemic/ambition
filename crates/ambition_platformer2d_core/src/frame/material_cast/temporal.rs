//! The time intervals in which an affine cell contains material.

use std::ops::Range;

use super::translation::{self, MovingPlane};
use super::{CombatVolume, NumericFault};
#[cfg(test)]
use super::DVec2;

fn holds(body: &CombatVolume, planes: &[MovingPlane], time: f64, interior: bool) -> bool {
    let mut region = body
        .polygon()
        .unwrap_or_else(|| super::corners(body.bounds()).to_vec())
        .into_iter()
        .map(|p| p.as_dvec2())
        .collect::<Vec<_>>();
    for plane in planes {
        region = super::clip(
            &region,
            super::MaterialPlane {
                offset: plane.plane.offset + plane.rate * time,
                ..plane.plane
            },
        );
        if region.is_empty() {
            return false;
        }
    }
    if interior && !crate::frame::thread::has_area(&region) {
        return false;
    }
    match body {
        CombatVolume::Circle { center, radius } => {
            let center = center.as_dvec2();
            if planes
                .iter()
                .all(|p| center.dot(p.plane.inward) >= p.plane.offset + p.rate * time)
            {
                return true;
            }
            let distance = super::edges(&region)
                .map(|(a, b)| center.distance(super::closest_on_segment(center, a, b)))
                .fold(f64::INFINITY, f64::min);
            if interior {
                distance < f64::from(*radius)
            } else {
                distance <= f64::from(*radius)
            }
        }
        _ => !region.is_empty(),
    }
}

/// The intersection is convex in source position and time. Its projection on
/// time is one interval. A boundary touch alone contains no material area.
pub(super) fn occupied(
    body: &CombatVolume,
    planes: &[MovingPlane],
    time: Range<f64>,
) -> Result<Option<Range<f64>>, NumericFault> {
    window(body, planes, time, true)
}

/// Boundary strata can carry contact even when they contain no material area.
pub(super) fn closure(
    body: &CombatVolume,
    planes: &[MovingPlane],
    time: Range<f64>,
) -> Result<Option<Range<f64>>, NumericFault> {
    window(body, planes, time, false)
}

fn window(
    body: &CombatVolume,
    planes: &[MovingPlane],
    time: Range<f64>,
    interior: bool,
) -> Result<Option<Range<f64>>, NumericFault> {
    let first = holds(body, planes, time.start, interior);
    let last = holds(body, planes, time.end, interior);
    if time.start == time.end {
        return Ok(first.then_some(time));
    }
    if first && last {
        return Ok(Some(time));
    }
    let template = super::corners(body.bounds()).map(|p| p.as_dvec2());
    let span = time.end - time.start;
    let lo = if first {
        time.start
    } else {
        let Some(hit) = translation::cast_closure(
            body,
            &template,
            planes.iter().map(|p| p.during(time.clone())),
        )?
        else {
            return Ok(None);
        };
        time.start + span * hit.time
    };
    let hi = if last {
        time.end
    } else {
        let Some(hit) = translation::cast_closure(
            body,
            &template,
            planes.iter().map(|p| MovingPlane {
                plane: super::MaterialPlane {
                    offset: p.plane.offset + p.rate * time.end,
                    ..p.plane
                },
                rate: -p.rate * span,
            }),
        )?
        else {
            return Ok(None);
        };
        time.end - span * hit.time
    };
    Ok(
        ((lo < hi || !interior && lo == hi) && holds(body, planes, lo + (hi - lo) * 0.5, interior))
            .then_some(lo..hi),
    )
}

#[cfg(test)]
mod tests {
    use super::super::MaterialPlane;
    use super::*;
    use crate::{Aabb, Vec2};

    #[test]
    fn material_can_exist_only_between_the_endpoint_poses() {
        let planes = [
            MovingPlane {
                plane: MaterialPlane {
                    inward: DVec2::X,
                    offset: -4.0,
                },
                rate: 8.0,
            },
            MovingPlane {
                plane: MaterialPlane {
                    inward: -DVec2::X,
                    offset: 2.0,
                },
                rate: -8.0,
            },
        ];
        for body in [
            CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::ONE)),
            CombatVolume::circle(Vec2::ZERO, 1.0),
        ] {
            let time = occupied(&body, &planes, 0.0..1.0).unwrap().unwrap();
            assert!((time.start - 0.125).abs() < 1.0e-12, "{time:?}");
            assert!((time.end - 0.625).abs() < 1.0e-12, "{time:?}");
        }
    }

    #[test]
    fn a_circle_does_not_fill_its_bounding_corner() {
        let plane = |inward, offset| MovingPlane {
            plane: MaterialPlane { inward, offset },
            rate: 0.0,
        };
        let body = CombatVolume::circle(Vec2::ZERO, 1.0);
        assert!(
            occupied(
                &body,
                &[plane(DVec2::X, 0.8), plane(DVec2::Y, 0.8)],
                0.0..1.0
            )
            .unwrap()
            .is_none()
        );
        assert!(
            occupied(&body, &[plane(DVec2::X, 1.0)], 0.0..1.0)
                .unwrap()
                .is_none()
        );
        assert!(
            closure(
                &body,
                &[plane(DVec2::X, 0.8), plane(DVec2::Y, 0.8)],
                0.0..1.0
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(
            closure(&body, &[plane(DVec2::X, 1.0)], 0.0..1.0).unwrap(),
            Some(0.0..1.0)
        );
        let arriving = MovingPlane {
            plane: MaterialPlane {
                inward: DVec2::X,
                offset: 2.0,
            },
            rate: -1.0,
        };
        assert_eq!(
            closure(&body, &[arriving], 0.0..1.0).unwrap(),
            Some(1.0..1.0)
        );
    }
}
