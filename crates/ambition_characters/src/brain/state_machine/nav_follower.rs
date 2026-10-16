//! Follow a route to a place: the part of navigation each brain that
//! navigates has in common.
//!
//! A brain owns WHERE to go and why ([`NavFollower::go_to`]). The follower
//! (`ae::navigation::RouteFollower`, shared with the autopilot that drives a
//! player's seat) owns how the body gets there: it holds the goal, takes the
//! next leg from the navigation advisor (the snapshot's
//! [`ae::navigation::NavAdvice`]), follows that leg with the one rule the
//! graph was checked with (`ae::navigation::follow_leg`). This module makes
//! a leg's input ordinary locomotion and jump intent. `Roam` and a
//! `MeleeBrute` that navigates both drive one: there is one way a body
//! follows a route.
//!
//! All of it decides what the body does next, so all of it is state a rewind
//! restores (it is in `Brain`, which is stored by clone).

use ambition_platformer2d_core as ae;
use ae::navigation::{approach_point, FollowFacts, LegFacts};

use super::BrainSnapshot;
use crate::actor::control::ActorControlFrame;

pub use ae::navigation::{Followed, RouteFollower as NavFollower};

/// A point farther than this is "far" to the pace a brain gives (px).
pub const FAR_BEYOND: f32 = 160.0;

/// A brain's drive of its [`NavFollower`]: the route answer is the advisor's
/// (`BrainSnapshot::navigation`), and a leg's input becomes the brain's
/// locomotion and jump intent.
pub trait DriveFollower {
    /// One tick. `pace(far)` is the speed (px/s) to a point the body walks
    /// to, for a point that is far or near; a run-up and a jump are at the
    /// body's top speed. The frame is written only while a leg is followed.
    fn drive(&mut self, snapshot: &BrainSnapshot, pace: impl Fn(bool) -> f32, out: &mut ActorControlFrame) -> Followed;
}

impl DriveFollower for NavFollower {
    fn drive(&mut self, snapshot: &BrainSnapshot, pace: impl Fn(bool) -> f32, out: &mut ActorControlFrame) -> Followed {
        let advice = &snapshot.navigation;
        let frame = snapshot.acceleration_frame();
        let side = frame.to_world(ae::Vec2::X);
        let facts = FollowFacts {
            now: snapshot.sim_time,
            leg: LegFacts {
                feet: advice.feet,
                vel: snapshot.actor_vel,
                on_ground: snapshot.actor_on_ground,
                side,
                down: frame.to_world(ae::Vec2::Y),
                dt: snapshot.dt,
                wall: snapshot.side_contact_normal,
            },
            walk_speed: pace(false),
            // The advisor answers the goal it read at the start of the tick.
            answer: advice.goal.map(|goal| (goal, advice.next)),
        };
        let step = self.step(&facts);
        if let Some((input, leg)) = step.input {
            out.locomotion = if input.full_speed {
                // Up is toward -y in the body's local axes.
                ae::LocalAxes::new(input.axis, ae::navigation::leg_stick_y(input))
            } else {
                let far = (approach_point(&leg) - facts.leg.feet).dot(side).abs() > FAR_BEYOND;
                snapshot.locomotion_for(ae::LocalAxes::new(input.axis * pace(far), 0.0))
            };
            if input.axis.abs() > 0.05 {
                out.facing = input.axis.signum();
            }
            out.jump_pressed = input.jump_pressed;
            out.jump_held = input.jump_held;
            out.fly_toggle_pressed = input.fly_toggle;
            out.burst_pressed = input.dash_pressed;
        }
        step.followed
    }
}
