use super::*;
use crate::actor::attack_gesture::AttackDir;
use crate::actor::ActorFaction;
use crate::brain::attack_kit::{ActionLegality, AttackBinding, AttackCandidate, AttackVerb};
use crate::perception::{BodyPhase, PerceivedActor, SelfView, StageView, WorldView};
use ambition_platformer2d_core as ae;

fn frames(startup_s: f32, reach: f32, recovery_s: f32) -> MoveFrameData {
    MoveFrameData {
        total_s: startup_s + 0.1 + recovery_s,
        charge_hold_at_s: None,
        startup_s,
        active_spans: vec![(startup_s, startup_s + 0.1)],
        recovery_s,
        cancel_windows: Vec::new(),
        reach,
        ignores_guard: false,
        //  the fixture's move is a FORWARD POKE, and now it says so. `reach`
        // is only the `+x` face of the authored volumes, so a fixture that set it
        // alone described a move with no hittable region at all once the scorer
        // started reading the region. This is the box a poke of that length
        // actually covers, which is what production derives from the volumes.
        coverage: (reach > 0.0).then(|| ambition_entity_catalog::MoveCoverage {
            min: (0.0, -12.0),
            max: (reach, 12.0),
        }),
        max_damage: 1,
        max_knockback: 0.0,
        start_impulse: (0.0, 0.0),
        // No self-motion at all. `lifting_candidate` below is what opts a
        // fixture INTO carrying a route, so the ordinary move stays a move.
        lift_speed: 0.0,
        lift_at_s: 0.0,
        lift_side: 0.0,
        recovery_route: Default::default(),
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
        legality: ActionLegality::Now,
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
            burst: ambition_platformer2d_core::BurstManeuver::Dash,
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

/// The feature that makes the brain understand a new character. The hittable
/// region comes from CM7's frame data, so a brain handed an unfamiliar moveset
/// prices its jab as a jab without anyone typing a table.
///
///  and it prices an ANTI-AIR as an anti-air, which is the whole reason this
/// feature stopped being a scalar. Its predecessor compared
/// `reach` — the `+x` face alone — against `(foe.pos - me.pos).length()`, so a
/// move whose volume sits above the shoulder was indistinguishable from a poke
/// of the same length and the vertical half of every authored kit was never
/// selected for the reason it exists.
#[test]
fn coverage_fit_peaks_where_the_move_can_actually_hit() {
    use ambition_entity_catalog::MoveCoverage;

    // A forward poke: 0..100 ahead, a body's height tall.
    let poke = MoveCoverage {
        min: (0.0, -12.0),
        max: (100.0, 12.0),
    };
    // The same swing aimed UP: the same 100px of extent, above the shoulder.
    let anti_air = MoveCoverage {
        min: (-12.0, -100.0),
        max: (12.0, 0.0),
    };
    let point = (0.0, 0.0);

    // ── the curve, unchanged from the scalar it generalises ──────────────
    assert_eq!(coverage_fit(Some(&poke), (100.0, 0.0), point), 1.0);
    assert!(coverage_fit(Some(&poke), (120.0, 0.0), point) < 1.0);
    assert!(
        coverage_fit(Some(&poke), (120.0, 0.0), point)
            > coverage_fit(Some(&poke), (180.0, 0.0), point)
    );
    // Whiffing by a mile and by two miles are equally useless.
    assert_eq!(coverage_fit(Some(&poke), (400.0, 0.0), point), 0.0);
    assert_eq!(coverage_fit(Some(&poke), (900.0, 0.0), point), 0.0);
    // A move that is TOO LONG for the gap is also a bad fit — you get hit out
    // of a lunge you started from touching distance.
    let long = MoveCoverage {
        min: (0.0, -12.0),
        max: (200.0, 12.0),
    };
    assert!(
        coverage_fit(Some(&long), (20.0, 0.0), point)
            < coverage_fit(Some(&long), (190.0, 0.0), point)
    );
    // A move that lands no volume (a buff, a summon) has no fit anywhere.
    assert_eq!(coverage_fit(None, (50.0, 0.0), point), 0.0);

    // ──  THE CLAIM THE SCALAR COULD NOT MAKE ──────────────────────────
    //
    // One opponent 100px AHEAD and one 100px ABOVE are the SAME `gap`, and the
    // old feature scored them identically for every move in every kit. The two
    // moves must now disagree about them, and disagree in opposite directions.
    let ahead = (100.0, 0.0);
    let above = (0.0, -100.0);
    assert!(
        coverage_fit(Some(&poke), ahead, point) > coverage_fit(Some(&poke), above, point),
        "a forward poke rates an opponent overhead as well as one in front of it"
    );
    assert!(
        coverage_fit(Some(&anti_air), above, point) > coverage_fit(Some(&anti_air), ahead, point),
        "an anti-air rates an opponent in front of it as well as one overhead — \
         which is the measured defect, not a hypothetical one"
    );
    //  and the poison for the pair: the two moves are the SAME SIZE. If the
    // anti-air simply had more extent it would win everywhere and the two
    // assertions above would be measuring a bigger hitbox rather than a
    // direction.
    assert_eq!(
        coverage_fit(Some(&poke), ahead, point),
        coverage_fit(Some(&anti_air), above, point),
        "the two fixtures are mirror images, so a move facing its own opponent \
         must score identically — otherwise the comparison above is about size"
    );

    // A hitbox catches a HURTBOX: a body whose centre sits past the volume is
    // still hit when its near edge is inside.
    assert!(
        coverage_fit(Some(&poke), (112.0, 0.0), (14.0, 14.0))
            > coverage_fit(Some(&poke), (112.0, 0.0), point)
    );
}

#[test]
fn frame_advantage_is_measured_against_the_attacks_own_commitment() {
    // The kit's slowest startup is the scale, so the number is comparable
    // BETWEEN moves rather than each one being normalised by itself.
    const SLOWEST: f32 = 0.5;
    // A 0.1s jab into a 0.3s commitment lands with room to spare.
    assert!(frame_advantage(0.1, 0.3, SLOWEST) > 0.0);
    // A 0.5s smash into the same window does not.
    assert!(frame_advantage(0.5, 0.3, SLOWEST) < 0.0);
    // The jab beats the smash into the same window, which is the whole point.
    assert!(frame_advantage(0.1, 0.3, SLOWEST) > frame_advantage(0.5, 0.3, SLOWEST));
    // AN UNCOMMITTED OPPONENT ANSWERS IMMEDIATELY: any startup is a gamble, and
    // A SLOWER MOVE IS A WORSE ONE.
    //
    // ⛔ STRICTLY worse. This was `<=` and both sides were `-1.0`, because the
    // scale used to be the move's own startup — so the assertion passed while
    // its own comment was false, and the CPU threw no jabs for as long as
    // anybody has been watching.
    assert!(frame_advantage(0.5, 0.0, SLOWEST) < frame_advantage(0.1, 0.0, SLOWEST));
    // The slowest move in the kit is the floor.
    assert_eq!(frame_advantage(0.5, 0.0, SLOWEST), -1.0);
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

/// A committed opponent is what makes a slow attack viable at all.
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

/// Stage risk is a COST. Committing near a blastzone is how a level-9 CPU
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

/// A body past the blastzone has exactly one problem, and a kit of swings is
/// not an answer to it. No offensive attack is offered at all — not a
/// low-scoring one, none. `Recovery` is not a preference.
///
///  the rule is about the REPERTOIRE, not about the situation. These two
/// moves lift nobody (`lift_speed: 0.0`), which is why nothing is offered; the
/// pair below shows what happens when one of them does.
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

/// The same candidate, plus the one number that makes it a way home.
fn lifting_candidate(id: &str, lift_speed: f32, lift_at_s: f32) -> AttackCandidate {
    let mut c = candidate(id, 0.2, 40.0);
    c.frames.lift_speed = lift_speed;
    c.frames.lift_at_s = lift_at_s;
    // ⛔ THE FIXTURE BUILDS `MoveFrameData` BY HAND, so it has to state what
    // `MoveSpec::frame_data` would fold for it: a commanded rise IS a burst.
    // Setting only the scalar would build a move that lifts and offers no route,
    // which production cannot produce.
    c.frames.recovery_route = ambition_entity_catalog::RecoveryRoute::Burst {
        speed: lift_speed,
        side: c.frames.lift_side,
        at_s: lift_at_s,
    };
    c.binding = AttackBinding {
        verb: AttackVerb::Special,
        direction: AttackDir::Up,
    };
    c
}

/// A RECOVERING BODY IS OFFERED THE MOVE THAT LIFTS IT, AND ONLY THAT MOVE.
///
///  and the selection is geometric. Nothing here names a character, a verb
/// or a move id — the jab and the smash are excluded because they command no
/// against-gravity speed, and `ascend` is offered because it commands one. Give
/// a second body a rising move and it is understood by the same line.
#[test]
fn a_recovering_body_is_offered_the_move_that_lifts_it() {
    let kit = [
        candidate("jab", 0.1, 100.0),
        candidate("smash", 0.4, 100.0),
        lifting_candidate("ascend", 980.0, 0.2),
    ];
    let opts = generate_options(
        Perceived::cheating(&view_with(-40.0, 400.0)),
        Situation::Recovery,
        &kit,
        &UtilityWeights::v1(),
    );
    let ids: Vec<&str> = opts.attacks.iter().map(|a| a.move_id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["ascend"],
        "a recovering body must be offered its way home and nothing else"
    );
    // And it is offered as the PRESS that reaches it, so the winner is
    // executable on the ordinary gesture seam a human uses.
    assert_eq!(
        opts.best_attack().unwrap().binding,
        AttackBinding {
            verb: AttackVerb::Special,
            direction: AttackDir::Up,
        }
    );
}

/// THE STRONGEST LIFT LEADS, AND A TIE BREAKS ON THE MOVE ID. Two ways home
/// is a legal repertoire, and which one the brain reaches for must be a function
/// of the numbers rather than of the kit's declaration order (ADR 0023).
#[test]
fn the_strongest_lift_leads_and_ties_do_not_depend_on_kit_order() {
    let strong = lifting_candidate("ascend", 980.0, 0.2);
    let weak = lifting_candidate("hop", 300.0, 0.1);
    let forward = [weak.clone(), strong.clone()];
    let reversed = [strong, weak];
    let opts = |kit: &[AttackCandidate]| -> Vec<String> {
        generate_options(
            Perceived::cheating(&view_with(-40.0, 400.0)),
            Situation::Recovery,
            kit,
            &UtilityWeights::v1(),
        )
        .attacks
        .into_iter()
        .map(|a| a.move_id)
        .collect()
    };
    assert_eq!(opts(&forward), vec!["ascend", "hop"]);
    assert_eq!(opts(&forward), opts(&reversed));
}

/// A BODY WITH A REAL RECOVERY MOVE STOPS BEING OFFERED THE TRAVERSAL VERB.
///
/// Blink is a general-purpose way of being somewhere else; leaning on it to get
/// home is what a fighter does when its repertoire has no answer. Both halves
/// are observed here — the same blink-capable body IS offered it while its kit
/// commands no lift — so this cannot pass by the option having quietly gone
/// away for everybody.
#[test]
fn an_authored_lift_displaces_the_traversal_verb_in_recovery() {
    let mut view = view_with(-40.0, 400.0);
    view.self_view.can_blink = true;
    let w = UtilityWeights::v1();
    let verbs = |kit: &[AttackCandidate]| -> Vec<MovementVerb> {
        generate_options(Perceived::cheating(&view), Situation::Recovery, kit, &w)
            .movement
            .into_iter()
            .map(|m| m.verb)
            .collect()
    };
    let swings = [candidate("jab", 0.1, 100.0)];
    assert!(
        verbs(&swings).contains(&MovementVerb::Blink),
        "a body with no way home of its own still falls back on traversal"
    );
    let with_lift = [
        candidate("jab", 0.1, 100.0),
        lifting_candidate("ascend", 980.0, 0.2),
    ];
    assert!(!verbs(&with_lift).contains(&MovementVerb::Blink));
    assert_eq!(
        verbs(&with_lift).first().copied(),
        Some(MovementVerb::Recover),
        "the obligation is unchanged: steer home, and now throw the move too"
    );
}

/// A LIFTING MOVE IS STILL AN ORDINARY ATTACK EVERYWHERE ELSE. The affordance
/// changes what `Recovery` offers and nothing else — in neutral, `ascend` is
/// scored against the foe by the same five features as every other move, so a
/// recovery special that happens to be a good anti-air stays one.
#[test]
fn a_lifting_move_is_scored_as_an_ordinary_attack_in_neutral() {
    let kit = [
        candidate("jab", 0.1, 40.0),
        lifting_candidate("ascend", 980.0, 0.2),
    ];
    let opts = generate_options(
        Perceived::cheating(&view_with(300.0, 340.0)),
        Situation::Neutral,
        &kit,
        &UtilityWeights::v1(),
    );
    assert!(opts
        .attacks
        .iter()
        .any(|a| a.move_id == "ascend" && a.features != Features::default()));
    assert!(opts.attacks.iter().any(|a| a.move_id == "jab"));
}

/// `lifting_candidates` reads the number and nothing else. The unit under
/// every rule above, on its own, so a failure says which half broke.
#[test]
fn lifting_candidates_selects_on_commanded_lift_alone() {
    let kit = [
        candidate("jab", 0.1, 100.0),
        lifting_candidate("ascend", 980.0, 0.2),
        lifting_candidate("hop", 300.0, 0.1),
    ];
    let ids: Vec<&str> = lifting_candidates(&kit)
        .into_iter()
        .map(|c| c.move_id.as_str())
        .collect();
    assert_eq!(ids, vec!["ascend", "hop"]);
    assert!(lifting_candidates(&[candidate("jab", 0.1, 100.0)]).is_empty());
}

/// ⭐⭐ A ROUTE THAT IS NOT A BURST IS STILL A ROUTE.
///
/// ⛔⛔ D250: this filter asked `lift_speed > 0.0`, which is the shape of exactly
/// one route kind. A summoned steerable mount and a teleport both command no
/// impulse, so a fighter whose only way home is one of them was offered nothing
/// at all. ⛔ AND THE ORDER IS NOT A RANKING — bursts keep the order they have
/// always had, the carrying routes follow by claimed carry, and ties break on
/// the move id (ADR 0023). The LENS decides which one is useful.
#[test]
fn lifting_candidates_offers_every_kind_of_way_home() {
    use ambition_entity_catalog::RecoveryRoute;
    let carrying = |id: &str, route: RecoveryRoute| {
        let mut c = candidate(id, 0.2, 40.0);
        c.frames.recovery_route = route;
        c
    };
    let kit = [
        candidate("jab", 0.1, 100.0),
        carrying("blink", RecoveryRoute::Teleport { distance: 250.0 }),
        lifting_candidate("ascend", 980.0, 0.2),
        carrying(
            "shark",
            RecoveryRoute::SustainedAuthority {
                seconds: 5.0,
                reach: 650.0,
            },
        ),
    ];
    let ids: Vec<&str> = lifting_candidates(&kit)
        .into_iter()
        .map(|c| c.move_id.as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["ascend", "shark", "blink"],
        "a fighter whose recovery is a ride or a teleport was offered nothing"
    );
    assert!(
        lifting_candidates(&[candidate("jab", 0.1, 100.0)]).is_empty(),
        "a move that is no way home became one"
    );
}

/// Movement expresses the situation's ONE obligation, so a brain with no L3
/// still plays a recognizable game.
///
/// Two shielding fighters who never move is a stable state, and it is what the Smash stage did for
/// a whole match the day these bodies were first given the capability. Shield is a reaction to a
/// swing — [`disadvantage_shields_only_against_an_incoming_swing`] is the other half of this pair
/// and asserts it still happens when there IS one.
#[test]
fn each_situation_has_its_obligation() {
    let kit = [candidate("jab", 0.1, 100.0)];
    let w = UtilityWeights::v1();
    for (situation, expect) in [
        (Situation::Disadvantage, MovementVerb::Retreat),
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

/// A shield is a reaction to a SWING, not a stance for being cornered.
///
/// The half that keeps the change above honest: with a hostile actually
/// attacking, the guard is still the best answer in `Disadvantage`. Without one
/// the same view retreats — so this pair says the verb is GATED, not removed.
#[test]
fn disadvantage_shields_only_against_an_incoming_swing() {
    let kit = [candidate("jab", 0.1, 100.0)];
    let w = UtilityWeights::v1();

    let quiet = view_with(300.0, 400.0);
    let calm = generate_options(
        Perceived::cheating(&quiet),
        Situation::Disadvantage,
        &kit,
        &w,
    );
    assert_eq!(
        calm.best_movement().unwrap().verb,
        MovementVerb::Retreat,
        "cornered with nothing incoming is a spacing problem, not a guarding one"
    );

    let mut swinging = view_with(300.0, 400.0);
    swinging.actors[0].phase = crate::perception::BodyPhase::AttackStartup;
    let threatened = generate_options(
        Perceived::cheating(&swinging),
        Situation::Disadvantage,
        &kit,
        &w,
    );
    assert_eq!(
        threatened.best_movement().unwrap().verb,
        MovementVerb::Shield,
        "a hostile is mid-swing and the guard was not offered, so the gate \
         removed the verb instead of timing it"
    );
}

/// A body without a capability never proposes it. The brain physically cannot
/// ask for what the body would refuse (invariant I3).
#[test]
fn the_capability_mask_gates_every_verb() {
    let kit = [candidate("jab", 0.1, 100.0)];
    let w = UtilityWeights::v1();
    let mut v = view_with(300.0, 400.0);
    v.self_view.can_shield = false;
    v.self_view.burst = ambition_platformer2d_core::BurstManeuver::None;

    let opts = generate_options(Perceived::cheating(&v), Situation::Disadvantage, &kit, &w);
    assert!(opts
        .movement
        .iter()
        .all(|m| m.verb != MovementVerb::Shield && m.verb != MovementVerb::Dash));
    assert_eq!(opts.best_movement().unwrap().verb, MovementVerb::Retreat);
}

/// THE EVADE VERB IS WHICHEVER MANEUVER THE PRESS ACTUALLY PRODUCES.
///
///  `apply_dodge` claims the dash buffer before `apply_dash` can see it, so on
/// a dodge-capable body the press is a roll. The Smash fighters author
/// `dash: true` AND `dodge: true`, which made every burst this brain chose on
/// that stage a maneuver it had not named and the shadow rollout had not
/// modelled.
///
///  AND THE FIRST REPAIR WAS STILL WRONG, which is why this test now
/// varies a resolved maneuver rather than two capability flags. `can_dodge` says
/// the body OWNS a dodge; on cooldown, `apply_dodge` declines without consuming
/// the press and `apply_dash` performs a dash. A test that varies capabilities
/// cannot see that case at all — it is the fourth row below, and it was
/// unreachable by the previous instrument.
#[test]
fn the_evade_verb_is_whichever_maneuver_the_dash_button_actually_produces() {
    use ambition_platformer2d_core::BurstManeuver;
    let kit = [candidate("jab", 0.1, 100.0)];
    let w = UtilityWeights::v1();
    let verbs = |burst: BurstManeuver| -> Vec<MovementVerb> {
        let mut v = view_with(300.0, 400.0);
        v.self_view.burst = burst;
        generate_options(Perceived::cheating(&v), Situation::Neutral, &kit, &w)
            .movement
            .iter()
            .map(|m| m.verb)
            .collect()
    };

    for rolling in [BurstManeuver::GroundDodge, BurstManeuver::AirDodge] {
        let offered = verbs(rolling);
        assert!(
            offered.contains(&MovementVerb::Dodge) && !offered.contains(&MovementVerb::Dash),
            "a body whose press rolls must be offered the roll and never the dash \
             ({rolling:?}): {offered:?}"
        );
    }

    // It is ALSO the case the capability instrument could not express — a Smash fighter owning
    // both, mid-dodge-cooldown, resolves here.
    let offered = verbs(BurstManeuver::Dash);
    assert!(
        offered.contains(&MovementVerb::Dash) && !offered.contains(&MovementVerb::Dodge),
        "a press that dashes is offered as a dash, whatever the body OWNS: {offered:?}"
    );

    let offered = verbs(BurstManeuver::None);
    assert!(
        !offered.contains(&MovementVerb::Dash) && !offered.contains(&MovementVerb::Dodge),
        "and a press that does nothing is not an option at all: {offered:?}"
    );
}

/// A roll is worth more against a swing than a dash is — the defensive slot
/// prices the maneuver, not the button.
///
///  comparative on purpose: pinning `0.75` would go green on a build where
/// every score had drifted together, and what matters is that i-frames outrank
/// plain travel while both stay below the guard.
#[test]
fn the_evade_outscores_a_plain_dash_when_something_is_swinging() {
    let kit = [candidate("jab", 0.1, 100.0)];
    let w = UtilityWeights::v1();
    let score_of = |dodge: bool, verb: MovementVerb| {
        let mut v = view_with(300.0, 400.0);
        v.self_view.burst = if dodge {
            ambition_platformer2d_core::BurstManeuver::GroundDodge
        } else {
            ambition_platformer2d_core::BurstManeuver::Dash
        };
        v.actors[0].phase = crate::perception::BodyPhase::AttackStartup;
        generate_options(Perceived::cheating(&v), Situation::Disadvantage, &kit, &w)
            .movement
            .iter()
            .find(|m| m.verb == verb)
            .map(|m| m.score)
            .unwrap_or_else(|| panic!("{verb:?} was offered"))
    };
    assert!(
        score_of(true, MovementVerb::Dodge) > score_of(false, MovementVerb::Dash),
        "an evade with i-frames answers a swing; a dash is only travel"
    );
}

/// Determinism. Two attacks that score identically are ordered by move id,
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
        capture_value: 0.0,
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
/// jab always won (faster  more frame advantage, nothing priced power). With
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
        legality: ActionLegality::Now,
    };
    let jab = AttackCandidate {
        move_id: "jab".to_string(),
        binding: AttackBinding {
            verb: AttackVerb::Basic,
            direction: AttackDir::Forward,
        },
        legality: ActionLegality::Now,
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

/// The brain was not wrong — until the smash stage, every room in this engine was ENCLOSED, so
/// `Approach` was always safe and nothing had to score a ledge.
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

/// A body with no perceived terrain is not penalised. An airborne fighter,
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

/// An attack that cannot reach is not offered at all.
///
/// `reach_fit` priced a hopeless swing at zero and left it in the list, and the
/// consumer takes `attacks.first()` whenever L3 names nothing — so the list
/// being non-empty IS the decision. Scoring it low was never going to be enough.
///
///  a zero-reach move (a buff, a summon) stays: reach is not its question, and
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

// ── what a hold is worth ( policy half) ────────────────────────────

/// A grab candidate with a real capture box, the way `capture_candidate` builds
/// one: no damage, no `reach` from a volume, guard ignored, and a `coverage`
/// taken from the `CAPTURE_ATTEMPT` params rather than from a hitbox.
fn grab_candidate(reach: f32) -> AttackCandidate {
    let mut frames = frames(0.07, 0.0, 0.2);
    frames.reach = reach;
    frames.coverage = Some(ambition_entity_catalog::MoveCoverage {
        min: (0.0, -12.0),
        max: (reach, 12.0),
    });
    //  the two facts that make a grab a grab and not a weak poke.
    frames.max_damage = 0;
    frames.ignores_guard = true;
    AttackCandidate {
        move_id: "grab".to_string(),
        frames,
        binding: AttackBinding {
            verb: AttackVerb::Grab,
            direction: AttackDir::Neutral,
        },
        legality: ActionLegality::Now,
    }
}

fn guarding(mut view: WorldView) -> WorldView {
    let foe = &mut view.actors[0];
    foe.shield_raised = true;
    foe.on_ground = true;
    foe.phase = BodyPhase::Shielding;
    view
}

/// A hold is worth most against a raised guard — the third leg of the
/// triangle `rollout.rs` already writes down, now visible to L2.
///
///  L2 is the layer that matters here: L3's rollout has known "grab beats
/// shield" since a shielding opponent made the whole kit worth zero, but
/// `attacks.first()` is what answers whenever L3 names nothing.
#[test]
fn a_hold_is_worth_more_against_a_guard_than_against_a_free_body() {
    let free = view_with(300.0, 340.0);
    let held = guarding(view_with(300.0, 340.0));

    let against_free = capture_value(&free.actors[0]);
    let against_guard = capture_value(&held.actors[0]);

    assert!(
        against_guard > against_free,
        "a grab is the genre's answer to a shield, but it priced a guarding \
         body at {against_guard} and a free one at {against_free}"
    );
    //  the zero floor: if BOTH were zero the comparison above would be
    // vacuous, and a policy that never fires is the state this replaced.
    assert!(
        against_guard > 0.0,
        "the guard term did not fire at all, so this test proves nothing"
    );
}

/// A body already in hitstun is the WRONG grab, and it is refused explicitly.
///
/// It is the case a naive "they cannot answer, so grab" rule scores HIGHEST —
/// they are maximally helpless — and it is the case where spending the grab's
/// startup trades a live combo for a hold.
#[test]
fn a_reeling_body_is_not_worth_grabbing() {
    let mut view = view_with(300.0, 340.0);
    view.actors[0].phase = BodyPhase::Hitstun;
    view.actors[0].damage_taken = 90;
    assert_eq!(
        capture_value(&view.actors[0]),
        0.0,
        "hitstun is the one state where helplessness must NOT read as grab value"
    );
}

/// Grab value is zero outside the body's authored grab reach, even against a
/// high-value guarding opponent.
#[test]
fn a_hold_is_never_worth_a_grab_the_body_cannot_reach() {
    let mut view = guarding(view_with(300.0, 410.0));
    view.actors[0].damage_taken = 140;

    let kit = [
        grab_candidate(42.0),
        // A poke that DOES cover the gap. It is the option the grab must not
        // outrank, and it is deliberately the weakest thing in the kit.
        candidate("poke", 0.1, 120.0),
    ];
    let opts = generate_options(
        Perceived::cheating(&view),
        Situation::Neutral,
        &kit,
        &UtilityWeights::v1(),
    );
    let best = opts.best_attack().expect("the kit offers something");
    assert_eq!(
        best.move_id, "poke",
        "a grab out at 110px with a 42px reach outranked a poke that covers the \
         gap — this is the reverted throw-damage pricing coming back"
    );
}

/// And the converse, so the guard above is not satisfied by a policy that
/// never fires: in reach and against a shield, the grab DOES win.
#[test]
fn in_reach_and_against_a_guard_the_grab_wins() {
    let view = guarding(view_with(300.0, 330.0));
    let kit = [grab_candidate(42.0), candidate("poke", 0.1, 40.0)];
    let opts = generate_options(
        Perceived::cheating(&view),
        Situation::Neutral,
        &kit,
        &UtilityWeights::v1(),
    );
    let best = opts.best_attack().expect("the kit offers something");
    assert_eq!(
        best.move_id, "grab",
        "a shielding opponent 30px away is the textbook grab and the scorer \
         picked `{}` instead",
        best.move_id
    );
}

/// The feature is zero for every move that is not a capture, asserted at the
/// scorer rather than at `capture_value`, because the call site is where it
/// could quietly start pricing ordinary swings.
#[test]
fn only_a_capture_carries_capture_value() {
    let view = guarding(view_with(300.0, 330.0));
    let kit = [grab_candidate(42.0), candidate("poke", 0.1, 40.0)];
    let opts = generate_options(
        Perceived::cheating(&view),
        Situation::Neutral,
        &kit,
        &UtilityWeights::v1(),
    );
    let mut saw_grab = false;
    for attack in &opts.attacks {
        if attack.move_id == "grab" {
            saw_grab = true;
            assert!(attack.features.capture_value > 0.0);
        } else {
            assert_eq!(
                attack.features.capture_value, 0.0,
                "`{}` is not a capture and was priced as one",
                attack.move_id
            );
        }
    }
    assert!(
        saw_grab,
        "the grab was filtered out, so this measured nothing"
    );
}

///  A HOLD ON AN AIRBORNE BODY IS WORTH NOTHING, because the rules refuse
/// to sell it.
///
/// `acquire_captures` skips any victim whose `ground.on_ground` is false, so a
/// grab thrown at a body in the air plays, costs its recovery and catches
/// nobody. This is deliberately asserted against the opponent's most
/// grab-attractive state otherwise — guarding and at high percent — so it
/// cannot pass by the other terms happening to be small.
#[test]
fn an_airborne_body_is_worth_nothing_to_hold() {
    let mut view = guarding(view_with(300.0, 330.0));
    view.actors[0].damage_taken = 140;

    // Grounded, this is the most valuable hold in the game.
    let grounded = capture_value(&view.actors[0]);
    assert!(
        grounded > 0.0,
        "the fixture is supposed to be the textbook grab; it priced at {grounded}"
    );

    view.actors[0].on_ground = false;
    assert_eq!(
        capture_value(&view.actors[0]),
        0.0,
        "a body in the air cannot be captured at all, so no state of it is worth \
         spending a grab on"
    );
}

// ── legality: can this action begin at all? ──────────────────────────────

///  AN ATTACK THE BODY CANNOT BEGIN IS NOT AN OPTION.
///
///  the sibling filter cannot catch it. "An attack that cannot REACH is not
/// an option" refuses a move that cannot touch the foe; this one is in reach and
/// still cannot happen.
#[test]
fn an_attack_the_body_cannot_begin_is_not_offered() {
    let view = view_with(300.0, 340.0);
    let mut blocked = candidate("smash", 0.1, 60.0);
    blocked.legality = ActionLegality::BlockedByPlayback;
    let free = candidate("jab", 0.1, 60.0);
    let kit = [blocked, free];

    let opts = generate_options(
        Perceived::cheating(&view),
        Situation::Neutral,
        &kit,
        &UtilityWeights::v1(),
    );

    //  the zero floor: a run that offered NOTHING would satisfy "the blocked
    // move is absent" while proving the filter deletes everything.
    assert!(
        !opts.attacks.is_empty(),
        "the whole kit was filtered out, so this measured nothing"
    );
    assert!(
        opts.attacks.iter().all(|a| a.move_id != "smash"),
        "a move the body cannot begin was offered anyway: {:?}",
        opts.attacks.iter().map(|a| &a.move_id).collect::<Vec<_>>()
    );
    assert!(
        opts.attacks.iter().any(|a| a.move_id == "jab"),
        "the startable move should still be there"
    );
}

/// Legality is a FILTER, not a penalty — asserted where the difference shows.
///
/// The blocked move here is the BEST option in the kit by every feature: it
/// reaches perfectly and the alternative barely reaches at all. Scoring it low
/// would still let it win, because `attacks.first()` always answers; only
/// removing it produces the right press.
#[test]
fn a_blocked_move_loses_even_when_it_is_the_best_one() {
    let view = view_with(300.0, 340.0);
    let mut best = candidate("perfect", 0.05, 40.0);
    best.legality = ActionLegality::BlockedByPlayback;
    // The poorer candidate must still reach; otherwise the sibling reachability
    // filter empties the kit and makes this test vacuous.
    let poor = candidate("stubby", 0.3, 20.0);
    let kit = [best, poor];

    let opts = generate_options(
        Perceived::cheating(&view),
        Situation::Neutral,
        &kit,
        &UtilityWeights::v1(),
    );
    assert_eq!(
        opts.best_attack().map(|a| a.move_id.as_str()),
        Some("stubby"),
        "the blocked move was the strongest and still must not be pressed"
    );
}

/// A lifting move the body cannot begin does not answer `Recovery` either.
///
/// A body past the blastzone has exactly one problem, which is what makes this
/// the tempting place to skip the check — but a route the press cannot take is
/// not a route.
#[test]
fn recovery_does_not_offer_a_lift_the_body_cannot_begin() {
    let mut blocked = lifting_candidate("blocked_lift", 500.0, 0.05);
    blocked.legality = ActionLegality::BlockedByPlayback;
    let free = lifting_candidate("free_lift", 100.0, 0.05);
    let kit = [blocked, free];

    let lifts = lifting_candidates(&kit);
    assert!(
        !lifts.is_empty(),
        "no lift survived at all, so this measured nothing"
    );
    assert!(
        lifts.iter().all(|c| c.move_id != "blocked_lift"),
        "the strongest lift cannot be started and was still offered"
    );
}

/// ⛔⛔ A FOE-RELATIVE VERB WITH NO FOE IS AN OPTION THE EMITTER CANNOT ACT ON.
///
/// `apply_movement` builds Approach/Retreat/Dash from the direction to
/// `nearest_hostile()` and emits a ZERO stick when there is none — so the brain
/// chose *"approach"*, pressed nothing, and kept choosing it. Measured on seed 0
/// of `ladder_rig --sweep-below`: the l5 partner, after its opponent died, held
/// `offered=[Approach, Retreat, Jump] chose=Some(Approach) emit_x=0.0` for the
/// rest of the bout.
///
/// ⭐ THE SAME ARGUMENT THE FILE ALREADY MAKES ABOUT `Jump`, which was offered
/// unconditionally until a body with an empty jump budget was handed "an option
/// pressing does nothing for" — L3 rolls the verb, the line goes nowhere, and
/// nowhere scores as SAFE.
#[test]
fn a_body_with_no_foe_is_offered_no_foe_relative_verb() {
    let mut view = view_with(300.0, 380.0);
    view.actors.clear();

    let kit: [AttackCandidate; 0] = [];
    let weights = UtilityWeights::default();
    let options = generate_options(
        Perceived::cheating(&view),
        Situation::Neutral,
        &kit,
        &weights,
    );
    let verbs: Vec<_> = options.movement.iter().map(|m| m.verb).collect();
    for verb in [
        MovementVerb::Approach,
        MovementVerb::Retreat,
        MovementVerb::Dash,
    ] {
        assert!(
            !verbs.contains(&verb),
            "{verb:?} needs a foe to point at, and there is none: {verbs:?}"
        );
    }

    // ⛔ THE PREMISE: with a foe, they ARE offered. Otherwise this passes on a
    // generator that offers nothing at all.
    let with_foe = generate_options(
        Perceived::cheating(&view_with(300.0, 380.0)),
        Situation::Neutral,
        &kit,
        &weights,
    );
    let with_foe: Vec<_> = with_foe.movement.iter().map(|m| m.verb).collect();
    assert!(
        with_foe.contains(&MovementVerb::Approach),
        "a body WITH a foe must still be offered Approach: {with_foe:?}"
    );
}
