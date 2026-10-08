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

use ambition_demo_mary_o::movement::WALK_THROTTLE;
use ambition_demo_mary_o::powerups::{cinder_beacon, star_wand, SPARK_VISUAL};
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
///
/// The prompt names both roles from the same data: her rules say `Run`, and the
/// beacon's technique grant says `Run / Spark` on the same slot.
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
            // She runs when the full throttle, not the walk's, reaches her body.
            ran |= app
                .world()
                .get::<ambition_platformer2d::characters::control::ActorControl>(body)
                .is_some_and(|control| control.0.locomotion.x.abs() > WALK_THROTTLE);
            let mut q = app
                .world_mut()
                .query::<&ambition_platformer2d::projectiles::ProjectileVisualId>();
            seen = seen.max(q.iter(app.world()).filter(|visual| visual.0 == SPARK_VISUAL).count());
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

    fn run_label(app: &App) -> Option<String> {
        app.world()
            .resource::<ambition_platformer2d::sim_view::ControlPrompt>()
            .label_for(ambition_platformer2d::entity_catalog::action_scheme::ControlSlot::Modifier)
            .map(str::to_owned)
    }

    assert_eq!(
        run_press_sparks(&mut app, body).0,
        0,
        "small Mary-O has no lantern — run is only run"
    );
    assert_eq!(run_label(&app).as_deref(), Some("Run"), "her rules name the run");

    wear(&mut app, body, vec![star_wand()]);
    assert_eq!(
        run_press_sparks(&mut app, body).0,
        0,
        "the wand is armor only; the grown form still throws nothing"
    );
    assert_eq!(run_label(&app).as_deref(), Some("Run"), "the wand grants no technique");

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
    assert_eq!(
        run_label(&app).as_deref(),
        Some("Run / Spark"),
        "the beacon's grant names both roles of the one button"
    );
}

/// Her sparks leave at the cadence the spark authors, and two fly at once.
///
/// Measured in her shipped host, where a renderer draws every shot. The
/// press is held down on every tick, so only the weapon's gates decide when a
/// spark leaves: its `refire_s`, and its `max_live`.
///
/// ⚠ On this floor a spark spends its two bounces before a third could leave,
/// so the limit does not refuse anything here: an engine that ignored it still
/// passes this test (measured). The refusal itself is guarded by
/// `a_weapon_at_its_live_shot_limit_refuses_the_firing_move` in
/// `ambition_combat`; this test shows her authored cadence lets two fly. Time is
/// the sum of each sim tick's `sim_dt`, recorded inside the sim schedule,
/// because a frame can run more than one tick and its dt is not fixed.
///
/// ⛔ TWO CLOCKS GATED ONE SHOT. Mary-O kept her own cooldown, and the
/// body's `RangedRefire` applied the engine's default of 1.1 s, because the
/// spark did not author `refire_s`. The slower clock won, and a press refused
/// by one clock still spent the other. And the live-spark count read the
/// visual id, which the renderer also puts on each shot's sprite, so each
/// shot counted twice and she could not have two out.
#[test]
fn her_sparks_leave_at_their_authored_cadence_and_two_fly_at_once() {
    use ambition_platformer2d::characters::equipment::EquipmentGrant;
    use ambition_platformer2d::platformer::schedule::SimScheduleExt;
    use ambition_platformer2d::projectiles::{ProjectileOwner, ProjectileVisualId};

    /// Sim seconds so far, and each new spark with the second it appeared.
    #[derive(Resource, Default)]
    struct Launches {
        elapsed: f32,
        sparks: Vec<(Entity, Entity, f32)>,
    }
    fn record(
        time: Res<ambition_platformer2d::time::WorldTime>,
        mut launches: ResMut<Launches>,
        new_shots: Query<(Entity, &ProjectileOwner, &ProjectileVisualId), Added<ProjectileOwner>>,
    ) {
        launches.elapsed += time.sim_dt();
        let at = launches.elapsed;

        for (shot, owner, visual) in &new_shots {
            if visual.0 == SPARK_VISUAL {
                launches.sparks.push((shot, owner.0, at));
            }
        }
    }

    let mut app = boot();
    let body = seated(&mut app).expect("Mary-O is seated");
    app.init_resource::<Launches>();
    let sim = app.sim_schedule();
    app.add_systems(sim, record);
    let beacon = cinder_beacon();
    let spark = beacon
        .grants
        .iter()
        .find_map(|grant| match grant {
            EquipmentGrant::Ranged(ranged) => Some(ranged.clone()),
            _ => None,
        })
        .expect("the beacon grants the spark");
    let refire_s = spark.refire_s;
    let max_live = usize::from(spark.max_live.expect("the spark authors how many may fly"));
    app.world_mut()
        .entity_mut(body)
        .insert(WornEquipment::new(vec![beacon]));
    for _ in 0..30 {
        step(&mut app, ControlFrame::default());
    }

    let mut most_in_flight = 0usize;
    for _ in 0..LIVENESS_CAP {
        step(
            &mut app,
            ControlFrame {
                modifier_held: true,
                modifier_pressed: true,
                ..aim(0.0, 0.0)
            },
        );
        let mut shots = app
            .world_mut()
            .query::<(&ProjectileOwner, &ProjectileVisualId)>();
        let in_flight = shots
            .iter(app.world())
            .filter(|(owner, visual)| owner.0 == body && visual.0 == SPARK_VISUAL)
            .count();
        most_in_flight = most_in_flight.max(in_flight);
        if app.world().resource::<Launches>().sparks.len() >= 2 {
            break;
        }
    }
    let launches = app.world().resource::<Launches>();
    let hers: Vec<f32> = launches
        .sparks
        .iter()
        .filter(|(_, owner, _)| *owner == body)
        .map(|(_, _, at)| *at)
        .collect();
    assert!(
        hers.len() >= 2,
        "she threw {} spark(s) with the button pressed on every tick",
        hers.len()
    );
    let gap_s = hers[1] - hers[0];
    assert!(
        (gap_s - refire_s).abs() <= 0.02,
        "the second spark left {gap_s:.3} s after the first; the spark authors \
         {refire_s} s between shots"
    );
    assert_eq!(
        most_in_flight, max_live,
        "her sparks in flight at once, counted by owner"
    );
}

/// A body that restarts while its weapon recharges comes back ready to fire.
///
/// Combat owns `RangedRefire`, so combat answers `BodyRestarted`. The restart
/// is raised through the body's own latch, which is the road every reset
/// takes, so this also shows the shipped composition installs the answer.
#[test]
fn a_restart_gives_her_weapon_back_ready() {
    use ambition_platformer2d::combat::components::RangedRefire;

    let mut app = boot();
    let body = seated(&mut app).expect("Mary-O is seated");
    app.world_mut()
        .entity_mut(body)
        .insert(WornEquipment::new(vec![cinder_beacon()]));
    for _ in 0..30 {
        step(&mut app, ControlFrame::default());
    }
    let recharging = |app: &App| {
        app.world()
            .get::<RangedRefire>(body)
            .is_some_and(|refire| !refire.ready())
    };
    for _ in 0..LIVENESS_CAP {
        step(
            &mut app,
            ControlFrame {
                modifier_held: true,
                modifier_pressed: true,
                ..aim(0.0, 0.0)
            },
        );
        if recharging(&app) {
            break;
        }
    }
    assert!(recharging(&app), "the premise: a spark left and her weapon recharges");

    app.world_mut()
        .get_mut::<ambition_platformer2d::engine_core::BodyRestartLatch>(body)
        .expect("her body carries the restart latch")
        .pending = true;
    step(&mut app, ControlFrame::default());
    assert!(
        !recharging(&app),
        "she restarted with her weapon still recharging"
    );
}

/// Her spark does not push her back (Jon, 2026-10-08).
///
/// A ranged action that states no discharge takes the generic one, and the
/// generic one kicks the shooter back along the shot.
#[test]
fn her_spark_does_not_push_her_back() {
    let mut app = boot();
    let body = seated(&mut app).expect("Mary-O is seated");
    app.world_mut()
        .entity_mut(body)
        .insert(WornEquipment::new(vec![cinder_beacon()]));
    // Let her take the fire form and come to rest.
    for _ in 0..90 {
        step(&mut app, ControlFrame::default());
    }
    let at = |app: &App| *app.world().get::<ae::BodyKinematics>(body).expect("her body");
    let rest = at(&app);
    assert!(rest.vel.x.abs() < 0.01, "premise: she stands still ({:?})", rest.vel);

    let mut sparks = 0;
    let mut furthest = 0.0f32;
    for tick in 0..60 {
        step(
            &mut app,
            ControlFrame {
                modifier_pressed: tick == 0,
                modifier_held: tick < 4,
                ..ControlFrame::default()
            },
        );
        let mut live = app
            .world_mut()
            .query::<&ambition_platformer2d::projectiles::ProjectileVisualId>();
        sparks = sparks.max(live.iter(app.world()).filter(|visual| visual.0 == SPARK_VISUAL).count());
        furthest = furthest.max((at(&app).pos.x - rest.pos.x).abs());
    }
    assert!(sparks > 0, "premise: she threw no spark, so nothing could push her");
    assert!(
        furthest < 0.01,
        "she stood still, threw a spark, and moved {furthest:.2} px: the shot pushed her"
    );
}
