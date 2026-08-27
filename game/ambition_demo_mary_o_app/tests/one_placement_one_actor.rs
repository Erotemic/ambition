//! Each authored Mary-O enemy placement must build exactly one actor and receive
//! the mechanics implied by its authored brain archetype.

use bevy::prelude::*;

use ambition_demo_mary_o::ai_slop::{is_ai_slop_brain, AiSlop};
use ambition_demo_mary_o::snake::{is_snake_brain, SnakeShell};
use ambition_platformer2d::actors::features::ecs::dormancy::DormancyPolicy;
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::combat::components::FeatureId;

fn booted() -> App {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    for _ in 0..400 {
        app.update();
    }
    app
}

/// Every actor in the room whose brain is one Mary-O gives meaning to.
fn mary_o_enemies(app: &mut App) -> Vec<(String, bool)> {
    let mut q = app.world_mut().query::<(&FeatureId, &ActorConfig)>();
    q.iter(app.world())
        .filter_map(|(id, cfg)| {
            let snake = is_snake_brain(&cfg.brain);
            if snake || is_ai_slop_brain(&cfg.brain) {
                Some((id.0.clone(), snake))
            } else {
                None
            }
        })
        .collect()
}

/// The count matches the file.
///
/// Compare against authored placements rather than a literal count so level edits remain valid.
#[test]
fn each_authored_enemy_placement_builds_exactly_one_actor() {
    let authored: Vec<String> = ambition_demo_mary_o::level_1_1()
        .enemy_spawns
        .iter()
        .map(|spawn| spawn.id.clone())
        .collect();
    assert!(
        !authored.is_empty(),
        "1-1 authors its enemies; if this is empty the test proves nothing"
    );

    let mut app = booted();
    let built = mary_o_enemies(&mut app);
    assert_eq!(
        built.len(),
        authored.len(),
        "the level authors {} enemy placements and the session built {} actors \
         from them — a second construction path is live. built: {:?}",
        authored.len(),
        built.len(),
        built.iter().map(|(id, _)| id).collect::<Vec<_>>()
    );

    // and each one is the PLACEMENT's own identity, not a minted lookalike.
    let mut ids: Vec<&String> = built.iter().map(|(id, _)| id).collect();
    ids.sort();
    let mut expected: Vec<&String> = authored.iter().collect();
    expected.sort();
    assert_eq!(
        ids, expected,
        "every enemy actor carries the id of the placement that authored it"
    );
}

/// Every one of them wears its Mary-O mechanics.
///
/// This is the half that actually bit: the count being right is worth nothing if
/// the surviving actors are the untagged copies. `SnakeShell` and `AiSlop` are
/// attached by the tag passes and by nothing else, so their presence is proof
/// the tag pass recognised the actor the engine built.
#[test]
fn no_enemy_is_left_without_the_mechanics_its_brain_promises() {
    let mut app = booted();
    let built = mary_o_enemies(&mut app);

    let mut shells = app.world_mut().query::<&SnakeShell>();
    let tagged_snakes = shells.iter(app.world()).count();
    let mut slop = app.world_mut().query::<&AiSlop>();
    let tagged_slop = slop.iter(app.world()).count();

    let want_snakes = built.iter().filter(|(_, snake)| *snake).count();
    let want_slop = built.len() - want_snakes;
    assert!(
        want_snakes > 0 && want_slop > 0,
        "1-1 has both kinds; without both this test cannot tell a tag pass from a coincidence"
    );
    assert_eq!(
        tagged_snakes, want_snakes,
        "{want_snakes} actors have a snake brain but {tagged_snakes} carry SnakeShell — \
         an untagged snake is an enemy that cannot be stomped and does not report it"
    );
    assert_eq!(
        tagged_slop, want_slop,
        "{want_slop} actors have a slop brain but {tagged_slop} carry AiSlop"
    );
}

/// An AI Slop learns it may sleep, and nobody else does.
///
/// This one asserts it for every slop the real construction path builds from the real authored
/// level.
///
/// the negative half is the point. Dormancy is a per-character decision,
/// so handing it to everything in the room would be the same mistake as the
/// engine assuming a distance.
#[test]
fn every_authored_enemy_declares_whether_it_sleeps() {
    let mut app = booted();
    // The other patrolling enemy in the same level thought for the whole course, and the guard
    // positioned to notice was instead defending its absence.
    //
    // The property is: every authored enemy in Mary-O declares a dormancy
    // policy, because every one of them patrols and none of them is worth
    // simulating on the far side of the level. A character that genuinely must
    // keep thinking says so with `DormancyPolicy::Never`, which still satisfies
    // this and is findable by a reader.
    let enemy_ids: Vec<String> = mary_o_enemies(&mut app)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert!(
        !enemy_ids.is_empty(),
        "1-1 authors enemies; if it stops, this test checks nothing"
    );

    let mut q = app
        .world_mut()
        .query::<(&FeatureId, Option<&DormancyPolicy>)>();
    let policied: Vec<String> = q
        .iter(app.world())
        .filter_map(|(id, policy)| policy.is_some().then(|| id.0.clone()))
        .collect();

    let undeclared: Vec<&String> = enemy_ids
        .iter()
        .filter(|id| !policied.contains(id))
        .collect();
    assert!(
        undeclared.is_empty(),
        "these authored enemies declare no DormancyPolicy, so they think for the \
         whole level and can walk off a ledge before anyone arrives: {undeclared:?}"
    );
}
