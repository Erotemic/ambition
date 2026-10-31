//! The Smash reaction poses (`docs/planning/demos/smash-expressive-fighter-poses.md`,
//! Phase 2), drawn in the shipped composition.
//!
//! Two Pointed Polygons are seated: seat 0 is a human on the first pad, seat 1
//! a CPU. Each event is made deterministically, and the test reads both what the
//! body ASKED to be drawn as (`ActorAnimIndex`) and the row its animator DREW.
//! The sequence each event drew is printed, so a run is a record of it.
//!
//! The events: launched up, out and down, landing face up and face down, and
//! the getup; a tech; a hit high and low; a run and its stop; a teeter on the
//! stage's edge, held and left. The launches are real launches through the
//! launch gateway. The tech and the recoils write the state the kernel and the
//! hit resolver write, because a CPU does not tech or take a low hit on cue.

use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
use ambition_platformer2d::characters::actor::BodyCombat;
use ambition_platformer2d::characters::control::{ControlHold, ControlHolds, DrivingParticipant};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use ambition_platformer2d::render::rendering::FeatureVisual;
use ambition_platformer2d::sim_view::ActorAnimIndex;
use ambition_platformer2d::sprite_sheet::character::CharacterAnimator;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

const FIGHTER: &str = "pointed_polygon";

/// The human seat's body and the CPU seat's body, now. A body that is knocked
/// out comes back as another entity, so this is asked again, not kept.
fn seats(app: &mut App) -> Option<(Entity, Entity)> {
    let world = app.world_mut();
    let mut seats =
        world.query_filtered::<(Entity, Has<DrivingParticipant>), (With<MatchSeat>, With<BodyKinematics>)>();
    let found: Vec<(Entity, bool)> = seats.iter(world).collect();
    let human = found.iter().find(|(_, driven)| *driven).map(|(body, _)| *body);
    let cpu = found.iter().find(|(_, driven)| !*driven).map(|(body, _)| *body);
    human.zip(cpu)
}

/// The human seat and the CPU seat, in a live round.
fn seated() -> (App, Entity, Entity, Entity) {
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
        |_| {},
    );
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)));
    let pad = app.world_mut().spawn(Gamepad::default()).id();
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([FIGHTER, FIGHTER]));
    app.world_mut().write_message(ShellCommand::GoTo(ShellRouteId::new(
        ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
    )));
    for _ in 0..900 {
        app.update();
        if seats(&mut app).is_some() {
            break;
        }
    }
    // Past the opening count: nobody acts while it runs.
    for _ in 0..600 {
        app.update();
        let Some((human, cpu)) = seats(&mut app) else {
            continue;
        };
        let opening = |body| {
            app.world()
                .get::<ControlHolds>(body)
                .is_some_and(|holds| holds.holds(ControlHold::Opening))
        };
        if !opening(human) && !opening(cpu) {
            break;
        }
    }
    for _ in 0..30 {
        app.update();
    }
    let (human, cpu) = seats(&mut app).expect("a human seat and a CPU seat");
    (app, pad, human, cpu)
}

/// What `body` asked to be drawn as, and what its animator drew: the row, or
/// the semantic pose when it drew no row.
fn asked_and_drew(app: &mut App, body: Entity) -> (Option<String>, String) {
    let id = app.world().get::<FeatureId>(body).expect("a seated body has an id").0.clone();
    let asked = app
        .world()
        .resource::<ActorAnimIndex>()
        .get(&id)
        .and_then(|frame| frame.clip.as_ref().map(|clip| clip.clip.clone()));
    let world = app.world_mut();
    let mut visuals = world.query::<(&FeatureVisual, &CharacterAnimator)>();
    let drew = visuals
        .iter(world)
        .find(|(visual, _)| visual.id == id)
        .map(|(_, animator)| {
            animator
                .drawn_clip()
                .map_or_else(|| format!("{:?}", animator.current), str::to_string)
        })
        .expect("a seated body is drawn");
    (asked, drew)
}

/// Step `ticks` and return the rows `body` drew, each once, in the order drawn.
fn drawn_over(app: &mut App, body: Entity, ticks: u32, label: &str) -> Vec<String> {
    let mut drawn: Vec<String> = Vec::new();
    for _ in 0..ticks {
        app.update();
        // A body knocked out comes back as another entity: its record ends.
        if app.world().get::<FeatureId>(body).is_none() {
            drawn.push("(knocked out)".to_string());
            break;
        }
        let (asked, drew) = asked_and_drew(app, body);
        let facts = app.world().get::<ae::BodyMotionFacts>(body).copied().unwrap_or_default();
        let floor = format!(
            "{}{}{}",
            if facts.tumbling { "T" } else { "" },
            if facts.knocked_down { "K" } else { "" },
            facts.getup.map_or(String::new(), |kind| format!("{kind:?}"))
        );

        if let Some(asked) = &asked {
            // A request the sheet answers with another row is a fallback, and
            // the twelve reaction rows are all on this sheet.
            if is_reaction_row(asked) {
                assert_eq!(&drew, asked, "{label}: asked for {asked} and drew {drew}");
            }
        }
        // The floor game's facts beside the row (T tumbling, K knocked
        // down, then the getup kind), so the record says why.
        let vel = app.world().get::<BodyKinematics>(body).map_or(ae::Vec2::ZERO, |k| k.vel);
        let shown = if floor.is_empty() { drew } else { format!("{drew}[{floor}]") };
        if drawn.last().map(|last| bare(last)) != Some(bare(&shown)) {
            eprintln!("[poses]   {label}: {shown} at vel ({:.0}, {:.0})", vel.x, vel.y);
        }
        if drawn.last() != Some(&shown) {
            drawn.push(shown);
        }
    }
    eprintln!("[poses] {label}: {}", drawn.join(" -> "));
    drawn
}

fn is_reaction_row(row: &str) -> bool {
    matches!(
        row,
        "launch_up"
            | "launch_horizontal"
            | "launch_down"
            | "hit_high"
            | "hit_mid"
            | "hit_low"
            | "hit_air"
            | "prone_face_up"
            | "prone_face_down"
            | "tech"
            | "run_stop"
    )
}

/// A recorded row without its floor-game note.
fn bare(row: &str) -> &str {
    row.split('[').next().unwrap_or(row)
}

/// Rows in `drawn` in this order, others between them allowed.
fn in_order(drawn: &[String], rows: &[&str]) -> bool {
    let mut at = 0;
    for row in drawn {
        if at < rows.len() && bare(row) == rows[at] {
            at += 1;
        }
    }
    at == rows.len()
}

/// Hold `body` still, or let it go, with the hold a scripted beat claims.
fn hold(app: &mut App, body: Entity, held: bool) {
    let mut entity = app.world_mut().entity_mut(body);
    match (entity.get_mut::<ControlHolds>(), held) {
        (Some(mut holds), true) => holds.claim(ControlHold::Sequence),
        (None, true) => {
            entity.insert(ControlHolds::only(ControlHold::Sequence));
        }
        (Some(mut holds), false) => {
            holds.release(ControlHold::Sequence);
            if holds.is_empty() {
                entity.remove::<ControlHolds>();
            }
        }
        (None, false) => {}
    }
}

/// Throw `body` toward the stage's centre at `toward` px/s and `up` px/s,
/// from `height` px above where it stands, facing the centre or away from it,
/// through the launch gateway a hit uses.
fn launch(app: &mut App, body: Entity, centre_x: f32, faces_centre: bool, toward: f32, up: f32, height: f32) {
    let mut kinematics = app.world_mut().get_mut::<BodyKinematics>(body).unwrap();
    let to_centre = (centre_x - kinematics.pos.x).signum();
    kinematics.facing = if faces_centre { to_centre } else { -to_centre };
    kinematics.pos.y -= height;
    kinematics.vel = ae::Vec2::ZERO;
    app.world_mut()
        .get_mut::<ae::BodyFlightState>(body)
        .unwrap()
        .stage_launch(ae::Vec2::new(toward * to_centre, -up), false);
}

/// The launch, the landing and the getup, for one body. Every throw is toward
/// the stage's centre, so the body lands on the stage and not at a ledge.
/// `acts`: the body is a CPU, which may act out of its tumble and tech.
fn floor_game(app: &mut App, centre_x: f32, who: &str, acts: bool) {
    // Over the stage's tumble speed (250) and well inside its blast zones.
    for (label, faces_centre, toward, up, height, expected) in [
        ("thrown up", true, 0.0, 700.0, 0.0, ["launch_up", "prone_face_up", "getup"]),
        ("thrown back", false, 450.0, 300.0, 0.0, ["launch_horizontal", "prone_face_up", "getup"]),
        ("thrown forward", true, 450.0, 300.0, 0.0, ["launch_horizontal", "prone_face_down", "getup"]),
        ("spiked", true, 150.0, -700.0, 200.0, ["launch_down", "prone_face_down", "getup"]),
    ] {
        let body = seats(app).map(|(human, cpu)| if acts { cpu } else { human }).expect("both seats");
        launch(app, body, centre_x, faces_centre, toward, up, height);
        let drawn = drawn_over(app, body, 300, &format!("{who} {label}"));
        if acts {
            // A CPU chooses its landing: it may act out of the tumble and tech.
            // The launch is the same row a human draws; the landing is the
            // prone row a human draws, or the tech the CPU made.
            let landed = [expected[1], "tech", "roll", "tech_roll", "getup_roll"];
            let launched_at = drawn.iter().position(|row| bare(row) == expected[0]);
            assert!(
                launched_at.is_some_and(|at| drawn[at..].iter().any(|row| landed.contains(&bare(row)))),
                "{who} {label}: drew {drawn:?}, wanted {} then one of {landed:?}",
                expected[0]
            );
        } else {
            assert!(in_order(&drawn, &expected), "{who} {label}: drew {drawn:?}, wanted {expected:?} in order");
        }
        for _ in 0..60 {
            app.update();
        }
    }
}

#[test]
fn every_reaction_pose_is_drawn_by_human_and_cpu_seats() {
    let (mut app, pad, human, cpu) = seated();
    let home = app.world().get::<BodyKinematics>(human).unwrap().pos;
    let cpu_home = app.world().get::<BodyKinematics>(cpu).unwrap().pos;
    let reset = |app: &mut App, body: Entity, at: ae::Vec2| {
        if let Some(mut kinematics) = app.world_mut().get_mut::<BodyKinematics>(body) {
            kinematics.pos = at;
            kinematics.vel = ae::Vec2::ZERO;
        }
    };

    // The CPU is held while the human's events run, so it does not interfere.
    hold(&mut app, cpu, true);
    let centre_x = (home.x + cpu_home.x) * 0.5;
    floor_game(&mut app, centre_x, "human", false);

    // The tech: the state the kernel writes when a press catches the floor.
    reset(&mut app, human, home);
    for _ in 0..30 {
        app.update();
    }
    if let Some(ae::MotionModel::AxisSwept(axis)) = app.world_mut().get_mut::<ae::MotionModel>(human).as_deref_mut() {
        axis.state.getup_invuln_timer = ae::movement::knockdown::GETUP_INVULN;
        axis.state.getup_kind = ae::movement::knockdown::GetupKind::Tech;
    }
    let drawn = drawn_over(&mut app, human, 12, "human tech");
    assert!(drawn.iter().any(|row| bare(row) == "tech"), "the tech drew {drawn:?}");

    // A hit high and a hit low: what the hit resolver writes on an accepted hit.
    for (label, fraction, row) in [("hit high", 0.4, "hit_high"), ("hit low", -0.4, "hit_low")] {
        for _ in 0..30 {
            app.update();
        }
        let height = app.world().get::<BodyKinematics>(human).unwrap().size.y;
        let mut combat = app.world_mut().get_mut::<BodyCombat>(human).unwrap();
        combat.hit_flash = 0.2;
        combat.struck_height = fraction * height;
        let drawn = drawn_over(&mut app, human, 6, &format!("human {label}"));
        assert!(drawn.iter().any(|r| bare(r) == row), "{label} drew {drawn:?}");
    }

    // A run, and its stop: a real stick on the human's pad.
    reset(&mut app, human, home);
    for _ in 0..30 {
        app.update();
    }
    let toward_cpu = if cpu_home.x > home.x { GamepadButton::DPadRight } else { GamepadButton::DPadLeft };
    app.world_mut().write_message(bevy::input::gamepad::RawGamepadEvent::Button(
        bevy::input::gamepad::RawGamepadButtonChangedEvent::new(pad, toward_cpu, 1.0),
    ));
    let running = drawn_over(&mut app, human, 40, "human run");
    assert!(running.iter().any(|row| matches!(bare(row), "Run" | "Sprint")), "the stick did not run: {running:?}");
    app.world_mut().write_message(bevy::input::gamepad::RawGamepadEvent::Button(
        bevy::input::gamepad::RawGamepadButtonChangedEvent::new(pad, toward_cpu, 0.0),
    ));
    let stopping = drawn_over(&mut app, human, 30, "human stop");
    assert!(stopping.iter().any(|row| bare(row) == "run_stop"), "a run's stop drew {stopping:?}");

    let (human, cpu) = seats(&mut app).expect("both seats");
    hold(&mut app, cpu, true);
    // A teeter: stand the human ever nearer the stage's far edge until the
    // kernel says it teeters, then watch it hold and leave.
    reset(&mut app, cpu, cpu_home + ae::Vec2::new(0.0, -400.0));
    let away = if cpu_home.x > home.x { -1.0 } else { 1.0 };
    let mut teetering = false;
    for step in 0..400 {
        reset(&mut app, human, home + ae::Vec2::new(away * step as f32 * 3.0, 0.0));
        // A body teeters on the brink it FACES.
        if let Some(mut kinematics) = app.world_mut().get_mut::<BodyKinematics>(human) {
            kinematics.facing = away;
        }
        for _ in 0..3 {
            app.update();
        }
        let facts = app.world().get::<ae::BodyMotionFacts>(human).copied().unwrap_or_default();
        if facts.teetering {
            teetering = true;
            break;
        }
    }
    assert!(teetering, "no stand on the stage's edge teeters");
    let held = drawn_over(&mut app, human, 20, "human teeter");
    assert!(held.iter().any(|row| matches!(bare(row), "teeter" | "Teeter")), "the brink drew {held:?}");
    let back = if away > 0.0 { GamepadButton::DPadLeft } else { GamepadButton::DPadRight };
    app.world_mut().write_message(bevy::input::gamepad::RawGamepadEvent::Button(
        bevy::input::gamepad::RawGamepadButtonChangedEvent::new(pad, back, 1.0),
    ));
    let left = drawn_over(&mut app, human, 20, "human leaves the edge");
    assert!(
        left.last().is_some_and(|row| !matches!(bare(row), "teeter" | "Teeter")),
        "walking off the brink kept the teeter: {left:?}"
    );
    app.world_mut().write_message(bevy::input::gamepad::RawGamepadEvent::Button(
        bevy::input::gamepad::RawGamepadButtonChangedEvent::new(pad, back, 0.0),
    ));

    // The CPU: the same launches draw the same rows. The human is held now.
    let (human, cpu) = seats(&mut app).expect("both seats");
    reset(&mut app, human, home);
    reset(&mut app, cpu, cpu_home);
    hold(&mut app, human, true);
    hold(&mut app, cpu, false);
    for _ in 0..30 {
        app.update();
    }
    floor_game(&mut app, centre_x, "cpu", true);
    // On the ground for the low hit: in the air every hit is the air recoil.
    let (_, cpu) = seats(&mut app).expect("both seats");
    hold(&mut app, cpu, true);
    for _ in 0..120 {
        app.update();
        if app.world().get::<ae::BodyGroundState>(cpu).is_some_and(|ground| ground.on_ground) {
            break;
        }
    }
    let height = app.world().get::<BodyKinematics>(cpu).unwrap().size.y;
    let mut combat = app.world_mut().get_mut::<BodyCombat>(cpu).unwrap();
    combat.hit_flash = 0.2;
    combat.struck_height = -0.4 * height;
    let drawn = drawn_over(&mut app, cpu, 6, "cpu hit low");
    assert!(drawn.iter().any(|r| bare(r) == "hit_low"), "the CPU's low hit drew {drawn:?}");
}

/// INSTRUMENT: how a fighter's drawn motion snaps on a display faster than
/// the sim: the shipped presented pose, and from the same tick history the
/// pose extrapolated a tick ahead and interpolated a tick behind. Two CPU Pointed Polygons fight for 20 s at
/// 144 Hz; per frame, the change of each body's drawn step is the jerk.
#[test]
#[ignore = "instrument: prints the measurement"]
fn drawn_motion_jerk_extrapolated_and_interpolated() {
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
        |_| {},
    );
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 144.0)));
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_levels([FIGHTER, FIGHTER], &[5, 5]));
    app.world_mut().write_message(ShellCommand::GoTo(ShellRouteId::new(
        ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
    )));
    for _ in 0..1500 {
        app.update();
    }
    let mut last: std::collections::HashMap<Entity, [(ae::Vec2, ae::Vec2); 3]> = Default::default();
    let mut jerks: [Vec<f32>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for _ in 0..(144 * 20) {
        app.update();
        let phase = app.world().resource::<ambition_platformer2d::sim_view::PresentationPhase>().get();
        let world = app.world_mut();
        let mut bodies = world.query_filtered::<(Entity, &ambition_platformer2d::sim_view::PresentedPose), With<MatchSeat>>();
        let now: Vec<(Entity, [ae::Vec2; 3])> = bodies
            .iter(world)
            .map(|(body, pose)| {
                let extrapolated = pose.authoritative() + pose.tick_delta() * phase;
                let interpolated = pose.authoritative() - pose.tick_delta() * (1.0 - phase);
                (body, [pose.presented(), extrapolated, interpolated])
            })
            .collect();
        for (body, drawn) in now {
            let previous = last.get(&body).copied();
            let mut next = drawn.map(|at| (at, ae::Vec2::ZERO));
            for way in 0..3 {
                if let Some(before) = previous {
                    let step = drawn[way] - before[way].0;
                    // A respawn is a jump, not motion.
                    if step.length() < 200.0 {
                        jerks[way].push((step - before[way].1).length());
                    }
                    next[way] = (drawn[way], step);
                }
            }
            last.insert(body, next);
        }
    }
    for (way, name) in [(0, "shipped"), (1, "extrapolated"), (2, "interpolated")] {
        let mut sorted = jerks[way].clone();
        sorted.sort_by(f32::total_cmp);
        let at = |q: f32| sorted[((sorted.len() - 1) as f32 * q) as usize];
        let spikes = sorted.iter().filter(|j| **j > 4.0).count();
        eprintln!(
            "[jerk] {name}: frames {} p50 {:.2} p95 {:.2} p99 {:.2} max {:.1} px; frames over 4 px: {spikes}",
            sorted.len(),
            at(0.5),
            at(0.95),
            at(0.99),
            at(1.0)
        );
    }
}

/// Every SFX message of a frame, by its variant name, for the grab-cascade
/// instrument below.
#[derive(Resource, Default)]
struct HeardThisFrame(Vec<String>);

fn hear(
    mut messages: bevy::ecs::message::MessageReader<ambition_platformer2d::sfx::OwnedSfxMessage>,
    mut heard: ResMut<HeardThisFrame>,
) {
    for message in messages.read() {
        let name = format!("{:?}", message.request);
        heard.0.push(name.split([' ', '{', '(']).next().unwrap_or("?").to_string());
    }
}

/// INSTRUMENT: the sounds a grab makes. Two CPU fighters fight for 60 s; each
/// frame's SFX are tallied apart for frames in which somebody is held and
/// frames in which nobody is. A grab that sounds a landing every tick shows as
/// `Land` per held frame near one.
#[test]
#[ignore = "instrument: prints the measurement"]
fn the_sounds_of_a_grab() {
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
        |_| {},
    );
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)));
    app.init_resource::<HeardThisFrame>();
    app.add_systems(Last, hear);
    for _ in 0..30 {
        app.update();
    }
    let fighter = std::env::var("GRAB_FIGHTER").unwrap_or_else(|_| FIGHTER.to_string());
    app.world_mut().insert_resource(ambition_demo_smash::smash_roster_at_levels(
        [fighter.as_str(), fighter.as_str()],
        &[9, 9],
    ));
    app.world_mut().write_message(ShellCommand::GoTo(ShellRouteId::new(
        ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
    )));
    for _ in 0..700 {
        app.update();
    }
    let mut held_frames = 0usize;
    let mut free_frames = 0usize;
    let mut held: std::collections::BTreeMap<String, usize> = Default::default();
    let mut free: std::collections::BTreeMap<String, usize> = Default::default();
    let mut worst_hold = 0usize;
    let mut run = 0usize;
    for _ in 0..(60 * 60) {
        app.world_mut().resource_mut::<HeardThisFrame>().0.clear();
        app.update();
        let world = app.world_mut();
        let mut captives =
            world.query_filtered::<(), (With<MatchSeat>, With<ambition_platformer2d::combat::capture::CapturedBy>)>();
        let holding = captives.iter(world).count() > 0;
        let heard = std::mem::take(&mut app.world_mut().resource_mut::<HeardThisFrame>().0);
        let lands = heard.iter().filter(|name| name.as_str() == "Land").count();
        if holding {
            held_frames += 1;
            run += lands;
        } else {
            worst_hold = worst_hold.max(run);
            run = 0;
            free_frames += 1;
        }
        for name in heard {
            *(if holding { &mut held } else { &mut free }).entry(name).or_default() += 1;
        }
    }
    worst_hold = worst_hold.max(run);
    eprintln!("[grab] {fighter}: held frames {held_frames}, free frames {free_frames}, most landings in one hold {worst_hold}");
    eprintln!("[grab] held: {held:?}");
    eprintln!("[grab] free: {free:?}");
}
