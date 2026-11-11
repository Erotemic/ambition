//! Admitted polygon material against one sampled target region.

mod curved;

use super::{MaterialCastCells, translation};
use crate::frame::thread::{Refusal, RefusalReason, VolumePiece};
use crate::frame::{NumericFault, Sim2};
use crate::Vec2;

/// Contact in the physical chart, with both material paths.
#[derive(Clone, Debug, PartialEq)]
pub struct SampledPieceHit {
    pub toi: f64,
    pub point: Vec2,
    /// Direction from the sampled target into the moving polygon.
    pub normal: Vec2,
    pub source_path: Vec<u32>,
    pub target_path: Vec<u32>,
    /// Map from the target's root chart to the contacting physical chart.
    pub target_map: Sim2,
    /// Accepted source material at contact on the original sweep.
    pub source_map: Sim2,
}

impl MaterialCastCells<'_> {
    /// Contact with a fixed target, retaining both source and target cuts.
    /// Report occupied starts. Keep analytic target circles through the cast.
    pub fn against_piece(&self, target: &VolumePiece) -> Result<Option<SampledPieceHit>, Refusal> {
        let numeric = |fault| {
            let mut path = target.path.clone();
            let link = path.pop();
            Refusal { path, link, reason: RefusalReason::Numeric(fault) }
        };
        let polygon = self.material.domain.polygon().ok_or_else(|| numeric(NumericFault::InvalidFrame))?;
        if polygon.len() < 3 { return Err(numeric(NumericFault::InvalidFrame)); }
        let points: Vec<_> = polygon.iter().map(|point| point.as_dvec2()).collect();
        let origin = points[0];
        let area: f64 = super::edges(&points).map(|(a,b)| (a-origin).perp_dot(b-origin)).sum();
        if area == 0.0 { return Err(numeric(NumericFault::InvalidFrame)); }
        for (a,b) in super::edges(&points) {
            let edge = b-a;
            if points.iter().any(|point| {
                let arm = *point-a;
                edge.perp_dot(arm)*area.signum() < -edge.abs().dot(arm.abs())*f64::EPSILON*16.0
            }) { return Err(numeric(NumericFault::InvalidFrame)); }
        }
        super::super::thread::map_volume(target.source(), &target.transform()).map_err(numeric)?;
        let inverse = target.transform().inverse().map_err(numeric)?;
        let bounds = target.world_volume().bounds();
        let mut first: Option<SampledPieceHit> = None;
        for part in &self.parts {
            let Some(source_bounds) = self.bounds(part)? else { continue; };
            if (0..2).any(|axis| source_bounds.max[axis] < bounds.min[axis].next_down()
                || source_bounds.min[axis] > bounds.max[axis].next_up()) { continue; }
            let hit=if part.curve.is_some() {
                curved::cast(part,&points,target).map_err(|reason| {
                    let mut path=part.path.clone();
                    let link=path.pop();
                    Refusal {path,link,reason}
                })?
            } else {
                let map = part.map.then(inverse).map_err(numeric)?;
                let motion = inverse.direction_wide(part.travel)*inverse.scale_wide();
                let interval = part.time.clone();
                let region: Vec<_> = points.iter().map(|point| map.point_wide(*point)+motion*interval.start).collect();
                let mut planes = target.source_bounds().iter().copied().map(translation::MovingPlane::from).collect::<Vec<_>>();
                for plane in &part.planes {
                    planes.push(super::carry_plane(*plane,map,motion).map_err(numeric)?);
                }
                let planes = planes.into_iter().map(|plane| plane.during(interval.clone())).collect::<Vec<_>>();
                translation::cast(target.source(), bevy_math::DVec2::ZERO,
                    &region, motion*(interval.end-interval.start), planes.into_iter(), super::InitialOverlap::Report)
                    .map_err(numeric)?.map(|hit| super::CastHit {
                        time:interval.start+(interval.end-interval.start)*hit.time,..hit
                    })
            };
            let Some(hit)=hit else {continue;};
            let time=hit.time;
            if part.time.start < part.time.end && part.time.end < self.until
                && time >= part.time.end-f64::EPSILON*64.0 { continue; }
            let region=curved::region(part,&points,time,target,inverse).map_err(numeric)?;
            let point=super::curved::intersection(target.source(),&region).unwrap_or(hit.point);
            let contact = SampledPieceHit {
                toi: time,
                point: target.transform().point_from_wide(point).map_err(numeric)?,
                normal: -target.transform().direction(hit.normal.as_vec2()).normalize_or_zero(),
                source_path: part.path.clone(),
                target_path: target.path.clone(),
                target_map: target.path_transform().map_err(numeric)?,
                source_map: part.at(time).map_err(numeric)?,
            };
            if first.as_ref().is_none_or(|old| contact.toi.total_cmp(&old.toi)
                .then(contact.source_path.cmp(&old.source_path)).is_lt()) { first=Some(contact); }
        }
        Ok(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Aabb, CombatVolume};
    use crate::frame::thread::MaterialPrimitive;
    use bevy_math::Mat2;

    fn sweep_polygon_against_piece(polygon: &[Vec2], map: Sim2, delta: Vec2, target: &VolumePiece)
        -> Result<Option<SampledPieceHit>, Refusal> {
        let domain = CombatVolume::convex(polygon.to_vec());
        super::super::MaterialCast::new(MaterialPrimitive { domain: &domain, to_world: map },
            map.point(Vec2::ZERO).unwrap(), delta, 0.0..1.0)
            .with_initial_overlap(super::super::InitialOverlap::Report).cells(None)?.against_piece(target)
    }

    fn square() -> [Vec2; 4] {
        [Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0)]
    }

    #[test]
    fn a_carried_source_tip_hits_without_its_cut_away_root_box() {
        use crate::frame::{MapConvention, PortalAperture, PortalFrame};
        use crate::frame::thread::{MotionAdmission, PieceBudget, PortalLink};
        let entry = PortalAperture { frame: PortalFrame::fixed(Vec2::ZERO, -Vec2::Y), half_length: 40.0 };
        let exit = PortalAperture { frame: PortalFrame::fixed(Vec2::new(500.0,-200.0), Vec2::Y), half_length: 80.0 };
        let links = [PortalLink::new(entry,exit,0),PortalLink::new(exit,entry,1)];
        let domain = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::splat(4.0)));
        let map = Sim2::new(1.0,Mat2::IDENTITY,Vec2::new(0.0,-12.0)).unwrap();
        let make = || super::super::MaterialCast::new(MaterialPrimitive { domain: &domain,to_world:map },
            Vec2::new(0.0,-12.0),Vec2::Y*10.0,0.0..1.0)
            .with_initial_overlap(super::super::InitialOverlap::Report);
        let target = VolumePiece::whole(CombatVolume::circle(Vec2::new(500.0,-198.0),1.0));
        assert!(make().cells(None).unwrap().against_piece(&target).unwrap().is_none());
        let cells = make().cells(Some(MotionAdmission { links:&links,first:Some(0),
            budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection })).unwrap();
        let hit = cells.against_piece(&target).unwrap().unwrap();
        assert!((hit.toi-0.85).abs()<1.0e-8,"{hit:?}");
        assert_eq!(hit.source_path,vec![0]);
        assert_eq!(hit.source_map.scale(),2.0);
        assert!(hit.source_map.reflected());
        let ghost = VolumePiece::whole(CombatVolume::circle(Vec2::new(0.0,1.0),0.5));
        assert!(make().cells(None).unwrap().against_piece(&ghost).unwrap().is_some());
        assert!(cells.against_piece(&ghost).unwrap().is_none());
    }

    #[test]
    fn a_turning_exit_carries_source_material_into_a_native_circle_between_matching_poses() {
        use crate::frame::{ApertureStep, MapConvention, PortalAperture, PortalFrame};
        use crate::frame::thread::{MotionAdmission, PieceBudget, PortalLink};
        let entry=PortalAperture {frame:PortalFrame::fixed(Vec2::ZERO,-Vec2::Y),half_length:40.0};
        let exit=PortalAperture {frame:PortalFrame::fixed(Vec2::new(500.0,-200.0),Vec2::Y),half_length:80.0};
        let mut links=[PortalLink::new(entry,exit,0),PortalLink::new(exit,entry,1)];
        let domain=CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::splat(4.0)));
        let root=Vec2::new(0.0,-2.0);
        let map=Sim2::new(1.0,Mat2::IDENTITY,root).unwrap();
        let target=VolumePiece::whole(CombatVolume::circle(Vec2::new(500.0,-203.0),0.25));
        let query=|links:&[PortalLink],target:&VolumePiece| super::super::MaterialCast::new(MaterialPrimitive {
            domain:&domain,to_world:map },root,Vec2::ZERO,0.0..1.0).cells(Some(MotionAdmission {
                links,first:Some(0),budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection,
            })).unwrap().against_piece(target);
        assert!(query(&links,&target).unwrap().is_none(),"the matching endpoint poses miss");
        links[0].exit_step=ApertureStep::ending_at(exit.frame,Vec2::ZERO,std::f32::consts::TAU);
        links[1].enter_step=links[0].exit_step;
        let hit=query(&links,&target).unwrap().unwrap();
        assert!(hit.toi>0.0 && hit.toi<0.5,"{hit:?}");
        assert_eq!(hit.source_path,vec![0]);
        assert!((hit.point-Vec2::new(500.0,-203.0)).length()<=0.251,"{hit:?}");
        let miss=VolumePiece::whole(CombatVolume::circle(Vec2::new(507.0,-207.0),0.1));
        assert!(query(&links,&miss).unwrap().is_none(),"the swept envelope is not occupied material");
    }

    #[test]
    fn a_native_circle_is_reached_between_endpoint_poses() {
        for parity in [-1.0, 1.0] {
            let axes = Mat2::from_angle(0.43) * Mat2::from_diagonal(Vec2::new(parity, 1.0));
            let map = Sim2::new(1.7, axes, Vec2::new(1_000_000.0, -500_000.0)).unwrap();
            let circle = CombatVolume::circle(Vec2::new(1.0 / 3.0, 2.0 / 7.0), 3.0);
            let target = VolumePiece::from_material(MaterialPrimitive { domain: &circle, to_world: map }).unwrap();
            let moving = map.shifted(map.direction_wide(bevy_math::DVec2::new(-20.0, 0.0)) * map.scale_wide()).unwrap();
            let delta = map.vector(Vec2::new(40.0, 0.0)).unwrap();
            let hit = sweep_polygon_against_piece(&square(), moving, delta, &target).unwrap().unwrap();
            assert!(hit.toi > 0.0 && hit.toi < 1.0);
            let reference_map = Sim2::new(1.7, axes, Vec2::ZERO).unwrap();
            let reference_target = VolumePiece::from_material(MaterialPrimitive { domain: &circle, to_world: reference_map }).unwrap();
            let reference_moving = reference_map.shifted(reference_map.direction_wide(bevy_math::DVec2::new(-20.0, 0.0)) * reference_map.scale_wide()).unwrap();
            let reference = sweep_polygon_against_piece(&square(), reference_moving, delta, &reference_target).unwrap().unwrap();
            assert!((reference.toi - hit.toi).abs() < 1.0e-8, "reference {reference:?}, translated {hit:?}");
            assert!(hit.normal.dot(delta) < 0.0, "{hit:?}");
            assert!(circle.intersects(&CombatVolume::circle(map.inverse().unwrap().point(hit.point).unwrap(), 0.1)));
            let large=square().map(|point|point*10.0);
            let occupied=sweep_polygon_against_piece(&large,map,Vec2::ZERO,&target).unwrap().unwrap();
            assert!(circle.intersects(&CombatVolume::circle(map.inverse().unwrap().point(occupied.point).unwrap(),0.01)),
                "an occupied start keeps its contact inside the target: {occupied:?}");
        }
    }

    #[test]
    fn a_convex_target_does_not_fill_its_empty_corner() {
        let shape = CombatVolume::convex(vec![Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(0.0, 10.0)]);
        let target = VolumePiece::whole(shape);
        assert!(sweep_polygon_against_piece(&square(), Sim2::new(1.0, Mat2::IDENTITY, Vec2::new(8.0, 8.0)).unwrap(), Vec2::ZERO, &target).unwrap().is_none());
        assert!(sweep_polygon_against_piece(&square(), Sim2::new(1.0, Mat2::IDENTITY, Vec2::new(2.0, 2.0)).unwrap(), Vec2::ZERO, &target).unwrap().is_some());
    }

    #[test]
    fn invalid_polygon_motion_has_a_typed_numeric_refusal() {
        let target = VolumePiece::whole(CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::ONE)));
        assert!(matches!(sweep_polygon_against_piece(&square(), Sim2::IDENTITY, Vec2::splat(f32::NAN), &target).unwrap_err().reason,
            RefusalReason::Numeric(NumericFault::NonFinite)));
    }
}
