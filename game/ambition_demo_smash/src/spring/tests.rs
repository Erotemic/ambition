//! ⛔⛔ THE RE-ARM IS THE ONE NOBODY WOULD THINK TO TEST, and it is the reason a
//! three-use plate is not a one-frame catastrophe: a launch does not move a body
//! out of the plate's box on the tick it happens, so without a re-arm the plate
//! spends every use in three frames and reads as one enormous throw.

use super::*;
use ambition_platformer2d::actor::MatchSeat;

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    app.add_message::<ActorActionMessage>();
    let mut time = app
        .world_mut()
        .resource_mut::<ambition_platformer2d::time::WorldTime>();
    time.scaled_dt = 1.0 / 60.0;
    time.raw_dt = 1.0 / 60.0;
    app.add_systems(Update, (drop_authored_springs, fire_and_expire_springs).chain());
    app
}

fn body(app: &mut App, seat: usize, at: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: at,
                facing: 1.0,
                ..Default::default()
            },
            MatchSeat(seat),
        ))
        .id()
}

fn params() -> PlaceSpringParams {
    PlaceSpringParams {
        // Up is NEGATIVE y, as everywhere in this codebase.
        launch: (0.0, -900.0),
        half_extents: (22.0, 6.0),
        lifetime_s: 8.0,
        uses: 3,
        offset: (0.0, 18.0),
    }
}

fn drop_plate(app: &mut App, actor: Entity) {
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(PLACE_SPRING.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params())
            .expect("spring params serialize"),
    };
    app.world_mut()
        .write_message(ActorActionMessage { actor, request });
    app.update();
}

fn plates(app: &mut App) -> Vec<PlacedSpring> {
    app.world_mut()
        .query::<&PlacedSpring>()
        .iter(app.world())
        .cloned()
        .collect()
}

#[test]
fn the_plate_lands_where_the_move_asked_and_throws_who_steps_on_it() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, -200.0));
    drop_plate(&mut app, engineer);
    assert_eq!(plates(&mut app).len(), 1);
    // ⛔⛔ WALK HIM OFF IT. A dropper is INSIDE his own plate by construction —
    // it lands 18px from him and the tolerance is 32 — so leaving him there
    // makes this test measure HIM rather than the walker, which is what the
    // first two versions of it did. ⇒ That is not a fixture wrinkle: it is the
    // reason `arm_s` exists, discovered here.
    app.world_mut()
        .entity_mut(engineer)
        .insert(ae::BodyKinematics {
            pos: ae::Vec2::new(600.0, -200.0),
            facing: 1.0,
            ..Default::default()
        });

    // ⚠ THE PLATE IS AT THE DROPPER'S POSITION PLUS THE OFFSET, not at the
    // world origin — `(0, -200) + (0, 18)`. Getting this wrong is how the first
    // version of this test reported "the plate did not throw him" about a plate
    // 200px away, which is the geometry being wrong rather than the code.
    let walker = body(&mut app, 0, ae::Vec2::new(0.0, -182.0));
    // ⛔ PAST THE ARMING DELAY. The plate is inert for 0.30s so its dropper can
    // step off it — see `PlacedSpring::arm_s`, which a guard here found the need
    // for on the first run.
    for _ in 0..20 {
        app.update();
    }
    let vel = app.world().get::<ae::BodyKinematics>(walker).unwrap().vel;
    assert!(vel.y < -800.0, "the plate did not throw him: {vel:?}");
    assert_eq!(plates(&mut app)[0].uses_left, 2, "it spent more than one use");
}

/// ⭐⭐ IT THROWS ANYBODY, which is what makes it a piece of STAGE rather than a
/// piece of kit. A plate that served only its dropper would be a second recovery.
#[test]
fn the_plate_throws_the_fighter_who_dropped_it_too() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, 0.0));
    drop_plate(&mut app, engineer);
    // He is standing right over it — and the plate is ARMING, so it must not
    // answer him yet.
    app.update();
    assert_eq!(
        app.world().get::<ae::BodyKinematics>(engineer).unwrap().vel,
        ae::Vec2::ZERO,
        "the plate threw its dropper on the tick he dropped it"
    );
    for _ in 0..20 {
        app.update();
    }
    let vel = app.world().get::<ae::BodyKinematics>(engineer).unwrap().vel;
    assert!(
        vel.y < -800.0,
        "his own plate refused him, so it is kit and not stage: {vel:?}"
    );
}

/// ⛔ THE RE-ARM: without it, one body standing still spends every use at once.
#[test]
fn a_body_standing_on_it_does_not_spend_every_use_at_once() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, -200.0));
    drop_plate(&mut app, engineer);
    let loiterer = body(&mut app, 0, ae::Vec2::new(0.0, -182.0));
    // Three consecutive ticks standing in the box.
    for _ in 0..20 {
        app.update();
    }
    for _ in 0..3 {
        // Hold him there: the launch sets velocity, and nothing here integrates.
        app.world_mut()
            .entity_mut(loiterer)
            .insert(ae::BodyKinematics {
                pos: ae::Vec2::new(0.0, -182.0),
                facing: 1.0,
                ..Default::default()
            });
        app.update();
    }
    assert_eq!(
        plates(&mut app)[0].uses_left,
        2,
        "the plate spent more than one use on one continuous stand"
    );
}

#[test]
fn the_plate_is_taken_away_when_its_uses_run_out() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, 0.0));
    drop_plate(&mut app, engineer);
    // Long enough for three launches at a 0.25s re-arm.
    for _ in 0..(3.0 * 60.0) as usize {
        app.world_mut()
            .entity_mut(engineer)
            .insert(ae::BodyKinematics {
                pos: ae::Vec2::new(0.0, 18.0),
                facing: 1.0,
                ..Default::default()
            });
        app.update();
        if plates(&mut app).is_empty() {
            return;
        }
    }
    panic!("the plate outlived its three uses");
}

/// ⛔⛔ THE LAUNCH REPLACES WHATEVER YOU ARRIVED WITH, and this test exists
/// because a poison proved nothing held it: every other fixture here spawns a
/// body at rest, where `vel = launch` and `vel += launch` are the same line.
/// ⇒ A plate that ADDED would throw a fast-falling body less far than a walking
/// one, which is the opposite of what anybody expects from a spring — and it
/// would have shipped green.
#[test]
fn the_launch_replaces_the_speed_you_arrived_with() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, -200.0));
    drop_plate(&mut app, engineer);
    app.world_mut()
        .entity_mut(engineer)
        .insert(ae::BodyKinematics {
            pos: ae::Vec2::new(600.0, -200.0),
            facing: 1.0,
            ..Default::default()
        });
    let faller = body(&mut app, 0, ae::Vec2::new(0.0, -182.0));
    // Arriving hard DOWNWARD — the fast-fall case, where adding would cancel
    // most of the launch.
    app.world_mut()
        .entity_mut(faller)
        .insert(ae::BodyKinematics {
            pos: ae::Vec2::new(0.0, -182.0),
            vel: ae::Vec2::new(0.0, 700.0),
            facing: 1.0,
            ..Default::default()
        });
    for _ in 0..20 {
        app.update();
    }
    let vel = app.world().get::<ae::BodyKinematics>(faller).unwrap().vel;
    assert!(
        vel.y < -800.0,
        "a fast-faller was thrown {:?}, so the plate ADDED to his speed instead \
         of replacing it",
        vel
    );
}

#[test]
fn the_plate_is_taken_away_when_its_clock_runs_out() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, -400.0));
    drop_plate(&mut app, engineer);
    for _ in 0..(8.0 * 60.0) as usize + 4 {
        app.update();
    }
    assert!(plates(&mut app).is_empty(), "the plate outlived its clock");
}
