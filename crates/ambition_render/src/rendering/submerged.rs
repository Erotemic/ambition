//! A body under the stage is not drawn.
//!
//! ⭐⭐ THE SECOND MODAL BODY MORPH, and the file next door already predicted
//! it: `morph_ball.rs` ends with *"generalize modal body morphs — that is what
//! this means, and it deletes this whole file."* This is not that
//! generalization; it is the second customer, kept deliberately in the same
//! shape so the eventual generalization has two examples to be right about
//! rather than one.
//!
//! ⛔⛔ IT RUNS AFTER THE MORPH-BALL SYNC AND IS THE LAST WORD. Both systems
//! restore a hidden body to `Inherited`, and morph-ball's restore is
//! unconditional on "not morphed" — so a body hidden HERE and read THERE on the
//! same frame would be handed back to the renderer visible, standing under the
//! stage in full view. Ordering is the fix rather than teaching morph-ball about
//! submersion, because the thing morph-ball is wrong about is that it believes
//! it is the only mode that hides a body.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;

/// Hide every submerged body, and hand every other one back.
///
/// ⛔ `Inherited` ON THE WAY OUT, NEVER A HARD `Visible`. A death overlay or a
/// room-transition fade hides bodies through the parent; overriding to `Visible`
/// would make a fighter who happened to surface mid-fade the one thing still on
/// screen. Morph-ball states the same rule for the same reason.
pub fn sync_submerged_visibility(
    mut bodies: Query<(&ambition_sim_view::BodyPoseView, &mut Visibility), With<PlayerVisual>>,
) {
    for (pose, mut visibility) in &mut bodies {
        if pose.submerged {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        } else if matches!(*visibility, Visibility::Hidden) {
            *visibility = Visibility::Inherited;
        }
    }
}

#[cfg(test)]
mod tests;
