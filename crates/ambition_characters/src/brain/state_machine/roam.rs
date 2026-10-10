//! Roam: go to places in the room, one after the other, and rest at each.
//!
//! The brain owns the POLICY: which place, when, and how long to rest. It
//! does not know the room. Each tick the navigation advisor gives it
//! ([`ae::navigation::NavAdvice`], in the snapshot) some places it can reach
//! from where it stands, and a place beside its target when a route goes
//! there. The brain chooses one and hands it to its [`NavFollower`], which
//! gets the body there. Any body with this brain and the advisor navigates:
//! nothing here is about one character.
//!
//! ALL of [`RoamState`] gates what the body does next, so all of it is
//! rewound (`SnapshotCursor for Brain`).

use ambition_platformer2d_core as ae;
use ae::navigation::mix;

use super::nav_follower::{Followed, NavFollower};
use super::BrainSnapshot;
use crate::actor::control::ActorControlFrame;

/// The pause at a place when the body is at play (s).
const PLAY_PAUSE_S: f32 = 0.25;
/// The pause after a place the body could not get to (s).
const GAVE_UP_PAUSE_S: f32 = 0.5;
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
    /// How often the next place is beside the body it attends to (the
    /// nearest player, for a friendly body) and not a place by chance, 0 to 1.
    pub company: f32,
    /// A body farther than this from the one it attends to goes to it next,
    /// when a route goes there (px). Zero: the body does not keep near.
    pub stay_within: f32,
    /// How often the body is in a playful mood, 0 to 1. A mood lasts four
    /// places: the body runs to each and does not rest between them.
    pub playful: f32,
    /// A resting body faces a target this near (px).
    pub notice_radius: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoamState {
    /// The place the body is going to, and how far it is on the way.
    pub nav: NavFollower,
    /// The sim time (s) when the rest ends.
    pub until: f32,
    /// How many goals this brain has chosen: the seed of the next choice.
    pub picks: u32,
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
    let rest = |picks: u32| {
        if playful(cfg, picks) {
            return PLAY_PAUSE_S;
        }
        cfg.rest_min_s + (cfg.rest_max_s - cfg.rest_min_s).max(0.0) * chance(picks, 0x51)
    };

    let at_play = playful(cfg, state.picks);
    let pace = |far: bool| match (at_play, far) {
        // At play the body runs, and the follower slows it at the point.
        (true, _) => snapshot.max_run_speed,
        (false, true) => cfg.trot_speed,
        (false, false) => cfg.speed,
    };
    // An errand comes before rest and before a place by chance. The place is
    // a fact of where the item lies, so it is the same each tick, and the
    // follower is not sent again while it goes there.
    if let Some(place) = advice.errand.place() {
        if state.nav.goal != Some(place) && !state.nav.committed() {
            state.nav.go_to(place);
        }
        state.nav.drive(snapshot, pace, out);
        return;
    }
    match state.nav.drive(snapshot, pace, out) {
        Followed::Going => return,
        Followed::Arrived => {
            state.until = now + rest(state.picks);
            return;
        }
        Followed::GaveUp => {
            state.until = now + GAVE_UP_PAUSE_S;
            return;
        }
        Followed::Idle => {}
    }

    // At rest. Look at the attended body when it is near. The advice's place
    // is beside it, and a friendly body has no combat target to read.
    let frame = snapshot.acceleration_frame();
    let to_company = advice.target_place.map(|place| frame.to_local(place - advice.feet));
    if let Some(offset) = to_company.filter(|offset| offset.length() <= cfg.notice_radius) {
        out.facing = snapshot.face_toward(offset.x);
    }
    // A body starts with a rest, not with a journey.
    if state.picks == 0 && state.until == 0.0 {
        state.until = now + rest(0);
    }
    if now < state.until || !snapshot.actor_on_ground {
        return;
    }
    let far = |place: &&ae::Vec2| place.distance(advice.feet) > SAME_PLACE;
    let pool: Vec<&ae::Vec2> = advice.waypoints().iter().filter(far).collect();
    if pool.is_empty() {
        // Nowhere to go from here (or no advisor): rest again.
        state.until = now + rest(state.picks).max(0.5);
        state.picks = state.picks.wrapping_add(1);
        return;
    }
    // Beside the target, when the body wants company or is too far from it,
    // and a route goes there. If not, a place by chance.
    let lonely = cfg.stay_within > 0.0 && to_company.is_some_and(|offset| offset.length() > cfg.stay_within);
    let beside_target = advice
        .target_place
        .filter(|place| place.distance(advice.feet) > SAME_PLACE)
        .filter(|_| lonely || chance(state.picks, 0xC0) < cfg.company);
    let place = beside_target.unwrap_or_else(|| *pool[(mix(state.picks as u64 ^ 0xA7) % pool.len() as u64) as usize]);
    state.nav.go_to(place);
    state.picks = state.picks.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ae::navigation::{LegPhase, NavAdvice, NavLeg, NavLegKind, NavNext};
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

    /// A hop that comes down at the landing's height on ANOTHER surface is a
    /// miss: the follower plans again, and gives the goal up after three in a
    /// row. With the height alone as the test it was an arrival each time, the
    /// misses went back to zero, and the follower never gave up.
    #[test]
    fn a_follower_that_lands_on_another_surface_counts_a_miss_and_gives_up_after_three() {
        use super::super::nav_follower::{Followed, NavFollower};
        // `THERE` is on a surface from x 360 to 440. The other surface of
        // that height is under x 100.
        let leg = NavLeg {
            kind: NavLegKind::Hop,
            start: HERE,
            takeoff: HERE,
            land: THERE,
            land_span: [360.0, 440.0],
        };
        let landed_at = |x: f32| {
            let mut snapshot = standing(5.0);
            snapshot.navigation.feet = Vec2::new(x, THERE.y);
            snapshot
        };
        let in_the_air = |misses: u8| NavFollower {
            goal: Some(THERE),
            leg: Some(leg),
            phase: LegPhase::Air,
            until: 100.0,
            misses,
        };
        let mut out = ActorControlFrame::neutral();
        // The control: on the landing's surface, short of the point. The leg
        // is done and the misses are forgotten.
        let mut nav = in_the_air(2);
        assert_eq!(nav.drive(&landed_at(380.0), |_| 50.0, &mut out), Followed::Going);
        assert_eq!((nav.leg, nav.misses), (None, 0));

        let mut nav = in_the_air(0);
        for miss in 1..=3 {
            nav.leg = Some(leg);
            nav.phase = LegPhase::Air;
            assert_eq!(nav.drive(&landed_at(100.0), |_| 50.0, &mut out), Followed::Going);
            assert_eq!((nav.leg, nav.misses), (None, miss), "landing {miss} on the other surface");
        }
        assert_eq!(nav.drive(&landed_at(100.0), |_| 50.0, &mut out), Followed::GaveUp);
    }

    #[test]
    fn a_roamer_rests_chooses_a_place_takes_its_leg_and_rests_again() {
        let mut state = RoamState::default();
        // It starts with a rest.
        tick(&CFG, &mut state, &standing(0.1));
        assert!(state.nav.goal.is_none() && state.until >= 1.1 && state.until <= 2.1, "{state:?}");
        tick(&CFG, &mut state, &standing(0.5));
        assert!(state.nav.goal.is_none());
        // The rest ends: the one place that is not here.
        tick(&CFG, &mut state, &standing(3.0));
        assert_eq!((state.nav.goal, state.picks), (Some(THERE), 1));

        // The advisor answers the goal with a hop from the edge.
        let leg = NavLeg {
            kind: NavLegKind::Hop,
            start: HERE,
            takeoff: HERE,
            land: THERE,
            land_span: [THERE.x - 40.0, THERE.x + 40.0],
        };
        let mut answered = standing(3.1);
        answered.navigation.goal = Some(THERE);
        answered.navigation.next = NavNext::Leg(leg);
        tick(&CFG, &mut state, &answered);
        assert_eq!((state.nav.leg, state.nav.phase), (Some(leg), LegPhase::Approach));
        // At rest at the start: it commits, then it jumps toward the place.
        tick(&CFG, &mut state, &answered);
        assert_eq!(state.nav.phase, LegPhase::Commit);
        let jump = tick(&CFG, &mut state, &answered);
        assert!(jump.jump_pressed && jump.jump_held && jump.locomotion.x > 0.0, "{jump:?}");

        // On the landing surface: the leg is done, and the advisor says so.
        state.nav.phase = LegPhase::Air;
        let mut landed = standing(4.0);
        landed.navigation.feet = THERE;
        landed.navigation.goal = Some(THERE);
        landed.navigation.next = NavNext::Arrived;
        tick(&CFG, &mut state, &landed);
        assert!(state.nav.leg.is_none() && state.nav.goal == Some(THERE));
        tick(&CFG, &mut state, &landed);
        assert!(state.nav.goal.is_none() && state.until > 4.0, "{state:?}");
    }

    #[test]
    fn an_answer_to_another_goal_is_not_followed() {
        let mut state = RoamState { picks: 1, ..Default::default() };
        state.nav.go_to(THERE);
        let mut stale = standing(1.0);
        stale.navigation.goal = Some(HERE);
        stale.navigation.next = NavNext::Leg(NavLeg::default());
        tick(&CFG, &mut state, &stale);
        assert!(state.nav.leg.is_none() && state.nav.goal == Some(THERE));
        // The control: the same answer for the goal it holds is followed.
        stale.navigation.goal = Some(THERE);
        tick(&CFG, &mut state, &stale);
        assert!(state.nav.leg.is_some());
    }

    /// An errand is before rest: a resting body goes to the errand's place,
    /// and is not sent again while it goes there. The control: with no
    /// errand, the same resting body stays.
    #[test]
    fn a_resting_roamer_goes_on_its_errand() {
        let errand = |sight| {
            let mut snapshot = standing(1.0);
            snapshot.navigation.errand = sight;
            snapshot
        };
        let mut resting = RoamState { until: 100.0, picks: 1, ..Default::default() };
        tick(&CFG, &mut resting, &errand(ae::navigation::ErrandSight::None));
        assert_eq!(resting.nav.goal, None, "control: a resting body with no errand stays");
        tick(&CFG, &mut resting, &errand(ae::navigation::ErrandSight::At(THERE)));
        assert_eq!(resting.nav.goal, Some(THERE));
        let mut going = errand(ae::navigation::ErrandSight::At(THERE));
        going.navigation.goal = Some(THERE);
        going.navigation.next = NavNext::Leg(NavLeg::default());
        tick(&CFG, &mut resting, &going);
        let leg = resting.nav.leg;
        assert!(leg.is_some(), "the body follows the leg to its errand");
        tick(&CFG, &mut resting, &going);
        assert_eq!(resting.nav.goal, Some(THERE), "the errand did not send the follower again");
        // A place the errand cannot reach is not a place to go to.
        let mut idle = RoamState { until: 100.0, picks: 1, ..Default::default() };
        tick(&CFG, &mut idle, &errand(ae::navigation::ErrandSight::NoRoute));
        assert_eq!(idle.nav.goal, None);
        // The zone that leads to the item is a place to go to.
        let mut idle = RoamState { until: 100.0, picks: 1, ..Default::default() };
        tick(&CFG, &mut idle, &errand(ae::navigation::ErrandSight::Door(THERE)));
        assert_eq!(idle.nav.goal, Some(THERE), "a body goes to the zone that leads to its errand");
    }

    #[test]
    fn a_roamer_too_far_from_its_target_goes_beside_it_when_a_route_goes_there() {
        let keeps_near = RoamCfg { stay_within: 200.0, ..CFG };
        let beside = Vec2::new(700.0, 500.0);
        // A friendly body: no combat target. What it attends to is in the advice.
        let far_target = |place: Option<Vec2>| {
            let mut snapshot = standing(5.0);
            snapshot.navigation.target_place = place;
            snapshot
        };
        let mut state = RoamState { until: 1.0, picks: 1, ..Default::default() };
        tick(&keeps_near, &mut state, &far_target(Some(beside)));
        assert_eq!(state.nav.goal, Some(beside));
        // No route to the target: a place by chance.
        let mut state = RoamState { until: 1.0, picks: 1, ..Default::default() };
        tick(&keeps_near, &mut state, &far_target(None));
        assert_eq!(state.nav.goal, Some(THERE));
        // The control: a body that does not keep near goes by chance too.
        let mut state = RoamState { until: 1.0, picks: 1, ..Default::default() };
        tick(&CFG, &mut state, &far_target(Some(beside)));
        assert_eq!(state.nav.goal, Some(THERE));
    }
}
