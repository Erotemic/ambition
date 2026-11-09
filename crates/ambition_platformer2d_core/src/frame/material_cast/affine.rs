//! Material partitions over translating aperture frames.

use std::{collections::VecDeque, ops::Range};

use super::translation::MovingPlane;
use super::{DVec2, MaterialCast, MaterialCell, NumericFault, Refusal, RefusalReason, Sim2};
use crate::frame::thread::{MaterialPlane, MotionAdmission};

#[derive(Clone, Copy)]
struct Scalar {
    origin: f64,
    travel: f64,
}
impl Scalar {
    fn at(self, time: f64) -> f64 {
        self.origin + self.travel * time
    }
    fn root(self, value: f64, time: &Range<f64>, cuts: &mut Vec<f64>) {
        if self.travel != 0.0 {
            let at = (value - self.origin) / self.travel;
            if at > time.start && at < time.end {
                cuts.push(at);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Reference {
    origin: DVec2,
    travel: DVec2,
    along: Option<(DVec2, Scalar)>,
}
impl Reference {
    fn projection(self, origin: DVec2, travel: DVec2, axis: DVec2) -> Scalar {
        let mut result = Scalar {
            origin: (self.origin - origin).dot(axis),
            travel: (self.travel - travel).dot(axis),
        };
        if let Some((tangent, along)) = self.along {
            let factor = tangent.dot(axis);
            result.origin += factor * along.origin;
            result.travel += factor * along.travel;
        }
        result
    }
}


struct Link {
    key: u32,
    map: Sim2,
    travel: DVec2,
    origin: DVec2,
    shift: DVec2,
    normal: DVec2,
    tangent: DVec2,
    half: f64,
    exit: DVec2,
    exit_shift: DVec2,
    exit_tangent: DVec2,
    exit_half: f64,
    depth: Option<f64>,
}
impl Link {
    fn planes(&self, cell: &MaterialCell) -> Vec<MovingPlane> {
        let pull = |inward: DVec2, offset: f64| MovingPlane {
            plane: MaterialPlane {
                inward: cell.map.inverse_direction_wide(inward),
                offset: (offset - inward.dot(cell.map.translation())) / cell.map.scale_wide(),
            },
            rate: inward.dot(self.shift - cell.travel) / cell.map.scale_wide(),
        };
        let mut planes = vec![
            pull(-self.normal, -self.normal.dot(self.origin)),
            pull(self.tangent, self.tangent.dot(self.origin) - self.half),
            pull(-self.tangent, -self.tangent.dot(self.origin) - self.half),
        ];
        if let Some(depth) = self.depth {
            planes.push(pull(self.normal, self.normal.dot(self.origin) - depth));
        }
        planes
    }
}

#[derive(Clone)]
struct Work {
    cell: MaterialCell,
    root: Reference,
    forced: Option<u32>,
    /// A stay region retains the boundary of the column already cut from it.
    cut: Vec<u32>,
}

fn failure(path: &[u32], link: Option<u32>, reason: RefusalReason) -> Refusal {
    Refusal {
        path: path.to_vec(),
        link,
        reason,
    }
}

fn flip(p: MovingPlane) -> MovingPlane {
    MovingPlane {
        plane: MaterialPlane {
            inward: -p.plane.inward,
            offset: -p.plane.offset,
        },
        rate: -p.rate,
    }
}

fn windows(mut cuts: Vec<f64>, time: Range<f64>) -> Vec<Range<f64>> {
    cuts.extend([time.start, time.end]);
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    if cuts.len() == 1 {
        return vec![time];
    }
    cuts.windows(2).map(|p| p[0]..p[1]).collect()
}

fn simultaneous<'a>(
    body: &crate::CombatVolume,
    cells: impl Iterator<Item = &'a MaterialCell>,
) -> Result<usize, NumericFault> {
    let mut events = Vec::new();
    let mut points = Vec::new();
    for cell in cells {
        let Some(time) = super::temporal::occupied(body, &cell.planes, cell.time.clone())? else {
            continue;
        };
        if time.start == time.end {
            points.push(time.start);
        } else {
            events.extend([(time.start, 1_i32), (time.end, -1_i32)]);
        }
    }
    events.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut count = 0_i32;
    let mut most = 0_i32;
    for (_, change) in events {
        count += change;
        most = most.max(count);
    }
    points.sort_by(f64::total_cmp);
    let mut n = 0;
    let mut previous = None;
    for point in points {
        n = if previous == Some(point) { n + 1 } else { 1 };
        most = most.max(n);
        previous = Some(point);
    }
    Ok(most as usize)
}

/// Exact material cells for translating aperture frames.
pub(super) fn build(
    cast: &MaterialCast<'_>,
    admission: MotionAdmission<'_>,
) -> Result<Vec<MaterialCell>, Refusal> {
    let initial = |reason| failure(&[], None, reason);
    if admission.budget.max_pieces == 0 {
        return Err(initial(RefusalReason::PieceLimit));
    }
    if !cast
        .material
        .to_world
        .admits_scale_exponent(admission.budget.max_scale_exponent)
    {
        return Err(initial(RefusalReason::ScaleLimit));
    }
    let mut links = Vec::new();
    for link in admission.links {
        let numeric = |reason| failure(&[], Some(link.key), RefusalReason::Numeric(reason));
        link.enter_step.sample(link.enter, 0.0).map_err(numeric)?;
        link.exit_step.sample(link.exit, 0.0).map_err(numeric)?;
        let end = link.map(admission.convention).map_err(numeric)?;
        if link.depth.is_some_and(|d| !d.is_finite() || d < 0.0) {
            return Err(numeric(NumericFault::InvalidFrame));
        }
        let enter_shift = link.enter_step.displacement.as_dvec2();
        let exit_shift = link.exit_step.displacement.as_dvec2();
        let shift = exit_shift - end.direction_wide(enter_shift) * end.scale_wide();
        let normal = link.enter.frame.normal.as_dvec2().normalize();
        let exit_normal = link.exit.frame.normal.as_dvec2().normalize();
        links.push(Link {
            key: link.key,
            map: end
                .shifted(-shift * (1.0 - cast.time.start))
                .map_err(numeric)?,
            travel: shift * (cast.time.end - cast.time.start),
            origin: link.enter.frame.origin.as_dvec2() - enter_shift * (1.0 - cast.time.start),
            shift: enter_shift * (cast.time.end - cast.time.start),
            normal,
            tangent: DVec2::new(-normal.y, normal.x),
            half: f64::from(link.enter.half_length),
            exit: link.exit.frame.origin.as_dvec2() - exit_shift * (1.0 - cast.time.start),
            exit_shift: exit_shift * (cast.time.end - cast.time.start),
            exit_tangent: DVec2::new(-exit_normal.y, exit_normal.x),
            exit_half: f64::from(link.exit.half_length),
            depth: link.depth.map(f64::from),
        });
    }
    let first = Work {
        cell: MaterialCell {
            path: vec![],
            curve:None,
            map: cast.material.to_world,
            travel: cast.delta.as_dvec2(),
            planes: vec![],
            time: 0.0..cast.until,
        },
        root: Reference {
            origin: cast.root.as_dvec2(),
            travel: cast.delta.as_dvec2(),
            along: None,
        },
        forced: admission.first,
        cut: Vec::new(),
    };
    let mut queue = VecDeque::from([first]);
    let mut out = Vec::new();
    let mut steps = 0_usize;
    let limit = 8_usize
        .saturating_mul(usize::from(admission.budget.max_pieces))
        .saturating_mul(links.len().max(1));
    while let Some(work) = queue.pop_front() {
        let Work {
            cell,
            root,
            forced,
            cut,
        } = work;
        let fail = |link, reason| failure(&cell.path, link, reason);
        steps += 1;
        if steps > limit {
            return Err(fail(None, RefusalReason::WorkLimit));
        }
        let ranks = links
            .iter()
            .map(|l| root.projection(l.origin, l.shift, l.normal))
            .collect::<Vec<_>>();
        let mut possible = Vec::new();
        let mut cuts = Vec::new();
        for (i, link) in links.iter().enumerate() {
            if cut.contains(&link.key) {
                continue;
            }
            if Some(link.key) != forced
                && ranks[i].at(cell.time.start).max(ranks[i].at(cell.time.end)) <= 0.0
            {
                continue;
            }
            steps += 1;
            if steps > limit {
                return Err(fail(Some(link.key), RefusalReason::WorkLimit));
            }
            let planes = link.planes(&cell);
            let mut through = cell.planes.clone();
            through.extend_from_slice(&planes);
            if let Some(time) =
                super::temporal::closure(cast.material.domain, &through, cell.time.clone())
                    .map_err(|e| fail(Some(link.key), RefusalReason::Numeric(e)))?
            {
                if time.start - cell.time.start > f64::EPSILON * 64.0 {
                    cuts.push(time.start);
                }
                if cell.time.end - time.end > f64::EPSILON * 64.0 {
                    cuts.push(time.end);
                }
                possible.push(i);
            }
        }
        for (position, a) in possible.iter().enumerate() {
            let rank = ranks[*a];
            rank.root(0.0, &cell.time, &mut cuts);
            for b in &possible[..position] {
                let other = ranks[*b];
                steps += 1;
                if steps > limit {
                    return Err(fail(None, RefusalReason::WorkLimit));
                }
                Scalar {
                    origin: rank.origin - other.origin,
                    travel: rank.travel - other.travel,
                }
                .root(0.0, &cell.time, &mut cuts);
            }
        }
        let spans = windows(cuts, cell.time.clone());
        if spans.len() > 1 {
            queue.extend(spans.into_iter().map(|time| Work {
                cell: MaterialCell {
                    time,
                    ..cell.clone()
                },
                root,
                forced,
                cut: cut.clone(),
            }));
            continue;
        }
        let middle = cell.time.start + (cell.time.end - cell.time.start) * 0.5;
        let mut order = possible
            .into_iter()
            .filter(|i| Some(links[*i].key) == forced || ranks[*i].at(middle) > 0.0)
            .collect::<Vec<_>>();
        order.sort_by(|a, b| {
            let rank = |i: usize| {
                if Some(links[i].key) == forced {
                    f64::NEG_INFINITY
                } else {
                    ranks[i].at(middle)
                }
            };
            rank(*a)
                .total_cmp(&rank(*b))
                .then(links[*a].key.cmp(&links[*b].key))
        });
        let mut next = None;
        for i in order {
            steps += 1;
            if steps > limit {
                return Err(fail(Some(links[i].key), RefusalReason::WorkLimit));
            }
            let planes = links[i].planes(&cell);
            let mut through = cell.planes.clone();
            through.extend_from_slice(&planes);
            if let Some(mut time) =
                super::temporal::closure(cast.material.domain, &through, cell.time.clone())
                    .map_err(|e| fail(Some(links[i].key), RefusalReason::Numeric(e)))?
            {
                let error = f64::EPSILON * 64.0;
                if time.start - cell.time.start <= error {
                    time.start = cell.time.start;
                }
                if cell.time.end - time.end <= error {
                    time.end = cell.time.end;
                }
                next = Some((i, planes, time));
                break;
            }
        }
        let Some((i, planes, active)) = next else {
            out.push(cell);
            continue;
        };
        let link = &links[i];
        if active != cell.time && active.start < active.end {
            let spans = windows(vec![active.start, active.end], cell.time.clone());
            queue.extend(spans.into_iter().map(|time| Work {
                cell: MaterialCell {
                    time,
                    ..cell.clone()
                },
                root,
                forced,
                cut: cut.clone(),
            }));
            continue;
        }
        if cell.path.len() >= usize::from(admission.budget.max_hops) {
            return Err(fail(Some(link.key), RefusalReason::HopLimit));
        }
        if !cell
            .map
            .admits_scale_product(link.map, admission.budget.max_scale_exponent)
        {
            return Err(fail(Some(link.key), RefusalReason::ScaleLimit));
        }
        let numeric = |e| fail(Some(link.key), RefusalReason::Numeric(e));
        let mut candidates = vec![
            vec![flip(planes[0])],
            vec![planes[0], flip(planes[1])],
            vec![planes[0], flip(planes[2])],
        ];
        if planes.len() == 4 {
            candidates.push(vec![planes[1], planes[2], flip(planes[3])]);
        }
        let mut children = Vec::new();
        let mut stay_cut = cut;
        stay_cut.push(link.key);
        for more in candidates {
            let mut stay = cell.clone();
            stay.planes.extend(more);
            if let Some(time) =
                super::temporal::closure(cast.material.domain, &stay.planes, stay.time.clone())
                    .map_err(numeric)?
            {
                stay.time = time;
                children.push(Work {
                    cell: stay,
                    root,
                    forced: None,
                    cut: stay_cut.clone(),
                });
            }
        }
        let mut carried = cell.clone();
        carried.time = active;
        carried.planes.extend(planes);
        carried.path.push(link.key);
        carried.map = cell.map.then(link.map).map_err(numeric)?;
        carried.travel = link.map.direction_wide(cell.travel) * link.map.scale_wide() + link.travel;
        let local = root.projection(link.origin, link.shift, link.tangent);
        let along_factor =
            link.map.direction_wide(link.tangent).dot(link.exit_tangent) * link.map.scale_wide();
        let along = Scalar {
            origin: local.origin * along_factor,
            travel: local.travel * along_factor,
        };
        let mut clamp = Vec::new();
        along.root(-link.exit_half, &carried.time, &mut clamp);
        along.root(link.exit_half, &carried.time, &mut clamp);
        for time in windows(clamp, carried.time.clone()) {
            let at = along.at(time.start + (time.end - time.start) * 0.5);
            let value = if at < -link.exit_half {
                Scalar {
                    origin: -link.exit_half,
                    travel: 0.0,
                }
            } else if at > link.exit_half {
                Scalar {
                    origin: link.exit_half,
                    travel: 0.0,
                }
            } else {
                along
            };
            children.push(Work {
                cell: MaterialCell {
                    time,
                    ..carried.clone()
                },
                root: Reference {
                    origin: link.exit,
                    travel: link.exit_shift,
                    along: Some((link.exit_tangent, value)),
                },
                forced: None,
                cut: Vec::new(),
            });
        }
        if simultaneous(
            cast.material.domain,
            out.iter()
                .chain(queue.iter().map(|w| &w.cell))
                .chain(children.iter().map(|w| &w.cell)),
        )
        .map_err(numeric)?
            > usize::from(admission.budget.max_pieces)
        {
            return Err(fail(Some(link.key), RefusalReason::PieceLimit));
        }
        queue.extend(children);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::MaterialCastCells;
    use super::*;
    use crate::frame::thread::{MaterialPrimitive, PieceBudget, PortalLink};
    use crate::frame::{ApertureStep, MapConvention, PortalAperture, PortalFrame};
    use crate::{Aabb, CombatVolume, Vec2};

    fn links(
        entry: Vec2,
        entry_shift: Vec2,
        exit: Vec2,
        exit_shift: Vec2,
        half: f32,
    ) -> [PortalLink; 2] {
        let a = PortalAperture {
            frame: PortalFrame::fixed(entry, -Vec2::X),
            half_length: half,
        };
        let b = PortalAperture {
            frame: PortalFrame::fixed(exit, Vec2::X),
            half_length: half,
        };
        PortalLink::both_ways_with_steps(
            0,
            [
                (a, ApertureStep::ending_at(a.frame, entry_shift, 0.0)),
                (b, ApertureStep::ending_at(b.frame, exit_shift, 0.0)),
            ],
        )
    }

    fn cast<'a>(
        body: &'a CombatVolume,
        delta: Vec2,
        links: &[PortalLink],
        pieces: u16,
    ) -> Result<MaterialCastCells<'a>, Refusal> {
        let request = MaterialCast::new(
            MaterialPrimitive::world(body),
            Vec2::new(-6.0, 0.0),
            delta,
            0.0..1.0,
        );
        request.cells(Some(MotionAdmission {
            links,
            first: None,
            budget: PieceBudget {
                max_pieces: pieces,
                ..PieceBudget::COMPOSED
            },
            convention: MapConvention::Reflection,
        }))
    }

    #[test]
    fn a_translating_host_and_its_aperture_share_the_boundary_for_the_whole_step() {
        let links = links(
            Vec2::new(20.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(100.0, 0.0),
            Vec2::ZERO,
            40.0,
        );
        for body in [
            CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::splat(10.0))),
            CombatVolume::circle(Vec2::new(-6.0, 0.0), 10.0),
        ] {
            let cells = cast(&body, Vec2::new(20.0, 0.0), &links, 32).unwrap();
            let host = super::super::corners(Aabb::new(Vec2::new(25.0, 0.0), Vec2::new(5.0, 50.0)));
            for cell in cells.parts() {
                assert!(
                    cells
                        .against(cell, &host, Vec2::new(20.0, 0.0))
                        .unwrap()
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn a_translating_exit_carries_a_far_part_into_an_obstacle_between_endpoint_poses() {
        let links = links(
            Vec2::ZERO,
            Vec2::ZERO,
            Vec2::new(120.0, 0.0),
            Vec2::new(20.0, 0.0),
            40.0,
        );
        let body = CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::splat(10.0)));
        let cells = cast(&body, Vec2::ZERO, &links, 32).unwrap();
        let target = super::super::corners(Aabb::new(Vec2::new(111.0, 0.0), Vec2::new(1.0, 20.0)));
        let hit = cells
            .parts()
            .iter()
            .filter_map(|part| cells.against(part, &target, Vec2::ZERO).unwrap())
            .min_by(|a, b| a.toi.total_cmp(&b.toi))
            .unwrap();
        assert_eq!(hit.path, [0]);
        assert!((hit.toi - 0.3).abs() < 1.0e-12, "{hit:?}");
        assert!((hit.world_point.x - 110.0).abs() < 0.00001);
        assert!(hit.map.point(hit.point).unwrap().distance(hit.world_point) < 0.00001);
    }

    #[test]
    fn a_sliding_opening_keeps_the_material_that_exists_only_inside_the_step() {
        let links = links(
            Vec2::new(0.0, 40.0),
            Vec2::new(0.0, 80.0),
            Vec2::new(100.0, 0.0),
            Vec2::ZERO,
            5.0,
        );
        for body in [
            CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::splat(10.0))),
            CombatVolume::circle(Vec2::new(-6.0, 0.0), 10.0),
        ] {
            let cells = cast(&body, Vec2::ZERO, &links, 4).unwrap();
            assert!(
                cells.parts().len() > 4,
                "time windows do not consume simultaneous piece slots"
            );
            assert_eq!(simultaneous(&body, cells.parts().iter()).unwrap(), 4);
            let far = cells
                .parts()
                .iter()
                .filter(|cell| cell.path == [0])
                .collect::<Vec<_>>();
            assert!(!far.is_empty());
            assert!(
                far.iter()
                    .all(|cell| cell.time.start > 0.0 && cell.time.end < 1.0)
            );
            assert!(far.iter().any(|cell| cell.time.contains(&0.5)));
            assert!(matches!(
                cast(&body, Vec2::ZERO, &links, 3),
                Err(Refusal {
                    reason: RefusalReason::PieceLimit,
                    ..
                })
            ));
            for step in 0..80 {
                let time = (f64::from(step) + 0.314159) / 80.0;
                for point in 0..100 {
                    let p = DVec2::new(
                        -15.9 + f64::from(point % 10) * 1.99,
                        -9.7 + f64::from(point / 10) * 1.97,
                    );
                    if matches!(body, CombatVolume::Circle { .. })
                        && p.distance(DVec2::new(-6.0, 0.0)) >= 10.0
                    {
                        continue;
                    }
                    let count = cells
                        .parts()
                        .iter()
                        .filter(|cell| {
                            cell.time.contains(&time)
                                && cell.planes.iter().all(|plane| {
                                    p.dot(plane.plane.inward)
                                        > plane.plane.offset + plane.rate * time
                                })
                        })
                        .count();
                    assert_eq!(count, 1, "t={time}, point={p}, body={body:?}");
                }
            }
        }
    }

    #[test]
    fn a_common_frame_boost_preserves_the_temporal_partition() {
        let body = CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::splat(10.0)));
        let links = links(
            Vec2::new(8.0, 3.0),
            Vec2::new(8.0, 3.0),
            Vec2::new(120.0, -10.0),
            Vec2::new(20.0, -10.0),
            20.0,
        );
        let boost = Vec2::new(14.0, -9.0);
        let mut moved = links;
        for link in &mut moved {
            for (end, step) in [
                (&mut link.enter, &mut link.enter_step),
                (&mut link.exit, &mut link.exit_step),
            ] {
                end.frame.origin += boost;
                end.frame.pivot += boost;
                step.displacement += boost;
            }
        }
        let admission = |links| MotionAdmission {
            links,
            first: None,
            budget: PieceBudget::COMPOSED,
            convention: MapConvention::Reflection,
        };
        let request = MaterialCast::new(
            MaterialPrimitive::world(&body),
            Vec2::new(-6.0, 0.0),
            Vec2::new(3.0, 1.0),
            0.25..0.75,
        );
        let boosted = MaterialCast::new(
            MaterialPrimitive {
                domain: &body,
                to_world: Sim2::new(1.0, bevy_math::Mat2::IDENTITY, boost * 0.25).unwrap(),
            },
            request.root + boost * 0.25,
            request.delta + boost * 0.5,
            request.time.clone(),
        );
        let base = request.cells(Some(admission(&links))).unwrap();
        let moved = boosted.cells(Some(admission(&moved))).unwrap();
        for time in [0.0, 0.125, 0.5, 0.875, 0.999] {
            for point in 0..100 {
                let p = DVec2::new(
                    -15.9 + f64::from(point % 10) * 1.99,
                    -9.7 + f64::from(point / 10) * 1.97,
                );
                let locate = |cells: &MaterialCastCells<'_>| {
                    cells
                        .parts()
                        .iter()
                        .find(|c| {
                            c.time.contains(&time)
                                && c.planes.iter().all(|plane| {
                                    p.dot(plane.plane.inward)
                                        > plane.plane.offset + plane.rate * time
                                })
                        })
                        .map(|c| (c.path.clone(), c.at(time).unwrap().point_wide(p)))
                };
                let (a, pa) = locate(&base).unwrap();
                let (b, pb) = locate(&moved).unwrap();
                assert_eq!(a, b);
                assert!(pb.distance(pa + boost.as_dvec2() * (0.25 + 0.5 * time)) < 0.00001);
            }
        }
    }
}
