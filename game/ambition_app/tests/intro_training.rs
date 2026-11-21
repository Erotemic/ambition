//! The optional lesson uses the shared movement and hit paths.

#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_room_options};
use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    characters::actor::{BodyCombat, BodyHealth, CharacterForm},
    cutscene::ActiveCutscene,
    engine_core as ae,
    persistence::save::AmbitionGameSave,
    platformer::sim_id::SimId,
};

fn workshop(training: bool, rewind: usize) -> Platformer2dSimHarness {
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    save.set_flag("intro_training_active", training);
    let options = fixed_60hz_room_options("intro_wake_room").with_save(save);
    let mut sim = Platformer2dSimHarness::new_with_options(
        options.with_sync_test_rollback_settings(rewind, 12),
    )
    .expect("the workshop builds");
    for _ in 0..30 {
        sim.step(base());
    }
    sim
}

fn scene(sim: &Platformer2dSimHarness) -> Option<&str> {
    sim.world().resource::<ActiveCutscene>().iter().next().map(|playing| playing.script_id())
}

fn body(sim: &mut Platformer2dSimHarness, form: &str) -> (ae::BodyKinematics, BodyHealth) {
    let world = sim.world_mut();
    world
        .query::<(&SimId, &CharacterForm, &ae::BodyKinematics, &BodyHealth)>()
        .iter(world)
        .filter(|(_, worn, ..)| worn.id() == form)
        .min_by(|(left, ..), (right, ..)| left.cmp(right))
        .map(|(_, _, kin, health)| (*kin, *health))
        .expect("the body is present")
}

fn finish_scene(sim: &mut Platformer2dSimHarness) {
    for tick in 0..240 {
        if scene(sim).is_none() {
            return;
        }
        sim.step(AgentAction {
            confirm: tick % 10 == 0,
            ..base()
        });
    }
    panic!("the scene did not finish: {:?}", scene(sim));
}

#[test]
fn the_lesson_waits_for_a_landing_and_a_real_hit_under_rewinds() {
    for rewind in [0, 4] {
        let mut sim = workshop(true, rewind);
        for attempt in 0..2 {
            sim.world_mut()
                .resource_mut::<AmbitionGameSave>()
                .data_mut()
                .set_flag("intro_training_active", true);
            sim.teleport_player((660.0, 328.0));
            sim.rebase_rollback_history().unwrap();
            for _ in 0..90 {
                sim.step(base());
            }
            assert!(
                scene(&sim).is_none(),
                "attempt {attempt}: waiting does not complete the exercise"
            );
            for tick in 0..150 {
                sim.step(AgentAction {
                    move_x: if tick < 21 { -1.0 } else { 0.0 },
                    jump: tick == 0,
                    jump_held: tick < 25,
                    jump_released: tick == 25,
                    ..base()
                });
                if scene(&sim) == Some("intro_training_combat") {
                    break;
                }
            }
            assert_eq!(
                scene(&sim),
                Some("intro_training_combat"),
                "a real jump lands on the block"
            );
            finish_scene(&mut sim);
            // A swing facing away from the bag cannot pass the combat exercise.
            sim.step(AgentAction {
                move_x: 1.0,
                ..base()
            });
            for tick in 0..60 {
                sim.step(AgentAction {
                    attack: tick == 0,
                    ..base()
                });
            }
            assert!(scene(&sim).is_none(), "a missed swing does not pass combat");
            let mut hit_landed = false;
            for tick in 0..180 {
                sim.step(AgentAction {
                    move_x: if tick < 17 { -1.0 } else { 0.0 },
                    attack: tick >= 17 && tick % 20 == 0,
                    ..base()
                });
                let world = sim.world_mut();
                hit_landed |= world
                    .query::<(&CharacterForm, &BodyCombat)>()
                    .iter(world)
                    .any(|(form, combat)| {
                        form.id() == "practice_sandbag" && combat.recently_struck()
                    });
                if scene(&sim) == Some("intro_training_finished") {
                    break;
                }
            }
            let bag = body(&mut sim, "practice_sandbag");
            let world = sim.world_mut();
            let learner = *world.query_filtered::<&ae::BodyKinematics, ambition_platformer2d::platformer::markers::PrimaryPlayerOnly>()
            .single(world).unwrap();
            assert!(hit_landed, "rewind {rewind}: the shared hit path hits the bag; bag {bag:?}, learner {learner:?}");
            assert_eq!(scene(&sim), Some("intro_training_finished"));
            finish_scene(&mut sim);
            for _ in 0..15 {
                sim.step(base());
            }
            let save = sim.world().resource::<AmbitionGameSave>();
            assert!(save.data().flag("intro_training_complete"));
            assert!(
                !save.data().flag("intro_raiders_coming"),
                "practice does not call the raid"
            );
            sim.rollback_health().unwrap();
            assert_eq!(
                sim.world()
                    .resource::<ambition_platformer2d::extension::ExtensionFaults>()
                    .total,
                0
            );
        }
    }
}

#[test]
fn the_raiders_wait_in_view_until_the_last_line_is_dismissed() {
    let mut sim = workshop(false, 0);
    sim.teleport_player((1080.0, 328.0));
    sim.world_mut()
        .resource_mut::<AmbitionGameSave>()
        .data_mut()
        .set_flag("intro_raid_ready", true);
    sim.rebase_rollback_history().unwrap();
    let mut saw_raider = false;
    for tick in 0..1500 {
        let active = sim.world().resource::<ActiveCutscene>();
        let raider_speaks = active.iter().any(|playing| playing.presentation.speaker_character_id == "npc_lab_raider");
        if raider_speaks {
            let (creator, creator_health) = body(&mut sim, "npc_creator");
            assert!(
                (creator.pos.x - 1060.0).abs() < 24.0,
                "the Creator reaches the bench: {creator:?}"
            );
            let (raider, _) = body(&mut sim, "npc_lab_raider");
            assert!(
                raider.pos.x < 1320.0,
                "the raider has walked through the door: {raider:?}"
            );
            assert!(
                (raider.pos.x - creator.pos.x).abs() < 360.0,
                "both speakers fit in the shot"
            );
            for _ in 0..180 {
                sim.step(base());
            }
            assert_eq!(
                body(&mut sim, "npc_creator").1,
                creator_health,
                "reading does not expose the Creator to attacks"
            );
            assert!(
                sim.world()
                    .resource::<ActiveCutscene>()
                    .iter()
                    .any(|playing| playing.presentation.speaker_character_id == "npc_lab_raider"),
                "the raider stopped speaking"
            );
            saw_raider = true;
            break;
        }
        sim.step(AgentAction {
            confirm: tick % 20 == 0,
            ..base()
        });
    }
    assert!(saw_raider, "the arrival scene reaches its staged raider");
    finish_scene(&mut sim);
    assert!(sim
        .world()
        .resource::<AmbitionGameSave>()
        .data()
        .flag("wrong_list_clue_1"));
}
