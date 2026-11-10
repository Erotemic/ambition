//! External body pose admission and prepared physical changes.

use bevy::prelude::Entity;
use ambition_platformer2d_core::frame::thread::Refusal;
use super::geometry::BodyGeometry;

/// Admission context for a body whose pose an external owner controls.
#[derive(bevy::ecs::system::SystemParam)]
pub struct BodyPoseAdmission<'w, 's> {
    geometry: Option<bevy::prelude::Res<'w, BodyGeometry>>,
    live: crate::lifecycle::LiveRooms<'w, 's>,
    acceleration: bevy::prelude::Query<'w, 's, &'static crate::frame_env::CarriedAcceleration>,
}

impl BodyPoseAdmission<'_, '_> {
    pub fn constrain(
        &self,
        body: Entity,
        kinematics: &mut ambition_platformer2d_core::BodyKinematics,
        sweep: Option<&mut ambition_platformer2d_core::SweepSample>,
        desired: ambition_platformer2d_core::BodyKinematics,
    ) -> Result<(), Refusal> {
        self.prepare(body, kinematics, sweep.as_deref(), desired)?.commit(kinematics, sweep);
        Ok(())
    }

    /// Prepare one member of a coordinated pose change. No physical state changes.
    pub fn prepare(
        &self,
        body: Entity,
        kinematics: &ambition_platformer2d_core::BodyKinematics,
        sweep: Option<&ambition_platformer2d_core::SweepSample>,
        desired: ambition_platformer2d_core::BodyKinematics,
    ) -> Result<ambition_platformer2d_core::movement::PreparedBodyPose, Refusal> {
        let carried = self.acceleration.get(body).ok()
            .map_or(&[][..], |acceleration| std::slice::from_ref(&acceleration.0));
        ambition_platformer2d_core::movement::prepare_body_pose(kinematics, sweep, desired,
            self.geometry.as_deref().and_then(|geometry| geometry.motion_for(body, self.live.of(body))), carried)
    }
}

#[cfg(test)]
mod tests;
