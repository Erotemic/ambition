//! Q152: a collected pickup authored to regrow comes back on the world clock
//! (`GameplayElapsed`), also while its room is not live. It is the second
//! customer of the schedule the breakable respawn uses (`WorldTimeSchedule`).
//!
//! `basement_breakables` authors "regrowing heart" (`AfterSeconds`, 4 s) beside
//! a plain heart that does not regrow. Harvested, then left: a player who comes
//! back before the four seconds are up finds it still gone, for the time that
//! remains; one who comes back later finds it regrown. Before this, a collected
//! pickup had no record at all, so it was back on every rebuild of its room.

#![cfg(feature = "rl_sim")]

use ambition_app::Platformer2dSimHarness;
use ambition_platformer2d::actors::features::GameplayElapsed;
use ambition_platformer2d::combat::components::{CenteredAabb, Collected, FeatureName, RespawnTimer};
use ambition_platformer2d::engine_core::Vec2;

use crate::breakable_respawn_across_rooms::cross_to;
use crate::common::{base, fixed_60hz_room_sim};

const ROOM: &str = "basement_breakables";
const HUB: &str = "central_hub_complex";
const HEART: &str = "regrowing heart";
/// The heart beside it, authored `Never`: gone for the run once taken (Q154).
const PLAIN: &str = "one-time heart";
const REGROW_S: f32 = 4.0;

fn now(sim: &Platformer2dSimHarness) -> f32 {
    sim.world().resource::<GameplayElapsed>().0
}

/// (collected, the time its regrowth still needs) of the pickup `name`, or
/// `None` when its room is not built.
fn pickup(sim: &mut Platformer2dSimHarness, name: &str) -> Option<(bool, Option<f32>)> {
    let world = sim.world_mut();
    let mut q = world.query::<(&FeatureName, Option<&Collected>, Option<&RespawnTimer>)>();
    q.iter(world)
        .find(|(feature, _, _)| feature.0.as_str() == name)
        .map(|(_, collected, timer)| (collected.is_some(), timer.map(|timer| timer.0)))
}

fn records(sim: &Platformer2dSimHarness) -> usize {
    sim.world()
        .resource::<ambition_platformer2d::actors::features::ecs::world_time_schedule::WorldTimeSchedule>()
        .records()
        .count()
}

fn place_the_player(sim: &mut Platformer2dSimHarness, at: Vec2) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<
        &mut ambition_platformer2d::engine_core::BodyKinematics,
        bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
    >();
    let mut kin = q.single_mut(world).expect("the player's body is in the world");
    kin.pos = at;
    kin.vel = Vec2::ZERO;
}

fn player_at(sim: &mut Platformer2dSimHarness) -> Vec2 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<
        &ambition_platformer2d::engine_core::BodyKinematics,
        bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
    >();
    q.single(world).expect("the player's body is in the world").pos
}

/// Collect the pickup `name` by standing on it, as a player does. Returns the
/// clock at the collection.
fn collect(sim: &mut Platformer2dSimHarness, name: &str) -> f32 {
    let at = {
        let world = sim.world_mut();
        let mut q = world.query::<(&FeatureName, &CenteredAabb)>();
        q.iter(world)
            .find(|(feature, _)| feature.0.as_str() == name)
            .map(|(_, aabb)| aabb.center)
            .unwrap_or_else(|| panic!("the room authors the pickup '{name}'"))
    };
    for _ in 0..30 {
        place_the_player(sim, at);
        sim.step(base());
        if pickup(sim, name).is_some_and(|(collected, _)| collected) {
            return now(sim);
        }
    }
    panic!("stood on '{name}' for 30 frames and it was not collected");
}

fn settle(sim: &mut Platformer2dSimHarness, frames: usize) {
    for _ in 0..frames {
        sim.step(base());
    }
}

/// The room, settled, with the heart collected and the player back where she
/// started, off the heart.
fn harvested_room() -> (Platformer2dSimHarness, f32) {
    let mut sim = fixed_60hz_room_sim(ROOM);
    settle(&mut sim, 30);
    assert_eq!(pickup(&mut sim, HEART), Some((false, None)), "precondition: the heart starts whole");
    let start = player_at(&mut sim);
    let collected_at = collect(&mut sim, HEART);
    place_the_player(&mut sim, start);
    settle(&mut sim, 2);
    (sim, collected_at)
}

/// Back before the four seconds are up: still gone, for what remains.
#[test]
fn a_heart_collected_before_leaving_is_still_gone_on_a_quick_return() {
    let (mut sim, collected_at) = harvested_room();
    assert_eq!(cross_to(&mut sim, HUB), HUB);
    assert_eq!(cross_to(&mut sim, ROOM), ROOM);
    let away = now(&sim) - collected_at;
    assert!(
        away < REGROW_S,
        "precondition: the round trip took {away:.2} s of the session clock, inside the {REGROW_S} s regrowth"
    );
    let (collected, remaining) = pickup(&mut sim, HEART).expect("the heart's room is built again");
    let remaining = remaining.unwrap_or(0.0);
    assert!(
        collected && (remaining - (REGROW_S - away)).abs() <= 2.0 / 60.0,
        "after {away:.3} s away: (collected, remaining) = ({collected}, {remaining:.3}); the regrowth must still need {:.3} s",
        REGROW_S - away
    );
}

/// Harvest, leave, wait, return: it has regrown, and its record is gone.
#[test]
fn a_heart_whose_regrowth_fell_due_while_away_has_regrown_on_return() {
    let (mut sim, collected_at) = harvested_room();
    assert_eq!(cross_to(&mut sim, HUB), HUB);
    settle(&mut sim, 300);
    assert_eq!(cross_to(&mut sim, ROOM), ROOM);
    assert!(now(&sim) - collected_at > REGROW_S, "precondition: the regrowth fell due while away");
    settle(&mut sim, 2);
    assert_eq!(
        (pickup(&mut sim, HEART), records(&sim)),
        (Some((false, None)), 0),
        "(the heart, the schedule's records) after the regrowth fell due away from the room"
    );
}

/// In its live room the heart regrows when its time is up. The control is the
/// plain heart beside it, collected in the same tick range: it does not.
#[test]
fn a_heart_regrows_in_its_live_room_and_a_plain_one_does_not() {
    let (mut sim, _) = harvested_room();
    let start = player_at(&mut sim);
    collect(&mut sim, PLAIN);
    place_the_player(&mut sim, start);
    settle(&mut sim, (REGROW_S * 60.0) as usize + 30);
    assert_eq!(
        (pickup(&mut sim, HEART), pickup(&mut sim, PLAIN), records(&sim)),
        (Some((false, None)), Some((true, None)), 0),
        "(the regrowing heart, the plain heart, the schedule's records) after the regrowth"
    );
}

/// A replay is a fresh attempt at the room: the heart is whole.
#[test]
fn a_replay_rebuilds_a_collected_heart_whole() {
    let (mut sim, _) = harvested_room();
    sim.world_mut().write_message(
        ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual(),
    );
    settle(&mut sim, 30);
    assert_eq!(
        (pickup(&mut sim, HEART), records(&sim)),
        (Some((false, None)), 0),
        "(the heart, the schedule's records) after a replay of its room"
    );
}

/// (collected, the schedule's records) after Alice banks a checkpoint, the
/// heart is collected, and Alice dies. With `bob_holds_it`, Bob holds the
/// heart's room and Alice is in the hub; else Alice is alone in it.
fn heart_after_alices_death(bob_holds_it: bool) -> (bool, usize) {
    // Longer than the death's interlude, so only the death can regrow it.
    const LONG_S: f32 = 30.0;
    let mut sim = if bob_holds_it {
        crate::two_players_two_live_rooms::alice_leaves_bob_in(
            ROOM,
            HUB,
            Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
            cross_to,
        )
        .0
    } else {
        let mut sim = fixed_60hz_room_sim(ROOM);
        settle(&mut sim, 30);
        sim
    };
    crate::death_restores_the_checkpoint::commit_a_checkpoint(&mut sim);
    // Collected as the collection leaves it: `Collected` and the countdown
    // of its regrowth. Alice is not in the room to stand on it.
    {
        let world = sim.world_mut();
        let mut q = world.query::<(bevy::prelude::Entity, &FeatureName)>();
        let heart = q
            .iter(world)
            .find(|(_, feature)| feature.0.as_str() == HEART)
            .map(|(entity, _)| entity)
            .expect("the room authors the regrowing heart");
        world.entity_mut(heart).insert((Collected, RespawnTimer(LONG_S)));
    }
    settle(&mut sim, 2);
    assert_eq!(
        (pickup(&mut sim, HEART).map(|(collected, _)| collected), records(&sim)),
        (Some(true), 1),
        "precondition: the heart is collected and its regrowth is scheduled"
    );
    crate::death_restores_the_checkpoint::die(&mut sim);
    let collected = pickup(&mut sim, HEART).is_some_and(|(collected, _)| collected);
    (collected, records(&sim))
}

/// Q151: a death is local to its participant and room. A heart collected in
/// Bob's live room after the checkpoint stays collected when Alice dies in
/// hers, and its regrowth record stays. The live timer serves it, as it serves
/// a breakable's respawn: the restore forgets every record, and
/// `regrow_pickups` records it again on the next tick from the running timer.
/// The control is Alice alone in the room: her restore rebuilds it, so the
/// heart is whole and the record is gone.
#[test]
fn a_death_keeps_the_regrowth_of_a_heart_in_another_players_room() {
    assert_eq!(
        heart_after_alices_death(false),
        (false, 0),
        "control: a death in the heart's own room rebuilds it whole and forgets its regrowth"
    );
    assert_eq!(
        heart_after_alices_death(true),
        (true, 1),
        "Alice's death in the hub took back the regrowth of the heart collected in Bob's live room"
    );
}

/// The one-time heart's row in the occurrence ledger.
fn ledger_row(sim: &mut Platformer2dSimHarness, name: &str) -> Option<String> {
    let world = sim.world_mut();
    let mut q = world.query::<(&FeatureName, &ambition_platformer2d::platformer::sim_id::SimId)>();
    let sim_id = q
        .iter(world)
        .find(|(feature, _)| feature.0.as_str() == name)
        .map(|(_, sim_id)| sim_id.clone())?;
    let ledger = world.resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>();
    Some(format!("{:?}", ledger.whereabouts(&sim_id)))
}

/// Q154: a heart authored `Never` is gone for the run once taken. Leave its
/// room, so the room retires, and come back: the room is built again without
/// it. Before, a collected pickup kept no record, so every rebuild of its
/// room built it again. The control is the same heart before the crossing:
/// collected where it lies, with its `Consumed` row.
#[test]
fn a_one_time_heart_stays_gone_when_its_room_is_built_again() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    settle(&mut sim, 30);
    let start = player_at(&mut sim);
    collect(&mut sim, PLAIN);
    place_the_player(&mut sim, start);
    settle(&mut sim, 2);
    assert_eq!(
        (pickup(&mut sim, PLAIN), ledger_row(&mut sim, PLAIN).as_deref()),
        (Some((true, None)), Some("Some(Consumed)")),
        "control: taken, where it lies, and remembered"
    );
    assert_eq!(cross_to(&mut sim, HUB), HUB);
    assert_eq!(cross_to(&mut sim, ROOM), ROOM);
    settle(&mut sim, 2);
    assert_eq!(pickup(&mut sim, PLAIN), None, "the rebuilt room built the one-time heart again");
}

/// The stable id of the pickup `name`, which its room must have built.
fn sim_id_of(sim: &mut Platformer2dSimHarness, name: &str) -> ambition_platformer2d::platformer::sim_id::SimId {
    let world = sim.world_mut();
    let mut q = world.query::<(&FeatureName, &ambition_platformer2d::platformer::sim_id::SimId)>();
    q.iter(world)
        .find(|(feature, _)| feature.0.as_str() == name)
        .map(|(_, sim_id)| sim_id.clone())
        .unwrap_or_else(|| panic!("the room authors the pickup '{name}'"))
}

/// Whether the one-time heart is there to take after Alice dies, when she
/// took it after the checkpoint (`after`) or before it.
fn one_time_heart_after_a_death(taken_after_the_checkpoint: bool) -> Option<(bool, Option<f32>)> {
    let mut sim = fixed_60hz_room_sim(ROOM);
    settle(&mut sim, 30);
    let start = player_at(&mut sim);
    if taken_after_the_checkpoint {
        crate::death_restores_the_checkpoint::commit_a_checkpoint(&mut sim);
    }
    collect(&mut sim, PLAIN);
    place_the_player(&mut sim, start);
    settle(&mut sim, 2);
    if !taken_after_the_checkpoint {
        crate::death_restores_the_checkpoint::commit_a_checkpoint(&mut sim);
    }
    crate::death_restores_the_checkpoint::die(&mut sim);
    pickup(&mut sim, PLAIN)
}

/// Q154 on the death horizon: the checkpoint is the world a death goes back
/// to. A one-time heart taken after it is back, whole; one taken before it
/// stays gone (the control), because the restored ledger remembers it.
#[test]
fn a_death_brings_back_a_one_time_heart_only_if_taken_after_the_checkpoint() {
    assert_eq!(
        (one_time_heart_after_a_death(true), one_time_heart_after_a_death(false)),
        (Some((false, None)), None),
        "(taken after the checkpoint, taken before it): the heart after Alice's death"
    );
}

/// The pickup `name`, collected as a collection leaves it. Bob's body is not
/// in the player population (no road seats a second player yet, Q153), so he
/// cannot stand on it; the pickup is in his live room, and that is what owns it.
fn collected_in_bobs_room(sim: &mut Platformer2dSimHarness, name: &str) {
    let world = sim.world_mut();
    let mut q = world.query::<(bevy::prelude::Entity, &FeatureName)>();
    let entity = q
        .iter(world)
        .find(|(_, feature)| feature.0.as_str() == name)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("the room authors the pickup '{name}'"));
    world.entity_mut(entity).insert(Collected);
}

/// The one-time heart when Alice comes back to its room after her death. It
/// was taken after the checkpoint, in Bob's room, which he then left (`by_bob`),
/// or by Alice alone, who then left. Either way its room is not live when
/// Alice dies.
fn one_time_heart_after_a_death_elsewhere(by_bob: bool) -> (Option<(bool, Option<f32>)>, String) {
    let heart;
    let mut sim = if by_bob {
        let (mut sim, _) = crate::two_players_two_live_rooms::alice_leaves_bob_in(
            ROOM,
            HUB,
            Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
            cross_to,
        );
        crate::death_restores_the_checkpoint::commit_a_checkpoint(&mut sim);
        heart = sim_id_of(&mut sim, PLAIN);
        collected_in_bobs_room(&mut sim, PLAIN);
        settle(&mut sim, 2);
        assert_eq!(
            ledger_row(&mut sim, PLAIN).as_deref(),
            Some("Some(Consumed)"),
            "precondition: the heart Bob's room holds is consumed"
        );
        let hub = crate::two_players_two_live_rooms::live_rooms(&mut sim)
            .into_iter()
            .find(|(_, id)| id == HUB)
            .map(|(room, _)| room)
            .expect("precondition: Alice holds the hub");
        crate::two_players_two_live_rooms::bob_goes_from(&mut sim, ROOM, HUB, hub);
        sim
    } else {
        let mut sim = fixed_60hz_room_sim(ROOM);
        settle(&mut sim, 30);
        crate::death_restores_the_checkpoint::commit_a_checkpoint(&mut sim);
        heart = sim_id_of(&mut sim, PLAIN);
        collect(&mut sim, PLAIN);
        assert_eq!(cross_to(&mut sim, HUB), HUB);
        settle(&mut sim, 30);
        sim
    };
    assert!(
        !crate::two_players_two_live_rooms::live_rooms(&mut sim).iter().any(|(_, id)| id == ROOM),
        "precondition: the heart's room retired"
    );
    crate::death_restores_the_checkpoint::die(&mut sim);
    // A restore that fails its verification leaves the ledger as it was and
    // stops play, which would also leave the heart gone.
    let outcome = format!(
        "{:?}",
        ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::actors::session::checkpoint::SessionCheckpointOutcomes>(sim.world())
        .expect("the live session root carries the checkpoint coordinator")
            .latest()
    );
    assert!(outcome.starts_with("Some(Committed"), "precondition: the death's restore committed: {outcome}");
    if sim.observation().active_room != ROOM {
        assert_eq!(cross_to(&mut sim, ROOM), ROOM);
    }
    settle(&mut sim, 2);
    let row = format!(
        "{:?}",
        sim.world()
            .resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
            .whereabouts(&heart)
    );
    (pickup(&mut sim, PLAIN), row)
}

/// Q151 for a one-time pickup: a heart Bob took after the checkpoint, in a
/// room he then left, is his consequence. Alice's death elsewhere does not
/// bring it back. The restore puts the pinned ledger back, which has no row
/// for it, and no live room is there to write the row again; so each row
/// consumed since the checkpoint names whose horizons own it, and the restore
/// writes again a row that a spared participant owns. The control is the same
/// heart taken by Alice alone: her death brings it back, whole.
#[test]
fn a_death_keeps_gone_a_one_time_heart_another_player_took_in_a_room_he_left() {
    assert_eq!(
        one_time_heart_after_a_death_elsewhere(false),
        (Some((false, None)), "None".to_string()),
        "control: Alice's death brings back the one-time heart she took"
    );
    assert_eq!(
        one_time_heart_after_a_death_elsewhere(true),
        (None, "Some(Consumed)".to_string()),
        "Alice's death in the hub brought back the one-time heart Bob took in a room he left"
    );
}

/// The room beside the hub that authors a coin pickup.
const COIN_ROOM: &str = "basement_treasure";
const COIN: &str = "currency pickup";

fn alices_balance(sim: &mut Platformer2dSimHarness) -> i32 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<
        &ambition_platformer2d::characters::actor::BodyWallet,
        bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
    >();
    q.single(world).expect("Alice's body has a wallet").balance
}

/// (Alice's balance after her death against the checkpoint's, whether the coin
/// is still gone in its room). After the checkpoint in the hub, Alice goes
/// into the coin room and takes the coin, then goes back to the hub and dies.
/// With `bob_stays`, Bob is in the coin room the whole time, so it stays
/// live; without him it retires when she leaves.
fn a_coin_taken_after_the_checkpoint(bob_stays: bool) -> (i32, Option<bool>) {
    a_coin_taken_after_the_checkpoint_then(bob_stays, 1)[0].0
}

/// [`a_coin_taken_after_the_checkpoint`], with Alice dying `deaths` times in
/// the hub. One reading after each death, with the owners of the coin's
/// grant that the record keeps.
fn a_coin_taken_after_the_checkpoint_then(
    bob_stays: bool,
    deaths: usize,
) -> Vec<((i32, Option<bool>), Vec<Vec<ambition_platformer2d::characters::control::PlayerSlot>>)> {
    let mut sim = if bob_stays {
        crate::two_players_two_live_rooms::alice_leaves_bob_in(
            COIN_ROOM,
            HUB,
            Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
            cross_to,
        )
        .0
    } else {
        let mut sim = fixed_60hz_room_sim(COIN_ROOM);
        settle(&mut sim, 30);
        assert_eq!(cross_to(&mut sim, HUB), HUB);
        settle(&mut sim, 30);
        sim
    };
    crate::death_restores_the_checkpoint::commit_a_checkpoint(&mut sim);
    let at_the_checkpoint = alices_balance(&mut sim);
    assert_eq!(cross_to(&mut sim, COIN_ROOM), COIN_ROOM);
    settle(&mut sim, 10);
    collect(&mut sim, COIN);
    assert_eq!(alices_balance(&mut sim), at_the_checkpoint + 25, "precondition: the coin paid Alice");
    assert_eq!(cross_to(&mut sim, HUB), HUB);
    settle(&mut sim, 30);
    let coin_room_live =
        crate::two_players_two_live_rooms::live_rooms(&mut sim).iter().any(|(_, id)| id == COIN_ROOM);
    assert_eq!(coin_room_live, bob_stays, "precondition: the coin room is live while Bob is in it");
    let mut readings = Vec::new();
    for _ in 0..deaths {
        crate::death_restores_the_checkpoint::die(&mut sim);
        let outcome = format!(
            "{:?}",
            ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::actors::session::checkpoint::SessionCheckpointOutcomes>(sim.world())
        .expect("the live session root carries the checkpoint coordinator")
                .latest()
        );
        assert!(outcome.starts_with("Some(Committed"), "precondition: the death's restore committed: {outcome}");
        settle(&mut sim, 2);
        let owners = sim
            .world()
            .resource::<ambition_platformer2d::actors::items::pickup::RewardGrantsSinceCheckpoint>()
            .grants()
            .iter()
            .filter_map(|grant| match &grant.source {
                ambition_platformer2d::actors::items::pickup::GrantSource::Authored { owners } => Some(owners.clone()),
                _ => None,
            })
            .collect();
        readings.push((
            (
                alices_balance(&mut sim) - at_the_checkpoint,
                pickup(&mut sim, COIN).map(|(collected, _)| collected),
            ),
            owners,
        ));
    }
    readings
}

/// Q151 for a grant: a coin Alice took after the checkpoint in Bob's room,
/// while he was there, is a consequence Bob's horizon owns too. Alice's death
/// elsewhere leaves the coin gone in his live room, so she keeps what it paid.
/// The control is the coin taken with nobody else in its room: her death
/// takes the money back, and the room built again on her return has the coin.
#[test]
fn a_death_keeps_the_coin_taken_in_another_players_live_room() {
    assert_eq!(
        a_coin_taken_after_the_checkpoint(false),
        (0, None),
        "control: alone, Alice's death takes back the coin's money, and its room is not live"
    );
    assert_eq!(
        a_coin_taken_after_the_checkpoint(true),
        (25, Some(true)),
        "(money kept, the coin still gone in Bob's live room)"
    );
}

/// The coin of [`a_death_keeps_the_coin_taken_in_another_players_live_room`],
/// and Alice dies twice (review 2026-10-05, finding 1). The first restore
/// keeps the coin's money, because Bob's horizon owns the grant. It must also
/// keep the grant recorded, now owned by Bob alone (Alice's horizon went
/// back), or the second restore of the same checkpoint has nothing to keep
/// and takes the money back while the coin stays gone in Bob's room. The
/// control is the first death.
#[test]
fn a_second_death_keeps_the_coin_taken_in_another_players_live_room() {
    use ambition_platformer2d::characters::control::PlayerSlot;
    let kept = ((25, Some(true)), vec![vec![PlayerSlot(1)]]);
    assert_eq!(
        a_coin_taken_after_the_checkpoint_then(true, 2),
        vec![kept.clone(), kept],
        "((money kept, the coin still gone in Bob's live room), the owners of the grant the record keeps) \
         after each of Alice's two deaths"
    );
}
