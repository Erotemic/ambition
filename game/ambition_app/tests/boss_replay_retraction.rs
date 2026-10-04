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

/// Where quest `id` stands: (progression, step) in the registry, then in the
/// save row that mirrors it.
fn quest(world: &mut World, id: &str) -> ((String, u8), (String, u8)) {
    let state = world
        .resource::<ambition_content::quest::QuestRegistry>()
        .get(id)
        .map(|state| (format!("{:?}", state.progression), state.step))
        .expect("the quest is authored");
    let (saved, step) = world
        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data()
        .quest(id);
    (state, (format!("{saved:?}"), step))
}

/// A quest the registry starts at boot is in progress at its first step,
/// and the save has no row for it until it first advances.
fn started_at_boot() -> ((String, u8), (String, u8)) {
    (("InProgress".to_string(), 0), ("NotStarted".to_string(), 0))
}

/// The mockingbird's defeat advances `pirate_treasure` past its first step.
/// Returns where the quest stands (before the fight, after the defeat, after
/// a replay); `checkpoint` commits a checkpoint between the defeat and the
/// replay.
fn pirate_treasure_across_a_replay(checkpoint: bool) -> Vec<((String, u8), (String, u8))> {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let mut stands = vec![quest(sim.world_mut(), "pirate_treasure")];
    spawn_mockingbird(&mut sim, "quest_giver");
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }
    force_kill_boss(&mut sim, "quest_giver");
    until_cleared(&mut sim, "quest_giver");
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
    stands.push(quest(sim.world_mut(), "pirate_treasure"));
    if checkpoint {
        sim.world_mut()
            .write_message(ambition_platformer2d::platformer::lifecycle::CheckpointCommitted);
        for _ in 0..5 {
            sim.step(AgentAction::default());
        }
    }
    replay(&mut sim);
    stands.push(quest(sim.world_mut(), "pirate_treasure"));
    stands
}

/// The quest step goes with the defeat: a replay that retracts the
/// mockingbird's defeat puts `pirate_treasure` back on "hunt the
/// mockingbird", in the registry and in the save.
#[test]
fn a_replay_takes_back_the_quest_step_the_retracted_defeat_advanced() {
    let at = |step: u8| (("InProgress".to_string(), step), ("InProgress".to_string(), step));
    assert_eq!(
        pirate_treasure_across_a_replay(false),
        vec![started_at_boot(), at(1), at(0)],
        "pirate_treasure (registry, save) before the fight, after the defeat, after the replay"
    );
}

/// The control: a defeat the checkpoint holds keeps the step it advanced.
#[test]
fn a_quest_step_from_a_defeat_before_the_checkpoint_survives_a_replay() {
    let at = |step: u8| (("InProgress".to_string(), step), ("InProgress".to_string(), step));
    assert_eq!(
        pirate_treasure_across_a_replay(true),
        vec![started_at_boot(), at(1), at(1)],
        "pirate_treasure (registry, save) before the fight, after the defeat, after the replay"
    );
}

/// Spawn a clockwork warden at the player, defeat it, and pick up the
/// `markrecall` it drops.
fn defeat_a_warden_and_take_its_ability(sim: &mut Platformer2dSimHarness, placement: &str) {
    use ambition_platformer2d::items::Item;
    let (px, py) = {
        let world = sim.world_mut();
        let kin = world
            .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("the player has a body");
        (kin.pos.x, kin.pos.y)
    };
    sim.spawn_boss_at(
        placement,
        "clockwork_warden",
        (px, py),
        (40.0, 40.0),
        ambition_platformer2d::entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "clockwork_warden".to_string(),
        },
    );
    kill_boss_with_a_real_hit(sim, placement, 600);
    until_cleared(sim, placement);
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
        if owns(sim.world_mut(), Item::MarkRecall) > 0 {
            return;
        }
    }
    panic!("precondition: the warden's markrecall was never picked up");
}

/// What a warden defeated in the hub leaves after a death: (boss cleared,
/// markrecall owned, intro_first_system_boss in the registry, the wallet's
/// change since before the fight). `banked` commits the checkpoint after the
/// defeat instead of before it; `elsewhere` walks into the neighbour room
/// before dying.
fn a_warden_defeat_across_a_death(banked: bool, elsewhere: bool) -> (bool, u32, (String, u8), i32) {
    use crate::death_restores_the_checkpoint::{commit_a_checkpoint, die, walk_to, NEIGHBOUR, ROOM};
    let mut sim = crate::common::fixed_60hz_room_sim(ROOM);
    let before = balance(sim.world_mut());
    if !banked {
        commit_a_checkpoint(&mut sim);
    }
    defeat_a_warden_and_take_its_ability(&mut sim, "warden");
    if banked {
        commit_a_checkpoint(&mut sim);
    }
    if elsewhere {
        walk_to(&mut sim, NEIGHBOUR);
    }
    die(&mut sim);
    (
        boss_cleared(&sim, "warden"),
        owns(sim.world_mut(), ambition_platformer2d::items::Item::MarkRecall),
        quest(sim.world_mut(), "intro_first_system_boss").0,
        balance(sim.world_mut()) - before,
    )
}

/// A death goes back to the checkpoint everywhere, so it retracts a boss
/// defeat since the checkpoint wherever the player dies. It took the ability
/// out of the bag before, and left the boss dead and the quest complete, in
/// the other room.
#[test]
fn a_death_in_another_room_retracts_a_defeat_since_the_checkpoint() {
    let undefeated = (false, 0, ("InProgress".to_string(), 0), 0);
    assert_eq!(
        (a_warden_defeat_across_a_death(false, false), a_warden_defeat_across_a_death(false, true)),
        (undefeated.clone(), undefeated),
        "(cleared, markrecall, quest, wallet change) after a death in the boss's room, then in another room"
    );
}

/// The control: a defeat the checkpoint holds survives a death in another
/// room, with its ability, its quest step and its bounty.
#[test]
fn a_defeat_before_the_checkpoint_survives_a_death_in_another_room() {
    assert_eq!(
        a_warden_defeat_across_a_death(true, true),
        (true, 1, ("Completed".to_string(), 0), 50),
        "(cleared, markrecall, quest, wallet change) after a death in another room: \
         the warden's 50-coin bounty stays with it"
    );
}

/// The mockingbird's defeat, then the hand-in to the admiral, which completes
/// `pirate_treasure` and pays out. Returns (pirate_treasure in the registry,
/// health cells gained, reward flag) after the hand-in and after a replay;
/// `banked` commits a checkpoint between the hand-in and the replay.
fn a_handed_in_treasure_across_a_replay(banked: bool) -> Vec<((String, u8), i64, bool)> {
    use ambition_platformer2d::items::Item;
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let cells = |sim: &mut Platformer2dSimHarness| i64::from(owns(sim.world_mut(), Item::HealthCell));
    let before = cells(&mut sim);
    let stands = |sim: &mut Platformer2dSimHarness| {
        let flag = sim
            .world()
            .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
            .data()
            .flag(ambition_content::quest::PIRATE_TREASURE_REWARD_FLAG);
        (quest(sim.world_mut(), "pirate_treasure").0, cells(sim) - before, flag)
    };
    spawn_mockingbird(&mut sim, "hoard_thief");
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }
    force_kill_boss(&mut sim, "hoard_thief");
    until_cleared(&mut sim, "hoard_thief");
    // The admiral's conversation sets this flag and queues it, as
    // `effect_bus::write_flag` does.
    sim.world_mut()
        .resource_mut::<ambition_content::quest::QuestRegistry>()
        .push_event(ambition_platformer2d::persistence::quest::QuestAdvanceEvent::FlagSet(
            "npc_pirate_admiral_talked".into(),
        ));
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
    let mut seen = vec![stands(&mut sim)];
    if banked {
        sim.world_mut()
            .write_message(ambition_platformer2d::platformer::lifecycle::CheckpointCommitted);
        for _ in 0..5 {
            sim.step(AgentAction::default());
        }
    }
    replay(&mut sim);
    seen.push(stands(&mut sim));
    seen
}

/// A quest that moved on after the retracted defeat goes back with it: the
/// hand-in could only follow the hunt, so the replay puts `pirate_treasure`
/// back on its first step, and the admiral's payout goes with it.
#[test]
fn a_replay_takes_back_the_quest_steps_and_payout_that_followed_the_defeat() {
    assert_eq!(
        a_handed_in_treasure_across_a_replay(false),
        vec![(("Completed".to_string(), 1), 3, true), (("InProgress".to_string(), 0), 0, false)],
        "(pirate_treasure, health cells gained, reward flag) after the hand-in, then after the replay"
    );
}

/// The control: a hand-in the checkpoint holds survives a replay, with its
/// payout.
#[test]
fn a_payout_before_the_checkpoint_survives_a_replay() {
    let paid = (("Completed".to_string(), 1), 3, true);
    assert_eq!(
        a_handed_in_treasure_across_a_replay(true),
        vec![paid.clone(), paid],
        "(pirate_treasure, health cells gained, reward flag) after the hand-in, then after the replay"
    );
}

/// Spawn mockingbird `id` where Bob stands, in Bob's live room `room`. A boss
/// spawned at Alice's hub position and moved into Bob's room stands at hub
/// coordinates there, outside its geometry, and its chest falls out of the
/// world.
fn spawn_mockingbird_beside_bob(sim: &mut Platformer2dSimHarness, id: &str, room: ambition_platformer2d::platformer::lifecycle::LiveRoomInstance) {
    let at = {
        let world = sim.world_mut();
        world
            .query::<(&ambition_platformer2d::combat::components::FeatureId, &ambition_platformer2d::engine_core::BodyKinematics)>()
            .iter(world)
            .find(|(feature, _)| feature.0 == "ow1_bob")
            .map(|(_, kin)| (kin.pos.x, kin.pos.y))
            .expect("Bob is in the world")
    };
    sim.spawn_boss_at(
        id,
        "mockingbird",
        at,
        (30.0, 30.0),
        ambition_platformer2d::entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
    sim.step(crate::common::base());
    let world = sim.world_mut();
    let entity = world
        .query::<(bevy::prelude::Entity, &BossConfig)>()
        .iter(world)
        .find(|(_, config)| config.id == id)
        .map(|(entity, _)| entity)
        .expect("the boss reached the world");
    world.entity_mut(entity).insert(InRoomInstance(room));
}

/// What Alice's and Bob's bosses leave standing, and where Bob is.
#[derive(Debug, PartialEq, Eq)]
struct BobsBoss {
    /// The save records Alice's boss defeated.
    alices_cleared: bool,
    /// The save records Bob's boss defeated.
    cleared: bool,
    /// A dead body of Bob's boss is in the world. Counted over every body: the
    /// placement is also authored in the hub, where it stands alive.
    dead_body: bool,
    /// Reward chests in the world.
    chests: usize,
    /// Bob is in the live room he was in when the checkpoint was banked.
    bob_in_his_first_room: bool,
    /// Bob is in a live room.
    bob_in_a_live_room: bool,
    /// Entities stamped with a live room that is no longer live: what a room
    /// retired without its residents would leave.
    orphans: usize,
    /// Live rooms after the ending.
    live_rooms: usize,
}

/// Post-checkpoint boss defeats in Alice's live room and in Bob's, and then,
/// if `die`, Alice's death in hers.
///
/// Alice banks a checkpoint; then a boss in her room and a boss in Bob's room
/// (`switch_lab`) are defeated, and only then does Alice die. Bob stays in his
/// room the whole time. With `together`, they share one room.
///
/// Bob is found by his identity, not by his seat: in this fixture he is a
/// placement of `switch_lab`, so a rebuild of that room builds him again
/// (Q153: a seat's body needs a home a rebuild does not replace).
fn bobs_boss_after_alices_death(die: bool, together: bool) -> BobsBoss {
    bobs_boss_after(if die { Ending::Death } else { Ending::Nothing }, together)
}

/// What ends the run in [`bobs_boss_after`].
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ending {
    Nothing,
    /// Alice dies in her room.
    Death,
    /// An explicit whole-session restart.
    NewGame,
}

/// [`bobs_boss_after_alices_death`], ended by `ending`.
fn bobs_boss_after(ending: Ending, together: bool) -> BobsBoss {
    use crate::death_restores_the_checkpoint::commit_a_checkpoint;
    use ambition_platformer2d::platformer::lifecycle::{LiveRoomInstance, RoomInstanceRoot};
    const BOSS: &str = "bobs_boss";
    const ALICES_BOSS: &str = "alices_boss";
    let (mut sim, first) = if together {
        crate::two_players_two_live_rooms::alice_beside_bob()
    } else {
        crate::two_players_two_live_rooms::alice_leaves_bob_for_a_replay()
    };
    commit_a_checkpoint(&mut sim);
    let alices_room = {
        let world = sim.world_mut();
        world
            .query_filtered::<&InRoomInstance, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("Alice is stamped with her live room")
            .0
    };
    assert_eq!(together, alices_room == first, "precondition: Alice is in Bob's room only when together");
    spawn_mockingbird(&mut sim, ALICES_BOSS);
    sim.step(crate::common::base());
    {
        let world = sim.world_mut();
        let entity = world
            .query::<(bevy::prelude::Entity, &BossConfig)>()
            .iter(world)
            .find(|(_, config)| config.id == ALICES_BOSS)
            .map(|(entity, _)| entity)
            .expect("the boss reached the world");
        world.entity_mut(entity).insert(InRoomInstance(alices_room));
    }
    spawn_mockingbird_beside_bob(&mut sim, BOSS, first);
    for _ in 0..15 {
        sim.step(crate::common::base());
    }
    for boss in [ALICES_BOSS, BOSS] {
        force_kill_boss(&mut sim, boss);
        until_cleared(&mut sim, boss);
    }
    for _ in 0..200 {
        sim.step(crate::common::base());
    }
    match ending {
        Ending::Nothing => {}
        Ending::Death => crate::death_restores_the_checkpoint::die(&mut sim),
        Ending::NewGame => {
            sim.world_mut()
                .write_message(ambition_platformer2d::actors::session::reset::NewGameRequested);
            for _ in 0..240 {
                sim.step(crate::common::base());
            }
        }
    }
    let world = sim.world_mut();
    let chests = world
        .query_filtered::<(), bevy::prelude::With<ambition_platformer2d::combat::components::BossRewardChest>>()
        .iter(world)
        .count();
    let live: Vec<LiveRoomInstance> = world
        .query_filtered::<&LiveRoomInstance, bevy::prelude::With<RoomInstanceRoot>>()
        .iter(world)
        .copied()
        .collect();
    let bobs_room = world
        .query::<(&ambition_platformer2d::combat::components::FeatureId, Option<&InRoomInstance>)>()
        .iter(world)
        .find(|(feature, _)| feature.0 == "ow1_bob")
        .and_then(|(_, room)| room.map(|room| room.0));
    let orphans = world
        .query::<&InRoomInstance>()
        .iter(world)
        .filter(|stamp| !live.contains(&stamp.0))
        .count();
    let dead_body = world
        .query::<(&BossConfig, &ambition_platformer2d::characters::actor::BodyHealth)>()
        .iter(world)
        .any(|(config, health)| config.id == BOSS && !health.alive());
    BobsBoss {
        alices_cleared: boss_cleared(&sim, ALICES_BOSS),
        cleared: boss_cleared(&sim, BOSS),
        dead_body,
        chests,
        bob_in_his_first_room: bobs_room == Some(first),
        bob_in_a_live_room: bobs_room.is_some_and(|room| live.contains(&room)),
        orphans,
        live_rooms: live.len(),
    }
}

/// Q151: an ordinary death is local to the participant and the room. Alice's
/// death takes back the boss defeat in her room and leaves the defeat in Bob's
/// live room: its save row, the dead boss, its chest and Bob's room stay.
///
/// Both defeats come after the checkpoint, so the one run shows that the
/// restore still retracts, and where. The control is the same run without the
/// death.
#[test]
fn a_death_in_one_room_leaves_the_boss_defeat_in_the_other_players_room() {
    assert_eq!(
        bobs_boss_after_alices_death(false, false),
        BobsBoss {
            alices_cleared: true,
            cleared: true,
            dead_body: true,
            chests: 2,
            bob_in_his_first_room: true,
            bob_in_a_live_room: true,
            orphans: 0,
            live_rooms: 2,
        },
        "control: before any death both defeats stand, with a chest each"
    );
    assert_eq!(
        bobs_boss_after_alices_death(true, false),
        BobsBoss {
            alices_cleared: false,
            cleared: true,
            dead_body: true,
            chests: 1,
            bob_in_his_first_room: true,
            bob_in_a_live_room: true,
            orphans: 0,
            live_rooms: 2,
        },
        "after Alice's death her defeat goes back, and Bob's defeat, its chest and his room stay"
    );
}

/// The same defeats when Alice and Bob share the room and the checkpoint: the
/// room is Alice's own, so her restore rebuilds it and takes back both
/// defeats, and the save and the room agree.
#[test]
fn a_death_in_a_shared_room_takes_back_the_defeat_in_the_room_both_stand_in() {
    assert_eq!(
        bobs_boss_after_alices_death(false, true),
        BobsBoss {
            alices_cleared: true,
            cleared: true,
            dead_body: true,
            chests: 2,
            bob_in_his_first_room: true,
            bob_in_a_live_room: true,
            orphans: 0,
            live_rooms: 1,
        },
        "control: before any death both defeats stand in the save and in the shared room"
    );
    assert_eq!(
        bobs_boss_after_alices_death(true, true),
        BobsBoss {
            alices_cleared: false,
            cleared: false,
            dead_body: false,
            chests: 0,
            bob_in_his_first_room: false,
            bob_in_a_live_room: true,
            orphans: 0,
            live_rooms: 1,
        },
        "after Alice's death the save and the shared room must agree"
    );
}

/// (coins Alice gained, Bob's chest opened, looted flag, Bob's boss cleared)
/// when Alice has opened the chest of Bob's boss in his room after the
/// checkpoint, and then again after Alice dies in the hub.
fn alice_loots_bobs_chest_then_dies() -> [(i32, Option<bool>, bool, bool); 2] {
    use crate::death_restores_the_checkpoint::commit_a_checkpoint;
    const BOSS: &str = "bobs_boss";
    let (mut sim, first) = crate::two_players_two_live_rooms::alice_leaves_bob_for_a_replay();
    commit_a_checkpoint(&mut sim);
    spawn_mockingbird_beside_bob(&mut sim, BOSS, first);
    for _ in 0..15 {
        sim.step(crate::common::base());
    }
    force_kill_boss(&mut sim, BOSS);
    until_cleared(&mut sim, BOSS);
    for _ in 0..120 {
        sim.step(crate::common::base());
    }
    {
        let world = sim.world_mut();
        let mut chests = world.query::<(&ambition_platformer2d::combat::components::BossRewardChest, &mut ambition_platformer2d::combat::components::ChestFeature)>();
        let (_, mut chest) = chests
            .iter_mut(world)
            .find(|(chest, _)| chest.encounter_id == BOSS)
            .expect("precondition: the defeat dropped its reward chest");
        chest.chest.reward = Some(ambition_platformer2d::entity_catalog::PickupKind::Currency { amount: 30 });
    }
    let before = balance(sim.world_mut());
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "switch_lab"), "switch_lab");
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
    let looted = |sim: &mut Platformer2dSimHarness| {
        sim.world()
            .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
            .data()
            .flag(&ambition_platformer2d::encounter::encounter_reward_looted_flag(BOSS))
    };
    let seen = |sim: &mut Platformer2dSimHarness| {
        (
            balance(sim.world_mut()) - before,
            reward_chest(sim.world_mut(), BOSS).map(|(opened, _)| opened),
            looted(sim),
            boss_cleared(sim, BOSS),
        )
    };
    let opened = seen(&mut sim);
    // Opening the chest holds Interact, which also crosses a door Alice stands in.
    if sim.observation().active_room != "central_hub_complex" {
        assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "central_hub_complex"), "central_hub_complex");
    }
    crate::death_restores_the_checkpoint::die(&mut sim);
    [opened, seen(&mut sim)]
}

/// Q151: Bob's boss defeat and its reward stay when Alice dies, also when
/// Alice took the reward. She opens the chest of Bob's boss in his room after
/// the checkpoint (30 coins), goes back to the hub and dies: she keeps the
/// coins, the chest stays opened and looted, and the boss stays cleared. So
/// the reward is neither lost nor in the world twice. The control is the
/// first reading, taken when the chest is opened.
#[test]
fn a_death_keeps_the_reward_taken_from_the_other_players_boss() {
    let looted = (30, Some(true), true, true);
    assert_eq!(
        alice_loots_bobs_chest_then_dies(),
        [looted, looted],
        "(coins gained, chest opened, looted flag, boss cleared) when Alice opened the chest, then after her death"
    );
}

/// Q151: an explicit whole-session restart may rewind the whole session, and
/// its boundary must be coherent: a room that stays live must not keep what
/// the restart took back.
///
/// The New Game's commit retires every other live room in the same
/// publication (`retires_beside`): Bob's room goes, root and residents, with
/// the dead boss in it, and no entity keeps the stamp of a retired room.
/// Where a seat's body goes on a restart is the join road's (Q153); here Bob
/// is a placement of his room, so he goes with it.
#[test]
fn a_new_game_leaves_no_live_room_holding_what_it_took_back() {
    let after = bobs_boss_after(Ending::NewGame, false);
    assert_eq!(
        (after.cleared, after.dead_body, after.chests, after.bob_in_his_first_room, after.orphans, after.live_rooms),
        (false, false, 0, false, 0, 1),
        "(Bob's boss cleared, its dead body, chests, Bob still in the room the restart did not touch, \
         entities stamped with a room that is not live, live rooms): {after:?}"
    );
}
