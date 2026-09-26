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
        hazard: None,
        // `None` keeps every fixture's aim on `startup_s`, which is what the
        // lead read before this field existed. A fixture that means to test the
        // SPLIT states its own time.
        threat_live_at_s: None,
        //  the fixture's move is a FORWARD POKE, and now it says so. `reach`
        // is only the `+x` face of the authored volumes, so a fixture that set it
        // alone described a move with no hittable region at all once the scorer
        // started reading the region. This is the box a poke of that length
        // actually covers, which is what production derives from the volumes.
        coverage: (reach > 0.0).then(|| ambition_entity_catalog::MoveCoverage {
            min: (0.0, -12.0),
            max: (reach, 12.0),
        }),
        // This fixture authors no windbox.
        push_coverage: None,
        push_dir: None,
        max_damage: 1,
        max_knockback: 0.0,
        // ⚠ EMPTY, SO `kill_potential` IS ZERO FOR EVERY FIXTURE MOVE and the
        // arms below rank on the features they are about. `candidate_with_launch`
        // is what opts a fixture INTO being a finisher.
        launch: ambition_entity_catalog::LaunchEnvelope::default(),
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
        // A move nobody has landed lately, which is what every fixture here
        // means unless it says otherwise.
        wear: crate::brain::attack_kit::MoveWear::FRESH,
    }
}

/// The same candidate, with an authored launch that GROWS with the victim's
/// damage — what makes a move a finisher rather than a poke.
fn candidate_with_launch(id: &str, startup_s: f32, reach: f32, launch: f32) -> AttackCandidate {
    let mut c = candidate(id, startup_s, reach);
    c.frames.max_knockback = launch;
    // A flat finisher: all base, no percent term. `candidate_with_growth` is the
    // arm that separates the two halves of the launch law.
    c.frames.launch = ambition_entity_catalog::LaunchEnvelope::default().with_volume(launch, None);
    c
}

/// A candidate whose launch is authored as a `(base, growth)` LINE.
///
/// ⛔ What `candidate_with_launch` cannot express, and the shape the whole
/// percent mechanic is about: a move can be weaker than another against a fresh
/// opponent and stronger against a worn one.
fn candidate_with_growth(
    id: &str,
    startup_s: f32,
    reach: f32,
    base: f32,
    growth: f32,
) -> AttackCandidate {
    let mut c = candidate(id, startup_s, reach);
    c.frames.max_knockback = base;
    c.frames.launch =
        ambition_entity_catalog::LaunchEnvelope::default().with_volume(base, Some(growth));
    c
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

/// Kill potential rises with the VICTIM's damage — **and with the launch the
/// move carries**. In a smash-percent game a move's value is who it can end,
/// and a jab ends nobody at 150%.
///
/// ⛔⛤ THE SECOND HALF WAS MISSING AND MADE THE WHOLE WEIGHT DEAD. The
/// feature was `foe.damage_frac()` alone — the same number for every
/// candidate — and an attack's score is only ever compared with another
/// attack's, so `kill_potential` could not change a ranking at any rung. This
/// arm passed throughout: one move in the kit, nothing to out-rank.
#[test]
fn kill_potential_reads_the_victims_meter_and_the_moves_launch() {
    let w = UtilityWeights::v1();
    let mut v = view_with(300.0, 400.0);

    // A single launching move: the share is 1, so the feature is the meter.
    let kit = [candidate_with_launch("jab", 0.1, 100.0, 80.0)];
    v.actors[0].damage_taken = 0;
    let fresh = generate_options(Perceived::cheating(&v), Situation::Neutral, &kit, &w);
    v.actors[0].damage_taken = 90;
    let ripe = generate_options(Perceived::cheating(&v), Situation::Neutral, &kit, &w);
    assert!(ripe.best_attack().unwrap().score > fresh.best_attack().unwrap().score);
    assert_eq!(ripe.best_attack().unwrap().features.kill_potential, 0.9);

    // ⭐ AND THE HALF THE OLD ARM COULD NOT ASK, because it needs a SECOND
    // move: the same damaged foe, two moves that differ only in launch. The
    // weaker one earns a proportional share and not the meter.
    let kit = [
        candidate_with_launch("jab", 0.1, 100.0, 40.0),
        candidate_with_launch("smash", 0.1, 100.0, 160.0),
    ];
    let ranked = generate_options(Perceived::cheating(&v), Situation::Neutral, &kit, &w);
    let of = |id: &str| {
        ranked
            .attacks
            .iter()
            .find(|a| a.move_id == id)
            .expect("both moves reach")
            .features
            .kill_potential
    };
    assert_eq!(of("smash"), 0.9, "the kit's best launch earns the full meter");
    assert!(
        (of("jab") - 0.225).abs() < 1e-6,
        "a quarter of the launch earns a quarter of the meter, got {}",
        of("jab")
    );

    // ⛔ A MOVE THAT LAUNCHES NOTHING EARNS NOTHING, however hurt the foe is.
    // That is what keeps a set-knockback shove out of the kill question.
    let kit = [candidate("poke", 0.1, 100.0)];
    let ranked = generate_options(Perceived::cheating(&v), Situation::Neutral, &kit, &w);
    assert_eq!(ranked.best_attack().unwrap().features.kill_potential, 0.0);
}

/// ⛔⛤ **THE FINISHER CHANGES AS THE OPPONENT WEARS DOWN, AND A COLLAPSED
/// SCALAR RANKED IT BACKWARDS.**
///
/// The launch law is `base + growth * damage`. A summary that keeps only
/// `base` says the bigger base always finishes harder, which is false
/// everywhere past the two lines' crossing — and the roster authors the
/// crossing: the Pugnacious Polygon's forward smash is `(162, 3.25)` and his
/// up smash is `(158, 5.83)`, which swap at about 2 damage.
///
/// ⭐ The arm asks for the SWAP rather than for a magnitude, because a
/// magnitude is a claim about the weight and this is a claim about the order.
#[test]
fn a_low_base_high_growth_finisher_overtakes_a_flat_one_as_the_foe_wears_down() {
    let w = UtilityWeights::v1();
    let mut v = view_with(300.0, 400.0);
    // Deliberately his real numbers, so the arm fails if the roster stops
    // authoring a crossing at all.
    let kit = [
        candidate_with_growth("forward_smash", 0.1, 100.0, 162.0, 3.25),
        candidate_with_growth("up_smash", 0.1, 100.0, 158.0, 5.83),
    ];
    let share = |view: &crate::perception::WorldView, id: &str| {
        generate_options(Perceived::cheating(view), Situation::Neutral, &kit, &w)
            .attacks
            .iter()
            .find(|a| a.move_id == id)
            .expect("both moves reach")
            .features
            .kill_potential
    };

    // ⚠ THE PREMISE: at ZERO damage the meter is zero and both shares are zero,
    // so the comparison has to be made where the feature is alive at all. One
    // point of damage is where the forward smash is still ahead on the line.
    v.actors[0].damage_taken = 1;
    assert!(
        share(&v, "forward_smash") > share(&v, "up_smash"),
        "against a nearly fresh opponent the bigger BASE is the finisher"
    );

    v.actors[0].damage_taken = 90;
    assert!(
        share(&v, "up_smash") > share(&v, "forward_smash"),
        "against a worn opponent the steeper GROWTH is the finisher, and a \
         scorer that folded launch to its base could never say so"
    );
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

/// A kick that trails a shove, the shape [`authoring::wake`] builds: one
/// hittable boot and one windbox beyond it. The push ALWAYS reaches further —
/// that helper asserts it, so an enclosed wake cannot be authored.
fn waked_kick(boot: f32, dust: f32) -> AttackCandidate {
    let mut c = candidate("waked_kick", 0.12, boot);
    c.frames.coverage = Some(ambition_entity_catalog::MoveCoverage {
        min: (0.0, -12.0),
        max: (boot, 12.0),
    });
    c.frames.push_coverage = Some(ambition_entity_catalog::MoveCoverage {
        min: (boot, -12.0),
        max: (dust, 12.0),
    });
    c
}

/// ⛔⛤ A SHOVE IS NOT A REACH, AND THE MERGED READING PRICED A KICK BY ITS
/// DUST.
///
/// `MoveFrameData::coverage` was the union of every Active volume, windboxes
/// included, and [`authoring::wake`] ASSERTS that the push reaches further than
/// the hit — so for every waked move the only thing the brain knew about where
/// it could land was the DUST's extent. `goblin::dirt_kick` read as an 82px poke
/// whose boot stops at 48.
///
/// ⇒ `reach_fit` now scores the boot. At 70px — past the boot, inside the dust
/// — the move is priced as the near-miss it is, strictly below the fit the
/// merged reading gave it.
#[test]
fn a_waked_kick_is_priced_by_its_boot_and_not_by_its_dust() {
    let w = UtilityWeights::v1();
    let boot_only = [waked_kick(48.0, 82.0)];
    // The same kick as the merged reading described it: one volume out to the
    // dust, which is what the old union produced.
    let mut as_if_merged = candidate("waked_kick", 0.12, 82.0);
    as_if_merged.frames.coverage = Some(ambition_entity_catalog::MoveCoverage {
        min: (0.0, -12.0),
        max: (82.0, 12.0),
    });
    let merged = [as_if_merged];
    let fit = |kit: &[AttackCandidate]| {
        generate_options(
            Perceived::cheating(&view_with(0.0, 70.0)),
            Situation::Neutral,
            kit,
            &w,
        )
        .best_attack()
        .map(|a| a.features.reach_fit)
    };
    let boot = fit(&boot_only).expect("the kick is still on the menu");
    let dust = fit(&merged).expect("the control kit offers the same move");
    assert!(
        boot < dust,
        "the boot cannot reach 70px and the dust can, so pricing by the boot \
         must be strictly worse: boot={boot} dust={dust}"
    );
    // ⚠ AND NOT ZERO, which would make this pass for the wrong reason. The
    // near-miss still ranks — that is what makes a brain commit to a spacing —
    // and whether a near-miss should be OFFERED at all is a separate question
    // this test deliberately does not answer.
    assert!(boot > 0.0, "a near-miss is still priced: {boot}");
}

/// The other foot of the same split: a move that can ONLY push still has a
/// range. Splitting the datum made a gust's `coverage` `None`, and the
/// "lands no volume" arm — written for buffs and summons — would then have
/// offered it from anywhere on the stage.
#[test]
fn a_move_that_only_shoves_is_offered_exactly_where_it_can_shove() {
    let mut gust = candidate("gust", 0.1, 0.0);
    gust.frames.coverage = None;
    gust.frames.push_coverage = Some(ambition_entity_catalog::MoveCoverage {
        min: (0.0, -12.0),
        max: (60.0, 12.0),
    });
    gust.frames.max_damage = 0;
    let kit = [gust];
    let w = UtilityWeights::v1();
    assert_eq!(
        generate_options(
            Perceived::cheating(&view_with(0.0, 40.0)),
            Situation::Neutral,
            &kit,
            &w,
        )
        .best_attack()
        .map(|a| a.move_id.as_str()),
        Some("gust"),
        "inside the push region"
    );
    assert!(
        generate_options(
            Perceived::cheating(&view_with(0.0, 300.0)),
            Situation::Neutral,
            &kit,
            &w,
        )
        .attacks
        .is_empty(),
        "300px away it pushes nobody"
    );
}

/// **A SHOVE IS WORTH WHERE IT PUSHES SOMEBODY, AND THAT IS ENOUGH TO CHOOSE
/// IT.**
///
/// ⛔⛤ Being OFFERED is not being CHOSEN, and the arm above only proves the
/// first. A pure windbox has `coverage: None` — so `reach_fit` is zero — and
/// `damage: 0` — so `expected_payoff` is zero. It was admitted exactly where
/// it can shove and then priced as though shoving were worth nothing, which
/// makes it a move the CPU is allowed to pick and never has a reason to.
///
/// ⭐ THE TWO READINGS DIFFER IN ONE THING: WHERE THE FOE IS STANDING. Same
/// kit, same 55px gap, same everything — and the gust wins beside the blast
/// line while the jab wins at centre stage. That is the feature's whole
/// content: a gust is a spacing tool in the middle and a kill at the edge.
///
/// ⚠ THE JAB IS THE CONTROL AND IT HAS TO BE A NEAR MISS. A jab that covers
/// the gap outscores the gust at both ends (a hit is worth more than a push,
/// and `reach_fit`'s weight says so), which would make this arm a statement
/// about the weights rather than about the position.
///
/// ⛔⛤ **AND IT IS A STATEMENT ABOUT THE SHAPE, NOT ABOUT ANY CHARACTER.** Two
/// moves is a fixture, not a kit: measured 2026-09-19 against the eleven
/// candidates the Officer's real table yields, the gust's rival beside the
/// ledge is his `tilt_forward` and not a jab, and the weight this arm passed at
/// left the feature inert on the shipped roster. What a character actually
/// presses is pinned where the character lives —
/// `he_uses_the_gust` in `game/ambition_content/src/officer_moveset.rs`.
#[test]
fn a_shove_outranks_a_near_miss_at_the_ledge_and_not_at_centre() {
    // 40px of reach against a 55px gap: admitted, and poorly.
    let jab = candidate("jab", 0.1, 40.0);
    let mut gust = candidate("gust", 0.1, 0.0);
    gust.frames.coverage = None;
    gust.frames.max_damage = 0;
    gust.frames.push_coverage = Some(ambition_entity_catalog::MoveCoverage {
        min: (0.0, -12.0),
        max: (60.0, 12.0),
    });
    let kit = [jab, gust];
    let w = UtilityWeights::v1();
    let best = |me_x: f32, foe_x: f32| {
        generate_options(
            Perceived::cheating(&view_with(me_x, foe_x)),
            Situation::Neutral,
            &kit,
            &w,
        )
        .best_attack()
        .map(|a| a.move_id.clone())
    };

    // The stage spans x 0..800. A foe at 745 is 55px from the blast line.
    assert_eq!(
        best(690.0, 745.0).as_deref(),
        Some("gust"),
        "beside the ledge the CPU still reached for a jab it can barely touch \
         them with, instead of the push that sends them off"
    );
    // The same gap, in the middle, where a push buys nothing.
    assert_eq!(
        best(345.0, 400.0).as_deref(),
        Some("jab"),
        "at centre stage a shove was preferred to a hit, which prices a gust as \
         though every push were a kill"
    );
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

/// The same, with no hittable region at all — an up-B that is purely a way
/// home. Two of the shipped roster's fifteen self-launchers are this shape.
fn hitless_lifting_candidate(id: &str, lift_speed: f32) -> AttackCandidate {
    let mut c = lifting_candidate(id, lift_speed, 0.1);
    c.frames.coverage = None;
    c.frames.reach = 0.0;
    c.frames.max_damage = 0;
    c
}

/// ⛔⛤ **A MOVE THAT TOUCHES NOTHING AND THROWS ME IN THE AIR IS NOT AN
/// ATTACK OPTION, AND IT USED TO BE OFFERED AT EVERY RANGE.**
///
/// The admission rule sorts candidates by what they touch, and its third arm
/// — *"touches nothing — a buff, a summon, a pure-motion move"* — admitted a
/// hitless recovery too. Nothing it could MISS, so nothing filtered it, and
/// with `reach_fit` and `expected_payoff` both zero it was priced on
/// `frame_advantage` and `stage_risk` alone. Whenever the gap grew past the
/// rest of the kit's reach it was what remained — and pressing it widens the
/// gap, so the next decision finds the same world one recovery later.
///
/// ⚠ MEASURED, NOT REASONED: on the 21-fighter grid sweep the medic threw
/// `medic_rescue_lift` 48 times in 91 starts and dealt 39%.
#[test]
fn a_hitless_recovery_is_not_on_the_neutral_menu_and_is_the_whole_recovery_menu() {
    let kit = [
        candidate("jab", 0.1, 40.0),
        hitless_lifting_candidate("up_b", 900.0),
    ];
    let w = UtilityWeights::v1();

    // ⚠ A RANGE WHERE THE JAB IS STILL OFFERED — not the longest one. The
    // rule is "not while there is something else", so a gap with an empty menu
    // would be the wrong question and the arm below asks it separately.
    let near = generate_options(
        Perceived::cheating(&view_with(300.0, 355.0)),
        Situation::Neutral,
        &kit,
        &w,
    );
    assert!(
        near.attacks.iter().any(|a| a.move_id == "jab"),
        "the premise: the jab must be on the menu, or excluding the recovery \
         proves nothing about preferring the alternative"
    );
    assert!(
        !near.attacks.iter().any(|a| a.move_id == "up_b"),
        "a hitless recovery sits on the neutral menu beside a move that can \
         actually hit"
    );

    // ⛔⛤ **AND IT DOES NOT COME BACK AS A LAST RESORT EITHER, WHICH THE FIRST
    // REPAIR DID.** An empty attack menu is not a body that cannot act —
    // movement is chosen separately and already offers `Approach` — so the
    // move belongs on the MOTION list, where it is judged by where it goes.
    // Here the foe is level with this body and the launch is straight up, so
    // the motion serves nothing and is withheld.
    let lone = [hitless_lifting_candidate("up_b", 900.0)];
    let level_foe = generate_options(
        Perceived::cheating(&view_with(300.0, 600.0)),
        Situation::Neutral,
        &lone,
        &w,
    );
    assert!(
        level_foe.attacks.is_empty(),
        "a hitless launcher is not an attack even when it is the only move in \
         the kit"
    );
    assert_eq!(
        level_foe
            .motions
            .iter()
            .map(|m| m.move_id.as_str())
            .collect::<Vec<_>>(),
        vec!["up_b"],
        "it is still OFFERED — as the motion it is"
    );
    assert!(
        level_foe.best_motion().is_none(),
        "…and withheld, because a launch straight up carries this body away \
         from an opponent standing level with it. Pressing it is what put the \
         medic in a loop she could not leave: got {:?}",
        level_foe.motions.first().map(|m| m.score)
    );

    // ⭐ AND IT IS NOT GONE FROM THE BRAIN — the recovery lens owns it, so a
    // body that needs to come home still reaches for it. Without this arm the
    // one above is satisfied by deleting the move from the kit.
    let recovering = generate_options(
        Perceived::cheating(&view_with(300.0, 600.0)),
        Situation::Recovery,
        &kit,
        &w,
    );
    assert_eq!(
        recovering
            .attacks
            .iter()
            .map(|a| a.move_id.as_str())
            .collect::<Vec<_>>(),
        vec!["up_b"],
        "the recovery situation does not offer the move that lifts the body"
    );

    // ⛔⛤ **A SUMMON IS NOT AN OFFER TO THE OPPONENT, AND FOR ONE DAY THIS
    // TEST SAID IT WAS.** The claim was that `call_the_shark` puts 650px of
    // ridable authority on the field for five seconds, which is the kind of
    // offer a bolt makes — so `frame_data` folded a `SustainedAuthority`'s
    // reach into `hazard_reach` and the hazard arm admitted it.
    //
    // ⛔ `call_the_shark`'S AUTHORING REFUTES IT IN ITS OWN WORDS: *"There is
    // no hurtbox on this up-b, it's purely a mobility special"*; it is a
    // `hitless_special` rather than a strike carrying an empty volume list;
    // the summoned shark is `Neutral` and deals no contact damage; and its
    // `reach` is authored as HALF THE RIDE'S STRAIGHT-LINE DISTANCE, which is
    // how far the admiral can GO. Recovery authority answers *"what movement
    // does this give me"* and hazard reach answers *"what can this do to
    // them"*. A future summon that does both states its offensive half as an
    // effect, like every other hazard in the table.
    //
    // ⇒ **BOTH CARRYING ROUTES ARE MOTION.** `travel_of` reads
    // `RecoveryRoute::carry`, which is the distance a summon and a teleport
    // already publish, and the motion score is a ratio against the gap rather
    // than a share of the kit's fastest SPEED — which is exactly the
    // "what does that ratio mean" question that held this slice while a
    // distance and a velocity had to share one maximum.
    let carrying = |id: &str, route: ambition_entity_catalog::RecoveryRoute| {
        let mut c = candidate(id, 0.3, 0.0);
        c.frames.coverage = None;
        c.frames.recovery_route = route;
        c
    };
    let summon = carrying(
        "summon",
        ambition_entity_catalog::RecoveryRoute::SustainedAuthority {
            seconds: 5.0,
            reach: 650.0,
        },
    );
    let blink = carrying(
        "blink",
        ambition_entity_catalog::RecoveryRoute::Teleport { distance: 210.0 },
    );
    let carriers = generate_options(
        // `view_with(me_x, foe_x)` — 650px of gap, which is exactly where the
        // ride arrives.
        Perceived::cheating(&view_with(300.0, 950.0)),
        Situation::Neutral,
        &[summon, blink],
        &w,
    );
    assert!(
        carriers.attacks.is_empty(),
        "a move that carries its owner and touches nobody was offered as an \
         ATTACK. Measured across 21 mirror matches of 3600 ticks: a pressed \
         move owns the body through its recovery and a body in a move does \
         not walk, so `player_robot_v3` threw `phase_shift` 157 times — one \
         every 23 ticks, the move's whole duration — at a mean gap of 223px \
         for 0% damage, against 103% with an ordinary menu. Offered: {:?}",
        carriers
            .attacks
            .iter()
            .map(|a| a.move_id.as_str())
            .collect::<Vec<_>>(),
    );
    // ⭐ AND THE RIDE IS ON THE LIST THAT PRICES TRAVEL, worth 1 because it
    // arrives: a rider steers with the control stick, so the whole 650px
    // counts toward whatever the objective is.
    let priced: Vec<(&str, f32)> = carriers
        .motions
        .iter()
        .map(|m| (m.move_id.as_str(), m.score))
        .collect();
    assert_eq!(
        priced.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        vec!["summon"],
        "the summoned ride is not on the motion list, so the admiral whose \
         only way to close a 650px gap is his shark is offered nothing: \
         {priced:?}"
    );
    assert!(
        (priced[0].1 - 1.0).abs() < 1e-4,
        "a 650px ride into a 650px gap arrives exactly, which is the peak of \
         the score: {priced:?}"
    );

    // ⛔⛤ **AND THE TELEPORT IS ON NEITHER LIST, WHICH IS A MEASURED REFUSAL
    // AND NOT AN UNFINISHED ONE.** It was put on this list and taken off
    // again the same day: a teleport goes where the move AIMS — `phase_shift`
    // is authored *"aimed, like every recovery: the stick, then straight up"*
    // — so pricing its `distance` as travel toward the opponent moves the
    // robot 210px upward and leaves the gap exactly where it was. Two
    // different overshoot prices, one outcome: **27%/22% on 9 distinct with
    // `phase_shift×186`**, then **32%/39% on 11 with ×54**, against
    // **225%/223% on 19** with the move offered nowhere. ⇒ The destination is
    // a fact about the move (`TeleportParams` carries `behind_nearest_foe`,
    // `behind_gap` and an aim) and none of it reaches `MoveFrameData`; that is
    // the resolved-action-offer slice, not a price.
    assert!(
        !priced.iter().any(|(id, _)| *id == "blink"),
        "a teleport is priced as travel toward the opponent again: {priced:?}"
    );

    // ⚠ AND A LIFTER THAT HITS IS UNTOUCHED, which is 13 of the roster's 15:
    // an uppercut is an ordinary attack that also rises, and `reach_fit`
    // already prices it.
    let hitting = [lifting_candidate("uppercut", 745.0, 0.1)];
    let close = generate_options(
        Perceived::cheating(&view_with(300.0, 340.0)),
        Situation::Neutral,
        &hitting,
        &w,
    );
    assert_eq!(
        close.best_attack().map(|a| a.move_id.as_str()),
        Some("uppercut"),
        "excluding hitless recoveries also removed one that hits"
    );
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
        displacement_value: 0.0,
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
            // This fixture authors no windbox.
            push_coverage: None,
            push_dir: None,
            max_damage: 20,
            ..frames(0.25, 100.0, 0.4)
        },
        binding: AttackBinding {
            verb: AttackVerb::Smash,
            direction: AttackDir::Forward,
        },
        legality: ActionLegality::Now,
        wear: crate::brain::attack_kit::MoveWear::FRESH,
    };
    let jab = AttackCandidate {
        move_id: "jab".to_string(),
        binding: AttackBinding {
            verb: AttackVerb::Basic,
            direction: AttackDir::Forward,
        },
        legality: ActionLegality::Now,
        wear: crate::brain::attack_kit::MoveWear::FRESH,
        frames: MoveFrameData {
            // This fixture authors no windbox.
            push_coverage: None,
            push_dir: None,
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

/// ⛔⛔ **THE LEDGE PENALTY SCALES WITH THE BODY'S WIDTH, so two fighters on the
/// same tile disagree about whether closing is safe.** `walks_off` is
/// `floor_ahead(toward) < half_extent.x * 2.0`, so a WIDER body reads "approach
/// walks me off" from further back and retreats where a narrower one advances.
///
/// ⭐ WHY THIS IS WORTH PINNING RATHER THAN A CURIOSITY. It is the only
/// fighter-varying term in movement scoring, and CPU engagement varies enormously
/// by fighter: measured 2026-09-10 on the duel harness, two CPUs of one shipped
/// fighter spend 33% of a match within attack range while two of another spend
/// 11%, and their damage rates track that almost exactly. This is the first place
/// to look for why — a body's WIDTH silently changes how far it is willing to
/// chase — and until now nothing said the dependence existed.
///
/// ⚠ IT IS NOT AN ASSERTION THAT THE SCALING IS WRONG. A wider body genuinely
/// needs more floor. What the guard fixes is that the dependence is DELIBERATE
/// and visible, so a future change to body widths cannot quietly re-tune every
/// fighter's willingness to approach.
#[test]
fn a_wider_body_refuses_an_approach_a_narrower_one_takes() {
    // Both stand at the same spot with the same foe, on a platform that ends
    // 40px to the right. Only the half-extent differs.
    let narrow = {
        let mut v = on_a_platform(360.0, 500.0, (100.0, 400.0));
        v.self_view.half_extent = ae::Vec2::new(12.0, 24.0);
        v
    };
    let wide = {
        let mut v = on_a_platform(360.0, 500.0, (100.0, 400.0));
        v.self_view.half_extent = ae::Vec2::new(24.0, 24.0);
        v
    };

    let weights = UtilityWeights::default();
    let narrow_opts =
        generate_options(Perceived::cheating(&narrow), Situation::Neutral, &[], &weights);
    let wide_opts =
        generate_options(Perceived::cheating(&wide), Situation::Neutral, &[], &weights);

    // 40px of floor ahead: the narrow body needs 24 and advances; the wide body
    // needs 48 and does not.
    assert_eq!(
        score_of(&narrow_opts, MovementVerb::Approach),
        0.5,
        "a 12px-half-extent body has 40px of floor ahead — more than the 24 it \
         needs — and was penalised anyway"
    );
    assert!(
        score_of(&wide_opts, MovementVerb::Approach)
            < score_of(&wide_opts, MovementVerb::Retreat),
        "a 24px-half-extent body needs 48px of floor and has 40, so closing \
         should lose to backing away — the ledge rule is not reading its width"
    );

    // ⭐ THE CONTROL, and without it this passes on a rule that penalises every
    // approach regardless of width. Same two bodies, mid-platform: both advance.
    for half_x in [12.0f32, 24.0] {
        let mut safe = on_a_platform(250.0, 500.0, (100.0, 400.0));
        safe.self_view.half_extent = ae::Vec2::new(half_x, 24.0);
        let opts =
            generate_options(Perceived::cheating(&safe), Situation::Neutral, &[], &weights);
        assert_eq!(
            score_of(&opts, MovementVerb::Approach),
            0.5,
            "a {half_x}px-half-extent body in the MIDDLE of the platform was \
             penalised, so the rule is not about the ledge at all"
        );
    }
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
/// ⛔⛤ **AND A ZERO-REACH MOVE USED TO STAY, ON THE REASONING THAT REACH IS NOT
/// ITS QUESTION.** It is not — and neither is any other question this scorer
/// can ask: with no `reach_fit`, no `expected_payoff` and no `kill_potential`,
/// a buff scores `frame_advantage` minus `stage_risk`, so the moment the gap
/// rule above started refusing swings that cannot land, the fast safe move that
/// does NOTHING became what a fighter pressed at range. Measured on the grid,
/// 2026-09-20: five of the seven fighters that regressed answered with a
/// counter or a buff as their most-thrown move, and their mean gap grew.
///
/// ⭐ SO THE QUESTION IS *"DOES IT OFFER THE OPPONENT ANYTHING"*, and a
/// launcher is the arm that keeps this from being *"delete the class"*: it
/// lands no volume either, and it reaches furthest of anything in a kit.
#[test]
fn an_attack_that_cannot_span_the_gap_is_not_offered() {
    let jab = candidate("jab", 0.08, 40.0);
    let mut buff = candidate("buff", 0.2, 0.0);
    buff.frames.max_damage = 0;
    // A launcher: no volume on the body, and a hazard that crosses the stage.
    let mut bolt = candidate("bolt", 0.2, 0.0);
    bolt.frames.max_damage = 0;
    // Placed and live immediately, so the whole of its 700px is available the
    // instant it exists — this arm is about the REACH rule and a flight time
    // would put a second variable in it.
    bolt.frames.hazard = Some(ambition_entity_catalog::MoveHazard::Spawned {
        travel: ambition_entity_catalog::ThreatTravel::Placed {
            reach: 700.0,
            detonates_by_s: 0.0,
        },
        // This arm is about the ADMISSION rule, so the hazard deals what the
        // candidate's volumes do — nothing. A damage here would price the
        // bolt above the jab and change which move wins, which is a different
        // question than whether it is offered at all.
        damage: 0,
    });
    let kit = vec![jab, buff, bolt];
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
        !offered.attacks.iter().any(|a| a.move_id == "buff"),
        "a move that lands no volume, shoves nobody and carries nobody is still \
         offered at a 600px gap — and with the jab correctly gone it is the \
         whole menu, so `attacks.first()` presses it every decision"
    );
    // ⭐ THE ANTI-VACUITY HALF. Without it the arm above is satisfied by a
    // filter that drops every reachless move, which would take the launcher
    // with it — the one move in the kit that really does reach 600px.
    assert!(
        offered.attacks.iter().any(|a| a.move_id == "bolt"),
        "a launcher whose hazard crosses 700px is not offered at a 600px gap, \
         so the filter is reading `coverage: None` as 'reaches nowhere' again"
    );
    // And it is not offered from beyond what its hazard covers either.
    let very_far = view_with(300.0, 1200.0);
    let offered = generate_options(
        crate::perception::Perceived::cheating(&very_far),
        Situation::Neutral,
        &kit,
        &weights,
    );
    assert!(
        !offered.attacks.iter().any(|a| a.move_id == "bolt"),
        "the launcher is offered at 900px, past the 700px its hazard travels — \
         a projectile's reach is a reach, not a licence"
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
        wear: crate::brain::attack_kit::MoveWear::FRESH,
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
/// The blocked move here is the BEST option in the kit by every feature: both
/// reach the same place and it gets there six times faster. Scoring it low
/// would still let it win, because `attacks.first()` always answers; only
/// removing it produces the right press.
#[test]
fn a_blocked_move_loses_even_when_it_is_the_best_one() {
    let view = view_with(300.0, 340.0);
    let mut best = candidate("perfect", 0.05, 40.0);
    best.legality = ActionLegality::BlockedByPlayback;
    // ⚠ THE POORER CANDIDATE MUST ACTUALLY COVER THE FOE, or the sibling
    // admission filter empties the kit and this test measures nothing. It used
    // to reach 20px at a 40px gap and survive on `reach_fit`'s tolerance band,
    // which is the reading `covers` exists to stop being an admission.
    let poor = candidate("stubby", 0.3, 40.0);
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

/// ⭐⭐ EVERY WEIGHT MUST BE ABLE TO CHANGE A SCORE — the guard against a knob
/// that is authored, tuned, and silently ignored.
///
/// ⛔ **THIS EXISTS BECAUSE ONE ALREADY WAS.** `read_weight` is authored on all
/// nine rungs of the shipped ladder, rising 0.0 → 0.9, and reaches the fighter
/// only through a rollout the shipped rows disable — so it has never changed a
/// decision in the shipped game. Nothing failed, because nothing compares an
/// authored knob against its own effect. See `docs/planning/engine/fighter-brain.md`.
///
/// ⇒ The failure mode this catches is the cheap and likely one: a field added to
/// [`UtilityWeights`] and forgotten in [`Features::dot`]. That field would
/// deserialize from the `.ron`, appear in every profile, be tuned by whoever
/// authored the rungs, and do nothing at all.
///
/// ⚠ It asserts REACHABILITY, not usefulness. A weight can pass this and still
/// never matter in a real matchup, because its feature is zero in the situations
/// fighters actually reach — `kill_potential` and `stage_risk` are exactly that
/// today, and the difference is recorded rather than conflated.
#[test]
fn every_utility_weight_can_change_a_score() {
    // Every feature non-zero, so no weight is excused by a zero multiplicand.
    // ⛔ Deliberately NOT `Features::default()`: a default of all zeroes would
    // make the dot product zero for every weight and the test would pass by
    // measuring nothing.
    let features = Features {
        reach_fit: 0.7,
        frame_advantage: 0.5,
        kill_potential: 0.3,
        stage_risk: 0.9,
        expected_payoff: 0.4,
        capture_value: 0.6,
        displacement_value: 0.8,
    };
    let base = UtilityWeights::v1();
    let baseline = features.dot(&base);

    // Each entry perturbs ONE field. Adding a field to `UtilityWeights` without
    // adding it here leaves this list short, which the count assertion below
    // turns into a failure rather than a silent gap.
    let perturbations: Vec<(&str, UtilityWeights)> = vec![
        ("reach_fit", UtilityWeights { reach_fit: base.reach_fit + 1.0, ..base }),
        ("frame_advantage", UtilityWeights { frame_advantage: base.frame_advantage + 1.0, ..base }),
        ("kill_potential", UtilityWeights { kill_potential: base.kill_potential + 1.0, ..base }),
        ("stage_risk", UtilityWeights { stage_risk: base.stage_risk + 1.0, ..base }),
        ("expected_payoff", UtilityWeights { expected_payoff: base.expected_payoff + 1.0, ..base }),
        ("capture_value", UtilityWeights { capture_value: base.capture_value + 1.0, ..base }),
        ("displacement_value", UtilityWeights { displacement_value: base.displacement_value + 1.0, ..base }),
    ];

    for (name, weights) in &perturbations {
        let moved = features.dot(weights);
        assert_ne!(
            moved, baseline,
            "changing `{name}` did not change the score, so it is a knob that can \
             be authored and tuned and will never affect a decision — the defect \
             `read_weight` already has. Add it to `Features::dot`."
        );
    }

    // ⛔ THE HALF THAT CATCHES THE NEXT ONE. The loop above only checks fields
    // somebody remembered to list; a NEW field is caught by this count, which
    // fails the moment the struct grows and the list does not.
    let field_count = {
        // One entry per field of `UtilityWeights`, destructured so the compiler
        // enforces it: adding a field makes this line stop compiling, which is a
        // louder failure than a stale number.
        let UtilityWeights {
            reach_fit: _,
            frame_advantage: _,
            kill_potential: _,
            stage_risk: _,
            expected_payoff: _,
            capture_value: _,
            displacement_value: _,
        } = base;
        7
    };
    assert_eq!(
        perturbations.len(),
        field_count,
        "the perturbation list has drifted from the struct's fields"
    );
}

/// ⛔⛤ **A LOW CEILING IS NOT A SIDE BLAST LINE, AND THE SHOVE FEATURE COULD
/// NOT TELL THEM APART.**
///
/// `StageView::distance_to_edge` is the minimum over all FOUR sides, so a body
/// standing in the middle of the stage but near the top read as maximally
/// edge-pressured. Multiplied by a left/right sign, that paid a sideways gust
/// almost full ledge value with both side walls a stage away.
///
/// ⭐ The arm is a COMPARISON against the same push at the same height by the
/// side wall, because an absolute number here would only re-state whatever the
/// normalisation happens to be.
#[test]
fn a_foe_under_the_ceiling_is_not_near_the_side_blast_line_a_shove_threatens() {
    let stage = stage();
    let basis = ae::AccelerationFrame::new(ae::Vec2::new(0.0, 1.0));
    let pressure_at = |x: f32, y: f32| {
        PushGeometry {
            basis,
            facing: 1.0,
            stage: &stage,
            at: ae::Vec2::new(x, y),
        }
        // A pure sideways shove, which is what the gust authors.
        .pressure(1.0, 0.0)
    };

    // Mid-stage horizontally, a hair under the ceiling. Nothing about a
    // sideways push is urgent here.
    let under_the_ceiling = pressure_at(400.0, 5.0);
    // The same height, against the right wall, where it very much is.
    let at_the_wall = pressure_at(795.0, 5.0);
    assert!(
        at_the_wall > under_the_ceiling + 0.5,
        "a shove at the wall must be worth far more than the same shove under \
         the ceiling — wall {at_the_wall}, ceiling {under_the_ceiling}"
    );
    assert!(
        under_the_ceiling < 0.25,
        "the ceiling is not the blast line this push sends them through, got \
         {under_the_ceiling}"
    );

    // ⭐ AND THE SIGN IS STILL THERE: shoving the same cornered body INBOARD is
    // a rescue, so it is worth nothing at all.
    let inboard = PushGeometry {
        basis,
        facing: -1.0,
        stage: &stage,
        at: ae::Vec2::new(795.0, 300.0),
    }
    .pressure(1.0, 0.0);
    assert!(
        inboard < at_the_wall,
        "pushing a cornered body back toward centre is not ledge control"
    );
}

/// ⭐⭐ **AND IT SURVIVES SIDEWAYS GRAVITY, WHICH THE SIGNED VERSION DID NOT.**
///
/// The old feature took its outward direction from WORLD `x` while `push_dir`
/// is body-local, and called them the same frame. On a stage the body stands
/// sideways on they are ninety degrees apart, so a shove aimed at the near
/// blast line was priced as one aimed along it.
#[test]
fn a_shove_is_priced_in_the_bodys_own_frame_when_gravity_points_sideways() {
    let stage = stage();
    // Gravity toward world `+x`: the body's feet point right, so its local
    // forward `+x` is world `-y` (up the screen).
    let basis = ae::AccelerationFrame::new(ae::Vec2::new(1.0, 0.0));
    let pressure_at = |x: f32, y: f32| {
        PushGeometry {
            basis,
            facing: 1.0,
            stage: &stage,
            at: ae::Vec2::new(x, y),
        }
        .pressure(1.0, 0.0)
    };

    // Under this gravity a forward shove travels along world `-y`, so the
    // victim who is ABOUT to be pushed out is the one near the TOP.
    let near_the_top = pressure_at(400.0, 5.0);
    let near_the_side = pressure_at(795.0, 300.0);
    assert!(
        near_the_top > near_the_side + 0.5,
        "with gravity sideways, forward is up — top {near_the_top}, side {near_the_side}"
    );
}

/// ⭐⭐ **THE SAME MOVE IS A TRAP FOR ONE FIGHTER AND AN APPROACH FOR ANOTHER,
/// AND WHAT SEPARATES THEM IS WHERE THE OPPONENT IS.**
///
/// The grid sweep found both, 2026-09-19. The medic threw `medic_rescue_lift`
/// 48 times in 91 starts for 39% damage with `medic_tourniquet` unused at
/// 90px; blanket-excluding the same move shape cost Emmy 180 points of damage
/// and grew her gap from 82 to 123, because for her it was the way in. A rule
/// that names either fighter is a cheat. Asking whether the displacement
/// points at the opponent answers both.
#[test]
fn a_launch_is_pressed_at_a_foe_above_and_withheld_at_one_alongside() {
    let w = UtilityWeights::v1();
    let kit = [hitless_lifting_candidate("up_b", 900.0)];

    // Level with this body, a long way off: the launch goes nowhere useful.
    let alongside = generate_options(
        Perceived::cheating(&view_with(300.0, 600.0)),
        Situation::Neutral,
        &kit,
        &w,
    );
    assert!(alongside.best_motion().is_none());

    // ⭐ THE SAME KIT, THE SAME GAP, THE FOE MOVED UPSTAIRS. `+y` is toward
    // this body's feet, so a foe overhead is a NEGATIVE local `y` — the
    // direction a lift travels.
    //
    // ⚠ **450px, WHICH IS WHERE THIS LAUNCH ARRIVES** (900px/s for the
    // fixture's 0.5s move). The height used to be 280px and was arbitrary,
    // because the score contained no LENGTH and only the direction mattered;
    // now that a motion is priced against the gap it actually has to cross,
    // the distance is load-bearing and the arm states one where the move does
    // its job. The ALONGSIDE control is untouched, so the flip this test is
    // about is still attributed to where the opponent is.
    let mut overhead = view_with(300.0, 300.0);
    overhead.actors[0].pos.y = 300.0 - 450.0;
    let above = generate_options(
        Perceived::cheating(&overhead),
        Situation::Neutral,
        &kit,
        &w,
    );
    let pressed = above
        .best_motion()
        .expect("a launch toward a foe overhead is worth pressing");
    assert_eq!(pressed.move_id, "up_b");

    // ⚠ AND THE PREMISE: the two worlds differ ONLY in where the foe stands,
    // so the flip cannot be attributed to the kit or the weights.
    assert_eq!(above.motions.len(), alongside.motions.len());
    assert!(
        above.motions[0].score > alongside.motions[0].score,
        "above {} vs alongside {}",
        above.motions[0].score,
        alongside.motions[0].score
    );
}

/// ⛔⛤ **A BURST COVERS THE GROUND IT COVERS AFTER IT FIRES, AND THE PRICE
/// COUNTED THE WINDUP TOO — REVIEWED 2026-09-20.**
///
/// `travel_of` multiplied the commanded speed by the WHOLE move duration, so a
/// move whose impulse arrives a fifth of the way in was credited with a fifth
/// more distance than the body ever travels. `medic_rescue_lift` — `(34,
/// -905)` at 0.12s of a 0.48s move — was priced at 435px against a real 326px,
/// which against a 700px gap is 0.62 (pressed) versus 0.47 (not).
///
/// ⚠ **THE ARM IS A FLIP, NOT A THRESHOLD, BECAUSE THE TENT IS WIDE.** The
/// score is `1 - |travelled/gap - 1|`, so both the true distance and the
/// inflated one clear `MOTION_WORTH_PRESSING` at most gaps; what the defect
/// changes is WHICH gap this move is best at, which is the thing the option
/// layer actually spends. So the two probes are the two candidate answers, and
/// the question is which one peaks.
#[test]
fn a_burst_covers_ground_only_after_its_impulse_fires() {
    let w = UtilityWeights::v1();
    // 900px/s, fired at 0.1s of a 0.5s move: 360px of travel, and 450px if the
    // windup is counted as flight.
    let kit = [hitless_lifting_candidate("up_b", 900.0)];
    let overhead_by = |above: f32| {
        let mut view = view_with(300.0, 300.0);
        view.actors[0].pos.y = 300.0 - above;
        generate_options(Perceived::cheating(&view), Situation::Neutral, &kit, &w)
            .motions
            .first()
            .map(|m| m.score)
            .expect("the launch is not on the motion list at all")
    };

    let where_the_burst_lands = overhead_by(360.0);
    let where_the_whole_move_would_land = overhead_by(450.0);
    assert!(
        where_the_burst_lands > where_the_whole_move_would_land,
        "this body arrives at 360px and the price says it arrives at 450px: \
         {where_the_burst_lands} against {where_the_whole_move_would_land}"
    );
    // ⭐ AND IT IS THE PEAK, not merely the larger of two — without this the
    // arm passes for any rule that shrinks the distance, including one that
    // shrinks it too far.
    assert!(
        (where_the_burst_lands - 1.0).abs() < 1e-3,
        "a burst that exactly covers the gap is the top of the tent: \
         {where_the_burst_lands}"
    );
}

/// ⛔⛤ **THE SAME LAUNCH AT THREE DIFFERENT DISTANCES IS THREE DIFFERENT
/// DECISIONS, AND THE SCORE USED TO BE THE SAME NUMBER FOR ALL THREE.**
///
/// The motion score was `alignment × speed / kit's fastest speed`. There is no
/// LENGTH anywhere in that expression, so the gap magnitude cancels: a
/// full-strength recovery was worth exactly as much against an opponent 5px
/// away as against one 280px away as against one 900px away. Reviewed
/// 2026-09-20; `medic_rescue_lift` applies about `(34, -905)` and is the
/// production instance.
///
/// ⭐ THE THREE CASES, all on one kit and one direction so the only thing that
/// varies is how far there is to go. The MIDDLE one is the useful one: a
/// motion is worth pressing when it approximately arrives.
#[test]
fn a_motion_is_priced_by_how_much_of_the_gap_it_actually_covers() {
    let w = UtilityWeights::v1();
    // 900px/s straight up, fired 0.1s into a 0.5s move: **360px** of
    // travel, not 450 — the body is in its windup for the first 0.1s.
    // ⛔ The fixture states that windup (`hitless_lifting_candidate`
    // authors `lift_at_s: 0.1`), so a probe distance read off the naive
    // product would be certifying the bug this arm is priced against.
    let kit = [hitless_lifting_candidate("up_b", 900.0)];

    // `+y` is toward this body's feet, so a foe overhead is a NEGATIVE local
    // `y` — the direction the lift travels. Only the height changes.
    let overhead_by = |above: f32| {
        let mut view = view_with(300.0, 300.0);
        view.actors[0].pos.y = 300.0 - above;
        generate_options(Perceived::cheating(&view), Situation::Neutral, &kit, &w)
            .motions
            .first()
            .map(|m| m.score)
            .expect("the launch is not on the motion list at all")
    };

    let on_top_of_me = overhead_by(5.0);
    let a_useful_distance = overhead_by(360.0);
    let far_beyond_reach = overhead_by(1400.0);

    assert!(
        a_useful_distance >= MOTION_WORTH_PRESSING,
        "a launch that lands on an opponent 360px overhead is the one case \
         this move exists for, and it is not worth pressing: {a_useful_distance}"
    );
    assert!(
        on_top_of_me < MOTION_WORTH_PRESSING,
        "throwing myself 360px into the air to reach somebody 5px away is a \
         way to leave, not a way to arrive: {on_top_of_me}"
    );
    assert!(
        far_beyond_reach < MOTION_WORTH_PRESSING,
        "a 360px launch covers about a quarter of a 1400px gap, so pressing \
         it spends the body's whole commitment and still does not arrive: \
         {far_beyond_reach}"
    );
    // ⚠ AND THE MIDDLE IS THE PEAK, not merely above a threshold two ends
    // happen to miss — without this the arm passes for a score that rises
    // monotonically with distance and clips at both ends.
    assert!(
        a_useful_distance > on_top_of_me && a_useful_distance > far_beyond_reach,
        "arriving is not the best case: on_top={on_top_of_me} \
         useful={a_useful_distance} beyond={far_beyond_reach}"
    );
}

/// ⛔⛤ **THE BRAIN RANKED ITS FINISHERS UNDER A LAUNCH LAW THE GAME DOES NOT
/// USE, AND THE DOC SAID SO WHILE ARGUING IT DID NOT MATTER.**
///
/// `LaunchEnvelope::at` evaluated `base + growth × victim_damage` and its own
/// comment held that the ruleset's percent scale, the per-`base` steepening,
/// the victim's weight and rage are *"COMMON to every candidate one attacker
/// weighs against one opponent, so none of them can reorder a kit"*. Every one
/// of them multiplies the PERCENT TERM and not `base`, so they move the
/// CROSSOVER between two candidates — see `ambition_entity_catalog::launch`,
/// which now owns the one copy of the law and carries the arithmetic.
///
/// ⭐ THE SUBJECT IS THE PAIR THE REVIEW NAMED: George Booul's forward smash
/// `(185, 3.45)` against his up smash `(178, 6.28)`, at TWO points of victim
/// damage — a crossover the identity law puts on the other side.
///
/// ⚠ **THE TWO CANDIDATES ARE IDENTICAL IN EVERY OTHER RESPECT**, so the flip
/// below cannot come from reach, startup or payoff. And the first arm is the
/// CONTROL: under the identity law the old answer is the RIGHT one, which is
/// what makes this a statement about the conditions rather than about two
/// numbers that happen to be close.
// `6.28` is George's AUTHORED up-smash growth, not TAU — see
// `george_booul_moveset.rs`, which carries the same allow. The pair `(185,
// 3.45)` / `(178, 6.28)` is the one the review named, so rounding either to
// please the lint would make this a test about two invented numbers.
#[allow(clippy::approx_constant)]
#[test]
fn a_finisher_is_ranked_under_the_launch_law_the_stage_actually_declares() {
    let w = UtilityWeights::v1();
    let kit = [
        candidate_with_growth("forward_smash", 0.2, 60.0, 185.0, 3.45),
        candidate_with_growth("up_smash", 0.2, 60.0, 178.0, 6.28),
    ];
    let best_under = |law: crate::perception::LaunchLaw, weight: f32| -> String {
        let mut view = view_with(300.0, 340.0);
        view.actors[0].damage_taken = 2;
        view.actors[0].knockback_weight = weight;
        view.launch_law = law;
        generate_options(Perceived::cheating(&view), Situation::Neutral, &kit, &w)
            .best_attack()
            .map(|a| a.move_id.clone())
            .expect("both candidates reach a foe 40px away")
    };

    assert_eq!(
        best_under(Default::default(), 1.0),
        "forward_smash",
        "under the identity law against a reference body the forward smash \
         still wins at two damage — without that the flip below is not about \
         the conditions"
    );

    // The smash stage's own declared law, by value: scale `1.25` against a
    // fighter whose authored knockback weight is `0.85`. Cited rather than
    // imported — this crate is below the demo.
    //
    // ⛔ AND THE GROWTH-BASE CURVE IS `IDENTITY` BECAUSE THE STAGE DECLARES
    // `growth_base: None`. It once declared `48 / 0.25 / 1.40`; that law was
    // retired in favour of explicitly authored `knockback_growth`, and writing
    // the retired constants in here would make this arm certify a model the
    // stage no longer runs. The flip does not need them: two factors the old
    // envelope omitted are enough on their own.
    let declared = crate::perception::LaunchLaw {
        growth_scale: 1.25,
        growth_base: ambition_entity_catalog::launch::GrowthBaseCurve::IDENTITY,
        // The stage's fallback growth. Both candidates author their own, so it
        // moves nothing here — stated because a `0.0` would be the undeclared
        // world wearing the stage's name.
        ruleset_growth: 0.02,
        rage: 1.0,
    };
    assert_eq!(
        best_under(declared, 0.85),
        "up_smash",
        "at the SAME victim damage, on the stage this fighter is standing on \
         and against the body it is actually hitting, the up smash has already \
         overtaken — and the brain picked the other one"
    );
}

/// ⛔⛤ **STAGE RISK USED TO BE THE SAME NUMBER FOR EVERY CANDIDATE, SO IT
/// COULD NOT COST ONE MOVE MORE THAN ANOTHER.**
///
/// An attack's score is only ever compared with another attack's, and
/// `edge_proximity(me.pos)` is a fact about the BODY: it added a constant to
/// every option. The arm that showed the same jab scoring lower near a ledge
/// was proving arithmetic — both moves fell by the same amount, so the ranking
/// never moved. This asks the question the feature is named for: standing in
/// the same place, does the move that throws me at the blast line cost more
/// than the one that does not.
#[test]
fn the_move_that_hurls_me_at_the_blastzone_costs_more_than_the_one_that_stays_put() {
    let w = UtilityWeights::v1();

    let planted = candidate("planted", 0.1, 100.0);
    let mut hurled = candidate("hurled", 0.1, 100.0);
    hurled.frames.start_impulse = (1400.0, 0.0);
    let kit = [planted, hurled];

    let risk_at = |me_x: f32, foe_x: f32, id: &str| {
        generate_options(
            Perceived::cheating(&view_with(me_x, foe_x)),
            Situation::Neutral,
            &kit,
            &w,
        )
        .attacks
        .iter()
        .find(|a| a.move_id == id)
        .unwrap_or_else(|| panic!("{id} must reach at ({me_x}, {foe_x})"))
        .features
        .stage_risk
    };

    // Backed against the right wall with the foe in front.
    assert!(
        risk_at(700.0, 780.0, "hurled") > risk_at(700.0, 780.0, "planted"),
        "a move that carries this body at the wall must cost more than one \
         thrown from the same spot: hurled {} planted {}",
        risk_at(700.0, 780.0, "hurled"),
        risk_at(700.0, 780.0, "planted")
    );

    // ⭐⭐ THE CONTROL THAT MAKES THAT A CLAIM ABOUT DIRECTION. The mirrored
    // spot is 100 units from the LEFT wall, so `distance_to_edge` reads
    // exactly the same there — the old constant could not tell these two
    // worlds apart at all — and the identical lunge now has seven hundred
    // units of room ahead of it instead of one hundred.
    let outboard = risk_at(700.0, 780.0, "hurled");
    let inboard = risk_at(100.0, 180.0, "hurled");
    assert!(
        inboard < outboard,
        "lunging into the stage is not the hazard lunging off it is: inboard \
         {inboard}, outboard {outboard}"
    );
    assert_eq!(
        risk_at(700.0, 780.0, "planted"),
        risk_at(100.0, 180.0, "planted"),
        "the premise: a move with no self-motion reads the same in both, so \
         the difference above is the DIRECTION and not the position"
    );
}

/// ⛔⛤ **A BOLT IS NOT A THREAT WHERE IT IS THROWN, AND ADMISSION AIMED AS IF
/// IT WERE.**
///
/// The lead was repaired twice in one day and both repairs were about the
/// THROW. First it led by `startup_s`, which for a projectile move is the
/// whole duration — a large over-lead. Then by
/// `MoveFrameData::threat_live_at_s`, the instant the hazard leaves — which is
/// exact for a swing and an under-lead of the entire flight for anything that
/// travels. `director_train_of_thought` crosses 671px at 300px/s, so its shot
/// lands up to two seconds after it is thrown.
///
/// ⭐ **THE FLIGHT LAW IS WHAT MAKES THE THIRD ANSWER POSSIBLE**, and it is
/// what the old `hazard_reach: f32` folded away: it computed `speed ×
/// lifetime` and kept only the product.
/// [`ambition_entity_catalog::ThreatTravel`] carries the law, and the fourth
/// arm below is the reason it is a law and not a speed — a hazard that is not
/// live when it arrives is refused for a reason that has no speed in it.
///
/// ⚠ **TWO CONTROLS, BECAUSE "REFUSED" HAS TWO INNOCENT EXPLANATIONS.** A
/// stationary hazard over the same geometry must still be admitted — otherwise
/// the arm is measuring the reach rule — and a stationary FOE must be admitted
/// against the travelling hazard — otherwise `speed` is being read as a
/// penalty rather than as a flight time.
#[test]
fn a_travelling_hazard_is_aimed_where_the_foe_will_be_when_it_arrives() {
    let weights = UtilityWeights::default();
    // 600px of gap, and the foe walking away at 200px/s.
    let retreating = |vel: f32| {
        let mut view = view_with(300.0, 900.0);
        view.actors[0].vel = ae::Vec2::new(vel, 0.0);
        view
    };
    let offered = |hazard: ambition_entity_catalog::MoveHazard, foe_vel: f32| {
        let mut bolt = candidate("bolt", 0.2, 0.0);
        bolt.frames.hazard = Some(hazard);
        let view = retreating(foe_vel);
        generate_options(
            crate::perception::Perceived::cheating(&view),
            Situation::Neutral,
            std::slice::from_ref(&bolt),
            &weights,
        )
        .attacks
        .iter()
        .any(|a| a.move_id == "bolt")
    };

    // 700px of reach, crossed at 300px/s: the shot needs over two seconds to
    // arrive, and in that time the foe has walked past the end of its flight.
    let travelling = ambition_entity_catalog::MoveHazard::Spawned {
        travel: ambition_entity_catalog::ThreatTravel::Straight {
            speed: 300.0,
            span: 700.0,
            free: 0.0,
        },
        damage: 0,
    };
    // ⚠ **THE CONTROL IS A PLACEMENT THAT ARMS INSTANTLY, AND IT IS NOT THE
    // SHIPPED BOMB.** It used to be described as *"a laid bomb"*, which was a
    // category error the reviewer caught: the polygon's bomb sits on a
    // four-second fuse and is the subject of its own arm below. What this
    // control needs is the same 700px of reach with the FLIGHT term removed,
    // so that a refusal above can only be about the flight.
    let placed_and_live = ambition_entity_catalog::MoveHazard::Spawned {
        travel: ambition_entity_catalog::ThreatTravel::Placed {
            reach: 700.0,
            detonates_by_s: 0.0,
        },
        damage: 0,
    };

    assert!(
        !offered(travelling, 200.0),
        "a 700px shot at 300px/s was offered at a 600px gap against somebody \
         walking away at 200px/s — it arrives where they were, two seconds ago"
    );
    assert!(
        offered(placed_and_live, 200.0),
        "THE CONTROL: the same 700px over the same 600px gap, put in place \
         instead of thrown. Refusing this would mean the arm above is about \
         the reach rule and not about the flight"
    );
    assert!(
        offered(travelling, 0.0),
        "THE SECOND CONTROL: the same travelling shot against somebody \
         standing still. Refusing this would mean `speed` is being spent as a \
         penalty rather than as a time"
    );

    // ⛔⛤ **AND A FUSE IS NOT A FLIGHT TIME — THIS ARM ASSERTED THE OPPOSITE
    // FOR ONE SWEEP.** Teaching the laid bomb its four-second fuse was right;
    // feeding that fuse to the AIM LEAD was not. A lead carries the opponent
    // forward at the velocity last seen, and four seconds of that is
    // arithmetic about a walk nobody takes: Projectile Polygon's bomb reaches
    // 72px, so at ANY walking speed the extrapolated opponent is outside it.
    // Measured on the grid — her row went 144/228 to **131/183** and her
    // repertoire from 17 distinct moves to 15, which is the move being deleted
    // rather than corrected.
    //
    // ⭐ ⇒ The aim asks `travel_to` (where do I point this so it lands on
    // them) and the fuse is `live_at_s` (when can it hurt anybody at all). A
    // placed object is placed where it is placed, so it is aimed nowhere and
    // travels for zero, and this arm now says THAT. Nothing prices the fuse
    // yet and that is written down on the type rather than patched in here.
    let on_a_fuse = ambition_entity_catalog::MoveHazard::Spawned {
        travel: ambition_entity_catalog::ThreatTravel::Placed {
            reach: 700.0,
            detonates_by_s: 4.0,
        },
        damage: 0,
    };
    assert!(
        offered(on_a_fuse, 200.0),
        "a placed hazard was aimed at where the opponent will be in four \
         seconds, which is how the polygon lost her bomb"
    );
    assert_eq!(
        on_a_fuse.travel_to(600.0),
        Some(0.0),
        "a placed hazard reported a flight time"
    );
    assert_eq!(
        on_a_fuse.detonates_by_s(),
        4.0,
        "the fuse survived being taken out of the lead"
    );
}

/// ⛔⛤ **TWO LAUNCHERS WERE INDISTINGUISHABLE ON POWER, HOWEVER HARD EITHER
/// ONE HIT.**
///
/// `expected_payoff` is a move's damage over the kit's strongest, and it asked
/// `MoveFrameData::max_damage` — a fold over ACTIVE VOLUMES. A launcher
/// authors none, so a kit of nothing but launchers had `kit_max_damage == 0`
/// and every candidate's power was zero: the feature was not merely wrong
/// about the order, it was switched off for the whole projectile half of the
/// roster.
///
/// ⚠ **THE SUBJECT IS THE ORDER, so the two candidates differ in ONE thing.**
/// Same startup, same (absent) volume, same hazard reach and law; only the
/// damage the hazard deals. Projectile Polygon's real pair is `7` and `4`.
///
/// ⭐ **AND THE CONTROL IS THE SWAP**, because "the stronger one wins" is also
/// what declaration order, `Vec` order and a stable sort would produce. With
/// the numbers exchanged the other move must win, or this arm is measuring the
/// list and not the feature.
#[test]
fn the_stronger_of_two_launchers_is_the_one_offered() {
    let launcher = |id: &str, damage: i32| {
        let mut c = candidate(id, 0.2, 0.0);
        // No volume on the body: the shot IS the damage, which is the shape
        // this arm exists for.
        c.frames.max_damage = 0;
        c.frames.hazard = Some(ambition_entity_catalog::MoveHazard::Spawned {
            // ⚠ **A SHOT THAT IS LIVE WHERE IT IS THROWN, NOT A PLACED
            // OBJECT.** `free` is the ground a hazard covers WITHOUT flying,
            // so this reaches 700px at zero flight time and neither
            // candidate's flight can separate them for a second reason. It
            // said `Placed { detonates_by_s: 0.0 }` until 2026-09-21, which
            // was borrowing a TRAP's zero-flight answer to stand in for an
            // instant SHOT — and a trap's damage is deliberately not priced
            // as immediate, so the fixture stopped exercising the property
            // this arm is named for.
            travel: ambition_entity_catalog::ThreatTravel::Straight {
                speed: 100.0,
                span: 0.0,
                free: 700.0,
            },
            damage,
        });
        c
    };
    let weights = UtilityWeights::v1();
    // ⛔ A COMMITTED OPPONENT, AND THE FEATURE'S OWN DOC SAYS WHY. Payoff is
    // `power` GATED on the move plausibly landing, and in neutral that gate
    // is zero for everybody — deliberately, so power cannot smuggle itself
    // into every exchange. Asking this question of a neutral view would
    // measure the tie-break.
    let view = {
        let mut v = view_with(300.0, 400.0);
        v.actors[0].phase = BodyPhase::AttackRecovery;
        v.actors[0].phase_remaining = 0.6;
        v
    };
    let best = |kit: &[AttackCandidate]| {
        generate_options(
            crate::perception::Perceived::cheating(&view),
            Situation::Advantage,
            kit,
            &weights,
        )
        .attacks
        .first()
        .map(|a| a.move_id.clone())
    };

    assert_eq!(
        best(&[launcher("ponytail", 7), launcher("cannon", 4)]).as_deref(),
        Some("ponytail"),
        "the 4-damage shot outranked the 7-damage one"
    );
    assert_eq!(
        best(&[launcher("ponytail", 4), launcher("cannon", 7)]).as_deref(),
        Some("cannon"),
        "THE CONTROL: with the damages exchanged the winner did not change, \
         so this arm is reading the list order and not the damage"
    );
}

/// ⛔⛤ **THE PAYOFF GATE ASKED `startup_s`, AND FOR A LAUNCHER THAT IS THE
/// WHOLE MOVE — SO EVERY RANGED MOVE IN THE GAME WAS PRICED AT ZERO POWER
/// WHATEVER IT DEALT.**
///
/// `startup_s` is *"time until the first Active window"* and falls back to the
/// move's DURATION when there is none, which is the shape of every launcher.
/// The gate is `their_commitment > startup_s`: it was asking whether the
/// opponent is committed for longer than the attacker's entire animation.
///
/// ⭐ **THIS IS THE SAME DEFECT THE AIM LEAD WAS CORRECTED FOR ONE DAY AND ONE
/// FIELD EARLIER**, which is why it is worth a test rather than a one-line
/// change: a field derived with a fallback answers two questions, and fixing
/// the first reader does not fix the second. The lead got
/// `threat_live_at_s`; this gets *when the move CONNECTS*, which for a shot is
/// the throw plus the flight.
/// A TRAP'S DAMAGE IS NOT AN IMMEDIATE PUNISH.
///
/// ⛔⛤ **RAISED BY REVIEW 2026-09-21.** `ThreatTravel::Placed` answers a zero
/// flight because a laid object does not travel, and that is the right answer
/// to the AIMING question. The payoff term read it as *"the damage is
/// available the moment it is dropped"*, so the polygon's 12-damage bomb was
/// priced as a punish inside its 72px reach — while the type's own doc says
/// nothing prices the four-second fuse. The runtime model had learned that a
/// bomb is not a projectile and the decision model was still spending it as
/// one.
///
/// ⚠ **THE CONTROL IS A FLYING SHOT WITH THE SAME DAMAGE AND THE SAME ZERO
/// FLIGHT**, so the arm cannot pass because the whole payoff term is dead or
/// because the window was too short: one number changes, and it is the one the
/// law now answers.
#[test]
fn a_laid_trap_is_not_priced_as_damage_that_lands_when_it_is_laid() {
    let hazard = |travel: ambition_entity_catalog::ThreatTravel| {
        let mut c = candidate("thing", 0.2, 0.0);
        // No volume: what this move does to somebody, it does through the
        // thing it puts in the world.
        c.frames.max_damage = 0;
        c.frames.hazard = Some(ambition_entity_catalog::MoveHazard::Spawned {
            travel,
            damage: 12,
        });
        c
    };
    // A bomb: reaches 72px, goes off by itself in four seconds.
    let laid = ambition_entity_catalog::ThreatTravel::Placed {
        reach: 72.0,
        detonates_by_s: 4.0,
    };
    // The SAME reach and the same zero flight, but it flies — `free` is the
    // ground a hazard covers without flying.
    let thrown = ambition_entity_catalog::ThreatTravel::Straight {
        speed: 100.0,
        span: 0.0,
        free: 72.0,
    };

    // Point blank and committed, which is the most favourable reading a
    // trap could get: the opponent is inside the blast and cannot leave.
    let view = {
        let mut v = view_with(300.0, 340.0);
        v.actors[0].phase = BodyPhase::AttackRecovery;
        v.actors[0].phase_remaining = 0.6;
        v
    };
    let weights = UtilityWeights::v1();
    let payoff = |c: AttackCandidate| {
        generate_options(
            Perceived::cheating(&view),
            Situation::Advantage,
            &[c],
            &weights,
        )
        .attacks
        .first()
        .map(|a| a.features.expected_payoff)
    };

    assert_eq!(
        payoff(hazard(laid)),
        Some(0.0),
        "a four-second fuse was priced as damage that lands on the tick the bomb \
         is dropped"
    );
    assert!(
        payoff(hazard(thrown)).is_some_and(|p| p > 0.0),
        "the control says nothing is worth anything here, so the arm above is not \
         about traps"
    );
}

#[test]
fn a_launchers_payoff_is_gated_on_when_its_shot_arrives_not_on_the_whole_move() {
    // A launcher shaped like `polygon_projectile_charge_shot`: fires at 0.26s,
    // reports `startup_s` 0.58s because it lands no volume.
    let launcher = |id: &str, travel: ambition_entity_catalog::ThreatTravel| {
        let mut c = candidate(id, 0.58, 0.0);
        c.frames.threat_live_at_s = Some(0.26);
        c.frames.max_damage = 0;
        c.frames.hazard = Some(ambition_entity_catalog::MoveHazard::Spawned {
            travel,
            damage: 9,
        });
        c
    };
    // Instant: 700px of reach covered WITHOUT flying (`free`), so
    // `connects_at` is the throw itself. ⚠ It said `Placed` until 2026-09-21 —
    // a trap's zero flight standing in for an instant shot, which stopped
    // being a valid stand-in when a trap's damage stopped being priced as
    // immediate.
    let instant = ambition_entity_catalog::ThreatTravel::Straight {
        speed: 100.0,
        span: 0.0,
        free: 700.0,
    };
    // The same 700px of reach, crossed at 100px/s. Against a foe 100px away
    // the flight alone is a second — longer than any opening.
    let slow = ambition_entity_catalog::ThreatTravel::Straight {
        speed: 100.0,
        span: 700.0,
        free: 0.0,
    };

    // A committed opponent: 0.4s of recovery left, which is longer than the
    // 0.26s throw and SHORTER than the 0.58s animation. That band is exactly
    // where the old gate and the new one disagree.
    let mut view = view_with(300.0, 400.0);
    view.actors[0].phase = BodyPhase::AttackRecovery;
    view.actors[0].phase_remaining = 0.4;
    let weights = UtilityWeights::v1();
    let payoff = |c: AttackCandidate| {
        let id = c.move_id.clone();
        generate_options(
            Perceived::cheating(&view),
            Situation::Advantage,
            &[c],
            &weights,
        )
        .attacks
        .iter()
        .find(|a| a.move_id == id)
        .map(|a| a.features.expected_payoff)
    };

    assert!(
        payoff(launcher("quick", instant)).is_some_and(|p| p > 0.0),
        "a shot that is live the moment it is thrown, into a window twice as \
         long as the throw, was still worth nothing"
    );
    // ⭐ THE CONTROL, and it is what separates this from simply swapping one
    // field for another: the SAME reach and the same throw, but the shot
    // needs a second to get there, so the opening is gone before it arrives.
    assert_eq!(
        payoff(launcher("slow", slow)),
        Some(0.0),
        "a shot with a second of flight was paid for an opening that closes \
         in 0.4s, so the gate is reading the throw and not the arrival"
    );

    // ⚠ AND AN ORDINARY STRIKE IS UNTOUCHED. `threat_live_at_s` is `None`
    // here, so `connects_at` falls back to `startup_s` — the reading this
    // gate has always had for a move that hits with its own body.
    // ⚠ 150px of reach, because the foe is 100px away and an attack that
    // cannot span the gap is not offered at all — a 40px jab would leave the
    // list empty and the arm would read `None` as a lost payoff.
    let mut strike = candidate("jab", 0.08, 150.0);
    strike.frames.max_damage = 9;
    assert!(
        payoff(strike).is_some_and(|p| p > 0.0),
        "a 0.08s jab into a 0.4s window lost its payoff, so the change \
         reached further than launchers"
    );
}

/// ⛔⛤ **A WORN MOVE WAS PRICED AT ITS FRESH DAMAGE, AND THE HIT RESOLVER HAD
/// BEEN STALING IT ALL ALONG.**
///
/// `expected_payoff` is the move's power over the kit's strongest, and it read
/// `MoveFrameData` — a pure derivation of the `MoveSpec`, identical for a move
/// thrown once and a move thrown nine times. The runtime resolves the landing
/// as `damage × stale_scale(n)`, so on the smash stage's declared `0.05 / 0.55`
/// the scorer was pricing a fully worn move at 182% of what it deals.
///
/// ⭐ **AND THE CONTROL IS THE SWAP.** "The fresh one wins" is also what
/// declaration order produces, so the two candidates exchange their wear and
/// the other one must win.
#[test]
fn a_move_this_body_has_worn_out_loses_to_the_one_it_has_not() {
    use crate::brain::attack_kit::MoveWear;
    // Identical in every respect the scorer can see EXCEPT the wear: same
    // startup, same reach, same damage, same coverage.
    let worn_pair = |a: MoveWear, b: MoveWear| {
        let mut first = candidate("overused", 0.1, 100.0);
        first.frames.max_damage = 10;
        first.wear = a;
        let mut second = candidate("rested", 0.1, 100.0);
        second.frames.max_damage = 10;
        second.wear = b;
        vec![first, second]
    };
    // The floor and the influence the smash stage declares, reached after nine
    // recent landings.
    let worn = MoveWear {
        damage: 0.55,
        launch_growth: 0.865,
    };

    // A committed opponent, because `expected_payoff` is gated on the move
    // plausibly landing and that gate is zero for everybody in neutral.
    let mut view = view_with(300.0, 400.0);
    view.actors[0].phase = BodyPhase::AttackRecovery;
    view.actors[0].phase_remaining = 0.6;
    let weights = UtilityWeights::v1();
    let best = |kit: Vec<AttackCandidate>| {
        generate_options(
            Perceived::cheating(&view),
            Situation::Advantage,
            &kit,
            &weights,
        )
        .best_attack()
        .map(|a| a.move_id.clone())
    };

    assert_eq!(
        best(worn_pair(worn, MoveWear::FRESH)).as_deref(),
        Some("rested"),
        "the worn move still outbid the fresh one, so the wear never reached \
         the payoff"
    );
    assert_eq!(
        best(worn_pair(MoveWear::FRESH, worn)).as_deref(),
        Some("overused"),
        "THE CONTROL: with the wear exchanged the winner did not change, so \
         this arm is reading the list order"
    );
}

/// ⛔⛤ **AND THE PERCENT TERM IS WORN SEPARATELY, WHICH IS THE HALF A SINGLE
/// SCALE WOULD HAVE GOT WRONG.**
///
/// The hit resolver spends `victim_percent_knockback_scale ×
/// knockback_stale_scale(..)` on the launch's GROWTH and leaves `base` alone —
/// the reason is recorded where it made the split: multiplying the whole
/// launch by the stale factor threw away half of everything at high percent
/// and the stock stopped ending. So a worn finisher must launch a FRESH
/// opponent exactly as far as it always did and a worn one less far.
#[test]
fn wearing_a_finisher_out_costs_it_the_percent_term_and_not_its_base() {
    use crate::brain::attack_kit::MoveWear;
    let worn = MoveWear {
        damage: 0.55,
        launch_growth: 0.865,
    };
    // Base 100 growing 2.0 per point, against a FLAT 150 that never grows.
    // The two cross somewhere in the percent range, which is what makes the
    // share below able to move at all — a kit of one candidate always shares
    // 1.0 against itself.
    let kit = |wear: MoveWear| {
        let mut finisher = candidate_with_growth("finisher", 0.1, 100.0, 100.0, 2.0);
        finisher.wear = wear;
        // ⚠ THE REFERENCE IS FRESH, SET, AND BIGGER THAN EITHER READING OF
        // THE FINISHER. A launch with no growth declines the percent term
        // entirely (`launch_speed` short-circuits on zero growth), so the
        // wear cannot touch it — and keeping it above both readings means
        // `kit_max_launch` is the SAME number in the two runs, so the share
        // moves only because the finisher did. A yardstick that the subject
        // can overtake is not a yardstick.
        let mut reference = candidate_with_launch("reference", 0.1, 100.0, 400.0);
        reference.wear = MoveWear::FRESH;
        vec![finisher, reference]
    };
    let kill_of = |damage_taken: i32, wear: MoveWear| {
        let mut view = view_with(300.0, 400.0);
        view.actors[0].damage_taken = damage_taken;
        generate_options(
            Perceived::cheating(&view),
            Situation::Neutral,
            &kit(wear),
            &UtilityWeights::v1(),
        )
        .attacks
        .iter()
        .find(|a| a.move_id == "finisher")
        .map(|a| a.features.kill_potential)
        .expect("the finisher is in the kit")
    };

    // ⚠ AT 0% THERE IS NOTHING TO LOSE. `kill_potential` rides the foe's
    // damage fraction, so this reads zero either way — which is the
    // base-is-untouched claim at the one percent where base is the whole
    // launch, and it is a CONTROL rather than the subject.
    assert_eq!(kill_of(0, worn), kill_of(0, MoveWear::FRESH), "0%");

    // ⭐ AT 60% THE PERCENT TERM IS MOST OF THE LAUNCH, and the worn
    // finisher's share of the kit's best must fall.
    let fresh = kill_of(60, MoveWear::FRESH);
    let stale = kill_of(60, worn);
    assert!(fresh > 0.0, "the fixture scored no kill potential at all");
    assert!(
        stale < fresh,
        "a worn finisher was priced to finish as hard as a fresh one \
         ({stale} vs {fresh})"
    );
    // ⛔⛤ **AND THE AMOUNT, NOT A BAND — A LOOSE ONE TOOK A POISON CLEAN.**
    // This said `stale > fresh * 0.55`, reasoning that 55% of the fresh share
    // would mean the DAMAGE answer had been spent on the launch. Spending
    // `wear.damage` on the growth term instead resolves to 0.249 against a
    // fresh 0.33 — comfortably inside that band and exactly the confusion the
    // two fields exist to prevent. ⇒ The ratio is arithmetic the fixture can
    // state: `kill_potential` is `damage_frac × launch / kit_max_launch`, so
    // the foe's meter and the yardstick cancel and what is left is the two
    // launches. Written out from the fixture's own numbers rather than as a
    // constant, so a reader can see which factor rides which term.
    let base = 100.0_f32;
    let growth = 2.0_f32;
    let victim_damage = 60.0_f32;
    let expected =
        (base + growth * worn.launch_growth * victim_damage) / (base + growth * victim_damage);
    assert!(
        (stale / fresh - expected).abs() < 1.0e-4,
        "the worn finisher kept {} of its fresh kill share; {expected} is the \
         influence riding the GROWTH, and {} would be the damage answer spent \
         on the launch instead",
        stale / fresh,
        (base + growth * worn.damage * victim_damage) / (base + growth * victim_damage),
    );
}
