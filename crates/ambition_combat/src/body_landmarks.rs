//! The one landmark query: where a body's hand, head or foot is.
//!
//! A consumer (a pet gesture, a muzzle, a rider's grip) asks [`BodyLandmarks`]
//! for a [`Landmark`] of a body. Two sources can answer, and the consumer does
//! not know which did (ruling Q41,
//! `docs/planning/engine/runtime-rigged-sprite-animation.md`):
//!
//! 1. the body's rig ([`BodyRig`]), from its solved pose;
//! 2. the landmark table its character's art package publishes
//!    (`ambition_sprite_sheet::baked_landmarks`), at the scale the body's art
//!    is drawn at.
//!
//! The rig outranks the table. A body that neither answers gives `None`, and
//! the consumer states its own fallback: there is no silent default offset.
//!
//! ⛔ Every input is a SIMULATION fact: the body's clocks, its rig, its worn
//! character, and its drawn scale (`SpritePosedBody`, `ActorRenderSize`), all
//! rollback state. Nothing reads a render transform. So an answer needs no
//! state of its own and is the same after a restore.
//!
//! Space: an answer is in the body's rig space (feet origin, +x the way the
//! body faces, +y down, world units), or in the world.

use bevy::ecs::system::SystemParam;
use bevy::math::Vec2;
use bevy::prelude::*;

use ambition_characters::actor::body_rig::PreparedBodyRig;
use ambition_characters::actor::{BodyLandmarkTable, Landmark, WornCharacter};
use ambition_characters::prepared::PreparedCharacterRegistry;
use ambition_platformer2d_core as ae;

use crate::body_rig::{select_pose_frame, BodyRig, BodyRigPose};
use crate::hurtbox_resolution::{BodyPoseClock, Gait};

/// The pose a landmark is asked in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LandmarkPose<'a> {
    /// The pose the body's clocks select this tick.
    ThisTick,
    /// The first clip of `chain` the source has, at normalized progress in
    /// `[0, 1]`: the pose of a gesture a script plays or will play, which the
    /// body's clocks do not hold. The chain is the row chain the gesture is
    /// drawn from, so the answer is of the row on screen.
    Clip { chain: &'a [&'a str], phase: f32 },
}

/// The body clocks that select this tick's pose: the move clock, then the
/// pose clock ([`select_pose_frame`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct PoseClocks<'a> {
    pub active_move: Option<(&'a ambition_entity_catalog::ClipBinding, f32)>,
    pub pose: Option<(&'a str, f32)>,
    pub gait: Option<(Gait, f32)>,
}

/// `landmark` from a rig, in rig space.
///
/// This tick's pose is the body's resolved [`BodyRigPose`]. A named clip is
/// solved here: a script asks for it a few times, not every tick.
pub fn rig_landmark(
    landmark: Landmark,
    pose: LandmarkPose<'_>,
    rig: &PreparedBodyRig,
    resolved: Option<&BodyRigPose>,
) -> Option<Vec2> {
    match pose {
        LandmarkPose::ThisTick => resolved?.attachment(rig, landmark.attachment()),
        LandmarkPose::Clip { chain, phase } => {
            let attachment = &rig.attachments()[rig.attachment_index(landmark.attachment())?];
            let (name, clip) = rig.first_clip(chain.iter().copied())?;
            let mut joints = Vec::new();
            rig.solve(name, clip.frame_at_phase(phase), &mut joints)
                .then(|| joints[usize::from(attachment.joint)].transform_point2(attachment.offset))
        }
    }
}

/// `landmark` from a package's table, in rig space.
///
/// `world_per_pixel` is the world size of one sheet pixel of THIS body: the
/// table is in sheet pixels, and two bodies of one character can be drawn at
/// two sizes.
pub fn package_landmark(
    landmark: Landmark,
    pose: LandmarkPose<'_>,
    table: &BodyLandmarkTable,
    world_per_pixel: f32,
    clocks: PoseClocks<'_>,
) -> Option<Vec2> {
    let pixels = match pose {
        LandmarkPose::ThisTick => {
            let (clip, frame) = select_pose_frame(table, clocks.active_move, clocks.pose, clocks.gait)?;
            table.point(landmark, clip, frame)
        }
        LandmarkPose::Clip { chain, phase } => {
            let (name, _) = table.first_clip(chain.iter().copied())?;
            table.point_at_phase(landmark, name, phase)
        }
    }?;
    Some(pixels * world_per_pixel)
}

/// The world size of one sheet pixel of a body's art.
///
/// A posed body states it. Any other body states the quad its frame is drawn
/// in, and the frame's pixel height gives the scale. A body that states
/// neither is drawn at a size the renderer derives, which the simulation does
/// not hold: it has no scale and so no package answer.
pub fn drawn_world_per_pixel(
    posed: Option<&ambition_sprite_sheet::character::SpritePosedBody>,
    render_size: Option<&crate::components::ActorRenderSize>,
    frame_height: f32,
) -> Option<f32> {
    posed
        .map(|posed| posed.world_per_pixel)
        .or_else(|| render_size.map(|size| size.0.y / frame_height.max(1.0)))
        .filter(|scale| scale.is_finite() && *scale > 0.0)
}

/// The landmark query. See the module docs.
///
/// It reads no `BodyKinematics`, so a system that moves bodies can hold it. A
/// caller that wants a world place gives the kinematics it already holds
/// ([`BodyLandmarks::in_world`]).
#[derive(SystemParam)]
pub struct BodyLandmarks<'w, 's> {
    characters: Option<Res<'w, PreparedCharacterRegistry>>,
    bodies: Query<
        'w,
        's,
        (
            Option<&'static WornCharacter>,
            Option<&'static ambition_sprite_sheet::character::SpritePosedBody>,
            Option<&'static crate::components::ActorRenderSize>,
            (Option<&'static BodyRig>, Option<&'static BodyRigPose>),
            Option<&'static crate::moveset::MovePlayback>,
            Option<&'static BodyPoseClock>,
        ),
    >,
}

impl BodyLandmarks<'_, '_> {
    /// `landmark` of `body` in its rig space: an offset from its feet, +x the
    /// way it faces, +y down, in world units. `None` when the body has no rig
    /// attachment and no published point of that name in that pose.
    pub fn in_rig_space(&self, body: Entity, landmark: Landmark, pose: LandmarkPose<'_>) -> Option<Vec2> {
        let (worn, posed, render_size, (rig, resolved), playback, clock) = self.bodies.get(body).ok()?;
        if let Some(point) = rig.and_then(|rig| rig_landmark(landmark, pose, rig.0.as_ref(), resolved)) {
            return Some(point);
        }
        let sheet = self.characters.as_ref()?.get(worn?.id())?.sheet.as_deref()?;
        let table = ambition_sprite_sheet::baked_landmarks::body_landmarks(sheet)?;
        let world_per_pixel = drawn_world_per_pixel(posed, render_size, table.frame_height)?;
        let clocks = PoseClocks {
            active_move: playback.map(|playback| (&playback.spec.clip, playback.phase())),
            pose: clock.map(|clock| (clock.pose.as_str(), clock.elapsed_s)),
            gait: clock.map(|clock| (clock.gait, clock.gait_elapsed_s)),
        };
        package_landmark(landmark, pose, &table, world_per_pixel, clocks)
    }

    /// The hand `body` makes a gesture with (a pet, a shot), in its rig
    /// space: the hand its character states
    /// ([`ambition_characters::actor::GestureHand`], the near hand when it
    /// states none), else the other hand.
    pub fn gesture_hand(&self, body: Entity, pose: LandmarkPose<'_>) -> Option<Vec2> {
        let stated = self
            .bodies
            .get(body)
            .ok()
            .and_then(|(worn, ..)| Some(self.characters.as_ref()?.get(worn?.id())?.gesture_hand))
            .unwrap_or_default();
        stated
            .hands()
            .into_iter()
            .find_map(|hand| self.in_rig_space(body, hand, pose))
    }

    /// `landmark` of `body` in the world, for a body at `kin` whose unit DOWN
    /// is `down`. `kin` is the body's own kinematics, which the caller holds.
    pub fn in_world(
        &self,
        body: Entity,
        landmark: Landmark,
        pose: LandmarkPose<'_>,
        kin: &ae::BodyKinematics,
        down: Vec2,
    ) -> Option<Vec2> {
        let point = self.in_rig_space(body, landmark, pose)?;
        Some(feet_of(kin, down) + BodyRigPose::to_body(point, kin.facing, down))
    }
}

/// A body's feet: the middle of the face of its box that is toward `down`.
pub fn feet_of(kin: &ae::BodyKinematics, down: Vec2) -> Vec2 {
    kin.pos + down * (kin.size.y * 0.5)
}

#[cfg(test)]
mod tests;
