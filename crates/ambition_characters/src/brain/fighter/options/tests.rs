//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::actor::attack_gesture::AttackDir;
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
        max_damage: 1,
        max_knockback: 0.0,
        start_impulse: (0.0, 0.0),
    }
}

fn candidate(id: &str, startup_s: f32, reach: f32) -> AttackCandidate {
    AttackCandidate {
        move_id: id.to_string(),
        frames: frames(startup_s, reach, 0.2),
        // The plain forward press. What binding a candidate carries is the
        // CALLER's answer (it enumerates them against the real moveset); L2
        // scores the move and hands the binding back untouched.
        binding: AttackBinding {
            verb: AttackVerb::Basic,
            direction: AttackDir::Forward,
        },
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
        expected_payoff: 0.0,
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

/// FB2 recorded the gap; FB6a closes it, and this is the recorded scenario:
/// a punish window both a jab and a smash fit. Without `expected_payoff` the
/// jab always won (faster ⇒ more frame advantage, nothing priced power). With
/// it, the smash that out-damages the jab — and still lands inside the
/// window — outbids it. In NEUTRAL the payoff is zero for everyone and the
/// jab keeps winning, which is the feature gating on a plausible landing
/// rather than smuggling power into every exchange.
#[test]
fn the_smash_outbids_the_jab_on_a_punish_it_fits() {
    let smash = AttackCandidate {
        move_id: "smash".to_string(),
        frames: MoveFrameData {
            max_damage: 20,
            ..frames(0.25, 100.0, 0.4)
        },
        binding: AttackBinding {
            verb: AttackVerb::Smash,
            direction: AttackDir::Forward,
        },
    };
    let jab = AttackCandidate {
        move_id: "jab".to_string(),
        binding: AttackBinding {
            verb: AttackVerb::Basic,
            direction: AttackDir::Forward,
        },
        frames: MoveFrameData {
            max_damage: 4,
            ..frames(0.1, 100.0, 0.2)
        },
    };
    let kit = [jab, smash];
    let w = UtilityWeights::v1();

    // A punish window longer than either startup: the opponent is committed.
    let mut view = view_with(300.0, 400.0);
    view.actors[0].phase = BodyPhase::AttackRecovery;
    view.actors[0].phase_remaining = 0.6;
    let opts = generate_options(Perceived::cheating(&view), Situation::Advantage, &kit, &w);
    assert_eq!(
        opts.best_attack().unwrap().move_id,
        "smash",
        "a priced punish takes the strong move, not the fast one"
    );

    // Neutral: nobody is committed, payoff gates to zero, the jab wins again.
    let view = view_with(300.0, 400.0);
    let opts = generate_options(Perceived::cheating(&view), Situation::Neutral, &kit, &w);
    assert_eq!(opts.best_attack().unwrap().move_id, "jab");
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

// ── the ledge ────────────────────────────────────────────────────────────────

/// A platform with edges, and a body standing on it.
fn on_a_platform(me_x: f32, foe_x: f32, platform: (f32, f32)) -> WorldView {
    use crate::perception::{PerceivedSolid, SolidKind};
    let mut view = view_with(me_x, foe_x);
    // Half-extents, so "a body width" is a real number rather than the default 0.
    view.self_view.half_extent = ae::Vec2::new(12.0, 24.0);
    view.terrain = vec![PerceivedSolid {
        aabb: ae::Aabb {
            min: ae::Vec2::new(platform.0, 324.0),
            max: ae::Vec2::new(platform.1, 380.0),
        },
        kind: SolidKind::Solid,
    }];
    view
}

fn score_of(options: &OptionSet, verb: MovementVerb) -> f32 {
    options
        .movement
        .iter()
        .find(|option| option.verb == verb)
        .map(|option| option.score)
        .unwrap_or(f32::NAN)
}

/// **Approaching toward a ledge is penalised.** (2026-07-31)
///
/// The defect: a fighter lost all three of its stocks WITHOUT BEING HIT, by
/// running past its opponent and off the edge. The brain was not wrong — until
/// the smash stage, every room in this engine was ENCLOSED, so `Approach` was
/// always safe and nothing had to score a ledge.
#[test]
fn approaching_off_the_edge_of_a_platform_scores_worse_than_approaching_inward() {
    // Foe to the RIGHT, and the platform ends 10px to the right: closing means
    // walking off.
    let at_the_edge = on_a_platform(390.0, 500.0, (100.0, 400.0));
    // The same fighter with the same foe, in the middle of the platform.
    let mid_platform = on_a_platform(250.0, 500.0, (100.0, 400.0));

    let weights = UtilityWeights::default();
    let edge = generate_options(
        Perceived::cheating(&at_the_edge),
        Situation::Neutral,
        &[],
        &weights,
    );
    let safe = generate_options(
        Perceived::cheating(&mid_platform),
        Situation::Neutral,
        &[],
        &weights,
    );

    assert!(
        score_of(&edge, MovementVerb::Approach) < score_of(&safe, MovementVerb::Approach),
        "closing toward a ledge scores the same as closing across open floor, so \
         the brain walks off the stage chasing somebody"
    );
    assert!(
        score_of(&edge, MovementVerb::Approach) < score_of(&edge, MovementVerb::Retreat),
        "at the edge, walking off still outranks backing away — which is how a \
         fighter loses a stock without being hit"
    );
}

/// **A body with no perceived terrain is not penalised.** An airborne fighter,
/// or a view whose terrain was never filled, is not a ledge question — and
/// treating "I cannot see the floor" as "the floor ends here" would freeze every
/// brain in a composition that does not build terrain.
#[test]
fn a_view_with_no_terrain_scores_movement_exactly_as_before() {
    let no_terrain = view_with(390.0, 500.0);
    let weights = UtilityWeights::default();
    let options = generate_options(
        Perceived::cheating(&no_terrain),
        Situation::Neutral,
        &[],
        &weights,
    );
    assert_eq!(
        score_of(&options, MovementVerb::Approach),
        0.5,
        "a view with no terrain acquired a ledge penalty, so a brain that cannot \
         see the floor refuses to move"
    );
}

#[test]
fn a_body_with_no_jumps_left_is_not_offered_a_jump() {
    // Every other verb in `generate_options` asks whether the body can do it.
    // Jump was the one that did not, and an option that presses to nothing is
    // worse than a wasted press: the rollout rolls the verb, the shadow's air
    // jump is budgeted too so the line goes nowhere, and nowhere scores as safe.
    let mut view = view_with(300.0, 500.0);
    view.self_view.on_ground = false;
    view.self_view.air_jumps_left = 0;
    let options = generate_options(
        Perceived::cheating(&view),
        Situation::Neutral,
        &[],
        &UtilityWeights::v1(),
    );
    assert!(
        !options
            .movement
            .iter()
            .any(|option| option.verb == MovementVerb::Jump),
        "a body with no jumps left was offered one: {:?}",
        options.movement
    );

    view.self_view.air_jumps_left = 1;
    let options = generate_options(
        Perceived::cheating(&view),
        Situation::Neutral,
        &[],
        &UtilityWeights::v1(),
    );
    assert!(
        options
            .movement
            .iter()
            .any(|option| option.verb == MovementVerb::Jump),
        "and one jump left is a jump on offer: {:?}",
        options.movement
    );
}

/// **An attack that cannot reach is not offered at all.**
///
/// `reach_fit` priced a hopeless swing at zero and left it in the list, and the
/// consumer takes `attacks.first()` whenever L3 names nothing — so the list
/// being non-empty IS the decision. Scoring it low was never going to be enough.
///
/// ⚠ a zero-reach move (a buff, a summon) stays: reach is not its question, and
/// dropping it would delete a whole class of move from every kit that has one.
#[test]
fn an_attack_that_cannot_span_the_gap_is_not_offered() {
    let jab = candidate("jab", 0.08, 40.0);
    let mut buff = candidate("buff", 0.2, 0.0);
    buff.frames.max_damage = 0;
    let kit = vec![jab, buff];
    let weights = UtilityWeights::default();

    // In reach: both are offered.
    let close = view_with(300.0, 340.0);
    let offered = generate_options(
        crate::perception::Perceived::cheating(&close),
        Situation::Neutral,
        &kit,
        &weights,
    );
    assert!(
        offered.attacks.iter().any(|a| a.move_id == "jab"),
        "a 40px jab at a 40px gap has to be on the list, or the assertion below \
         is about an empty kit"
    );

    // Far out of reach: the jab goes, the reachless buff stays.
    let far = view_with(300.0, 900.0);
    let offered = generate_options(
        crate::perception::Perceived::cheating(&far),
        Situation::Neutral,
        &kit,
        &weights,
    );
    assert!(
        !offered.attacks.iter().any(|a| a.move_id == "jab"),
        "a 40px jab is still offered at a 600px gap — and every press of it \
         costs the body `SLASH_RECOIL` backwards, which is how the fighter swung \
         itself off the stage"
    );
    assert!(
        offered.attacks.iter().any(|a| a.move_id == "buff"),
        "a zero-reach move has no reach question and must survive the filter"
    );
}
