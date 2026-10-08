//! Q63: an authored chest that a body opened stays opened. Its row in the
//! occurrence ledger is `Spent`, so each later build of its room builds it
//! opened (Q105: through the same `ChestSpec::opened` an author writes) and
//! its reward is not granted again. Before, an opened chest kept no record,
//! so every rebuild of its room built it closed with its reward.
//!
//! `basement_breakables` (`sandbox.ldtk`) authors one chest, `Chest`, with a
//! heart in it.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::combat::components::{CenteredAabb, ChestFeature, FeatureName, Opened};

use crate::breakable_respawn_across_rooms::cross_to;
use crate::common::{base, fixed_60hz_room_sim};

const ROOM: &str = "basement_breakables";
const HUB: &str = "central_hub_complex";
const CHEST: &str = "Chest";

/// Whether the chest is opened, or `None` when its room is not built.
fn chest_opened(sim: &mut Platformer2dSimHarness) -> Option<bool> {
    let world = sim.world_mut();
    let mut q = world.query::<(&ChestFeature, &FeatureName, Option<&Opened>)>();
    q.iter(world)
        .find(|(_, name, _)| name.0.as_str() == CHEST)
        .map(|(_, _, opened)| opened.is_some())
}

/// The chest's row in the occurrence ledger, as text.
fn ledger_row(sim: &mut Platformer2dSimHarness) -> String {
    let world = sim.world_mut();
    let mut q = world.query::<(
        &ChestFeature,
        &FeatureName,
        &ambition_platformer2d::platformer::sim_id::SimId,
    )>();
    let sim_id = q
        .iter(world)
        .find(|(_, name, _)| name.0.as_str() == CHEST)
        .map(|(_, _, sim_id)| sim_id.clone())
        .expect("the room authors the chest");
    let ledger = world.resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>();
    format!("{:?}", ledger.whereabouts(&sim_id))
}

/// Stand on the chest and press Interact once, as a player does; then step
/// back to where the player was.
fn open_the_chest(sim: &mut Platformer2dSimHarness) {
    let at = {
        let world = sim.world_mut();
        let mut q = world.query::<(&ChestFeature, &FeatureName, &CenteredAabb)>();
        q.iter(world)
            .find(|(_, name, _)| name.0.as_str() == CHEST)
            .map(|(_, _, aabb)| (aabb.center.x, aabb.center.y))
            .expect("the room authors the chest")
    };
    let start = sim.observation().player_pos;
    sim.teleport_player(at);
    sim.step(base());
    sim.step(AgentAction { interact: true, ..base() });
    sim.step_n(base(), 2);
    assert_eq!(chest_opened(sim), Some(true), "precondition: the press opened the chest");
    sim.teleport_player((start.0, start.1));
    sim.step_n(base(), 2);
}

/// Open the chest, leave its room so the room retires, and come back: the
/// room is built again with the chest opened. The control is the chest
/// before the press: built closed, with no row.
#[test]
fn an_opened_chest_is_built_opened_when_its_room_is_built_again() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 30);
    assert_eq!(
        (chest_opened(&mut sim), ledger_row(&mut sim).as_str()),
        (Some(false), "None"),
        "control: the chest is authored closed, and nothing is remembered"
    );
    open_the_chest(&mut sim);
    assert_eq!(ledger_row(&mut sim), "Some(Spent)", "the opened chest is remembered");
    assert_eq!(cross_to(&mut sim, HUB), HUB);
    assert_eq!(chest_opened(&mut sim), None, "precondition: the chest's room retired");
    assert_eq!(cross_to(&mut sim, ROOM), ROOM);
    sim.step_n(base(), 2);
    assert_eq!(
        chest_opened(&mut sim),
        Some(true),
        "the rebuilt room built the opened chest closed, with its reward in it again"
    );
}

/// The chest after the player dies, when it was opened after the checkpoint
/// (`true`) or before it.
fn chest_after_a_death(opened_after_the_checkpoint: bool) -> Option<bool> {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 30);
    if opened_after_the_checkpoint {
        crate::death_restores_the_checkpoint::commit_a_checkpoint(&mut sim);
    }
    open_the_chest(&mut sim);
    if !opened_after_the_checkpoint {
        crate::death_restores_the_checkpoint::commit_a_checkpoint(&mut sim);
    }
    crate::death_restores_the_checkpoint::die(&mut sim);
    sim.step_n(base(), 2);
    chest_opened(&mut sim)
}

/// Q151 on the death horizon: the checkpoint is the world a death goes back
/// to. A chest opened after it is closed again, with its reward (which the
/// restore takes back); one opened before it stays opened (the control),
/// because the restored ledger remembers it.
#[test]
fn a_death_closes_again_only_a_chest_opened_after_the_checkpoint() {
    assert_eq!(
        (chest_after_a_death(true), chest_after_a_death(false)),
        (Some(false), Some(true)),
        "(opened after the checkpoint, opened before it): the chest after the death"
    );
}

/// The chest after a load of `file` into its room, as a fresh process does.
fn chest_after_a_load(
    file: &ambition_platformer2d::persistence::save_data::AmbitionGameSaveData,
) -> Option<bool> {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .0 = file.clone();
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::actors::session::durable_horizon::SaveRestored>()
        .0 = false;
    sim.step_n(base(), 90);
    assert!(
        sim.world()
            .resource::<ambition_platformer2d::actors::session::durable_horizon::SaveRestored>()
            .0,
        "precondition: the load landed"
    );
    chest_opened(&mut sim)
}

/// Save, quit, load: a file whose ledger says the chest is spent builds it
/// opened. The control is a file with no row, which builds it closed.
#[test]
fn a_load_builds_opened_a_chest_the_file_remembers_spent() {
    use ambition_platformer2d::persistence::save_data::{
        AmbitionGameSaveData, PersistedOccurrence, PersistedWhereabouts,
    };
    let chest = {
        let mut sim = fixed_60hz_room_sim(ROOM);
        sim.step_n(base(), 2);
        let world = sim.world_mut();
        let mut q = world.query::<(
            &ChestFeature,
            &FeatureName,
            &ambition_platformer2d::platformer::sim_id::SimId,
        )>();
        q.iter(world)
            .find(|(_, name, _)| name.0.as_str() == CHEST)
            .map(|(_, _, sim_id)| sim_id.as_str().to_string())
            .expect("the room authors the chest")
    };
    let mut spent = AmbitionGameSaveData::new();
    spent.set_durable_horizon(
        vec![PersistedOccurrence::new(chest, PersistedWhereabouts::Spent)],
        Vec::new(),
    );
    assert_eq!(
        (chest_after_a_load(&AmbitionGameSaveData::new()), chest_after_a_load(&spent)),
        (Some(false), Some(true)),
        "(a file with no row, a file with the chest spent): the chest after the load"
    );
}
