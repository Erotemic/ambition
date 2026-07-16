//! Executable repros ("oracles") for the momentum-speedway bugs observed on
//! 2026-07-16: held-Up loop orbiting, finicky ramp entry, and edge sticking.
//!
//! Every oracle drives the REAL `sanic_speedway()` geometry through the real
//! movement kernel (`ae::step_motion`) with the catalog's authored momentum
//! numbers, records a per-tick trace, and asserts the behavior a player is
//! entitled to. All nine originally reproduced live defects (see the section
//! comments); the 2026-07-16 solver fixes turned them green and they now
//! stand as the speedway's permanent regression gates.

use ambition::engine_core as ae;

use crate::{sanic_speedway, LOOP_CLOSURE_POINT_INDEX, LOOP_ENTRY_POINT_INDEX, LOOP_SEGMENTS};

/// The rig gravity every momentum test in this crate uses (60 Hz, y-down).
const GRAVITY: f32 = 1450.0;
const DT: f32 = 1.0 / 60.0;

/// The catalog-authored Sanic momentum row: `ground_accel`/`top_speed`/
/// `jump_speed`/`stick_factor` are authored, everything else hydrates from
/// the same `MomentumParams::default()` production uses.
fn sanic_params() -> ae::MomentumParams {
    ae::MomentumParams {
        ground_accel: 900.0,
        top_speed: 1200.0,
        jump_speed: 700.0,
        stick_factor: 4.0,
        ..Default::default()
    }
}

fn chain_index(world: &ae::World, name: &str) -> usize {
    world
        .chains
        .iter()
        .position(|chain| chain.name == name)
        .unwrap_or_else(|| panic!("the speedway authors a '{name}' chain"))
}

/// One recorded kernel tick.
#[derive(Clone, Copy, Debug)]
struct Sample {
    tick: u32,
    pos: ae::Vec2,
    vel: ae::Vec2,
    /// `Some((surface, arc, v_t))` while riding.
    ride: Option<(ae::SurfaceRef, f32, f32)>,
    lane: i8,
}

/// A traced surface-momentum body stepped through the production kernel —
/// the same dispatch (`ae::step_motion`) and radius derivation
/// (`size.min_element() * 0.5`, so a `splat(32)` box rides as a radius-16
/// circle) the game uses.
struct Probe {
    scratch: ae::BodyClusterScratch,
    model: ae::MotionModel,
    trace: Vec<Sample>,
}

impl Probe {
    fn with_state(
        pos: ae::Vec2,
        vel: ae::Vec2,
        state: ae::SurfaceMotion,
        lane: i8,
        params: ae::MomentumParams,
    ) -> Self {
        let mut scratch =
            ae::BodyClusterScratch::new_with_abilities(pos, ae::AbilitySet::default());
        scratch.kinematics.size = ae::Vec2::splat(32.0);
        scratch.kinematics.vel = vel;
        let mut model = ae::MotionModel::surface_momentum(params);
        let ae::MotionModel::SurfaceMomentum(m) = &mut model else {
            unreachable!()
        };
        m.state = state;
        m.depth_lane = lane;
        Self {
            scratch,
            model,
            trace: Vec::new(),
        }
    }

    /// A ballistic body on simulated-depth lane `lane`.
    fn airborne(pos: ae::Vec2, vel: ae::Vec2, lane: i8, params: ae::MomentumParams) -> Self {
        Self::with_state(pos, vel, ae::SurfaceMotion::Airborne, lane, params)
    }

    /// A rider attached to `world.chains[chain_index]` at arc `s`, moving at
    /// signed tangential speed `v_t`.
    fn riding_chain(
        world: &ae::World,
        chain_index: usize,
        s: f32,
        v_t: f32,
        params: ae::MomentumParams,
    ) -> Self {
        let chain = &world.chains[chain_index];
        let frame = chain.frame_at(s);
        Self::with_state(
            frame.point + frame.normal * 16.0,
            frame.tangent * v_t,
            ae::SurfaceMotion::Riding {
                on: ae::SurfaceRef::Chain(chain_index),
                s,
                v_t,
            },
            chain.segment_depth(frame.segment),
            params,
        )
    }

    /// One 60 Hz kernel tick under the standard downward gravity frame.
    fn step(&mut self, world: &ae::World, steer: ae::Vec2, jump: bool) {
        let mut clusters = self.scratch.as_mut();
        ae::step_motion(
            &mut self.model,
            &mut clusters,
            ae::MotionStepContext {
                world,
                input: ae::InputState {
                    axes: ae::LocalAxes::new(steer.x, steer.y),
                    jump_pressed: jump,
                    ..ae::InputState::default()
                },
                frame: ae::MotionFrame::from_acceleration(ae::Vec2::new(0.0, GRAVITY))
                    .expect("non-zero acceleration"),
                facing_intent: 0.0,
                dt: DT,
            },
        );
        let ae::MotionModel::SurfaceMomentum(m) = &self.model else {
            unreachable!()
        };
        let ride = match m.state {
            ae::SurfaceMotion::Riding { on, s, v_t } => Some((on, s, v_t)),
            ae::SurfaceMotion::Airborne => None,
        };
        self.trace.push(Sample {
            tick: self.trace.len() as u32,
            pos: self.scratch.kinematics.pos,
            vel: self.scratch.kinematics.vel,
            ride,
            lane: m.depth_lane,
        });
    }

    fn motion(&self) -> ae::SurfaceMotion {
        let ae::MotionModel::SurfaceMomentum(m) = &self.model else {
            unreachable!()
        };
        m.state
    }

    fn pos(&self) -> ae::Vec2 {
        self.scratch.kinematics.pos
    }

    fn riding(&self) -> bool {
        matches!(self.motion(), ae::SurfaceMotion::Riding { .. })
    }
}

fn fmt_sample(s: &Sample) -> String {
    let ride = match s.ride {
        Some((on, arc, v_t)) => format!("{on:?} s={arc:7.1} v_t={v_t:7.1}"),
        None => "Airborne".to_string(),
    };
    format!(
        "t={:>4} pos=({:7.1},{:6.1}) vel=({:7.1},{:7.1}) lane={:+} {}",
        s.tick, s.pos.x, s.pos.y, s.vel.x, s.vel.y, s.lane, ride
    )
}

fn dump_tail(trace: &[Sample], n: usize) -> String {
    let start = trace.len().saturating_sub(n);
    trace[start..]
        .iter()
        .map(fmt_sample)
        .collect::<Vec<_>>()
        .join("\n")
}

fn dump_window(trace: &[Sample], center: usize, before: usize, after: usize) -> String {
    let start = center.saturating_sub(before);
    let end = (center + after).min(trace.len());
    trace[start..end]
        .iter()
        .map(fmt_sample)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Count how many times a CONTINUOUS ride on the loop chain crossed the arc
/// position of the loop's topmost point. Junction hops (arc teleports between
/// coincident occurrences) are excluded, so each count is one genuine pass
/// over the top of the loop — one lap, in either travel direction.
fn loop_top_crossings(trace: &[Sample], loop_chain: usize, top_s: f32) -> usize {
    let mut laps = 0;
    for pair in trace.windows(2) {
        let (Some((ae::SurfaceRef::Chain(c0), s0, _)), Some((ae::SurfaceRef::Chain(c1), s1, _))) =
            (pair[0].ride, pair[1].ride)
        else {
            continue;
        };
        if c0 != loop_chain || c1 != loop_chain {
            continue;
        }
        // A rider moves at most ~20px/tick; a junction switch teleports the
        // arc by the whole loop circumference. Only count real travel.
        if (s1 - s0).abs() > 60.0 {
            continue;
        }
        if (s0 < top_s) != (s1 < top_s) {
            laps += 1;
        }
    }
    laps
}

fn rode_chain(trace: &[Sample], chain: usize) -> bool {
    trace
        .iter()
        .any(|s| matches!(s.ride, Some((ae::SurfaceRef::Chain(c), _, _)) if c == chain))
}

/// Lap crossings up to (and including) the first sample satisfying `exit`,
/// plus whether the exit was reached at all. Counting past the exit would
/// blame the oracle's route for whatever the course does afterwards — e.g.
/// the directional booster pad legitimately throwing the finished rider back
/// toward the ramp for a fresh, player-held second entry.
fn laps_until_exit(
    trace: &[Sample],
    loop_chain: usize,
    top_s: f32,
    exit: impl Fn(&Sample) -> bool,
) -> (usize, bool) {
    match trace.iter().position(exit) {
        Some(end) => (loop_top_crossings(&trace[..=end], loop_chain, top_s), true),
        None => (loop_top_crossings(trace, loop_chain, top_s), false),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Oracle A/B — the course contract at AUTHORED params: a runner holding Up
// (the input the course demands — it is the only way onto the ramp) must ride
// the loop exactly once and come out the far side.
//
// Today these pin long before the routing question even arises: at authored
// speed the convex joints near the ramp top (forward) and on the descent
// (reverse) trip the launch rule, the launched circle is instantly recaptured
// by the same surface, and the attach/shed pair repeats every tick with the
// position frozen and v_t intact — the "stuck on an edge, then unstuck at
// full speed" a player feels. The pure routing bug is isolated separately in
// the `route_bias_isolation` oracles below.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn oracle_held_up_forward_run_rides_the_loop_exactly_once() {
    let room = sanic_speedway();
    let loop_idx = chain_index(&room.world, "sanic_loop");
    let floor_idx = chain_index(&room.world, "sanic_floor_route");
    let chain = &room.world.chains[loop_idx];
    let closure_s = chain.arc_at_vertex(LOOP_CLOSURE_POINT_INDEX);
    let top_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX + LOOP_SEGMENTS / 2);

    // On the floor guide left of the ramp fork, running right, holding Up the
    // whole way — the exact input a player uses to take the high route.
    let mut probe = Probe::riding_chain(&room.world, floor_idx, 1200.0, 900.0, sanic_params());
    for _ in 0..1200 {
        probe.step(&room.world, ae::Vec2::new(1.0, -1.0), false);
    }

    assert!(
        rode_chain(&probe.trace, loop_idx),
        "precondition: holding Up+Right must transfer from the floor guide onto the ramp\n{}",
        dump_tail(&probe.trace, 30)
    );
    let (laps, reached_overpass) = laps_until_exit(
        &probe.trace,
        loop_idx,
        top_s,
        |s| matches!(s.ride, Some((ae::SurfaceRef::Chain(c), arc, _)) if c == loop_idx && arc > closure_s + 100.0),
    );
    assert!(
        laps == 1 && reached_overpass,
        "held Up must ride the loop exactly once and exit onto the overpass; \
         rode {laps} laps in 20s, reached_overpass={reached_overpass}\n{}",
        dump_tail(&probe.trace, 30)
    );
}

#[test]
fn oracle_held_up_reverse_run_exits_down_the_ramp_after_one_lap() {
    let room = sanic_speedway();
    let loop_idx = chain_index(&room.world, "sanic_loop");
    let floor_idx = chain_index(&room.world, "sanic_floor_route");
    let chain = &room.world.chains[loop_idx];
    let entry_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX);
    let top_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX + LOOP_SEGMENTS / 2);

    // Jon's concrete repro: start right of the runout, run LEFT holding Up.
    // Up at the runout fork climbs the descent; the correct route is then one
    // reverse revolution and out down the entry ramp.
    let mut probe = Probe::riding_chain(&room.world, floor_idx, 3400.0, -600.0, sanic_params());
    for _ in 0..1400 {
        probe.step(&room.world, ae::Vec2::new(-1.0, -1.0), false);
    }

    assert!(
        rode_chain(&probe.trace, loop_idx),
        "precondition: holding Up+Left must climb onto the runout at the x=2920 fork\n{}",
        dump_tail(&probe.trace, 30)
    );
    let (laps, exited_down_ramp) =
        laps_until_exit(&probe.trace, loop_idx, top_s, |s| match s.ride {
            Some((ae::SurfaceRef::Chain(c), arc, _)) if c == loop_idx => arc < entry_s - 100.0,
            Some((ae::SurfaceRef::Chain(c), _, _)) if c == floor_idx => s.pos.x < 1700.0,
            _ => false,
        });
    assert!(
        laps == 1 && exited_down_ramp,
        "a reverse run must make exactly one revolution and leave down the entry ramp; \
         rode {laps} laps in ~23s, exited_down_ramp={exited_down_ramp}\n{}",
        dump_tail(&probe.trace, 30)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Oracle A2/B2 — pure route topology: held Up must not orbit the loop.
//
// Sticky, slope-free tuning (the same isolation `reverse_loop_exits_after_
// one_revolution_instead_of_reentering_forever` uses) neutralizes the
// convex-launch pin so these oracles ask ONLY the routing question. Both
// loop shoulders rise, so a held Up out-scores the flat overpass and the
// falling ramp at the mouth junction on every pass: route bias is a LEVEL,
// re-applied per crossing, when player intent is an EDGE ("take the high
// route at the next fork").
// ─────────────────────────────────────────────────────────────────────────────

/// Route-topology isolation tuning: nothing sheds, nothing launches, speed
/// never changes — only junction selection is under test.
fn sticky_topology_params() -> ae::MomentumParams {
    ae::MomentumParams {
        ground_accel: 0.0,
        brake: 0.0,
        friction: 0.0,
        slope_factor: 0.0,
        top_speed: 2000.0,
        air_accel: 0.0,
        stick_factor: 1000.0,
        min_stick_speed: 0.0,
        ..Default::default()
    }
}

#[test]
fn oracle_route_bias_isolation_held_up_forward_rider_exits_after_one_lap() {
    let room = sanic_speedway();
    let loop_idx = chain_index(&room.world, "sanic_loop");
    let chain = &room.world.chains[loop_idx];
    let entry_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX);
    let closure_s = chain.arc_at_vertex(LOOP_CLOSURE_POINT_INDEX);
    let top_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX + LOOP_SEGMENTS / 2);

    let mut probe = Probe::riding_chain(
        &room.world,
        loop_idx,
        entry_s - 30.0,
        900.0,
        sticky_topology_params(),
    );
    for _ in 0..900 {
        probe.step(&room.world, ae::Vec2::new(0.0, -1.0), false);
    }
    let (laps, reached_overpass) = laps_until_exit(
        &probe.trace,
        loop_idx,
        top_s,
        |s| matches!(s.ride, Some((ae::SurfaceRef::Chain(c), arc, _)) if c == loop_idx && arc > closure_s + 100.0),
    );
    assert!(
        laps == 1 && reached_overpass,
        "held Up must ride the loop once and exit onto the overpass; \
         rode {laps} laps in 15s, reached_overpass={reached_overpass}\n{}",
        dump_tail(&probe.trace, 20)
    );
}

#[test]
fn oracle_route_bias_isolation_held_up_reverse_rider_exits_after_one_lap() {
    let room = sanic_speedway();
    let loop_idx = chain_index(&room.world, "sanic_loop");
    let chain = &room.world.chains[loop_idx];
    let entry_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX);
    let top_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX + LOOP_SEGMENTS / 2);
    let closure_s = chain.arc_at_vertex(LOOP_CLOSURE_POINT_INDEX);

    let mut probe = Probe::riding_chain(
        &room.world,
        loop_idx,
        closure_s + 180.0,
        -900.0,
        sticky_topology_params(),
    );
    for _ in 0..900 {
        probe.step(&room.world, ae::Vec2::new(0.0, -1.0), false);
    }
    let (laps, exited_down_ramp) = laps_until_exit(
        &probe.trace,
        loop_idx,
        top_s,
        |s| matches!(s.ride, Some((ae::SurfaceRef::Chain(c), arc, _)) if c == loop_idx && arc < entry_s - 100.0),
    );
    assert!(
        laps == 1 && exited_down_ramp,
        "a reverse rider holding Up must make exactly one revolution and leave down the \
         entry ramp; rode {laps} laps in 15s, exited_down_ramp={exited_down_ramp}\n{}",
        dump_tail(&probe.trace, 20)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Oracle C/D — surface identity: landings must attach to the ROUTE CHAIN.
//
// The floor is two coincident rideable surfaces: the authored guide chain
// (which carries the ramp/runout junctions) and the solid block (which
// carries none). Which one a landing attaches to decides whether holding Up
// at a fork does anything at all.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn oracle_flat_floor_landings_attach_to_the_route_chain_not_the_block() {
    let room = sanic_speedway();
    let floor_idx = chain_index(&room.world, "sanic_floor_route");

    for (x, vx) in [
        (300.0, 0.0),
        (300.0, 250.0),
        (900.0, -250.0),
        (1500.0, 250.0),
        (2600.0, 0.0),
        (3300.0, 250.0),
    ] {
        let mut probe = Probe::airborne(
            ae::Vec2::new(x, 520.0),
            ae::Vec2::new(vx, 0.0),
            0,
            sanic_params(),
        );
        for _ in 0..240 {
            probe.step(&room.world, ae::Vec2::ZERO, false);
            if probe.riding() {
                break;
            }
        }
        match probe.motion() {
            ae::SurfaceMotion::Riding {
                on: ae::SurfaceRef::Chain(c),
                ..
            } if c == floor_idx => {}
            other => panic!(
                "drop at x={x} vx={vx}: expected to land on the floor ROUTE CHAIN \
                 (junction steering lives there); attached to {other:?} instead\n{}",
                dump_tail(&probe.trace, 15)
            ),
        }
    }
}

#[test]
fn oracle_descent_launched_rider_lands_on_the_route_chain() {
    let room = sanic_speedway();
    let floor_idx = chain_index(&room.world, "sanic_floor_route");

    // The exact airborne state the convex descent leaves behind: fast,
    // rightward, still on simulated-depth lane +1 (the outbound rail's lane),
    // over open floor. Launching there is authored as valid Sonic behavior,
    // so the landing must return the rider to the route network.
    let mut probe = Probe::airborne(
        ae::Vec2::new(3050.0, 532.0),
        ae::Vec2::new(900.0, 150.0),
        1,
        sanic_params(),
    );
    for _ in 0..240 {
        probe.step(&room.world, ae::Vec2::ZERO, false);
        if probe.riding() {
            break;
        }
    }
    match probe.motion() {
        ae::SurfaceMotion::Riding {
            on: ae::SurfaceRef::Chain(c),
            ..
        } if c == floor_idx => {}
        other => panic!(
            "a rider launched from the descent must land back on the floor ROUTE CHAIN; \
             attached to {other:?} — stranded off the junction network\n{}",
            dump_tail(&probe.trace, 15)
        ),
    }
}

#[test]
fn oracle_block_stranded_rider_can_still_take_the_raised_route_by_holding_up() {
    let room = sanic_speedway();
    let loop_idx = chain_index(&room.world, "sanic_loop");

    // Land the way a descent launch actually lands today (lane +1 over open
    // floor → the block catches it), then run LEFT holding Up across BOTH
    // raised-route forks (x=2920 runout, x=1740 ramp). A player doing this is
    // asking for the high route and must get it.
    let mut probe = Probe::airborne(
        ae::Vec2::new(3050.0, 532.0),
        ae::Vec2::new(600.0, 150.0),
        1,
        sanic_params(),
    );
    for _ in 0..600 {
        probe.step(&room.world, ae::Vec2::new(-1.0, -1.0), false);
        if probe.pos().x < 60.0 || rode_chain(&probe.trace, loop_idx) {
            break;
        }
    }
    assert!(
        rode_chain(&probe.trace, loop_idx),
        "holding Up+Left crossed the x=2920 and x=1740 forks without ever entering the \
         raised route — the rider is riding the floor BLOCK, where junctions do not exist; \
         final={:?}\n{}",
        probe.motion(),
        dump_tail(&probe.trace, 20)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Oracle E — the authored speed booster must launch a momentum rider.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn oracle_speed_booster_boosts_a_momentum_rider() {
    let room = sanic_speedway();
    let floor_idx = chain_index(&room.world, "sanic_floor_route");

    // Run right across the pad at x=1640..1712 (authored impulse (1120,-260):
    // "Feed the raised ramp with enough horizontal speed"). For a RIDING
    // momentum body the pad is a speed booster: the impulse projects onto the
    // running tangent, so the observable is tangential speed near the pad's
    // 1120 px/s — not an airborne launch.
    let mut probe = Probe::riding_chain(&room.world, floor_idx, 1500.0, 600.0, sanic_params());
    for _ in 0..90 {
        probe.step(&room.world, ae::Vec2::X, false);
    }
    let boosted = probe
        .trace
        .iter()
        .any(|s| matches!(s.ride, Some((_, _, v_t)) if v_t.abs() > 1050.0) || s.vel.x > 1050.0);
    assert!(
        boosted,
        "the rider crossed the speed booster at running speed and the authored \
         (1120,-260) impulse never fired\n{}",
        dump_tail(&probe.trace, 20)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Oracle F — the stuck-then-burst detector.
//
// "Getting stuck breaks going fast": no window may hold the body's position
// pinned while it reports meaningful speed, and the ride state may not flap
// between Riding and Airborne. The scripted scenarios cover the inputs from
// the report plus slow shoulder climbs (the shed/reattach boundary) and
// seeded pseudo-random play.
// ─────────────────────────────────────────────────────────────────────────────

/// Position pinned (< 2px net over 15 ticks) while reported speed never drops
/// below 200 px/s — the "frozen position, integrating velocity" signature.
fn find_pin(trace: &[Sample]) -> Option<usize> {
    const WINDOW: usize = 15;
    for i in WINDOW..trace.len() {
        let disp = (trace[i].pos - trace[i - WINDOW].pos).length();
        let min_speed = trace[i - WINDOW..=i]
            .iter()
            .map(|s| s.vel.length())
            .fold(f32::INFINITY, f32::min);
        if disp < 2.0 && min_speed > 200.0 {
            return Some(i);
        }
    }
    None
}

/// Riding/Airborne flapping: 8+ attach/shed transitions inside 40 ticks.
fn find_flapping(trace: &[Sample]) -> Option<usize> {
    const WINDOW: usize = 40;
    const MAX_FLIPS: usize = 8;
    let riding: Vec<bool> = trace.iter().map(|s| s.ride.is_some()).collect();
    for i in WINDOW..riding.len() {
        let flips = riding[i - WINDOW..=i]
            .windows(2)
            .filter(|pair| pair[0] != pair[1])
            .count();
        if flips >= MAX_FLIPS {
            return Some(i);
        }
    }
    None
}

/// Deterministic pseudo-random input script (no wall-clock, no `rand`).
fn scripted_inputs(seed: u64, ticks: usize) -> Vec<(ae::Vec2, bool)> {
    let mut state = seed;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as u32
    };
    let mut out = Vec::with_capacity(ticks);
    while out.len() < ticks {
        let hold = 8 + (next() % 17) as usize;
        let ax = (next() % 3) as f32 - 1.0;
        let ay = (next() % 3) as f32 - 1.0;
        let jump = next() % 8 == 0;
        for i in 0..hold {
            if out.len() == ticks {
                break;
            }
            out.push((ae::Vec2::new(ax, ay), jump && i == 0));
        }
    }
    out
}

fn held(steer: ae::Vec2, ticks: usize) -> Vec<(ae::Vec2, bool)> {
    vec![(steer, false); ticks]
}

#[test]
fn oracle_soak_position_never_pins_and_ride_state_never_flaps() {
    let room = sanic_speedway();
    let loop_idx = chain_index(&room.world, "sanic_loop");
    let floor_idx = chain_index(&room.world, "sanic_floor_route");
    let chain = &room.world.chains[loop_idx];
    let entry_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX);
    let closure_s = chain.arc_at_vertex(LOOP_CLOSURE_POINT_INDEX);

    let scenarios: Vec<(&str, Probe, Vec<(ae::Vec2, bool)>)> = vec![
        (
            "spawn drop, hold Right",
            Probe::airborne(
                ae::Vec2::new(160.0, 608.0),
                ae::Vec2::ZERO,
                0,
                sanic_params(),
            ),
            held(ae::Vec2::X, 1800),
        ),
        (
            "floor run, hold Up+Right",
            Probe::riding_chain(&room.world, floor_idx, 600.0, 300.0, sanic_params()),
            held(ae::Vec2::new(1.0, -1.0), 1800),
        ),
        (
            "right side, hold Up+Left",
            Probe::riding_chain(&room.world, floor_idx, 3600.0, -300.0, sanic_params()),
            held(ae::Vec2::new(-1.0, -1.0), 1800),
        ),
        (
            "slow climb up the right shoulder",
            Probe::riding_chain(
                &room.world,
                loop_idx,
                entry_s + 150.0,
                250.0,
                sanic_params(),
            ),
            held(ae::Vec2::new(1.0, -1.0), 1800),
        ),
        (
            "slow reverse climb up the left shoulder",
            Probe::riding_chain(
                &room.world,
                loop_idx,
                closure_s - 150.0,
                -250.0,
                sanic_params(),
            ),
            held(ae::Vec2::new(-1.0, -1.0), 1800),
        ),
        (
            "seeded random play #11",
            Probe::riding_chain(&room.world, floor_idx, 800.0, 300.0, sanic_params()),
            scripted_inputs(11, 1800),
        ),
        (
            "seeded random play #23",
            Probe::riding_chain(&room.world, floor_idx, 800.0, 300.0, sanic_params()),
            scripted_inputs(23, 1800),
        ),
        (
            "seeded random play #47",
            Probe::riding_chain(&room.world, floor_idx, 800.0, 300.0, sanic_params()),
            scripted_inputs(47, 1800),
        ),
        (
            "seeded random play #101",
            Probe::riding_chain(&room.world, floor_idx, 800.0, 300.0, sanic_params()),
            scripted_inputs(101, 1800),
        ),
    ];

    let mut failures = Vec::new();
    for (name, mut probe, inputs) in scenarios {
        for (steer, jump) in inputs {
            probe.step(&room.world, steer, jump);
            let pos = probe.pos();
            // Off the course (fell out / past the world): stop the script.
            if pos.y > 1100.0 || pos.x < -100.0 || pos.x > 4100.0 {
                break;
            }
        }
        if let Some(at) = find_pin(&probe.trace) {
            failures.push(format!(
                "[{name}] POSITION PINNED at tick {at}: <2px of travel across 15 ticks \
                 while speed never dropped below 200 px/s\n{}",
                dump_window(&probe.trace, at, 25, 10)
            ));
        }
        if let Some(at) = find_flapping(&probe.trace) {
            failures.push(format!(
                "[{name}] RIDE-STATE FLAPPING at tick {at}: 8+ attach/shed transitions \
                 inside 40 ticks\n{}",
                dump_window(&probe.trace, at, 45, 10)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "stuck/jitter detector fired {} time(s):\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
