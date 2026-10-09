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
/// The pause at a place when the body is at play (s).
const PLAY_PAUSE_S: f32 = 0.25;
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
    /// How often the next place is beside the body's target (the player,
    /// for a friendly body) and not a place by chance, 0 to 1.
    pub company: f32,
    /// A body farther than this from its target goes to it next, when a
    /// route goes there (px). Zero: the body does not keep near its target.
    pub stay_within: f32,
    /// How often the body is in a playful mood, 0 to 1. A mood lasts four
    /// places: the body runs to each and does not rest between them.
    pub playful: f32,
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

/// Is the body in a playful mood for its choice number `picks`? A mood is a
/// function of the count of choices, so it is no more state to rewind.
fn playful(cfg: &RoamCfg, picks: u32) -> bool {
    cfg.playful > 0.0 && chance(picks / 4, 0x9A) < cfg.playful
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
            let far = approach_distance(&leg, &facts) > TROT_BEYOND;
            let pace = match (playful(cfg, state.picks), far) {
                // At play the body runs, and `follow_leg` slows it at the point.
                (true, _) => snapshot.max_run_speed,
                (false, true) => cfg.trot_speed,
                (false, false) => cfg.speed,
            };
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

    let rest = |picks: u32| {
        if playful(cfg, picks) {
            return PLAY_PAUSE_S;
        }
        cfg.rest_min_s + (cfg.rest_max_s - cfg.rest_min_s).max(0.0) * chance(picks, 0x51)
    };
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
        // Beside the target, when the body wants company or is too far from
        // it, and a route goes there. If not, a place by chance.
        let lonely = cfg.stay_within > 0.0 && to_target.vec().length() > cfg.stay_within;
        let beside_target = advice
            .target_place
            .filter(|place| snapshot.target_alive && place.distance(advice.feet) > SAME_PLACE)
            .filter(|_| lonely || chance(state.picks, 0xC0) < cfg.company);
        state.goal = beside_target
            .or_else(|| pool.get((mix(state.picks as u64 ^ 0xA7) % pool.len() as u64) as usize).map(|place| **place));
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

#[cfg(test)]
mod tests {
    use super::*;
    use ae::navigation::{NavAdvice, NavLegKind};
    use ae::Vec2;

    const CFG: RoamCfg = RoamCfg {
        speed: 50.0,
        trot_speed: 100.0,
        rest_min_s: 1.0,
        rest_max_s: 2.0,
        company: 0.0,
        stay_within: 0.0,
        playful: 0.0,
        notice_radius: 0.0,
    };
    const HERE: Vec2 = Vec2::new(100.0, 500.0);
    const THERE: Vec2 = Vec2::new(400.0, 440.0);

    /// A body on the ground at `HERE` that can reach `THERE`.
    fn standing(sim_time: f32) -> BrainSnapshot {
        let mut advice = NavAdvice { feet: HERE, waypoint_count: 2, ..Default::default() };
        advice.waypoints[0] = HERE;
        advice.waypoints[1] = THERE;
        BrainSnapshot {
            actor_pos: HERE - Vec2::Y * 32.0,
            actor_on_ground: true,
            sim_time,
            dt: 1.0 / 60.0,
            max_run_speed: 120.0,
            navigation: advice,
            ..BrainSnapshot::idle()
        }
    }

    fn tick(cfg: &RoamCfg, state: &mut RoamState, snapshot: &BrainSnapshot) -> ActorControlFrame {
        let mut out = ActorControlFrame::neutral();
        tick_roam(cfg, state, snapshot, &mut out);
        out
    }

    #[test]
    fn a_roamer_rests_chooses_a_place_takes_its_leg_and_rests_again() {
        let mut state = RoamState::default();
        // It starts with a rest.
        tick(&CFG, &mut state, &standing(0.1));
        assert!(state.goal.is_none() && state.until >= 1.1 && state.until <= 2.1, "{state:?}");
        tick(&CFG, &mut state, &standing(0.5));
        assert!(state.goal.is_none());
        // The rest ends: the one place that is not here.
        tick(&CFG, &mut state, &standing(3.0));
        assert_eq!((state.goal, state.picks), (Some(THERE), 1));

        // The advisor answers the goal with a hop from the edge.
        let leg = NavLeg {
            kind: NavLegKind::Hop,
            start: HERE,
            takeoff: HERE,
            land: THERE,
        };
        let mut answered = standing(3.1);
        answered.navigation.goal = Some(THERE);
        answered.navigation.next = NavNext::Leg(leg);
        tick(&CFG, &mut state, &answered);
        assert_eq!((state.leg, state.phase), (Some(leg), LegPhase::Approach));
        // At rest at the start: it commits, then it jumps toward the place.
        tick(&CFG, &mut state, &answered);
        assert_eq!(state.phase, LegPhase::Commit);
        let jump = tick(&CFG, &mut state, &answered);
        assert!(jump.jump_pressed && jump.jump_held && jump.locomotion.x > 0.0, "{jump:?}");

        // On the landing surface: the leg is done, and the advisor says so.
        state.phase = LegPhase::Air;
        let mut landed = standing(4.0);
        landed.navigation.feet = THERE;
        landed.navigation.goal = Some(THERE);
        landed.navigation.next = NavNext::Arrived;
        tick(&CFG, &mut state, &landed);
        assert!(state.leg.is_none() && state.goal == Some(THERE));
        tick(&CFG, &mut state, &landed);
        assert!(state.goal.is_none() && state.until > 4.0, "{state:?}");
    }

    #[test]
    fn an_answer_to_another_goal_is_not_followed() {
        let mut state = RoamState { goal: Some(THERE), picks: 1, ..Default::default() };
        let mut stale = standing(1.0);
        stale.navigation.goal = Some(HERE);
        stale.navigation.next = NavNext::Leg(NavLeg::default());
        tick(&CFG, &mut state, &stale);
        assert!(state.leg.is_none() && state.goal == Some(THERE));
        // The control: the same answer for the goal it holds is followed.
        stale.navigation.goal = Some(THERE);
        tick(&CFG, &mut state, &stale);
        assert!(state.leg.is_some());
    }

    #[test]
    fn a_roamer_too_far_from_its_target_goes_beside_it_when_a_route_goes_there() {
        let keeps_near = RoamCfg { stay_within: 200.0, ..CFG };
        let beside = Vec2::new(700.0, 500.0);
        let far_target = |place: Option<Vec2>| {
            let mut snapshot = standing(5.0);
            snapshot.target_alive = true;
            snapshot.target_pos = Vec2::new(760.0, 468.0);
            snapshot.navigation.target_place = place;
            snapshot
        };
        let mut state = RoamState { until: 1.0, picks: 1, ..Default::default() };
        tick(&keeps_near, &mut state, &far_target(Some(beside)));
        assert_eq!(state.goal, Some(beside));
        // No route to the target: a place by chance.
        let mut state = RoamState { until: 1.0, picks: 1, ..Default::default() };
        tick(&keeps_near, &mut state, &far_target(None));
        assert_eq!(state.goal, Some(THERE));
        // The control: a body that does not keep near goes by chance too.
        let mut state = RoamState { until: 1.0, picks: 1, ..Default::default() };
        tick(&CFG, &mut state, &far_target(Some(beside)));
        assert_eq!(state.goal, Some(THERE));
    }
}
