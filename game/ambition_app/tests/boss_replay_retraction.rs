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

/// The primary player's wallet balance.
fn balance(world: &mut World) -> i32 {
    world
        .query_filtered::<&ambition_platformer2d::characters::actor::BodyWallet, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .expect("the player has a wallet")
        .balance
}

/// The coins go with the defeat (Q51). The player holds 7 coins from before
/// the fight; the boss's bounty, collected, adds to them; a replay that
/// retracts the defeat takes the bounty back and leaves the 7.
#[test]
fn a_replay_takes_back_the_bounty_the_retracted_defeat_paid() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    spawn_mockingbird(&mut sim, "bounty_payer");
    {
        let world = sim.world_mut();
        world
            .query_filtered::<&mut ambition_platformer2d::characters::actor::BodyWallet, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single_mut(world)
            .expect("the player has a wallet")
            .balance = 7;
    }
    sim.rebase_rollback_history().expect("the rollback history rebases over the wallet");
    kill_boss_with_a_real_hit(&mut sim, "bounty_payer", 600);
    until_cleared(&mut sim, "bounty_payer");
    let mut paid = balance(sim.world_mut());
    for _ in 0..240 {
        if paid > 7 {
            break;
        }
        sim.step(AgentAction::default());
        paid = balance(sim.world_mut());
    }
    replay(&mut sim);
    assert_eq!(
        (paid, balance(sim.world_mut())),
        (7 + 50, 7),
        "(balance with the bounty collected, balance after the replay)"
    );
}

/// How many of `item` the bag holds.
fn owns(world: &mut World, item: ambition_platformer2d::items::Item) -> u32 {
    world.resource::<ambition_platformer2d::items::OwnedItems>().count(item)
}

/// The ability goes with the defeat (Q51). The clockwork warden drops
/// `markrecall`, which the player picks up into the bag; a replay that
/// retracts the defeat takes it out again. `blink`, which the player held
/// before the fight, stays.
#[test]
fn a_replay_takes_back_the_ability_the_retracted_defeat_granted() {
    use ambition_platformer2d::items::Item;
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let (px, py) = {
        let world = sim.world_mut();
        let kin = world
            .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("the player has a body");
        (kin.pos.x, kin.pos.y)
    };
    sim.spawn_boss_at(
        "ability_giver",
        "clockwork_warden",
        (px, py),
        (40.0, 40.0),
        ambition_platformer2d::entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "clockwork_warden".to_string(),
        },
    );
    let before = (owns(sim.world_mut(), Item::MarkRecall), owns(sim.world_mut(), Item::Blink));
    kill_boss_with_a_real_hit(&mut sim, "ability_giver", 600);
    until_cleared(&mut sim, "ability_giver");
    let mut granted = 0;
    for _ in 0..240 {
        let at = {
            let world = sim.world_mut();
            world
                .query::<(&ambition_platformer2d::combat::components::PickupFeature, &ambition_platformer2d::combat::components::CenteredAabb)>()
                .iter(world)
                .find(|(pickup, _)| matches!(&pickup.pickup.kind, ambition_platformer2d::entity_catalog::PickupKind::Ability { ability_id } if ability_id == "markrecall"))
                .map(|(_, aabb)| aabb.center)
        };
        if let Some(at) = at {
            sim.teleport_player((at.x, at.y));
        }
        sim.step(AgentAction::default());
        granted = owns(sim.world_mut(), Item::MarkRecall);
        if granted > 0 {
            break;
        }
    }
    replay(&mut sim);
    assert_eq!(
        (before, granted, (owns(sim.world_mut(), Item::MarkRecall), owns(sim.world_mut(), Item::Blink))),
        ((0, 1), 1, (0, 1)),
        "((markrecall, blink) before, markrecall picked up, (markrecall, blink) after the replay)"
    );
}

/// The reward chest of placement `placement`: (opened, center).
fn reward_chest(world: &mut World, placement: &str) -> Option<(bool, ambition_platformer2d::engine_core::Vec2)> {
    use ambition_platformer2d::combat::components::{BossRewardChest, CenteredAabb, Opened};
    world
        .query::<(&BossRewardChest, &CenteredAabb, Option<&Opened>)>()
        .iter(world)
        .find(|(chest, _, _)| chest.encounter_id == placement)
        .map(|(_, aabb, opened)| (opened.is_some(), aabb.center))
}

/// The opened reward chest goes with the defeat (Q51). A content pack's boss
/// chest that grants 30 coins is opened by the player; a replay that retracts
/// the defeat takes the coins back, the chest goes, and the save no longer
/// says it was looted. Killed directly, so the boss drops no bounty.
#[test]
fn a_replay_takes_back_the_reward_chest_the_retracted_defeat_dropped() {
    const BOSS: &str = "chest_giver";
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    spawn_mockingbird(&mut sim, BOSS);
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }
    force_kill_boss(&mut sim, BOSS);
    until_cleared(&mut sim, BOSS);
    for _ in 0..120 {
        sim.step(AgentAction::default());
    }
    // The pack's reward, in place of mockingbird's `Custom` relic.
    {
        let world = sim.world_mut();
        let mut chests = world.query::<(&ambition_platformer2d::combat::components::BossRewardChest, &mut ambition_platformer2d::combat::components::ChestFeature)>();
        let (_, mut chest) = chests
            .iter_mut(world)
            .find(|(chest, _)| chest.encounter_id == BOSS)
            .expect("precondition: the defeat dropped its reward chest");
        chest.chest.reward = Some(ambition_platformer2d::entity_catalog::PickupKind::Currency { amount: 30 });
    }
    let looted = |sim: &mut Platformer2dSimHarness| {
        sim.world()
            .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
            .data()
            .flag(&ambition_platformer2d::encounter::encounter_reward_looted_flag(BOSS))
    };
    let before = balance(sim.world_mut());
    for _ in 0..60 {
        let Some((opened, at)) = reward_chest(sim.world_mut(), BOSS) else {
            break;
        };
        if opened {
            break;
        }
        sim.teleport_player((at.x, at.y));
        sim.step(AgentAction { interact: true, interact_held: true, ..AgentAction::default() });
        sim.step(AgentAction::default());
    }
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
    let opened = (balance(sim.world_mut()) - before, reward_chest(sim.world_mut(), BOSS).map(|(opened, _)| opened), looted(&mut sim));
    replay(&mut sim);
    let after = (balance(sim.world_mut()) - before, reward_chest(sim.world_mut(), BOSS).map(|(opened, _)| opened), looted(&mut sim));
    assert_eq!(
        (opened, after),
        ((30, Some(true), true), (0, None, false)),
        "((coins, chest opened, looted flag) when opened, the same after the replay)"
    );
}

/// Cut-rope's boss, defeated before the checkpoint; then the pending slot is
/// taken by another lifecycle operation, and while it is, the player chooses
/// "try again" when `try_again` is set. Later a plain replay of the room.
/// Returns (cleared after the refused request, cleared after the plain replay).
fn cut_rope_after_a_refused_request(try_again: bool) -> (bool, bool) {
    cut_rope_after_a_request(true, try_again)
}

/// [`cut_rope_after_a_refused_request`], with the pending slot taken only when
/// `slot_taken` is set.
fn cut_rope_after_a_request(slot_taken: bool, try_again: bool) -> (bool, bool) {
    use ambition_platformer2d::actors::session::lifecycle_commit::{
        LifecycleIntent, PendingLifecycleCommit, RoomReconstitutionIntent,
    };
    const ROOM: &str = "you_have_to_cut_the_rope";
    let mut sim = crate::common::fixed_60hz_room_sim(ROOM);
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let boss = {
        let world = sim.world_mut();
        let mut q = world.query::<&BossConfig>();
        q.iter(world)
            .find(|config| ambition_content::bosses::is_cut_rope_boss(&config.behavior.id))
            .map(|config| config.id.clone())
            .expect("the arena authors the cut-rope boss")
    };
    force_kill_boss(&mut sim, &boss);
    until_cleared(&mut sim, &boss);
    sim.world_mut()
        .write_message(ambition_platformer2d::platformer::lifecycle::CheckpointCommitted);
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
    if slot_taken {
        let _ = sim.world_mut().resource_mut::<PendingLifecycleCommit>().record(
            0,
            LifecycleIntent::ReconstituteRoom(RoomReconstitutionIntent { target_room: ROOM.to_string() }),
        );
    }
    if try_again {
        sim.world_mut().write_message(ambition_content::bosses::CutRopeRoomReplayRequested);
    }
    sim.step(AgentAction::default());
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let after_the_request = boss_cleared(&sim, &boss);
    replay(&mut sim);
    (after_the_request, boss_cleared(&sim, &boss))
}

/// A "try again" the lifecycle refuses is not a re-fight. Cut-rope's boss is
/// defeated before the checkpoint, so only the re-fight road can put it back.
/// The player chooses "try again" while another lifecycle operation owns the
/// pending slot, so the replay is refused; later a plain replay of the room is
/// admitted. The plain replay is not a re-fight: the defeat stays, as it does
/// in the control, where nobody chose "try again". When the re-fight was a
/// latch the refused request left set, the plain replay took it.
#[test]
fn a_refused_try_again_does_not_make_a_later_replay_a_refight() {
    assert_eq!(
        (cut_rope_after_a_refused_request(false), cut_rope_after_a_refused_request(true)),
        ((true, true), (true, true)),
        "((cleared after the refused request, cleared after a later plain replay) with no \
         try-again, the same with a refused try-again)"
    );
}

/// The re-fight road itself: an admitted "try again" puts cut-rope's boss back
/// to `Untouched`, though its defeat fell before the checkpoint. A later plain
/// replay leaves it so.
#[test]
fn an_admitted_try_again_re_fights_a_defeat_from_before_the_checkpoint() {
    assert_eq!(
        cut_rope_after_a_request(false, true),
        (false, false),
        "(cleared after the admitted try-again, cleared after a later plain replay)"
    );
}
