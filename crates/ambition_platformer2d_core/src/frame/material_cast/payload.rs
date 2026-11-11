//! Validate mapped payloads over each occupied material cell and its full time interval.

use super::{
    CombatVolume, MaterialCastCells, MaterialCell, MaterialPlane, NumericFault, Refusal,
    RefusalReason,
};
use crate::Vec2;
use crate::frame::{MapConvention, thread::MotionAdmission};
use bevy_math::{DMat2, DVec2};

/// Velocity conversion at one material contact point.
/// The drift includes imposed source turns and moving portal surfaces.
#[derive(Clone,Copy,Debug)]
pub struct ContactVelocityFrame {
    linear: crate::frame::Sim2,
    drift: DVec2,
}

impl ContactVelocityFrame {
    pub const IDENTITY: Self=Self {linear:crate::frame::Sim2::IDENTITY,drift:DVec2::ZERO};

    pub fn to_physical(self, velocity: Vec2) -> Result<Vec2,NumericFault> {
        if !velocity.is_finite() { return Err(NumericFault::NonFinite); }
        let value=self.linear.direction_wide(velocity.as_dvec2())*self.linear.scale_wide()+self.drift;
        crate::frame::Sim2::IDENTITY.vector_from_wide(value)
    }

    pub fn to_source(self, velocity: Vec2) -> Result<Vec2,NumericFault> {
        if !velocity.is_finite() { return Err(NumericFault::NonFinite); }
        self.linear.inverse()?.vector_from_wide(velocity.as_dvec2()-self.drift)
    }
}

#[derive(Clone, Copy)]
struct Affine {
    linear: DMat2,
    offset: DVec2,
}
impl Affine {
    fn constant(offset: DVec2) -> Self {
        Self {
            linear: DMat2::ZERO,
            offset,
        }
    }
    fn add(self, other: Self) -> Self {
        Self {
            linear: self.linear + other.linear,
            offset: self.offset + other.offset,
        }
    }
    fn scale(self, linear: DMat2) -> Self {
        Self {
            linear: linear * self.linear,
            offset: linear * self.offset,
        }
    }
    fn shift(self, offset: DVec2) -> Self {
        Self {
            offset: self.offset + offset,
            ..self
        }
    }
    fn norm(self, body: &CombatVolume) -> f64 {
        super::corners(body.bounds())
            .into_iter()
            .map(|point| (self.linear * point.as_dvec2() + self.offset).length())
            .fold(0.0, f64::max)
    }
}

fn failure(cell: &MaterialCell, reason: RefusalReason) -> Refusal {
    let mut path = cell.path.clone();
    let link = path.pop();
    Refusal { path, link, reason }
}
fn round(value: f64) -> f64 {
    value.abs() * f64::EPSILON * 128.0
}

impl MaterialCastCells<'_> {
    /// Use the same velocity law as payload admission at the contact point.
    /// Input velocity is at the source root before the recorded turn.
    pub fn contact_velocity_frame(&self, hit: &super::SampledPieceHit) -> Result<ContactVelocityFrame,Refusal> {
        let cell=self.parts.iter().find(|cell| cell.path==hit.source_path
            && hit.toi>=cell.time.start && hit.toi<=cell.time.end)
            .ok_or_else(||super::numeric(NumericFault::InvalidFrame))?;
        let numeric=|fault|failure(cell,RefusalReason::Numeric(fault));
        if cell.path.is_empty() && self.source.rotation.is_none_or(|turn|turn.turn==0.0) {
            return Ok(ContactVelocityFrame::IDENTITY);
        }
        let scene=self.scene.as_ref().ok_or_else(||numeric(NumericFault::InvalidFrame))?;
        let values=self.payloads(cell,scene.admission(),hit.toi,0.0,Vec2::ZERO,&[]).map_err(numeric)?;
        let speed=values.last().unwrap().0;
        let drift=speed.linear*hit.source_point+speed.offset;
        if !drift.is_finite() { return Err(numeric(NumericFault::Overflow)); }
        let linear=self.source.map_at(hit.toi).and_then(|map|map.inverse())
            .and_then(|map|map.then(cell.at(hit.toi)?)).map_err(numeric)?;
        Ok(ContactVelocityFrame {linear,drift})
    }

    /// Admit a response over all occupied material at one accepted time.
    pub fn admit_payloads_at(&self, time:f64, velocity:Vec2, carried:&[Vec2]) -> Result<(),Refusal> {
        if !time.is_finite() || !velocity.is_finite() || carried.iter().any(|value|!value.is_finite()) {
            return Err(super::numeric(NumericFault::NonFinite));
        }
        if time<0.0 || time>self.until { return Err(super::numeric(NumericFault::InvalidFrame)); }
        let Some(scene)=self.scene.as_ref() else {return Ok(());};
        for cell in self.parts.iter().filter(|cell|time>=cell.time.start && time<=cell.time.end) {
            let numeric=|fault|failure(cell,RefusalReason::Numeric(fault));
            let Some(planes)=cell.planes_at(time).map_err(numeric)? else {continue;};
            let planes=planes.into_iter().map(|plane|plane.plane).collect::<Vec<_>>();
            let values=self.payloads(cell,scene.admission(),time,0.0,velocity,carried).map_err(numeric)?;
            let Some(bounds)=value_bounds(self.material.domain,&planes,&values) else {continue;};
            let limit=f64::from(f32::MAX);
            if bounds.iter().any(|(lo,hi)|*lo < -limit || *hi > limit) {
                return Err(numeric(NumericFault::Overflow));
            }
        }
        Ok(())
    }

    /// Geometry and payloads use the same cells, portal paths and world clock.
    pub fn admit_payloads(&self, velocity: Vec2, carried: &[Vec2]) -> Result<(), Refusal> {
        if !velocity.is_finite() || carried.iter().any(|vector| !vector.is_finite()) {
            return Err(super::numeric(NumericFault::NonFinite));
        }
        let Some(scene) = &self.scene else {
            return Ok(());
        };
        let admission = scene.admission();
        let mut work = 0usize;
        for cell in self.parts.iter().filter(|cell| !cell.path.is_empty() || self.source.rotation.is_some()) {
            let mut pending = vec![cell.time.clone()];
            while let Some(time) = pending.pop() {
                work += 1;
                if work > 16_384 {
                    return Err(failure(cell, RefusalReason::WorkLimit));
                }
                let numeric = |reason| failure(cell, RefusalReason::Numeric(reason));
                let Some(planes) = cell.relaxed_planes(&time).map_err(numeric)? else {
                    continue;
                };
                let (middle, half) = crate::frame::interval_center(&time);
                let values = self
                    .payloads(cell, admission, middle, half, velocity, carried)
                    .map_err(numeric)?;
                let Some(bounds) = value_bounds(self.material.domain, &planes, &values) else {
                    continue;
                };
                let limit = f64::from(f32::MAX);
                if bounds
                    .iter()
                    .all(|(low, high)| *low >= -limit && *high <= limit)
                {
                    continue;
                }
                if let Some(planes) = cell.planes_at(middle).map_err(numeric)? {
                    let planes: Vec<_> = planes.into_iter().map(|plane| plane.plane).collect();
                    let exact: Vec<_> = values.iter().map(|(value, _)| (*value, 0.0)).collect();
                    if value_bounds(self.material.domain, &planes, &exact).is_some_and(|bounds| {
                        bounds.into_iter().any(|(low, high)| {
                            low < -limit - round(low) || high > limit + round(high)
                        })
                    }) {
                        return Err(numeric(NumericFault::Overflow));
                    }
                }
                if middle == time.start || middle == time.end {
                    return Err(numeric(NumericFault::Precision));
                }
                pending.push(middle..time.end);
                pending.push(time.start..middle);
            }
        }
        Ok(())
    }

    fn payloads(
        &self,
        cell: &MaterialCell,
        admission: MotionAdmission<'_>,
        time: f64,
        half: f64,
        velocity: Vec2,
        carried: &[Vec2],
    ) -> Result<Vec<(Affine, f64)>, NumericFault> {
        let body = self.material.domain;
        let map = self.source.map_at(time)?;
        let basis = DMat2::from_cols(map.direction_wide(DVec2::X), map.direction_wide(DVec2::Y))
            * map.scale_wide();
        let mut point = Affine {
            linear: basis,
            offset: map.translation(),
        };
        let mut point_change = self.source.variation(body.bounds(), time, half);
        let j = DMat2::from_cols(DVec2::Y, -DVec2::X);
        let (mut speed, mut speed_change) = match self.source.rotation {
            Some(rotation) => {
                let pivot = rotation.pivot + self.source.delta * time;
                let pivot_velocity = velocity.as_dvec2() - j * self.source.root_arm() * rotation.spin;
                (point.shift(-pivot).scale(j * rotation.spin).shift(pivot_velocity),
                    rotation.spin.abs() * (point_change + self.source.delta.length() * half))
            }
            None => (Affine::constant(velocity.as_dvec2()), 0.0),
        };
        let mut vectors: Vec<_> = carried
            .iter()
            .map(|vector| (Affine::constant(vector.as_dvec2()), 0.0))
            .collect();
        let world_span = self.time.end - self.time.start;
        let world_time = self.time.start + world_span * time;
        let world_half = world_span * half;
        for key in &cell.path {
            let link = admission
                .links
                .iter()
                .find(|link| link.key == *key)
                .ok_or(NumericFault::InvalidFrame)?;
            let enter = link.enter_step.sample_wide(link.enter, world_time)?;
            let exit = link.exit_step.sample_wide(link.exit, world_time)?;
            let along = match admission.convention {
                MapConvention::Reflection => 1.0,
                MapConvention::Rotation => -1.0,
            };
            let q = DMat2::from_cols(
                -exit.normal * enter.normal.x + exit.tangent() * (along * enter.tangent().x),
                -exit.normal * enter.normal.y + exit.tangent() * (along * enter.tangent().y),
            );
            let scale = exit.half / enter.half;
            let linear = q * scale;
            let parity = if q.determinant() < 0.0 { -1.0 } else { 1.0 };
            let turn = exit.turn - parity * enter.turn;
            let angle = (turn.abs() * world_half + enter.phase_error + exit.phase_error).min(2.0);
            let reach = point.shift(-enter.origin).norm(body);
            let next_point_change = exit.variation(world_half)
                + scale * (point_change + enter.variation(world_half) + angle * reach);
            let next_point = point.shift(-enter.origin).scale(linear).shift(exit.origin);
            let enter_spin = f64::from(link.enter.frame.spin);
            let exit_spin = f64::from(link.exit.frame.spin);
            let entry_surface = point
                .shift(-enter.pivot)
                .scale(j * enter_spin)
                .shift(link.enter.frame.velocity.as_dvec2());
            let relative = speed.add(entry_surface.scale(-DMat2::IDENTITY));
            let relative_change = speed_change
                + enter_spin.abs() * (point_change + enter.shift.length() * world_half);
            let next_speed = relative
                .scale(linear)
                .add(next_point.shift(-exit.pivot).scale(j * exit_spin))
                .shift(link.exit.frame.velocity.as_dvec2());
            speed_change = scale * (relative_change + angle * relative.norm(body))
                + exit_spin.abs() * (next_point_change + exit.shift.length() * world_half);
            for (vector, change) in &mut vectors {
                *change = scale * (*change + angle * vector.offset.length());
                *vector = vector.scale(linear);
                if *change != 0.0 {
                    *change += round(vector.offset.length() + *change);
                }
            }
            speed = next_speed;
            if speed_change != 0.0 {
                speed_change += round(speed.norm(body) + speed_change);
            }
            point = next_point;
            point_change = next_point_change + round(point.norm(body) + next_point_change);
        }
        vectors.push((speed, speed_change));
        if vectors.iter().any(|(value, change)| {
            !value.linear.is_finite() || !value.offset.is_finite() || !change.is_finite()
        }) {
            return Err(NumericFault::Overflow);
        }
        Ok(vectors)
    }
}

fn value_bounds(
    body: &CombatVolume,
    planes: &[MaterialPlane],
    values: &[(Affine, f64)],
) -> Option<Vec<(f64, f64)>> {
    let mut result = Vec::new();
    for (value, change) in values {
        for axis in [DVec2::X, DVec2::Y] {
            let direction = value.linear.transpose() * axis;
            let (low, high) = support(body, planes, direction)?;
            let offset = axis.dot(value.offset);
            let error = if value.linear == DMat2::ZERO && *change == 0.0 {
                0.0
            } else {
                *change + round(low.abs() + high.abs() + offset.abs())
            };
            result.push((low + offset - error, high + offset + error));
        }
    }
    Some(result)
}

fn support(body: &CombatVolume, planes: &[MaterialPlane], direction: DVec2) -> Option<(f64, f64)> {
    let points = match body {
        CombatVolume::Circle { center, radius } => {
            let center = center.as_dvec2();
            let radius = f64::from(*radius);
            let mut points = vec![center];
            let axis = if direction == DVec2::ZERO {
                DVec2::X
            } else {
                direction.normalize()
            };
            points.extend([center + axis * radius, center - axis * radius]);
            for plane in planes {
                let length = plane.inward.length();
                let normal = plane.inward / length;
                let distance = (plane.offset - plane.inward.dot(center)) / length;
                if distance.abs() <= radius {
                    let foot = center + normal * distance;
                    let tangent = DVec2::new(-normal.y, normal.x)
                        * (radius * radius - distance * distance).max(0.0).sqrt();
                    points.extend([foot + tangent, foot - tangent]);
                }
            }
            for (index, a) in planes.iter().enumerate() {
                for b in &planes[index + 1..] {
                    let determinant = a.inward.perp_dot(b.inward);
                    if determinant != 0.0 {
                        points.push(
                            DVec2::new(
                                a.offset * b.inward.y - a.inward.y * b.offset,
                                a.inward.x * b.offset - a.offset * b.inward.x,
                            ) / determinant,
                        );
                    }
                }
            }
            points
                .into_iter()
                .filter(|point| {
                    (*point - center).length_squared()
                        <= radius * radius * (1.0 + f64::EPSILON * 16.0)
                        && planes.iter().all(|plane| {
                            plane.inward.dot(*point)
                                >= plane.offset
                                    - round(
                                        plane.inward.abs().dot(point.abs()) + plane.offset.abs(),
                                    )
                        })
                })
                .collect()
        }
        _ => {
            let mut points = body
                .polygon()?
                .into_iter()
                .map(Vec2::as_dvec2)
                .collect::<Vec<_>>();
            for plane in planes {
                points = super::clip(&points, *plane);
            }
            points
        }
    };
    let mut span: Option<(f64, f64)> = None;
    for point in points {
        let projection = point.dot(direction);
        span = Some(span.map_or((projection, projection), |(low, high)| {
            (low.min(projection), high.max(projection))
        }));
    }
    span
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::material_cast::MaterialCast;
    use crate::frame::{
        ApertureStep, PortalAperture, PortalFrame,
        thread::{MaterialPrimitive, PieceBudget, PortalLink},
    };

    fn entry() -> PortalAperture {
        PortalAperture {
            frame: PortalFrame::fixed(Vec2::ZERO, Vec2::Y),
            half_length: 20.0,
        }
    }
    fn exit() -> PortalAperture {
        PortalAperture {
            frame: PortalFrame::fixed(Vec2::new(100.0, 0.0), -Vec2::Y),
            half_length: 20.0,
        }
    }

    #[test]
    fn force_overflow_between_equal_portal_endpoint_poses_refuses_the_source() {
        let mut link = PortalLink::new(entry(), exit(), 0);
        link.exit_step =
            ApertureStep::ending_at(link.exit.frame, Vec2::ZERO, std::f32::consts::TAU);
        let links = [link];
        let admission = MotionAdmission {
            links: &links,
            first: Some(0),
            budget: PieceBudget::COMPOSED,
            convention: MapConvention::Reflection,
        };
        let body = CombatVolume::circle(Vec2::new(0.0, -2.0), 0.5);
        let source = MaterialCast::new(
            MaterialPrimitive::world(&body),
            Vec2::new(0.0, -2.0),
            Vec2::ZERO,
            0.0..1.0,
        );
        let force = Vec2::splat(f32::MAX * 0.8);
        assert!(
            link.map(admission.convention)
                .unwrap()
                .vector(force)
                .is_ok()
        );
        let early = MaterialCast::new(
            MaterialPrimitive::world(&body),
            Vec2::new(0.0, -2.0),
            Vec2::ZERO,
            0.0..1.0,
        )
        .prefix(0.01)
        .unwrap();
        assert!(admission.admit(early, Vec2::ZERO, &[force]).is_ok());
        let failure = admission.admit(source, Vec2::ZERO, &[force]).unwrap_err();
        assert_eq!(
            failure.reason,
            RefusalReason::Numeric(NumericFault::Overflow)
        );
        assert_eq!(failure.link, Some(0));
    }

    #[test]
    fn a_constant_map_accepts_the_largest_finite_payload_component() {
        let links = [PortalLink::new(entry(), exit(), 0)];
        let admission = MotionAdmission {
            links: &links,
            first: Some(0),
            budget: PieceBudget::COMPOSED,
            convention: MapConvention::Reflection,
        };
        let body = CombatVolume::circle(Vec2::new(0.0, -2.0), 0.5);
        let source = MaterialCast::new(
            MaterialPrimitive::world(&body),
            Vec2::new(0.0, -2.0),
            Vec2::ZERO,
            0.0..1.0,
        );
        assert!(
            admission
                .admit(source, Vec2::X * f32::MAX, &[Vec2::X * f32::MAX])
                .is_ok()
        );
    }

    #[test]
    fn payload_extrema_use_the_cut_circle_instead_of_its_box() {
        let circle = CombatVolume::circle(Vec2::ZERO, 1.0);
        let planes = [MaterialPlane {
            inward: DVec2::X,
            offset: 0.5,
        }];
        let (low, high) = support(&circle, &planes, DVec2::Y).unwrap();
        let height = 0.75_f64.sqrt();
        assert!((low + height).abs() < 1.0e-14 && (high - height).abs() < 1.0e-14);
    }
}
