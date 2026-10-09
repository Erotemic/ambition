//! The navigation contract: one leg of a route, and the rule that follows it.
//!
//! A route is a list of legs between standing surfaces. This module has the
//! data of one leg ([`NavLeg`]) and the one rule that turns a leg and the
//! body's state into input ([`follow_leg`]). Two readers use the rule:
//!
//! - the graph builder (`ambition_platformer2d_world::navigation`) rolls a leg
//!   out in the movement kernel with this rule, and keeps the leg only when
//!   the body arrives;
//! - a brain follows a leg with this rule.
//!
//! Thus a leg in the graph is a leg the body can do, with the same input. A
//! second copy of the rule in a brain would make the graph a guess.
//!
//! Positions are FEET points in world space: the centre of the body's lower
//! edge. The frame (`side`, `down`) is the body's motion frame.
//!
//! See `docs/planning/engine/platformer-navigation-and-reachability.md`.

use crate::Vec2;

/// A body is at a point when its feet are this near it, along the surface.
pub const ARRIVE_TOLERANCE: f32 = 4.0;
/// A body is at rest when it is slower than this, along the surface (px/s).
pub const REST_SPEED: f32 = 12.0;
/// A body is on a surface when its feet are this near the top of it.
pub const LAND_TOLERANCE: f32 = 6.0;
/// The most waypoints one [`NavAdvice`] holds.
pub const NAV_WAYPOINTS: usize = 8;

/// How a leg moves the body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavLegKind {
    /// Walk along the current surface to `land`.
    #[default]
    Walk,
    /// Jump at `takeoff` and steer to `land` in the air.
    Hop,
    /// Walk off the surface past `takeoff` and steer to `land` in the air.
    Drop,
}

/// One leg of a route.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NavLeg {
    pub kind: NavLegKind,
    /// Where the body stands, at rest, before it commits. From here to
    /// `takeoff` is the run-up. Equal to `takeoff` for a standing hop.
    pub start: Vec2,
    /// Where the body leaves the surface.
    pub takeoff: Vec2,
    /// Where the body stands when the leg is done.
    pub land: Vec2,
}

/// Where a body is in a leg.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LegPhase {
    /// Go to `start` and stop there.
    #[default]
    Approach,
    /// Run from `start` to `takeoff`.
    Commit,
    /// The jump is pressed; the body is not off the ground yet.
    Launch,
    /// In the air, steering to `land`.
    Air,
}

/// What [`follow_leg`] reads of the body.
#[derive(Clone, Copy, Debug)]
pub struct LegFacts {
    pub feet: Vec2,
    pub vel: Vec2,
    pub on_ground: bool,
    /// The body's motion frame: its side axis and the direction it falls.
    pub side: Vec2,
    pub down: Vec2,
    pub dt: f32,
}

/// The input of one step of a leg.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LegInput {
    /// Along the side axis, -1 to 1.
    pub axis: f32,
    /// `axis` is a fraction of the body's top speed. If not, the caller
    /// scales it to its own walk speed.
    pub full_speed: bool,
    pub jump_pressed: bool,
    pub jump_held: bool,
}

/// What a step of a leg came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegProgress {
    Going(LegPhase),
    /// The body stands on the surface of `land`.
    Arrived,
    /// The body is not where the leg needs it. Plan again.
    Failed,
}

/// One step of `leg`: the input for this step, and the phase after it.
pub fn follow_leg(leg: &NavLeg, phase: LegPhase, facts: &LegFacts) -> (LegInput, LegProgress) {
    let along = |point: Vec2| (point - facts.feet).dot(facts.side);
    let below = |point: Vec2| (point - facts.feet).dot(facts.down);
    let speed = facts.vel.dot(facts.side);
    let steer_to = |point: Vec2, band: f32| (along(point) / band).clamp(-1.0, 1.0);
    match phase {
        LegPhase::Approach => {
            let target = if leg.kind == NavLegKind::Walk { leg.land } else { leg.start };
            if facts.on_ground && below(target).abs() > LAND_TOLERANCE {
                return (LegInput::default(), LegProgress::Failed);
            }
            let offset = along(target);
            if offset.abs() <= ARRIVE_TOLERANCE {
                let stopped = facts.on_ground && speed.abs() < REST_SPEED;
                let progress = match (stopped, leg.kind) {
                    (false, _) => LegProgress::Going(LegPhase::Approach),
                    (true, NavLegKind::Walk) => LegProgress::Arrived,
                    (true, _) => LegProgress::Going(LegPhase::Commit),
                };
                return (LegInput::default(), progress);
            }
            // Slower near the point, so the body stops in the tolerance.
            let axis = offset.signum() * (offset.abs() / 32.0).clamp(0.3, 1.0);
            (LegInput { axis, ..Default::default() }, LegProgress::Going(LegPhase::Approach))
        }
        LegPhase::Commit => {
            let run = (leg.takeoff - leg.start).dot(facts.side);
            let reach = (leg.land - leg.takeoff).dot(facts.side);
            let direction = if run.abs() > 0.5 {
                run.signum()
            } else if reach.abs() > 0.5 {
                reach.signum()
            } else {
                0.0
            };
            let running = LegInput { axis: direction, full_speed: true, ..Default::default() };
            match leg.kind {
                NavLegKind::Walk => (LegInput::default(), LegProgress::Going(LegPhase::Approach)),
                NavLegKind::Hop => {
                    if !facts.on_ground {
                        return (LegInput::default(), LegProgress::Failed);
                    }
                    // Press on the last step before the take-off point.
                    let to_takeoff = along(leg.takeoff) * direction;
                    if direction == 0.0 || run.abs() <= 0.5 || to_takeoff - speed.abs() * facts.dt <= 0.0 {
                        let input = LegInput {
                            axis: steer_to(leg.land, 16.0),
                            full_speed: true,
                            jump_pressed: true,
                            jump_held: true,
                        };
                        return (input, LegProgress::Going(LegPhase::Launch));
                    }
                    (running, LegProgress::Going(LegPhase::Commit))
                }
                NavLegKind::Drop => {
                    if direction == 0.0 {
                        return (LegInput::default(), LegProgress::Failed);
                    }
                    let next = if facts.on_ground { LegPhase::Commit } else { LegPhase::Air };
                    (running, LegProgress::Going(next))
                }
            }
        }
        LegPhase::Launch => {
            let input = LegInput {
                axis: steer_to(leg.land, 16.0),
                full_speed: true,
                jump_pressed: facts.on_ground,
                jump_held: true,
            };
            let next = if facts.on_ground { LegPhase::Launch } else { LegPhase::Air };
            (input, LegProgress::Going(next))
        }
        LegPhase::Air => {
            if facts.on_ground {
                let landed = below(leg.land).abs() <= LAND_TOLERANCE;
                let progress = if landed { LegProgress::Arrived } else { LegProgress::Failed };
                return (LegInput::default(), progress);
            }
            let input = LegInput {
                axis: steer_to(leg.land, 16.0),
                full_speed: true,
                jump_pressed: false,
                jump_held: leg.kind == NavLegKind::Hop,
            };
            (input, LegProgress::Going(LegPhase::Air))
        }
    }
}

/// What the route to a goal starts with.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum NavNext {
    /// No answer: the body is on no standing surface (it is in the air), or
    /// there is no goal.
    #[default]
    Unknown,
    /// The body stands at the goal.
    Arrived,
    Leg(NavLeg),
    /// No route goes from the body's surface to the goal.
    Unreachable,
}

/// What a body can know of the routes from where it stands, for one tick.
///
/// Perception, not state: the navigation advisor writes it again each tick,
/// before the brains decide, from the room's surface graph, the body, and the
/// goal the body's brain holds. A brain reads it in its `BrainSnapshot`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NavAdvice {
    /// The body's feet point this tick: what the legs are measured from.
    pub feet: Vec2,
    /// Points the body can reach from its surface, one on each of some
    /// surfaces. Feet points.
    pub waypoints: [Vec2; NAV_WAYPOINTS],
    pub waypoint_count: u8,
    /// The goal that `next` answers. A brain ignores `next` when this is not
    /// the goal it holds now.
    pub goal: Option<Vec2>,
    pub next: NavNext,
    /// A place beside the body the navigating body attends to, when that
    /// body stands over a surface in reach. A feet point. The attended body
    /// is the navigating body's combat target; a body with no combat target
    /// (a friendly one) attends to the nearest player in its room.
    pub target_place: Option<Vec2>,
    /// The attended body stands over the surface the body stands on: a walk
    /// gets there, and no leg is needed.
    pub target_shares_surface: bool,
}

impl NavAdvice {
    pub fn waypoints(&self) -> &[Vec2] {
        &self.waypoints[..(self.waypoint_count as usize).min(NAV_WAYPOINTS)]
    }
}

/// SplitMix64: one well-mixed number from one number. The one source of
/// choice in navigation, so a choice is the same on each peer.
pub fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(feet: Vec2, vel: Vec2, on_ground: bool) -> LegFacts {
        LegFacts { feet, vel, on_ground, side: Vec2::X, down: Vec2::Y, dt: 1.0 / 60.0 }
    }

    #[test]
    fn a_walk_leg_arrives_only_at_rest_at_its_point() {
        let leg = NavLeg { kind: NavLegKind::Walk, land: Vec2::new(100.0, 0.0), ..Default::default() };
        let (far, progress) = follow_leg(&leg, LegPhase::Approach, &facts(Vec2::ZERO, Vec2::ZERO, true));
        assert!(far.axis > 0.0 && !far.full_speed);
        assert_eq!(progress, LegProgress::Going(LegPhase::Approach));
        let moving = facts(Vec2::new(99.0, 0.0), Vec2::new(60.0, 0.0), true);
        assert_eq!(follow_leg(&leg, LegPhase::Approach, &moving).1, LegProgress::Going(LegPhase::Approach));
        let stopped = facts(Vec2::new(99.0, 0.0), Vec2::ZERO, true);
        assert_eq!(follow_leg(&leg, LegPhase::Approach, &stopped).1, LegProgress::Arrived);
    }

    #[test]
    fn a_hop_presses_the_jump_at_its_take_off_and_arrives_on_the_landing_top() {
        let leg = NavLeg {
            kind: NavLegKind::Hop,
            start: Vec2::ZERO,
            takeoff: Vec2::new(100.0, 0.0),
            land: Vec2::new(180.0, -40.0),
        };
        let running = facts(Vec2::new(50.0, 0.0), Vec2::new(120.0, 0.0), true);
        let (input, progress) = follow_leg(&leg, LegPhase::Commit, &running);
        assert!(!input.jump_pressed && input.full_speed && input.axis == 1.0);
        assert_eq!(progress, LegProgress::Going(LegPhase::Commit));
        let at_the_edge = facts(Vec2::new(99.0, 0.0), Vec2::new(120.0, 0.0), true);
        let (input, progress) = follow_leg(&leg, LegPhase::Commit, &at_the_edge);
        assert!(input.jump_pressed);
        assert_eq!(progress, LegProgress::Going(LegPhase::Launch));
        let landed = facts(Vec2::new(176.0, -40.0), Vec2::ZERO, true);
        assert_eq!(follow_leg(&leg, LegPhase::Air, &landed).1, LegProgress::Arrived);
        let fell = facts(Vec2::new(140.0, 200.0), Vec2::ZERO, true);
        assert_eq!(follow_leg(&leg, LegPhase::Air, &fell).1, LegProgress::Failed);
    }
}
