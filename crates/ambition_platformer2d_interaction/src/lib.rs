//! Live-room interaction reach and body gestures.

use ambition_platformer2d_core::{AabbExt, CenteredAabb};
use bevy::prelude::*;

/// What one body's Interact press reaches. The one rule for the press
/// ([`interact_ecs_actors_and_switches`], `open_ecs_chests`), for the door
/// that shares the press ([`TalkableBodies`]) and for the prompt
/// (`ambition_sim_view::update_nearest_interactable`), so the prompt names
/// what the press does. Before, the prompt asked a copy with no live-room
/// check, no door rule and no facing gate (review 2026-10-08).
#[derive(Clone, Copy, Debug)]
pub struct InteractReach {
    pos: ambition_platformer2d_core::Vec2,
    facing: f32,
    /// The run axis of the body's own frame: forward for the facing gate.
    side: ambition_platformer2d_core::Vec2,
    /// The body's collision box: what a chest or a switch must touch.
    reach: ambition_platformer2d_core::Aabb,
    /// The box a body talks from ([`ambition_interaction::talk_reach`]).
    talk: ambition_platformer2d_core::Aabb,
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

impl InteractReach {
    /// The reach of a body with these kinematics, last step and frame, in
    /// the live room `room`.
    pub fn of(
        kin: &ambition_platformer2d_core::BodyKinematics,
        step: Option<&ambition_platformer2d_core::SweepSample>,
        frame: Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    ) -> Self {
        let reach = kin.collision_box(step);
        Self {
            pos: kin.pos,
            facing: kin.facing,
            side: frame.map_or(ambition_platformer2d_core::Vec2::X, |frame| {
                frame.basis().side
            }),
            reach,
            // A body talks from a little farther than it touches.
            talk: ambition_interaction::talk_reach(
                reach,
                step.map_or(ambition_platformer2d_core::Vec2::ZERO, |step| step.down),
            ),
            room,
        }
    }

    /// The body's collision box.
    pub fn collision_box(&self) -> ambition_platformer2d_core::Aabb {
        self.reach
    }

    /// Where the body stands.
    pub fn pos(&self) -> ambition_platformer2d_core::Vec2 {
        self.pos
    }

    /// The facing gate (Q63): a target that requires facing is reached only
    /// by a body that faces it.
    fn passes_the_facing_gate(
        &self,
        target: ambition_platformer2d_core::Vec2,
        requires_facing: bool,
    ) -> bool {
        !requires_facing || ambition_interaction::faces(self.pos, self.facing, self.side, target)
    }

    /// Can the press talk to a peaceful body at `aabb` in live room `room`?
    /// The door rule is not part of it: see [`Self::a_door_keeps_the_press`].
    pub fn can_talk_to(
        &self,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        aabb: &CenteredAabb,
        requires_facing: bool,
    ) -> bool {
        // OW1 cut 7b: two live rooms can hold a body at one position.
        room == self.room
            && aabb.aabb().strict_intersects(self.talk)
            && self.passes_the_facing_gate(aabb.center, requires_facing)
    }

    /// Does the press reach a chest or a switch at `aabb` in live room `room`?
    pub fn touches(
        &self,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        aabb: &CenteredAabb,
        requires_facing: bool,
    ) -> bool {
        room == self.room
            && aabb.aabb().strict_intersects(self.reach)
            && self.passes_the_facing_gate(aabb.center, requires_facing)
    }

    /// One press can be for a door or for a body to talk to: the nearer of
    /// the two takes it, and a tie goes to the door. `door` is the distance
    /// to the door the body stands in, `talker` the distance to the body.
    pub fn a_door_keeps_the_press(door: Option<f32>, talker: f32) -> bool {
        door.is_some_and(|door| door <= talker)
    }
}

/// Play the interact gesture on the body that ACTED.
pub fn pose_interact(
    anims: &mut Query<&mut ambition_characters::actor::BodyAnimFacts>,
    body: Entity,
    hold_secs: f32,
) {
    if let Ok(mut anim) = anims.get_mut(body) {
        anim.interact_anim_timer = hold_secs;
    }
}
