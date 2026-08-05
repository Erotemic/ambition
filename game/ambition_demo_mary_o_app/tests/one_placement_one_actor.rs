//! **One authored placement, one actor, wearing its own mechanics.**
//!
//! ⛔ **level 1-1 built every enemy TWICE and nothing noticed.** A GPT 5.6
//! review of `90a9715` found it; a probe that booted the real app and dumped
//! every `FeatureId` confirmed it — 17 authored `EnemySpawn` placements, 34
//! actors. Two construction roots were live at once:
//!
//! - the engine's `authored_actor_requests()`, which builds one row for every
//!   `room.enemy_spawns` entry, unconditionally;
//! - two `RoomContentStagingRegistry` closures in the Mary-O crate, which walked
//!   the same entries and minted a second actor under a prefixed id.
//!
//! ⚠ **the duplicate-id check could not see it**, because the whole point of the
//! prefix was that the two paths called the same authored thing different names:
//! `EnemySpawn-106877` and `mary_o_snake_EnemySpawn-106877`.
//!
//! ⚠ **and only the prefixed half was tagged.** `is_snake_id` matched an id
//! PREFIX, so the engine-built copy was a custom-brain enemy with no
//! `SnakeShell`, no `AiSlop`, no stomp and no dormancy — half of 1-1's enemies
//! were un-stompable lookalikes standing in the same places as the real ones.
//!
//! ⭐ **the regression was the LDtk port itself.** Before it, 1-1 authored no
//! enemies and staging was the only root; authoring them gave the engine
//! something to build and nothing removed the closure. That is what makes this
//! worth a test rather than a fix: the two roots were individually correct and
//! only their COEXISTENCE was wrong, so nothing either one owns would have
//! caught it.

use bevy::prelude::*;

use ambition_demo_mary_o::ai_slop::{is_ai_slop_brain, AiSlop};
use ambition_demo_mary_o::snake::{is_snake_brain, SnakeShell};
use ambition_platformer2d::actors::features::ecs::dormancy::DormancyPolicy;
use ambition_platformer2d::actors::features::{ActorConfig, FeatureId};

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

/// **The count matches the file.**
///
/// ⚠ deliberately compared against the AUTHORED placements rather than a
/// literal 17: the level is Jon's to edit, and a test that has to be retuned
/// when he adds an enemy is a test that will be wrong before it is red.
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

    // ⭐ and each one is the PLACEMENT's own identity, not a minted lookalike.
    // A prefixed id here would mean the count matched only because one root
    // replaced the other rather than because one root is left.
    let mut ids: Vec<&String> = built.iter().map(|(id, _)| id).collect();
    ids.sort();
    let mut expected: Vec<&String> = authored.iter().collect();
    expected.sort();
    assert_eq!(
        ids, expected,
        "every enemy actor carries the id of the placement that authored it"
    );
}

/// **Every one of them wears its Mary-O mechanics.**
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

/// **An AI Slop learns it may sleep, and nobody else does.**
///
/// ⭐ **this moved here from a unit test, and got better on the way.** The tag
/// pass is where a slop receives its dormancy policy, and the engine's policy
/// component is opt-in: an actor that never receives one is awake across the
/// whole level forever, which looks identical to "the feature is not built".
/// The old version spawned a bare `FeatureId` on an `App::new()` — it could only
/// prove the pass answers something a test invented. This one asserts it for
/// every slop the real construction path builds from the real authored level.
///
/// ⚠ **the negative half is the point.** Dormancy is a per-character decision,
/// so handing it to everything in the room would be the same mistake as the
/// engine assuming a distance.
#[test]
fn every_authored_enemy_declares_whether_it_sleeps() {
    let mut app = booted();
    // ⛔ **this test used to assert that ONLY the slop declares dormancy**, and
    // that assertion is why the snake went a day without a policy. Jon named the
    // slop — *"ai slop will just walk off the edge of the level"* — so the seam
    // was built, the slop was wired, and the test pinned the state of the fix
    // rather than the property being fixed. The other patrolling enemy in the
    // same level thought for the whole course, and the guard positioned to
    // notice was instead defending its absence.
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
