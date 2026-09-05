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
    // ⛔⛔ THE CUE CHANNEL, AND WITHOUT IT NEITHER SPRING SYSTEM RUNS AT ALL. A
    // world that does not register a message a system writes fails that system's
    // parameter validation and drops it silently — every test in this file went
    // red at once the moment the plate learned to announce itself. Third time
    // this shape has appeared in this demo, and it is always a whole file.
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

/// ⛔⛔ TWO FIGHTERS ON ONE PLATE: THE SAME ONE IS LAUNCHED WHICHEVER ORDER
/// THEY WERE SPAWNED IN.
///
/// A plate has ONE use to give and this loop used to `break` on the first
/// overlapping body with the seat ignored (`_seat`), so Bevy's iteration order
/// picked the winner. ⇒ Nobody authored that choice, and it is not stable across
/// a rollback resimulation — the two peers can resimulate the same tick and
/// launch different fighters, which is a desync with a plausible-looking cause.
///
/// ⭐ THE TEST IS THE REVIEW'S OWN SHAPE: identical geometry, REVERSED SPAWN
/// ORDER, same outcome. Asserting "somebody was launched" would pass on the
/// broken code; only comparing the two orders can see it.
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

/// ⛔⛔ A PLATE ANNOUNCES ITSELF WHEN IT ARRIVES AND WHEN IT FIRES.
///
/// `PlacedSpring` draws NOTHING of its own — no sprite, no effect — while the
/// remote mine is visible for free because it is a `GroundItem` and
/// `item_visuals` gives those a sprite. ⇒ Two objects a fighter puts on the
/// floor, one readable and one invisible, **and only the invisible one launches
/// you**. A plate nobody saw arrive is an ambush rather than a move.
///
/// ⭐ BOTH MOMENTS, AND THEY ARE DIFFERENT AUDIENCES. Placement is what the
/// OTHER player must see; firing is what the LAUNCHED player must be able to
/// attribute — without it a fighter is thrown by nothing.
///
/// ⚠ THIS IS THE ANNOUNCEMENT HALF ONLY. It does not make the plate visible
/// while it sits there; the shipped road for that is the mine's, a `GroundItem`
/// with authored art, and that is a content decision rather than a field.
#[test]
fn a_plate_with_an_authored_cue_announces_both_its_arrival_and_its_launch() {
    // ⛔⛔ ONE CURSOR, HELD ACROSS THE WHOLE RUN. A fresh cursor per tick
    // re-reads whatever is still in the double buffer, so a single cue counts
    // TWICE and the totals are quietly inflated — which is exactly what a first
    // version of this test reported (3 for 1). ⇒ The cursor is the thing that
    // remembers what has been seen; making a new one each tick throws that away.
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
    });
    placed.update();
    assert_eq!(
        cues(&mut placed),
        1,
        "the plate arrived silently — the other player has no way to know it is there"
    );

    // ⛔ WALK THE DROPPER OFF FIRST. He is INSIDE his own plate by construction —
    // it lands 18px away and the tolerance is 32 — so leaving him there spends
    // the single use before the victim exists, which is the same wrinkle the
    // throw test above records and which I walked straight into.
    placed
        .world_mut()
        .entity_mut(dropper)
        .insert(ae::BodyKinematics {
            pos: ae::Vec2::new(600.0, -200.0),
            facing: 1.0,
            ..Default::default()
        });
    // ON the plate: the drop offset is `(0.0, 18.0)` body-local, so it lands at
    // -182 from a body at -200. The same placement the throw test uses.
    let _victim = body(&mut placed, 1, ae::Vec2::new(0.0, -182.0));
    // ⛔⛔ HARVEST PER TICK. `Messages` are double-buffered, so the fire cue —
    // emitted the moment the plate arms, around tick 18 — is GONE by tick 24.
    // Reading once at the end saw nothing and reported that the plate fired
    // silently, which is the second time today this exact trap has produced a
    // convincing false failure.
    let mut fired = 0usize;
    for _ in 0..24 {
        placed.update();
        fired += cues(&mut placed);
    }
    assert_eq!(
        fired, 1,
        "the plate fired silently — the fighter it launched was thrown by nothing"
    );

    // ⛔ POISON GUARD, AND IT CHANGED SHAPE WHEN THE FIELD BECAME REQUIRED. It
    // used to author a plate with no cue and assert silence; that state is now
    // unrepresentable through `author_place_spring`, which refuses an empty
    // announcement outright. ⇒ So the control drives the ADAPTER directly with an
    // empty cue — the shape a hand-built `PlaceSpringParams` could still reach —
    // and asserts it stays quiet, which is what proves the two assertions above
    // are about the authored field rather than about a system that announces
    // unconditionally.
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

/// ⛔⛔ AUTHORING A SILENT PLATE IS REFUSED AT THE SEAM, NOT LEFT TO DISCIPLINE.
///
/// `PlaceSpringParams::vfx` was `Option<String>` with `#[serde(default)]` for one
/// commit, and a peer caught the shape in a sentence I had written myself: *"None
/// draws nothing, which is what every plate authored before this field existed
/// did."* ⇒ **The default value of the new field was exactly the invisible-ambush
/// state the field exists to end.** Both shipped authors set it; nothing made a
/// third.
///
/// ⭐ Required and asserted turns "an author remembered" into "an author could
/// not omit it" — the same move as gating the gravity modifier on its timer and
/// deriving `overlapped` rather than mirroring it.
#[test]
#[should_panic(expected = "announces nothing")]
fn a_plate_authored_with_no_cue_is_refused() {
    let mut silent = params();
    silent.vfx = String::new();
    let _ = ambition_platformer2d::characters::smash_spring::author_place_spring(
        ambition_platformer2d::characters::moveset_authoring::hitless_special(
            "silent_plate",
            "special",
            0.0,
            0.10,
        ),
        0.05,
        silent,
    );
}
