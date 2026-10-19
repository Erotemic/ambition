//! Whether a body's own presentation draws.
//!
//! ONE system decides it, from the body's pose. A visual that takes the
//! body's place (the morph ball, the trapdoor) is its own entity and does not
//! write the body's `Visibility`.
//!
//! ⛔ Two writers was the defect. The morph-ball sync hid a morphed body, and
//! the submerged sync, which ran after it, set every hidden body that was not
//! submerged back to `Inherited`. So the standing sprite, and the rig that
//! follows it (`rigged::root_shows`), drew behind the ball on each frame.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;

#[cfg(test)]
mod tests;

/// Whether `pose` takes the body's own presentation off the screen: a body
/// that is morphed (the ball draws instead), under the stage (the trapdoor
/// draws instead) or knocked out of the world.
pub fn body_presentation_hidden(pose: &ambition_sim_view::BodyPoseView) -> bool {
    pose.morph_ball || pose.submerged || pose.out_of_the_world
}

/// Hide each body whose pose takes its presentation away, and hand every
/// other one back.
///
/// Restore to `Inherited`, never `Visible`. A death overlay or a
/// room-transition fade hides bodies through the parent; `Visible` would leave
/// a surfacing fighter as the only thing on screen. A body that is not hidden
/// is left as it is, so this system never turns another owner's `Visible`
/// into `Inherited`.
pub fn sync_body_visibility(
    mut bodies: Query<(&ambition_sim_view::BodyPoseView, &mut Visibility), With<PlayerVisual>>,
) {
    for (pose, mut visibility) in &mut bodies {
        if body_presentation_hidden(pose) {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        } else if matches!(*visibility, Visibility::Hidden) {
            *visibility = Visibility::Inherited;
        }
    }
}
