//! The Inugami, played headlessly in the throne room: Wolfenstein falls, his
//! head tears off and rises as a spirit, a boss of its own, and it fights
//! (`ambition_content_modules::inugami`; `docs/planning/game/bosses.md`, "The
//! Inugami").

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_content::bosses::inugami::{conductor_of, InugamiView, INUGAMI_ID};
use ambition_content::bosses::wolfenstein::WOLFENSTEIN_ID;
use ambition_platformer2d::boss_encounter::{BossConfig, BossEncounter, BossEncounterPhase};
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const ARENA: &str = "castle_godwin_throne";

fn arena() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED: the tolerant road falls back to the start room, which has
        // no Wolfenstein, and every assertion below would be vacuous.
        .with_required_start_room(ARENA);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the throne room builds headlessly");
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
    }
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    sim
}

#[derive(Debug)]
struct Head {
    kin: ae::BodyKinematics,
    phase: BossEncounterPhase,
    alive: bool,
    view: Option<InugamiView>,
}

fn boss_entity(sim: &mut Platformer2dSimHarness, id: &str) -> Entity {
    let world = sim.world_mut();
    world
        .query::<(Entity, &BossConfig)>()
        .iter(world)
        .find(|(_, config)| config.behavior.id == id)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("{id} is in the throne room"))
}

fn head(sim: &mut Platformer2dSimHarness) -> Head {
    let entity = boss_entity(sim, INUGAMI_ID);
    let world = sim.world_mut();
    let kin = world.get::<ae::BodyKinematics>(entity).expect("a body").clone();
    let phase = world.get::<BossEncounter>(entity).expect("an encounter").encounter_phase();
    let alive = world.get::<BodyHealth>(entity).expect("health").alive();
    let view = conductor_of(world, entity);
    Head { kin, phase, alive, view }
}

/// Kill the boss `id` as a killing blow does.
fn kill(sim: &mut Platformer2dSimHarness, id: &str) {
    let entity = boss_entity(sim, id);
    let world = sim.world_mut();
    world.get_mut::<BodyHealth>(entity).expect("health").health.current = 0;
    if let Some(phase) = world.get_mut::<BossEncounter>(entity).expect("an encounter").encounter.as_mut() {
        let _ = phase.kill();
    }
}

fn step_until(sim: &mut Platformer2dSimHarness, frames: usize, what: &str, mut until: impl FnMut(&mut Platformer2dSimHarness) -> bool) {
    for _ in 0..frames {
        sim.step(AgentAction::default());
        if until(sim) {
            return;
        }
    }
    panic!("waited {frames} frames for {what}; last: {:?}", head(sim));
}

/// It sleeps while he lives; when he falls it rises from his body, flies, and
/// fights; when it falls his head drops to the floor.
#[test]
fn when_wolfenstein_falls_his_head_rises_as_the_inugami_and_fights() {
    let mut sim = arena();
    // Wake him, then fight him a little: the head sleeps all the while.
    for _ in 0..60 * 3 {
        sim.step(AgentAction::default());
    }
    let h = head(&mut sim);
    assert!(h.alive && h.phase == BossEncounterPhase::Dormant, "it woke while he lives: {h:?}");
    // Its name is not over him while he lives: that would give it away.
    assert!(!plates(&mut sim).iter().any(|label| label == NAME), "its plate shows while he lives: {:?}", plates(&mut sim));

    kill(&mut sim, WOLFENSTEIN_ID);
    step_until(&mut sim, 30, "it to wake when he is cleared", |sim| head(sim).phase != BossEncounterPhase::Dormant);
    let floor = head(&mut sim).view.and_then(|v| v.hall).expect("it measured the room").floor;
    assert!(!plates(&mut sim).iter().any(|label| label == NAME), "its plate shows before its title: {:?}", plates(&mut sim));
    step_until(&mut sim, 60 * 14, "it to rise and fight", |sim| head(sim).view.is_some_and(|v| v.woke));
    let h = head(&mut sim);
    assert!(h.kin.pos.y + h.kin.size.y * 0.5 < floor - 20.0, "it does not float over the floor: {h:?}");

    step_until(&mut sim, 60 * 10, "its first move", |sim| head(sim).view.is_some_and(|v| v.performing.is_some()));
    assert!(plates(&mut sim).iter().any(|label| label == NAME), "it fights with no plate: {:?}", plates(&mut sim));
    // No ward in its first phase: it is hit as any boss (Jon, 2026-10-24).
    assert!(head(&mut sim).view.is_some_and(|v| !v.warded) && !guarded(&mut sim), "it is warded in its first phase");

    // Hurt it past its transition: it hunts you to the final hall. It stays
    // with you while you are not there, and it comes into the hall when you do.
    // It flies there: no tick moves it further than a fast flight does (it
    // was put in the final hall on the tick of its transition, and flew back).
    wound(&mut sim, INUGAMI_ID, 0.45);
    step_until(&mut sim, 60, "its ward to kindle", |sim| head(sim).view.is_some_and(|v| v.warded));
    let mut last = head(&mut sim).kin.pos;
    let mut jump = (0.0_f32, 0usize);
    for k in 0..60 * 4 {
        sim.step(AgentAction::default());
        let now = head(&mut sim).kin.pos;
        if (now - last).length() > jump.0 {
            jump = ((now - last).length(), k);
        }
        last = now;
    }
    assert!(jump.0 < MOST_IN_A_TICK, "it jumped {} px in one tick ({} ticks after its wound)", jump.0, jump.1);
    let h = head(&mut sim);
    assert!(h.phase == BossEncounterPhase::Phase2 && h.kin.pos.x > FINAL_HALL_EAST, "it did not stay on you: {h:?}");
    assert!(guarded(&mut sim), "its ward turns no blow in the chase");
    // You run the route to the final hall (carried along it at a run): out
    // of the arena's east gate, down the drop, and west under the castle a
    // step down at a time. It hunts you there on its own wings, along your
    // scent: no sweep, no teleport, and nowhere it is held (Jon, 2026-10-22:
    // "there is a spot that the head cant actually pass on the way to the
    // final arena").
    let body = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
        q.single(world).expect("the player")
    };
    let route = chase_route();
    let mut last = head(&mut sim).kin.pos;
    let mut jump = (0.0_f32, last, last);
    for leg in route.windows(2) {
        let (a, b) = (leg[0], leg[1]);
        let ticks = ((b - a).length() / 260.0 * 60.0).ceil() as usize;
        for i in 0..ticks {
            {
                let world = sim.world_mut();
                let mut kin = world.get_mut::<ae::BodyKinematics>(body).expect("a body");
                kin.pos = a + (b - a) * (i as f32 / ticks as f32);
                kin.vel = (b - a).normalize() * 260.0;
            }
            sim.step(AgentAction::default());
            let h = head(&mut sim);
            if h.view.is_some_and(|v| v.running) {
                let world = sim.world_mut();
                let view = ambition_platformer2d::sim_view::the_only_view(world);
                let snap = &world.get::<ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot>(view)
                    .and_then(|s| s.frame()).expect("the chase has a camera frame").snapshot;
                let extent = (h.kin.pos - snap.center_world).abs() + h.kin.size * 0.5;
                assert!(extent.x <= snap.visible_view.x * 0.5 + 1.0 && extent.y <= snap.visible_view.y * 0.5 + 1.0,
                    "the live pursuer is outside the camera: head={:?}, camera={snap:?}", h.kin);
            }
            let now = h.kin.pos;
            if (now - last).length() > jump.0 {
                jump = ((now - last).length(), last, now);
            }
            last = now;
        }
    }
    assert!(jump.0 < MOST_IN_A_TICK, "it jumped {} px in one tick on its run, from {} to {}", jump.0, jump.1, jump.2);
    step_until(&mut sim, 60 * 15, "it to come into the final hall after you", |sim| {
        let h = head(sim);
        h.phase == BossEncounterPhase::Phase2 && h.kin.pos.x < FINAL_HALL_EAST && h.view.is_some_and(|v| !v.running)
    });
    assert!(guarded(&mut sim), "its ward is down before Kagero breaks it");

    // You stand in the final hall with it: Kagero comes to your aid, her
    // spell breaks its ward (she does not cut it), and she is gone up her
    // line. Then it fights, and your blows land.
    let before = health(&mut sim, INUGAMI_ID);
    step_until(&mut sim, 60 * 4, "Kagero to come", |sim| kagero(sim).is_some());
    step_until(&mut sim, 60 * 4, "her spell to break its ward", |sim| head(sim).view.is_some_and(|v| !v.warded) && !guarded(sim));
    step_until(&mut sim, 60 * 4, "Kagero to leave", |sim| kagero(sim).is_none());
    assert_eq!(health(&mut sim, INUGAMI_ID), before, "she cut it: the fight is yours");
    step_until(&mut sim, 60 * 10, "it to fight in the hall", |sim| head(sim).view.is_some_and(|v| v.performing.is_some()));
    for _ in 0..60 * 3 {
        sim.step(AgentAction::default());
    }
    assert!(kagero(&mut sim).is_none(), "she came back");

    // The killing blow: the world hangs while it falls, then comes back.
    kill(&mut sim, INUGAMI_ID);
    let full = 1.0 / 60.0;
    let mut slowest = f32::MAX;
    for _ in 0..60 * 4 {
        sim.step(AgentAction::default());
        let dt = sim.world_mut().resource::<ambition_platformer2d::time::WorldTime>().sim_dt();
        if dt > 0.0 {
            slowest = slowest.min(dt / full);
        }
    }
    assert!(slowest < 0.3, "time did not hang at its finish: the slowest pace was {slowest}");
    let back = sim.world_mut().resource::<ambition_platformer2d::time::WorldTime>().sim_dt() / full;
    assert!(back > 0.95, "time did not come back up to speed after its finish: {back}");
    let h = head(&mut sim);
    assert!(!h.alive, "{h:?}");
    assert!(h.kin.pos.y + h.kin.size.y * 0.5 > FINAL_HALL_FLOOR - 30.0, "his head did not drop to the hall's floor: {h:?}");
}

/// Its name, as its plate shows it (Jon, 2026-10-24).
const NAME: &str = "Inugami Wolfenstein";

/// The east end of the throne room's final hall, at the west end of the room
/// (`specs/castle_godwin/make_specs.py`, `FINAL_W`).
const FINAL_HALL_EAST: f32 = 1600.0;
/// The top of the final hall's floor.
const FINAL_HALL_FLOOR: f32 = 2624.0;

/// The way out of the throne room the chase takes (`make_specs.py`,
/// `throne`; Jon, 2026-10-25: "right, down, then left"): east across the
/// arena (its floor at 832) through its east gate and along the way east,
/// down the drop at its end, then west a step down at a time (`RUN_STEPS`
/// of `RUN_DROP`, each 888 long, the first at 1664) into the final hall.
/// Points of a body's middle.
fn chase_route() -> Vec<ae::Vec2> {
    const W: f32 = 10480.0;
    const STEP: f32 = 888.0;
    // The arena and the way east are hopped too, over the gate's sill and
    // the way's barricade.
    let mut route = vec![ae::Vec2::new(7000.0, 708.0), ae::Vec2::new(10150.0, 708.0), ae::Vec2::new(10150.0, 1640.0)];
    // A hop over the floor (the body's middle 124 over it): over the run's
    // barricades and rubble, as a runner jumps them. A body carried into one
    // is pushed out of it, and the camera rightly follows that jump.
    for k in 0..10 {
        let floor = 1664.0 + 96.0 * k as f32 - 124.0;
        let west = W - STEP * (k + 1) as f32;
        route.push(ae::Vec2::new(west - 30.0, floor));
        route.push(ae::Vec2::new(west - 60.0, floor + 96.0));
    }
    route.push(ae::Vec2::new(800.0, FINAL_HALL_FLOOR - 24.0));
    route
}

/// Its conductor guards it (its ward): blows clang off.
fn guarded(sim: &mut Platformer2dSimHarness) -> bool {
    let entity = boss_entity(sim, INUGAMI_ID);
    sim.world_mut().get::<BossEncounter>(entity).expect("an encounter").guarded
}

/// A bounded flight or lunge must not become a teleport.
const MOST_IN_A_TICK: f32 = 56.0;

/// Wound the boss `id` to `fraction` of its health.

fn wound(sim: &mut Platformer2dSimHarness, id: &str, fraction: f32) {
    let entity = boss_entity(sim, id);
    let world = sim.world_mut();
    let mut health = world.get_mut::<BodyHealth>(entity).expect("health");
    health.health.current = (health.max() as f32 * fraction).ceil() as i32;
}

fn health(sim: &mut Platformer2dSimHarness, id: &str) -> i32 {
    let entity = boss_entity(sim, id);
    sim.world_mut().get::<BodyHealth>(entity).expect("health").health.current
}

#[test]
fn a_stopped_runner_takes_ghost_fire_and_lightning_with_a_forward_launch() {
    use ambition_platformer2d::combat::{hitbox::LandedBodyHit, strike::{Hitbox, RayStrike}};
    let mut sim = arena();
    kill(&mut sim, WOLFENSTEIN_ID);
    step_until(&mut sim, 60 * 14, "the spirit to rise", |sim| head(sim).view.is_some_and(|v| v.woke));
    sim.teleport_player((8800.0, 808.0));
    wound(&mut sim, INUGAMI_ID, 0.45);
    step_until(&mut sim, 60 * 8, "the chase to start", |sim| head(sim).view.is_some_and(|v| v.running));
    let pursuer = boss_entity(&mut sim, INUGAMI_ID);
    let body = {
        let world = sim.world_mut();
        let body = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
        world.get_mut::<BodyHealth>(body).unwrap().health.invulnerable.set(Invulnerability::SCRIPTED, false);
        body
    };
    let mut cursor = bevy::ecs::message::MessageCursor::<LandedBodyHit>::default();
    let mut contact = None;
    let mut connected = [false; 2];
    let mut before = sim.world().get::<BodyHealth>(body).unwrap().health.current;
    for tick in 0..60 * 18 {
        // Hold the runner in the open corridor. Each catch must move it forward.
        {
            let world = sim.world_mut();
            let mut kin = world.get_mut::<ae::BodyKinematics>(body).unwrap();
            kin.pos = ae::Vec2::new(8800.0, 808.0);
            kin.vel = ae::Vec2::ZERO;
        }
        sim.step(AgentAction::default());
        let world = sim.world();
        for hit in cursor.read(world.resource::<Messages<LandedBodyHit>>()) {
            if hit.attacker != pursuer || hit.victim != body { continue; }
            let Some(strike) = world.get::<Hitbox>(hit.hitbox) else { continue; };
            let kind = if world.get::<RayStrike>(hit.hitbox).is_some() { 1 }
                else if world.get::<Name>(hit.hitbox).is_some_and(|name| name.as_str() == "Inugami ghost fire") { 0 }
                else { continue; };
            let source = world.get::<ae::BodyKinematics>(pursuer).unwrap().pos;
            let victim = world.get::<ae::BodyKinematics>(body).unwrap().pos;
            let launch = strike.launch_world(source, victim).expect("a catch authors its launch").world();
            assert!(launch.x > 0.0 && launch.y < 0.0, "the catch sends the runner back: {launch:?}");
            contact = Some((tick, kind));
        }
        let after = world.get::<BodyHealth>(body).unwrap().health.current;
        if after < before {
            if let Some((_, kind)) = contact.filter(|(when, _)| tick - when <= 3) {
                let pending = world.get::<ae::BodyFlightState>(body).unwrap().pending_launch;
                let velocity = world.get::<ae::BodyKinematics>(body).unwrap().vel;
                let launch = if pending.length_squared() > 0.0 { pending } else { velocity };
                assert!(launch.x > 0.0 && launch.y < 0.0, "the damage did not launch the runner forward: {launch:?}");
                connected[kind] = true;
            }
        }
        before = after;
        if connected == [true, true] { return; }
    }
    panic!("the stopped runner was not hurt by both attacks: fire={}, lightning={}", connected[0], connected[1]);
}

/// Kagero's body, while she is in the world.
fn kagero(sim: &mut Platformer2dSimHarness) -> Option<Entity> {
    use ambition_platformer2d::characters::actor::WornCharacter;
    let world = sim.world_mut();
    world
        .query::<(Entity, &WornCharacter)>()
        .iter(world)
        .find(|(_, worn)| worn.id() == "npc_ninja_kagero")
        .map(|(entity, _)| entity)
}

/// The labels of the actor plates this frame.
fn plates(sim: &mut Platformer2dSimHarness) -> Vec<String> {
    let index = sim.world_mut().resource::<ambition_platformer2d::sim_view::NameplateIndex>();
    index.iter().map(|(_, fact)| fact.label.clone()).collect()
}

/// When it dies its head falls, and lies still where it lands (it shook,
/// held on the floor's line against the floor).
#[test]
fn its_dead_head_lies_still() {
    let mut sim = arena();
    for _ in 0..60 * 3 {
        sim.step(AgentAction::default());
    }
    kill(&mut sim, WOLFENSTEIN_ID);
    step_until(&mut sim, 60 * 14, "it to rise and fight", |sim| head(sim).view.is_some_and(|v| v.woke));
    for _ in 0..60 * 2 {
        sim.step(AgentAction::default());
    }
    kill(&mut sim, INUGAMI_ID);
    for _ in 0..60 * 4 {
        sim.step(AgentAction::default());
    }
    let mut last = head(&mut sim).kin.pos;
    for k in 0..60 {
        sim.step(AgentAction::default());
        let now = head(&mut sim).kin.pos;
        assert!((now - last).length() < 0.5, "the dead head moved {:?} at tick {k}", now - last);
        last = now;
    }
}

/// The robot's shot hurts Wolfenstein, through the Inugami asleep in his
/// body, and a full charge carries. Two faults, 2026-10-21
/// (Jon: "the robot's projectile doesn't seem to do damage anymore"): the
/// sleeping head stopped each shot and swallowed it, and a charged shot born
/// at the knee skidded through its bounces and died 120 px out.
#[test]
fn the_robots_shot_reaches_wolfenstein_past_the_sleeping_inugami() {
    let mut sim = arena();
    sim.teleport_player((6420.0, 792.0));
    for _ in 0..60 * 11 {
        sim.step(AgentAction::default());
    }
    let wolf = boss_entity(&mut sim, WOLFENSTEIN_ID);
    assert_eq!(sim.world_mut().get::<BossEncounter>(wolf).unwrap().encounter_phase(), BossEncounterPhase::Phase1);
    let hp = |sim: &mut Platformer2dSimHarness| sim.world_mut().get::<BodyHealth>(wolf).unwrap().health.current;
    // From 160 px, facing him, for each shot (near enough that his own moves
    // do not take him out of its way).
    let aim = |sim: &mut Platformer2dSimHarness, held: bool| {
        let at = sim.world_mut().get::<ae::BodyKinematics>(wolf).unwrap().pos;
        // Keep the shot inside the arena and retain its charge while aiming.
        let hall = head(sim).view.and_then(|view| view.hall).expect("the arena was measured");
        let side = if at.x - 160.0 < hall.left + 32.0 { -1.0 } else { 1.0 };
        sim.teleport_player((at.x - side * 160.0, 792.0));
        sim.step(AgentAction { move_x: side, projectile_held: held, ..AgentAction::default() });
    };
    aim(&mut sim, false);
    let before = hp(&mut sim);
    sim.step(AgentAction { projectile: true, projectile_held: true, ..AgentAction::default() });
    sim.step(AgentAction { projectile_released: true, ..AgentAction::default() });
    for _ in 0..90 {
        sim.step(AgentAction::default());
    }
    assert!(hp(&mut sim) < before, "a tap shot from 160 px did not hurt him");
    aim(&mut sim, false);
    sim.step(AgentAction { projectile: true, projectile_held: true, ..AgentAction::default() });
    for _ in 0..60 * 2 {
        sim.step(AgentAction { projectile_held: true, ..AgentAction::default() });
    }
    aim(&mut sim, true);
    let before = hp(&mut sim);
    sim.step(AgentAction { projectile_released: true, ..AgentAction::default() });
    for _ in 0..90 {
        sim.step(AgentAction::default());
    }
    assert!(before - hp(&mut sim) >= 8, "a full charge from 160 px took {} from him", before - hp(&mut sim));
}

/// The camera on the chase's route, from the throne room down the drop and
/// west under the castle into the final hall: the
/// player is carried along it at a run's pace, and the camera's centre is read
/// each tick (Jon, 2026-10-22: "the cameras have weird issues with going into
/// an out of arenas").
#[test]
fn the_camera_follows_the_chase_route_without_a_whip() {
    let mut sim = arena();
    // Both fights over, so their arena walls are down: a body carried into a
    // wall is held by it and then let through, and that is a jump of the body.
    kill(&mut sim, WOLFENSTEIN_ID);
    step_until(&mut sim, 30, "it to wake", |sim| head(sim).phase != BossEncounterPhase::Dormant);
    kill(&mut sim, INUGAMI_ID);
    step_until(&mut sim, 60 * 10, "the arena to open", |sim| head(sim).phase == BossEncounterPhase::Death || !head(sim).alive);
    for _ in 0..60 * 6 {
        sim.step(AgentAction::default());
    }
    // Over the gallery's steps and rubble: a body carried into a step is
    // pushed out of it, and the camera rightly follows that jump.
    let route = chase_route();
    const PACE: f32 = 300.0;
    let body = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
        q.single(world).expect("the player")
    };
    let mut rows = Vec::new();
    for leg in route.windows(2) {
        let (a, b) = (leg[0], leg[1]);
        let ticks = ((b - a).length() / PACE * 60.0).ceil() as usize;
        let vel = (b - a).normalize() * PACE;
        for k in 0..ticks {
            let at = a + (b - a) * (k as f32 / ticks as f32);
            {
                let world = sim.world_mut();
                let mut kin = world.get_mut::<ae::BodyKinematics>(body).expect("a body");
                kin.pos = at;
                kin.vel = vel;
            }
            sim.step(AgentAction::default());
            let world = sim.world_mut();
            let view = ambition_platformer2d::sim_view::the_only_view(world);
            let snap = world
                .entity(view)
                .get::<ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot>()
                .and_then(|s| s.frame())
                .map(|f| f.snapshot.clone());
            if let Some(snap) = snap {
                rows.push((at, snap.center_world));
            }
        }
    }
    assert!(rows.len() > 1000, "the camera was framed on {} ticks of the route", rows.len());
    // The camera does not jerk: the step it takes in a tick changes by little
    // from one tick to the next, also where a zone hands it on.
    let mut jerk = (0.0_f32, 0usize);
    for (i, three) in rows.windows(3).enumerate() {
        let change = ((three[2].1 - three[1].1) - (three[1].1 - three[0].1)).length();
        if change > jerk.0 {
            jerk = (change, i + 2);
        }
    }
    // Measured 3.99 on the old route, where the throne room's clamp stops
    // the camera at its edge; 8.0 on this one, where the carried body turns
    // from west to straight down at the run's last step, as the final hall's
    // zone takes the camera from the run's (2026-10-24). Before zones were
    // mixed across their edges the camera went from still to 38 units a tick
    // on leaving the arena.
    assert!(
        jerk.0 < 10.0,
        "the camera's step changed by {:.1} units in one tick at {:?}",
        jerk.0,
        rows[jerk.1].0
    );
}
