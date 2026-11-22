//! The Supreme Leader's fight, played headlessly in the War Room: the real
//! room, his scripted pattern, and the conducted module that performs it
//! (`ambition_content_modules::supreme_leader`). The shape of the fight is in
//! `docs/planning/game/bosses.md`, "The Supreme Leader".

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_content::bosses::supreme_leader::{conductor_of, Move, SupremeLeaderView, SUPREME_LEADER_ID};
use ambition_platformer2d::boss_encounter::{BossConfig, BossEncounter, BossEncounterPhase};
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

use crate::boss_lifecycle::{boss_cleared, boss_phase};

const ROOM: &str = "hall_supreme_leader_war_room";
const PLACEMENT: &str = "hall.supreme_leader";
const UPLINK: &str = "npc_command_uplink";

/// The War Room, his intro over.
fn war_room() -> Platformer2dSimHarness {
    let opts = crate::common::sandbox_options()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED: the tolerant road falls back to the start room, which has
        // no Supreme Leader, and every assertion below would be vacuous.
        .with_required_start_room(ROOM);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the War Room builds headlessly");
    step_until(&mut sim, 60 * 8, "his intro to end", |sim| leader(sim).view.woke);
    sim
}

#[derive(Debug)]
struct Leader {
    entity: Entity,
    kin: ae::BodyKinematics,
    view: SupremeLeaderView,
}

fn leader(sim: &mut Platformer2dSimHarness) -> Leader {
    let world = sim.world_mut();
    let (entity, kin) = world
        .query::<(Entity, &BossConfig, &ae::BodyKinematics)>()
        .iter(world)
        .find(|(_, config, _)| config.behavior.id == SUPREME_LEADER_ID)
        .map(|(entity, _, kin)| (entity, kin.clone()))
        .expect("the Supreme Leader is in the War Room");
    let view = conductor_of(world, entity).expect("his conductor has measured the room");
    Leader { entity, kin, view }
}

fn health_of(sim: &mut Platformer2dSimHarness, entity: Entity) -> i32 {
    sim.world().get::<BodyHealth>(entity).map_or(0, |h| h.current())
}

/// The encounter's guard on him, as the engine holds it.
fn guarded(sim: &mut Platformer2dSimHarness) -> bool {
    let world = sim.world_mut();
    let mut q = world.query::<(&BossConfig, &BossEncounter)>();
    q.iter(world).find(|(config, _)| config.behavior.id == SUPREME_LEADER_ID).map_or(false, |(_, s)| s.guarded)
}

/// His uplinks standing in the room: their bodies and health.
fn uplinks(sim: &mut Platformer2dSimHarness) -> Vec<(Entity, ae::BodyKinematics, i32)> {
    let world = sim.world_mut();
    let mut q = world.query::<(Entity, &ambition_platformer2d::combat::components::FeatureId, &ae::BodyKinematics, &BodyHealth)>();
    q.iter(world)
        .filter(|(_, id, _, health)| id.0.starts_with("uplink:") && health.alive())
        .map(|(entity, _, kin, health)| (entity, kin.clone(), health.current()))
        .collect()
}

fn place_player(sim: &mut Platformer2dSimHarness, at: ae::Vec2) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), PrimaryPlayerOnly>();
    let (mut clusters, mut model) = q.iter_mut(world).next().expect("the player");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

fn player(sim: &mut Platformer2dSimHarness) -> (ae::BodyKinematics, i32) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(&ae::BodyKinematics, &BodyHealth), PrimaryPlayerOnly>();
    let (kin, health) = q.iter(world).next().expect("the player");
    (kin.clone(), health.current())
}

fn untouchable_player(sim: &mut Platformer2dSimHarness, untouchable: bool) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, untouchable);
    }
}

/// Keep the player's health topped up, so a long run is not ended by it.
fn heal_player(sim: &mut Platformer2dSimHarness) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        let max = health.max();
        health.health.current = max;
    }
}

/// Stand on the floor at `x`.
fn stand_at(sim: &mut Platformer2dSimHarness, x: f32) {
    let hall = leader(sim).view.hall.expect("he measured his room");
    let (kin, _) = player(sim);
    place_player(sim, ae::Vec2::new(x, hall.floor - kin.size().y * 0.5 - 1.0));
}

/// A real blow on `entity`'s body, as a player's sword lands one.
fn strike(sim: &mut Platformer2dSimHarness, entity: Entity, damage: i32) {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    let at = sim.world().get::<ae::BodyKinematics>(entity).expect("a body").pos;
    let player = {
        let world = sim.world_mut();
        world.query_filtered::<Entity, PrimaryPlayerOnly>().iter(world).next()
    };
    sim.world_mut().write_message(HitEvent {
        origin: None,
        properties: Default::default(),
        contact: None,
        pieces: Vec::new(),
        volume: ae::Aabb::new(at, ae::Vec2::splat(30.0)).into(),
        damage,
        source: HitSource::Melee,
        attacker: player,
        room: None,
        target: HitTarget::UnresolvedFeatures,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
        strike_sfx: None,
        attacker_move_instance: None,
    });
}

fn step_until(
    sim: &mut Platformer2dSimHarness,
    frames: usize,
    what: &str,
    mut until: impl FnMut(&mut Platformer2dSimHarness) -> bool,
) {
    for _ in 0..frames {
        sim.step(AgentAction::default());
        if until(sim) {
            return;
        }
    }
    panic!("waited {frames} frames for {what}; last: {:?}", leader(sim));
}

/// He addresses the room and calls his first uplink down at the west end;
/// then the fight starts, and the uplink stands.
#[test]
fn his_intro_calls_the_first_uplink_down_at_the_west_end() {
    let mut sim = war_room();
    let l = leader(&mut sim);
    assert_eq!(l.view.uplinks_called, 1, "one uplink called in his intro: {l:?}");
    assert!(l.view.uplink_up, "it stands: {l:?}");
    step_until(&mut sim, 60, "the uplink's body", |sim| !uplinks(sim).is_empty());
    let standing = uplinks(&mut sim);
    assert_eq!(standing.len(), 1, "one uplink at a time: {standing:?}");
    let hall = leader(&mut sim).view.hall.expect("room");
    let (_, kin, _) = &standing[0];
    assert!(kin.pos.x < (hall.left + hall.right) * 0.5, "the first uplink is at the west end: {kin:?}");
    assert!(
        (kin.pos.y + kin.size().y * 0.5 - hall.floor).abs() < 6.0,
        "it stands on the floor: {kin:?}, floor {}",
        hall.floor
    );
    assert_eq!(boss_phase(sim.world_mut(), PLACEMENT), Some(BossEncounterPhase::Phase1));
    let _ = UPLINK;
}

/// While the uplink stands, an order is guarded: a blow on him lands on
/// nothing. The lines it fires down the room meet a body standing on the
/// floor in front of it, and pass a body standing behind it.
#[test]
fn an_order_through_a_standing_uplink_is_guarded_and_its_lines_sweep_the_floor() {
    let mut sim = war_room();
    step_until(&mut sim, 60 * 2, "the uplink's body", |sim| !uplinks(sim).is_empty());
    let (_, uplink, _) = uplinks(&mut sim)[0].clone();
    let hall = leader(&mut sim).view.hall.expect("room");
    // Behind the uplink, by the west wall: out of its lines.
    let behind = (hall.left + uplink.pos.x) * 0.5;
    stand_at(&mut sim, behind);
    step_until(&mut sim, 60 * 12, "an order", |sim| {
        stand_at(sim, behind);
        heal_player(sim);
        matches!(leader(sim).view.performing, Some((Move::Lines, false)))
    });
    assert!(guarded(&mut sim), "an order through a standing uplink is guarded");
    let l = leader(&mut sim);
    let before = health_of(&mut sim, l.entity);
    strike(&mut sim, l.entity, 5);
    sim.step(AgentAction::default());
    assert_eq!(health_of(&mut sim, l.entity), before, "a blow landed through his guard");

    // Behind it the whole order: no line meets you.
    let (_, at_start) = player(&mut sim);
    let mut struck_behind = false;
    for _ in 0..60 * 3 {
        stand_at(&mut sim, behind);
        sim.step(AgentAction::default());
        let (_, hp) = player(&mut sim);
        struck_behind |= hp < at_start;
        if !matches!(leader(&mut sim).view.performing, Some((Move::Lines, _))) {
            break;
        }
    }
    assert!(!struck_behind, "a line met a body behind the uplink");

    // In front of it, on the floor, at the next lines: the low line meets you.
    let front = uplink.pos.x + 260.0;
    step_until(&mut sim, 60 * 30, "the next lines", |sim| {
        stand_at(sim, front);
        heal_player(sim);
        matches!(leader(sim).view.performing, Some((Move::Lines, false)))
    });
    let (_, full) = player(&mut sim);
    let mut struck_front = false;
    for _ in 0..60 * 3 {
        stand_at(&mut sim, front);
        sim.step(AgentAction::default());
        let (_, hp) = player(&mut sim);
        struck_front |= hp < full;
        if struck_front {
            break;
        }
    }
    assert!(struck_front, "the low line did not meet a body standing on the floor in front of the uplink");
}

/// Break the uplink and he reports (a hand to his ear, open to every blow);
/// an order after it is unguarded, and a blow lands.
#[test]
fn a_broken_uplink_leaves_his_orders_unanswered() {
    let mut sim = war_room();
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 60 * 2, "the uplink's body", |sim| !uplinks(sim).is_empty());
    let (_, at, hp) = uplinks(&mut sim)[0].clone();
    // The player's own sword: stand east of the mast, face it, and swing.
    let mut frames = 0;
    let mut reported = false;
    while !uplinks(&mut sim).is_empty() && frames < 60 * 20 {
        stand_at(&mut sim, at.pos.x + 34.0);
        let swing = frames % 20 == 0;
        sim.step(AgentAction { move_x: if frames % 20 == 1 { -1.0 } else { 0.0 }, attack: swing, ..AgentAction::default() });
        frames += 1;
    }
    // He reports the loss (a hand to his ear) until his next tell.
    for _ in 0..30 {
        reported |= leader(&mut sim).view.reporting;
        if reported {
            break;
        }
        sim.step(AgentAction::default());
    }
    assert!(uplinks(&mut sim).is_empty(), "the uplink ({hp} blows) did not break under the sword in {frames} frames");
    assert!(!leader(&mut sim).view.uplink_up, "he did not notice the loss");
    assert!(reported, "he did not report the loss");
    // The next order: no guard, and a blow lands.
    step_until(&mut sim, 60 * 20, "an order", |sim| {
        leader(sim).view.performing.is_some_and(|(mv, _)| mv.is_order())
    });
    for _ in 0..90 {
        assert!(!guarded(&mut sim), "an order with no uplink is guarded: {:?}", leader(&mut sim));
        if leader(&mut sim).view.reporting {
            break;
        }
        sim.step(AgentAction::default());
    }
    let l = leader(&mut sim);
    let before = health_of(&mut sim, l.entity);
    strike(&mut sim, l.entity, 3);
    sim.step(AgentAction::default());
    assert!(health_of(&mut sim, l.entity) < before, "a blow did not land on an unanswered order");
    assert_eq!(uplinks(&mut sim).len(), 0, "a broken uplink came back by itself in the Command");
}

/// Fought to the end with real blows between his moves: the Command, the
/// Contingency (a new uplink at the east end), the Disruption, the
/// Emergency Powers (the uplink drained away, the Mandate and its
/// overheat), and his fall through his gate. His fall is saved and drops
/// his chest.
#[test]
fn he_falls_through_his_three_phases_and_escapes_through_his_gate() {
    let mut sim = war_room();
    untouchable_player(&mut sim, true);
    let mut phases: Vec<BossEncounterPhase> = Vec::new();
    let (mut east_uplink, mut mandate, mut overheat, mut emergency) = (false, false, false, false);
    let hall = leader(&mut sim).view.hall.expect("room");
    let middle = (hall.left + hall.right) * 0.5;
    for frame in 0..60 * 150 {
        if let Some(phase) = boss_phase(sim.world_mut(), PLACEMENT) {
            if phases.last() != Some(&phase) {
                phases.push(phase);
            }
            let l = leader(&mut sim);
            east_uplink |= uplinks(&mut sim).iter().any(|(_, kin, _)| kin.pos.x > middle);
            mandate |= matches!(l.view.performing, Some((Move::Mandate, true)));
            overheat |= l.view.overheated;
            emergency |= l.view.emergency;
            // A blow a second between his own moves, so each phase plays;
            // slower once he burns, so the Mandate comes round.
            let every = if l.view.emergency && !mandate { 120 } else { 60 };
            if !phase.boss_invulnerable() && !l.view.guarded && frame % every == 0 {
                strike(&mut sim, l.entity, 4);
            }
            if matches!(phase, BossEncounterPhase::Death) {
                break;
            }
        }
        let x = (leader(&mut sim).kin.pos.x - 200.0).max(hall.left + 30.0);
        stand_at(&mut sim, x);
        sim.step(AgentAction::default());
    }
    for _ in 0..60 * 4 {
        sim.step(AgentAction::default());
        if boss_cleared(&sim, PLACEMENT) {
            break;
        }
    }
    for wanted in [BossEncounterPhase::Phase1, BossEncounterPhase::Phase2, BossEncounterPhase::Enrage, BossEncounterPhase::Death] {
        assert!(phases.contains(&wanted), "the fight never reached {wanted:?}: {phases:?}");
    }
    assert!(east_uplink, "the Contingency called no uplink down at the east end");
    assert!(emergency, "the Emergency Powers never came");
    assert!(mandate, "the Mandate was never fired");
    assert!(overheat, "the sceptre never overheated after the Mandate");
    assert!(uplinks(&mut sim).is_empty(), "an uplink outlived him");
    assert!(boss_cleared(&sim, PLACEMENT), "his fall was not saved (phases {phases:?})");
    let chests = {
        let world = sim.world_mut();
        world.query::<&ambition_platformer2d::combat::components::BossRewardChest>().iter(world).count()
    };
    assert_eq!(chests, 1, "his fall dropped no chest");
}

/// The hall's life switch brings him back: a new fight from his address,
/// a new uplink at the west end, his own size.
#[test]
fn the_life_switch_brings_him_back_for_a_new_fight() {
    use ambition_platformer2d::encounter::switches::SwitchActivationQueue;
    let mut sim = war_room();
    let press = |sim: &mut Platformer2dSimHarness| {
        ambition_platformer2d::platformer::lifecycle::session_world_component_mut::<SwitchActivationQueue>(sim.world_mut())
            .expect("the live session root carries the switch queue")
            .0
            .push(
                ambition_platformer2d::encounter::SwitchActivation {
                    id: "hall_supreme_leader_life_switch".into(),
                    action: "BossLife".into(),
                    target_encounter: PLACEMENT.into(),
                }
                .into(),
            );
    };
    press(&mut sim);
    for _ in 0..60 * 3 {
        sim.step(AgentAction::default());
    }
    assert!(boss_cleared(&sim, PLACEMENT), "the switch did not beat him");
    assert!(uplinks(&mut sim).is_empty(), "his uplink outlived him");
    press(&mut sim);
    step_until(&mut sim, 60 * 10, "a new fight", |sim| {
        let l = leader(sim);
        l.view.woke && l.view.uplink_up && l.view.transitions == 0
    });
    let l = leader(&mut sim);
    assert!(!l.view.emergency, "he came back with his Emergency Powers: {l:?}");
    assert!(l.view.uplinks_called >= 1, "he came back with no uplink: {l:?}");
}
