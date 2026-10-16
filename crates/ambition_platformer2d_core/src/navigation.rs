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
/// How far above a landing a [`NavLegKind::Fly`] goes before it goes along to
/// it: half of it is where the body turns flight off.
pub const FLIGHT_CLEARANCE: f32 = 32.0;
/// The most waypoints one [`NavAdvice`] holds.
pub const NAV_WAYPOINTS: usize = 8;
/// A stick that moves a body on its floor below its top speed and starts no
/// dash: above the kernel's run threshold (0.1), below its dash threshold
/// (0.5). For the last part of an approach.
pub const CREEP_STICK: f32 = 0.3;

/// How a body comes to rest on a point, measured in the kernel (the
/// traversal envelope). A follower that has it runs to the point at its top
/// speed and lets go of the stick when its slide would take it there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApproachLaw {
    /// The body's braking on its floor with no stick (px/s²).
    pub braking: f32,
    /// Nearer than this to the point, a press from rest would run past it (a
    /// press from rest is a dash, at speed on its first step): creep instead.
    pub creep_band: f32,
}

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
    /// Stand on a one-way surface at `takeoff`, hold down and press jump to
    /// fall through it, and steer to `land` below.
    DropThrough,
    /// Jump at `takeoff`, jump again in the air at the top of the first
    /// arc, and steer to `land`: for a body with an air jump.
    DoubleHop,
    /// Jump at `takeoff` (and again at the top of the arc, when the body
    /// has an air jump) to the wall face under `land`, hold into it and up:
    /// the body clings, climbs, takes the ledge and pulls itself up onto it.
    /// For a body that can cling and climb.
    WallClimb,
    /// Jump at `takeoff`, dash toward `land` at the top of the arc, and
    /// steer to it: for a body with a dash, over a gap no jump clears.
    DashHop,
    /// Jump at `takeoff` to the wall face beside `land`, and kick off each
    /// wall the body touches below `land` until it is above it, then steer
    /// onto it: up a shaft of two walls. For a body that can wall jump.
    WallJump,
    /// Toggle flight at `start`, fly up and along to the air above `land`,
    /// toggle flight off there and fall onto it. For a body that can toggle
    /// flight.
    Fly,
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
    /// The surface `land` is on, along the body's side axis: the least and
    /// the most place the feet point of a body that stands on it can be. A
    /// body that comes down at the height of `land` and not in this span
    /// stands on another surface of the same height, and the leg has failed.
    pub land_span: [f32; 2],
}

impl NavLeg {
    /// Is a feet point at `along` (its place on the side axis) on the
    /// surface this leg lands on?
    pub fn lands_at(&self, along: f32) -> bool {
        along >= self.land_span[0] && along <= self.land_span[1]
    }
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
    /// In the air, steering to `land`. A [`NavLegKind::Fly`] is in flight.
    Air,
    /// In the air after the air jump of a [`NavLegKind::DoubleHop`], or
    /// falling after the flight of a [`NavLegKind::Fly`].
    SecondAir,
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
    /// The wall the body touches, as the side its face points to along
    /// `side` (`1.0`: away from it is `+side`), or `None`.
    pub wall: Option<f32>,
    /// How the body stops on a point, when its follower knows: then an
    /// approach is at the body's top speed, and stops by braking. `None`: the
    /// approach is a fraction of the follower's own walk (a brain's pace).
    pub approach: Option<ApproachLaw>,
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
    /// Hold up: climb a wall, and pull up from a ledge.
    pub up: bool,
    /// Hold down: with `jump_pressed`, fall through a one-way surface.
    pub down: bool,
    /// Press the flight toggle.
    pub fly_toggle: bool,
    /// Press the dash (the burst press), aimed along `axis`.
    pub dash_pressed: bool,
}

/// The stick's axis toward the feet that `input` holds, in the body's local
/// axes: up is -1, down is 1. Each driver of a leg maps it the same way.
pub fn leg_stick_y(input: LegInput) -> f32 {
    match (input.up, input.down) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => 0.0,
    }
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
            let going = LegProgress::Going(LegPhase::Approach);
            if let Some(law) = facts.approach {
                // Run, and let go when the slide reaches the point.
                let toward = (speed * offset.signum()).max(0.0);
                let slide = toward * toward / (2.0 * law.braking.max(1.0));
                let axis = if slide >= offset.abs() - ARRIVE_TOLERANCE * 0.5 {
                    0.0
                } else if offset.abs() <= law.creep_band {
                    offset.signum() * CREEP_STICK
                } else {
                    offset.signum()
                };
                return (LegInput { axis, full_speed: true, ..Default::default() }, going);
            }
            // Slower near the point, so the body stops in the tolerance.
            let axis = offset.signum() * (offset.abs() / 32.0).clamp(0.3, 1.0);
            (LegInput { axis, ..Default::default() }, going)
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
                NavLegKind::Hop
                | NavLegKind::DoubleHop
                | NavLegKind::DashHop
                | NavLegKind::WallClimb
                | NavLegKind::WallJump => {
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
                            ..LegInput::default()
                        };
                        return (input, LegProgress::Going(LegPhase::Launch));
                    }
                    (running, LegProgress::Going(LegPhase::Commit))
                }
                // Flight starts where the body stands: no run-up.
                NavLegKind::Fly => {
                    if !facts.on_ground {
                        return (LegInput::default(), LegProgress::Failed);
                    }
                    let input = LegInput { fly_toggle: true, ..LegInput::default() };
                    (input, LegProgress::Going(LegPhase::Air))
                }
                NavLegKind::Drop => {
                    if direction == 0.0 {
                        return (LegInput::default(), LegProgress::Failed);
                    }
                    let next = if facts.on_ground { LegPhase::Commit } else { LegPhase::Air };
                    (running, LegProgress::Going(next))
                }
                // Where the body stands: down and jump until the surface lets
                // it go.
                NavLegKind::DropThrough => {
                    let input = LegInput { down: true, jump_pressed: true, ..LegInput::default() };
                    let next = if facts.on_ground { LegPhase::Commit } else { LegPhase::Air };
                    (input, LegProgress::Going(next))
                }
            }
        }
        LegPhase::Launch => {
            let input = LegInput {
                axis: steer_to(leg.land, 16.0),
                full_speed: true,
                jump_pressed: facts.on_ground,
                jump_held: true,
                ..LegInput::default()
            };
            let next = if facts.on_ground { LegPhase::Launch } else { LegPhase::Air };
            (input, LegProgress::Going(next))
        }
        // In flight: rise to the air above the landing and go along to it,
        // then toggle flight off and fall onto it. The body is on the ground
        // on the first steps, and that is not a landing.
        LegPhase::Air if leg.kind == NavLegKind::Fly => {
            // How far the feet are above the landing's height.
            let above = below(leg.land);
            let over = along(leg.land).abs() <= ARRIVE_TOLERANCE * 2.0;
            if above >= FLIGHT_CLEARANCE * 0.5 && over {
                let input = LegInput { fly_toggle: true, ..LegInput::default() };
                return (input, LegProgress::Going(LegPhase::SecondAir));
            }
            let input = LegInput {
                // Go along only when clear of the landing's height, so the
                // body does not fly into the side of the surface.
                axis: if above >= FLIGHT_CLEARANCE * 0.5 { steer_to(leg.land, 16.0) } else { 0.0 },
                full_speed: true,
                up: above < FLIGHT_CLEARANCE,
                ..LegInput::default()
            };
            (input, LegProgress::Going(LegPhase::Air))
        }
        LegPhase::Air | LegPhase::SecondAir => {
            if facts.on_ground {
                // At the height of the landing AND on its surface. The height
                // alone said "arrived" on any surface of that height (found
                // in review, 2026-10-09), and the follower then dropped the
                // leg as done and forgot its misses.
                let landed =
                    below(leg.land).abs() <= LAND_TOLERANCE && leg.lands_at(facts.feet.dot(facts.side));
                let progress = if landed { LegProgress::Arrived } else { LegProgress::Failed };
                return (LegInput::default(), progress);
            }
            let falling = facts.vel.dot(facts.down) > 0.0;
            let climb = leg.kind == NavLegKind::WallClimb;
            // The air jump of a double hop: at the top of the first arc, where
            // it lifts the body highest. Before it, the body is below a
            // landing that one jump does not reach, and that is not a miss.
            // A wall climb takes it too, to come to the face higher.
            if matches!(leg.kind, NavLegKind::DoubleHop | NavLegKind::WallClimb) && phase == LegPhase::Air {
                let input = LegInput {
                    axis: steer_to(leg.land, 16.0),
                    full_speed: true,
                    jump_pressed: falling,
                    jump_held: !climb,
                    up: climb,
                    down: false,
                    fly_toggle: false,
                    dash_pressed: false,
                };
                let next = if falling { LegPhase::SecondAir } else { LegPhase::Air };
                return (input, LegProgress::Going(next));
            }
            // The dash of a dash hop: at the top of the arc, straight toward
            // the landing, so it carries the body along at the height of the
            // apex. Before it, the body is short of a landing that one jump
            // does not reach, and that is not a miss.
            if leg.kind == NavLegKind::DashHop && phase == LegPhase::Air {
                let toward = along(leg.land).signum();
                let input = LegInput {
                    axis: if falling { toward } else { steer_to(leg.land, 16.0) },
                    full_speed: true,
                    jump_held: true,
                    dash_pressed: falling,
                    ..LegInput::default()
                };
                let next = if falling { LegPhase::SecondAir } else { LegPhase::Air };
                return (input, LegProgress::Going(next));
            }
            // A wall jump: kick off each wall touched below the landing, keep
            // going the way the kick sends the body to the next wall, and
            // steer onto the landing once above it. The body is below its
            // landing the whole way up: no early miss.
            if leg.kind == NavLegKind::WallJump {
                // The feet are over the landing's height: it is below them.
                let above = below(leg.land) > LAND_TOLERANCE;
                let input = match facts.wall {
                    Some(away) if !above => LegInput {
                        axis: away,
                        full_speed: true,
                        jump_pressed: true,
                        jump_held: true,
                        ..LegInput::default()
                    },
                    _ if above => LegInput {
                        axis: steer_to(leg.land, 16.0),
                        full_speed: true,
                        jump_held: true,
                        ..LegInput::default()
                    },
                    _ => LegInput {
                        axis: speed.signum(),
                        full_speed: true,
                        jump_held: true,
                        ..LegInput::default()
                    },
                };
                return (input, LegProgress::Going(phase));
            }
            // A climbing body is below its landing the whole way up, and it
            // slides down a little before it climbs: no early miss. A climb
            // that does not get there ends on the ground somewhere else.
            if climb {
                let input = LegInput {
                    axis: steer_to(leg.land, 16.0),
                    full_speed: true,
                    up: true,
                    ..LegInput::default()
                };
                return (input, LegProgress::Going(phase));
            }
            // Below the landing top and still falling: the body has missed,
            // and it cannot come back up. Fail now, not where it lands. A hop
            // to a higher surface is below the top on its way up too, so the
            // test also needs downward motion.
            if below(leg.land) < -LAND_TOLERANCE && falling {
                return (LegInput::default(), LegProgress::Failed);
            }
            let input = LegInput {
                axis: steer_to(leg.land, 16.0),
                full_speed: true,
                jump_pressed: false,
                jump_held: matches!(leg.kind, NavLegKind::Hop | NavLegKind::DoubleHop | NavLegKind::DashHop),
                ..LegInput::default()
            };
            (input, LegProgress::Going(phase))
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
    /// What the advisor sees of the item the body's errand asks for.
    pub errand: ErrandSight,
}

/// What a body's errand item is to the body this tick: perception, written by
/// the navigation advisor. The errand's owner settles the errand from it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ErrandSight {
    /// The body has no errand, or the advisor did not look.
    #[default]
    None,
    /// The item lies over a surface the body can get to: the feet point under
    /// the item on that surface.
    At(Vec2),
    /// The item lies in another live room, and the route there starts at a
    /// zone of this room that the body can get to: the feet point under the
    /// zone. The body goes there, and its errand takes it through.
    Door(Vec2),
    /// The item lies in the body's room, or behind a zone of it, and no route
    /// gets there.
    NoRoute,
    /// The item lies in no live room that a route of live rooms gets to.
    Gone,
}

impl ErrandSight {
    /// Where the body goes for its errand this tick: to the item, or to the
    /// zone that leads to it.
    pub fn place(self) -> Option<Vec2> {
        match self {
            Self::At(place) | Self::Door(place) => Some(place),
            Self::None | Self::NoRoute | Self::Gone => None,
        }
    }
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

/// A leg is given up when it takes this much longer than its walk (s).
pub const LEG_TIME_MARGIN_S: f32 = 6.0;
/// A goal is given up after this many legs failed in a row.
pub const MISSES_PER_GOAL: u8 = 3;

/// Follow a route to a place: the part of navigation that each body that
/// navigates has in common, a brain's body or a body an autopilot drives.
///
/// Its owner says where to go ([`Self::go_to`]). The follower holds the goal,
/// takes the next leg from a route answer, follows the leg with
/// [`follow_leg`] (the rule the graph was checked with), and says what input
/// the body gets. It is plain state: a brain keeps it in its rewound state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RouteFollower {
    /// The place the body is going to: a feet point.
    pub goal: Option<Vec2>,
    /// The leg the body is on, and where in it.
    pub leg: Option<NavLeg>,
    pub phase: LegPhase,
    /// The time (s) when the leg is given up, on the owner's clock.
    pub until: f32,
    /// The legs that failed in a row on the way to this goal.
    pub misses: u8,
}

/// What one step of a [`RouteFollower`] came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Followed {
    /// No goal. No input.
    Idle,
    /// On the way. The input is this step's (none while the body waits for a
    /// route answer).
    Going,
    /// The body stands at the goal. The goal is dropped.
    Arrived,
    /// No route, or the legs failed. The goal is dropped.
    GaveUp,
}

/// What a [`RouteFollower`] reads in one step.
#[derive(Clone, Copy, Debug)]
pub struct FollowFacts {
    /// The owner's clock (s), for the time a leg may take.
    pub now: f32,
    /// The body's state, as [`follow_leg`] reads it.
    pub leg: LegFacts,
    /// The body's walk speed (px/s), for the time a leg may take.
    pub walk_speed: f32,
    /// The route answer, and the goal it answers. Read only when the
    /// follower has no leg: see [`RouteFollower::needs_route`].
    pub answer: Option<(Vec2, NavNext)>,
}

/// One step of a [`RouteFollower`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FollowStep {
    pub followed: Followed,
    /// The input of the leg the body is on, and that leg.
    pub input: Option<(LegInput, NavLeg)>,
}

impl RouteFollower {
    /// Go to `goal`. A leg in progress is dropped: call this for a new place,
    /// not each step for the same one.
    pub fn go_to(&mut self, goal: Vec2) {
        *self = Self { goal: Some(goal), ..Self::default() };
    }

    pub fn stop(&mut self) {
        *self = Self::default();
    }

    /// The body is past the start of a leg: in its run-up or in the air. Its
    /// owner does not take the body from the follower now.
    pub fn committed(&self) -> bool {
        self.leg.is_some() && self.phase != LegPhase::Approach
    }

    /// The goal a route answer is wanted for in this step, if the follower
    /// will read one: it has a goal and no leg. An owner that plans on demand
    /// plans only then.
    pub fn needs_route(&self) -> Option<Vec2> {
        if self.leg.is_some() || self.misses >= MISSES_PER_GOAL {
            return None;
        }
        self.goal
    }

    /// One step.
    pub fn step(&mut self, facts: &FollowFacts) -> FollowStep {
        let going = |input| FollowStep { followed: Followed::Going, input };
        if let Some(leg) = self.leg {
            if facts.now > self.until {
                self.leg = None;
                self.misses = self.misses.saturating_add(1);
                return going(None);
            }
            let (input, progress) = follow_leg(&leg, self.phase, &facts.leg);
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
            return going(Some((input, leg)));
        }
        let Some(goal) = self.goal else {
            return FollowStep { followed: Followed::Idle, input: None };
        };
        if self.misses >= MISSES_PER_GOAL {
            self.stop();
            return FollowStep { followed: Followed::GaveUp, input: None };
        }
        // An answer to another goal is stale: wait for one to this goal.
        let next = match facts.answer {
            Some((answered, next)) if answered == goal => next,
            _ => return going(None),
        };
        match next {
            NavNext::Arrived => {
                self.stop();
                FollowStep { followed: Followed::Arrived, input: None }
            }
            NavNext::Leg(leg) => {
                let side = facts.leg.side;
                let walk = (approach_point(&leg) - facts.leg.feet).dot(side).abs() / facts.walk_speed.max(1.0);
                self.leg = Some(leg);
                self.phase = LegPhase::Approach;
                self.until = facts.now + walk + LEG_TIME_MARGIN_S;
                going(None)
            }
            NavNext::Unreachable => {
                self.stop();
                FollowStep { followed: Followed::GaveUp, input: None }
            }
            // In the air, or on no surface the graph knows: wait to land.
            NavNext::Unknown => going(None),
        }
    }
}

/// The point a body walks to before a leg commits.
pub fn approach_point(leg: &NavLeg) -> Vec2 {
    if leg.kind == NavLegKind::Walk {
        leg.land
    } else {
        leg.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(feet: Vec2, vel: Vec2, on_ground: bool) -> LegFacts {
        LegFacts { feet, vel, on_ground, side: Vec2::X, down: Vec2::Y, dt: 1.0 / 60.0, wall: None, approach: None }
    }

    /// With an approach law, the approach runs at full speed, lets go where
    /// the body's slide ends at the point, and creeps where a press from rest
    /// would run past it.
    #[test]
    fn an_approach_by_law_runs_lets_go_and_creeps() {
        let leg = NavLeg { kind: NavLegKind::Walk, land: Vec2::new(400.0, 0.0), ..Default::default() };
        let law = ApproachLaw { braking: 1000.0, creep_band: 50.0 };
        let at = |x: f32, speed: f32| LegFacts { approach: Some(law), ..facts(Vec2::new(x, 0.0), Vec2::new(speed, 0.0), true) };
        let axis = |x: f32, speed: f32| {
            let (input, _) = follow_leg(&leg, LegPhase::Approach, &at(x, speed));
            assert!(input.full_speed, "an approach by law is in parts of the top speed");
            input.axis
        };
        assert_eq!(axis(0.0, 0.0), 1.0, "far and at rest: run");
        // 500 px/s slides 125 px at 1000 px/s²: from 100 px it lets go.
        assert_eq!(axis(300.0, 500.0), 0.0, "the slide reaches the point: let go");
        assert_eq!(axis(100.0, 500.0), 1.0, "the slide falls short: run on");
        assert_eq!(axis(370.0, 0.0), CREEP_STICK, "near and at rest: creep, no dash");
        // Past the point, it turns back.
        assert!(axis(420.0, 0.0) < 0.0);
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
            land_span: [150.0, 300.0],
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

    /// Below the landing top and falling, a leg has missed: it cannot come back
    /// up. Below the top and rising is a hop to a higher surface on its way.
    #[test]
    fn a_body_below_its_landing_and_falling_has_missed() {
        let leg = NavLeg {
            kind: NavLegKind::Hop,
            start: Vec2::new(0.0, 100.0),
            takeoff: Vec2::new(0.0, 100.0),
            land: Vec2::new(80.0, 60.0),
            land_span: [50.0, 200.0],
        };
        let below_the_top = Vec2::new(60.0, 60.0 + LAND_TOLERANCE + 1.0);
        let falling = follow_leg(&leg, LegPhase::Air, &facts(below_the_top, Vec2::new(100.0, 50.0), false));
        let rising = follow_leg(&leg, LegPhase::Air, &facts(below_the_top, Vec2::new(100.0, -50.0), false));
        assert_eq!((falling.1, rising.1), (LegProgress::Failed, LegProgress::Going(LegPhase::Air)));
    }

    /// Two platforms of one height. A hop to the far one that comes down on
    /// the near one has not arrived: the height is right and the surface is
    /// not.
    #[test]
    fn a_landing_at_the_right_height_on_another_surface_has_failed() {
        // The near platform is x 0 to 80, the far one 180 to 260, both y 200.
        let leg = NavLeg {
            kind: NavLegKind::Hop,
            start: Vec2::new(-60.0, 260.0),
            takeoff: Vec2::new(-60.0, 260.0),
            land: Vec2::new(200.0, 200.0),
            land_span: [180.0, 260.0],
        };
        let on_the_near_one = facts(Vec2::new(40.0, 200.0), Vec2::ZERO, true);
        let on_the_far_one = facts(Vec2::new(190.0, 200.0), Vec2::ZERO, true);
        // Short of the point and on its surface is an arrival: a walk leg
        // takes the body the rest of the way.
        assert_eq!(follow_leg(&leg, LegPhase::Air, &on_the_far_one).1, LegProgress::Arrived);
        assert_eq!(follow_leg(&leg, LegPhase::Air, &on_the_near_one).1, LegProgress::Failed);
    }
}