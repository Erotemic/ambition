//! Unit tests for the time-control authority: `ProperTimeScale` defaults and
//! the `ClockScaleRequest` → policy dispatch in the `Solo` regime.

use super::*;
use crate::player::components::PlayerSlot;
use ambition_time::ProperTimeScale;

#[test]
fn proper_time_scale_default_is_one() {
    let pts = ProperTimeScale::default();
    assert_eq!(pts, ProperTimeScale::ONE);
    assert_eq!(pts.value(), 1.0);
}

#[test]
fn proper_time_scale_or_default_falls_back_to_one() {
    let some = ProperTimeScale(2.5);
    assert_eq!(ProperTimeScale::or_default(Some(&some)).value(), 2.5);
    assert_eq!(ProperTimeScale::or_default(None), ProperTimeScale::ONE);
}

#[test]
fn solo_regime_grants_every_requester_every_domain() {
    let policy = RegimePolicy {
        regime: Regime::Solo,
    };
    for requester in [
        ClockRequester::Player(PlayerSlot::PRIMARY),
        ClockRequester::DevTool,
        ClockRequester::Scripted,
        ClockRequester::Engine,
        ClockRequester::Boss,
    ] {
        for domain in [
            ClockDomain::SimClock,
            ClockDomain::PlayerClock(ambition_time::ClockObserver::PRIMARY),
            ClockDomain::WallClock,
        ] {
            assert_eq!(
                policy.permission_for(requester, domain),
                Permission::Grant,
                "Solo must grant {:?} -> {:?}",
                requester,
                domain,
            );
        }
    }
}

#[test]
fn rl_deterministic_denies_every_request() {
    let policy = RegimePolicy {
        regime: Regime::RLDeterministic,
    };
    assert_eq!(
        policy.permission_for(
            ClockRequester::Player(PlayerSlot::PRIMARY),
            ClockDomain::SimClock,
        ),
        Permission::Deny,
    );
    assert_eq!(
        policy.permission_for(ClockRequester::Scripted, ClockDomain::SimClock),
        Permission::Deny,
    );
}

#[test]
fn cinematic_grants_scripted_denies_player() {
    let policy = RegimePolicy {
        regime: Regime::Cinematic,
    };
    assert_eq!(
        policy.permission_for(ClockRequester::Scripted, ClockDomain::SimClock),
        Permission::Grant,
    );
    assert_eq!(
        policy.permission_for(
            ClockRequester::Player(PlayerSlot::PRIMARY),
            ClockDomain::SimClock,
        ),
        Permission::Deny,
    );
    // Engine retains authority during cinematics so pause /
    // suspended-gameplay still works.
    assert_eq!(
        policy.permission_for(ClockRequester::Engine, ClockDomain::SimClock),
        Permission::Grant,
    );
}

/// End-to-end: build a minimal app, fire a request, run the
/// system, observe the target change. This is the canonical
/// regression check that the dispatch pipeline is wired.
/// SandboxSimState::time_scale is touched by the SMOOTHER
/// (smooth_sim_clock_toward_target_system), not by this system.
#[test]
fn solo_grant_writes_requested_clock_scale() {
    let mut app = App::new();
    app.add_message::<ClockScaleRequest>()
        .insert_resource(RegimePolicy::default())
        .insert_resource(RequestedClockScale::default())
        .add_systems(Update, apply_clock_scale_requests);

    app.world_mut().write_message(ClockScaleRequest {
        domain: ClockDomain::SimClock,
        scale: 0.125,
        requester: ClockRequester::Player(PlayerSlot::PRIMARY),
        reason: "bullet_blink_test",
    });

    app.update();

    let target = app.world().resource::<RequestedClockScale>();
    assert!((target.sim_clock - 0.125).abs() < 1e-6);
}

#[test]
fn rl_regime_denies_blocks_the_scale_change() {
    let mut app = App::new();
    app.add_message::<ClockScaleRequest>()
        .insert_resource(RegimePolicy {
            regime: Regime::RLDeterministic,
        })
        .insert_resource(RequestedClockScale::default())
        .add_systems(Update, apply_clock_scale_requests);

    app.world_mut().write_message(ClockScaleRequest {
        domain: ClockDomain::SimClock,
        scale: 0.125,
        requester: ClockRequester::Player(PlayerSlot::PRIMARY),
        reason: "denied_test",
    });

    app.update();

    let target = app.world().resource::<RequestedClockScale>();
    assert!(
        (target.sim_clock - 1.0).abs() < 1e-6,
        "RL must keep sim-clock target at 1.0",
    );
}

/// Wall clock is by definition unscaled. A grant targeting it is
/// a no-op so the host's real-time tick keeps advancing.
#[test]
fn wall_clock_grant_does_not_mutate_target() {
    let mut app = App::new();
    app.add_message::<ClockScaleRequest>()
        .insert_resource(RegimePolicy::default())
        .insert_resource(RequestedClockScale::default())
        .add_systems(Update, apply_clock_scale_requests);

    app.world_mut().write_message(ClockScaleRequest {
        domain: ClockDomain::WallClock,
        scale: 0.25,
        requester: ClockRequester::DevTool,
        reason: "wall_noop_test",
    });

    app.update();

    let target = app.world().resource::<RequestedClockScale>();
    assert!((target.sim_clock - 1.0).abs() < 1e-6);
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
        .insert_resource(SandboxFeelTuning::default())
        .insert_resource(Time::<()>::default())
        .add_systems(
            Update,
            (
                apply_clock_scale_requests,
                smooth_sim_clock_toward_target_system,
            )
                .chain(),
        );

    // Pump a fixed 16ms tick into Bevy's Time so the smoother
    // sees a non-zero frame_dt.
    let frame = std::time::Duration::from_millis(16);
    for _ in 0..30 {
        app.world_mut().write_message(ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 0.125,
            requester: ClockRequester::Player(PlayerSlot::PRIMARY),
            reason: "ramp_test",
        });
        app.world_mut().resource_mut::<Time>().advance_by(frame);
        app.update();
    }

    let clock = app.world().resource::<ClockState>();
    // SandboxFeelTuning::time_ramp_down_rate is 6.0 (units/s).
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
    // Path is relative to `crates/ambition_gameplay_core/src/`.
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
        // Cutscenes intentionally suspend bullet-time and run on the
        // wall clock so banner/fade beats advance independent of any
        // sim time-scale. The playback system moved here from
        // `ambition_render::cutscene` (runtime extraction, 2026-06-25).
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
        // `attack_advance_system` was drained out of `app/world_flow/attack.rs`
        // into the combat runtime (commit b30cfe7f); it still wraps its own
        // sandbox_dt for the attack phase machine. Same ADR 0011 follow-up.
        (
            "combat/attack.rs",
            "attack advance wraps its own sandbox_dt() — ADR 0011 follow-up",
        ),
        (
            "app/input_systems.rs",
            "input buffer decay; ADR 0011 player-clock follow-up",
        ),
        // Home/player body reaction + gesture + presentation-flash timers folded
        // down from `ambition_app::app::sim_systems` (C4). They decay on the frame
        // clock (presentation flash runs even while paused, by design; the reaction
        // timers still compute their own scaled dt manually) — same ADR 0011
        // player-clock follow-up as the app-tick path they moved out of.
        (
            "player/input_systems.rs",
            "home-body reaction/gesture/flash timers moved from the app (C4); ADR 0011 follow-up",
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

/// Integration check: with `WorldTime::scaled_dt` at 0.25, a
/// timer driven by `sim_dt()` advances at exactly 0.25× the
/// wall-clock dt. This pins the contract that bullet-time
/// downstream consumers actually slow down — the exact bug the
/// `Res<Time>` -> `Res<WorldTime>` refactor was meant to fix.
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
    use crate::game_mode::{gameplay_suspended, GameMode};
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
    use crate::game_mode::{gameplay_suspended, GameMode};
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
