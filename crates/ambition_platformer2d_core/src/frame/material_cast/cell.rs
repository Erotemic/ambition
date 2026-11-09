//! A material cell with translating boundary lines and a translating map.

use bevy_math::{DVec2, DVec3};

use super::translation::MovingPlane;
use crate::Aabb;
use crate::frame::{NumericFault, Sim2};

/// One source region and its map during a cast. Time runs from zero to one.
#[derive(Clone, Debug)]
pub struct MaterialCell {
    pub path: Vec<u32>,
    pub(super) map: Sim2,
    pub(super) travel: DVec2,
    pub(super) planes: Vec<MovingPlane>,
    pub(super) time: std::ops::Range<f64>,
}

impl MaterialCell {
    pub(super) fn at(&self, time: f64) -> Result<Sim2, NumericFault> {
        self.map.shifted(self.travel * time)
    }

    /// Bounds of the source box and cell at all times. The box is only a
    /// broad phase envelope; contact uses the original primitive.
    pub(super) fn bounds(&self, body: Aabb, delta: DVec2) -> Result<Option<Aabb>, NumericFault> {
        let vertices = if self.planes.is_empty() {
            super::corners(body)
                .into_iter()
                .flat_map(|p| {
                    [
                        (p.as_dvec2() + delta * self.time.start).extend(self.time.start),
                        (p.as_dvec2() + delta * self.time.end).extend(self.time.end),
                    ]
                })
                .collect()
        } else {
            let mut planes = self.planes.clone();
            for (inward, offset) in [
                (DVec2::X, f64::from(body.min.x)),
                (-DVec2::X, -f64::from(body.max.x)),
                (DVec2::Y, f64::from(body.min.y)),
                (-DVec2::Y, -f64::from(body.max.y)),
            ] {
                planes.push(MovingPlane {
                    plane: crate::frame::thread::MaterialPlane { inward, offset },
                    rate: inward.dot(delta),
                });
            }
            prism(&planes, self.time.clone())?
        };
        let Some((lo, hi)) = vertices
            .into_iter()
            .map(|p| self.map.point_wide(p.truncate()) + self.travel * p.z)
            .try_fold(None, |bounds: Option<(DVec2, DVec2)>, p| {
                if !p.is_finite() {
                    return Err(NumericFault::Overflow);
                }
                Ok(Some(
                    bounds.map_or((p, p), |(lo, hi)| (lo.min(p), hi.max(p))),
                ))
            })?
        else {
            return Ok(None);
        };
        super::enclosing_bounds(lo, hi).map(Some)
    }
}

/// Vertices of a bounded cell in `(source_x, source_y, time)`.
fn prism(planes: &[MovingPlane], time: std::ops::Range<f64>) -> Result<Vec<DVec3>, NumericFault> {
    let mut constraints: Vec<_> = planes
        .iter()
        .map(|p| (p.plane.inward.extend(-p.rate), p.plane.offset))
        .collect();
    constraints.extend([(DVec3::Z, time.start), (-DVec3::Z, -time.end)]);
    if constraints
        .iter()
        .any(|(n, d)| !n.is_finite() || !d.is_finite())
    {
        return Err(NumericFault::NonFinite);
    }
    let mut vertices = Vec::new();
    for a in 0..constraints.len() {
        let (na, da) = constraints[a];
        for b in a + 1..constraints.len() {
            let (nb, db) = constraints[b];
            for c in b + 1..constraints.len() {
                let (nc, dc) = constraints[c];
                let det = na.dot(nb.cross(nc));
                if det == 0.0 {
                    continue;
                }
                let p = (nb.cross(nc) * da + nc.cross(na) * db + na.cross(nb) * dc) / det;
                if !p.is_finite() {
                    return Err(NumericFault::Overflow);
                }
                if constraints.iter().enumerate().all(|(index, (n, d))| {
                    if index == a || index == b || index == c {
                        return true;
                    }
                    let error = (p.abs().dot(n.abs()) + d.abs()) * f64::EPSILON * 32.0;
                    p.dot(*n) - d >= -error
                }) {
                    vertices.push(p);
                }
            }
        }
    }
    Ok(vertices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vec2;
    use crate::frame::thread::MaterialPlane;
    use bevy_math::Mat2;

    #[test]
    fn bounds_include_a_cell_that_exists_only_between_endpoint_poses() {
        let cell = MaterialCell {
            path: vec![],
            time: 0.0..1.0,
            map: Sim2::IDENTITY,
            travel: DVec2::ZERO,
            planes: vec![
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
            ],
        };
        let body = Aabb::new(Vec2::ZERO, Vec2::ONE);
        let bounds = cell.bounds(body, DVec2::ZERO).unwrap().unwrap();
        assert_eq!(bounds, body);
    }

    #[test]
    fn a_cell_and_material_with_equal_motion_stay_apart() {
        let cell = MaterialCell {
            path: vec![],
            time: 0.0..1.0,
            map: Sim2::IDENTITY,
            travel: DVec2::ZERO,
            planes: vec![MovingPlane {
                plane: MaterialPlane {
                    inward: DVec2::X,
                    offset: 2.0,
                },
                rate: 10.0,
            }],
        };
        let body = Aabb::new(Vec2::ZERO, Vec2::ONE);
        assert!(cell.bounds(body, DVec2::new(10.0, 0.0)).unwrap().is_none());
    }

    #[test]
    fn bounds_include_the_map_travel_with_scale_and_parity() {
        let body = Aabb::new(Vec2::new(3.0, -2.0), Vec2::new(2.0, 1.0));
        let delta = DVec2::new(4.0, 7.0);
        for scale in [0.001, 0.5, 2.0, 1000.0] {
            for parity in [-1.0, 1.0] {
                let rotation = Mat2::from_angle(0.37);
                let map = Sim2::new(
                    scale,
                    Mat2::from_cols(rotation.x_axis * parity, rotation.y_axis),
                    Vec2::new(100.0, -50.0),
                )
                .unwrap();
                let cell = MaterialCell {
                    path: vec![],
                    time: 0.0..1.0,
                    map,
                    travel: DVec2::new(41.0, -37.0),
                    planes: vec![],
                };
                let bounds = cell.bounds(body, delta).unwrap().unwrap();
                for time in [0.0, 0.125, 0.5, 0.875, 1.0] {
                    for corner in super::super::corners(body) {
                        let p =
                            map.point_wide(corner.as_dvec2() + delta * time) + cell.travel * time;
                        assert!(
                            p.cmpge(bounds.min.as_dvec2()).all()
                                && p.cmple(bounds.max.as_dvec2()).all()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn oblique_cell_bounds_contain_each_intermediate_clipped_region() {
        let mut seed = 1729_u64;
        let mut number = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 32) as u32) as f64 / f64::from(u32::MAX)
        };
        let body = Aabb::new(Vec2::ZERO, Vec2::new(2.0, 1.0));
        for index in 0..1000 {
            let rotation = Mat2::from_angle((number() * 6.0) as f32);
            let parity = if index % 2 == 0 { 1.0 } else { -1.0 };
            let scale = [0.001, 0.5, 2.0, 1000.0][index % 4];
            let map = Sim2::new(
                scale,
                Mat2::from_cols(rotation.x_axis * parity, rotation.y_axis),
                Vec2::new(100.0, -50.0),
            )
            .unwrap();
            let delta = DVec2::new(number() * 12.0 - 6.0, number() * 12.0 - 6.0);
            let cell = MaterialCell {
                path: vec![],
                time: 0.0..1.0,
                map,
                travel: DVec2::new(number() * 60.0 - 30.0, number() * 60.0 - 30.0),
                planes: (0..4)
                    .map(|_| {
                        let angle = number() * 6.0;
                        MovingPlane {
                            plane: MaterialPlane {
                                inward: DVec2::new(angle.cos(), angle.sin()),
                                offset: number() * 8.0 - 4.0,
                            },
                            rate: number() * 20.0 - 10.0,
                        }
                    })
                    .collect(),
            };
            let bounds = cell.bounds(body, delta).unwrap();
            for step in 0..=50 {
                let time = f64::from(step) / 50.0;
                let mut region = super::super::corners(body)
                    .map(|p| p.as_dvec2() + delta * time)
                    .to_vec();
                for plane in &cell.planes {
                    region = super::super::clip(
                        &region,
                        MaterialPlane {
                            offset: plane.plane.offset + plane.rate * time,
                            ..plane.plane
                        },
                    );
                }
                for point in region {
                    let bounds = bounds.expect("a nonempty region has bounds");
                    let point = map.point_wide(point) + cell.travel * time;
                    let error = (point.abs().max_element()
                        + bounds.min.abs().max_element() as f64
                        + bounds.max.abs().max_element() as f64)
                        * f64::EPSILON
                        * 128.0;
                    assert!(
                        (point + DVec2::splat(error))
                            .cmpge(bounds.min.as_dvec2())
                            .all()
                            && (point - DVec2::splat(error))
                                .cmple(bounds.max.as_dvec2())
                                .all(),
                        "case={index}, time={time}, point={point}, bounds={bounds:?}"
                    );
                }
            }
        }
    }
}
