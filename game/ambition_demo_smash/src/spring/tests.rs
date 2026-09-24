//! The re-arm matters: a launch does not move a body out of the plate's box on
//! the same tick, so without a re-arm a three-use plate spends every use in
//! three frames.

use super::*;
use ambition_platformer2d::actor::MatchSeat;

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    app.add_message::<ActorActionMessage>();
    // The cue channel: without this registered message, both spring systems
    // fail parameter validation and are silently dropped.
    app.add_message::<ambition_platformer2d::vfx::vfx::VfxMessage>();
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
        vfx: "test_plate".to_string(),
        // Up is negative y, as everywhere in this codebase.
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
        .write_message(ActorActionMessage { actor, request, move_instance: None });
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
    // Walk him off it. A dropper is inside his own plate by construction (it
    // lands 18px away; the tolerance is 32), so otherwise this measures him,
    // not the walker. This is why `arm_s` exists.
    app.world_mut()
        .entity_mut(engineer)
        .insert(ae::BodyKinematics {
            pos: ae::Vec2::new(600.0, -200.0),
            facing: 1.0,
            ..Default::default()
        });

    // The plate is at the dropper's position plus the offset,
    // `(0, -200) + (0, 18)`, not at the world origin.
    let walker = body(&mut app, 0, ae::Vec2::new(0.0, -182.0));
    // Past the arming delay: the plate is inert for 0.30s so its dropper can
    // step off (see `PlacedSpring::arm_s`).
    for _ in 0..20 {
        app.update();
    }
    let vel = app.world().get::<ae::BodyKinematics>(walker).unwrap().vel;
    assert!(vel.y < -800.0, "the plate did not throw him: {vel:?}");
    assert_eq!(plates(&mut app)[0].uses_left, 2, "it spent more than one use");
}

/// It throws anybody, so it is stage, not kit. A plate that served only its
/// dropper would be a second recovery.
#[test]
fn the_plate_throws_the_fighter_who_dropped_it_too() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, 0.0));
    drop_plate(&mut app, engineer);
    // He stands right over it, and the plate is arming, so it must not answer
    // him yet.
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

/// The re-arm: without it, one body standing still spends every use at once.
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
        // Hold him there: the launch sets velocity, and nothing integrates.
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

/// The launch replaces the arriving velocity. Other fixtures start at rest,
/// where `vel = launch` and `vel += launch` agree. An additive plate would
/// throw a fast-falling body less far than a walking one.
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
    // Arriving hard downward: the fast-fall case, where adding would cancel
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

/// Two fighters on one plate: the same one is launched in either spawn order.
///
/// A plate has one use to give; the winner must not depend on query order, or
/// peers can launch different fighters on a resimulated tick. Same geometry,
/// reversed spawn order, same outcome; "somebody was launched" would not catch
/// it.
#[test]
fn two_fighters_on_one_plate_launch_the_same_one_in_either_spawn_order() {
    let on_the_plate = ae::Vec2::new(0.0, 0.0);

    let launched_seat = |reversed: bool| -> usize {
        let mut app = app();
        let seats: [usize; 2] = if reversed { [1, 0] } else { [0, 1] };
        let bodies: Vec<(usize, Entity)> = seats
            .iter()
            .map(|&s| (s, body(&mut app, s, on_the_plate)))
            .collect();
        app.world_mut().spawn(PlacedSpring {
            vfx: String::new(),
            pos: on_the_plate,
            half_extents: ae::Vec2::new(22.0, 6.0),
            launch: ae::Vec2::new(0.0, -900.0),
            remaining_s: 8.0,
            uses_left: 1,
            rearm_s: 0.0,
            arm_s: 0.0,
        });
        app.update();
        let moved: Vec<usize> = bodies
            .iter()
            .filter(|(_, e)| {
                app.world()
                    .get::<ae::BodyKinematics>(*e)
                    .is_some_and(|k| k.vel.y < -1.0)
            })
            .map(|(s, _)| *s)
            .collect();
        assert_eq!(
            moved.len(),
            1,
            "a one-use plate launched {} bodies (reversed={reversed}) — the use \
             count is not what limits it",
            moved.len()
        );
        moved[0]
    };

    let forward = launched_seat(false);
    let backward = launched_seat(true);
    assert_eq!(
        forward, backward,
        "the plate launched seat {forward} when the fighters were spawned in one \
         order and seat {backward} in the other — the winner is Bevy's iteration \
         order, so two peers resimulating this tick can launch different fighters"
    );
}

/// A plate announces itself when it arrives and when it fires.
///
/// `PlacedSpring` draws nothing of its own (the remote mine gets a sprite as a
/// `GroundItem`), so an unannounced plate is an ambush. Placement is for the
/// other player to see; firing lets the launched player attribute the throw.
///
/// This covers only the announcements, not a visible plate at rest (that would
/// be a content decision, like the mine's `GroundItem` art).
#[test]
fn a_plate_with_an_authored_cue_announces_both_its_arrival_and_its_launch() {
    // One cursor for the whole run: a fresh cursor per tick re-reads the
    // double buffer and counts a cue twice.
    let mut seen =
        bevy::ecs::message::MessageCursor::<ambition_platformer2d::vfx::vfx::VfxMessage>::default();
    let mut cues = |app: &mut App| -> usize {
        let messages = app
            .world()
            .resource::<Messages<ambition_platformer2d::vfx::vfx::VfxMessage>>();
        seen.read(messages)
            .filter(|m| {
                matches!(
                    m,
                    ambition_platformer2d::vfx::vfx::VfxMessage::Effect { .. }
                )
            })
            .count()
    };

    let mut placed = app();
    let dropper = body(&mut placed, 0, ae::Vec2::new(0.0, -200.0));
    let mut authored = params();
    authored.vfx = "oil_slick".to_string();
    placed.world_mut().write_message(ActorActionMessage {
        actor: dropper,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::Special(PLACE_SPRING.to_string()),
            params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&authored)
                .expect("spring params serialize"),
        },
        move_instance: None,
    });
    placed.update();
    assert_eq!(
        cues(&mut placed),
        1,
        "the plate arrived silently — the other player has no way to know it is there"
    );

    // Walk the dropper off first: he is inside his own plate by construction,
    // and would spend its single use before the victim exists.
    placed
        .world_mut()
        .entity_mut(dropper)
        .insert(ae::BodyKinematics {
            pos: ae::Vec2::new(600.0, -200.0),
            facing: 1.0,
            ..Default::default()
        });
    // On the plate: the drop offset `(0.0, 18.0)` puts it at -182 from a body
    // at -200, as in the throw test.
    let _victim = body(&mut placed, 1, ae::Vec2::new(0.0, -182.0));
    // Harvest per tick: messages are double-buffered, so the fire cue (around
    // tick 18) is gone by tick 24.
    let mut fired = 0usize;
    for _ in 0..24 {
        placed.update();
        fired += cues(&mut placed);
    }
    assert_eq!(
        fired, 1,
        "the plate fired silently — the fighter it launched was thrown by nothing"
    );

    // Poison guard. `author_place_spring` refuses an empty announcement, so
    // the control drives the adapter directly with an empty cue (a hand-built
    // `PlaceSpringParams` could still reach this) and asserts silence. That
    // proves the assertions above depend on the authored field.
    let mut quiet = app();
    let hand = body(&mut quiet, 0, ae::Vec2::new(0.0, -200.0));
    quiet.world_mut().write_message(ActorActionMessage {
        actor: hand,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::Special(PLACE_SPRING.to_string()),
            params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&{
                let mut silent = params();
                silent.vfx = String::new();
                silent
            })
            .expect("spring params serialize"),
        },
        move_instance: None,
    });
    quiet.update();
    let quiet_cues = {
        let messages = quiet
            .world()
            .resource::<Messages<ambition_platformer2d::vfx::vfx::VfxMessage>>();
        let mut cursor = messages.get_cursor();
        cursor
            .read(messages)
            .filter(|m| {
                matches!(
                    m,
                    ambition_platformer2d::vfx::vfx::VfxMessage::Effect { .. }
                )
            })
            .count()
    };
    assert_eq!(
        quiet_cues, 0,
        "a plate that authored NO cue announced itself anyway, so the field is \
         decoration and the assertions above prove nothing about authoring"
    );
}

/// Authoring a silent plate is refused at the seam.
///
/// `PlaceSpringParams::vfx` is required and asserted, so an author cannot
/// omit it; a default would be the invisible-ambush state the field exists to
/// end.
#[test]
#[should_panic(expected = "announces nothing")]
fn a_plate_authored_with_no_cue_is_refused() {
    let mut silent = params();
    silent.vfx = String::new();
    let _ = ambition_platformer2d::entity_catalog::smash_spring::author_place_spring(
        ambition_entity_catalog::authoring::hitless_special(
            "silent_plate",
            "special",
            0.0,
            0.10,
        ),
        0.05,
        silent,
    );
}
