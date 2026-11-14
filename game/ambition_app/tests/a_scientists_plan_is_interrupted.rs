#![cfg(feature = "rl_sim")]
//! RADIANT-NPC-HTN slice 4: interrupt and replan.
//!
//! Nikobolt's half past nine is a working one: to the bench, an experiment.
//! Three things stop that plan, and each time he plans again from the new
//! facts, and his life says why (`Life::because`):
//!
//! - the player talks to him: he stops where he is while they talk, and when
//!   the conversation ends he plans again and does the experiment;
//! - no route gets him to the bench (he stands on a gantry and cannot move):
//!   he plans to read notes instead, and his facts keep that the bench was
//!   out of reach;
//! - the bench is gone (the room no longer names it): he reads notes, and his
//!   facts keep that the bench was absent.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::actors::body::life::{Life, LifeStop, Value};
use ambition_platformer2d::characters::actor::CharacterForm;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::time::{DayClock, DayCycle};
use bevy::prelude::Entity;

const LAB: &str = "eureka_coil_lab";
const NIKOBOLT: &str = "npc_science_nikobolt";
const BENCH: &str = "workbench";
const WORKBENCH_X: f32 = 1072.0;

fn the_lab() -> (ambition_app::Platformer2dSimHarness, Entity) {
    let mut sim = fixed_60hz_room_sim(LAB);
    sim.step(base());
    let world = sim.world_mut();
    let nikobolt = world
        .query::<(Entity, &CharacterForm)>()
        .iter(world)
        .find(|(_, worn)| worn.id() == NIKOBOLT)
        .map(|(entity, _)| entity)
        .expect("the coil lab stages Nikobolt");
    (sim, nikobolt)
}

/// Put the clock at `hour`: the next tick is a new hour, and a new plan.
fn set_the_clock(sim: &mut ambition_app::Platformer2dSimHarness, hour: f32) {
    let world = sim.world_mut();
    let cycle = *world.resource::<DayCycle>();
    *world.resource_mut::<DayClock>() = DayClock::at_hour(&cycle, hour);
}

fn life(sim: &ambition_app::Platformer2dSimHarness, body: Entity) -> Life {
    sim.world().get::<Life>(body).cloned().expect("Nikobolt has his life")
}

fn x_of(sim: &ambition_app::Platformer2dSimHarness, body: Entity) -> f32 {
    sim.world().get::<ae::BodyKinematics>(body).expect("a body").pos.x
}

fn talking_to(sim: &ambition_app::Platformer2dSimHarness, body: Entity) -> bool {
    sim.world()
        .get_resource::<ambition_platformer2d::conversation::ActiveConversation>()
        .is_some_and(|conversation| conversation.talker() == Some(body))
}

/// Step until `done` holds of his life, at most `ticks`.
fn until(
    sim: &mut ambition_app::Platformer2dSimHarness,
    body: Entity,
    ticks: u32,
    done: impl Fn(&Life) -> bool,
) -> Option<Life> {
    for _ in 0..ticks {
        sim.step(base());
        let life = life(sim, body);
        if done(&life) {
            return Some(life);
        }
    }
    None
}

fn experiments(life: &Life) -> i64 {
    match life.facts.get("experiments") {
        Some(Value::Int(count)) => *count,
        _ => 0,
    }
}

/// ⭐ A CONVERSATION STOPS HIS PLAN, AND HE PLANS AGAIN AFTER IT. Talked to on
/// his way to the bench, he stops (his life says it was the conversation) and
/// stands while they talk; when it ends he plans again, goes on to the bench
/// and does the experiment.
#[test]
fn a_conversation_interrupts_his_work_and_he_plans_again_after() {
    let (mut sim, nikobolt) = the_lab();
    set_the_clock(&mut sim, 9.5);
    let walking = until(&mut sim, nikobolt, 120, |life| life.task().is_some_and(|step| step.task == "go_to_bench"))
        .expect("premise: at half past nine he sets off for the bench");
    assert_eq!(walking.because, None, "premise: a plan of the clock");
    sim.step_n(base(), 20);

    // The player steps up to him and talks.
    let at = sim.world().get::<ae::BodyKinematics>(nikobolt).expect("a body").pos;
    sim.teleport_player((at.x - 40.0, at.y));
    sim.step(base());
    sim.step(ambition_app::AgentAction { interact: true, interact_held: true, ..base() });
    let mut opened = false;
    for _ in 0..10 {
        if talking_to(&sim, nikobolt) {
            opened = true;
            break;
        }
        sim.step(base());
    }
    assert!(opened, "premise: Interact beside Nikobolt opens a conversation with him");
    let stopped_at = x_of(&sim, nikobolt);
    sim.step_n(base(), 90);
    let held = life(&sim, nikobolt);
    assert!(talking_to(&sim, nikobolt), "premise: the conversation is still open");
    assert_eq!(held.stopped, Some(LifeStop::Interrupted("conversation".into())), "the conversation did not stop his plan");
    assert!(held.steps.is_empty() && held.claim.is_none(), "an interrupted plan keeps steps or a claim: {held:?}");
    let drift = (x_of(&sim, nikobolt) - stopped_at).abs();
    assert!(drift < 8.0, "he walked {drift} px on while they talked");

    // The conversation ends: he plans again, and works.
    sim.world_mut().resource_mut::<ambition_platformer2d::dialog::DialogState>().close();
    sim.teleport_player((at.x - 400.0, at.y));
    let replanned = until(&mut sim, nikobolt, 60, |life| !life.steps.is_empty())
        .expect("after the conversation he made no new plan");
    assert_eq!(replanned.because, Some(LifeStop::Interrupted("conversation".into())), "the new plan does not say why it was made");
    let done = until(&mut sim, nikobolt, 60 * 30, |life| experiments(life) >= 1)
        .unwrap_or_else(|| panic!("after the conversation no experiment was done: {:?}", life(&sim, nikobolt)));
    let x = x_of(&sim, nikobolt);
    assert!((x - WORKBENCH_X).abs() < 48.0, "the experiment was done away from the bench: x {x}");
    assert_eq!(done.stopped, None);
}

/// ⭐ NO ROUTE TO THE BENCH: HE READS NOTES, AND KNOWS WHY. Stood on the east
/// gantry with no way to move, the bench on the floor is out of reach: the
/// visit is refused, and his next plan is to read notes, made because of the
/// refused route, with `unreachable:workbench` among his facts.
#[test]
fn a_bench_out_of_reach_is_a_plan_to_read_notes() {
    let (mut sim, nikobolt) = the_lab();
    {
        let world = sim.world_mut();
        let mut contributions = world.get_mut::<ae::AbilityContributions>(nikobolt).expect("an authored kit");
        contributions.set(
            "test: Nikobolt cannot move",
            ae::AbilityContribution::Ceiling(ae::AbilitySet { move_horizontal: false, jump: false, ..ae::AbilitySet::ALL }),
        );
        let mut bodies = world.query::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel)>();
        let (mut clusters, mut model) = bodies.get_mut(world, nikobolt).expect("a body");
        let mut clusters = clusters.as_clusters_mut();
        // The east gantry is 720..960 wide, its top at y 288.
        ae::movement::transit_body(&mut model, &mut clusters, ae::Vec2::new(840.0, 240.0), ae::movement::TransitVelocity::Zero);
    }
    sim.step_n(base(), 30);
    assert_eq!(
        sim.world().get::<ae::BodyAbilities>(nikobolt).map(|a| a.abilities.move_horizontal),
        Some(false),
        "premise: the ceiling took his movement"
    );
    set_the_clock(&mut sim, 9.5);
    let reading = until(&mut sim, nikobolt, 240, |life| life.task().is_some_and(|step| step.task == "read_notes"))
        .unwrap_or_else(|| panic!("out of reach of the bench he did not plan to read notes: {:?}", life(&sim, nikobolt)));
    assert_eq!(reading.because, Some(LifeStop::NoRoute(BENCH.into())), "the plan to read notes does not say why");
    assert_eq!(reading.facts.get("unreachable:workbench"), Some(&Value::Bool(true)));
    let y = sim.world().get::<ae::BodyKinematics>(nikobolt).unwrap().pos.y;
    assert!(y < 288.0, "premise: he is still on the gantry (y {y})");
}

/// ⭐ THE BENCH IS GONE: HE READS NOTES, AND KNOWS WHY. The room no longer
/// names the bench; his plan to go to it stops there, and his next plan is to
/// read notes, made because the place is absent.
#[test]
fn a_bench_that_is_gone_is_a_plan_to_read_notes() {
    let (mut sim, nikobolt) = the_lab();
    {
        let world = sim.world_mut();
        let mut sets = world.query::<&mut ambition_platformer2d::world::rooms::RoomSet>();
        let mut removed = 0;
        for mut set in sets.iter_mut(world) {
            for room in set.rooms.iter_mut().filter(|room| room.id == LAB) {
                let before = room.debug_labels.len();
                room.debug_labels.retain(|label| label.name != BENCH);
                removed += before - room.debug_labels.len();
            }
        }
        assert_eq!(removed, 1, "premise: the lab named its bench once, and now does not");
    }
    set_the_clock(&mut sim, 9.5);
    let reading = until(&mut sim, nikobolt, 120, |life| life.task().is_some_and(|step| step.task == "read_notes"))
        .unwrap_or_else(|| panic!("with the bench gone he did not plan to read notes: {:?}", life(&sim, nikobolt)));
    assert_eq!(reading.because, Some(LifeStop::NoPlace(BENCH.into())), "the plan to read notes does not say why");
    assert_eq!(reading.facts.get("absent:workbench"), Some(&Value::Bool(true)));
}
