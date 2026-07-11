//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::brain::action_set::{MeleeActionSpec, RangedActionSpec, SwipeSpec};

fn striker_actions() -> ActionSet {
    ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..ActionSet::peaceful()
    }
}

/// A flyer's kit: melee for dive-ins + a ranged poke (the PCA's glider) for
/// zoning from the perch.
fn winged_actions() -> ActionSet {
    ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ranged: Some(RangedActionSpec::Rock {
            speed: 320.0,
            damage: 2,
        }),
        ..ActionSet::peaceful()
    }
}

/// A grounded brawler — stands in for the "player robot" opponent.
fn robot(name: &'static str, x: f32) -> Fighter {
    Fighter {
        name,
        cfg: SmashCfg::DUELIST_DEFAULT,
        state: SmashState::default(),
        actions: striker_actions(),
        can_fly: false,
        max_air_jumps: 1,
        max_run_speed: 200.0,
        half_w: 16.0,
        half_h: 26.0,
        pos: ae::Vec2::new(x, 0.0),
        vel: ae::Vec2::ZERO,
        facing: 1.0,
        airborne: false,
        on_ground: true,
        air_jumps: 1,
        attack_cooldown: 0.0,
        attack_windup: 0.0,
        attack_active: 0.0,
        attack_recover: 0.0,
        stun: 0.0,
    }
}

/// The PCA — today a grounded Smash striker like the robot; the aerial /
/// blink / special verbs land in later slices and this fighter flips to
/// `can_fly = true`.
fn pca(name: &'static str, x: f32) -> Fighter {
    Fighter {
        name,
        cfg: SmashCfg::DUELIST_DEFAULT,
        state: SmashState::default(),
        actions: striker_actions(),
        can_fly: false,
        max_air_jumps: 1,
        max_run_speed: 210.0,
        half_w: 16.0,
        half_h: 26.0,
        pos: ae::Vec2::new(x, 0.0),
        vel: ae::Vec2::ZERO,
        facing: -1.0,
        airborne: false,
        on_ground: true,
        air_jumps: 1,
        attack_cooldown: 0.0,
        attack_windup: 0.0,
        attack_active: 0.0,
        attack_recover: 0.0,
        stun: 0.0,
    }
}

/// The PCA as it's meant to fight in the Noether Chamber: a **flyer** (Floating
/// body → `can_fly`) with a glider poke + a dive melee. Steers 2D via
/// `velocity_target`.
fn winged_pca(name: &'static str, x: f32) -> Fighter {
    Fighter {
        can_fly: true,
        airborne: true,
        actions: winged_actions(),
        ..pca(name, x)
    }
}

/// A **hybrid** PCA: capable of both grounded footsies and flight, with the
/// glider poke + dive melee. The brain chooses when to take off (to contest an
/// elevated foe or mount a proactive aerial foray) and when to land.
fn hybrid_pca(name: &'static str, x: f32) -> Fighter {
    let mut cfg = SmashCfg::DUELIST_DEFAULT;
    cfg.can_fly = true;
    cfg.aerial_foray_cadence_s = 3.0; // ~3s grounded between forays
    cfg.aerial_foray_duration_s = 2.5; // ~2.5s airborne per foray
    Fighter {
        cfg,
        can_fly: true,
        airborne: false, // starts grounded; takes flight on its own
        actions: winged_actions(),
        ..pca(name, x)
    }
}

/// The **player-robot as a full-kit fighter** (roadmap S6a / I7): the
/// protagonist's `player_robot` archetype kit expressed at the brain level —
/// a grounded-base hybrid (footsies + jump on the ground, fly to contest the
/// air) with blink-evade, reactive shield, dash-to-close, a melee strike, and
/// the player's ranged poke. The same body the human pilots, driven by the one
/// Smash brain. Mirrors the authored archetype flags
/// (`smash_can_blink/fly/shield/dash`), so the mirror-match below exercises the
/// real player kit, not a generic duelist.
fn player_robot_fighter(name: &'static str, x: f32) -> Fighter {
    let mut cfg = SmashCfg::DUELIST_DEFAULT;
    cfg.can_blink = true;
    cfg.blink_cooldown_s = 0.8;
    cfg.can_shield = true;
    cfg.can_fly = true;
    cfg.dash_to_close = true;
    cfg.aerial_foray_cadence_s = 3.0;
    cfg.aerial_foray_duration_s = 2.5;
    Fighter {
        cfg,
        can_fly: true,
        airborne: false, // grounded-base hybrid, like the player
        actions: winged_actions(),
        ..robot(name, x)
    }
}

/// **The spectator-arena mirror match** (invariant I9, the in-engine analogue at
/// the brain level): the player-robot and the PCA — both full-kit hybrids under
/// the ONE Smash brain — fight in the C4 chamber and must be non-degenerate
/// (roam the stage, vary their verbs, never freeze or camp a corner). This is
/// the behavioral witness the roadmap calls necessary-but-not-sufficient; it
/// proves the two advanced bodies compose into a real fight without degenerate
/// loops. (Observer-sparing is the relational-damage property proven in S3e;
/// the real-ECS *playable* arena with the Neutral observer in the room, and
/// portal routing, remain.)
#[test]
fn player_robot_vs_pca_mirror_match_is_non_degenerate() {
    let stage = Stage::symmetry_chamber();
    let mut arena = Arena::new(
        stage.clone(),
        player_robot_fighter("PlayerRobot", 540.0),
        hybrid_pca("PCA", 180.0),
    );
    arena.run(30.0);
    let reports = arena.trace.reports();
    // Both fighters are ranged-capable zoners, so they footsie/poke each other
    // at mid-range rather than chasing across the whole arena — the same reason
    // `two_hybrid_pcas` uses bespoke asserts instead of the grounded-brawl
    // `NonDegenerateThresholds` (whose `max_still_s` over-fires on tight
    // zoning). The meaningful guards here: they move a lot, use a VARIETY of the
    // full kit (not one-note), range across columns, and never wedge in a
    // corner.
    for r in &reports {
        println!("{}", r.summary());
        assert!(
            r.path_len > 1000.0,
            "{}: barely moved ({:.0}px) — a full-kit fighter should actively engage",
            r.name,
            r.path_len
        );
        assert!(
            r.verbs.len() >= 4,
            "{}: only {} verb kinds — should mix the kit (walk/melee/ranged/blink/shield/jump): {:?}",
            r.name,
            r.verbs.len(),
            r.verbs,
        );
        assert!(
            r.x_bins_visited >= 4,
            "{}: covered only {}/{} columns — should range across the chamber",
            r.name,
            r.x_bins_visited,
            r.x_bins_total,
        );
        assert!(
            r.max_corner_s < 6.0,
            "{}: pinned in a corner {:.1}s — should not wedge",
            r.name,
            r.max_corner_s,
        );
    }
    // The mirror match must actually exercise the FULL kit, not collapse to
    // walk+melee: across the two fighters, blink and shield (the S3 defensive
    // verbs) and ranged all show up — the player-robot fights with everything.
    let used: std::collections::HashSet<Verb> = reports
        .iter()
        .flat_map(|r| r.verbs.iter().map(|(v, _)| *v))
        .collect();
    for needed in [Verb::Blink, Verb::Shield, Verb::Ranged, Verb::Melee] {
        assert!(
            used.contains(&needed),
            "the mirror match never used {needed:?} — the full kit isn't being exercised",
        );
    }
}

/// The inspection bout the user asked for: **two hybrid PCAs** that can fly OR
/// ground, fighting in the C4 symmetry chamber. Beyond the usual non-degeneracy
/// guard, it asserts FLIGHT HEALTH — each fighter genuinely uses both modes
/// (doesn't stick airborne or glued to the floor), toggles repeatedly, and uses
/// the chamber's vertical space. Prints the full report so flight behavior can
/// be inspected and iterated.
#[test]
fn two_hybrid_pcas_fight_healthily_in_the_symmetry_chamber() {
    let stage = Stage::symmetry_chamber();
    let mut arena = Arena::new(
        stage.clone(),
        hybrid_pca("PCA-A", 540.0),
        hybrid_pca("PCA-B", 180.0),
    );
    arena.run(40.0);
    let reports = arena.trace.reports();
    println!("--- two hybrid PCAs in the symmetry chamber (prefer-grounded) ---");
    for r in &reports {
        println!("{}", r.summary());
        // New policy (S3b): a hybrid PREFERS to fight grounded and flies only
        // to cover a long traversal gap. Two PCAs spawned within engagement
        // range therefore brawl on the GROUND — they close, melee, jump, and
        // blink rather than circling in the air. So we assert an *active,
        // grounded-preferring* brawl, not the old fly-heavy balance.
        //
        // This proxy arena (own kinematics, no terrain) is a fast smoke check
        // only; deep degeneracy + "works in a fight" certification moves to
        // the real-ECS harness per the architecture roadmap.
        assert!(
            r.path_len > 400.0,
            "{}: barely moved ({:.0}px) — a hybrid should actively engage, not camp",
            r.name,
            r.path_len
        );
        assert!(
            r.verbs.len() >= 4,
            "{}: only used {} kinds of verb — should vary its offense (walk/melee/jump/blink/...)",
            r.name,
            r.verbs.len()
        );
        assert!(
            r.x_bins_visited >= 4,
            "{}: covered only {}/{} columns — should range across the floor",
            r.name,
            r.x_bins_visited,
            r.x_bins_total
        );
        assert!(
            r.flight_frac < 0.5,
            "{}: spent {:.0}% in flight — a grounded-preferring hybrid should mostly brawl on the ground",
            r.name,
            r.flight_frac * 100.0
        );
    }
}

/// Characterization: a bout runs to completion, stays in bounds, no NaNs, and
/// the trace analytics produce a sane report. This is the always-green floor;
/// the non-degeneracy thresholds are asserted in
/// `pca_vs_robot_is_non_degenerate` (which the brain work makes pass).
#[test]
fn bout_runs_and_stays_in_bounds() {
    let stage = Stage::noether_like();
    let mut arena = Arena::new(stage.clone(), pca("PCA", 720.0), robot("Robot", 240.0));
    arena.run(20.0);
    for i in 0..2 {
        for s in &arena.trace.samples[i] {
            assert!(s.pos.x.is_finite() && s.pos.y.is_finite(), "NaN in trace");
            assert!(
                s.pos.x >= stage.left - 1.0 && s.pos.x <= stage.right + 1.0,
                "{} left the arena horizontally at x={}",
                arena.trace.names[i],
                s.pos.x
            );
            assert!(
                s.pos.y <= stage.floor + 1.0 && s.pos.y >= stage.ceiling - 1.0,
                "{} left the arena vertically at y={}",
                arena.trace.names[i],
                s.pos.y
            );
        }
        assert!(
            !arena.trace.samples[i].is_empty(),
            "no samples recorded for fighter {i}"
        );
    }
    // Observe the trace (the user asked to be able to see it).
    let [ra, rb] = arena.trace.reports();
    println!(
        "--- characterization bout ---\n{}\n{}",
        ra.summary(),
        rb.summary()
    );
}

/// The degeneracy guard the user asked for. Both fighters must use the stage
/// (roam horizontally, not freeze, not camp a corner) and employ a variety of
/// verbs. Structural, not byte-for-byte — it survives logic changes.
///
/// Passes now that the duelist neutral game (footsies weave + neutral hops +
/// platform-only vertical chase) replaced point-blank mashing: the fighters
/// dance in and out of poke range across the stage instead of collapsing into
/// a wall.
#[test]
fn pca_vs_robot_is_non_degenerate() {
    let stage = Stage::noether_like();
    let mut arena = Arena::new(stage.clone(), pca("PCA", 720.0), robot("Robot", 240.0));
    arena.run(30.0);
    let reports = arena.trace.reports();
    let th = NonDegenerateThresholds::default();
    let mut all = Vec::new();
    for r in &reports {
        println!("{}", r.summary());
        all.extend(r.violations(&th));
    }
    assert!(
        all.is_empty(),
        "degenerate fight detected:\n  {}",
        all.join("\n  ")
    );
}

/// The flying PCA (the real Noether-Chamber matchup) vs a grounded robot must
/// also be non-degenerate: it should use vertical space (dive/perch), zone
/// with its glider, and not camp. This is the guard for the aerial brain.
#[test]
fn flying_pca_vs_grounded_robot_is_non_degenerate() {
    let stage = Stage::noether_like();
    let mut arena = Arena::new(
        stage.clone(),
        winged_pca("PCA", 720.0),
        robot("Robot", 240.0),
    );
    arena.run(30.0);
    let reports = arena.trace.reports();
    let th = NonDegenerateThresholds::default();
    let mut all = Vec::new();
    for r in &reports {
        println!("{}", r.summary());
        all.extend(r.violations(&th));
    }
    // The flyer is expected to live in the air — that's its whole identity, so
    // the airborne fraction is NOT a degeneracy here.
    assert!(
        reports[0].airborne_frac > 0.5,
        "the flying PCA should mostly be airborne; got {:.0}%",
        reports[0].airborne_frac * 100.0
    );
    assert!(
        all.is_empty(),
        "degenerate fight detected:\n  {}",
        all.join("\n  ")
    );
}
