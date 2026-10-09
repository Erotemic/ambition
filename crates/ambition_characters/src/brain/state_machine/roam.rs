//! Roam: go to places in the room, one after the other, and rest at each.
//!
//! The brain owns the POLICY: which place, when, and how long to rest. It
//! does not know the room. Each tick the navigation advisor gives it
//! ([`ae::navigation::NavAdvice`], in the snapshot) some places it can reach
//! from where it stands and, for the goal it holds, the next leg of the route.
//! The brain follows a leg with the one rule the graph was checked with
//! (`ae::navigation::follow_leg`), and emits ordinary locomotion and jump
//! intent. Any body with this brain and the advisor navigates: nothing here is
//! about one character.
//!
//! ALL of [`RoamState`] gates what the body does next, so all of it is
//! rewound (`SnapshotCursor for Brain`).

use ambition_platformer2d_core as ae;
use ae::navigation::{follow_leg, mix, LegFacts, LegPhase, LegProgress, NavLeg, NavNext};

use super::BrainSnapshot;
use crate::actor::control::ActorControlFrame;

/// A leg is given up when it takes this much longer than its walk (s).
const LEG_TIME_MARGIN_S: f32 = 6.0;
/// A point farther than this is gone to at the trot, not at the walk (px).
const TROT_BEYOND: f32 = 160.0;
/// A goal is given up after this many legs failed in a row.
const MISSES_PER_GOAL: u8 = 3;
/// A place this near is not a new place to go to (px).
const SAME_PLACE: f32 = 48.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoamCfg {
    /// The walk speed between take-offs (px/s). A run-up and a jump are at
    /// the body's top speed.
    pub speed: f32,
    /// The speed to a point that is far (px/s).
    pub trot_speed: f32,
    /// The shortest and the longest rest at a place (s).
    pub rest_min_s: f32,
    pub rest_max_s: f32,
    /// How often the next place is the one nearest the body's target (the
    /// player, for a friendly body) and not a place by chance, 0 to 1.
    pub company: f32,
    /// A resting body faces a target this near (px).
    pub notice_radius: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoamState {
    /// The place the body is going to: a feet point.
    pub goal: Option<ae::Vec2>,
    /// The leg the body is on, and where in it.
    pub leg: Option<NavLeg>,
    pub phase: LegPhase,
    /// The sim time (s) when the rest ends, or when the leg is given up.
    pub until: f32,
    /// How many goals this brain has chosen: the seed of the next choice.
    pub picks: u32,
    /// The legs that failed in a row on the way to this goal.
    pub misses: u8,
}

/// The point a body walks to before a leg commits.
fn approach_point(leg: &NavLeg) -> ae::Vec2 {
    if leg.kind == ae::navigation::NavLegKind::Walk {
        leg.land
    } else {
        leg.start
    }
}

/// How far the body is from that point, along its side axis.
fn approach_distance(leg: &NavLeg, facts: &LegFacts) -> f32 {
    (approach_point(leg) - facts.feet).dot(facts.side).abs()
}

/// A number from 0 to 1 for choice `salt` of pick `picks`.
fn chance(picks: u32, salt: u64) -> f32 {
    (mix(((picks as u64) << 8) ^ salt) % 10_000) as f32 / 9_999.0
}

pub(super) fn tick_roam(cfg: &RoamCfg, state: &mut RoamState, snapshot: &BrainSnapshot, out: &mut ActorControlFrame) {
    *out = ActorControlFrame::neutral();
    out.facing = snapshot.actor_facing;
    let now = snapshot.sim_time;
    let advice = &snapshot.navigation;

    if let Some(leg) = state.leg {
        if now > state.until {
            state.leg = None;
            state.misses = state.misses.saturating_add(1);
            return;
        }
        let frame = snapshot.acceleration_frame();
        let facts = LegFacts {
            feet: advice.feet,
            vel: snapshot.actor_vel,
            on_ground: snapshot.actor_on_ground,
            side: frame.to_world(ae::Vec2::X),
            down: frame.to_world(ae::Vec2::Y),
            dt: snapshot.dt,
        };
        let (input, progress) = follow_leg(&leg, state.phase, &facts);
        out.locomotion = if input.full_speed {
            ae::LocalAxes::new(input.axis, 0.0)
        } else {
            let pace = if approach_distance(&leg, &facts) > TROT_BEYOND { cfg.trot_speed } else { cfg.speed };
            snapshot.locomotion_for(ae::LocalAxes::new(input.axis * pace, 0.0))
        };
        if input.axis.abs() > 0.05 {
            out.facing = input.axis.signum();
        }
        out.jump_pressed = input.jump_pressed;
        out.jump_held = input.jump_held;
        match progress {
            LegProgress::Going(phase) => state.phase = phase,
            LegProgress::Arrived => {
                state.leg = None;
                state.misses = 0;
            }
            LegProgress::Failed => {
                state.leg = None;
                state.misses = state.misses.saturating_add(1);
            }
        }
        return;
    }

    let rest = |picks: u32| cfg.rest_min_s + (cfg.rest_max_s - cfg.rest_min_s).max(0.0) * chance(picks, 0x51);
    let Some(goal) = state.goal else {
        // At rest. Look at a target that is near.
        let to_target = snapshot.target_delta_local();
        if snapshot.target_alive && to_target.vec().length() <= cfg.notice_radius {
            out.facing = snapshot.face_toward(to_target.x);
        }
        // A body starts with a rest, not with a journey.
        if state.picks == 0 && state.until == 0.0 {
            state.until = now + rest(0);
        }
        if now < state.until || !snapshot.actor_on_ground {
            return;
        }
        let places = advice.waypoints();
        let far = |place: &&ae::Vec2| place.distance(advice.feet) > SAME_PLACE;
        let pool: Vec<&ae::Vec2> = if places.iter().any(|place| far(&place)) {
            places.iter().filter(far).collect()
        } else {
            // Nowhere to go from here (or no advisor): rest again.
            state.until = now + rest(state.picks).max(0.5);
            state.picks = state.picks.wrapping_add(1);
            return;
        };
        let with_company = snapshot.target_alive && chance(state.picks, 0xC0) < cfg.company;
        let chosen = if with_company {
            pool.iter().min_by(|a, b| {
                a.distance_squared(snapshot.target_pos).total_cmp(&b.distance_squared(snapshot.target_pos))
            })
        } else {
            pool.get((mix(state.picks as u64 ^ 0xA7) % pool.len() as u64) as usize)
        };
        state.goal = chosen.map(|place| **place);
        state.picks = state.picks.wrapping_add(1);
        state.misses = 0;
        return;
    };

    if state.misses >= MISSES_PER_GOAL {
        state.goal = None;
        state.until = now + rest(state.picks);
        return;
    }
    // The advisor answers the goal it read at the start of the tick. An answer
    // to another goal is one tick old.
    if advice.goal != Some(goal) {
        return;
    }
    match advice.next {
        NavNext::Arrived => {
            state.goal = None;
            state.until = now + rest(state.picks);
        }
        NavNext::Leg(leg) => {
            let frame = snapshot.acceleration_frame();
            let side = frame.to_world(ae::Vec2::X);
            let walk = ((approach_point(&leg) - advice.feet).dot(side)).abs() / cfg.speed.max(1.0);
            state.leg = Some(leg);
            state.phase = LegPhase::Approach;
            state.until = now + walk + LEG_TIME_MARGIN_S;
        }
        NavNext::Unreachable => {
            state.goal = None;
            state.until = now + 0.5;
        }
        // In the air, or on no surface the graph knows: wait to land.
        NavNext::Unknown => {}
    }
}
