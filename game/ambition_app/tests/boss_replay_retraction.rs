//! BOSS-REPLAY-RETRACTION (Q51, Q56): a replay that makes a boss undefeated
//! again does so for every boss family, and the reward its defeat minted goes
//! with it. The baseline is the last committed checkpoint: a defeat recorded
//! after it is retracted by a replay of its room, and a defeat recorded before
//! it survives. The replay is keyed by its live room, so a replay in one live
//! room keeps a defeat in another.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::boss_encounter::{BossConfig, BossEncounterPhase};
use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
use bevy::prelude::World;

use crate::boss_lifecycle::{boss_alive, boss_cleared, force_kill_boss, kill_boss_with_a_real_hit, spawn_mockingbird};

/// Ask for a replay of the controlled body's room and let it settle.
fn replay(sim: &mut Platformer2dSimHarness) {
    sim.world_mut()
        .write_message(ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual());
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
}

/// Step until the save records placement `placement` cleared.
fn until_cleared(sim: &mut Platformer2dSimHarness, placement: &str) {
    for _ in 0..400 {
        if boss_cleared(sim, placement) {
            return;
        }
        sim.step(AgentAction::default());
    }
    panic!("precondition: the defeat of `{placement}` was never recorded");
}

fn boss_phase(world: &mut World, placement: &str) -> Option<BossEncounterPhase> {
    let mut q = world.query::<(&BossConfig, &ambition_platformer2d::boss_encounter::BossEncounter)>();
    q.iter(world)
        .find(|(config, _)| config.id == placement)
        .and_then(|(_, status)| status.encounter.as_ref().map(|phase| phase.phase))
}

/// (record cleared, boss alive, boss in its Death phase), after a replay.
fn after_a_replay(sim: &mut Platformer2dSimHarness, placement: &str) -> (bool, Option<bool>, bool) {
    replay(sim);
    (
        boss_cleared(sim, placement),
        boss_alive(sim.world_mut(), placement),
        boss_phase(sim.world_mut(), placement) == Some(BossEncounterPhase::Death),
    )
}

/// A `BossSpawn` placement (mockingbird) defeated after the last checkpoint
/// is undefeated by a replay: its record is `Untouched`, and the rebuilt boss
/// is alive and not in its Death phase.
#[test]
fn a_boss_spawn_defeat_after_the_checkpoint_is_retracted_by_a_replay() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    spawn_mockingbird(&mut sim, "after_the_checkpoint");
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }
    force_kill_boss(&mut sim, "after_the_checkpoint");
    until_cleared(&mut sim, "after_the_checkpoint");
    assert_eq!(
        after_a_replay(&mut sim, "after_the_checkpoint"),
        (false, Some(true), false),
        "(cleared, alive, in Death) after a replay: the defeat since the checkpoint was not retracted"
    );
}

/// A conducted boss (the flying spaghetti monster, an extension module's
/// boss) defeated after the last checkpoint is undefeated by a replay, on the
/// same generic road.
#[test]
fn a_conducted_boss_defeat_after_the_checkpoint_is_retracted_by_a_replay() {
    let mut sim = crate::common::fixed_60hz_room_sim("flying_spaghetti_monster_arena");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let god = {
        let world = sim.world_mut();
        let mut q = world.query::<&BossConfig>();
        q.iter(world)
            .find(|config| config.behavior.id == ambition_content::bosses::fsm::FSM_ID)
            .map(|config| config.id.clone())
            .expect("the arena authors the god")
    };
    force_kill_boss(&mut sim, &god);
    until_cleared(&mut sim, &god);
    assert_eq!(
        after_a_replay(&mut sim, &god),
        (false, Some(true), false),
        "(cleared, alive, in Death) after a replay: the conducted boss's defeat was not retracted"
    );
}

/// The control: a defeat recorded before the last checkpoint survives a
/// replay of its own room. The checkpoint promises it.
#[test]
fn a_defeat_before_the_checkpoint_survives_a_replay() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    spawn_mockingbird(&mut sim, "before_the_checkpoint");
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }
    force_kill_boss(&mut sim, "before_the_checkpoint");
    until_cleared(&mut sim, "before_the_checkpoint");
    sim.world_mut()
        .write_message(ambition_platformer2d::platformer::lifecycle::CheckpointCommitted);
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
    let (cleared, alive, _) = after_a_replay(&mut sim, "before_the_checkpoint");
    assert_eq!(
        (cleared, alive),
        (true, Some(false)),
        "(cleared, alive) after a replay: a defeat the checkpoint holds was retracted"
    );
}

/// The mints of a boss: every live entity whose origin names `boss` as its
/// parent, and every ledger row the save's minted rows give that parent.
fn mints_of(world: &mut World, boss: &str) -> (usize, usize) {
    use ambition_platformer2d::platformer::construction::SpawnOrigin;
    let mut q = world.query::<&SpawnOrigin>();
    let live = q
        .iter(world)
        .filter(|origin| matches!(origin, SpawnOrigin::Dynamic { parent, .. } if parent.as_str() == boss))
        .count();
    let rows = world
        .get_resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
        .map_or(0, |ledger| {
            ledger
                .rows()
                .filter(|(sim_id, _)| {
                    world
                        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
                        .data()
                        .minted_items()
                        .iter()
                        .any(|row| row.parent == boss && row.occurrence == sim_id.as_str())
                })
                .count()
        });
    (live, rows)
}

/// The reward goes with the defeat (Q51): the signature gauntlet the
/// retracted defeat dropped, and its ledger row, are gone after the replay. A
/// minted item lying in a room has a `Placed` row, so without the retraction
/// the rebuild puts it back.
#[test]
fn a_replay_takes_back_the_gauntlet_the_retracted_defeat_dropped() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    spawn_mockingbird(&mut sim, "gauntlet_giver");
    let boss = {
        let world = sim.world_mut();
        let mut q = world.query::<(&BossConfig, &ambition_platformer2d::platformer::sim_id::SimId)>();
        q.iter(world)
            .find(|(config, _)| config.id == "gauntlet_giver")
            .map(|(_, sim_id)| sim_id.as_str().to_string())
            .expect("the boss has a simulation identity")
    };
    kill_boss_with_a_real_hit(&mut sim, "gauntlet_giver", 600);
    until_cleared(&mut sim, "gauntlet_giver");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let (live, rows) = mints_of(sim.world_mut(), &boss);
    assert!(
        live > 0 && rows > 0,
        "precondition: the defeat minted a reward with a ledger row (live {live}, rows {rows})"
    );
    replay(&mut sim);
    assert_eq!(
        mints_of(sim.world_mut(), &boss),
        (0, 0),
        "(live mints, ledger rows) of the retracted boss after the replay"
    );
}

/// The replay is keyed by its live room. Bob (slot 1) holds `switch_lab`
/// (#0), where a boss is defeated after the checkpoint; Alice, in the hub
/// (#1), asks for a replay of her own room. Bob's room's defeat stays. The
/// control is the single-room arm above, where the same defeat is retracted.
#[test]
fn a_replay_in_one_live_room_keeps_a_boss_defeat_in_another() {
    const BOSS: &str = "bobs_boss";
    let (mut sim, first) = crate::two_players_two_live_rooms::alice_leaves_bob_for_a_replay();
    spawn_mockingbird(&mut sim, BOSS);
    sim.step(crate::common::base());
    {
        let world = sim.world_mut();
        let boss = world
            .query::<(bevy::prelude::Entity, &BossConfig)>()
            .iter(world)
            .find(|(_, config)| config.id == BOSS)
            .map(|(entity, _)| entity)
            .expect("the boss reached the world");
        world.entity_mut(boss).insert(InRoomInstance(first));
    }
    for _ in 0..15 {
        sim.step(crate::common::base());
    }
    force_kill_boss(&mut sim, BOSS);
    until_cleared(&mut sim, BOSS);
    replay(&mut sim);
    assert!(
        boss_cleared(&sim, BOSS),
        "a replay of Alice's room retracted the defeat in Bob's room"
    );
}
