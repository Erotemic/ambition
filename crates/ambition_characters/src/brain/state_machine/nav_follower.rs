//! Follow a route to a place: the part of navigation each brain that
//! navigates has in common.
//!
//! A brain owns WHERE to go and why ([`NavFollower::go_to`]). The follower
//! owns how the body gets there: it holds the goal, asks the navigation
//! advisor for the next leg (through the snapshot's
//! [`ae::navigation::NavAdvice`]), follows that leg with the one rule the
//! graph was checked with (`ae::navigation::follow_leg`), and emits ordinary
//! locomotion and jump intent. `Roam` and a `MeleeBrute` that navigates both
//! drive one: there is one way a brain follows a route.
//!
//! All of it decides what the body does next, so all of it is state a rewind
//! restores (it is in `Brain`, which is stored by clone).

use ambition_platformer2d_core as ae;
use ae::navigation::{follow_leg, LegFacts, LegPhase, LegProgress, NavLeg, NavLegKind, NavNext};

use super::BrainSnapshot;
use crate::actor::control::ActorControlFrame;

/// A leg is given up when it takes this much longer than its walk (s).
const LEG_TIME_MARGIN_S: f32 = 6.0;
/// A goal is given up after this many legs failed in a row.
const MISSES_PER_GOAL: u8 = 3;
/// A point farther than this is "far" to the pace a brain gives (px).
pub const FAR_BEYOND: f32 = 160.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NavFollower {
    /// The place the body is going to: a feet point.
    pub goal: Option<ae::Vec2>,
    /// The leg the body is on, and where in it.
    pub leg: Option<NavLeg>,
    pub phase: LegPhase,
    /// The sim time (s) when the leg is given up.
    pub until: f32,
    /// The legs that failed in a row on the way to this goal.
    pub misses: u8,
}

/// What one tick of a follower came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Followed {
    /// No goal. The frame is not written.
    Idle,
    /// On the way. The frame is this tick's intent (neutral while the body
    /// waits for the advisor's answer).
    Going,
    /// The body stands at the goal. The goal is dropped.
    Arrived,
    /// No route, or the legs failed. The goal is dropped.
    GaveUp,
}

impl NavFollower {
    /// Go to `goal`. A leg in progress is dropped: call this for a new place,
    /// not each tick for the same one.
    pub fn go_to(&mut self, goal: ae::Vec2) {
        *self = Self { goal: Some(goal), ..Self::default() };
    }

    pub fn stop(&mut self) {
        *self = Self::default();
    }

    /// The body is past the start of a leg: in its run-up or in the air. A
    /// brain does not take the body from the follower now.
    pub fn committed(&self) -> bool {
        self.leg.is_some() && self.phase != LegPhase::Approach
    }

    /// One tick. `pace(far)` is the speed (px/s) to a point the body walks
    /// to, for a point that is far or near; a run-up and a jump are at the
    /// body's top speed.
    pub fn drive(
        &mut self,
        snapshot: &BrainSnapshot,
        pace: impl Fn(bool) -> f32,
        out: &mut ActorControlFrame,
    ) -> Followed {
        let now = snapshot.sim_time;
        let advice = &snapshot.navigation;
        let frame = snapshot.acceleration_frame();
        let side = frame.to_world(ae::Vec2::X);
        if let Some(leg) = self.leg {
            if now > self.until {
                self.leg = None;
                self.misses = self.misses.saturating_add(1);
                return Followed::Going;
            }
            let facts = LegFacts {
                feet: advice.feet,
                vel: snapshot.actor_vel,
                on_ground: snapshot.actor_on_ground,
                side,
                down: frame.to_world(ae::Vec2::Y),
                dt: snapshot.dt,
            };
            let (input, progress) = follow_leg(&leg, self.phase, &facts);
            out.locomotion = if input.full_speed {
                // Up is toward -y in the body's local axes.
                ae::LocalAxes::new(input.axis, if input.up { -1.0 } else { 0.0 })
            } else {
                let far = (approach_point(&leg) - facts.feet).dot(side).abs() > FAR_BEYOND;
                snapshot.locomotion_for(ae::LocalAxes::new(input.axis * pace(far), 0.0))
            };
            if input.axis.abs() > 0.05 {
                out.facing = input.axis.signum();
            }
            out.jump_pressed = input.jump_pressed;
            out.jump_held = input.jump_held;
            out.fly_toggle_pressed = input.fly_toggle;
            match progress {
                LegProgress::Going(phase) => self.phase = phase,
                LegProgress::Arrived => {
                    self.leg = None;
                    self.misses = 0;
                }
                LegProgress::Failed => {
                    self.leg = None;
                    self.misses = self.misses.saturating_add(1);
                }
            }
            return Followed::Going;
        }
        let Some(goal) = self.goal else {
            return Followed::Idle;
        };
        if self.misses >= MISSES_PER_GOAL {
            self.stop();
            return Followed::GaveUp;
        }
        // The advisor answers the goal it read at the start of the tick. An
        // answer to another goal is one tick old.
        if advice.goal != Some(goal) {
            return Followed::Going;
        }
        match advice.next {
            NavNext::Arrived => {
                self.stop();
                Followed::Arrived
            }
            NavNext::Leg(leg) => {
                let walk = (approach_point(&leg) - advice.feet).dot(side).abs() / pace(false).max(1.0);
                self.leg = Some(leg);
                self.phase = LegPhase::Approach;
                self.until = now + walk + LEG_TIME_MARGIN_S;
                Followed::Going
            }
            NavNext::Unreachable => {
                self.stop();
                Followed::GaveUp
            }
            // In the air, or on no surface the graph knows: wait to land.
            NavNext::Unknown => Followed::Going,
        }
    }
}

/// The point a body walks to before a leg commits.
fn approach_point(leg: &NavLeg) -> ae::Vec2 {
    if leg.kind == NavLegKind::Walk {
        leg.land
    } else {
        leg.start
    }
}
