// Portal integration test: only built with the portal mechanic + RL stepping
// API. Compiled out (empty test binary) when `portal` is disabled.
#![cfg(all(feature = "portal", feature = "rl_sim"))]
//! Regression guard for "walk into a floor portal, press nothing, bounce a few
//! times, then lose all momentum / FALL THROUGH THE FLOOR" (Jon, 2026-06-09).
//!
//! Station A in `portal_lab` is an authored ground↔ground pair. A body that
//! enters the first floor portal with no further input should ping-pong between
//! the two floor portals forever, carrying its momentum through each crossing —
//! it must NEVER ground on the (open) portal floor and must NEVER fall through it
//! and trip the fall-out-of-world death reset.
//!
//! The bug: the host-surface carve lagged a frame behind transit, so on every
//! re-contact the floor was still SOLID when collision ran — the body thudded
//! onto it (landing sfx), its entry velocity was zeroed, and it then re-sank from
//! rest (popping out at only the gravity-from-rest speed, never the real entry
//! speed). On less forgiving geometry the same grounding race let the body tunnel
//! through. The fix opens the carve the same frame the body reaches the opening
//! (keyed off the body's current position, before collision), so the body sinks
//! straight through carrying its momentum.
//!
//! Driven through the public SandboxSim API, asserting only on observed player
//! position / velocity / on-ground / reset counter.

mod common;
use common::{base, first_floor_authored_portal_pair};

use ambition_app::rl_sim::TimestepMode;
use ambition_app::{AgentAction, SandboxSim, SandboxSimOptions};

struct BounceStats {
    died_at: Option<usize>,
    grounded_frames: usize,
}

fn run_bounce(dt: f32) -> BounceStats {
    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::Fixed { dt })
        .with_start_room("portal_lab");
    let mut sim = SandboxSim::new_with_options(opts).expect("SandboxSim::new in portal_lab");

    let spawn = sim.observation().player_pos;

    // Phase 1: walk right onto the first floor portal center, then STOP.
    let mut prev = spawn;
    for _ in 0..480 {
        let obs = sim.step(AgentAction {
            move_x: 1.0,
            ..base()
        });
        let cur = obs.player_pos;
        let jump = ((cur.0 - prev.0).powi(2) + (cur.1 - prev.1).powi(2)).sqrt();
        prev = cur;
        if cur.0 >= 285.0 || jump > 150.0 {
            break;
        }
    }

    let resets_after_entry = sim.observation().resets;

    // Phase 2: release ALL input and let the body bounce between the floor portals.
    let mut died_at = None;
    let mut grounded_frames = 0usize;
    for frame in 0..240 {
        let obs = sim.step(base());
        if obs.on_ground {
            grounded_frames += 1;
        }
        if obs.resets != resets_after_entry {
            died_at = Some(frame);
            break;
        }
    }
    BounceStats {
        died_at,
        grounded_frames,
    }
}

/// The fall-through guard: across a range of frame rates (the real game runs a
/// variable wall-clock dt), a no-input floor-portal bounce must never die.
#[test]
fn floor_portal_bounce_never_falls_through() {
    for hz in [60.0_f32, 45.0, 30.0, 24.0, 20.0, 15.0] {
        let stats = run_bounce(1.0 / hz);
        assert_eq!(
            stats.died_at, None,
            "FELL THROUGH / DIED while bouncing between the two floor portals with \
             no input at {hz} Hz (frame {:?}). The body should ping-pong forever.",
            stats.died_at,
        );
    }
}

/// The VARIABLE-frame-rate guard (the real game runs a jittery wall-clock dt —
/// Jon measured 10.5ms..50ms swings). Under a large-dt frame a body can sink past
/// the thin capture box in one step and, during the post-transit cooldown, ground
/// at the bottom of the open carve — "stuck in the middle of the floor", momentum
/// killed. A healthy bounce keeps TRANSITING (ping-ponging between the two floor
/// portals); a stuck body stops crossing. This drives a deterministic dt jitter
/// and asserts the body keeps transiting (never embeds) for the whole run.
#[test]
fn floor_portal_bounce_survives_variable_frame_rate() {
    // Deterministic jitter pattern spanning ~13ms..50ms (the measured range),
    // including back-to-back big frames that maximize the sink-per-step.
    let dts_ms = [
        16.0_f32, 50.0, 13.0, 33.0, 16.0, 45.0, 11.0, 40.0, 25.0, 50.0,
    ];

    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::Fixed { dt: 1.0 / 60.0 })
        .with_start_room("portal_lab");
    let mut sim = SandboxSim::new_with_options(opts).expect("SandboxSim::new in portal_lab");

    let spawn = sim.observation().player_pos;

    // Walk onto the first floor portal center, then stop.
    let mut prev = spawn;
    for _ in 0..480 {
        let obs = sim.step(AgentAction {
            move_x: 1.0,
            ..base()
        });
        let cur = obs.player_pos;
        let jump = ((cur.0 - prev.0).powi(2) + (cur.1 - prev.1).powi(2)).sqrt();
        prev = cur;
        if cur.0 >= 285.0 || jump > 150.0 {
            break;
        }
    }

    let resets_after_entry = sim.observation().resets;

    // Bounce with JITTERED dt and count transits (single-frame x jumps > 150px —
    // only a portal transfer moves the player that far). A stuck/embedded body
    // stops transiting; a healthy bounce keeps crossing.
    let mut prev_x = sim.observation().player_pos.0;
    let mut transits = 0usize;
    let mut longest_stall = 0usize;
    let mut stall = 0usize;
    for frame in 0..600 {
        let dt = dts_ms[frame % dts_ms.len()] / 1000.0;
        sim.set_timestep(TimestepMode::Fixed { dt });
        let obs = sim.step(base());
        let jump = (obs.player_pos.0 - prev_x).abs();
        prev_x = obs.player_pos.0;
        if jump > 150.0 {
            transits += 1;
            stall = 0;
        } else {
            stall += 1;
            longest_stall = longest_stall.max(stall);
        }
        assert_eq!(
            obs.resets, resets_after_entry,
            "died (fell through) under variable dt at frame {frame}",
        );
    }
    eprintln!("transits={transits} longest_stall={longest_stall}");

    // A healthy floor↔floor bounce keeps ping-ponging over 600 jittered frames.
    // The discriminating signal is `longest_stall`: with the bug the body embeds
    // and stops transiting for a long stretch (measured ~300 frames against the
    // pre-rescue carve fix); once it can never get stuck the max stall is just the
    // airtime between bounces at the slow jittered rate (~35 frames).
    assert!(
        transits >= 15,
        "the body should keep ping-ponging between the floor portals under variable \
         dt, but only transited {transits} times in 600 frames — it is getting \
         stuck/embedded (momentum killed).",
    );
    assert!(
        longest_stall < 100,
        "the body stalled for {longest_stall} consecutive frames without transiting \
         under variable dt — it embedded in the floor instead of bouncing (a healthy \
         bounce's longest gap is the airtime between crossings, ~35 frames).",
    );
}

/// The PER-TRANSIT momentum-conservation guard under jittered dt — the invariant
/// the earlier `survives_variable_frame_rate` test misses: a momentum-KILLED
/// bounce still transits (weakly, from rest), so transit/stall counts stay green
/// while energy is quietly stolen. This asserts each crossing's exit speed is
/// commensurate with its entry speed.
///
/// The bug this pins (Jon, 2026-06-10 "still bleeding momentum"): the carve sweep
/// read a STALE `SimDt` (PortalSet::Carves runs before CoreSimulation, where
/// `mirror_sim_dt_into_runtime` writes it) and used PRE-gravity velocity, so on a
/// dt spike the sweep undershot the body's real motion, the floor stayed solid
/// for one frame, and the integrator grounded the body — entry speed gone. The
/// dt-independent approach-box carve removes the whole class.
#[test]
fn floor_portal_bounce_conserves_momentum_per_transit_under_variable_dt() {
    use ambition_gameplay_core::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
    use bevy::prelude::*;

    const SMALL_DT: f32 = 0.008; // ~120 FPS baseline
    const SPIKE_DT: f32 = 0.050; // Jon's measured worst frame (~20 FPS hitch)
    const DROPS: usize = 8;

    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::Fixed { dt: SMALL_DT })
        .with_start_room("portal_lab");
    let mut sim = SandboxSim::new_with_options(opts).expect("SandboxSim::new in portal_lab");
    sim.step(base());

    // Drops happen onto the first live floor-to-floor authored pair.
    let (face_x, face_y) = {
        let (p, _) = first_floor_authored_portal_pair(&mut sim);
        (p.pos.x, p.pos.y)
    };

    // Teleport the primary player to `pos` with zero velocity.
    let place_player = |sim: &mut SandboxSim, pos: Vec2| {
        let mut q = sim
            .world_mut()
            .query_filtered::<&mut BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>();
        let mut kin = q.single_mut(sim.world_mut()).expect("primary player");
        kin.pos = pos;
        kin.vel = Vec2::ZERO;
    };

    // Each drop: fall from 500px above the portal (reaching ~terminal 950 px/s),
    // inject ONE SPIKE_DT frame in the strike zone just above the face — the
    // real-world condition: a frame hitch at the instant of re-entry, which the
    // carve cannot know the dt of in advance. The transfer preserves speed, so a
    // healthy crossing exits at ~entry speed; a momentum-killed one (floor stayed
    // solid on the spike frame, body grounded + re-sank from rest) exits at a
    // fraction of it.
    let mut ratios: Vec<f32> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    for drop in 0..DROPS {
        place_player(&mut sim, Vec2::new(face_x, face_y - 500.0));
        let mut entry_speed = 0.0_f32;
        let mut spiked = false;
        let mut transited = false;
        let mut prev_x = face_x;
        let mut exit_peak = 0.0_f32;
        let mut frames_after_transit = 0usize;
        for _frame in 0..600 {
            let obs_now = sim.observation();
            let above_face = face_y - obs_now.player_pos.1;
            // The miss window: the spike frame must carry the body from OUTSIDE
            // the stale-sweep carve-open range (bottom > ~22.6px above the face)
            // to BELOW the floor top in one step. The sim clamps a 50ms request
            // to ~33.3ms (≈31.7px at terminal 950), and the player half-height
            // is ~22px, so the center band is ~(45, 53)px above the face.
            let dt = if !spiked
                && !transited
                && obs_now.player_vel.1 > 700.0
                && (45.0..53.0).contains(&above_face)
            {
                spiked = true;
                entry_speed = obs_now.player_vel.1;
                SPIKE_DT
            } else {
                SMALL_DT
            };
            sim.set_timestep(TimestepMode::Fixed { dt });
            let obs = sim.step(base());
            if !transited && (obs.player_pos.0 - prev_x).abs() > 150.0 {
                transited = true;
            }
            prev_x = obs.player_pos.0;
            if transited {
                // Exit speed: peak upward speed in the frames after the crossing.
                exit_peak = exit_peak.max(-obs.player_vel.1);
                frames_after_transit += 1;
                if frames_after_transit > 40 {
                    break;
                }
            }
        }
        if !spiked {
            failures.push(format!("drop {drop}: spike never triggered (harness bug)"));
            continue;
        }
        if !transited {
            failures.push(format!(
                "drop {drop}: never transited after the dt spike (entry {entry_speed:.0} px/s) — \
                 grounded on a still-solid portal floor"
            ));
            continue;
        }
        ratios.push(exit_peak / entry_speed);
    }
    eprintln!("ratios={ratios:?} failures={failures:?}");

    assert!(
        failures.is_empty(),
        "momentum-kill drops under a dt spike at re-entry: {failures:?} (ratios so \
         far: {ratios:?})",
    );
    let violations: Vec<(usize, f32)> = ratios
        .iter()
        .enumerate()
        .filter(|(_, r)| **r < 0.7)
        .map(|(i, r)| (i, *r))
        .collect();
    assert!(
        violations.is_empty(),
        "momentum bled through portal transits under a dt spike at re-entry: drops \
         with exit speed < 0.7x entry speed: {violations:?} (all ratios: {ratios:?}). \
         The carve failed to open on the spike frame and the still-solid floor \
         grounded the body.",
    );
}

/// The momentum / landing-thud guard: the body must SINK THROUGH the open portal
/// floor, not repeatedly land on a still-solid floor. With the carve lagging a
/// frame the player grounded ~10+ frames per 240 (one thud per bounce, killing
/// its momentum); once the carve opens the same frame, grounding is near zero.
#[test]
fn floor_portal_bounce_does_not_thud_onto_the_open_floor() {
    let stats = run_bounce(1.0 / 60.0);
    assert!(
        stats.grounded_frames <= 4,
        "player grounded on the portal floor {} frames out of 240 while bouncing — \
         it should sink straight through the open aperture, not thud onto a \
         still-solid floor (the one-frame carve lag is back: momentum is being \
         killed on every entry and the landing sfx fires).",
        stats.grounded_frames,
    );
}
