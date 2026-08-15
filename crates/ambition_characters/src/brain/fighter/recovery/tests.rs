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
