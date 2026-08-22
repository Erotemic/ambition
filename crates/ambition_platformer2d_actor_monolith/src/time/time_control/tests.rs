//! Unit tests for the time-control authority: `ProperTimeScale` defaults and
//! the `ClockScaleRequest` → policy dispatch in the `Solo` regime.

use super::*;
use ambition_time::time_control::{
    apply_clock_scale_requests, apply_suspended_time_scale_system, RegimePolicy,
};
use ambition_time::ClockObserver;

/// The reaction-timer clock forks on purpose, and each side is pinned.
///
/// i-frames are a promise in REAL seconds — a bullet-time moment must not
/// hand out longer invulnerability — the same reason the double-tap gesture
/// windows are unscaled.
///
///  so this pins BOTH sides, because a fork with only one side guarded drifts
/// back. It finds its subjects by walking the crate rather than listing them.
#[test]
fn the_reaction_timer_clock_forks_on_purpose() {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&src, &mut files);

    // The one site that decays on the UNSCALED clock, and why.
    const CONTROLLED: &str = "control/input_systems.rs";

    let mut scaled = Vec::new();
    let mut unscaled = Vec::new();
    for path in &files {
        let rel = path
            .strip_prefix(&src)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or("<non-utf8>")
            .to_string();
        if rel.ends_with("tests.rs") || rel.contains("/tests/") {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(path) else {
            continue;
        };
        for (lineno, line) in contents.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("*") {
                continue;
            }
            let Some((_, arg)) = line.split_once(".decay_reaction_timers(") else {
                continue;
            };
            // A bare `dt` counts as the sim clock only if the file BINDS it from
            // there — accepting it on its name would let `let dt =
            // time.delta_secs()` satisfy this while reintroducing the fork.
            let sim = arg.contains("sim_dt")
                || (arg.starts_with("dt)") && contents.contains("let dt = world_time.sim_dt()"));
            let site = format!("{rel}:{}", lineno + 1);
            if sim {
                scaled.push(site);
            } else {
                unscaled.push(site);
            }
        }
    }

    assert!(
        scaled.len() >= 2,
        "expected the actor and boss decays to use the sim clock; found {scaled:?} —          the scan is broken, not the code"
    );
    assert!(
        unscaled.iter().all(|s| s.starts_with(CONTROLLED)),
        "a body outside the controlled road decays reaction timers on an UNSCALED \
         clock, so it will not slow with bullet-time: {unscaled:?}"
    );
    assert!(
        unscaled.iter().any(|s| s.starts_with(CONTROLLED)),
        "the CONTROLLED body's reaction timers moved onto the sim clock. That looks \
         like a consolidation and is not: hitstop is a sim_clock requester, so \
         `hitstop_timer` would then be slowed by the very freeze it ends, and the \
         i-frame window would stretch with it. Seven boss tests fail on this. See \
         the note beside the call in {CONTROLLED}."
    );
}

/// End-to-end: a SimClock grant + the smoother together actually
/// move time_scale. After ~10 frames at default ramp rates the
/// time_scale should be well below 1.0 (heading toward 0.125).
#[test]
fn smoother_ramps_sim_state_time_scale_toward_target() {
    let mut app = App::new();
    app.add_message::<ClockScaleRequest>()
        .insert_resource(RegimePolicy::default())
        .insert_resource(RequestedClockScale::default())
        .insert_resource(ClockState::default())
        .insert_resource(Platformer2dFeelTuningMonolith::default())
        .insert_resource(Time::<()>::default())
        .add_systems(
            Update,
            (
                apply_clock_scale_requests,
                smooth_sim_clock_toward_target_system,
            )
                .chain(),
        );

    let frame = std::time::Duration::from_millis(16);
    for _ in 0..30 {
        app.world_mut().write_message(ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 0.125,
            requester: ClockRequester::Player(ClockObserver::PRIMARY),
            reason: "ramp_test",
        });
        app.world_mut().resource_mut::<Time>().advance_by(frame);
        app.update();
    }

    let clock = app.world().resource::<ClockState>();
    // Platformer2dFeelTuningMonolith::time_ramp_down_rate is 6.0 (units/s).
    // 30 frames * 16ms = 480ms => deltas of ~2.88 units, way
    // past the (1.0 -> 0.125) gap of 0.875.
    assert!(
        (clock.time_scale - 0.125).abs() < 1e-4,
        "expected sim time_scale ~= 0.125 after ramp; got {}",
        clock.time_scale,
    );
}

/// Static guardrail: gameplay-tick files must NOT read
/// `Res<Time>` directly. Reading `Res<Time>` returns wall-clock
/// dt and silently bypasses bullet-time / pause / hitstop. The
/// canonical pattern (ADR 0010, see also
/// `feedback_time_domains` memory) is to read `Res<WorldTime>`
/// and pick a domain explicitly via `sim_dt()` (gameplay timer),
/// `wall_dt()` (UI / audio bus), or `player_dt(slot)` (input
/// buffer).
///
/// This is encoded as a source-tree scan rather than a type-
/// level constraint because Bevy's prelude leaks `Time` into
/// every system signature and there is no language-level seam
/// short of forking Bevy. The scan keeps the discipline visible
/// and reviewable; the allowlist documents the legitimate
/// wall-clock consumers so the next reviewer can audit by name.
///
/// Tighten this allowlist over time as the player-clock /
/// entity-clock plumbing lands (ADR 0011). Do NOT add gameplay
/// systems to the allowlist — fix them.
#[test]
fn gameplay_systems_must_not_read_res_time_directly() {
    use std::path::{Path, PathBuf};

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("src");

    // Files that are explicitly allowed to read `Res<Time>`.
    // Path is relative to `crates/ambition_platformer2d_actor_monolith/src/`.
    //
    // Add a one-line justification next to each entry — "why
    // does this need raw wall-clock dt and not WorldTime?"
    let allowlist: &[(&str, &str)] = &[
        // Producer side: `refresh_world_time` (now in `time::world_time`)
        // is THE system that writes WorldTime from Bevy's Time.
        (
            "time/world_time.rs",
            "refresh_world_time itself converts Time -> WorldTime",
        ),
        // The time-control pipeline runs on real wall-clock to
        // smoothly ramp time_scale; ramping on its own output
        // would be circular.
        (
            "time/time_control/mod.rs",
            "smoother / clock-scale dispatch is the controller, not a consumer",
        ),
        ("cutscene.rs", "cutscene beats are wall-clock by design"),
        // VFX particles are presentation; the design decision
        // is wall-clock so juice survives bullet-time. Revisit
        // if you want VFX to slow alongside the sim.
        (
            "presentation/fx.rs",
            "VFX particles are wall-clock by design",
        ),
        // Screen-effect shader modulation (CRT scanline jitter,
        // chromatic-aberration breathing, etc.) is a fullscreen
        // visual driven by wall-clock elapsed seconds so the
        // post-process still animates in bullet-time / hitstop.
        (
            "presentation/screen_effects.rs",
            "screen-effect shader modulation is wall-clock by design",
        ),
        // Music director (track switching, fades) is wall-clock;
        // music should not slow in bullet-time.
        (
            "music/director/mod.rs",
            "music timing is wall-clock by design",
        ),
        // Dialogue typewriter reveal is presentation timing: the
        // visible substring advances on the wall clock so text
        // doesn't crawl during bullet-time / hitstop. Yarn still
        // owns the line/option state machine.
        (
            "dialog/systems.rs",
            "typewriter reveal is wall-clock presentation timing",
        ),
        // Camera smoothing is wall-clock so glide responsiveness
        // stays consistent. Switch to scaled if bullet-time camera
        // feel is desired.
        (
            "presentation/rendering/camera.rs",
            "camera smoothing is wall-clock by design",
        ),
        // Physics debris is cosmetic; it spawns from sim events
        // but its falling animation is independent of sim time.
        (
            "world/physics.rs",
            "debris fall is cosmetic / wall-clock by design",
        ),
        // Player-input timers + the player tick still compute
        // their own scaled dt via `sandbox_dt(hitstop,
        // time_scale, frame_dt)`. Migration target for ADR 0011
        // (PlayerClock); allowed for now.
        (
            "app/player_tick.rs",
            "player tick wraps its own sandbox_dt() — ADR 0011 follow-up",
        ),
        (
            "app/sim_systems.rs",
            "input timers + attack advance still compute scaled dt manually — ADR 0011 follow-up",
        ),
        // Same ADR 0011 follow-up.
        (
            "combat/attack.rs",
            "attack advance wraps its own sandbox_dt() — ADR 0011 follow-up",
        ),
        (
            "app/input_systems.rs",
            "input buffer decay; ADR 0011 player-clock follow-up",
        ),
        // THE OLD JUSTIFICATION WAS FALSE, AND CORRECTING IT NAIVELY COST
        // SEVEN BOSS TESTS. It read *"the reaction timers still compute their
        // own scaled dt manually"*, and the file contains no scaling — so the
        // obvious repair was to move the decay onto `world_time.sim_dt()` like
        // the actor and boss ticks. `boss_contact_iframes`, `boss_lifecycle`
        // and `boss_motion_parity` went red at once, and they were right:
        // hitstop is a `sim_clock` requester, so `hitstop_timer` would be
        // slowed by the freeze it exists to end, and the i-frame and hitstun
        // windows would stretch with it.
        //
        // That is the trap worth remembering: a false justification does not mean the decision
        // under it is false, and "consolidating" a fork nobody explained is how a deliberate
        // one gets undone.
        //
        // ⭐ what is on the raw clock here, and why: the presentation flash,
        // which is meant to run while paused. Pinned by
        // `the_reaction_timer_clock_forks_on_purpose`.
        //
        // ⚠ **NARROWED 2026-08-22.** This waiver also covered the reaction
        // timers and the double-tap windows, which are gameplay and are
        // rollback-canonical state. They still want the same NUMBER — unscaled
        // seconds — but that number has a name, so they read
        // `WorldTime::wall_dt()` and are no longer exceptions to anything. What
        // is left here is genuinely presentation.
        (
            "control/input_systems.rs",
            "the presentation flash is REAL-time by design and runs while paused; \
             the gameplay timers in this file moved to WorldTime::wall_dt",
        ),
        // Hot reload polls disk in wall-clock cadence.
        (
            "world/ldtk_world/hot_reload.rs",
            "filesystem watcher cadence is wall-clock",
        ),
        // Mobile-touch menu bridge: UI bridging.
        (
            "host/mobile_input/menu_bridge.rs",
            "touch menu bridge is wall-clock UI",
        ),
        // Trace recorder timestamps each frame on the wall clock.
        (
            "dev/trace/systems.rs",
            "trace timestamps are wall-clock by design",
        ),
    ];

    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    let mut files = Vec::new();
    walk(&src, &mut files);

    let mut violations: Vec<String> = Vec::new();
    for path in &files {
        let rel = path
            .strip_prefix(&src)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or("<non-utf8>");
        // Skip test files: they legitimately name `Res<Time>` in
        // assertions / allowlist literals (this guardrail among them), and a
        // test is never a gameplay system. Robust to dir-conversion splits
        // (a file's `mod tests` moving into a sibling `tests.rs`).
        if rel.ends_with("tests.rs") || rel.contains("/tests/") {
            continue;
        }
        if allowlist.iter().any(|(p, _)| *p == rel) {
            continue;
        }
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        // Strip block-string literals before scanning so doc-
        // examples and rustdoc snippets don't trip the test.
        // Keep it dumb-simple: grep for `Res<Time>` (the exact
        // sig fragment) — comments mentioning it are fine because
        // they wouldn't compile as system params.
        for (lineno, line) in contents.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("*") {
                continue;
            }
            if line.contains("Res<Time>") {
                violations.push(format!(
                        "{rel}:{}: contains `Res<Time>` — use Res<WorldTime> + sim_dt()/wall_dt()/player_dt(). \
                         If this genuinely needs wall-clock dt, add an entry + justification to the \
                         `allowlist` in this test.",
                        lineno + 1,
                    ));
            }
        }
    }

    // Useful echo: which files ARE on the allowlist, so reviewers
    // can spot-check the justifications.
    if !violations.is_empty() {
        let allow_summary: Vec<String> = allowlist
            .iter()
            .map(|(p, why)| format!("  {p}: {why}"))
            .collect();
        panic!(
                "gameplay systems must read Res<WorldTime>, not Res<Time>. Violations:\n{}\n\nCurrent allowlist:\n{}",
                violations.join("\n"),
                allow_summary.join("\n"),
            );
    }
}

/// Integration check: with `WorldTime::scaled_dt` at 0.25, a timer driven by `sim_dt()`
/// advances at exactly 0.25× the wall-clock dt.
#[test]
fn world_time_sim_dt_respects_time_scale() {
    use ambition_time::WorldTime;

    let mut wt = WorldTime::default();
    wt.raw_dt = 0.016;
    wt.scaled_dt = 0.016 * 0.25;
    assert!((wt.wall_dt() - 0.016).abs() < 1e-6);
    assert!((wt.sim_dt() - 0.004).abs() < 1e-6);

    // Pause behaviour: time_scale == 0 -> sim_dt == 0 even
    // though wall_dt keeps ticking.
    let mut paused = WorldTime::default();
    paused.raw_dt = 0.016;
    paused.scaled_dt = 0.0;
    assert_eq!(paused.sim_dt(), 0.0);
    assert!((paused.wall_dt() - 0.016).abs() < 1e-6);
}

/// Regression: when gameplay is suspended (pause / dialogue / cutscene / room
/// transition), `apply_suspended_time_scale_system` must zero both
/// `ClockState::time_scale` AND `RequestedClockScale::sim_clock` BEFORE
/// `refresh_world_time` snapshots them — otherwise `WorldTime::scaled_dt` stays
/// non-zero on the first suspended frame and any presentation system multiplying
/// by it ticks one extra frame after pause lands.
#[test]
fn suspended_frame_zeros_world_time_scaled_dt() {
    use ambition_platformer2d_shared_tangle::schedule::{gameplay_suspended, GameMode};
    use ambition_time::WorldTime;
    use bevy::state::app::StatesPlugin;

    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.insert_state(GameMode::Paused);
    app.insert_resource(ClockState { time_scale: 1.0 });
    app.insert_resource(RequestedClockScale {
        sim_clock: 1.0,
        ..Default::default()
    });
    app.insert_resource(WorldTime {
        raw_dt: 0.016,
        scaled_dt: 0.016,
    });
    app.insert_resource(Time::<()>::default());

    // Mirror the host ordering from `register_player_input_systems`:
    // suspended-zero FIRST, then refresh.
    app.add_systems(
        Update,
        (
            apply_suspended_time_scale_system.run_if(gameplay_suspended),
            ambition_time::refresh_world_time,
        )
            .chain(),
    );

    let frame = std::time::Duration::from_millis(16);
    app.world_mut().resource_mut::<Time>().advance_by(frame);
    app.update();

    let clock = app.world().resource::<ClockState>();
    let target = app.world().resource::<RequestedClockScale>();
    let wt = app.world().resource::<WorldTime>();
    assert_eq!(
        clock.time_scale, 0.0,
        "suspended frame must zero ClockState.time_scale"
    );
    assert_eq!(
        target.sim_clock, 0.0,
        "suspended frame must zero RequestedClockScale.sim_clock"
    );
    assert_eq!(
        wt.scaled_dt, 0.0,
        "suspended frame must zero WorldTime.scaled_dt (refresh_world_time must \
         see the zeroed time_scale, not last frame's 1.0)"
    );
    assert!(
        (wt.wall_dt() - 0.016).abs() < 1e-6,
        "wall clock must keep ticking through pause"
    );
}

/// Gameplay-allowed frames take the regular emit → apply → smooth path; the
/// suspended fallback is short-circuited by `run_if`. `refresh_world_time` then
/// sees `ClockState::time_scale = 1.0` (the default) and reports a non-zero
/// `scaled_dt`.
#[test]
fn gameplay_frame_preserves_world_time_scaled_dt() {
    use ambition_platformer2d_shared_tangle::schedule::{gameplay_suspended, GameMode};
    use ambition_time::WorldTime;
    use bevy::state::app::StatesPlugin;

    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.insert_state(GameMode::Playing);
    app.insert_resource(ClockState::default());
    app.insert_resource(RequestedClockScale::default());
    app.insert_resource(WorldTime::default());
    app.insert_resource(Time::<()>::default());

    app.add_systems(
        Update,
        (
            apply_suspended_time_scale_system.run_if(gameplay_suspended),
            ambition_time::refresh_world_time,
        )
            .chain(),
    );

    let frame = std::time::Duration::from_millis(16);
    app.world_mut().resource_mut::<Time>().advance_by(frame);
    app.update();

    let wt = app.world().resource::<WorldTime>();
    assert!(
        wt.scaled_dt > 0.0,
        "gameplay frame must produce a non-zero scaled_dt; got {}",
        wt.scaled_dt
    );
}
