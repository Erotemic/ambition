//! Jostle's ARBITRATION, on a hand-built app.
//!
//! What only a multi-body pass can decide: that two bodies standing inside each
//! other separate, that they do so symmetrically, that an undeclared world is
//! untouched, and — the one that matters most — that POSITION is never written.

use super::*;

const WIDE: f32 = 40.0;

fn tuning(jostle_accel: f32) -> ambition_combat::rules::ResolvedCombatTuning {
    ambition_combat::rules::ResolvedCombatTuning {
        jostle_accel,
        ..Default::default()
    }
}

fn body(app: &mut App, id: &str, x: f32, grounded: bool) -> Entity {
    app.world_mut()
        .spawn((
            SimId::placement(id),
            ae::BodyKinematics {
                pos: ae::Vec2::new(x, 0.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(WIDE, 60.0),
                facing: 1.0,
            },
            ae::BodyGroundState {
                on_ground: grounded,
                ..Default::default()
            },
        ))
        .id()
}

fn app(jostle_accel: f32) -> App {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    app.insert_resource(tuning(jostle_accel));
    app.add_systems(Update, resolve_jostle);
    app
}

fn kin(app: &App, e: Entity) -> ae::BodyKinematics {
    *app.world()
        .get::<ae::BodyKinematics>(e)
        .expect("the body kept its kinematics")
}

/// **The mechanic: two bodies standing inside each other are pushed apart.**
///
/// ⚠ asserted as SIGNS and a symmetry, not as a magnitude — the constant is a
/// feel value Jon has not tuned, and a test that pinned it would have to be
/// edited every time he did, which is how a guard becomes a chore and then a
/// `#[ignore]`.
#[test]
fn two_bodies_standing_inside_each_other_are_pushed_apart() {
    let mut app = app(600.0);
    // Half a body-width apart: a real overlap, not a graze.
    let left = body(&mut app, "left", -WIDE * 0.25, true);
    let right = body(&mut app, "right", WIDE * 0.25, true);
    app.update();

    assert!(
        kin(&app, left).vel.x < 0.0,
        "the left body was not pushed left, it took {}",
        kin(&app, left).vel.x
    );
    assert!(
        kin(&app, right).vel.x > 0.0,
        "the right body was not pushed right, it took {}",
        kin(&app, right).vel.x
    );
    assert!(
        (kin(&app, left).vel.x + kin(&app, right).vel.x).abs() < 1e-4,
        "the push was not symmetric: {} and {}",
        kin(&app, left).vel.x,
        kin(&app, right).vel.x
    );
}

/// ⛔⛔ **THE RULE JON ACTUALLY STATED: position is never written.**
///
/// The whole reason this is an acceleration is that a rewind must restore the
/// same answer from the same inputs. A version that separated bodies by moving
/// them would pass the test above and break that, so the claim is asserted
/// directly rather than inferred from the velocity one.
#[test]
fn jostle_never_writes_position() {
    let mut app = app(600.0);
    let left = body(&mut app, "left", -WIDE * 0.25, true);
    let right = body(&mut app, "right", WIDE * 0.25, true);
    let before = (kin(&app, left).pos, kin(&app, right).pos);
    app.update();

    assert_eq!(
        (kin(&app, left).pos, kin(&app, right).pos),
        before,
        "jostle moved a body instead of accelerating it — the kernel owns \
         position, and a pass that writes it is the pushout Jon's rule is about"
    );
}

/// **Undeclared is OFF, and off is byte-identical.** Every world that does not
/// ask for jostle must run exactly as it did; a pass that cost one float in the
/// baseline would be a platform-fighter rule charged to every game.
#[test]
fn a_world_that_declares_no_jostle_is_untouched() {
    let mut app = app(0.0);
    let left = body(&mut app, "left", -WIDE * 0.25, true);
    let right = body(&mut app, "right", WIDE * 0.25, true);
    app.update();

    assert_eq!(kin(&app, left).vel.x, 0.0);
    assert_eq!(kin(&app, right).vel.x, 0.0);
}

/// ⚠ **and with no rules resource at all** — the shape every non-combat
/// composition actually has, which `Option<Res<..>>` exists for. A `Res<..>`
/// here would panic the moment a room without combat ran this system.
#[test]
fn a_composition_with_no_combat_rules_at_all_is_untouched() {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    app.add_systems(Update, resolve_jostle);
    let left = body(&mut app, "left", -WIDE * 0.25, true);
    app.update();

    assert_eq!(kin(&app, left).vel.x, 0.0);
}

/// **Airborne bodies pass through each other**, which is the genre's rule: a
/// fighter juggling another has to be able to occupy the same space to keep
/// hitting them.
///
/// ⚠ the grounded pair is asserted in the SAME run, so this cannot pass by the
/// pass having done nothing at all.
#[test]
fn airborne_bodies_are_not_jostled_but_grounded_ones_still_are() {
    let mut app = app(600.0);
    let flying_a = body(&mut app, "flying_a", -WIDE * 0.25, false);
    let flying_b = body(&mut app, "flying_b", WIDE * 0.25, false);
    let standing_a = body(&mut app, "standing_a", 1000.0 - WIDE * 0.25, true);
    let standing_b = body(&mut app, "standing_b", 1000.0 + WIDE * 0.25, true);
    app.update();

    assert_eq!(
        kin(&app, flying_a).vel.x,
        0.0,
        "an airborne body was jostled"
    );
    assert_eq!(
        kin(&app, flying_b).vel.x,
        0.0,
        "an airborne body was jostled"
    );
    assert!(
        kin(&app, standing_a).vel.x < 0.0 && kin(&app, standing_b).vel.x > 0.0,
        "the grounded pair was not jostled either, so this test proved nothing"
    );
}

/// **Bodies merely touching are left alone.** Without this the push never
/// reaches zero — two fighters standing shoulder to shoulder would jitter
/// against each other forever, which reads as a physics bug rather than as
/// weight.
#[test]
fn bodies_that_only_touch_are_not_pushed() {
    let mut app = app(600.0);
    let left = body(&mut app, "left", -WIDE * 0.5, true);
    let right = body(&mut app, "right", WIDE * 0.5, true);
    app.update();

    assert_eq!(kin(&app, left).vel.x, 0.0);
    assert_eq!(kin(&app, right).vel.x, 0.0);
}

/// **Deeper overlap pushes harder.** The mechanic is proportional, so a body
/// shoved fully inside another separates faster than one barely inside — which
/// is what stops a deep overlap taking visibly longer to resolve than a shallow
/// one.
#[test]
fn a_deeper_overlap_pushes_harder() {
    let shallow = {
        let mut app = app(600.0);
        let a = body(&mut app, "a", -WIDE * 0.45, true);
        body(&mut app, "b", WIDE * 0.45, true);
        app.update();
        kin(&app, a).vel.x.abs()
    };
    let deep = {
        let mut app = app(600.0);
        let a = body(&mut app, "a", -WIDE * 0.05, true);
        body(&mut app, "b", WIDE * 0.05, true);
        app.update();
        kin(&app, a).vel.x.abs()
    };
    assert!(
        deep > shallow,
        "a deep overlap ({deep}) did not push harder than a shallow one \
         ({shallow}) — the push is not proportional to depth"
    );
}

/// ⛔ **Three bodies in one pile: each takes ONE combined push, and the middle
/// one's two pushes cancel.** The accumulate-then-apply split exists for this —
/// a pass that wrote velocity as it walked the pairs would let the second pair
/// read a velocity the first had already changed, making the answer depend on
/// pair order.
#[test]
fn a_body_between_two_others_takes_both_pushes_and_stays_put() {
    let mut app = app(600.0);
    let left = body(&mut app, "left", -WIDE * 0.4, true);
    let middle = body(&mut app, "middle", 0.0, true);
    let right = body(&mut app, "right", WIDE * 0.4, true);
    app.update();

    assert!(
        kin(&app, middle).vel.x.abs() < 1e-4,
        "the middle body's two pushes did not cancel, it took {}",
        kin(&app, middle).vel.x
    );
    assert!(
        kin(&app, left).vel.x < 0.0 && kin(&app, right).vel.x > 0.0,
        "the outer bodies were not pushed outward, so the cancellation above \
         proved nothing"
    );
}
