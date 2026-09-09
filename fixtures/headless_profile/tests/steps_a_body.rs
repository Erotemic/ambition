//! A9's minimum, as a behaviour rather than as a compile.
//!
//! ⛔⛔ COMPOSING IS NOT STEPPING, and a fixture that only asserted the app builds
//! would certify the profile while its simulation did nothing. The frontier's
//! sentence is *"a constructed body advancing against world geometry"* — three
//! claims, and the third is the one a build cannot make.

use ambition_platformer2d::app::prelude::*;
use ambition_platformer2d::bevy::prelude::*;
use headless_profile::HeadlessModule;

/// It composes with no capability selected at all.
#[test]
fn the_headless_profile_composes() {
    let app = PlatformerApp::headless()
        .mount(HeadlessModule)
        .try_build()
        .expect("the A9 minimum profile composes headless");
    assert!(
        app.get_schedule(FixedUpdate).is_some(),
        "a composed app runs a fixed-step simulation; its absence means no \
         engine was installed at all, which a did-it-panic test cannot \
         distinguish from success"
    );
}

/// ⛔⛔ THE RENDERER'S ABSENCE IS NOT ASSERTED HERE, AND THAT IS THE FINDING.
///
/// A first version of this file tried `app.get_sub_app(bevy::render::RenderApp)`
/// and did not compile: this consumer has no `bevy` dependency of its own, and
/// the `bevy` the facade re-exports at this profile is built without its render
/// feature, so `RenderApp` IS NOT A NAMEABLE TYPE from here. The absence is
/// enforced by the type system rather than by an assertion — which is stronger
/// than the test I set out to write, and worth saying instead of quietly
/// deleting.
///
/// ⇒ The two halves A9's acceptance names have two homes. COMPILE CLOSURE is
/// `the-featureless-facade-links-none-of-these` in
/// `scripts/check_absence_contracts.py`, which walks the feature-resolved tree.
/// RUNTIME INSTALLATION for a profile that DOES render is the map's own witness
/// (`every_room_the_map_calls_visited_has_its_visit_on_the_save`). This fixture
/// is the third fact neither of those can state: that the profile RUNS.

/// A BODY ADVANCING AGAINST WORLD GEOMETRY: the third claim, and the reason this
/// file exists rather than a second compile check.
///
/// ⛔⛔ **IT ASSERTS WHERE THE BODY CAME TO REST, not that it moved.** A first
/// version asserted motion and then stillness, and poisoning the floor — moving
/// the block 100,000px away — left it GREEN: the body settled at the ROOM BOUND
/// either way, so the assertion was about the room's edge and the floor had no
/// part in it. The reference fixture this room was copied from has the same
/// shape, which is why [`headless_profile::experience::room`] raises its floor
/// clear of the bound: the resting height now NAMES the surface that stopped it.
#[test]
fn a_body_falls_and_the_raised_floor_stops_it() {
    use ambition_platformer2d::engine_core::BodyKinematics;
    use headless_profile::experience::FLOOR_TOP;

    fn lowest_body_y(app: &mut App) -> Option<f32> {
        let world = app.world_mut();
        let mut bodies = world.query::<&BodyKinematics>();
        bodies
            .iter(world)
            .map(|kin| kin.pos.y)
            .min_by(|a, b| a.total_cmp(b))
    }

    let mut app = PlatformerApp::headless()
        .mount(HeadlessModule)
        .try_build()
        .expect("the A9 minimum profile composes headless");
    // ⚠ WARM UP UNTIL THE BODY EXISTS, bounded. A session spawns its playable
    // over several frames, and a fixed step count here would be a number that
    // works today — measured: nothing at one step, a body by two.
    let mut start = None;
    for _ in 0..60 {
        app.update();
        start = lowest_body_y(&mut app);
        if start.is_some() {
            break;
        }
    }
    let start = start.expect(
        "no body exists after 60 steps, so every assertion below would be vacuous",
    );
    assert!(
        start < FLOOR_TOP,
        "the body spawned at or below the floor (y {start}, floor top {FLOOR_TOP}),          so it never has to fall and this test measures nothing"
    );

    for _ in 0..120 {
        app.update();
    }
    let landed = lowest_body_y(&mut app).expect("the body vanished mid-simulation");

    assert!(
        landed > start,
        "the body has not fallen in 120 steps (y {start} -> {landed}); the profile          composes but does not simulate"
    );
    // ⛔ ON THE FLOOR. Falling THROUGH the only block this room authors is the
    // alternative, and it is what the poisoned run does: y=583.
    assert!(
        landed < FLOOR_TOP,
        "the body came to rest at y {landed}, at or past the floor top \
         {FLOOR_TOP} — it fell THROUGH the block this room authored, which is the \
         only piece of world geometry in the profile"
    );

    // ⛔ AND IT STAYED. Falling forever is also motion.
    for _ in 0..30 {
        app.update();
    }
    let after = lowest_body_y(&mut app).expect("the body vanished mid-simulation");
    assert!(
        (after - landed).abs() < 1.0,
        "the body is still moving after 150 steps (y {landed} -> {after})"
    );
}
