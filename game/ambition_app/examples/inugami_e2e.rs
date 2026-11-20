//! The Wolfenstein fight, played end to end by the `wolfenstein` autoplay,
//! headless: a developer's check of its sequencing, run by hand and by no
//! test suite (Jon, 2026-11-19: "a development test ... that just is used to
//! autoplaytest through the wolfenstein fight to make sure the sequencing
//! happens right and the head doesn't get stuck").
//!
//! The route flies with the robot's fireball, never dies and advances each
//! boss a fifth of its health a blow (`autoplay.ron`, its `assist`). This
//! watches the fight go by, prints each step of it as it comes, and fails
//! (exit 1) with the last state when a step does not come in its time or the
//! head stalls in its chase:
//!
//! 1. Wolfenstein wakes, is fought, and falls.
//! 2. His head tears off and rises as the Inugami, and it fights.
//! 3. Its transition: its ward kindles, it crashes the arena's wall, and it
//!    hunts the robot out of the arena, down the drop and west to the final
//!    hall, on its heels the whole way.
//! 4. It comes into the final hall and howls; Kagero comes, her spell breaks
//!    its ward and her talismans stay on it; she goes.
//! 5. It fights in the hall to the last of its strength; it bursts into its
//!    fire, Kagero's great seal strikes, and the next blow ends it in
//!    hellfire, and the cage of the lever up the old drain opens. The route
//!    completes.
//!
//! ```sh
//! cargo run -p ambition_app --example inugami_e2e
//! cargo run -p ambition_app --example inugami_e2e -- --every 30   # a state row every 30 ticks
//! ```

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_content::bosses::inugami::{Conductor, INUGAMI_ID};
use ambition_content::bosses::wolfenstein::WOLFENSTEIN_ID;
use ambition_platformer2d::boss_encounter::{BossConfig, BossEncounter, BossEncounterPhase};
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const ROOM: &str = "castle_godwin_throne";
/// The cage of the lever in the final hall (`make_specs.py`, `throne`).
const CAGE: &str = "castle_godwin_throne_lever_cage";
/// The whole run may take this long (ticks at 60 Hz).
const BUDGET: u64 = 60 * 60 * 8;

#[derive(Clone, Debug, Default)]
struct Boss {
    entity: Option<Entity>,
    pos: ae::Vec2,
    size: ae::Vec2,
    phase: Option<BossEncounterPhase>,
    hp: i32,
    max: i32,
    alive: bool,
    guarded: bool,
}

fn boss(world: &mut World, id: &str) -> Boss {
    let found = world
        .query::<(Entity, &BossConfig)>()
        .iter(world)
        .find(|(_, config)| config.behavior.id == id)
        .map(|(entity, _)| entity);
    let Some(entity) = found else { return Boss::default() };
    let kin = world.get::<ae::BodyKinematics>(entity);
    let health = world.get::<BodyHealth>(entity);
    let encounter = world.get::<BossEncounter>(entity);
    Boss {
        entity: Some(entity),
        pos: kin.map_or(ae::Vec2::ZERO, |k| k.pos),
        size: kin.map_or(ae::Vec2::ZERO, |k| k.size()),
        phase: encounter.map(|e| e.encounter_phase()),
        hp: health.map_or(0, |h| h.health.current),
        max: health.map_or(0, |h| h.max()),
        alive: health.is_some_and(|h| h.alive()),
        guarded: encounter.is_some_and(|e| e.guarded),
    }
}

fn conductor(world: &World, head: Option<Entity>) -> Option<Conductor> {
    ambition_content::bosses::inugami::conductor_record(world, head?)
}

fn kagero(world: &mut World) -> Option<ae::Vec2> {
    use ambition_platformer2d::characters::actor::CharacterForm;
    world
        .query::<(&CharacterForm, &ae::BodyKinematics)>()
        .iter(world)
        .find(|(form, _)| form.id() == "npc_ninja_kagero")
        .map(|(_, kin)| kin.pos)
}

fn player(world: &mut World) -> (ae::Vec2, ae::Vec2) {
    let mut q = world.query_filtered::<&ae::BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).map_or((ae::Vec2::ZERO, ae::Vec2::ZERO), |k| (k.pos, k.vel))
}

/// One step of the fight, and how it is known.
struct Step {
    name: &'static str,
    /// Ticks it may take after the step before it.
    within: u64,
    done: fn(&State) -> bool,
}

#[derive(Default)]
struct State {
    wolf: Boss,
    head: Boss,
    c: Option<Conductor>,
    kagero: Option<ae::Vec2>,
    player: ae::Vec2,
    completed: bool,
    pilot: String,
    /// The lever's cage in the final hall has opened (`boss.cleared` of the
    /// Inugami's encounter): the way back up the old drain.
    cage_open: bool,
}

impl State {
    fn c(&self) -> Option<&Conductor> {
        self.c.as_ref()
    }
}

const STEPS: &[Step] = &[
    Step { name: "Wolfenstein wakes", within: 60 * 30, done: |s| s.wolf.phase.is_some_and(|p| p != BossEncounterPhase::Dormant) },
    Step { name: "Wolfenstein is hurt", within: 60 * 60, done: |s| s.wolf.hp < s.wolf.max },
    Step { name: "Wolfenstein falls", within: 60 * 180, done: |s| s.wolf.entity.is_some() && !s.wolf.alive },
    Step { name: "the head wakes", within: 60 * 10, done: |s| s.head.phase.is_some_and(|p| p != BossEncounterPhase::Dormant) },
    Step { name: "the Inugami rises and fights", within: 60 * 20, done: |s| s.c().is_some_and(|c| c.woke()) },
    Step { name: "the Inugami is hurt", within: 60 * 60, done: |s| s.head.hp < s.head.max },
    Step { name: "its ward kindles", within: 60 * 120, done: |s| s.c().is_some_and(|c| c.warded()) },
    Step { name: "it crashes the arena's wall", within: 60 * 10, done: |s| s.c().is_some_and(|c| c.crashed) },
    Step { name: "it hunts you", within: 60 * 10, done: |s| s.c().is_some_and(|c| c.running()) },
    Step { name: "it comes into the final hall", within: 60 * 120, done: |s| s.c().is_some_and(|c| c.arrived) },
    Step { name: "Kagero comes", within: 60 * 30, done: |s| s.kagero.is_some() },
    Step { name: "her spell breaks its ward", within: 60 * 15, done: |s| s.c().is_some_and(|c| !c.warded()) && !s.head.guarded },
    Step { name: "Kagero goes", within: 60 * 15, done: |s| s.kagero.is_none() },
    Step { name: "it bursts into its finish", within: 60 * 120, done: |s| s.c().is_some_and(|c| c.finish.is_some()) },
    Step { name: "her great seal leaves it open", within: 60 * 20, done: |s| s.c().is_some_and(|c| c.exposed) },
    Step { name: "the killing blow", within: 60 * 60, done: |s| s.head.entity.is_some() && !s.head.alive },
    Step { name: "it burns away in hellfire", within: 60 * 10, done: |s| s.c().is_some_and(|c| c.dead_for.is_some_and(|t| t > 2.0)) },
    Step { name: "the lever's cage opens", within: 60 * 10, done: |s| s.cage_open },
    Step { name: "the route completes", within: 60 * 30, done: |s| s.completed },
];

fn row(tick: u64, s: &State) -> String {
    let c = s.c();
    format!(
        "t={:>6.2}s player=({:>7.1},{:>7.1}) wolf[{:?} hp {}/{}] head[{:?} hp {}/{} at ({:>7.1},{:>7.1}) size ({:>5.0},{:>5.0}) guard={}] \
         pilot={} c[woke={} warded={} run={:?} crashed={} arrived={} appear={:?} ally={} binding={:?} finish={:?} exposed={} scale={:.2} pace={:.0}] kagero={:?}",
        tick as f32 / 60.0,
        s.player.x, s.player.y,
        s.wolf.phase, s.wolf.hp, s.wolf.max,
        s.head.phase, s.head.hp, s.head.max, s.head.pos.x, s.head.pos.y, s.head.size.x, s.head.size.y, s.head.guarded,
        s.pilot,
        c.is_some_and(|c| c.woke()), c.is_some_and(|c| c.warded()), c.and_then(|c| c.run), c.is_some_and(|c| c.crashed),
        c.is_some_and(|c| c.arrived), c.and_then(|c| c.appear), c.is_some_and(|c| c.ally_called()), c.and_then(|c| c.binding),
        c.and_then(|c| c.finish), c.is_some_and(|c| c.exposed), c.map_or(0.0, |c| c.scale), c.map_or(0.0, |c| c.run_pace),
        s.kagero,
    )
}

fn main() {
    let mut every: u64 = 120;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--every" => every = args.next().and_then(|n| n.parse().ok()).expect("--every TICKS"),
            other => {
                eprintln!("inugami_e2e: unknown argument {other}");
                std::process::exit(2);
            }
        }
    }
    let options = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(ROOM)
        // The shipped local session: a sync test that never re-simulates.
        .with_sync_test_rollback_settings(0, 12);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the throne room builds headlessly");
    ambition_platformer2d::scripted_input::install_autoplay(sim.app_mut(), "wolfenstein");

    let mut step = 0usize;
    let mut since = 0u64;
    // The head's chase: where it was each tick, to see it stall.
    let mut trail: std::collections::VecDeque<(ae::Vec2, ae::Vec2)> = Default::default();
    let mut worst_stall = 0.0_f32;
    for tick in 0..BUDGET {
        sim.app_mut().update();
        let world = sim.world_mut();
        let mut s = State {
            wolf: boss(world, WOLFENSTEIN_ID),
            head: boss(world, INUGAMI_ID),
            kagero: kagero(world),
            player: player(world).0,
            completed: world.get_resource::<ambition_content::autoplay::Itinerary>().is_some_and(|it| it.completed),
            cage_open: world
                .get_resource::<ambition_platformer2d::world::gated_lock_walls::GatedLockWallVerdicts>()
                .and_then(|v| v.by_wall.get(CAGE))
                .is_some_and(|outcome| outcome.is_satisfied()),
            pilot: world.get_resource::<ambition_platformer2d::autopilot::Autopilot>()
                .map_or("-".into(), |pilot| format!("{:?}", pilot.status())),
            ..State::default()
        };
        s.c = conductor(world, s.head.entity);
        if tick % every == 0 {
            println!("{}", row(tick, &s));
        }
        // A chase that stalls: over two seconds of its run the head has come
        // no nearer and moved little, while the runner is out of its reach.
        // Its lunge is a dart at your back and its own motion: not a stall.
        if s.c().is_some_and(|c| c.running() && c.lunge.is_none()) {
            trail.push_back((s.head.pos, s.player));
            if trail.len() > 120 {
                trail.pop_front();
            }
            if trail.len() == 120 {
                let (h0, p0) = trail[0];
                let moved = (s.head.pos - h0).length();
                let gap0 = (p0 - h0).length() - s.head.size.x * 0.5;
                let gap = (s.player - s.head.pos).length() - s.head.size.x * 0.5;
                if gap > 400.0 && gap >= gap0 - 20.0 && moved < 120.0 {
                    println!("{}", row(tick, &s));
                    println!("FAIL: the head stalled in its chase: moved {moved:.0} px in 2 s, its face {gap:.0} px from the runner (was {gap0:.0})");
                    std::process::exit(1);
                }
                worst_stall = worst_stall.max(gap - gap0);
            }
        } else {
            trail.clear();
        }
        since += 1;
        while step < STEPS.len() && (STEPS[step].done)(&s) {
            println!("[{:>6.2}s] STEP {:>2}: {}", tick as f32 / 60.0, step + 1, STEPS[step].name);
            step += 1;
            since = 0;
        }
        if step == STEPS.len() {
            println!("PASS: every step of the fight came, in {:.1} s (the chase's gap grew at most {worst_stall:.0} px over 2 s)", tick as f32 / 60.0);
            return;
        }
        if since > STEPS[step].within {
            println!("{}", row(tick, &s));
            println!("FAIL: `{}` did not come within {:.0} s of the step before it", STEPS[step].name, STEPS[step].within as f32 / 60.0);
            std::process::exit(1);
        }
    }
    println!("FAIL: the run took longer than its budget, at step `{}`", STEPS.get(step).map_or("-", |s| s.name));
    std::process::exit(1);
}
