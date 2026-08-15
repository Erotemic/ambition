//! The lowering's own properties. Calibration lives in the ladder rigs; these
//! assert that the answer comes from the BODY and from the SURFACES, and
//! nowhere else.

use super::*;
use crate::perception::{PerceivedSolid, SelfView, StageView};

const DT: f32 = 1.0 / 60.0;

/// A 800x600 stage carrying one shelf: `x` in `340..460`, top face at `y = 316`.
/// Everything else is void, so a body that misses it leaves the envelope.
fn shelf_stage(airborne_at: ae::Vec2) -> WorldView {
    WorldView {
        self_view: SelfView {
            pos: airborne_at,
            gravity_down: ae::Vec2::new(0.0, 1.0),
            half_extent: ae::Vec2::new(12.0, 16.0),
            alive: true,
            on_ground: false,
            health_max: 100,
            ..Default::default()
        },
        stage: StageView {
            bounds: ae::Aabb::new(ae::Vec2::new(400.0, 300.0), ae::Vec2::new(400.0, 300.0)),
        },
        terrain: vec![PerceivedSolid {
            aabb: ae::Aabb::new(ae::Vec2::new(400.0, 332.0), ae::Vec2::new(60.0, 16.0)),
            kind: SolidKind::Solid,
        }],
        ..Default::default()
    }
}

fn kit(abilities: ae::AbilitySet) -> BodyKit {
    BodyKit {
        abilities,
        movement: ae::MovementTuning::default(),
        // The bodies in these fixtures author no rising move: the lens must keep
        // answering for a plain drift-and-jump kit, which is what every seat
        // that is not a platform fighter still has.
        lift: None,
    }
}

fn with_an_air_jump() -> ae::AbilitySet {
    ae::AbilitySet {
        double_jump: true,
        ..ae::AbilitySet::basic()
    }
}

/// **THE FALSIFIER FOR THE WHOLE SLICE: same place, different body, different
/// verdict.**
///
/// Position, velocity, geometry, gravity and the unspent air-jump COUNT are
/// byte-identical between the two probes. The only difference is one boolean in
/// the body's own kit — whether it owns the mid-air jump verb at all — and the
/// movement kernel gates the jump on the verb AND the budget together
/// (`simulation.rs`: `abilities.double_jump && air_jumps_available > 0`).
///
/// ⛔ this is what the refused *"airborne, below the lip, outside the span ⇒
/// already dead"* rule could not do. That predicate reads only the position, so
/// both of these bodies would get the same answer and the answer would be wrong
/// for one of them. If this test ever passes with both verdicts equal, the
/// verdict has stopped coming from the body and the slice has failed.
#[test]
fn the_same_position_gets_opposite_verdicts_from_two_different_kits() {
    // Left of the shelf's span and already below its top face, falling.
    let start = ae::Vec2::new(300.0, 330.0);
    let view = shelf_stage(start);
    let at = RecoveryQuery {
        pos: start,
        vel: ae::Vec2::ZERO,
        air_jumps_left: 1,
    };

    let grounded_kit = RecoveryLens::from_view(&view, kit(ae::AbilitySet::basic()), DT)
        .expect("the stage is known and gravity is non-zero");
    let outlook = grounded_kit.outlook(at);
    assert!(
        !outlook.regained(),
        "a body with no mid-air jump falls past the shelf it is already below and \
         out of the envelope, but the probe reported {outlook:?}"
    );

    let jumping_kit = RecoveryLens::from_view(&view, kit(with_an_air_jump()), DT)
        .expect("the stage is known and gravity is non-zero");
    let outlook = jumping_kit.outlook(at);
    assert!(
        outlook.regained(),
        "the SAME fall, by a body that owns the mid-air jump, climbs back over \
         the shelf and lands on it — got {outlook:?}"
    );
}

/// **And the recovery came from the SURFACE, not from a permissive probe.**
///
/// The poison is the shelf itself: take it away and the identical body with the
/// identical kit from the identical place must be reported unrecovered. Without
/// this the test above passes for a lens that answered `Regained` to everything.
#[test]
fn taking_the_shelf_away_takes_the_recovery_with_it() {
    let start = ae::Vec2::new(300.0, 330.0);
    let at = RecoveryQuery {
        pos: start,
        vel: ae::Vec2::ZERO,
        air_jumps_left: 1,
    };

    let mut empty = shelf_stage(start);
    empty.terrain.clear();
    let lens = RecoveryLens::from_view(&empty, kit(with_an_air_jump()), DT)
        .expect("an empty stage is still a stage");
    let outlook = lens.outlook(at);
    assert!(
        !outlook.regained(),
        "poison: with nothing to land on, the jump buys altitude and nothing \
         else — got {outlook:?}"
    );
}

/// **A body standing on the shelf is recovered, and says so immediately.**
///
/// Cheap on purpose: the probe's first effort is "stand still", so a supported
/// body costs one kernel step. This is what makes the veto affordable to ask on
/// every airborne line — the expensive answer is only paid for by a body that is
/// actually in trouble.
#[test]
fn a_body_already_on_the_shelf_regains_on_the_first_step() {
    let feet_on_the_shelf = ae::Vec2::new(400.0, 300.0);
    let view = shelf_stage(feet_on_the_shelf);
    let lens = RecoveryLens::from_view(&view, kit(ae::AbilitySet::basic()), DT)
        .expect("the stage is known");
    let outlook = lens.outlook(RecoveryQuery {
        pos: feet_on_the_shelf,
        vel: ae::Vec2::ZERO,
        air_jumps_left: 0,
    });
    assert!(
        outlook.regained(),
        "a body resting on the shelf has already recovered — got {outlook:?}"
    );
}

/// **No stage, no envelope, no question.** A view that names no room cannot say
/// where dying starts, and inventing one would be the brain deciding a world
/// fact it was never told.
#[test]
fn a_view_with_no_stage_builds_no_lens() {
    let mut view = shelf_stage(ae::Vec2::new(300.0, 330.0));
    view.stage = StageView::default();
    assert!(RecoveryLens::from_view(&view, kit(with_an_air_jump()), DT).is_none());
}

/// A kit that also carries an authored way up.
fn kit_with_lift(abilities: ae::AbilitySet, speed: f32, after_s: f32) -> BodyKit {
    BodyKit {
        lift: Some(RecoveryLift { speed, after_s }),
        ..kit(abilities)
    }
}

/// **THE VETO NOW CONSIDERS THE MOVE THE BODY WOULD ACTUALLY THROW.**
///
/// ⛔ this is the header's own standing warning, cashed: *"a body that recovers
/// by … a recovery attack is not explored"*, which was sound only while no
/// fighter had one. A body with no jump verb at all is below the shelf and
/// falling — drift alone can never climb, so the buttons-only search is right to
/// report nothing — and the SAME body with the SAME kit gets home once the
/// search is allowed to spend the rise its repertoire commands.
///
/// ⭐ both terms are observed, so a lens that answered `Regained` to everything
/// would fail the first half and a burst that did nothing would fail the second.
#[test]
fn a_kit_that_commands_a_rise_is_probed_with_it() {
    // Steers, and owns nothing that climbs.
    let drifter = ae::AbilitySet {
        move_horizontal: true,
        ..ae::AbilitySet::NONE
    };
    let start = ae::Vec2::new(300.0, 400.0);
    let view = shelf_stage(start);
    let at = RecoveryQuery {
        pos: start,
        vel: ae::Vec2::ZERO,
        air_jumps_left: 0,
    };

    let buttons_only =
        RecoveryLens::from_view(&view, kit(drifter), DT).expect("the stage is known");
    let without = buttons_only.outlook(at);
    assert!(
        !without.regained(),
        "a body that can only drift cannot climb back to a shelf it is already \
         below, but the probe reported {without:?}"
    );

    // 900px/s against gravity is 180px of climb under the engine baseline —
    // comfortably over the ~84px back up to the shelf's face.
    let armed = RecoveryLens::from_view(&view, kit_with_lift(drifter, 900.0, 0.15), DT)
        .expect("the stage is known");
    let with = armed.outlook(at);
    assert!(
        with.regained(),
        "the same body, probed with the rise its own repertoire commands, gets \
         back — got {with:?}"
    );
}

/// **AND A NEGATIVE STILL SAYS WHICH SEARCH PRODUCED IT.** The lens's whole
/// honesty contract is that the veto is bounded by its policy; arming the search
/// has to widen the bound as well as the search, or a consumer comparing two
/// negatives is comparing two different questions and cannot tell.
#[test]
fn an_armed_negative_is_bounded_by_the_armed_search() {
    let drifter = ae::AbilitySet {
        move_horizontal: true,
        ..ae::AbilitySet::NONE
    };
    let start = ae::Vec2::new(300.0, 400.0);
    let view = shelf_stage(start);
    let at = RecoveryQuery {
        pos: start,
        vel: ae::Vec2::ZERO,
        air_jumps_left: 0,
    };
    // Far too weak to climb 84px (30² / 4500 = 0.2px), so it still fails.
    let feeble = RecoveryLens::from_view(&view, kit_with_lift(drifter, 30.0, 0.15), DT)
        .expect("the stage is known");
    let bound = feeble
        .outlook(at)
        .bounded_by()
        .expect("a body that found no support is bounded by its search");
    assert!(
        bound.policy.burst.is_some(),
        "an armed search that failed must not be reported as the bare one"
    );
    assert_ne!(
        bound.policy,
        ae::movement::recovery::RecoveryPolicy::DRIFT_AND_JUMP
    );
}

/// **A KIT WITH NO LIFT IS PROBED EXACTLY AS BEFORE.** The identity case, pinned
/// so that every seat which is not a platform fighter keeps the search it has
/// always had — the change must add routes for bodies that authored one, not
/// alter the verdict for bodies that did not.
#[test]
fn a_kit_with_no_lift_probes_with_the_bare_policy() {
    let view = shelf_stage(ae::Vec2::new(300.0, 330.0));
    let lens = RecoveryLens::from_view(&view, kit(with_an_air_jump()), DT).expect("stage is known");
    let bound = lens
        .outlook(RecoveryQuery {
            pos: ae::Vec2::new(300.0, 330.0),
            vel: ae::Vec2::ZERO,
            air_jumps_left: 0,
        })
        .bounded_by()
        .expect("a spent body below the shelf finds nothing");
    assert_eq!(
        bound.policy,
        ae::movement::recovery::RecoveryPolicy::DRIFT_AND_JUMP
    );
}
