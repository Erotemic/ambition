//! Source scans of this crate's clock discipline: gameplay systems read the
//! simulation clock (`WorldTime`), not Bevy's wall clock, and the reaction
//! timers fork on purpose. They stayed here when the time-control systems
//! moved to `ambition_platformer2d_runtime::sim_clock`: what they scan is this
//! crate's source.

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

    // ⛔ TWO ROOTS, because the BOSS half left this crate. The boss ECS tick moved
    // to `ambition_boss_encounter` on 2026-08-28, and a scan rooted at one crate
    // then found ONE site and reported *"the scan is broken"* — which it was. The
    // invariant did not move; the code did. ⚠ each site is reported relative to
    // its OWN root, so `CONTROLLED` below still matches on its own path.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("src");
    let mut files = Vec::new();
    walk(&src, &mut files);
    let boss_src = manifest
        .parent()
        .expect("the crate sits inside `crates/`")
        .join("ambition_boss_encounter")
        .join("src");
    let mut boss_files = Vec::new();
    walk(&boss_src, &mut boss_files);

    // The one site that decays on the UNSCALED clock, and why.
    const CONTROLLED: &str = "control/input_systems.rs";

    let mut scaled = Vec::new();
    let mut unscaled = Vec::new();
    for (path, root) in files
        .iter()
        .map(|p| (p, &src))
        .chain(boss_files.iter().map(|p| (p, &boss_src)))
    {
        let rel = path
            .strip_prefix(root)
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
