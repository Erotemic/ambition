//! A body's semantic rig on the body, and its pose resolved from simulation
//! clocks every tick.
//!
//! [`BodyRig`] is the prepared rig a character grants its bodies.
//! [`BodyRigPose`] is where each joint and attachment IS this tick, solved from
//! the rig and the body's authoritative clocks: the move clock
//! (`MovePlayback`) when a move plays, else the pose clock (`BodyPoseClock`).
//! It is rollback-DERIVED: it holds no fact that a clock does not, and the
//! simulation rebuilds it before any consumer reads it, so a restore needs only
//! one resolver pass.
//!
//! ⛔ The renderer never writes this and never reads a sprite to make it. A hand
//! is where the rig puts it, and the drawing follows the same clocks.

use std::sync::Arc;

use bevy::math::{Affine2, Vec2};
use bevy::prelude::*;

use ambition_characters::actor::body_rig::PreparedBodyRig;

use crate::hurtbox_resolution::{BodyPoseClock, POSE_AIRBORNE, POSE_CROUCH, POSE_HITSTUN};

/// The prepared rig this body's character granted. Shared by every body of
/// the character.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct BodyRig(pub Arc<PreparedBodyRig>);

/// Where this body's joints and attachments are this tick, in its rig space:
/// feet origin, +x the way the art faces, +y down, world units. Place a point
/// in the world with [`BodyRigPose::to_body`] and the body's own frame.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct BodyRigPose {
    /// The clip that was solved, or `None` before the first resolve.
    pub clip: Option<String>,
    pub frame: usize,
    /// One transform per rig joint, in the rig's (parent-first) joint order.
    pub joints: Vec<Affine2>,
    /// One point per rig attachment, in the rig's attachment order.
    pub attachments: Vec<Vec2>,
}

impl BodyRigPose {
    /// Attachment `name` this tick, in rig space.
    pub fn attachment(&self, rig: &PreparedBodyRig, name: &str) -> Option<Vec2> {
        rig.attachment_index(name)
            .and_then(|index| self.attachments.get(index).copied())
    }

    /// This pose's hurt parts as centre-relative hurt volumes for a body
    /// facing the way its art faces, or `None` when the rig has no hurt parts
    /// or this pose is not resolved yet.
    ///
    /// `feet_below_center` is how far the body's feet are below its centre
    /// (half its current height): the rig is feet-anchored, a hurt volume is
    /// placed from the centre. Each part is published as the axis-aligned
    /// bound of its shape at its joint, which is what the damageable-volume
    /// seam speaks; a bound errs toward hittable, never toward invulnerable.
    pub fn hurt_volumes(
        &self,
        rig: &PreparedBodyRig,
        feet_below_center: f32,
    ) -> Option<Vec<ambition_entity_catalog::HurtboxVolume>> {
        if rig.hurt_parts().is_empty() || self.joints.len() != rig.joint_names().len() {
            return None;
        }
        Some(
            rig.hurt_parts()
                .iter()
                .map(|part| {
                    let (min, max) = part.shape.bounds_in(self.joints[usize::from(part.joint)]);
                    let center = (min + max) * 0.5;
                    let half = (max - min) * 0.5;
                    ambition_entity_catalog::HurtboxVolume {
                        shape: ambition_entity_catalog::VolumeShape::Rect {
                            offset: (center.x, center.y + feet_below_center),
                            half_extents: (half.x, half.y),
                        },
                    }
                })
                .collect(),
        )
    }

    /// A rig-space point as an offset from the body's FEET in world axes,
    /// for a body facing `facing` (`< 0` faces left) whose unit DOWN is `down`.
    ///
    /// A body facing left mirrors the rig about its own feet: the same rule
    /// `hurtbox_world_aabb` applies to an authored hurt volume. A body under
    /// turned gravity turns the rig with it, so its head stays away from its
    /// floor. Under default gravity (`down = +y`) this is `(±x, y)`.
    pub fn to_body(point: Vec2, facing: f32, down: Vec2) -> Vec2 {
        let forward = if facing < 0.0 { -1.0 } else { 1.0 };
        let right = Vec2::new(down.y, -down.x);
        right * (point.x * forward) + down * point.y
    }
}

/// The set [`resolve_body_rig_poses`] runs in, published so a consumer orders
/// against a phase rather than against the function.
///
/// A consumer that reads [`BodyRigPose`] (a hurt-part resolver, a hand muzzle)
/// runs after this set. The installer puts it after the move and pose clocks
/// advance and before anything that materializes from an attachment.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyRigPoseResolved;

/// The rig clips a body pose asks for, in preference order. The names are the
/// sheet's row names, which is the vocabulary a rig clip is keyed by.
///
/// Walking is not a body pose: the pose clock reads `idle` for a body that
/// stands or walks, so a walking body resolves the `idle` clip. A locomotion
/// clip needs a simulation fact that says the body walks; the presentation
/// picker's speed threshold is not one.
pub fn pose_clip_chain(pose: &str) -> &'static [&'static str] {
    match pose {
        POSE_HITSTUN => &["hurt", "idle"],
        POSE_CROUCH => &["crouch", "idle"],
        POSE_AIRBORNE => &["jump", "idle"],
        _ => &["idle"],
    }
}

/// Which clip and frame a body shows, from its authoritative clocks.
///
/// A playing move outranks the body pose, and its clip is slaved to the move's
/// progress: the same precedence and the same slaving the sheet row follows.
/// A rig with none of the asked clips falls back to `idle`, then to its first
/// clip, so a rig always resolves some pose.
pub fn select_rig_frame<'a>(
    rig: &'a PreparedBodyRig,
    active_move: Option<(&'a ambition_entity_catalog::ClipBinding, f32)>,
    pose: Option<(&str, f32)>,
) -> Option<(&'a str, usize)> {
    if let Some((binding, phase)) = active_move {
        let chain = std::iter::once(binding.clip.as_str())
            .chain(binding.fallbacks.iter().map(String::as_str));
        if let Some((name, clip)) = rig.first_clip(chain) {
            return Some((name, clip.frame_at_phase(phase)));
        }
    }
    let (pose_id, elapsed_s) = pose.unwrap_or(("idle", 0.0));
    let (name, clip) = rig
        .first_clip(pose_clip_chain(pose_id).iter().copied())
        .or_else(|| rig.first_clip(rig.clip_names()))?;
    Some((name, clip.frame_at_time(elapsed_s)))
}

/// Solve every rigged body's pose for this tick.
///
/// Writes only when the selected clip or frame changed, so a held pose does
/// not trip `Changed<BodyRigPose>` every tick.
pub fn resolve_body_rig_poses(
    mut bodies: Query<(
        &BodyRig,
        Option<&crate::moveset::MovePlayback>,
        Option<&BodyPoseClock>,
        &mut BodyRigPose,
    )>,
) {
    for (rig, playback, pose_clock, mut pose) in &mut bodies {
        let rig = rig.0.as_ref();
        let active_move = playback.map(|playback| (&playback.spec.clip, playback.phase()));
        let pose_input = pose_clock.map(|clock| (clock.pose.as_str(), clock.elapsed_s));
        let Some((clip, frame)) = select_rig_frame(rig, active_move, pose_input) else {
            continue;
        };
        if pose.clip.as_deref() == Some(clip) && pose.frame == frame && !pose.joints.is_empty() {
            continue;
        }
        let pose = pose.as_mut();
        rig.solve(clip, frame, &mut pose.joints);
        pose.attachments.clear();
        pose.attachments.extend(rig.attachments().iter().map(|attachment| {
            pose.joints[usize::from(attachment.joint)].transform_point2(attachment.offset)
        }));
        pose.clip = Some(clip.to_string());
        pose.frame = frame;
    }
}

#[cfg(test)]
mod tests;
