//! the pair is the point. A body that never moves passes the first test
//! for the wrong reason, so the second is its non-vacuity control: the SAME
//! body, the SAME step, hitlag cleared, must travel. Measuring only the freeze
//! is how you ship a body that is frozen forever.

use super::*;

const DT: f32 = 1.0 / 60.0;
/// Fast enough that one tick's travel is unmistakable against float noise.
const SPEED: f32 = 600.0;

fn open_world() -> ae::World {
    // No blocks: the body is in free flight, so the only thing that can stop it
    // is the step's own `dt`. A floor would give the test a second explanation.
    ae::World::new(
        "hitlag probe",
        ae::Vec2::new(20_000.0, 20_000.0),
        ae::Vec2::ZERO,
        Vec::new(),
    )
}

fn body_travelling() -> ae::BodyClusterScratch {
    ae::BodyClusterScratch::new_with_abilities(ae::Vec2::ZERO, ae::AbilitySet::basic())
        .with_velocity(ae::Vec2::new(SPEED, 0.0))
}

fn frame() -> ae::MotionFrame {
    ae::MotionFrame::from_acceleration(ae::Vec2::new(0.0, ae::movement::GRAVITY))
        .expect("a non-zero gravity is a valid frame")
}

/// Step `body` once with the attempt ALREADY ENDED, and answer how far it moved.
fn travel_out_of_play(body: &mut ae::BodyClusterScratch, combat: &mut BodyCombat) -> f32 {
    let world = open_world();
    let before = body.kinematics.pos;
    {
        let (model, mut clusters) = body.parts();
        step_body(
            model,
            &mut clusters,
            combat,
            ae::MovementTuning::default(),
            true,
            // This fixture steps a body nothing is holding.
            false,
            ae::MotionStepContext {
                world: &world,
                input: ae::InputState::default(),
                frame: frame(),
                facing_intent: 0.0,
                dt: DT,
                contact: ae::BodyContactField::NONE,
            },
        );
    }
    (body.kinematics.pos - before).length()
}

/// Step `body` once and answer how far it actually travelled.
fn travel(body: &mut ae::BodyClusterScratch, combat: &mut BodyCombat) -> f32 {
    stepped(
        body,
        combat,
        ae::MovementTuning::default(),
        ae::InputState::default(),
    )
    .length()
}

/// The same step, with the tuning and the input named — for SDI, which is a
/// property of both.
fn stepped(
    body: &mut ae::BodyClusterScratch,
    combat: &mut BodyCombat,
    tuning: ae::MovementTuning,
    input: ae::InputState,
) -> ae::Vec2 {
    let world = open_world();
    let before = body.kinematics.pos;
    {
        let (model, mut clusters) = body.parts();
        step_body(
            model,
            &mut clusters,
            combat,
            tuning,
            false,
            // This fixture steps a body nothing is holding.
            false,
            ae::MotionStepContext {
                world: &world,
                input,
                frame: frame(),
                facing_intent: 0.0,
                dt: DT,
                contact: ae::BodyContactField::NONE,
            },
        );
    }
    body.kinematics.pos - before
}

/// SDI IS THE ONE THING A FROZEN BODY MAY STILL DO.
///
/// three assertions and each kills a different wrong version: a body with no
/// authored budget must not move (every body in Ambition), one with a budget and
/// a held stick must, and the shift must be the STICK's direction rather than
/// the body's own velocity — a version that nudged along `vel` would look right
/// in a test where the two happen to agree and be a different mechanic.
#[test]
fn a_frozen_body_can_still_influence_where_the_next_hit_finds_it() {
    let mut frozen = BodyCombat {
        hitstop_timer: 0.08,
        ..BodyCombat::default()
    };
    assert!(frozen.is_in_hitlag(), "the fixture must arm the freeze");
    // the body travels along +x; the stick is held along -y (up), so the two
    // cannot be confused for each other.
    let held = ae::InputState {
        axes: ae::LocalAxes::new(0.0, -1.0),
        ..Default::default()
    };
    let mut tuning = ae::MovementTuning::default();
    tuning.sdi_step = 3.0;

    let unauthored = stepped(
        &mut body_travelling(),
        &mut frozen,
        ae::MovementTuning::default(),
        held.clone(),
    );
    assert_eq!(
        unauthored,
        ae::Vec2::ZERO,
        "a body that authored no SDI budget influenced anyway"
    );

    let shifted = stepped(&mut body_travelling(), &mut frozen, tuning.clone(), held);
    assert_eq!(
        shifted,
        ae::Vec2::new(0.0, -3.0),
        "SDI did not move the frozen body one step along the held stick"
    );

    // and a null stick is still a freeze: the budget alone must not drift a
    // body that is asking for nothing.
    let idle = stepped(
        &mut body_travelling(),
        &mut frozen,
        tuning,
        ae::InputState::default(),
    );
    assert_eq!(idle, ae::Vec2::ZERO, "a budget alone drifted a frozen body");
}

#[test]
fn a_body_in_hitlag_does_not_travel_through_its_own_freeze() {
    let mut body = body_travelling();
    let mut combat = BodyCombat {
        hitstop_timer: 0.08,
        ..BodyCombat::default()
    };
    assert!(
        combat.is_in_hitlag(),
        "the fixture must actually arm the freeze, or this asserts nothing"
    );

    let moved = travel(&mut body, &mut combat);

    assert_eq!(
        moved, 0.0,
        "a body carrying hitstop walked {moved}px through its own freeze"
    );
}

#[test]
fn and_the_same_body_travels_once_the_freeze_clears() {
    let mut body = body_travelling();
    let mut combat = BodyCombat::default();
    assert!(
        !combat.is_in_hitlag(),
        "the control must NOT be frozen, or it proves nothing about the other test"
    );

    let moved = travel(&mut body, &mut combat);

    // One tick at `SPEED` is 10px; assert a floor rather than the exact figure so
    // gravity's contribution to the same tick is not a false failure.
    assert!(
        moved > 1.0,
        "an unfrozen body should have travelled; it moved {moved}px, so the \
         freeze test above may be passing because nothing moves at all"
    );
}

/// A DEAD BODY STOPS WHERE IT DIED — velocity cleared, not merely unread.
///
/// death should stop her velocity to play her death animation, so the camera
/// should stop too as a side effect."*
///
/// `OutOfPlay` only ever gated a `BodyReset`, so gravity and carried momentum
/// went on integrating and she slid through her own death animation — while the
/// component's doc claimed *"nothing moves her now"*. This pins the claim.
#[test]
fn a_body_whose_attempt_ended_does_not_travel() {
    let mut body = body_travelling();
    let mut combat = BodyCombat::default();
    assert_eq!(
        travel_out_of_play(&mut body, &mut combat),
        0.0,
        "a body that is out of play moved"
    );
    assert_eq!(
        body.kinematics.vel,
        ae::Vec2::ZERO,
        "the velocity survived the death, so it would be spent the instant the \
         body came back"
    );
}

/// The non-vacuity control the header demands: the SAME body, the SAME step,
/// still in play, must travel. Without this the test above passes for a body
/// that could never move at all.
#[test]
fn the_same_body_still_in_play_travels() {
    let mut body = body_travelling();
    let mut combat = BodyCombat::default();
    let moved = travel(&mut body, &mut combat);
    assert!(
        moved > 1.0,
        "the control case did not move ({moved}), so the out-of-play assertion \
         above proves nothing"
    );
}

/// ⭐⭐ HITFALL: THE ATTACKER FAST-FALLS THE FRAME THE FREEZE ENDS.
///
/// The parity inventory listed *"Hitfall"* as absent. It is not: nothing gates
/// fast-fall on being mid-move or mid-hit — `can_fast_fall` is the ability flag
/// and nothing else — so a player who holds down through the freeze of their own
/// connecting aerial falls fast on the first live tick. This measures that
/// rather than asserting it, and guards it, because "the mechanic is already
/// reachable" is exactly the claim that rots.
///
/// ⛔ THE CONTROL IS THE SAME BODY WITH THE STICK NEUTRAL. A body that fell fast
/// for some other reason — gravity, a stale flag — would satisfy the headline
/// while the press did nothing.
#[test]
fn an_attacker_holding_down_fast_falls_the_tick_its_hitlag_ends() {
    let mut frozen = BodyCombat {
        hitstop_timer: 0.08,
        ..BodyCombat::default()
    };
    let mut live = BodyCombat::default();

    // A body that can fast-fall at all, airborne, with no horizontal travel to
    // confuse the reading.
    let airborne = || {
        let mut abilities = ae::AbilitySet::basic();
        abilities.fast_fall = true;
        ae::BodyClusterScratch::new_with_abilities(ae::Vec2::ZERO, abilities)
    };
    let holding_down = ae::InputState {
        movement: ae::ActionEdges::EMPTY.with(
            ae::MovementAction::FastFall,
            ae::Edge {
                pressed: true,
                held: true,
                released: false,
            },
        ),
        ..Default::default()
    };

    // Through the freeze, then one live tick — the press held the whole way, as
    // a player mashing down through their own hitlag would.
    let mut body = airborne();
    let frozen_travel = stepped(
        &mut body,
        &mut frozen,
        ae::MovementTuning::default(),
        holding_down.clone(),
    );
    assert_eq!(
        frozen_travel,
        ae::Vec2::ZERO,
        "the body moved DURING hitlag, so the freeze is not a freeze and the \
         reading below is not about the press"
    );
    let after_freeze = stepped(
        &mut body,
        &mut live,
        ae::MovementTuning::default(),
        holding_down,
    );

    // The control: same body, same two steps, stick neutral.
    let mut neutral_body = airborne();
    stepped(
        &mut neutral_body,
        &mut frozen,
        ae::MovementTuning::default(),
        ae::InputState::default(),
    );
    let neutral = stepped(
        &mut neutral_body,
        &mut live,
        ae::MovementTuning::default(),
        ae::InputState::default(),
    );

    assert!(
        after_freeze.y > neutral.y,
        "holding down through the freeze fell {} against a neutral {} — the \
         press did not reach the first live tick, so there is no hitfall",
        after_freeze.y,
        neutral.y
    );
}

/// ASDI IS PAID ONCE PER FREEZE, ON THE FAR SIDE OF IT — and the test's own
/// name says "once when the freeze lifts", which is the accurate half.
///
/// ⛔⛔ "ONCE PER HIT" IS THE WRONG SENTENCE and it was this heading until
/// 2026-08-25. `BodyCombat::asdi_owed` is a `bool`, so two hits landing before
/// the body next steps freely collapse to one payment — and a review reasonably
/// read the old wording as a defect the state shape could not satisfy.
///
/// ⭐ THE BOOL IS DELIBERATE, and `asdi_owed`'s own doc says why: a fresh hit
/// RE-ARMS the freeze rather than queueing behind it, so hits arriving during
/// hitlag extend ONE freeze episode. The body is displaced once when that
/// episode ends, which is the beat a player reads. A per-hit counter would pay
/// a multihit several displacements out of a single freeze, which is not the
/// mechanic.
///
/// ⇒ if the rule is ever changed to per-hit, the state has to change with it —
/// but do not change the state to satisfy a sentence that was only ever a
/// description.
///
/// Four arms, because the mechanic is defined as much by where it does NOT
/// apply as by where it does. The measurement is always the DIFFERENCE between
/// two otherwise identical runs — one declaring the step and one not — so
/// gravity, velocity and the SDI shift all cancel and what is left can only be
/// this rule.
#[test]
fn the_automatic_shift_is_paid_once_when_the_freeze_lifts() {
    const ASDI: f32 = 6.0;
    // Held UP, against gravity, so nothing else about the step can produce a
    // negative-y displacement and be mistaken for this one.
    let held = ae::InputState {
        axes: ae::LocalAxes::new(0.0, -1.0),
        ..Default::default()
    };

    // `sdi_step` is left at zero throughout: this test is about the OTHER
    // number, and a body that could also SDI would blur the two.
    let tuning = |asdi: f32| {
        let mut t = ae::MovementTuning::default();
        t.asdi_step = asdi;
        t
    };

    // Run the same three steps at two settings and answer (during, first free,
    // second free) y-displacement.
    let run = |asdi: f32| {
        let mut body = body_travelling();
        let mut combat = BodyCombat {
            hitstop_timer: 0.08,
            ..BodyCombat::default()
        };
        assert!(combat.is_in_hitlag(), "the fixture must arm the freeze");
        let during = stepped(&mut body, &mut combat, tuning(asdi), held).y;
        // The freeze lifts: this is what `decay_reaction_timers` would leave.
        combat.hitstop_timer = 0.0;
        let first_free = stepped(&mut body, &mut combat, tuning(asdi), held).y;
        let second_free = stepped(&mut body, &mut combat, tuning(asdi), held).y;
        (during, first_free, second_free)
    };

    let (off_during, off_first, off_second) = run(0.0);
    let (on_during, on_first, on_second) = run(ASDI);

    // ARM 1 — NOT DURING THE FREEZE. That is SDI's half; paying here would make
    // this one more SDI tick rather than a separate rule.
    assert!(
        (on_during - off_during).abs() < 0.01,
        "the automatic shift moved a body that was still frozen: {on_during} vs {off_during}"
    );

    // ARM 2 — THE PAYMENT, on the first step after the freeze lifts, in the
    // direction the stick is holding (up, so negative y).
    let paid = off_first - on_first;
    assert!(
        (paid - ASDI).abs() < 0.5,
        "the automatic shift did not pay its declared step when the freeze lifted: \
         paid {paid}, declared {ASDI}"
    );

    // ARM 3 — ONCE. A latch that never cleared would pay every step forever,
    // which is a body that floats away rather than one that got nudged.
    assert!(
        (on_second - off_second).abs() < 0.01,
        "the automatic shift was paid twice for one hit: {on_second} vs {off_second}"
    );

    // ARM 4 — A BODY THAT DECLARES NOTHING IS UNTOUCHED, which is every body in
    // Ambition. Asserted against a freeze that really happened, so this is not
    // passing for want of a hit.
    let mut unaffected = body_travelling();
    let mut combat = BodyCombat {
        hitstop_timer: 0.08,
        ..BodyCombat::default()
    };
    let before = unaffected.kinematics.pos;
    stepped(&mut unaffected, &mut combat, tuning(0.0), held);
    combat.hitstop_timer = 0.0;
    stepped(&mut unaffected, &mut combat, tuning(0.0), held);
    assert!(
        unaffected.kinematics.pos.y >= before.y,
        "a body declaring no automatic shift rose anyway: {} -> {}",
        before.y,
        unaffected.kinematics.pos.y
    );
}
