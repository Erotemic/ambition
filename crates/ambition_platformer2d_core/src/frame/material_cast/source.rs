//! One recorded rigid source curve for geometry, roots and payloads.

use bevy_math::{DMat2, DVec2};
use super::{Aabb, NumericFault, Sim2};

#[derive(Clone, Copy, Debug)]
pub(in crate::frame) struct Rotation {
    pub pivot: DVec2,
    pub turn: f64,
    pub spin: f64,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::frame) struct SourceMotion {
    pub map: Sim2,
    pub root: DVec2,
    pub delta: DVec2,
    pub rotation: Option<Rotation>,
}

pub(super) fn perp(point: DVec2) -> DVec2 { DVec2::new(-point.y, point.x) }

/// Keep the displacement from a small turn about a distant pivot.
pub(in crate::frame) fn rotation_delta(angle: f64, point: DVec2) -> DVec2 {
    perp(point) * angle.sin() - point * (2.0 * (angle * 0.5).sin().powi(2))
}

impl SourceMotion {
    pub fn turn(self) -> f64 { self.rotation.map_or(0.0, |rotation| rotation.turn) }
    pub fn root_arm(self) -> DVec2 { self.rotation.map_or(DVec2::ZERO, |rotation| self.root - rotation.pivot) }

    pub fn root_at(self, time: f64) -> DVec2 {
        match self.rotation {
            Some(rotation) => self.root + self.delta * time
                + rotation_delta(rotation.turn * time, self.root_arm()),
            None => self.root + self.delta * time,
        }
    }

    pub fn root_velocity(self, time: f64) -> DVec2 {
        self.delta + perp(DMat2::from_angle(self.turn() * time) * self.root_arm()) * self.turn()
    }

    pub fn map_at(self, time: f64) -> Result<Sim2, NumericFault> {
        let Some(rotation) = self.rotation else { return self.map.shifted(self.delta * time); };
        let turn = DMat2::from_angle(rotation.turn * time);
        let axes = DMat2::from_cols(turn * self.map.direction_wide(DVec2::X), turn * self.map.direction_wide(DVec2::Y));
        let origin = self.map.translation() + self.delta * time
            + rotation_delta(rotation.turn * time, self.map.translation() - rotation.pivot);
        Sim2::from_wide_translation(self.map.scale_wide(), axes, origin)
    }

    /// Derivative of the map's origin in the original source parameter.
    pub fn map_velocity(self, time: f64) -> DVec2 {
        self.delta + self.rotation.map_or(DVec2::ZERO, |rotation| {
            perp(DMat2::from_angle(rotation.turn * time) * (self.map.translation() - rotation.pivot)) * rotation.turn
        })
    }

    pub fn angle_bound(self, time: f64, half: f64) -> f64 {
        if half == 0.0 || self.turn() == 0.0 { return 0.0; }
        (self.turn().abs() * half * (1.0 + f64::EPSILON * 8.0)
            + self.turn().abs() * time.abs() * f64::EPSILON * 16.0).min(2.0)
    }

    pub fn root_variation(self, time: f64, half: f64) -> f64 {
        self.delta.length() * half + self.angle_bound(time, half) * self.root_arm().length()
    }

    pub fn variation(self, body: Aabb, time: f64, half: f64) -> f64 {
        let Some(rotation) = self.rotation else { return self.delta.length() * half; };
        let center = (body.min.as_dvec2() + body.max.as_dvec2()) * 0.5;
        let radius = (body.max.as_dvec2() - body.min.as_dvec2()).length() * 0.5;
        let reach = (self.map.translation() - rotation.pivot
            + self.map.direction_wide(center) * self.map.scale_wide()).length()
            + radius * self.map.scale_wide();
        self.delta.length() * half + self.angle_bound(time, half) * reach
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CombatVolume, Vec2};
    use crate::frame::{ApertureStep, MapConvention, PortalAperture, PortalFrame};
    use crate::frame::material_cast::{MaterialCast, MaterialCastHit};
    use crate::frame::thread::{MaterialPrimitive, MotionAdmission, PieceBudget, PortalLink, RefusalReason};

    fn first(cells: &super::super::MaterialCastCells<'_>, target: Aabb) -> Option<MaterialCastHit> {
        let points = super::super::corners(target);
        cells.parts().iter().filter_map(|part| cells.against(part, &points, Vec2::ZERO).unwrap())
            .min_by(|a,b| a.toi.total_cmp(&b.toi))
    }

    #[test]
    fn a_source_full_turn_hits_a_wall_that_both_endpoint_poses_miss() {
        let body = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::new(5.0, 0.5)));
        let target = Aabb::new(Vec2::new(0.0, 4.0), Vec2::splat(0.25));
        let make = || MaterialCast::new(MaterialPrimitive::world(&body), Vec2::ZERO, Vec2::ZERO, 0.0..1.0)
            .rotating_about(Vec2::ZERO, std::f64::consts::TAU, 1.0).unwrap();
        let cells = make().cells(None).unwrap();
        let hit = first(&cells, target).expect("the rotating source must meet the wall");
        assert!(hit.toi > 0.1 && hit.toi < 0.25, "{hit:?}");
        assert!((hit.world_point.y - target.min.y).abs() < 1.0e-4, "{hit:?}");
        let normal = DMat2::from_angle(std::f64::consts::TAU * hit.toi) * -DVec2::Y;
        assert!(hit.normal.as_dvec2().distance(normal) < 1.0e-4,
            "the owner normal must use the source pose at contact: {hit:?}");
        assert!(first(&make().prefix(0.05).unwrap().cells(None).unwrap(), target).is_none());
    }

    #[test]
    fn an_orbiting_circle_keeps_its_analytic_empty_corner() {
        let body = CombatVolume::circle(Vec2::new(5.0, 0.0), 0.5);
        let make = || MaterialCast::new(MaterialPrimitive::world(&body), Vec2::new(5.0, 0.0), Vec2::ZERO, 0.0..1.0)
            .rotating_about(Vec2::ZERO, std::f64::consts::TAU, 1.0).unwrap();
        assert!(first(&make().cells(None).unwrap(), Aabb::new(Vec2::new(0.0, 5.0), Vec2::splat(0.1))).is_some());
        assert!(first(&make().cells(None).unwrap(), Aabb::new(Vec2::new(4.5, 4.5), Vec2::splat(0.1))).is_none());
    }

    #[test]
    fn a_source_turn_refuses_a_portal_cut_between_clear_endpoint_poses() {
        let body = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::new(0.5, 5.0)));
        let links = [PortalLink::new(
            PortalAperture { frame: PortalFrame::fixed(Vec2::new(4.0,0.0), -Vec2::X), half_length: 10.0 },
            PortalAperture { frame: PortalFrame::fixed(Vec2::new(100.0,0.0), Vec2::X), half_length: 10.0 }, 7)];
        let admission = MotionAdmission { links: &links, first: None,
            budget: PieceBudget { max_hops: 0, ..PieceBudget::COMPOSED }, convention: MapConvention::Reflection };
        let cast = MaterialCast::new(MaterialPrimitive::world(&body), Vec2::ZERO, Vec2::ZERO, 0.0..1.0)
            .rotating_about(Vec2::ZERO, std::f64::consts::TAU, 1.0).unwrap();
        let refusal = admission.admit(cast, Vec2::ZERO, &[]).unwrap_err();
        assert_eq!(refusal.reason, RefusalReason::HopLimit);
        assert_eq!(refusal.link, Some(7));
    }

    #[test]
    fn an_orbiting_root_crosses_at_its_recorded_time() {
        let body = CombatVolume::circle(Vec2::new(5.0, 0.0), 0.5);
        let end = PortalAperture { frame: PortalFrame::fixed(Vec2::new(0.0,4.0), -Vec2::Y), half_length: 6.0 };
        let step = ApertureStep::fixed(end.frame.origin, end.frame.normal);
        let cast = MaterialCast::new(MaterialPrimitive::world(&body), Vec2::new(5.0,0.0), Vec2::ZERO, 0.0..1.0)
            .rotating_about(Vec2::ZERO, std::f64::consts::TAU, 1.0).unwrap();
        let hit = cast.crossing(step, end, 1.0).unwrap().expect("the orbit crosses the opening");
        assert!((hit.toi - 0.8_f64.asin() / std::f64::consts::TAU).abs() < 1.0e-12, "{hit:?}");
        assert!(hit.point.distance(Vec2::new(3.0,4.0)) < 1.0e-5, "{hit:?}");
    }

    #[test]
    fn a_root_fit_uses_the_source_orientation_at_crossing() {
        let body = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::new(4.0,40.0)));
        let from = Vec2::new(0.0,-10.0);
        let material = MaterialPrimitive { domain: &body,
            to_world: Sim2::new(1.0, bevy_math::Mat2::from_angle(-std::f32::consts::FRAC_PI_2), from).unwrap() };
        let cast = MaterialCast::new(material, from, Vec2::Y * 20.0, 0.0..1.0)
            .rotating_about(from, std::f64::consts::PI, 1.0).unwrap();
        let end = PortalAperture { frame: PortalFrame::fixed(Vec2::ZERO,-Vec2::Y), half_length: 10.0 };
        let hit = cast.crossing(ApertureStep::fixed(end.frame.origin,end.frame.normal),end,1.0).unwrap().unwrap();
        assert_eq!(hit.toi,0.5);
    }

    #[test]
    fn a_spinning_source_validates_velocity_away_from_its_root_without_portals() {
        let body = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::new(10.0,1.0)));
        let cast = MaterialCast::new(MaterialPrimitive::world(&body),Vec2::ZERO,Vec2::ZERO,0.0..1.0)
            .rotating_about(Vec2::ZERO,std::f64::consts::TAU,std::f64::consts::TAU / f64::from(f32::MAX)).unwrap();
        let admission = MotionAdmission { links: &[], first: None, budget: PieceBudget::COMPOSED, convention: MapConvention::Reflection };
        assert_eq!(admission.admit(cast,Vec2::ZERO,&[]).unwrap_err().reason,
            RefusalReason::Numeric(NumericFault::Overflow));
    }

    #[test]
    fn a_later_winding_can_fit_after_the_first_source_pose_does_not() {
        let body = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::new(15.0, 1.0)));
        let root = Vec2::new(0.0,5.0);
        let material = MaterialPrimitive { domain: &body,
            to_world: Sim2::new(1.0, bevy_math::Mat2::from_angle(std::f32::consts::FRAC_PI_4), root).unwrap() };
        let cast = MaterialCast::new(material,root,Vec2::ZERO,0.0..1.0)
            .rotating_about(root,std::f64::consts::PI,1.0).unwrap();
        let end = PortalAperture { frame: PortalFrame::fixed(Vec2::ZERO,Vec2::X),half_length:10.0 };
        let step = ApertureStep::ending_at(end.frame,Vec2::ZERO,2.0 * std::f32::consts::TAU);
        let hit = cast.crossing(step,end,1.0).unwrap().expect("the later source pose fits");
        assert!((hit.toi - 0.75).abs() < 1.0e-6, "{hit:?}");
    }

    #[test]
    fn a_rotating_source_prefix_keeps_its_original_world_clock() {
        let body = CombatVolume::circle(Vec2::new(5.0,0.0),0.5);
        let end = PortalAperture { frame: PortalFrame::fixed(Vec2::new(0.0,4.0),-Vec2::Y),half_length:6.0 };
        let step = ApertureStep::fixed(end.frame.origin,end.frame.normal);
        let make = || MaterialCast::new(MaterialPrimitive::world(&body),Vec2::new(5.0,0.0),Vec2::ZERO,0.25..0.75)
            .rotating_about(Vec2::ZERO,std::f64::consts::TAU,0.5).unwrap();
        assert!(make().prefix(0.1).unwrap().crossing(step,end,1.0).unwrap().is_none());
        let hit = make().prefix(0.2).unwrap().crossing(step,end,1.0).unwrap().unwrap();
        assert!((hit.toi - 0.8_f64.asin() / std::f64::consts::TAU).abs() < 1.0e-12);
        assert_eq!(hit.world_time, 0.25 + 0.5 * hit.toi);
    }


    #[test]
    fn a_small_turn_about_a_distant_pivot_keeps_its_radial_displacement() {
        let source = SourceMotion { map: Sim2::IDENTITY,root:DVec2::ZERO,delta:DVec2::ZERO,
            rotation:Some(Rotation { pivot:DVec2::new(1.0e15,0.0),turn:1.0e-8,spin:1.0e-8 }) };
        let root = source.root_at(1.0);
        assert!((root.x - 0.05).abs() < 1.0e-15, "{root:?}");
        assert_eq!(root,source.map_at(1.0).unwrap().translation());
        assert_eq!(root.y,-1.0e7);
    }


    #[test]
    fn a_small_turn_about_a_distant_pivot_can_cross_a_nearby_plane() {
        let body = CombatVolume::circle(Vec2::ZERO,0.5);
        let cast = MaterialCast::new(MaterialPrimitive::world(&body),Vec2::ZERO,Vec2::ZERO,0.0..1.0)
            .rotating_about(Vec2::new(1.0e15,0.0),1.0e-8,1.0).unwrap();
        let end = PortalAperture { frame: PortalFrame::fixed(Vec2::new(0.025,-7.0e6),-Vec2::X),half_length:1.0e8 };
        let hit = cast.crossing(ApertureStep::fixed(end.frame.origin,end.frame.normal),end,1.0).unwrap().unwrap();
        assert!((hit.toi - 0.5_f64.sqrt()).abs() < 1.0e-6,"{hit:?}");
        assert!((hit.point.x - 0.025).abs() < 1.0e-7,"{hit:?}");
    }


    #[test]
    fn a_source_turn_without_a_game_policy_has_no_implicit_scale_cap() {
        let body = CombatVolume::circle(Vec2::ZERO,0.5);
        let material = MaterialPrimitive { domain: &body,
            to_world: Sim2::new(8192.0,bevy_math::Mat2::IDENTITY,Vec2::ZERO).unwrap() };
        let make = || MaterialCast::new(material,Vec2::ZERO,Vec2::ZERO,0.0..1.0)
            .rotating_about(Vec2::ZERO,std::f64::consts::TAU,1.0).unwrap();
        assert_eq!(make().cells(None).unwrap().parts().len(),1);
        let admission = MotionAdmission { links: &[],first:None,budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection };
        assert!(matches!(make().cells(Some(admission)),Err(refusal) if refusal.reason == RefusalReason::ScaleLimit));
    }


    #[test]
    fn a_source_turn_and_a_moving_exit_share_the_material_velocity_derivative() {
        let root = Vec2::new(0.0,-3.0);
        let body = CombatVolume::circle(root,0.5);
        let enter = PortalAperture { frame:PortalFrame::fixed(Vec2::ZERO,Vec2::Y),half_length:10.0 };
        let mut exit = PortalAperture { frame:PortalFrame::fixed(Vec2::new(100.0,0.0),-Vec2::Y),half_length:10.0 };
        exit.frame.pivot = Vec2::new(100.0,-3.0);
        exit.frame.spin = (2.0 * std::f64::consts::TAU) as f32;
        let mut link = PortalLink::new(enter,exit,7);
        link.exit_step = ApertureStep::ending_at(exit.frame,Vec2::ZERO,2.0 * std::f32::consts::TAU);
        let links = [link];
        let admission = MotionAdmission { links:&links,first:Some(7),budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection };
        let cells = MaterialCast::new(MaterialPrimitive::world(&body),root,Vec2::ZERO,0.0..1.0)
            .rotating_about(root,std::f64::consts::TAU,1.0).unwrap().cells(Some(admission)).unwrap();
        let part = cells.parts().iter().find(|part|part.path == [7]).unwrap();
        let point = root.as_dvec2() + DVec2::X * 0.3;
        let time = 0.31;
        let h = 1.0e-6;
        let difference = (part.at(time+h).unwrap().point_wide(point) - part.at(time-h).unwrap().point_wide(point)) / (2.0*h);
        assert!(part.velocity(point,time).unwrap().distance(difference) < 1.0e-6,"{difference:?}");
    }

    #[test]
    fn unoccupied_source_chart_speeds_do_not_refuse_finite_physical_material() {
        let root = Vec2::new(0.0,-3.0);
        let body = CombatVolume::circle(root,2.0);
        let enter = PortalAperture { frame:PortalFrame::fixed(Vec2::ZERO,Vec2::Y),half_length:10.0 };
        let mut exit = PortalAperture { frame:PortalFrame::fixed(Vec2::new(100.0,0.0),-Vec2::Y),half_length:10.0 };
        exit.frame.pivot = Vec2::new(100.0,-3.0);
        exit.frame.spin = f32::MAX;
        let mut link = PortalLink::new(enter,exit,7);
        link.exit_step = ApertureStep::ending_at(exit.frame,Vec2::ZERO,std::f32::consts::TAU);
        let links = [link];
        let admission = MotionAdmission { links:&links,first:Some(7),budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection };
        let duration = f64::from(std::f32::consts::TAU) / f64::from(f32::MAX);
        let cast = MaterialCast::new(MaterialPrimitive::world(&body),root,Vec2::ZERO,0.0..1.0)
            .rotating_about(root,f64::from(std::f32::consts::TAU),duration).unwrap();
        admission.admit(cast,Vec2::ZERO,&[]).expect("the reflected source spin cancels the exit spin");
    }


    #[test]
    fn source_rotation_contact_is_covariant_under_scale_parity_and_a_common_boost() {
        let body = CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::new(5.0,0.5)));
        let target = Aabb::new(Vec2::new(0.0,4.0),Vec2::splat(0.25));
        let original = MaterialCast::new(MaterialPrimitive::world(&body),Vec2::ZERO,Vec2::ZERO,0.0..1.0)
            .rotating_about(Vec2::ZERO,std::f64::consts::TAU,1.0).unwrap().cells(None).unwrap();
        let expected = first(&original,target).unwrap();
        for scale in [0.5,2.0] {
            for reflected in [false,true] {
                let axes = bevy_math::Mat2::from_cols(Vec2::Y,if reflected {Vec2::X} else {-Vec2::X});
                let map = Sim2::new(scale,axes,Vec2::new(2000.0,1000.0)).unwrap();
                let root = map.point(Vec2::ZERO).unwrap();
                let boost = Vec2::new(40.0,-20.0);
                let cells = MaterialCast::new(MaterialPrimitive { domain:&body,to_world:map },root,boost,0.0..1.0)
                    .rotating_about(root,if reflected {-std::f64::consts::TAU} else {std::f64::consts::TAU},1.0)
                    .unwrap().cells(None).unwrap();
                let obstacle = super::super::corners(target).map(|point|map.point(point).unwrap()+boost);
                let hit = cells.parts().iter().filter_map(|part|cells.against(part,&obstacle,boost).unwrap())
                    .min_by(|a,b|a.toi.total_cmp(&b.toi)).unwrap();
                assert!((hit.toi-expected.toi).abs()<1.0e-9,"{hit:?}");
                let point = map.point(expected.world_point).unwrap()+boost*hit.toi as f32;
                assert!(hit.world_point.distance(point)<1.0e-3,"{hit:?}");
                assert!(hit.normal.distance(map.direction(expected.normal))<1.0e-5,"{hit:?}");
            }
        }
    }

}
