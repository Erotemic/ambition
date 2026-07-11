//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::actor::ActorFaction;
use crate::perception::{BodyPhase, PerceivedActor, SelfView, StageView, WorldView};
use ambition_engine_core as ae;

fn frames(startup_s: f32, reach: f32, recovery_s: f32) -> MoveFrameData {
    MoveFrameData {
        total_s: startup_s + 0.1 + recovery_s,
        startup_s,
        active_spans: vec![(startup_s, startup_s + 0.1)],
        recovery_s,
        cancel_windows: Vec::new(),
        reach,
    }
}

fn candidate(id: &str, startup_s: f32, reach: f32) -> AttackCandidate {
    AttackCandidate {
        move_id: id.to_string(),
        frames: frames(startup_s, reach, 0.2),
    }
}

fn stage() -> StageView {
    StageView {
        bounds: ae::Aabb::new(ae::Vec2::new(400.0, 300.0), ae::Vec2::new(400.0, 300.0)),
    }
}

fn view_with(me_x: f32, foe_x: f32) -> WorldView {
    WorldView {
        self_view: SelfView {
            pos: ae::Vec2::new(me_x, 300.0),
            gravity_down: ae::Vec2::new(0.0, 1.0),
            alive: true,
            on_ground: true,
            can_dash: true,
            can_shield: true,
            health_max: 100,
            ..Default::default()
        },
        stage: stage(),
        actors: vec![PerceivedActor {
            id: "foe".to_string(),
            pos: ae::Vec2::new(foe_x, 300.0),
            faction: ActorFaction::Enemy,
            hostile_to_self: true,
            alive: true,
            on_ground: true,
            health_max: 100,
            ..Default::default()
        }],
        ..Default::default()
    }
}

// ── the features ─────────────────────────────────────────────────────────

/// **The feature that makes the brain understand a new character.** Reach comes
/// from CM7's frame data, so a brain handed an unfamiliar moveset prices its
/// jab as a jab without anyone typing a table.
#[test]
fn reach_fit_peaks_when_the_attack_exactly_spans_the_gap() {
    assert_eq!(reach_fit(100.0, 100.0), 1.0);
    assert!(reach_fit(100.0, 120.0) < 1.0);
    assert!(reach_fit(100.0, 120.0) > reach_fit(100.0, 180.0));
    // Whiffing by a mile and by two miles are equally useless.
    assert_eq!(reach_fit(100.0, 400.0), 0.0);
    assert_eq!(reach_fit(100.0, 900.0), 0.0);
    // A move that is TOO LONG for the gap is also a bad fit — you get hit out
    // of a lunge you started from touching distance.
    assert!(reach_fit(200.0, 20.0) < reach_fit(200.0, 190.0));
    // A reachless move (a buff, a summon) has no fit anywhere.
    assert_eq!(reach_fit(0.0, 50.0), 0.0);
}

#[test]
fn frame_advantage_is_measured_against_the_attacks_own_commitment() {
    // A 0.1s jab into a 0.3s commitment lands with a whole startup to spare.
    assert_eq!(frame_advantage(0.1, 0.3), 1.0);
    // A 0.5s smash into the same window does not.
    assert!(frame_advantage(0.5, 0.3) < 0.0);
    // An uncommitted opponent answers immediately: any startup is a gamble,
    // and a slower move is a worse one.
    assert!(frame_advantage(0.5, 0.0) <= frame_advantage(0.1, 0.0));
    assert_eq!(frame_advantage(0.1, 0.0), -1.0);
}

// ── the scorer ───────────────────────────────────────────────────────────

/// The whole point: at 100px, the 100px-reach jab beats the 400px lunge, and at
/// 400px it is the other way round. Nobody typed that; the frame data did.
#[test]
fn the_best_attack_is_the_one_whose_reach_fits_the_gap() {
    let kit = [candidate("jab", 0.1, 100.0), candidate("lunge", 0.1, 400.0)];
    let w = UtilityWeights::v1();

    let near = generate_options(
        Perceived::cheating(&view_with(300.0, 400.0)),
        Situation::Neutral,
        &kit,
        &w,
    );
    assert_eq!(near.best_attack().unwrap().move_id, "jab");

    let far = generate_options(
        Perceived::cheating(&view_with(100.0, 500.0)),
        Situation::Neutral,
        &kit,
        &w,
    );
    assert_eq!(far.best_attack().unwrap().move_id, "lunge");
}

/// **A committed opponent is what makes a slow attack viable at all.**
///
/// Note what this does NOT assert: that the smash BEATS the jab on a punish.
/// It should — but none of §1's four features reads a move's POWER, so at v1
/// weights the faster move wins every window it also fits. That is a real gap
/// (see this module's docs), and it is FB4's ladder to settle, not a unit
/// test's: §FB6 is explicit that *"scoring weights are NOT divined up front."*
/// What IS unarguable is the feature: only a committed opponent gives a slow
/// attack a non-negative frame advantage.
#[test]
fn only_a_committed_opponent_makes_a_slow_attacks_frame_advantage_non_negative() {
    let kit = [candidate("smash", 0.4, 100.0)];
    let w = UtilityWeights::v1();

    let v = view_with(300.0, 400.0);
    let free = generate_options(Perceived::cheating(&v), Situation::Neutral, &kit, &w);
    assert!(free.best_attack().unwrap().features.frame_advantage < 0.0);

    let mut v = view_with(300.0, 400.0);
    v.actors[0].phase = BodyPhase::AttackRecovery;
    v.actors[0].phase_remaining = 0.5;
    let punish = generate_options(Perceived::cheating(&v), Situation::Advantage, &kit, &w);
    let fa = punish.best_attack().unwrap().features.frame_advantage;
    assert!(fa >= 0.0, "a 0.4s smash into a 0.5s window lands: {fa}");

    // ...and an opponent whose ACTIVE frames are out is not committed to
    // anything the brain may walk into. `is_punishable` says so, and the
    // feature follows.
    let mut v = view_with(300.0, 400.0);
    v.actors[0].phase = BodyPhase::AttackActive;
    v.actors[0].phase_remaining = 0.5;
    let into_the_hitbox = generate_options(Perceived::cheating(&v), Situation::Neutral, &kit, &w);
    assert!(
        into_the_hitbox
            .best_attack()
            .unwrap()
            .features
            .frame_advantage
            < 0.0
    );
}

/// Kill potential rises with the VICTIM's damage, not with the move's. In a
/// smash-percent game a move's value is who it can end.
#[test]
fn kill_potential_reads_the_victims_meter() {
    let kit = [candidate("jab", 0.1, 100.0)];
    let w = UtilityWeights::v1();
    let mut v = view_with(300.0, 400.0);

    v.actors[0].damage_taken = 0;
    let fresh = generate_options(Perceived::cheating(&v), Situation::Neutral, &kit, &w);
    v.actors[0].damage_taken = 90;
    let ripe = generate_options(Perceived::cheating(&v), Situation::Neutral, &kit, &w);

    assert!(ripe.best_attack().unwrap().score > fresh.best_attack().unwrap().score);
    assert_eq!(ripe.best_attack().unwrap().features.kill_potential, 0.9);
}

/// **Stage risk is a COST.** Committing near a blastzone is how a level-9 CPU
/// dies to a level-3 one, and the weight is negative so it can never be bought
/// back by kill potential alone.
#[test]
fn committing_near_a_blastzone_costs_score() {
    let kit = [candidate("jab", 0.1, 100.0)];
    let w = UtilityWeights::v1();
    let safe = generate_options(
        Perceived::cheating(&view_with(400.0, 500.0)),
        Situation::Neutral,
        &kit,
        &w,
    );
    let edge = generate_options(
        Perceived::cheating(&view_with(10.0, 110.0)),
        Situation::Neutral,
        &kit,
        &w,
    );
    assert!(edge.best_attack().unwrap().score < safe.best_attack().unwrap().score);
    assert!(w.stage_risk < 0.0);
}

/// **A body past the blastzone has exactly one problem.** No attack is offered
/// at all — not a low-scoring one, none. `Recovery` is not a preference.
#[test]
fn recovery_offers_no_attacks_and_exactly_one_obligation() {
    let kit = [candidate("jab", 0.1, 100.0), candidate("smash", 0.4, 100.0)];
    let opts = generate_options(
        Perceived::cheating(&view_with(-40.0, 400.0)),
        Situation::Recovery,
        &kit,
        &UtilityWeights::v1(),
    );
    assert!(opts.attacks.is_empty());
    assert!(opts.best_attack().is_none());
    assert_eq!(opts.best_movement().unwrap().verb, MovementVerb::Recover);
}

/// Movement expresses the situation's ONE obligation, so a brain with no L3
/// still plays a recognizable game.
#[test]
fn each_situation_has_its_obligation() {
    let kit = [candidate("jab", 0.1, 100.0)];
    let w = UtilityWeights::v1();
    for (situation, expect) in [
        (Situation::Disadvantage, MovementVerb::Shield),
        (Situation::Advantage, MovementVerb::Approach),
        (Situation::EdgeGuard, MovementVerb::Approach),
        (Situation::Neutral, MovementVerb::Approach),
    ] {
        let opts = generate_options(
            Perceived::cheating(&view_with(300.0, 400.0)),
            situation,
            &kit,
            &w,
        );
        assert_eq!(
            opts.best_movement().unwrap().verb,
            expect,
            "{situation:?} should reach for {expect:?}"
        );
    }
}

/// A body without a capability never proposes it. The brain physically cannot
/// ask for what the body would refuse (invariant I3).
#[test]
fn the_capability_mask_gates_every_verb() {
    let kit = [candidate("jab", 0.1, 100.0)];
    let w = UtilityWeights::v1();
    let mut v = view_with(300.0, 400.0);
    v.self_view.can_shield = false;
    v.self_view.can_dash = false;

    let opts = generate_options(Perceived::cheating(&v), Situation::Disadvantage, &kit, &w);
    assert!(opts
        .movement
        .iter()
        .all(|m| m.verb != MovementVerb::Shield && m.verb != MovementVerb::Dash));
    assert_eq!(opts.best_movement().unwrap().verb, MovementVerb::Retreat);
}

/// **Determinism.** Two attacks that score identically are ordered by move id,
/// not by the kit's declaration order. Otherwise `best_attack` depends on how a
/// content author sorted a RON file (ADR 0023).
#[test]
fn ties_break_on_the_move_id_not_on_the_kits_order() {
    let w = UtilityWeights::v1();
    let v = view_with(300.0, 400.0);
    let a = generate_options(
        Perceived::cheating(&v),
        Situation::Neutral,
        &[
            candidate("zeta", 0.1, 100.0),
            candidate("alpha", 0.1, 100.0),
        ],
        &w,
    );
    let b = generate_options(
        Perceived::cheating(&v),
        Situation::Neutral,
        &[
            candidate("alpha", 0.1, 100.0),
            candidate("zeta", 0.1, 100.0),
        ],
        &w,
    );
    assert_eq!(a.best_attack().unwrap().move_id, "alpha");
    assert_eq!(b.best_attack().unwrap().move_id, "alpha");
}

/// `score == Σ weight_i · feature_i` by construction, so a failing ladder run
/// can be READ. Zeroed weights make every attack score zero — the ablation that
/// proves no feature is smuggled in outside the dot product.
#[test]
fn the_score_is_exactly_the_weighted_features() {
    let kit = [candidate("jab", 0.1, 100.0)];
    let zero = UtilityWeights {
        reach_fit: 0.0,
        frame_advantage: 0.0,
        kill_potential: 0.0,
        stage_risk: 0.0,
    };
    let opts = generate_options(
        Perceived::cheating(&view_with(300.0, 400.0)),
        Situation::Neutral,
        &kit,
        &zero,
    );
    assert_eq!(opts.best_attack().unwrap().score, 0.0);

    let w = UtilityWeights::v1();
    let opts = generate_options(
        Perceived::cheating(&view_with(300.0, 400.0)),
        Situation::Neutral,
        &kit,
        &w,
    );
    let a = opts.best_attack().unwrap();
    assert!((a.score - a.features.dot(&w)).abs() < 1e-6);
}

/// No opponent, no attacks — and no panic. A brain alone on the stage is not a
/// brain with a zero-scored kit; it is a brain with nothing to price.
#[test]
fn a_brain_with_no_opponent_offers_no_attacks() {
    let mut v = view_with(300.0, 400.0);
    v.actors.clear();
    let opts = generate_options(
        Perceived::cheating(&v),
        Situation::Neutral,
        &[candidate("jab", 0.1, 100.0)],
        &UtilityWeights::v1(),
    );
    assert!(opts.attacks.is_empty());
    assert!(!opts.movement.is_empty());
}
