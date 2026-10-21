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
    assert!(!plates(&mut sim).iter().any(|label| label == "Inugami"), "its plate shows while he lives: {:?}", plates(&mut sim));

    kill(&mut sim, WOLFENSTEIN_ID);
    step_until(&mut sim, 30, "it to wake when he is cleared", |sim| head(sim).phase != BossEncounterPhase::Dormant);
    let floor = head(&mut sim).view.and_then(|v| v.hall).expect("it measured the room").floor;
    assert!(!plates(&mut sim).iter().any(|label| label == "Inugami"), "its plate shows before its title: {:?}", plates(&mut sim));
    step_until(&mut sim, 60 * 14, "it to rise and fight", |sim| head(sim).view.is_some_and(|v| v.woke));
    let h = head(&mut sim);
    assert!(h.kin.pos.y + h.kin.size.y * 0.5 < floor - 20.0, "it does not float over the floor: {h:?}");

    step_until(&mut sim, 60 * 10, "its first move", |sim| head(sim).view.is_some_and(|v| v.performing.is_some()));
    assert!(plates(&mut sim).iter().any(|label| label == "Inugami"), "it fights with no plate: {:?}", plates(&mut sim));

    // Hurt it past its transition: the chase is over, and it comes back in
    // the hall at the end of its room.
    wound(&mut sim, INUGAMI_ID, 0.45);
    step_until(&mut sim, 60 * 4, "it to come back in the final hall", |sim| {
        let h = head(sim);
        h.phase == BossEncounterPhase::Phase2 && h.kin.pos.x > FINAL_HALL_X
    });

    // You stand in the final hall with it, and at its next window Kagero
    // comes to your aid: she cuts it, and is gone up her line.
    sim.teleport_player((FINAL_HALL_X + 800.0, FINAL_HALL_FLOOR - 40.0));
    let before = health(&mut sim, INUGAMI_ID);
    step_until(&mut sim, 60 * 30, "Kagero to come", |sim| kagero(sim).is_some());
    step_until(&mut sim, 60 * 4, "Kagero to cut it", |sim| health(sim, INUGAMI_ID) < before);
    step_until(&mut sim, 60 * 4, "Kagero to leave", |sim| kagero(sim).is_none());
    assert!(head(&mut sim).view.is_some_and(|v| v.ally_called));
    let cut = before - health(&mut sim, INUGAMI_ID);
    assert!(cut >= 3, "her cuts took {cut} from it");
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

/// Where the throne room's final hall starts: the last 1600 of its 9600
/// (`specs/castle_godwin/make_specs.py`, `FINAL_W`).
const FINAL_HALL_X: f32 = 8000.0;
/// The top of the final hall's floor (`HIGH_FLOOR`).
const FINAL_HALL_FLOOR: f32 = 880.0;

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
