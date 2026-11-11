//! Convex source cells with recorded curved maps and conditional aperture cuts.

use super::translation::PlaneMotion;
use super::{
    CombatVolume, DVec2, MaterialCast, MaterialCell, MaterialPlane, NumericFault, Refusal,
    RefusalReason, Sim2,
};
use crate::frame::thread::{MotionAdmission, PieceBudget, PortalLink};
use crate::frame::{MapConvention, motion::{PreparedApertureStep, WideAperture}};
use bevy_math::DMat2;
use std::{collections::VecDeque, ops::Range, sync::{Arc, OnceLock}};

const MAX_WORK: usize = 16_384;
fn perp(p: DVec2) -> DVec2 {
    DVec2::new(-p.y, p.x)
}
fn round(magnitude: f64) -> f64 {
    magnitude * f64::EPSILON * 128.0
}

#[derive(Clone, Copy, Debug)]
struct Pose {
    enter: WideAperture,
    exit: WideAperture,
    map: Sim2,
    velocity: DVec2,
    spin: f64,
}

#[derive(Debug)]
pub(super) struct Scene {
    body: CombatVolume,
    source: super::SourceMotion,
    time: Range<f64>,
    until: f64,
    links: Vec<PortalLink>,
    prepared_steps: OnceLock<Vec<Result<(PreparedApertureStep, PreparedApertureStep), NumericFault>>>,
    first: Option<u32>,
    budget: PieceBudget,
    convention: MapConvention,
}

#[derive(Clone, Copy)]
struct State {
    time: f64,
    evaluations: usize,
    map: Sim2,
    velocity: DVec2,
    spin: f64,
    reference: DVec2,
    reference_velocity: DVec2,
    variation: f64,
    reference_variation: f64,
    on: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
enum Kind {
    Through,
    Stay(u8),
}

#[derive(Clone, Copy, Debug)]
struct Clause {
    stage: usize,
    link: usize,
    kind: Kind,
    prior: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
struct Hole {
    link: usize,
    exit: bool,
    side: u8,
    admitted: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Truth {
    Yes,
    No,
    Maybe,
}

fn selected_planes(
    kind: Kind,
    n: DVec2,
    tangent: DVec2,
    half: f64,
    depth: Option<f64>,
) -> Vec<(DVec2, f64)> {
    let through = [
        (-n, 0.0),
        (tangent, -half),
        (-tangent, -half),
        (n, -depth.unwrap_or(0.0)),
    ];
    let indices: Vec<(usize, f64)> = match kind {
        Kind::Through => (0..if depth.is_some() { 4 } else { 3 })
            .map(|i| (i, 1.0))
            .collect(),
        Kind::Stay(0) => vec![(0, -1.0)],
        Kind::Stay(1) => vec![(0, 1.0), (1, -1.0)],
        Kind::Stay(2) => vec![(0, 1.0), (2, -1.0)],
        Kind::Stay(3) => vec![(1, 1.0), (2, 1.0), (3, -1.0)],
        Kind::Stay(_) => unreachable!(),
    };
    indices
        .into_iter()
        .map(|(i, sign)| {
            let (n, d) = through[i];
            (n * sign, d * sign)
        })
        .collect()
}

/// Each conditional branch has one convex source region at an instant.
#[derive(Clone, Debug)]
pub(super) struct CurveCell {
    scene: Arc<Scene>,
    path: Vec<usize>,
    clauses: Vec<Clause>,
    base: Vec<super::translation::MovingPlane>,
    holes: Vec<Hole>,
}

impl Scene {
    pub(super) fn admission(&self) -> MotionAdmission<'_> {
        MotionAdmission { links: &self.links, first: self.first, budget: self.budget, convention: self.convention }
    }

    pub fn new(cast: &MaterialCast<'_>, admission: MotionAdmission<'_>) -> Arc<Self> {
        Arc::new(Self {
            body: cast.material.domain.clone(),
            source: cast.source(),
            time: cast.time.clone(),
            until: cast.until,
            links: admission.links.to_vec(),
            prepared_steps: OnceLock::new(),
            first: admission.first,
            budget: admission.budget,
            convention: admission.convention,
        })
    }
    fn pose(&self, link_index: usize, time: f64) -> Result<Pose, NumericFault> {
        let link = self.links[link_index];
        let span = self.time.end - self.time.start;
        let world = self.time.start + span * time;
        // Geometry stays fixed for this scene. Each sample checks its clock.
        // Failed preparation uses full sampling to keep the refusal order.
        let prepared = self.prepared_steps.get_or_init(|| self.links.iter().map(|link| {
            Ok((link.enter_step.prepare_wide(link.enter)?,
                link.exit_step.prepare_wide(link.exit)?))
        }).collect());
        let (mut enter, mut exit) = match &prepared[link_index] {
            Ok((enter, exit)) => (enter.sample_wide(world)?, exit.sample_wide(world)?),
            Err(_) => (link.enter_step.sample_wide(link.enter, world)?,
                link.exit_step.sample_wide(link.exit, world)?),
        };
        enter.shift *= span;
        enter.turn *= span;
        exit.shift *= span;
        exit.turn *= span;
        let along = match self.convention {
            MapConvention::Reflection => 1.0,
            MapConvention::Rotation => -1.0,
        };
        let en = enter.normal;
        let ex = exit.normal;
        let et = enter.tangent();
        let xt = exit.tangent();
        let q = DMat2::from_cols(
            -ex * en.x + xt * (along * et.x),
            -ex * en.y + xt * (along * et.y),
        );
        let scale = exit.half / enter.half;
        let translation = exit.origin - q * enter.origin * scale;
        let map = Sim2::from_wide_translation(scale, q, translation)?;
        let parity = if map.reflected() { -1.0 } else { 1.0 };
        let spin = exit.turn - parity * enter.turn;
        let velocity = exit.origin_velocity()
            - map.direction_wide(enter.origin_velocity()) * scale
            - perp(map.direction_wide(enter.origin)) * scale * spin;
        Ok(Pose {
            enter,
            exit,
            map,
            velocity,
            spin,
        })
    }

    fn states(
        &self,
        path: &[usize],
        time: f64,
        half: f64,
    ) -> Result<(Vec<State>, Vec<Pose>), NumericFault> {
        let poses = (0..self.links.len())
            .map(|i| self.pose(i, time))
            .collect::<Result<Vec<_>, _>>()?;
        let bounds = self.body.bounds();
        let center = (bounds.min.as_dvec2() + bounds.max.as_dvec2()) * 0.5;
        let radius = (bounds.max.as_dvec2() - bounds.min.as_dvec2()).length() * 0.5;
        let mut states = vec![State {
            time,
            evaluations: 1,
            map: self.source.map_at(time)?,
            velocity: self.source.map_velocity(time),
            spin: self.source.turn(),
            reference: self.source.root_at(time),
            reference_velocity: self.source.root_velocity(time),
            variation: self.source.variation(bounds, time, half),
            reference_variation: self.source.root_variation(time, half),
            on: None,
        }];
        for &index in path {
            let old = *states.last().unwrap();
            let pose = poses[index];
            let scale = pose.map.scale_wide();
            let rotate = |v| pose.map.direction_wide(v) * scale;
            let map = old.map.then(pose.map)?;
            let velocity = pose.velocity
                + rotate(old.velocity)
                + perp(rotate(old.map.translation())) * pose.spin;
            let spin = pose.spin
                + if pose.map.reflected() {
                    -old.spin
                } else {
                    old.spin
                };
            let delta = old.reference - pose.enter.origin;
            let factor = match self.convention {
                MapConvention::Reflection => scale,
                MapConvention::Rotation => -scale,
            };
            let along = pose.enter.tangent().dot(delta) * factor;
            let along_rate = (pose
                .enter
                .tangent()
                .dot(old.reference_velocity - pose.enter.origin_velocity())
                + perp(pose.enter.tangent()).dot(delta) * pose.enter.turn)
                * factor;
            let clamped = along.clamp(-pose.exit.half, pose.exit.half);
            let reference = pose.exit.origin + pose.exit.tangent() * clamped;
            let reference_velocity = pose.exit.origin_velocity()
                + perp(pose.exit.tangent()) * (pose.exit.turn * clamped)
                + pose.exit.tangent()
                    * if along.abs() < pose.exit.half {
                        along_rate
                    } else {
                        0.0
                    };
            let enter_move = pose.enter.variation(half);
            let exit_move = pose.exit.variation(half);
            let angle = if half == 0.0 || pose.spin == 0.0 {
                0.0
            } else {
                (pose.spin.abs() * half * (1.0 + f64::EPSILON * 8.0)
                    + pose.enter.phase_error
                    + pose.exit.phase_error)
                    .min(2.0)
            };
            let reach = old.map.point_wide(center).distance(pose.enter.origin)
                + old.map.scale_wide() * radius;
            let variation = exit_move + scale * (old.variation + enter_move + angle * reach);
            let reference_variation = exit_move
                + clamped.abs() * pose.exit.angle_bound(half)
                + scale
                    * (old.reference_variation
                        + enter_move
                        + pose.enter.angle_bound(half) * delta.length());
            if !velocity.is_finite()
                || !reference_velocity.is_finite()
                || !variation.is_finite()
                || !reference_variation.is_finite()
            {
                return Err(NumericFault::Overflow);
            }
            states.push(State {
                time,
                evaluations: old.evaluations + 1,
                map,
                velocity,
                spin,
                reference,
                reference_velocity,
                variation,
                reference_variation,
                on: Some(index),
            });
        }
        Ok((states, poses))
    }

    fn rank(&self, state: State, index: usize, pose: Pose, half: f64) -> (f64, f64) {
        // The carried reference is on the exit plane, including its end clamps.
        if state.on.is_some_and(|last| {
            self.links[last].exit == self.links[index].enter
                && self.links[last].exit_step == self.links[index].enter_step
        }) {
            return (0.0, 0.0);
        }
        let relative = state.reference - pose.enter.origin;
        let rank = pose.enter.normal.dot(relative);
        let error = state.reference_variation
            + pose.enter.variation(half)
            + pose.enter.angle_bound(half) * relative.length();
        let rounding = if half == 0.0 {
            0.0
        } else {
            round(state.reference.length() + pose.enter.origin.length()) * state.evaluations as f64
        };
        (rank, error + rounding)
    }

    fn eligible(&self, state: State, stage: usize, index: usize, pose: Pose, half: f64) -> Truth {
        if stage == 0 && self.first == Some(self.links[index].key) {
            return Truth::Yes;
        }
        if stage == 0 && self.source.turn() == 0.0
            && (self.links[index].enter_step.turn == 0.0 || self.time.start == self.time.end)
        {
            let link = self.links[index];
            let start = if self.time.start == self.time.end {
                pose.enter.origin
            } else {
                link.enter.frame.origin.as_dvec2()
                    - link.enter_step.displacement.as_dvec2() * (1.0 - self.time.start)
            };
            let origin = pose.enter.normal.dot(self.source.root - start);
            let rate = pose.enter.normal.dot(self.source.delta - pose.enter.shift);
            let a = origin + rate * (state.time - half).max(0.0);
            let b = origin + rate * (state.time + half).min(self.until);
            let low = a.min(b);
            let high = a.max(b);
            // The incoming side owns a reference that reaches the plane.
            if low > 0.0 || low == 0.0 && rate < 0.0 {
                return Truth::Yes;
            }
            if high < 0.0 || high == 0.0 && rate >= 0.0 {
                return Truth::No;
            }
            return Truth::Maybe;
        }
        let (rank, error) = self.rank(state, index, pose, half);
        if rank - error > 0.0 {
            Truth::Yes
        } else if rank + error <= 0.0 {
            Truth::No
        } else {
            Truth::Maybe
        }
    }

    fn condition(&self, clause: Clause, state: State, poses: &[Pose], half: f64) -> Truth {
        let eligible = self.eligible(state, clause.stage, clause.link, poses[clause.link], half);
        if matches!(clause.kind, Kind::Through) || eligible == Truth::No {
            return eligible;
        }
        let Some(chosen) = clause.prior else {
            return eligible;
        };
        if clause.stage == 0 {
            if self.first == Some(self.links[chosen].key) {
                return Truth::No;
            }
            if self.first == Some(self.links[clause.link].key) {
                return Truth::Yes;
            }
        }
        let (a, ea) = self.rank(state, clause.link, poses[clause.link], half);
        let (b, eb) = self.rank(state, chosen, poses[chosen], half);
        let error = ea + eb;
        let precedes = |a: f64, b: f64| {
            a.total_cmp(&b)
                .then(self.links[clause.link].key.cmp(&self.links[chosen].key))
                .is_lt()
        };
        if !precedes(a - error, b + error) {
            Truth::No
        } else if precedes(a + error, b - error) && eligible == Truth::Yes {
            Truth::Yes
        } else {
            Truth::Maybe
        }
    }

    fn planes(
        &self,
        clause: Clause,
        state: State,
        pose: Pose,
        half: f64,
    ) -> Vec<(PlaneMotion, f64)> {
        let n = pose.enter.normal;
        let tangent = pose.enter.tangent();
        let planes = selected_planes(
            clause.kind,
            n,
            tangent,
            pose.enter.half,
            self.links[clause.link].depth.map(f64::from),
        );
        planes
            .into_iter()
            .map(|(n, constant)| self.pull(state, pose.enter, n, constant, half))
            .collect()
    }

    fn pull(
        &self,
        state: State,
        aperture: WideAperture,
        n: DVec2,
        constant: f64,
        half: f64,
    ) -> (PlaneMotion, f64) {
        let scale = state.map.scale_wide();
        let bounds = self.body.bounds();
        let center = (bounds.min.as_dvec2() + bounds.max.as_dvec2()) * 0.5;
        let radius = (bounds.max.as_dvec2() - bounds.min.as_dvec2()).length() * 0.5;
        let reach = state.map.point_wide(center).distance(aperture.origin) + scale * radius;
        let error =
            (state.variation + aperture.variation(half) + aperture.angle_bound(half) * reach)
                / scale;
        let normal_rate = perp(n) * aperture.turn;
        let delta = aperture.origin - state.map.translation();
        let plane = MaterialPlane {
            inward: state.map.inverse_direction_wide(n),
            offset: (n.dot(delta) + constant) / scale,
        };
        let normal_rate = state
            .map
            .inverse_direction_wide(normal_rate - perp(n) * state.spin);
        let offset_rate = (perp(n).dot(delta) * aperture.turn
            + n.dot(aperture.origin_velocity() - state.velocity))
            / scale;
        let rounding = round(
            (aperture.origin.length() + state.map.translation().length()) / scale
                + center.length()
                + radius
                + plane.offset.abs()
                + constant.abs() / scale,
        ) * state.evaluations as f64;
        (
            PlaneMotion {
                plane,
                normal_rate,
                offset_rate,
            },
            error + rounding,
        )
    }

    fn through_possible(
        &self,
        clause: Clause,
        state: State,
        pose: Pose,
        half: f64,
        previous: Option<usize>,
    ) -> bool {
        if clause.stage == 0 && self.first == Some(self.links[clause.link].key) {
            return true;
        }
        if previous.is_some_and(|last| {
            self.links[last].exit.frame.normal == self.links[clause.link].enter.frame.normal
                && self.links[last].exit_step.turn == self.links[clause.link].enter_step.turn
        }) {
            // Carried material is in front of its exit. A parallel entry
            // with a front reference cannot cut that material.
            return false;
        }
        let bounds = self.body.bounds();
        let center = (bounds.min.as_dvec2() + bounds.max.as_dvec2()) * 0.5;
        let radius =
            (bounds.max.as_dvec2() - bounds.min.as_dvec2()).length() * 0.5 * state.map.scale_wide();
        let center = state.map.point_wide(center);
        let reach =
            center.distance(state.reference) + radius + state.variation + state.reference_variation;
        let distance = center.distance(pose.enter.origin);
        let error = radius
            + state.variation
            + pose.enter.variation(half)
            + round(distance + reach + pose.enter.half);
        // A front reference and a point behind the entry bound the depth.
        distance <= error + pose.enter.half.hypot(reach)
    }
}

impl CurveCell {
    pub(super) fn whole(scene: Arc<Scene>) -> Self {
        Self { scene, path: Vec::new(), clauses: Vec::new(), base: Vec::new(), holes: Vec::new() }
    }

    pub fn from_affine(scene: Arc<Scene>, cell: &MaterialCell) -> Result<Self, NumericFault> {
        let path = cell
            .path
            .iter()
            .map(|key| {
                scene
                    .links
                    .iter()
                    .position(|l| l.key == *key)
                    .ok_or(NumericFault::InvalidFrame)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            scene,
            path,
            clauses: Vec::new(),
            base: cell.planes.clone(),
            holes: Vec::new(),
        })
    }

    fn hole_condition(&self, hole: Hole, state: State, poses: &[Pose], half: f64) -> Truth {
        if hole.exit || hole.admitted {
            Truth::Yes
        } else {
            self.scene
                .eligible(state, self.path.len(), hole.link, poses[hole.link], half)
        }
    }
    fn hole_plane(
        &self,
        hole: Hole,
        state: State,
        poses: &[Pose],
        half: f64,
    ) -> (PlaneMotion, f64) {
        let pose = poses[hole.link];
        let aperture = if hole.exit { pose.exit } else { pose.enter };
        let n = aperture.tangent() * if hole.side == 0 { -1.0 } else { 1.0 };
        self.scene.pull(state, aperture, n, aperture.half, half)
    }

    fn world_planes(&self, time: &Range<f64>) -> Result<Vec<super::translation::MovingPlane>, NumericFault> {
        if self.path.iter().any(|&index| {
            let link = self.scene.links[index];
            link.enter_step.turn != 0.0 || link.exit_step.turn != 0.0
        }) { return Ok(Vec::new()); }
        let (middle, half) = crate::frame::interval_center(time);
        let (states, poses) = self.scene.states(&self.path, middle, half)?;
        let origin = (0..self.scene.links.len()).map(|i| self.scene.pose(i, 0.0))
            .collect::<Result<Vec<_>, _>>()?;
        let suffix = |stage: usize| -> Result<(Sim2, DVec2), NumericFault> {
            let (mut map, mut travel) = (Sim2::IDENTITY, DVec2::ZERO);
            for &index in &self.path[stage..] {
                let pose = origin[index];
                travel = pose.map.direction_wide(travel) * pose.map.scale_wide() + pose.velocity;
                map = map.then(pose.map)?;
            }
            Ok((map, travel))
        };
        let mut out = Vec::new();
        for &clause in &self.clauses {
            let link = self.scene.links[clause.link];
            if link.enter_step.turn != 0.0 { continue; }
            // Stay(0) can omit its cut when the reference is not eligible.
            if matches!(clause.kind, Kind::Stay(0))
                && self.scene.condition(clause, states[clause.stage], &poses, half) != Truth::Yes { continue; }
            let pose = origin[clause.link];
            let (map, travel) = suffix(clause.stage)?;
            for (inward, constant) in selected_planes(clause.kind, pose.enter.normal,
                pose.enter.tangent(), pose.enter.half, link.depth.map(f64::from)) {
                let plane = super::translation::MovingPlane {
                    plane: MaterialPlane { inward, offset: inward.dot(pose.enter.origin) + constant },
                    rate: inward.dot(pose.enter.origin_velocity()),
                };
                out.push(super::carry_plane(plane, map, travel)?);
            }
        }
        let state = *states.last().unwrap();
        for &hole in &self.holes {
            let link = self.scene.links[hole.link];
            if (if hole.exit { link.exit_step.turn } else { link.enter_step.turn }) != 0.0 { continue; }
            if hole.side == 0 && self.hole_condition(hole, state, &poses, half) != Truth::Yes { continue; }
            let pose = origin[hole.link];
            let aperture = if hole.exit { pose.exit } else { pose.enter };
            let inward = aperture.tangent() * if hole.side == 0 { -1.0 } else { 1.0 };
            out.push(super::translation::MovingPlane {
                plane: MaterialPlane { inward, offset: inward.dot(aperture.origin) + aperture.half },
                rate: inward.dot(aperture.origin_velocity()),
            });
        }
        Ok(out)
    }

    fn affine_planes(
        &self,
        time: &Range<f64>,
    ) -> Result<Option<Vec<super::translation::MovingPlane>>, NumericFault> {
        if self.scene.source.turn() != 0.0
            || self.path.iter().any(|&index| {
                let link = self.scene.links[index];
                link.enter_step.turn != 0.0 || link.exit_step.turn != 0.0
            })
            || self.clauses.iter().any(|clause| self.scene.links[clause.link].enter_step.turn != 0.0)
            || self.holes.iter().any(|hole| {
                let link = self.scene.links[hole.link];
                if hole.exit { link.exit_step.turn != 0.0 } else { link.enter_step.turn != 0.0 }
            })
        {
            return Ok(None);
        }
        let (middle, half) = crate::frame::interval_center(time);
        let (states, poses) = self.scene.states(&self.path, middle, half)?;
        let state = *states.last().unwrap();
        let mut planes = self.base.clone();
        let affine = |p: PlaneMotion| super::translation::MovingPlane {
            plane: MaterialPlane {
                offset: p.plane.offset - p.offset_rate * middle,
                ..p.plane
            },
            rate: p.offset_rate,
        };
        for &clause in &self.clauses {
            let state = states[clause.stage];
            match self.scene.condition(clause, state, &poses, half) {
                Truth::No if matches!(clause.kind, Kind::Stay(0)) => continue,
                Truth::Yes => {}
                _ => return Ok(None),
            }
            planes.extend(self.scene.planes(clause, state, poses[clause.link], 0.0)
                .into_iter().map(|(plane, _)| affine(plane)));
        }
        for &hole in &self.holes {
            match self.hole_condition(hole, state, &poses, half) {
                Truth::No if hole.side == 0 => continue,
                Truth::Yes => {}
                _ => return Ok(None),
            }
            let (p, _) = self.hole_plane(hole, state, &poses, 0.0);
            planes.push(affine(p));
        }
        Ok(Some(planes))
    }

    pub fn line_parts(
        &self,
        cell: &MaterialCell,
        points: &[crate::Vec2],
        shift: crate::Vec2,
        turn: f32,
        pivot: crate::Vec2,
        admitted: Option<&[u32]>,
    ) -> Result<Vec<MaterialCell>, Refusal> {
        let mut holes = Vec::new();
        for (index, link) in self.scene.links.iter().enumerate() {
            let mut endpoints = vec![(link.enter, link.enter_step, false)];
            if self.path.last() == Some(&index) {
                endpoints.push((link.exit, link.exit_step, true));
            }
            for (aperture, step, exit) in endpoints {
                if !exit && admitted.is_some_and(|keys| !keys.contains(&link.key)) {
                    continue;
                }
                if step.turn != turn {
                    continue;
                }
                let n = aperture.frame.normal.as_dvec2().normalize();
                if turn != 0.0 && (step.displacement != shift || aperture.frame.pivot != pivot) {
                    continue;
                }
                if turn == 0.0 && n.dot(step.displacement.as_dvec2() - shift.as_dvec2()) != 0.0 {
                    continue;
                }
                if !points.iter().all(|p| {
                    let side = n.dot(p.as_dvec2() - aperture.frame.origin.as_dvec2());
                    side.abs()
                        <= round(
                            n.abs()
                                .dot(p.as_dvec2().abs() + aperture.frame.origin.as_dvec2().abs()),
                        )
                }) {
                    continue;
                }
                holes.push(Hole {
                    link: index,
                    exit,
                    side: 0,
                    admitted: admitted.is_some(),
                });
            }
        }
        if holes.is_empty() {
            return Ok(vec![cell.clone()]);
        }
        let mut programs = vec![self.clone()];
        let mut work = Work { count: 0 };
        for hole in holes {
            let mut next = Vec::new();
            for program in programs {
                for side in 0..2 {
                    let mut child = program.clone();
                    child.holes.push(Hole { side, ..hole });
                    if potential(&child, &cell.time, &mut work)? {
                        next.push(child);
                    }
                }
            }
            programs = next;
        }
        programs
            .into_iter()
            .map(|curve| {
                let affine = curve
                    .affine_planes(&cell.time)
                    .map_err(|e| failure(&curve, None, RefusalReason::Numeric(e)))?;
                let world_planes = curve.world_planes(&cell.time)
                    .map_err(|e| failure(&curve, None, RefusalReason::Numeric(e)))?;
                Ok(match affine {
                    Some(planes) => MaterialCell {
                        planes, world_planes,
                        curve: None,
                        ..cell.clone()
                    },
                    None => MaterialCell {
                        world_planes, curve: Some(Arc::new(curve)),
                        ..cell.clone()
                    },
                })
            })
            .collect()
    }
    /// A shared rigid law preserves every signed distance to this face.
    pub fn excludes_rigid(
        &self,
        points: &[crate::Vec2],
        shift: crate::Vec2,
        turn: f32,
        pivot: crate::Vec2,
        time: &Range<f64>,
    ) -> Result<bool, NumericFault> {
        let (middle, half) = crate::frame::interval_center(time);
        let (states, poses) = self.scene.states(&self.path, middle, half)?;
        for &clause in &self.clauses {
            let link = self.scene.links[clause.link];
            let (frame, step, kind, depth, reverse) = if clause.stage == self.path.len() {
                if matches!(clause.kind, Kind::Stay(0))
                    && self
                        .scene
                        .condition(clause, states[clause.stage], &poses, half)
                        != Truth::Yes
                {
                    continue;
                }
                (
                    link.enter.frame,
                    link.enter_step,
                    clause.kind,
                    link.depth.map(f64::from),
                    false,
                )
            } else if matches!(clause.kind, Kind::Through)
                && clause.stage + 1 == self.path.len()
                && self.path[clause.stage] == clause.link
            {
                (
                    link.exit.frame,
                    link.exit_step,
                    Kind::Through,
                    link.depth.map(|d| {
                        f64::from(d) * f64::from(link.exit.half_length)
                            / f64::from(link.enter.half_length)
                    }),
                    true,
                )
            } else {
                continue;
            };
            if step.displacement != shift
                || step.turn != turn
                || turn != 0.0 && frame.pivot != pivot
            {
                continue;
            }
            let mut normal = frame.normal.as_dvec2().normalize();
            if reverse {
                normal = -normal;
            }
            let half = if reverse {
                f64::from(link.exit.half_length)
            } else {
                f64::from(link.enter.half_length)
            };
            for (n, constant) in selected_planes(kind, normal, perp(normal), half, depth) {
                if points.iter().all(|p| {
                    let relative = p.as_dvec2() - frame.origin.as_dvec2();
                    n.dot(relative) - constant
                        <= round(
                            n.abs()
                                .dot(p.as_dvec2().abs() + frame.origin.as_dvec2().abs())
                                + constant.abs(),
                        )
                }) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    pub(super) fn unmapped_source(&self) -> Option<super::SourceMotion> {
        self.path.is_empty().then_some(self.scene.source)
    }
    pub fn at(&self, time: f64) -> Result<Sim2, NumericFault> {
        Ok(self
            .scene
            .states(&self.path, time, 0.0)?
            .0
            .last()
            .unwrap()
            .map)
    }
    pub fn velocity(&self, point: DVec2, time: f64) -> Result<DVec2, NumericFault> {
        let (states, _) = self.scene.states(&self.path, time, 0.0)?;
        let state = states.last().unwrap();
        Ok(state.velocity
            + perp(state.map.direction_wide(point)) * state.map.scale_wide() * state.spin)
    }
    pub fn variation(&self, time: f64, half: f64) -> Result<f64, NumericFault> {
        Ok(self
            .scene
            .states(&self.path, time, half)?
            .0
            .last()
            .unwrap()
            .variation)
    }
    pub fn planes_at(&self, time: f64) -> Result<Option<Vec<PlaneMotion>>, NumericFault> {
        let (states, poses) = self.scene.states(&self.path, time, 0.0)?;
        let mut out = self
            .base
            .iter()
            .map(|p| PlaneMotion {
                plane: MaterialPlane {
                    offset: p.plane.offset + p.rate * time,
                    ..p.plane
                },
                normal_rate: DVec2::ZERO,
                offset_rate: p.rate,
            })
            .collect::<Vec<_>>();
        for &clause in &self.clauses {
            let state = states[clause.stage];
            if self.scene.condition(clause, state, &poses, 0.0) == Truth::No {
                if matches!(clause.kind, Kind::Stay(0)) {
                    continue;
                } else {
                    return Ok(None);
                }
            }
            out.extend(
                self.scene
                    .planes(clause, state, poses[clause.link], 0.0)
                    .into_iter()
                    .map(|(p, _)| p),
            );
        }
        let state = *states.last().unwrap();
        for &hole in &self.holes {
            if self.hole_condition(hole, state, &poses, 0.0) == Truth::No {
                if hole.side == 0 {
                    continue;
                } else {
                    return Ok(None);
                }
            }
            out.push(self.hole_plane(hole, state, &poses, 0.0).0);
        }
        Ok(Some(out))
    }
    pub fn relaxed(
        &self,
        interval: &Range<f64>,
    ) -> Result<Option<Vec<MaterialPlane>>, NumericFault> {
        self.envelope(interval, false)
    }
    pub fn inner(&self, interval: &Range<f64>) -> Result<Option<Vec<MaterialPlane>>, NumericFault> {
        self.envelope(interval, true)
    }
    fn envelope(
        &self,
        interval: &Range<f64>,
        inner: bool,
    ) -> Result<Option<Vec<MaterialPlane>>, NumericFault> {
        let (middle, half) = crate::frame::interval_center(interval);
        let (states, poses) = self.scene.states(&self.path, middle, half)?;
        let mut out = self
            .base
            .iter()
            .map(|p| p.envelope(interval, inner))
            .collect::<Vec<_>>();
        for &clause in &self.clauses {
            let state = states[clause.stage];
            let condition = self.scene.condition(clause, state, &poses, half);
            if matches!(clause.kind, Kind::Through)
                && !self.scene.through_possible(
                    clause,
                    state,
                    poses[clause.link],
                    half,
                    clause.stage.checked_sub(1).map(|stage| self.path[stage]),
                )
            {
                return Ok(None);
            }
            if condition == Truth::No {
                if matches!(clause.kind, Kind::Stay(0)) {
                    continue;
                } else {
                    return Ok(None);
                }
            }
            if condition == Truth::Maybe {
                if !inner && matches!(clause.kind, Kind::Stay(0)) {
                    continue;
                }
                if inner && !matches!(clause.kind, Kind::Stay(0)) {
                    return Ok(None);
                }
            }
            out.extend(
                self.scene
                    .planes(clause, state, poses[clause.link], half)
                    .into_iter()
                    .map(|(p, error)| MaterialPlane {
                        offset: p.plane.offset + if inner { error } else { -error },
                        ..p.plane
                    }),
            );
        }
        let state = *states.last().unwrap();
        for &hole in &self.holes {
            let condition = self.hole_condition(hole, state, &poses, half);
            if condition == Truth::No {
                if hole.side == 0 {
                    continue;
                } else {
                    return Ok(None);
                }
            }
            if condition == Truth::Maybe {
                if !inner && hole.side == 0 {
                    continue;
                }
                if inner && hole.side != 0 {
                    return Ok(None);
                }
            }
            let (p, error) = self.hole_plane(hole, state, &poses, half);
            out.push(MaterialPlane {
                offset: p.plane.offset + if inner { error } else { -error },
                ..p.plane
            });
        }
        Ok(Some(out))
    }
    fn region(&self, time: f64) -> Result<Option<Vec<DVec2>>, NumericFault> {
        let Some(planes) = self.planes_at(time)? else {
            return Ok(None);
        };
        let mut points = super::curved::domain_polygon(&self.scene.body);
        for plane in planes {
            points = super::clip(&points, plane.plane);
            if points.is_empty() {
                return Ok(None);
            }
        }
        Ok(Some(points))
    }
    fn potential(&self, interval: &Range<f64>) -> Result<bool, NumericFault> {
        let Some(planes) = self.relaxed(interval)? else {
            return Ok(false);
        };
        let mut points = super::curved::domain_polygon(&self.scene.body);
        for plane in planes {
            points = super::clip(&points, plane);
            if points.is_empty() {
                return Ok(false);
            }
        }
        Ok(super::curved::intersection(&self.scene.body, &points).is_some())
    }

    fn area_potential(&self, interval: &Range<f64>) -> Result<bool, NumericFault> {
        let (middle, half) = crate::frame::interval_center(interval);
        let (states, poses) = self.scene.states(&self.path, middle, half)?;
        let mut affine = self.holes.is_empty() && self.scene.source.turn() == 0.0;
        let mut planes = self.base.clone();
        for &clause in &self.clauses {
            let state = states[clause.stage];
            match self.scene.condition(clause, state, &poses, half) {
                Truth::No if matches!(clause.kind, Kind::Stay(0)) => continue,
                Truth::No => return Ok(false),
                Truth::Maybe => affine = false,
                Truth::Yes => {}
            }
            if self.scene.links[clause.link].enter_step.turn != 0.0
                || self.path[..clause.stage].iter().any(|&i| {
                    self.scene.links[i].enter_step.turn != 0.0
                        || self.scene.links[i].exit_step.turn != 0.0
                })
            {
                affine = false;
            }
            planes.extend(
                self.scene
                    .planes(clause, state, poses[clause.link], 0.0)
                    .into_iter()
                    .map(|(p, _)| super::translation::MovingPlane {
                        plane: MaterialPlane {
                            offset: p.plane.offset - p.offset_rate * middle,
                            ..p.plane
                        },
                        rate: p.offset_rate,
                    }),
            );
        }
        if affine {
            return Ok(
                super::temporal::occupied(&self.scene.body, &planes, interval.clone())?.is_some(),
            );
        }
        self.potential(interval)
    }
    pub fn bounds(&self, interval: &Range<f64>) -> Result<Option<crate::Aabb>, NumericFault> {
        if !self.potential(interval)? {
            return Ok(None);
        }
        let (middle, half) = crate::frame::interval_center(interval);
        let (states, _) = self.scene.states(&self.path, middle, half)?;
        let state = states.last().unwrap();
        let mut lo = DVec2::splat(f64::INFINITY);
        let mut hi = -lo;
        for corner in super::corners(self.scene.body.bounds()) {
            let p = state.map.point_wide(corner.as_dvec2());
            lo = lo.min(p);
            hi = hi.max(p);
        }
        let error = state.variation + round(lo.length() + hi.length()) * state.evaluations as f64;
        super::enclosing_bounds(lo - DVec2::splat(error), hi + DVec2::splat(error)).map(Some)
    }
}

struct Work {
    count: usize,
}
impl Work {
    fn tick(&mut self, cell: &CurveCell, link: Option<usize>) -> Result<(), Refusal> {
        self.count += 1;
        if self.count > MAX_WORK {
            Err(failure(cell, link, RefusalReason::WorkLimit))
        } else {
            Ok(())
        }
    }
}
fn failure(cell: &CurveCell, link: Option<usize>, reason: RefusalReason) -> Refusal {
    Refusal {
        path: cell.path.iter().map(|&i| cell.scene.links[i].key).collect(),
        link: link.map(|i| cell.scene.links[i].key),
        reason,
    }
}
fn potential(cell: &CurveCell, time: &Range<f64>, work: &mut Work) -> Result<bool, Refusal> {
    work.tick(cell, None)?;
    cell.potential(time)
        .map_err(|e| failure(cell, None, RefusalReason::Numeric(e)))
}
fn exists(cell: &CurveCell, work: &mut Work) -> Result<bool, Refusal> {
    let mut stack = vec![0.0..cell.scene.until];
    while let Some(time) = stack.pop() {
        if !potential(cell, &time, work)? {
            continue;
        }
        // Use exact translating constraints when this cell has no turn.
        // A distant turning link does not make an empty static cell curved.
        if let Some(planes) = cell.affine_planes(&time)
            .map_err(|e| failure(cell, None, RefusalReason::Numeric(e)))? {
            if super::temporal::closure(&cell.scene.body, &planes, time.clone())
                .map_err(|e| failure(cell, None, RefusalReason::Numeric(e)))?.is_some() {
                return Ok(true);
            }
            continue;
        }
        let middle = time.start + (time.end - time.start) * 0.5;
        for at in [time.start, middle, time.end] {
            if cell
                .region(at)
                .map_err(|e| failure(cell, None, RefusalReason::Numeric(e)))?
                .is_some_and(|p| super::curved::intersection(&cell.scene.body, &p).is_some())
            {
                return Ok(true);
            }
        }
        if middle == time.start || middle == time.end {
            return Err(failure(
                cell,
                None,
                RefusalReason::Numeric(NumericFault::Precision),
            ));
        }
        stack.extend([middle..time.end, time.start..middle]);
    }
    Ok(false)
}

fn excludes(
    cell: CurveCell,
    chosen: Option<usize>,
    work: &mut Work,
) -> Result<Vec<CurveCell>, Refusal> {
    let stage = cell.path.len();
    let mut cells = vec![cell];
    for index in 0..cells[0].scene.links.len() {
        if chosen == Some(index) {
            continue;
        }
        let mut next = Vec::new();
        for cell in cells {
            let condition = Clause {
                stage,
                link: index,
                kind: Kind::Stay(0),
                prior: chosen,
            };
            let (states, poses) = cell
                .scene
                .states(&cell.path, cell.scene.until * 0.5, cell.scene.until * 0.5)
                .map_err(|e| failure(&cell, Some(index), RefusalReason::Numeric(e)))?;
            if cell.scene.condition(condition, states[stage], &poses, cell.scene.until * 0.5) == Truth::No {
                next.push(cell);
                continue;
            }
            let mut probe = cell.clone();
            probe.clauses.push(Clause {
                stage,
                link: index,
                kind: Kind::Through,
                prior: None,
            });
            if !potential(&probe, &(0.0..probe.scene.until), work)? || !exists(&probe, work)? {
                next.push(cell);
                continue;
            }
            let count = if cell.scene.links[index].depth.is_some() {
                4
            } else {
                3
            };
            for kind in 0..count {
                let mut branch = cell.clone();
                branch.clauses.push(Clause {
                    kind: Kind::Stay(kind),
                    ..condition
                });
                if potential(&branch, &(0.0..branch.scene.until), work)? {
                    next.push(branch);
                }
            }
        }
        cells = next;
        if cells.is_empty() {
            return Ok(cells);
        }
    }
    Ok(cells)
}

fn occupied(cell: &CurveCell, time: f64) -> Result<bool, NumericFault> {
    let Some(points) = cell.region(time)? else {
        return Ok(false);
    };
    if !crate::frame::thread::has_area(&points) {
        return Ok(false);
    }
    match &cell.scene.body {
        CombatVolume::Circle { center, radius } => {
            if super::curved::polygon_planes(&points)
                .iter()
                .all(|p| p.inward.dot(center.as_dvec2()) >= p.offset)
            {
                return Ok(true);
            }
            Ok(super::edges(&points).any(|(a, b)| {
                center
                    .as_dvec2()
                    .distance(super::closest_on_segment(center.as_dvec2(), a, b))
                    < f64::from(*radius)
            }))
        }
        _ => Ok(true),
    }
}

fn verify_budget(cells: &[CurveCell], work: &mut Work) -> Result<(), Refusal> {
    let Some(first) = cells.first() else {
        return Ok(());
    };
    let limit = usize::from(first.scene.budget.max_pieces);
    let mut stack = vec![0.0..first.scene.until];
    while let Some(time) = stack.pop() {
        let mut possible = Vec::new();
        for cell in cells {
            work.tick(cell, None)?;
            if cell
                .area_potential(&time)
                .map_err(|e| failure(cell, None, RefusalReason::Numeric(e)))?
            {
                possible.push(cell);
            }
        }
        if possible.len() <= limit {
            continue;
        }
        let middle = time.start + (time.end - time.start) * 0.5;
        for at in [time.start, middle, time.end] {
            let mut count = 0;
            for cell in &possible {
                if occupied(cell, at).map_err(|e| failure(cell, None, RefusalReason::Numeric(e)))? {
                    count += 1;
                }
            }
            if count > limit {
                return Err(failure(first, None, RefusalReason::PieceLimit));
            }
        }
        if middle == time.start || middle == time.end {
            return Err(failure(
                first,
                None,
                RefusalReason::Numeric(NumericFault::Precision),
            ));
        }
        stack.extend([middle..time.end, time.start..middle]);
    }
    Ok(())
}

pub(super) fn build(
    cast: &MaterialCast<'_>,
    admission: MotionAdmission<'_>,
    scene: Arc<Scene>,
) -> Result<Vec<MaterialCell>, Refusal> {
    let first = CurveCell::whole(scene);
    if admission.budget.max_pieces == 0 {
        return Err(failure(&first, None, RefusalReason::PieceLimit));
    }
    if !cast
        .material
        .to_world
        .admits_scale_exponent(admission.budget.max_scale_exponent)
    {
        return Err(failure(&first, None, RefusalReason::ScaleLimit));
    }
    for (index, link) in admission.links.iter().enumerate() {
        first
            .scene
            .pose(index, 0.0)
            .map_err(|e| failure(&first, Some(index), RefusalReason::Numeric(e)))?;
        if link.depth.is_some_and(|d| !d.is_finite() || d < 0.0) {
            return Err(failure(
                &first,
                Some(index),
                RefusalReason::Numeric(NumericFault::InvalidFrame),
            ));
        }
    }
    let mut queue = VecDeque::from([first]);
    let mut out = Vec::new();
    let mut work = Work { count: 0 };
    while let Some(cell) = queue.pop_front() {
        work.tick(&cell, None)?;
        for final_cell in excludes(cell.clone(), None, &mut work)? {
            if exists(&final_cell, &mut work)? {
                out.push(final_cell);
            }
        }
        for index in 0..admission.links.len() {
            let mut carried = cell.clone();
            carried.clauses.push(Clause {
                stage: cell.path.len(),
                link: index,
                kind: Kind::Through,
                prior: None,
            });
            if !potential(&carried, &(0.0..carried.scene.until), &mut work)? {
                continue;
            }
            let policy = if cell.path.len() >= usize::from(admission.budget.max_hops) {
                Some(RefusalReason::HopLimit)
            } else if !cell
                .at(cell.scene.until * 0.5)
                .map_err(|e| failure(&cell, Some(index), RefusalReason::Numeric(e)))?
                .admits_scale_product(
                    cell.scene
                        .pose(index, cell.scene.until * 0.5)
                        .map_err(|e| failure(&cell, Some(index), RefusalReason::Numeric(e)))?
                        .map,
                    admission.budget.max_scale_exponent,
                )
            {
                Some(RefusalReason::ScaleLimit)
            } else {
                None
            };
            let children = excludes(carried, Some(index), &mut work)?;
            for mut child in children {
                if let Some(reason) = policy {
                    if exists(&child, &mut work)? {
                        return Err(failure(&cell, Some(index), reason));
                    }
                } else if exists(&child, &mut work)? {
                    child.path.push(index);
                    queue.push_back(child);
                }
            }
        }
    }
    verify_budget(&out, &mut work)?;
    out.into_iter()
        .map(|curve| {
            let map = curve
                .at(0.0)
                .map_err(|e| failure(&curve, None, RefusalReason::Numeric(e)))?;
            let path = curve
                .path
                .iter()
                .map(|&i| curve.scene.links[i].key)
                .collect();
            let time = 0.0..curve.scene.until;
            let affine = curve.affine_planes(&time)
                .map_err(|e| failure(&curve, None, RefusalReason::Numeric(e)))?;
            let travel = curve.velocity(DVec2::ZERO, 0.0)
                .map_err(|e| failure(&curve, None, RefusalReason::Numeric(e)))?;
            let world_planes = curve.world_planes(&time)
                .map_err(|e| failure(&curve, None, RefusalReason::Numeric(e)))?;
            Ok(match affine {
                Some(planes) => MaterialCell { path, map, travel, planes, world_planes, time, curve: None },
                None => MaterialCell { path, map, world_planes, travel: DVec2::ZERO,
                    planes: Vec::new(), time, curve: Some(Arc::new(curve)) },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::thread::MaterialPrimitive;
    use crate::frame::{ApertureStep, PortalAperture, PortalFrame};
    use crate::{Aabb, Block, SurfaceChain, Vec2, World};

    #[test]
    fn a_static_opening_keeps_a_body_above_its_end_with_a_distant_turning_pair() {
        let opening = |x, normal| PortalAperture {
            frame: PortalFrame::fixed(Vec2::new(x, 248.0), normal), half_length: 40.0,
        };
        let mut links = PortalLink::both_ways(0, (opening(2800.0, -Vec2::X), opening(2544.0, Vec2::X))).to_vec();
        let enter = PortalAperture { frame: PortalFrame::fixed(Vec2::new(2900.0, 336.0), -Vec2::Y), half_length: 46.0 };
        let exit = PortalAperture { frame: PortalFrame::fixed(Vec2::new(3136.0, 186.0), -Vec2::Y), half_length: 46.0 };
        links.extend(PortalLink::both_ways_with_steps(2, [
            (enter, ApertureStep::fixed(enter.frame.origin, enter.frame.normal)),
            (exit, ApertureStep::ending_at(exit.frame, Vec2::ZERO, 0.01)),
        ]));
        let domain = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::new(30.065935, 48.0_f32.next_up()) * 0.5));
        let root = Vec2::new(2785.4165, 264.0_f32.next_down());
        let material = MaterialPrimitive { domain: &domain, to_world: Sim2::new(1.0, bevy_math::Mat2::IDENTITY, root).unwrap() };
        let cast = MaterialCast::new(material, root, Vec2::X * 4.5, 0.0..1.0);
        let admission = MotionAdmission { convention: MapConvention::Rotation, ..admission(&links) };
        admission.admit(cast, Vec2::X * 270.0, &[Vec2::Y * 2250.0]).unwrap();
    }

    fn links(turn: f32) -> [PortalLink; 2] {
        let enter = PortalAperture {
            frame: PortalFrame::fixed(Vec2::ZERO, -Vec2::X),
            half_length: 40.0,
        };
        let normal = (DMat2::from_angle(f64::from(turn)) * DVec2::X).as_vec2();
        let exit = PortalAperture {
            frame: PortalFrame::fixed(Vec2::new(100.0, 0.0), normal),
            half_length: 40.0,
        };
        PortalLink::both_ways_with_steps(
            0,
            [
                (
                    enter,
                    ApertureStep::fixed(enter.frame.origin, enter.frame.normal),
                ),
                (exit, ApertureStep::ending_at(exit.frame, Vec2::ZERO, turn)),
            ],
        )
    }
    fn request(body: &CombatVolume) -> MaterialCast<'_> {
        MaterialCast::new(
            MaterialPrimitive::world(body),
            body.center(),
            Vec2::ZERO,
            0.0..1.0,
        )
    }
    fn admission(links: &[PortalLink]) -> MotionAdmission<'_> {
        MotionAdmission {
            links,
            first: None,
            budget: PieceBudget::COMPOSED,
            convention: MapConvention::Reflection,
        }
    }

    #[test]
    fn a_rotating_exit_carries_material_into_a_wall_between_matching_endpoint_poses() {
        let body = CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::new(10.0, 1.0)));
        let world = World::new(
            "curved exit",
            Vec2::splat(1000.0),
            Vec2::ZERO,
            vec![Block::solid(
                "target",
                Vec2::new(95.0, 3.0),
                Vec2::new(10.0, 1.0),
            )],
        );
        for turns in [1.0, 2.0, 16_777_216.0] {
            let turn = std::f32::consts::TAU * turns;
            let links = links(turn);
            let hit = world
                .first_material_sweep(request(&body), Some(admission(&links)), |_, _, _| Ok(true))
                .unwrap()
                .unwrap();
            let theta = (3.0_f64 / 17.0_f64.sqrt()).asin() - (1.0_f64 / 4.0).atan();
            assert!(
                (hit.contact.toi * f64::from(turn) - theta).abs() < 1.0e-6,
                "{turn}: {hit:?}"
            );
            assert_eq!(hit.contact.path, [0]);
            assert!((hit.contact.world_point.y - 3.0).abs() < 1.0e-5, "{hit:?}");
            assert!(
                hit.contact
                    .map
                    .point(hit.contact.point)
                    .unwrap()
                    .distance(hit.contact.world_point)
                    < 1.0e-5
            );
        }
    }

    #[test]
    fn a_spinning_aperture_keeps_the_empty_corner_of_a_circle_empty() {
        let body = CombatVolume::circle(Vec2::new(-0.6, 0.0), 1.0);
        let links = links(std::f32::consts::TAU);
        let world = World::new(
            "empty curved corner",
            Vec2::splat(1000.0),
            Vec2::ZERO,
            vec![Block::solid(
                "corner",
                Vec2::new(100.9, 0.9),
                Vec2::splat(0.1),
            )],
        );
        assert!(
            world
                .first_material_sweep(request(&body), Some(admission(&links)), |_, _, _| Ok(true))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_rotating_exit_and_its_solid_host_keep_the_same_seam() {
        let body = CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::new(10.0, 1.0)));
        let turn = std::f32::consts::TAU;
        let mut links = links(turn);
        let rotation = DMat2::from_angle(f64::from(turn));
        let pivot = Vec2::new(100.0, 0.0);
        let points = [
            Vec2::new(-10.0, -40.0),
            Vec2::new(0.0, -40.0),
            Vec2::new(0.0, 40.0),
            Vec2::new(-10.0, 40.0),
        ]
        .map(|p| pivot + (rotation * p.as_dvec2()).as_vec2())
        .to_vec();
        let edge = points[2] - points[1];
        let mut exit = links[0].exit;
        exit.frame.normal = Vec2::new(edge.y, -edge.x).normalize();
        links = PortalLink::both_ways_with_steps(
            0,
            [
                (links[0].enter, links[0].enter_step),
                (exit, ApertureStep::ending_at(exit.frame, Vec2::ZERO, turn)),
            ],
        );
        let mut host = SurfaceChain::solid("exit host", points);
        host.turn = turn;
        host.pivot = pivot;
        let world = World::new(
            "host seam",
            Vec2::splat(1000.0),
            Vec2::ZERO,
            vec![Block::solid(
                "entry host",
                Vec2::new(0.0, -40.0),
                Vec2::new(10.0, 80.0),
            )],
        )
        .with_chains(vec![host]);
        let hit = world
            .first_material_sweep(request(&body), Some(admission(&links)), |_, _, _| Ok(true))
            .unwrap();
        assert!(hit.is_none(), "{hit:?}");
    }

    #[test]
    fn a_sliding_line_opening_uses_its_position_at_contact() {
        let enter = PortalAperture {
            frame: PortalFrame::fixed(Vec2::new(0.0, 40.0), -Vec2::X),
            half_length: 5.0,
        };
        let exit = PortalAperture {
            frame: PortalFrame::fixed(Vec2::new(100.0, 0.0), Vec2::X),
            half_length: 5.0,
        };
        let links = PortalLink::both_ways_with_steps(
            0,
            [
                (
                    enter,
                    ApertureStep::ending_at(enter.frame, Vec2::new(0.0, 80.0), 0.0),
                ),
                (
                    exit,
                    ApertureStep::fixed(exit.frame.origin, exit.frame.normal),
                ),
            ],
        );
        let world =
            World::new("sliding hole", Vec2::splat(1000.0), Vec2::ZERO, vec![]).with_chains(vec![
                SurfaceChain::open("host", vec![Vec2::new(0.0, 100.0), Vec2::new(0.0, -100.0)]),
            ]);
        for (body, expected) in [
            (
                CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::ONE)),
                0.55,
            ),
            (
                CombatVolume::circle(Vec2::new(-6.0, 0.0), 1.0),
                (7320.0_f64 - (7320.0_f64.powi(2) - 4.0 * 6500.0 * 2060.0).sqrt()) / 13000.0,
            ),
        ] {
            let cast = MaterialCast {
                delta: Vec2::new(10.0, 0.0),
                ..request(&body)
            };
            let hit = world
                .first_material_sweep(cast, Some(admission(&links)), |_, _, _| Ok(true))
                .unwrap()
                .unwrap();
            assert!(
                (hit.contact.toi - expected).abs() < 1.0e-8,
                "{body:?}: {hit:?}, expected={expected}"
            );
        }
    }

    #[test]
    fn a_spinning_line_keeps_its_opening_and_its_finite_ends() {
        let turn = std::f32::consts::TAU;
        let mut links = links(turn);
        let exit = links[0].exit;
        let tangent = Vec2::new(-exit.frame.normal.y, exit.frame.normal.x);
        let mut line = SurfaceChain::open(
            "spinning host",
            vec![
                exit.frame.origin + tangent * 40.0,
                exit.frame.origin - tangent * 40.0,
            ],
        );
        line.turn = turn;
        line.pivot = exit.frame.pivot;
        let edge = line.points[1] - line.points[0];
        let mut exit = exit;
        exit.frame.normal = Vec2::new(-edge.y, edge.x).normalize();
        links = PortalLink::both_ways_with_steps(
            0,
            [
                (links[0].enter, links[0].enter_step),
                (exit, ApertureStep::ending_at(exit.frame, Vec2::ZERO, turn)),
            ],
        );
        let world = World::new(
            "spinning line hole",
            Vec2::splat(1000.0),
            Vec2::ZERO,
            vec![],
        )
        .with_chains(vec![line]);
        let open = CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::new(10.0, 1.0)));
        assert!(
            world
                .first_material_sweep(request(&open), Some(admission(&links)), |_, _, _| Ok(true))
                .unwrap()
                .is_none()
        );
        let edge = CombatVolume::aabb(Aabb::new(Vec2::new(-20.0, 39.5), Vec2::new(10.0, 1.0)));
        let cast = MaterialCast {
            delta: Vec2::new(20.0, 0.0),
            ..request(&edge)
        };
        let hit = world
            .first_material_sweep(cast, Some(admission(&links)), |_, _, _| Ok(true))
            .unwrap()
            .unwrap();
        assert_eq!(hit.contact.path, [0]);
        assert!((hit.contact.toi - 0.5).abs() < 1.0e-8, "{hit:?}");
    }

    #[test]
    fn two_curved_paths_and_a_third_link_tile_the_same_canonical_material() {
        let body = CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::new(10.0, 20.0)));
        let turn = std::f32::consts::TAU;
        let mut links = Vec::new();
        for (i, y) in [-12.0, 12.0].into_iter().enumerate() {
            let enter = PortalAperture {
                frame: PortalFrame::fixed(Vec2::new(0.0, y), -Vec2::X),
                half_length: 8.0,
            };
            let exit = PortalAperture {
                frame: PortalFrame::fixed(Vec2::new(100.0, y * 5.0), Vec2::X),
                half_length: 8.0,
            };
            links.extend(PortalLink::both_ways_with_steps(
                i,
                [
                    (
                        enter,
                        ApertureStep::fixed(enter.frame.origin, enter.frame.normal),
                    ),
                    (exit, ApertureStep::ending_at(exit.frame, Vec2::ZERO, turn)),
                ],
            ));
        }
        let third = PortalAperture {
            frame: PortalFrame::fixed(Vec2::new(102.0, -60.0), -Vec2::X),
            half_length: 4.0,
        };
        let end = PortalAperture {
            frame: PortalFrame::fixed(Vec2::new(200.0, -60.0), Vec2::X),
            half_length: 2.0,
        };
        links.extend(PortalLink::both_ways(2, (third, end)));
        for convention in [MapConvention::Reflection, MapConvention::Rotation] {
            let cells = request(&body)
                .cells(Some(MotionAdmission {
                    convention,
                    ..admission(&links)
                }))
                .unwrap();
            assert!(cells.parts().iter().any(|p| p.path == [0, 4]));
            assert!(cells.parts().iter().any(|p| p.path == [2]));
            for tick in 0..32 {
                let time = (f64::from(tick) + 0.314159) / 32.0;
                let regions = cells
                    .parts()
                    .iter()
                    .map(|p| p.planes_at(time).unwrap())
                    .collect::<Vec<_>>();
                for index in 0..200 {
                    let point = DVec2::new(
                        -15.97 + f64::from(index % 10) * 1.99,
                        -19.93 + f64::from(index / 10) * 1.99,
                    );
                    let mut count = 0;
                    let mut boundary = false;
                    for planes in regions.iter().flatten() {
                        if planes
                            .iter()
                            .any(|p| (point.dot(p.plane.inward) - p.plane.offset).abs() < 1.0e-9)
                        {
                            boundary = true;
                        }
                        if planes
                            .iter()
                            .all(|p| point.dot(p.plane.inward) > p.plane.offset)
                        {
                            count += 1;
                        }
                    }
                    if !boundary {
                        assert_eq!(
                            count, 1,
                            "time={time}, point={point}, convention={convention:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_rotating_entry_carries_its_clipped_material_along_the_recorded_curve() {
        let turn = std::f32::consts::TAU;
        let mut enter = links(turn)[0].enter;
        enter.frame.normal = (DMat2::from_angle(f64::from(turn)) * (-DVec2::X)).as_vec2();
        let exit = PortalAperture {
            frame: PortalFrame::fixed(Vec2::new(100.0, 0.0), Vec2::X),
            half_length: 40.0,
        };
        let links = PortalLink::both_ways_with_steps(
            0,
            [
                (
                    enter,
                    ApertureStep::ending_at(enter.frame, Vec2::ZERO, turn),
                ),
                (
                    exit,
                    ApertureStep::fixed(exit.frame.origin, exit.frame.normal),
                ),
            ],
        );
        let body = CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::new(10.0, 1.0)));
        let world = World::new(
            "turning entry",
            Vec2::splat(1000.0),
            Vec2::ZERO,
            vec![Block::solid(
                "target",
                Vec2::new(95.0, 3.0),
                Vec2::new(10.0, 1.0),
            )],
        );
        let hit = world
            .first_material_sweep(request(&body), Some(admission(&links)), |_, _, _| Ok(true))
            .unwrap()
            .unwrap();
        let theta = (3.0_f64 / 17.0_f64.sqrt()).asin() - (1.0_f64 / 4.0).atan();
        assert!(
            (hit.contact.toi * f64::from(turn) - theta).abs() < 1.0e-6,
            "{hit:?}"
        );
        assert_eq!(hit.contact.path, [0]);
    }

    #[test]
    fn curved_contact_is_covariant_under_scale_parity_and_a_common_frame_boost() {
        let turn = std::f32::consts::TAU;
        let body = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::new(10.0, 1.0)));
        let theta = (3.0_f64 / 17.0_f64.sqrt()).asin() - (1.0_f64 / 4.0).atan();
        for scale in [0.5, 1.0, 2.0] {
            for parity in [-1.0, 1.0] {
                for boost in [Vec2::ZERO, Vec2::new(16.0, -8.0)] {
                    let mut links = links(turn);
                    for link in &mut links {
                        for (aperture, step) in [
                            (&mut link.enter, &mut link.enter_step),
                            (&mut link.exit, &mut link.exit_step),
                        ] {
                            aperture.frame.origin = aperture.frame.origin * scale + boost;
                            aperture.frame.pivot = aperture.frame.pivot * scale + boost;
                            aperture.half_length *= scale;
                            *step = ApertureStep::ending_at(aperture.frame, boost, step.turn);
                        }
                    }
                    let mut target = Block::solid(
                        "target",
                        Vec2::new(95.0, 3.0) * scale + boost,
                        Vec2::new(10.0, 1.0) * scale,
                    );
                    target.velocity = boost;
                    let world = World::new(
                        "covariant curve",
                        Vec2::splat(1000.0),
                        Vec2::ZERO,
                        vec![target],
                    );
                    let root = Vec2::new(-6.0 * scale, 0.0);
                    let cast = MaterialCast::new(
                        MaterialPrimitive {
                            domain: &body,
                            to_world: Sim2::new(
                                scale,
                                bevy_math::Mat2::from_cols(Vec2::X * parity, Vec2::Y),
                                root,
                            )
                            .unwrap(),
                        },
                        root,
                        boost * 0.25,
                        0.0..0.25,
                    );
                    let hit = world
                        .first_material_sweep(cast, Some(admission(&links)), |_, _, _| Ok(true))
                        .unwrap()
                        .unwrap();
                    assert!(
                        (hit.contact.world_time * f64::from(turn) - theta).abs() < 1.0e-6,
                        "{hit:?}"
                    );
                    assert!((hit.contact.toi * 0.25 - hit.contact.world_time).abs() < 1.0e-12);
                    assert!(
                        (hit.contact.world_point.y
                            - scale * 3.0
                            - boost.y * hit.contact.world_time as f32)
                            .abs()
                            < 1.0e-4
                    );
                }
            }
        }
    }

    #[test]
    fn curved_maps_keep_contact_strata_out_of_the_material_piece_budget() {
        let links = links(std::f32::consts::TAU);
        let body = CombatVolume::aabb(Aabb::new(Vec2::new(-20.0, 0.0), Vec2::splat(10.0)));
        let admission = MotionAdmission {
            budget: PieceBudget {
                max_pieces: 1,
                ..PieceBudget::COMPOSED
            },
            ..admission(&links)
        };
        MaterialCast {
            delta: Vec2::new(10.0, 0.0),
            ..request(&body)
        }
        .cells(Some(admission))
        .unwrap();
        assert!(matches!(
            MaterialCast {
                delta: Vec2::new(11.0, 0.0),
                ..request(&body)
            }
            .cells(Some(admission)),
            Err(Refusal {
                reason: RefusalReason::PieceLimit,
                ..
            })
        ));
        let denied = MotionAdmission {
            budget: PieceBudget {
                max_hops: 0,
                ..PieceBudget::COMPOSED
            },
            ..admission
        };
        let through = CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::new(10.0, 1.0)));
        assert!(matches!(
            request(&through).cells(Some(denied)),
            Err(Refusal {
                reason: RefusalReason::HopLimit,
                ..
            })
        ));
    }

    #[test]
    fn a_curved_cut_bound_contains_material_at_both_adjacent_times() {
        let turn = 6_588_397.5_f32;
        let start = 0.1_f64;
        let end = start.next_up();
        let wanted = DVec2::new(-1.0e-5, -1.0).normalize();
        let normal = (DMat2::from_angle(f64::from(turn))
            * DMat2::from_angle(-f64::from(turn) * end)
            * wanted)
            .as_vec2();
        let enter = PortalAperture {
            frame: PortalFrame::fixed(Vec2::ZERO, normal),
            half_length: 5000.0,
        };
        let exit = PortalAperture {
            frame: PortalFrame::fixed(Vec2::new(10_000.0, 0.0), Vec2::X),
            half_length: 5000.0,
        };
        let links = PortalLink::both_ways_with_steps(
            0,
            [
                (
                    enter,
                    ApertureStep::ending_at(enter.frame, Vec2::ZERO, turn),
                ),
                (
                    exit,
                    ApertureStep::fixed(exit.frame.origin, exit.frame.normal),
                ),
            ],
        );
        let body = CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::new(4000.0, 0.1)));
        let scene = Scene::new(&request(&body), admission(&links));
        let cell = CurveCell {
            scene,
            path: Vec::new(),
            clauses: vec![Clause {
                stage: 0,
                link: 0,
                kind: Kind::Through,
                prior: None,
            }],
            base: Vec::new(),
            holes: Vec::new(),
        };
        let relaxed = cell.relaxed(&(start..end)).unwrap().unwrap();
        for at in [start, end] {
            let pose = cell.scene.pose(0, at).unwrap();
            let point = -pose.enter.tangent() * 3900.0;
            assert!((-4006.0..3994.0).contains(&point.x) && point.y.abs() < 0.1);
            for p in cell.planes_at(at).unwrap().unwrap() {
                assert!(point.dot(p.plane.inward) >= p.plane.offset - round(point.length()));
            }
            for plane in &relaxed {
                assert!(
                    point.dot(plane.inward) >= plane.offset - round(point.length()),
                    "time={at}, point={point}, plane={plane:?}"
                );
            }
        }
    }
}
