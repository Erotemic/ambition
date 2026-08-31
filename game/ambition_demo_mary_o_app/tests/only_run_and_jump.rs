//! Mary-O at home has RUN and JUMP, and nothing else.
//!
//! from smash in her game, and its messing things up there. She should only have
//! the run and jump in her game. And the run should double as the fireball
//! button when she has the lantern."*
//!
//! What kept that off her own speedway was supposed to be her `abilities: Some([RunJump])` row, and
//! it did not: `combat_actions` derived the Attack / Special slots from the MOVESET alone, so every
//! press answered.
//!
//! so this asserts on the TRIGGERABLE SET — what a move playback actually starts when the device
//! layer presses every combat button in every aim — and not on any field.

use bevy::prelude::*;

use ambition_demo_mary_o::movement::MaryOGait;
use ambition_demo_mary_o::powerups::{cinder_beacon, star_wand, MaryOSpark};
use ambition_demo_mary_o::test_course::TEST_COURSE_ROOM_ID;
use ambition_platformer2d::characters::equipment::WornEquipment;
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;

/// `app.update()` is a FRAME, not a tick — every loop here runs to a
/// condition under a ceiling rather than for a fixed count.
const LIVENESS_CAP: usize = 600;


/// Her real host, entering the fixture course (flat ground, no timing to go
/// stale) rather than 1-1.
fn boot() -> App {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(
        TEST_COURSE_ROOM_ID.to_string(),
    ));
    // the ordering lives in ONE place now — after the participant pipeline's routing stage and
    // before the frame→tick latch.
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..LIVENESS_CAP {
        app.update();
        if seated(&mut app).is_some() {
            // Let her settle onto the floor before anything is pressed.
            for _ in 0..30 {
                app.update();
            }
            return app;
        }
    }
    panic!("Mary-O never took a seat in her own demo");
}

fn seated(app: &mut App) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryPlayer>>();
    q.iter(app.world()).next()
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}

fn aim(ax: f32, ay: f32) -> ControlFrame {
    ControlFrame {
        axis_x: ax,
        axis_y: ay,
        aim_x: ax,
        aim_y: ay,
        left_pressed: ax < 0.0,
        right_pressed: ax > 0.0,
        up_pressed: ay > 0.0,
        down_pressed: ay < 0.0,
        ..ControlFrame::default()
    }
}

/// Every combat button the device layer can produce, in every aim, and the move
/// ids that answered. An entry here is a swing a player can trigger.
fn triggerable_swings(app: &mut App, body: Entity) -> Vec<String> {
    #[allow(clippy::type_complexity)]
    let buttons: [(&str, fn(&mut ControlFrame)); 5] = [
        ("attack", |f| {
            f.attack_pressed = true;
            f.attack_held = true;
        }),
        ("smash", |f| {
            f.attack_pressed = true;
            f.attack_held = true;
            f.attack_strength_hint = ambition_platformer2d::sim::AttackStrengthHint::Smash;
        }),
        ("special", |f| f.special_pressed = true),
        ("pogo", |f| f.pogo_pressed = true),
        ("projectile", |f| {
            f.projectile_pressed = true;
            f.projectile_held = true;
        }),
    ];
    let aims: [(&str, f32, f32); 5] = [
        ("neutral", 0.0, 0.0),
        ("forward", 1.0, 0.0),
        ("back", -1.0, 0.0),
        ("up", 0.0, 1.0),
        ("down", 0.0, -1.0),
    ];

    let mut found: Vec<String> = Vec::new();
    for (button, arm) in buttons {
        for (direction, ax, ay) in aims {
            // Press, then release, then let any started move play out — a swing
            // that starts on tick 30 counts exactly as much as one on tick 1.
            for tick in 0..30 {
                let mut frame = aim(ax, ay);
                if tick < 4 {
                    arm(&mut frame);
                }
                step(app, frame);
                if let Some(playback) = app.world().get::<MovePlayback>(body) {
                    let entry = format!("{button}/{direction} -> {}", playback.spec.id);
                    if !found.contains(&entry) {
                        found.push(entry);
                    }
                }
            }
        }
    }
    found
}

/// Hold a direction (optionally running) until her side speed stops climbing,
/// and report the top speed reached.
fn top_speed(app: &mut App, body: Entity, running: bool) -> f32 {
    let mut best = 0.0f32;
    let mut stalled = 0;
    for _ in 0..LIVENESS_CAP {
        step(
            app,
            ControlFrame {
                modifier_held: running,
                ..aim(1.0, 0.0)
            },
        );
        let speed = app
            .world()
            .get::<ae::BodyKinematics>(body)
            .map(|kin| kin.vel.x.abs())
            .unwrap_or(0.0);
        if speed > best + 0.5 {
            best = speed;
            stalled = 0;
        } else {
            best = best.max(speed);
            stalled += 1;
            if stalled > 60 {
                break;
            }
        }
    }
    best
}

/// THE GUARD. Nothing from the smash table answers a press at home, and the
/// two verbs she is supposed to have still do.
#[test]
fn mary_o_at_home_can_only_run_and_jump() {
    let mut app = boot();
    let body = seated(&mut app).expect("Mary-O is seated");

    let swings = triggerable_swings(&mut app, body);
    assert!(
        swings.is_empty(),
        "Mary-O's own game answered a combat press with a smash move. \
         She authors the table for the crossover grid; her `abilities: Some([RunJump])` \
         row is what must keep it unreachable here. Triggered: {swings:#?}"
    );
    assert!(
        app.world()
            .get::<ambition_platformer2d::combat::moveset::ActorMoveset>(body)
            .is_some_and(|m| !m.0.moves.is_empty()),
        "the fix must be the ABILITY GATE, not detaching her repertoire — the \
         crossover grid still wants those moves (D146)"
    );

    // ── and she still has the two she is supposed to have ────────────────────
    let walk = top_speed(&mut app, body, false);
    for _ in 0..60 {
        step(&mut app, ControlFrame::default());
    }
    let run = top_speed(&mut app, body, true);
    assert!(
        run > walk * 1.2,
        "the run modifier must still make her run: walk {walk}, run {run}"
    );

    let grounded = |app: &App| {
        app.world()
            .get::<ae::BodyGroundState>(body)
            .is_some_and(|g| g.on_ground)
    };
    for _ in 0..LIVENESS_CAP {
        step(&mut app, ControlFrame::default());
        if grounded(&app) {
            break;
        }
    }
    let mut left_the_floor = false;
    for tick in 0..LIVENESS_CAP {
        let mut frame = ControlFrame {
            jump_held: true,
            ..ControlFrame::default()
        };
        frame.jump_pressed = tick == 0;
        step(&mut app, frame);
        if !grounded(&app) {
            left_the_floor = true;
            break;
        }
    }
    assert!(left_the_floor, "the jump must still leave the ground");
}

/// The run button doubles as the fireball button, and ONLY with the lantern.
///
/// lantern."* The classic grammar — one button, two roles, the sustain still
/// meaning run. What arms it is the WORN cinder beacon, not an ability: the
/// beacon grants a `ranged` verb, so her fists stay empty while her hands are
/// full, which is why fixing the melee gate above could not have paid for this
/// one.
#[test]
fn the_run_button_throws_a_spark_only_while_she_wears_the_lantern() {
    let mut app = boot();
    let body = seated(&mut app).expect("Mary-O is seated");

    /// Hold run and tap the same button; report `(sparks seen, did she run)`.
    fn run_press_sparks(app: &mut App, body: Entity) -> (usize, bool) {
        let mut seen = 0usize;
        let mut ran = false;
        for tick in 0..90 {
            let mut frame = ControlFrame {
                modifier_held: true,
                ..aim(1.0, 0.0)
            };
            frame.modifier_pressed = tick % 30 == 0;
            step(app, frame);
            ran |= app
                .world()
                .get::<MaryOGait>(body)
                .is_some_and(|gait| gait.running);
            let mut q = app.world_mut().query::<&MaryOSpark>();
            seen = seen.max(q.iter(app.world()).count());
        }
        (seen, ran)
    }

    fn wear(
        app: &mut App,
        body: Entity,
        rows: Vec<ambition_platformer2d::characters::equipment::EquipmentRow>,
    ) {
        let mut entity = app.world_mut().entity_mut(body);
        match entity.get_mut::<WornEquipment>() {
            Some(mut worn) => {
                for row in rows {
                    worn.equip(row);
                }
            }
            None => {
                entity.insert(WornEquipment::new(rows));
            }
        }
        for _ in 0..30 {
            step(app, ControlFrame::default());
        }
    }

    assert_eq!(
        run_press_sparks(&mut app, body).0,
        0,
        "small Mary-O has no lantern — run is only run"
    );

    wear(&mut app, body, vec![star_wand()]);
    assert_eq!(
        run_press_sparks(&mut app, body).0,
        0,
        "the wand is armor only; the grown form still throws nothing"
    );

    wear(&mut app, body, vec![cinder_beacon()]);
    let (sparks, ran) = run_press_sparks(&mut app, body);
    assert!(
        sparks > 0,
        "with the cinder beacon worn, the run press must throw a spark"
    );
    assert!(
        ran,
        "...while the SAME button's held level keeps meaning run"
    );
}
