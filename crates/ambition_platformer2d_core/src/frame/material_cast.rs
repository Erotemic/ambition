//! Continuous contact in the spatial cells of a material partition.

use bevy_math::DVec2;

use super::thread::{
    MaterialPlane, MaterialPrimitive, MotionAdmission, Refusal, RefusalReason,
};
use super::{NumericFault, Sim2};
use crate::{Aabb, CombatVolume, Vec2};

mod translation;
mod penetration;
pub(crate) use penetration::PolygonAxisContact;
mod cell;
mod temporal;
mod affine;
mod curved;
mod arcs;
mod payload;
mod source;
mod sampled;
pub use sampled::{SampledPieceHit, sweep_polygon_against_piece};
pub(super) use source::{SourceMotion, rotation_delta};
pub use cell::MaterialCell;

/// How a cast treats material already inside an obstacle at its start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialOverlap {
    /// Permit motion that separates an existing overlap.
    AllowSeparation,
    /// Report the occupied start even when motion separates it.
    Report,
}

/// A primitive's recorded rigid motion in its owner's world chart.
#[derive(Clone)]
pub struct MaterialCast<'a> {
    pub(super) material: MaterialPrimitive<'a>,
    pub(super) root: Vec2,
    pub(super) delta: Vec2,
    /// The attempted motion's interval within the world's current tick.
    pub(super) time: std::ops::Range<f64>,
    pub(super) until: f64,
    pub(super) rotation: Option<source::Rotation>,
    pub(super) initial_overlap: InitialOverlap,
}

/// One contact, with both the owner-chart and physical-world witnesses.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialCastHit {
    pub toi: f64,
    /// Contact time within the world's current tick.
    pub world_time: f64,
    pub normal: Vec2,
    pub point: Vec2,
    pub world_point: Vec2,
    pub path: Vec<u32>,
    pub map: Sim2,
}

/// The actual primitive moves through cells with their own boundary and map motion.
pub struct MaterialCastCells<'a> {
    material: MaterialPrimitive<'a>,
    /// The canonical primitive stays fixed while its source map follows this curve.
    source: SourceMotion,
    until: f64,
    parts: Vec<MaterialCell>,
    time: std::ops::Range<f64>,
    scene:Option<std::sync::Arc<arcs::Scene>>,
    initial_overlap: InitialOverlap,
}

fn carry_plane(
    plane: translation::MovingPlane, map: Sim2, travel: DVec2,
) -> Result<translation::MovingPlane, NumericFault> {
    let inward = map.direction_wide(plane.plane.inward);
    let offset = plane.plane.offset * map.scale_wide() + inward.dot(map.translation());
    let rate = plane.rate * map.scale_wide() + inward.dot(travel);
    if !inward.is_finite() || !offset.is_finite() || !rate.is_finite() { return Err(NumericFault::Overflow); }
    Ok(translation::MovingPlane { plane: MaterialPlane { inward, offset }, rate })
}

fn numeric(reason: NumericFault) -> Refusal {
    Refusal {
        path: Vec::new(),
        link: None,
        reason: RefusalReason::Numeric(reason),
    }
}

fn numeric_at(part: &MaterialCell, reason: NumericFault) -> Refusal {
    let mut path = part.path.clone();
    let link = path.pop();
    Refusal {
        path,
        link,
        reason: RefusalReason::Numeric(reason),
    }
}

impl<'a> MaterialCast<'a> {
    /// Record the original line and its world clock.
    pub fn new(material: super::thread::MaterialPrimitive<'a>, root: Vec2, delta: Vec2, time: std::ops::Range<f64>) -> Self {
        Self { material, root, delta, time, until: 1.0, rotation: None, initial_overlap: InitialOverlap::AllowSeparation }
    }

    pub fn with_initial_overlap(mut self, initial_overlap: InitialOverlap) -> Self {
        self.initial_overlap = initial_overlap;
        self
    }

    /// Rotate the source about a translating pivot. Keep the full recorded turn.
    /// Duration is in seconds for the original curve. A prefix keeps it.
    /// Payload velocity is measured at the root at the start of the cast.
    pub fn rotating_about(mut self, pivot: Vec2, turn: f64, duration: f64) -> Result<Self, Refusal> {
        if !pivot.is_finite() || !turn.is_finite() || !duration.is_finite() { return Err(numeric(NumericFault::NonFinite)); }
        if duration < 0.0 || turn != 0.0 && (duration == 0.0 || self.time.start == self.time.end) {
            return Err(numeric(NumericFault::InvalidFrame));
        }
        let spin = if turn == 0.0 { 0.0 } else { turn / duration };
        if !spin.is_finite() { return Err(numeric(NumericFault::Overflow)); }
        if turn != 0.0 && spin == 0.0 { return Err(numeric(NumericFault::Underflow)); }
        self.rotation = Some(source::Rotation { pivot: pivot.as_dvec2(), turn, spin });
        Ok(self)
    }

    pub(super) fn source(&self) -> SourceMotion {
        SourceMotion { map: self.material.to_world, root: self.root.as_dvec2(), delta: self.delta.as_dvec2(), rotation: self.rotation }
    }

    /// Restrict the original parameter interval without changing the curve or clock.
    pub fn prefix(mut self, until: f64) -> Result<Self, Refusal> {
        if !until.is_finite() { return Err(numeric(NumericFault::NonFinite)); }
        if until < 0.0 || until > self.until { return Err(numeric(NumericFault::InvalidFrame)); }
        self.until = until;
        Ok(self)
    }

    /// Keep a crossing found during this transaction until its root rebase.
    pub(crate) fn motion_admission<'p>(
        &self, admission: Option<MotionAdmission<'p>>,
    ) -> Result<Option<MotionAdmission<'p>>, Refusal> {
        let Some(mut policy) = admission else { return Ok(None); };
        if policy.first.is_some() || self.time.start == self.time.end { return Ok(Some(policy)); }
        let mut first: Option<(f64, u32)> = None;
        for link in policy.links {
            let crossing = self.crossing(link.enter_step, link.enter, self.until)
                .map_err(|reason| Refusal { path: vec![], link: Some(link.key), reason })?;
            if let Some(crossing) = crossing {
                let key = (crossing.toi, link.key);
                if first.is_none_or(|old| key.0.total_cmp(&old.0).then(key.1.cmp(&old.1)).is_lt()) {
                    first = Some(key);
                }
            }
        }
        policy.first = first.map(|(_, key)| key);
        Ok(Some(policy))
    }

    pub fn cells(
        self,
        admission: Option<MotionAdmission<'_>>,
    ) -> Result<MaterialCastCells<'a>, Refusal> {
        let admission = self.motion_admission(admission)?;
        if !self.time.start.is_finite() || !self.time.end.is_finite() {
            return Err(numeric(NumericFault::NonFinite));
        }
        if self.time.start<0.0 || self.time.end>1.0 || self.time.start>self.time.end {
            return Err(numeric(NumericFault::InvalidFrame));
        }
        super::thread::map_volume(self.material.domain, &self.material.to_world)
            .map_err(numeric)?;
        if !self.delta.is_finite() || !self.root.is_finite() { return Err(numeric(NumericFault::NonFinite)); }
        let bounds = self.material.domain.bounds();
        let dimensions = bounds.max - bounds.min;
        // A finite cross-section sweeps a band without an artificial thickness.
        if dimensions.min_element() < 0.0 {
            return Err(numeric(NumericFault::InvalidScale));
        }
        let policy = admission.unwrap_or(MotionAdmission { links: &[], first: None,
            budget: super::thread::PieceBudget::COMPOSED, convention: super::MapConvention::Reflection });
        let rotating = self.rotation.is_some_and(|rotation| rotation.turn != 0.0);
        let scene = (!policy.links.is_empty() || self.rotation.is_some())
            .then(|| arcs::Scene::new(&self, policy));
        let source = self.source();
        let parts = if rotating {
            let scene = scene.as_ref().unwrap().clone();
            if admission.is_some() { arcs::build(&self, policy, scene)? }
            else {
                vec![MaterialCell { world_planes: Vec::new(), path: vec![], map: self.material.to_world, travel: DVec2::ZERO,
                    planes: vec![], time: 0.0..self.until, curve: Some(std::sync::Arc::new(arcs::CurveCell::whole(scene))) }]
            }
        } else if let Some(admission) = admission {
            if admission.links.iter().any(|link|link.enter_step.turn!=0.0 || link.exit_step.turn!=0.0) {
                if self.time.start==self.time.end {
                    let links=admission.links.iter().map(|link| {
                        let pose=link.snapshot_at(self.time.start).map_err(numeric)?;
                        Ok(super::thread::PortalLink {depth:pose.depth,..super::thread::PortalLink::new(pose.enter,pose.exit,pose.key)})
                    }).collect::<Result<Vec<_>,Refusal>>()?;
                    affine::build(&self,MotionAdmission {links:&links,..admission})?
                } else {arcs::build(&self,admission,scene.as_ref().unwrap().clone())?}
            } else {affine::build(&self, admission)?}
        } else {
            vec![MaterialCell { world_planes: Vec::new(), path: vec![], curve:None,map: self.material.to_world,
                travel: self.delta.as_dvec2(), planes: vec![], time: 0.0..self.until }]
        };
        for cell in &parts {
            for time in [cell.time.start, cell.time.end] {
                super::thread::map_volume(self.material.domain, &cell.at(time)
                    .map_err(|e| numeric_at(cell, e))?).map_err(|e| numeric_at(cell, e))?;
            }
            cell.bounds(bounds, DVec2::ZERO).map_err(|e| numeric_at(cell, e))?;
        }
        Ok(MaterialCastCells {
            material: self.material,
            source,
            until: self.until,
            parts,
            time:self.time,
            scene,
            initial_overlap: self.initial_overlap,
        })
    }
}

impl MaterialCastCells<'_> {
    pub fn parts(&self) -> &[MaterialCell] {
        &self.parts
    }

    /// Broad phase bounds in each part's physical chart.
    pub fn bounds(&self, part: &MaterialCell) -> Result<Option<Aabb>, Refusal> {
        part.bounds(self.material.domain.bounds(), DVec2::ZERO)
            .map_err(|reason| numeric_at(part, reason))
    }

    /// The obstacle is at the end of the world tick. Its displacement covers
    /// that tick. Its shape can be a convex polygon, a segment or a point.
    pub fn against(
        &self,
        part: &MaterialCell,
        obstacle: &[Vec2],
        displacement: Vec2,
    ) -> Result<Option<MaterialCastHit>, Refusal> {
        self.against_rigid_where(part,obstacle,displacement,0.0,Vec2::ZERO,|_|true)
    }

    fn solid_may_occupy(
        &self, part: &MaterialCell, obstacle: &[Vec2], displacement: Vec2,
    ) -> bool {
        if part.world_planes.is_empty() { return true; }
        let motion = displacement.as_dvec2() * (self.time.end - self.time.start);
        let mut region = obstacle.iter().map(|point| point.as_dvec2()
            - displacement.as_dvec2() * (1.0 - self.time.start)).collect::<Vec<_>>();
        for plane in &part.world_planes {
            // A matching rate keeps this cut fixed in the obstacle's chart.
            // Other cuts can admit new solid area later in the interval.
            if plane.rate == plane.plane.inward.dot(motion) {
                region = clip(&region, plane.plane);
            }
        }
        crate::frame::thread::has_area(&region)
    }

    fn against_translation(&self,part:&MaterialCell,obstacle:&[Vec2],displacement:Vec2)
        ->Result<Option<MaterialCastHit>,Refusal> {
        let inverse = part
            .map
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
        for point in &mut region { *point-=full_motion*(1.0-self.time.start); }
        let motion=full_motion*(self.time.end-self.time.start)
            - inverse.direction_wide(part.travel) * inverse.scale_wide();
        // A boundary stratum retains the incoming derivative at its instant.
        let interval=if part.time.start==part.time.end && part.time.start>0.0 {
            0.0..part.time.end
        } else { part.time.clone() };
        for point in &mut region { *point += motion * interval.start; }
        let duration = interval.end - interval.start;
        let contact = translation::cast(self.material.domain, DVec2::ZERO, &region, motion * duration,
            part.planes.iter().map(|p| p.during(interval.clone())), self.initial_overlap)
            .map_err(|fault| numeric_at(part, fault))?;
        let Some(contact) = contact else {
            return Ok(None);
        };
        let time=interval.start+duration*contact.time;
        if time<part.time.start {return Ok(None);}
        self.contact(part, CastHit { time, ..contact })
    }

    /// Follow the collider's published rigid motion, including full turns.
    pub fn against_rigid(
        &self, part: &MaterialCell, obstacle: &[Vec2], displacement: Vec2,
        turn: f32, pivot: Vec2,
    ) -> Result<Option<MaterialCastHit>, Refusal> {
        self.against_rigid_where(part,obstacle,displacement,turn,pivot,|_| true)
    }

    /// Reject a contact component and continue along the recorded curve.
    pub(crate) fn against_rigid_where(
        &self, part: &MaterialCell, obstacle: &[Vec2], displacement: Vec2,
        turn: f32, pivot: Vec2, mut accepts:impl FnMut(&MaterialCastHit)->bool,
    ) -> Result<Option<MaterialCastHit>, Refusal> {
        if !turn.is_finite() || !pivot.is_finite() {
            return Err(numeric_at(part, NumericFault::NonFinite));
        }
        if turn == 0.0 && obstacle.len() >= 3 && !self.solid_may_occupy(part, obstacle, displacement) {
            return Ok(None);
        }
        if turn==0.0 && part.curve.is_none() && (obstacle.len()>2 || self.scene.is_none()) {
            return Ok(self.against_translation(part,obstacle,displacement)?.filter(|hit|accepts(hit)));
        }
        let scopes = self.obstacle_scopes(part, obstacle, displacement, turn, pivot)?;
        let mut selected:Option<MaterialCastHit>=None;
        let mut failed=None;
        for scope in scopes.iter() {
            if turn==0.0 && scope.curve.is_none() {
                if let Some(hit)=self.against_translation(scope,obstacle,displacement)?.filter(|hit|accepts(hit)) {
                    if selected.as_ref().is_none_or(|old|hit.toi<old.toi){selected=Some(hit);}
                }
                continue;
            }
            let _ = curved::cast(self.material.domain, scope, obstacle, displacement,
            turn, pivot, self.time.clone(), self.initial_overlap, |contact| match self.contact(scope,contact) {
                Ok(Some(hit)) if accepts(&hit)=>{
                    if selected.as_ref().is_none_or(|old|hit.toi<old.toi){selected=Some(hit);} true
                },
                Ok(_)=>false,
                Err(reason)=>{ failed=Some(reason); true },
            }).map_err(|reason| {
                let mut path = part.path.clone();
                let link = path.pop();
                Refusal { path, link, reason }
            })?;
            if failed.is_some(){break;}
        }
        match failed {Some(reason)=>Err(reason),None=>Ok(selected)}
    }

    fn obstacle_scopes<'b>(&self, part: &'b MaterialCell, obstacle: &[Vec2], displacement: Vec2,
        turn: f32, pivot: Vec2) -> Result<std::borrow::Cow<'b, [MaterialCell]>, Refusal> {
        Ok(if obstacle.len()<=2 {
            if let Some(scene)=&self.scene {
                let program=match &part.curve {
                    Some(curve)=>(**curve).clone(),
                    None=>arcs::CurveCell::from_affine(scene.clone(),part).map_err(|e|numeric_at(part,e))?,
                };
                let admitted=if part.curve.is_none() {
                    Some(self.parts.iter().filter(|other|other.path.len()>part.path.len()
                        && other.path.starts_with(&part.path)
                        && (other.time.start.max(part.time.start)<other.time.end.min(part.time.end)
                            || other.time.start==other.time.end && other.time.start>=part.time.start && other.time.end<=part.time.end))
                        .map(|other|other.path[part.path.len()]).collect::<Vec<_>>())
                } else {None};
                std::borrow::Cow::Owned(program.line_parts(part,obstacle,displacement,turn,pivot,admitted.as_deref())?)
            } else {std::borrow::Cow::Borrowed(std::slice::from_ref(part))}
        } else {std::borrow::Cow::Borrowed(std::slice::from_ref(part))})
    }

    fn contact(&self, part: &MaterialCell, contact: CastHit) -> Result<Option<MaterialCastHit>, Refusal> {
        let time = contact.time;
        // Cells that start at a topology event own its contacts.
        if part.time.start<part.time.end && part.time.end < self.until
            && time >= part.time.end - f64::EPSILON * 64.0 { return Ok(None); }
        let root = self.source.map_at(time)
            .map_err(|reason| numeric_at(part, reason))?;
        let point = root
            .point_from_wide(contact.point)
            .map_err(|reason| numeric_at(part, reason))?;
        let pose = part.at(time).map_err(|reason| numeric_at(part, reason))?;
        let world_point = pose
            .point_from_wide(contact.point)
            .map_err(|reason| numeric_at(part, reason))?;
        let normal = root
            .direction(contact.normal.as_vec2())
            .normalize_or_zero();
        Ok(Some(MaterialCastHit {
            toi: time,
            world_time: self.time.start + (self.time.end-self.time.start) * time,
            normal,
            point,
            world_point,
            path: part.path.clone(),
            map: root.inverse()
                .and_then(|inverse| inverse.then(pose))
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

/// Round a broad phase envelope outwards, including subnormal coordinates.
pub(super) fn enclosing_bounds(lo: DVec2, hi: DVec2) -> Result<Aabb, NumericFault> {
    if !lo.is_finite() || !hi.is_finite() { return Err(NumericFault::NonFinite); }
    let mut min = lo.as_vec2();
    let mut max = hi.as_vec2();
    if !min.is_finite() || !max.is_finite() { return Err(NumericFault::Overflow); }
    if f64::from(min.x) > lo.x { min.x = min.x.next_down(); }
    if f64::from(min.y) > lo.y { min.y = min.y.next_down(); }
    if f64::from(max.x) < hi.x { max.x = max.x.next_up(); }
    if f64::from(max.y) < hi.y { max.y = max.y.next_up(); }
    if !min.is_finite() || !max.is_finite() { return Err(NumericFault::Overflow); }
    Ok(Aabb { min, max })
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
