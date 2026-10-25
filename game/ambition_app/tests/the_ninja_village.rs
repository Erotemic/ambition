//! The Ninja Village (gate address SILENT): the gate shrine, the street, the
//! dojo and the cedar forest (`tools/ambition_ldtk_tools/specs/ninja_village/`).
//!
//! The shipped session puts the player at the gate shrine, where the gate
//! arrives. From there the player walks by keys (`common::walk`): the low road
//! east through the street to the forest and the Shadow Dojo's door, the
//! branches up, and the high road back west over the roofs to the shrine's
//! terrace. In the dojo it fights the rank exam with Attack presses and
//! reports to the teacher. It talks to each resident on the way. No step puts
//! the body anywhere after the arrival.

use ambition_platformer2d::engine_core as ae;
use bevy::prelude::App;

use crate::common::walk::{fight, Keys};
use crate::neighbor_prefetch_prepares_rooms::alice as primary_body;
use crate::the_note_travels_from_alice_to_bob::flag;
use crate::the_pirate_port::{a_session_in, go, go_to, quest_completed, talk};

const SHRINE: &str = "ninja_village_gate_shrine";
const STREET: &str = "ninja_village_street";
const DOJO: &str = "ninja_village_dojo";
const FOREST: &str = "ninja_village_forest";
const CLAN: &str = "ninja_dojo";
const EXAM: &str = "ninja_village.rank_exam";

fn exam_cleared(world: &bevy::prelude::World) -> bool {
    world.resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>().data().encounter(EXAM)
        == ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared
}

fn feet_y(app: &mut App) -> f32 {
    let body = primary_body(app);
    app.world().get::<ae::BodyKinematics>(body).expect("a body").pos.y
}

/// ⭐ THE VILLAGE IS A PLACE TO WALK, TRAIN AND TALK. Its residents talk; the
/// dojo's rank exam is fought and the teacher awards the rank (the quest is
/// done, the rival declares herself); the forest's door goes to the Shadow
/// Dojo and back; the branches and the roofs are the road home.
#[test]
fn the_ninja_village_is_walked_round_its_loop_and_its_exam_is_passed() {
    let mut app = a_session_in(SHRINE);

    talk(&mut app, "npc_village_sato", "Do many come through the gate?");
    talk(&mut app, "npc_village_kenji", "Is the gate always like this?");

    // The low road, and the dojo on the way.
    go(&mut app, STREET, Some("shrine_east_low"));
    talk(&mut app, "npc_village_daichi", "What is the rank exam?");
    talk(&mut app, "npc_village_ume", "Why do people run on the roofs?");
    talk(&mut app, "npc_gator_geisha", "What brings an alligator to a mountain village?");
    go(&mut app, DOJO, None);
    talk(&mut app, "npc_village_hayashi", "How does the exam work?");
    talk(&mut app, "npc_village_ren", "How do I become your rival?");

    // The exam: walk onto the exam floor, and fight until the save has it.
    assert!(!exam_cleared(app.world()), "control: the exam is not passed before the fight");
    let floor = ae::Vec2::new(720.0, 480.0);
    go_to(&mut app, floor, "the exam floor");
    {
        use ambition_platformer2d::combat::components::EncounterMob;
        let frames = fight(
            &mut Keys::new(&mut app),
            |world, body| world.get::<EncounterMob>(body).is_some(),
            exam_cleared,
            floor,
            60 * 180,
        )
        .unwrap_or_else(|e| panic!("the rank exam: {e}"));
        eprintln!("[village] the rank exam was passed in {frames} frames");
    }
    for _ in 0..60 {
        app.update();
    }
    talk(&mut app, "npc_village_hayashi", "Accept the rank.");
    assert!(flag(&app, "ninja_village_rank_awarded"), "the teacher awards the rank");
    assert!(quest_completed(&app, "ninja_village_rank_exam"), "the quest is done");
    talk(&mut app, "npc_village_ren", "Sign the rival form.");

    go(&mut app, STREET, None);
    go(&mut app, FOREST, Some("street_east_low"));

    // The path to the Shadow Clan, and back.
    go(&mut app, CLAN, None);
    go(&mut app, FOREST, Some("ninja_dojo_village_door"));

    // The high road home: up the branches, over the roofs, down the stair.
    go(&mut app, STREET, Some("forest_west_high"));
    assert!(feet_y(&mut app) < 500.0, "the branches arrive on the roofs");
    go(&mut app, SHRINE, Some("street_west_high"));
    assert!(feet_y(&mut app) < 500.0, "the roofs arrive on the torii terrace");
    go_to(&mut app, ae::Vec2::new(640.0, 680.0), "the shrine's floor, by the stair");
    assert!(feet_y(&mut app) > 600.0, "the stair goes down to the shrine's floor");
}
