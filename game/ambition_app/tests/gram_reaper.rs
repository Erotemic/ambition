#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::characters::brain::{BossAttackProfile, BossAttackState};
use ambition_platformer2d::combat::strike::{Hitbox, HitboxAnchor};
use ambition_platformer2d::engine_core::{BodyKinematics, Vec2};
use ambition_platformer2d::extension::ExtensionFaults;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::vfx::vfx::AttackMarks;
use bevy::prelude::Entity;

const PRACTICE: &str = "hall_gram_reaper_arena";

#[derive(Clone, Copy, Debug, PartialEq)]
struct Region {
    center: Vec2,
    half: Vec2,
}

fn set_up(room: &str) -> (Platformer2dSimHarness, Entity, Entity) {
    let mut sim =
        Platformer2dSimHarness::new_with_options(crate::common::fixed_60hz_room_options(room))
            .expect("load the Reaper arena");
    sim.step_n(AgentAction::default(), 3);
    let (boss, player) = {
        let world = sim.world_mut();
        let boss = world
            .query::<(Entity, &BossConfig)>()
            .iter(world)
            .find(|(_, config)| config.behavior.id == "gram_reaper_boss")
            .unwrap()
            .0;
        let player = world
            .query_filtered::<Entity, PrimaryPlayerOnly>()
            .single(world)
            .unwrap();
        (boss, player)
    };
    let at = sim.world().get::<BodyKinematics>(boss).unwrap().pos;
    sim.teleport_player((at.x - 128.0, at.y));
    (sim, boss, player)
}

fn marks(sim: &Platformer2dSimHarness, boss: Entity) -> Vec<(u32, Region, bool)> {
    sim.world()
        .resource::<AttackMarks>()
        .0
        .iter()
        .filter(|mark| mark.key.owner == boss)
        .map(|mark| {
            (
                mark.key.slot,
                Region {
                    center: mark.center,
                    half: mark.half,
                },
                mark.active,
            )
        })
        .collect()
}

fn hitboxes(sim: &mut Platformer2dSimHarness, boss: Entity) -> Vec<(Entity, Region)> {
    let world = sim.world_mut();
    world
        .query::<(Entity, &Hitbox)>()
        .iter(world)
        .filter_map(|(entity, hitbox)| {
            let HitboxAnchor::World { center } = hitbox.anchor else {
                return None;
            };
            (hitbox.owner == boss).then_some((
                entity,
                Region {
                    center,
                    half: hitbox.half_extent,
                },
            ))
        })
        .collect()
}

fn phase_has(profile: &Option<BossAttackProfile>, key: &str) -> bool {
    matches!(profile, Some(BossAttackProfile::Special(name)) if name == key)
}

fn check_faults(sim: &Platformer2dSimHarness) {
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent);
}

fn check_volley(sim: &mut Platformer2dSimHarness, boss: Entity) {
    let mut announced = Vec::new();
    let mut order = Vec::new();
    for _ in 0..480 {
        sim.step(AgentAction::default());
        check_faults(sim);
        let marks = marks(sim, boss);
        if order.is_empty() && marks.len() == 3 && marks.iter().all(|(_, _, live)| !live) {
            if announced.is_empty() {
                announced = marks.iter().map(|(_, region, _)| *region).collect();
            }
            assert!(hitboxes(sim, boss).is_empty());
        }
        if announced.is_empty() {
            continue;
        }
        if let Some((slot, region, _)) = marks.iter().find(|(_, _, live)| *live) {
            assert_eq!(announced.get(*slot as usize), Some(region));
            let boxes = hitboxes(sim, boss);
            assert_eq!(boxes.len(), 1);
            assert_eq!(boxes[0].1, *region);
            if order.last() != Some(slot) {
                order.push(*slot);
            }
        }
        if order.len() == 3 && marks.is_empty() {
            assert_eq!(order, [0, 1, 2]);
            assert!(hitboxes(sim, boss).is_empty());
            return;
        }
    }
    panic!("the three announced cuts did not fire in order");
}

#[test]
fn the_gram_reaper_sweep_keeps_the_mark_and_releases_one_hitbox_in_both_arenas() {
    for room in ["math_hotel_gram_reaper", PRACTICE] {
        let (mut sim, boss, player) = set_up(room);
        let hp = sim
            .world()
            .get::<BodyHealth>(player)
            .unwrap()
            .health
            .current;
        let mut warned = None;
        let mut strike = None;
        let mut recovered = false;
        for _ in 0..660 {
            sim.step(AgentAction::default());
            check_faults(&sim);
            let marks = marks(&sim, boss);
            let graph_warning = phase_has(
                &sim.world()
                    .get::<BossAttackState>(boss)
                    .unwrap()
                    .telegraph_profile,
                "reaper_graph_cut",
            );
            let boxes = hitboxes(&mut sim, boss);
            if matches!(marks.as_slice(), [(0, _, false)]) && strike.is_none() {
                let region = marks[0].1;
                if let Some(first) = warned {
                    assert_eq!(first, region, "the mark moves in {room}");
                } else {
                    warned = Some(region);
                    let half_height =
                        sim.world().get::<BodyKinematics>(player).unwrap().size().y * 0.5;
                    sim.teleport_player((
                        region.center.x,
                        region.center.y + region.half.y - half_height,
                    ));
                }
                assert!(boxes.is_empty(), "the warning hurts in {room}");
            } else if matches!(marks.as_slice(), [(0, _, true)]) {
                assert_eq!(
                    Some(marks[0].1),
                    warned,
                    "the strike misses its mark in {room}"
                );
                assert_eq!(boxes.len(), 1);
                assert_eq!(boxes[0].1, marks[0].1);
                assert_eq!(
                    *strike.get_or_insert(boxes[0].0),
                    boxes[0].0,
                    "the strike respawns"
                );
            } else if strike.is_some() && marks.is_empty() {
                assert!(boxes.is_empty(), "damage remains during recovery in {room}");
                recovered = true;
                break;
            } else if graph_warning && warned.is_some() {
                panic!("the mark disappeared during the warning in {room}");
            }
        }
        assert!(
            recovered,
            "the sweep did not warn, strike and recover in {room}"
        );
        assert_eq!(
            sim.world()
                .get::<BodyHealth>(player)
                .unwrap()
                .health
                .current,
            hp - 1
        );
        if room == PRACTICE {
            check_volley(&mut sim, boss);
        }
    }
}

#[test]
fn the_gram_reaper_enrage_leaves_the_marked_gap_safe() {
    let (mut sim, boss, player) = set_up(PRACTICE);
    let mut second_phase = false;
    let mut projection = None;
    let mut saw_projection_strike = false;
    let mut enrage = false;
    let mut safe = None;
    let mut saw_strike = false;
    for _ in 0..900 {
        sim.step(AgentAction::default());
        check_faults(&sim);
        let marks = marks(&sim, boss);
        if !second_phase && !marks.is_empty() {
            sim.world_mut()
                .get_mut::<BodyHealth>(boss)
                .unwrap()
                .health
                .current = 26;
            second_phase = true;
        }
        if let [(0, column, active)] = marks.as_slice() {
            if column.half.y > column.half.x && !enrage {
                if *active {
                    assert_eq!(projection, Some(*column));
                    saw_projection_strike = true;
                } else if projection.is_none() {
                    projection = Some(*column);
                    let floor = column.center.y + column.half.y;
                    let half = sim.world().get::<BodyKinematics>(player).unwrap().size().y * 0.5;
                    sim.teleport_player((column.center.x + column.half.x + 96.0, floor - half));
                } else {
                    assert_eq!(projection, Some(*column));
                }
            }
        }
        if saw_projection_strike && !enrage && marks.is_empty() {
            sim.world_mut()
                .get_mut::<BodyHealth>(boss)
                .unwrap()
                .health
                .current = 8;
            enrage = true;
        }
        if let [(0, left, false), (1, right, false)] = marks.as_slice() {
            let right_edge = left.center.x + left.half.x;
            let left_edge = right.center.x - right.half.x;
            let height = sim.world().get::<BodyKinematics>(player).unwrap().size().y;
            assert!(left_edge - right_edge > 2.0 * height);
            if safe.is_none() {
                let floor = left.center.y + left.half.y;
                sim.teleport_player(((right_edge + left_edge) * 0.5, floor - height * 0.5));
                safe = Some(
                    sim.world()
                        .get::<BodyHealth>(player)
                        .unwrap()
                        .health
                        .current,
                );
            }
        }
        if let [(0, left, true), (1, right, true)] = marks.as_slice() {
            let boxes = hitboxes(&mut sim, boss);
            assert_eq!(boxes.len(), 2);
            assert!(boxes.iter().any(|(_, region)| region == left));
            assert!(boxes.iter().any(|(_, region)| region == right));
            saw_strike = true;
        }
        if saw_strike && marks.is_empty() {
            assert_eq!(
                Some(
                    sim.world()
                        .get::<BodyHealth>(player)
                        .unwrap()
                        .health
                        .current
                ),
                safe
            );
            assert!(hitboxes(&mut sim, boss).is_empty());
            return;
        }
    }
    panic!("the Reaper did not finish its enrage opener");
}
