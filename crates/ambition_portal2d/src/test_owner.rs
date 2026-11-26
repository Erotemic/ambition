//! A minimal motion owner for portal tests. Transit moves no root (portal
//! M2): a body's motion owner takes the crossing in its own step. A test body
//! with a recorded step here takes the first crossing in it the way an item
//! or a shot does (`prepare_body_crossing`, `commit_body_transfer`) and comes
//! out where its whole step ends through the pair's map. A body the
//! kernel steps (`BodyFrameOutput`) is the kernel's.

use bevy::prelude::*;
use ambition_platformer2d_core::{self as ae, BodyKinematics, SweepSample};
use ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry;
use ambition_platformer2d_shared_tangle::frame_env::CarriedAcceleration;

#[allow(clippy::type_complexity)]
pub fn cross_recorded_steps(
    mut commands: Commands,
    admitted: Option<Res<crate::AdmittedPortals>>,
    live: crate::rooms::PortalRooms,
    geometry: Option<Res<BodyGeometry>>,
    tuning: Res<crate::PortalTuning>,
    mut bodies: Query<(Entity, &mut BodyKinematics, &SweepSample, Option<&mut CarriedAcceleration>,
        Option<&mut crate::PortalTransit>, Option<&crate::PortalBackside>),
        (Without<ae::PoseOwnedExternally>, Without<ae::BodyInStorage>, Without<ae::BodyFrameOutput>)>,
) {
    let Some(admitted) = admitted else { return; };
    for (entity, mut kin, record, acceleration, transit, backside) in &mut bodies {
        let Some(record) = record.ending_at(kin.pos).copied() else { continue; };
        let room = live.of(entity);
        let all = admitted.0.in_room(room);
        let Ok(domain) = kin.canonical_box() else { continue; };
        let Ok(cast) = record.material_cast(&domain) else { continue; };
        let Ok(Some(hit)) = crate::first_portal_crossing_through(&cast, all, backside.copied(), 1.0) else { continue; };
        let admission = geometry.as_deref().and_then(|geometry| geometry.motion_for(entity, room));
        let at = BodyKinematics { pos: hit.root.point, ..*kin };
        let Ok((prepared, _)) = crate::prepare_body_crossing(at, acceleration.as_deref().map(|a| a.0), hit, all,
            admission, 0, &tuning) else { continue; };
        let end = prepared.transfer().map.point(record.curr).unwrap();
        crate::commit_body_transfer(&mut commands, entity, &mut kin, acceleration.map(|a| a.into_inner()),
            transit.map(|t| t.into_inner()), prepared, tuning.teleport_cooldown_s);
        kin.pos = end;
    }
}
