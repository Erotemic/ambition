//! A translating polygon against one sampled material region.

use super::{InitialOverlap, translation};
use crate::frame::thread::{Refusal, RefusalReason, VolumePiece};
use crate::frame::{NumericFault, Sim2};
use crate::Vec2;

/// Contact in the physical chart, with the sampled target's portal path.
#[derive(Clone, Debug, PartialEq)]
pub struct SampledPieceHit {
    pub toi: f64,
    pub point: Vec2,
    /// Direction from the sampled target into the moving polygon.
    pub normal: Vec2,
    pub target_path: Vec<u32>,
    /// Map from the target's root chart to the contacting physical chart.
    pub target_map: Sim2,
    /// Accepted polygon material at contact on the original sweep.
    pub source_map: Sim2,
}

/// Sweep a convex polygon through one fixed material region.
/// Vertices are canonical; `to_world` places their start pose. `delta` covers
/// the full normalized interval. Keep curved target boundaries and all cuts.
pub fn sweep_polygon_against_piece(
    polygon: &[Vec2],
    to_world: Sim2,
    delta: Vec2,
    target: &VolumePiece,
) -> Result<Option<SampledPieceHit>, Refusal> {
    let numeric = |fault| {
        let mut path = target.path.clone();
        let link = path.pop();
        Refusal { path, link, reason: RefusalReason::Numeric(fault) }
    };
    if !delta.is_finite() || polygon.iter().any(|point| !point.is_finite()) {
        return Err(numeric(NumericFault::NonFinite));
    }
    if polygon.len() < 3 {
        return Err(numeric(NumericFault::InvalidFrame));
    }
    let points: Vec<_> = polygon.iter().map(|point| point.as_dvec2()).collect();
    let origin = points[0];
    let area: f64 = super::edges(&points).map(|(a, b)| (a - origin).perp_dot(b - origin)).sum();
    if area == 0.0 { return Err(numeric(NumericFault::InvalidFrame)); }
    let winding = area.signum();
    for (a, b) in super::edges(&points) {
        let edge = b - a;
        if points.iter().any(|point| {
            let arm = *point - a;
            let rounding = edge.abs().dot(arm.abs()) * f64::EPSILON * 16.0;
            edge.perp_dot(arm) * winding < -rounding
        }) { return Err(numeric(NumericFault::InvalidFrame)); }
    }
    super::super::thread::map_volume(target.source(), &target.transform()).map_err(numeric)?;
    let inverse = target.transform().inverse().map_err(numeric)?;
    let region = polygon.iter().map(|point| {
        // Validate the physical projection without using it as source data.
        to_world.point(*point)?;
        Ok(inverse.point_wide(to_world.point_wide(point.as_dvec2())))
    }).collect::<Result<Vec<_>, NumericFault>>().map_err(numeric)?;
    let motion = inverse.direction_wide(delta.as_dvec2()) * inverse.scale_wide();
    let hit = translation::cast(
        target.source(),
        bevy_math::DVec2::ZERO,
        &region,
        motion,
        target.source_bounds().iter().copied().map(translation::MovingPlane::from),
        InitialOverlap::Report,
    ).map_err(numeric)?;
    hit.map(|hit| Ok(SampledPieceHit {
        toi: hit.time,
        point: target.transform().point_from_wide(hit.point).map_err(numeric)?,
        normal: -target.transform().direction(hit.normal.as_vec2()).normalize_or_zero(),
        target_path: target.path.clone(),
        target_map: target.path_transform().map_err(numeric)?,
        source_map: to_world.shifted(delta.as_dvec2() * hit.time).map_err(numeric)?,
    })).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Aabb, CombatVolume};
    use crate::frame::thread::MaterialPrimitive;
    use bevy_math::Mat2;

    fn square() -> [Vec2; 4] {
        [Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0)]
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
