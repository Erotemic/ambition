//! The lowering's own properties. Calibration lives in the ladder rigs; these
//! assert that the answer comes from the BODY and from the SURFACES, and
//! nowhere else.

use super::*;
use ambition_characters::perception::{PerceivedSolid, SelfView, StageView};

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

/// A lens over a body that authors no route at all — which is every seat that is
/// not a platform fighter, and the case the lens must keep answering unchanged.
fn bare_lens(view: &WorldView, abilities: ae::AbilitySet) -> Option<RecoveryLens> {
    RecoveryLens::from_view(view, kit(abilities), &[], DT)
}

fn with_an_air_jump() -> ae::AbilitySet {
    ae::AbilitySet {
        double_jump: true,
        ..ae::AbilitySet::basic()
    }
}

/// THE FALSIFIER FOR THE WHOLE SLICE: same place, different body, different
/// verdict.
///
/// Position, velocity, geometry, gravity and the unspent air-jump COUNT are
/// byte-identical between the two probes. The only difference is one boolean in
/// the body's own kit — whether it owns the mid-air jump verb at all — and the
/// movement kernel gates the jump on the verb AND the budget together
/// (`simulation.rs`: `abilities.double_jump && air_jumps_available > 0`).
///
/// this is what the refused *"airborne, below the lip, outside the span
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

    let grounded_kit = bare_lens(&view, ae::AbilitySet::basic())
        .expect("the stage is known and gravity is non-zero");
    let outlook = grounded_kit.outlook(at);
    assert!(
        !outlook.regained(),
        "a body with no mid-air jump falls past the shelf it is already below and \
         out of the envelope, but the probe reported {outlook:?}"
    );

    let jumping_kit =
        bare_lens(&view, with_an_air_jump()).expect("the stage is known and gravity is non-zero");
    let outlook = jumping_kit.outlook(at);
    assert!(
        outlook.regained(),
        "the SAME fall, by a body that owns the mid-air jump, climbs back over \
         the shelf and lands on it — got {outlook:?}"
    );
}

/// And the recovery came from the SURFACE, not from a permissive probe.
///
/// Without this the test above passes for a lens that answered `Regained` to everything.
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
    let lens = bare_lens(&empty, with_an_air_jump()).expect("an empty stage is still a stage");
    let outlook = lens.outlook(at);
    assert!(
        !outlook.regained(),
        "poison: with nothing to land on, the jump buys altitude and nothing \
         else — got {outlook:?}"
    );
}

/// A body standing on the shelf is recovered, and says so immediately.
///
/// Cheap on purpose: the probe's first effort is "stand still", so a supported
/// body costs one kernel step. This is what makes the veto affordable to ask on
/// every airborne line — the expensive answer is only paid for by a body that is
/// actually in trouble.
#[test]
fn a_body_already_on_the_shelf_regains_on_the_first_step() {
    let feet_on_the_shelf = ae::Vec2::new(400.0, 300.0);
    let view = shelf_stage(feet_on_the_shelf);
    let lens = bare_lens(&view, ae::AbilitySet::basic()).expect("the stage is known");
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

/// No stage, no envelope, no question. A view that names no room cannot say
/// where dying starts, and inventing one would be the brain deciding a world
/// fact it was never told.
#[test]
fn a_view_with_no_stage_builds_no_lens() {
    let mut view = shelf_stage(ae::Vec2::new(300.0, 330.0));
    view.stage = StageView::default();
    assert!(bare_lens(&view, with_an_air_jump()).is_none());
}

/// A straight-up route: rise only, no lateral component.
fn rise(speed: f32, after_s: f32) -> RecoveryLift {
    RecoveryLift {
        speed,
        side: 0.0,
        after_s,
    }
}

/// A lens over a body that owns the given routes, in the given order.
fn lens_with(view: &WorldView, abilities: ae::AbilitySet, routes: &[RecoveryLift]) -> RecoveryLens {
    RecoveryLens::from_view(view, kit(abilities), routes, DT).expect("the stage is known")
}

/// Steers, and owns nothing at all that climbs — so anything this body gets
/// home by came from a route rather than from a verb.
fn drifter() -> ae::AbilitySet {
    ae::AbilitySet {
        move_horizontal: true,
        ..ae::AbilitySet::NONE
    }
}

/// THE VETO NOW CONSIDERS THE MOVE THE BODY WOULD ACTUALLY THROW.
///
/// this is the header's own standing warning, cashed: *"a body that recovers
/// by … a recovery attack is not explored"*, which was sound only while no
/// fighter had one. A body with no jump verb at all is below the shelf and
/// falling — drift alone can never climb, so the buttons-only search is right to
/// report nothing — and the SAME body with the SAME kit gets home once the
/// search is allowed to spend the rise its repertoire commands.
///
/// both terms are observed, so a lens that answered `Regained` to everything
/// would fail the first half and a burst that did nothing would fail the second.
#[test]
fn a_kit_that_commands_a_rise_is_probed_with_it() {
    let start = ae::Vec2::new(230.0, 400.0);
    let view = shelf_stage(start);
    let at = RecoveryQuery {
        pos: start,
        vel: ae::Vec2::ZERO,
        air_jumps_left: 0,
    };

    let buttons_only = bare_lens(&view, drifter()).expect("the stage is known");
    let without = buttons_only.outlook(at);
    assert!(
        !without.regained(),
        "a body that can only drift cannot climb back to a shelf it is already \
         below, but the probe reported {without:?}"
    );

    // 900px/s against gravity is 180px of climb under the engine baseline, well
    // over the ~110px back up to the shelf's face — and the start is far enough
    // LEFT (the shelf spans x 340..460) that the rise clears the lip before the
    // drift, capped at 270px/s, carries the body into the span. Starting under
    // the edge would put the climb through the block's side.
    let armed = lens_with(&view, drifter(), &[rise(900.0, 0.15)]);
    let with = armed.best_route(at);
    assert!(
        with.regained(),
        "the same body, probed with the rise its own repertoire commands, gets \
         back — got {with:?}"
    );
    assert_eq!(
        with.route,
        Some(0),
        "and the verdict NAMES the route that did it — a caller that only \
         learned 'yes' could not press the move that earned the yes"
    );
}

/// AND A NEGATIVE STILL SAYS WHICH SEARCH PRODUCED IT. The lens's whole
/// honesty contract is that the veto is bounded by its policy; arming the search
/// has to widen the bound as well as the search, or a consumer comparing two
/// negatives is comparing two different questions and cannot tell.
#[test]
fn an_armed_negative_is_bounded_by_the_armed_search() {
    let start = ae::Vec2::new(230.0, 400.0);
    let view = shelf_stage(start);
    let at = RecoveryQuery {
        pos: start,
        vel: ae::Vec2::ZERO,
        air_jumps_left: 0,
    };
    // Far too weak to climb 84px (30² / 4500 = 0.2px), so it still fails.
    let feeble = lens_with(&view, drifter(), &[rise(30.0, 0.15)]);
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

/// A KIT WITH NO LIFT IS PROBED EXACTLY AS BEFORE. The identity case, pinned
/// so that every seat which is not a platform fighter keeps the search it has
/// always had — the change must add routes for bodies that authored one, not
/// alter the verdict for bodies that did not.
#[test]
fn a_kit_with_no_lift_probes_with_the_bare_policy() {
    let view = shelf_stage(ae::Vec2::new(300.0, 330.0));
    let lens = bare_lens(&view, with_an_air_jump()).expect("stage is known");
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

// ---------------------------------------------------------------------------
// A recovery whose problem is HORIZONTAL, and the affordance trap it exposes.
// ---------------------------------------------------------------------------

/// A 1200x800 stage whose only surface is far off to the right: `x` in
/// `650..1150`, top face at `y = 500`. A body starting high and far left is
/// ABOVE that face — the gap it has to close is lateral, and climbing does not
/// close a lateral gap.
fn distant_ledge_stage(airborne_at: ae::Vec2) -> WorldView {
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
            bounds: ae::Aabb::new(ae::Vec2::new(600.0, 400.0), ae::Vec2::new(600.0, 400.0)),
        },
        terrain: vec![PerceivedSolid {
            aabb: ae::Aabb::new(ae::Vec2::new(900.0, 516.0), ae::Vec2::new(250.0, 16.0)),
            kind: SolidKind::Solid,
        }],
        ..Default::default()
    }
}

/// A grapple line: 980px/s across, 300px/s up, after a 0.16s windup. The whole
/// move is the lateral half; the rise is barely enough to keep the body level
/// while it travels.
fn grapple_route() -> RecoveryLift {
    RecoveryLift {
        speed: 300.0,
        side: 980.0,
        after_s: 0.16,
    }
}

/// A small rising aerial — the stall-and-juggle every platform fighter has one
/// of. It commands a REAL and LARGER against-gravity speed than the grapple
/// above, and it is not a way home from anywhere.
fn rising_aerial_route() -> RecoveryLift {
    rise(420.0, 0.10)
}

/// Far left of the ledge and well above its face, falling from rest.
fn far_from_the_ledge() -> RecoveryQuery {
    RecoveryQuery {
        pos: ae::Vec2::new(150.0, 200.0),
        vel: ae::Vec2::ZERO,
        air_jumps_left: 0,
    }
}

/// THE POISON FIXTURE: A TINY UPWARD ATTACK MUST NOT SUPPRESS A VIABLE
/// RECOVERY MERELY BECAUSE IT LIFTS.
///
/// A fighter whose way home is a grapple line advertises a small rise (its energy went
/// sideways), so any rising aerial in the same kit outranks it, becomes "the recovery", fails
/// to get anywhere, and the route that would have worked is never explored.
///
/// the invariant, stated as behaviour: adding a lifting move to a kit
/// that already has a working route must not change the verdict, and the verdict
/// must still NAME the route that works.
///
/// three terms are observed, so this cannot pass vacuously:
/// 1. the grapple alone gets home (there is a route to suppress);
/// 2. the rising aerial alone does NOT (it really is the useless one, so the
///    assertion is about suppression rather than about two routes that both
///    happen to work);
/// 3. with both present, and the aerial FIRST — the order `lifting_candidates`
///    produces, because 420 > 300 — the verdict is unchanged and names the
///    grapple.
#[test]
fn a_tiny_lifting_move_does_not_suppress_a_viable_recovery() {
    let at = far_from_the_ledge();
    let view = distant_ledge_stage(at.pos);

    // (0) The baseline: no route at all. Drift is capped at 270px/s and the fall
    // lasts about half a second, so the body covers a fifth of the gap.
    let bare = bare_lens(&view, drifter()).expect("the stage is known");
    assert!(
        !bare.best_route(at).regained(),
        "drift alone cannot cross 500px here, or nothing below is measuring a \
         route"
    );

    // (1) The real recovery, alone.
    let alone = lens_with(&view, drifter(), &[grapple_route()]);
    let solo = alone.best_route(at);
    assert!(
        solo.regained(),
        "the grapple is supposed to be a WORKING route — got {solo:?}"
    );
    assert_eq!(solo.route, Some(0));

    // (2) The rising aerial, alone. A bigger against-gravity number and no way
    // home: 420px/s buys 39px of climb over an empty stage.
    let aerial_only = lens_with(&view, drifter(), &[rising_aerial_route()]);
    assert!(
        !aerial_only.best_route(at).regained(),
        "poison: the aerial must be genuinely useless from here, or the test \
         below is comparing two routes that both work"
    );
    assert!(
        rising_aerial_route().speed > grapple_route().speed,
        "poison: the aerial must OUTRANK the grapple on the scalar, or nothing \
         here reproduces the trap — {} vs {}",
        rising_aerial_route().speed,
        grapple_route().speed
    );

    // (3) THE INVARIANT. Both routes, aerial first.
    let both = lens_with(&view, drifter(), &[rising_aerial_route(), grapple_route()]);
    let verdict = both.best_route(at);
    assert!(
        verdict.regained(),
        "a kit that gained a small rising aerial lost its recovery: the search \
         stopped at the move with the biggest number instead of the move that \
         works — got {verdict:?}"
    );
    assert_eq!(
        verdict.route,
        Some(1),
        "and it must NAME the grapple. A caller told only 'yes' would press the \
         aerial it was offered first and die holding a working recovery"
    );
}

/// AND THE LATERAL HALF IS WHAT DID THE WORK — not the rise, and not the
/// probe being generous.
///
/// the sharpest poison available: take the grapple's own numbers and delete
/// only `side`. Same speed, same windup, same body, same stage. If that still
/// gets home, the side component is decorative and everything above passes for
/// the wrong reason.
#[test]
fn the_grapples_lateral_half_is_the_half_that_gets_home() {
    let at = far_from_the_ledge();
    let view = distant_ledge_stage(at.pos);

    let whole = lens_with(&view, drifter(), &[grapple_route()]);
    assert!(whole.best_route(at).regained());

    let shadow = grapple_route();
    let de_sided = lens_with(&view, drifter(), &[rise(shadow.speed, shadow.after_s)]);
    let outlook = de_sided.best_route(at);
    assert!(
        !outlook.regained(),
        "poison: the grapple's vertical shadow is a 300px/s hop that buys 20px \
         of altitude — if the probe brings THAT home, it is not modelling the \
         move at all. Got {outlook:?}"
    );
}

/// A BODY THAT IS ALREADY GETTING HOME IS TOLD IT NEEDS NOTHING.
///
/// the buttons-only baseline runs FIRST, so `route: None` beside a positive
/// means *"drift and jump is enough"*. That is a real fighting-game fact —
/// spending your recovery early is how you lose to an edgeguard — and it fell
/// out of the search order rather than being encoded as a rule.
#[test]
fn a_body_that_needs_no_route_is_told_so() {
    let on_the_ledge = ae::Vec2::new(900.0, 484.0);
    let view = distant_ledge_stage(on_the_ledge);
    let lens = lens_with(
        &view,
        ae::AbilitySet::basic(),
        &[rising_aerial_route(), grapple_route()],
    );
    let verdict = lens.best_route(RecoveryQuery {
        pos: on_the_ledge,
        vel: ae::Vec2::ZERO,
        air_jumps_left: 0,
    });
    assert!(verdict.regained());
    assert_eq!(
        verdict.route, None,
        "a body standing on the stage must not be told to throw its recovery — \
         got {verdict:?}"
    );
}

/// THE COST IS BOUNDED BY A PREFIX, NOT BY THE KIT'S SIZE.
///
/// every route is a whole `probe_recovery` and the lens is queried per rolled
/// line, so an unbounded route list would make the veto's cost a function of how
/// many moves a character authors. The cut is a PREFIX of
/// `lifting_candidates`' deterministic order, so which routes get probed never
/// depends on iteration luck.
#[test]
fn a_lens_probes_at_most_the_bounded_prefix_of_routes() {
    let at = far_from_the_ledge();
    let view = distant_ledge_stage(at.pos);
    // Three useless rises in front of the one that works pushes the grapple past
    // the cut, and the verdict must then be honest about not having found it
    // rather than silently searching forever.
    let mut routes = vec![rising_aerial_route(); MAX_PROBED_ROUTES];
    routes.push(grapple_route());
    let lens = lens_with(&view, drifter(), &routes);
    assert!(
        !lens.best_route(at).regained(),
        "a route past the bound must be UNSEARCHED, not silently searched — the \
         cost of the veto is what this bound exists to hold"
    );
    // And the same list with the grapple inside the bound finds it, so the bound
    // is a cut and not a bug.
    let inside = lens_with(&view, drifter(), &[rising_aerial_route(), grapple_route()]);
    assert!(inside.best_route(at).regained());
}
