//! Executable repros ("oracles") for the momentum-speedway bugs observed on
//! 2026-07-16: held-Up loop orbiting, finicky ramp entry, and edge sticking.
//!
//! Every oracle drives the REAL `sanic_speedway()` geometry through the real
//! movement kernel (`ae::step_motion`) with the catalog's authored momentum
//! numbers, records a per-tick trace, and asserts the behavior a player is
//! entitled to. All nine originally reproduced live defects (see the section
//! comments); the 2026-07-16 solver fixes turned them green and they now
//! stand as the speedway's permanent regression gates. Oracle G (jumps on the
//! track land on the track) was added with the same-day depth-occlusion
//! rework that made airborne collision lane-blind.

use ambition::engine_core as ae;

use crate::{
    sanic_speedway, FLOOR_TOP, LOOP_CLOSURE_POINT_INDEX, LOOP_ENTRY_POINT_INDEX, LOOP_SEGMENTS,
    PIT_LEFT_X, PIT_RIGHT_X,
};

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
    /// The kernel's world-hazard gate fired this tick (the game layer would
    /// respawn the body; the probe records the fact).
    reset: bool,
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
        let result = ae::step_motion(
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
            reset: result.events.reset,
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
    let runout_idx = chain_index(&room.world, "sanic_floor_runout");

    // Every drop point has clear sky down to the ground route — clear of the
    // one-way platforms (real landings since the momentum body learned to use
    // them) and of the raised loop structure (Oracle G territory). Drops onto
    // the hills still land on the ROUTE chain: the hills ARE the floor route.
    for (x, vx) in [
        (400.0, 0.0),
        (480.0, 250.0),
        (1200.0, -250.0),
        // 1460, not 1500: the super monitor's lid at ~1490 is a REAL solid.
        (1460.0, 0.0),
        (3300.0, 250.0),
        (5600.0, 0.0),
    ] {
        let mut probe = Probe::airborne(
            ae::Vec2::new(x, 480.0),
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
            } if c == floor_idx || c == runout_idx => {}
            other => panic!(
                "drop at x={x} vx={vx}: expected to land on a ground ROUTE CHAIN \
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

// ─────────────────────────────────────────────────────────────────────────────
// Oracle G — jumps on the track land on the track.
//
// Depth lanes are an implementation detail the player cannot perceive: the
// ramp (-1), loop body (0), and mouth deck / shoulders / runout (+1) all read
// as ONE solid course. Before the 2026-07-16 depth-occlusion rework, an
// airborne body was frozen in the lane it launched from and fell clean
// through every other lane's track — jump on the ramp holding forward and
// the loop mouth was intangible; jump from the floor and the ramp face was.
// Airborne collision is now lane-blind (launch-coincident foreign track is
// suppressed only until the flight separates), and these gates pin the
// Sonic entitlement: what you can see, you can land on.
// ─────────────────────────────────────────────────────────────────────────────

/// The first riding sample after the probe has actually gone airborne.
fn first_landing(trace: &[Sample]) -> Option<&Sample> {
    let launch = trace.iter().position(|s| s.ride.is_none())?;
    trace[launch..].iter().find(|s| s.ride.is_some())
}

#[test]
fn oracle_ramp_jump_relands_on_the_ramp() {
    let room = sanic_speedway();
    let loop_idx = chain_index(&room.world, "sanic_loop");
    let chain = &room.world.chains[loop_idx];
    let mid_ramp_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX) * 0.5;

    // A slow mid-ramp rider jumps and keeps its hands off: the perpendicular
    // arc must come back down onto the ramp it left, exactly like jumping on
    // any slope — not slip through to the base floor.
    let mut probe = Probe::riding_chain(&room.world, loop_idx, mid_ramp_s, 120.0, sanic_params());
    probe.step(&room.world, ae::Vec2::ZERO, true);
    for _ in 0..120 {
        probe.step(&room.world, ae::Vec2::ZERO, false);
    }
    let landing = first_landing(&probe.trace).expect("the jump arc must land within two seconds");
    assert!(
        matches!(landing.ride, Some((ae::SurfaceRef::Chain(c), ..)) if c == loop_idx),
        "a hands-off ramp jump landed off the raised route: {}\n{}",
        fmt_sample(landing),
        dump_tail(&probe.trace, 20)
    );
}

#[test]
fn oracle_ramp_jump_held_forward_lands_on_the_track() {
    let room = sanic_speedway();
    let loop_idx = chain_index(&room.world, "sanic_loop");
    let chain = &room.world.chains[loop_idx];
    let upper_ramp_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX) * 0.75;

    // The reported bug: jump on the upper ramp while holding toward the loop.
    // Air steering carries the arc over the crest into the loop-mouth
    // airspace; the mouth deck and shoulders must catch it. Before the
    // rework the ramp-lane flight fell through the foreground track and hit
    // the base floor.
    let mut probe = Probe::riding_chain(&room.world, loop_idx, upper_ramp_s, 240.0, sanic_params());
    probe.step(&room.world, ae::Vec2::X, true);
    for _ in 0..150 {
        probe.step(&room.world, ae::Vec2::X, false);
    }
    let landing = first_landing(&probe.trace).expect("the jump arc must land");
    assert!(
        matches!(landing.ride, Some((ae::SurfaceRef::Chain(c), ..)) if c == loop_idx),
        "a forward ramp jump fell through the visible track and landed off the \
         raised route: {}\n{}",
        fmt_sample(landing),
        dump_tail(&probe.trace, 20)
    );
}

#[test]
fn oracle_floor_jump_lands_on_the_ramp_face() {
    let room = sanic_speedway();
    let loop_idx = chain_index(&room.world, "sanic_loop");
    let floor_idx = chain_index(&room.world, "sanic_floor_route");
    let chain = &room.world.chains[loop_idx];
    let entry_s = chain.arc_at_vertex(LOOP_ENTRY_POINT_INDEX);

    // A floor runner jumps just past the ramp's base; the descending arc
    // comes down ON the raised ramp face. Landing there is the entitlement —
    // the ramp is visibly solid ground — and it also exercises the full
    // occlusion cycle: the launch starts coincident with the low ramp tail
    // (suppressed), separates at the apex (released), then lands on the very
    // track it launched beneath.
    let mut probe = Probe::riding_chain(&room.world, floor_idx, 1780.0, 300.0, sanic_params());
    probe.step(&room.world, ae::Vec2::ZERO, true);
    for _ in 0..120 {
        probe.step(&room.world, ae::Vec2::ZERO, false);
    }
    let landing = first_landing(&probe.trace).expect("the jump arc must land");
    let on_ramp_face = matches!(
        landing.ride,
        Some((ae::SurfaceRef::Chain(c), s, _)) if c == loop_idx && s < entry_s
    );
    assert!(
        on_ramp_face,
        "a floor jump under the ramp face passed through it instead of landing \
         on it: {}\n{}",
        fmt_sample(landing),
        dump_tail(&probe.trace, 20)
    );
}

#[test]
fn oracle_drop_onto_the_overpass_lands_on_the_raised_track() {
    let room = sanic_speedway();
    let loop_idx = chain_index(&room.world, "sanic_loop");

    // Straight drop above the post-loop descent (x=2700 keeps clear sky: the
    // marker platform at ~2600 is a REAL landing now). Under the old
    // strict-lane rule a base-lane body fell THROUGH this visibly solid
    // foreground track to the floor beneath; lane-blind collision catches it.
    let mut probe = Probe::airborne(
        ae::Vec2::new(2700.0, 520.0),
        ae::Vec2::ZERO,
        0,
        sanic_params(),
    );
    for _ in 0..240 {
        probe.step(&room.world, ae::Vec2::ZERO, false);
        if probe.riding() {
            break;
        }
    }
    assert!(
        matches!(
            probe.motion(),
            ae::SurfaceMotion::Riding { on: ae::SurfaceRef::Chain(c), .. } if c == loop_idx
        ),
        "a drop onto the overpass must land on the raised route: {:?}\n{}",
        probe.motion(),
        dump_tail(&probe.trace, 15)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Oracle H — the expanded course: hills, platforms, springs, and the pit.
//
// The 2026-07-16 course expansion (LDtk-authored level) added rolling hills,
// one-way platforms, a vertical + a diagonal spring, monitors, badniks, and a
// hazard pit. These gates pin the layout promises: every platform is either
// genuinely jumpable or spring-served, springs actually launch a rider, the
// hills flow at speed, and the pit is exactly as lethal as authored.
// ─────────────────────────────────────────────────────────────────────────────

/// Interpolated ground-route height under `x` (the hills are real geometry,
/// so "the ground" is the chain surface, not the flat slab top).
fn ground_y_at(room: &ambition::world::rooms::RoomSpec, x: f32) -> f32 {
    let mut best = f32::MAX;
    for name in ["sanic_floor_route", "sanic_floor_runout"] {
        let chain = &room.world.chains[chain_index(&room.world, name)];
        for pair in chain.points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if (a.x..=b.x).contains(&x) && (b.x - a.x) > 1.0e-3 {
                let t = (x - a.x) / (b.x - a.x);
                best = best.min(a.y + (b.y - a.y) * t);
            }
        }
    }
    best
}

#[test]
fn oracle_every_platform_is_jumpable_or_spring_served() {
    let room = sanic_speedway();
    let params = sanic_params();
    // Ballistic jump apex under the rig gravity, with a landing margin.
    let reachable_lift = params.jump_speed * params.jump_speed / (2.0 * GRAVITY) - 15.0;
    for block in &room.world.blocks {
        if !matches!(block.kind, ae::BlockKind::OneWay) {
            continue;
        }
        let center_x = (block.aabb.min.x + block.aabb.max.x) * 0.5;
        let lift = ground_y_at(&room, center_x) - block.aabb.min.y;
        let spring_served = room.world.blocks.iter().any(|pad| {
            matches!(pad.kind, ae::BlockKind::Rebound { impulse } if impulse.y <= -600.0)
                && pad.aabb.min.x < block.aabb.max.x + 200.0
                && pad.aabb.max.x > block.aabb.min.x - 200.0
                && pad.aabb.min.y > block.aabb.min.y
        });
        assert!(
            lift <= reachable_lift || spring_served,
            "platform at ({:.0},{:.0}) lifts {lift:.0}px — beyond the {reachable_lift:.0}px \
             jump apex and no spring serves it",
            block.aabb.min.x,
            block.aabb.min.y
        );
    }
}

#[test]
fn oracle_a_floor_jump_lands_on_a_marker_platform() {
    let room = sanic_speedway();
    let floor_idx = chain_index(&room.world, "sanic_floor_route");
    let floor = &room.world.chains[floor_idx];
    // Stand on the flat floor beneath marker platform 2 (top y=528, lift 144
    // — inside the ~169px apex) and jump straight up: the body passes through
    // the platform from below (one-way) and lands ON its head coming down.
    let (s, _) = floor.project(ae::Vec2::new(1608.0, FLOOR_TOP));
    let mut probe = Probe::riding_chain(&room.world, floor_idx, s, 0.0, sanic_params());
    probe.step(&room.world, ae::Vec2::ZERO, true);
    for _ in 0..90 {
        probe.step(&room.world, ae::Vec2::ZERO, false);
    }
    let landing = first_landing(&probe.trace).expect("the jump must land");
    let on_platform = matches!(
        landing.ride,
        Some((ae::SurfaceRef::Block(i), ..))
            if matches!(room.world.blocks[i].kind, ae::BlockKind::OneWay)
    );
    assert!(
        on_platform,
        "a full jump under a marker platform lands on the platform: {}\n{}",
        fmt_sample(landing),
        dump_tail(&probe.trace, 15)
    );
}

#[test]
fn oracle_the_vertical_spring_lifts_a_slow_walker_to_the_perch() {
    let room = sanic_speedway();
    let runout_idx = chain_index(&room.world, "sanic_floor_runout");
    let runout = &room.world.chains[runout_idx];
    // Walk onto the vertical spring (impulse (0,-1000)) at a stroll: the
    // launch keeps the walk speed, rises past the 256px perch lift (a plain
    // jump cannot reach it), and comes down on the perch.
    let (s, _) = runout.project(ae::Vec2::new(4700.0, FLOOR_TOP));
    let mut probe = Probe::riding_chain(&room.world, runout_idx, s, 100.0, sanic_params());
    let mut peak = f32::MAX;
    for _ in 0..150 {
        probe.step(&room.world, ae::Vec2::ZERO, false);
        peak = peak.min(probe.pos().y);
        if probe.trace.len() > 5 && probe.riding() {
            break;
        }
    }
    assert!(
        peak < FLOOR_TOP - 260.0,
        "the spring must carry the walker past the perch lift: peak {peak}"
    );
    let landing = first_landing(&probe.trace).expect("the spring arc must land");
    let on_perch = matches!(
        landing.ride,
        Some((ae::SurfaceRef::Block(i), ..))
            if matches!(room.world.blocks[i].kind, ae::BlockKind::OneWay)
                && (room.world.blocks[i].aabb.min.y - 416.0).abs() < 0.5
    );
    assert!(
        on_perch,
        "the walker lands on the spring perch: {}\n{}",
        fmt_sample(landing),
        dump_tail(&probe.trace, 15)
    );
}

#[test]
fn oracle_the_diagonal_spring_flings_the_runner_up_forward() {
    let room = sanic_speedway();
    let runout_idx = chain_index(&room.world, "sanic_floor_runout");
    let runout = &room.world.chains[runout_idx];
    let (s, _) = runout.project(ae::Vec2::new(5170.0, FLOOR_TOP));
    let mut probe = Probe::riding_chain(&room.world, runout_idx, s, 150.0, sanic_params());
    for _ in 0..150 {
        probe.step(&room.world, ae::Vec2::ZERO, false);
        if probe.trace.len() > 5 && probe.riding() {
            break;
        }
    }
    let launched = probe
        .trace
        .iter()
        .any(|sample| sample.ride.is_none() && sample.vel.x > 600.0 && sample.vel.y < -500.0);
    assert!(
        launched,
        "the diagonal pad launches up-forward\n{}",
        dump_tail(&probe.trace, 15)
    );
    let landing = first_landing(&probe.trace).expect("the diagonal arc must land");
    assert!(
        landing.pos.x > 5350.0,
        "the fling carries the runner forward: {}",
        fmt_sample(landing)
    );
}

#[test]
fn oracle_the_pit_swallows_a_walker_and_a_speeding_jump_clears_it() {
    let room = sanic_speedway();
    let floor_idx = chain_index(&room.world, "sanic_floor_route");
    let floor = &room.world.chains[floor_idx];

    // A stroll off the west lip drops into the pit and touches the hazard
    // floor: the kernel raises its reset event (the game layer respawns).
    let (s, _) = floor.project(ae::Vec2::new(PIT_LEFT_X - 40.0, FLOOR_TOP));
    let mut probe = Probe::riding_chain(&room.world, floor_idx, s, 250.0, sanic_params());
    let mut entered_pit = false;
    let mut reset_in_pit = false;
    for _ in 0..240 {
        probe.step(&room.world, ae::Vec2::X, false);
        let pos = probe.pos();
        let in_pit = pos.x > PIT_LEFT_X && pos.x < PIT_RIGHT_X && pos.y > FLOOR_TOP;
        entered_pit |= in_pit;
        if in_pit && probe.trace.last().is_some_and(|sample| sample.reset) {
            reset_in_pit = true;
            break;
        }
    }
    assert!(entered_pit, "a walker falls into the pit");
    assert!(
        reset_in_pit,
        "the pit hazard raises the kernel reset event\n{}",
        dump_tail(&probe.trace, 10)
    );

    // A speeding runner who jumps at the lip sails over the 256px gap and
    // continues on the east ground.
    let (s, _) = floor.project(ae::Vec2::new(PIT_LEFT_X - 400.0, FLOOR_TOP));
    let mut probe = Probe::riding_chain(&room.world, floor_idx, s, 900.0, sanic_params());
    let mut jumped = false;
    for _ in 0..240 {
        let jump = !jumped && probe.pos().x > PIT_LEFT_X - 80.0 && probe.riding();
        if jump {
            jumped = true;
        }
        probe.step(&room.world, ae::Vec2::X, jump);
        if probe.pos().x > PIT_RIGHT_X + 300.0 {
            break;
        }
    }
    assert!(jumped, "the runner reached the lip riding and jumped");
    assert!(
        probe.pos().x > PIT_RIGHT_X + 200.0 && probe.pos().y < FLOOR_TOP + 1.0,
        "a jump at speed clears the pit onto the east ground: {:?}\n{}",
        probe.pos(),
        dump_tail(&probe.trace, 15)
    );
}

#[test]
fn oracle_hills_flow_without_pins_or_flapping() {
    let room = sanic_speedway();
    let floor_idx = chain_index(&room.world, "sanic_floor_route");
    // Run the hills from the start, held right: momentum carries over both
    // crests (brief airborne hops at speed are physical) with no position
    // pins and no attach/shed flapping, reaching the booster approach.
    let mut probe = Probe::riding_chain(&room.world, floor_idx, 180.0, 300.0, sanic_params());
    for _ in 0..300 {
        probe.step(&room.world, ae::Vec2::X, false);
        if probe.pos().x > 1600.0 {
            break;
        }
    }
    assert!(
        probe.pos().x > 1600.0,
        "held-right crosses both hills to the booster approach\n{}",
        dump_tail(&probe.trace, 20)
    );
    assert!(
        find_pin(&probe.trace).is_none(),
        "no position pin crossing the hills\n{}",
        dump_tail(&probe.trace, 25)
    );
    assert!(
        find_flapping(&probe.trace).is_none(),
        "no ride-state flapping crossing the hills\n{}",
        dump_tail(&probe.trace, 25)
    );
}

#[test]
fn oracle_full_course_run_reaches_the_finish() {
    let room = sanic_speedway();
    let floor_idx = chain_index(&room.world, "sanic_floor_route");
    // The showcase line: hold Up+Right the whole way (hills → booster → ramp
    // → ONE loop lap → runout), jump the pit at the lip and hop the spike
    // strip. The run must reach the finish approach with no hazard reset
    // (a reset shows as a giant backwards teleport to spawn).
    let mut probe = Probe::riding_chain(&room.world, floor_idx, 180.0, 200.0, sanic_params());
    let steer = ae::Vec2::new(1.0, -1.0);
    let mut jumped_pit = false;
    let mut jumped_spikes = false;
    let mut best_x: f32 = 0.0;
    for _ in 0..1800 {
        let x = probe.pos().x;
        let jump_now = (!jumped_pit && x > PIT_LEFT_X - 90.0 && x < PIT_LEFT_X && probe.riding())
            || (!jumped_spikes && x > 5540.0 && x < 5640.0 && probe.riding());
        if jump_now && x < PIT_LEFT_X {
            jumped_pit = true;
        } else if jump_now {
            jumped_spikes = true;
        }
        probe.step(&room.world, steer, jump_now);
        assert!(
            !probe.trace.last().is_some_and(|sample| sample.reset),
            "the showcase line must never trip a hazard reset (x={:.0})\n{}",
            probe.pos().x,
            dump_tail(&probe.trace, 20)
        );
        best_x = best_x.max(probe.pos().x);
        if best_x > 6100.0 {
            break;
        }
    }
    assert!(
        best_x > 6100.0,
        "the scripted line reaches the finish approach: best x {best_x}\n{}",
        dump_tail(&probe.trace, 30)
    );
}
