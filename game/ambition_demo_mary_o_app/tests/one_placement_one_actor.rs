//! Each authored Mary-O enemy placement must build exactly one actor and receive
//! the mechanics implied by its authored brain archetype.

use bevy::prelude::*;

use ambition_demo_mary_o::ai_slop::{is_ai_slop_brain, AiSlop};
use ambition_demo_mary_o::snake::{is_snake_brain, SnakeShell};
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

/// Every authored enemy is a hostile her dormancy rule reaches, and an enemy
/// far from her is asleep.
///
/// Which bodies may sleep is the engine's rule (`dormancy::wake_radius`: a free
/// hostile). This asserts the composition that the real construction path
/// builds from the real authored level: Mary-O states a rule for her rooms,
/// each authored enemy is a body that rule reaches, and the far ones sleep.
#[test]
fn every_authored_enemy_sleeps_when_she_is_far() {
    use ambition_platformer2d::actors::features::ecs::dormancy::{
        wake_radius, Dormant, DormancyRule,
    };
    use ambition_platformer2d::characters::actor::limb::Limb;
    use ambition_platformer2d::characters::control::DrivingParticipant;
    use ambition_platformer2d::combat::components::{ActorFaction, EncounterMob};
    use ambition_platformer2d::combat::scoped_rules::DeclaredRules;
    use ambition_platformer2d::engine_core::BodyKinematics;
    use ambition_platformer2d::mount::Mountable;

    let mut app = booted();
    // Her rooms carry her mode; the rule declared for it governs them.
    let rule = app
        .world()
        .get_resource::<DeclaredRules<DormancyRule>>()
        .and_then(|rules| rules.governing(Some(ambition_demo_mary_o::MARY_O_MODE)))
        .expect("Mary-O states a dormancy rule for her rooms");
    assert_eq!(rule.hostile_wake_radius, ambition_demo_mary_o::MARY_O_WAKE_RADIUS);

    let mut eyes = app
        .world_mut()
        .query_filtered::<&BodyKinematics, With<DrivingParticipant>>();
    let eyes: Vec<Vec2> = eyes.iter(app.world()).map(|body| body.pos).collect();
    assert!(!eyes.is_empty(), "she is the observer; with none, nothing sleeps");

    let mut q = app.world_mut().query::<(
        &FeatureId,
        &ActorConfig,
        &ActorFaction,
        Has<EncounterMob>,
        Has<Mountable>,
        Has<Limb>,
        &BodyKinematics,
        Has<Dormant>,
    )>();
    let mut enemies = 0usize;
    let mut asleep = 0usize;
    let mut unreached = Vec::new();
    let mut wrong = Vec::new();
    for (id, config, faction, is_mob, is_mount, is_limb, body, is_dormant) in q.iter(app.world()) {
        if !(is_snake_brain(&config.brain) || is_ai_slop_brain(&config.brain)) {
            continue;
        }
        enemies += 1;
        let Some(radius) = wake_radius(Some(&rule), *faction, is_mob, is_mount || is_limb) else {
            unreached.push(format!("{} ({faction:?})", id.0));
            continue;
        };
        let nearest = eyes
            .iter()
            .map(|eye| eye.distance(body.pos))
            .fold(f32::INFINITY, f32::min);
        // The pass decided before this tick's movement, so a body within one
        // tile (32 units) of the radius can read either way.
        if (nearest - radius).abs() < 32.0 {
            continue;
        }
        let far = nearest > radius;
        asleep += usize::from(is_dormant);
        if far != is_dormant {
            wrong.push(format!("{} at {nearest:.0} dormant={is_dormant}", id.0));
        }
    }
    assert!(enemies > 0, "1-1 authors enemies; if it stops, this test checks nothing");
    assert!(
        unreached.is_empty(),
        "these authored enemies are not hostiles her rule reaches, so they \
         think for the whole level and can walk off a ledge before anyone \
         arrives: {unreached:?}"
    );
    assert!(
        wrong.is_empty(),
        "an enemy is asleep beside her or awake far from her: {wrong:?}"
    );
    assert!(
        asleep > 0,
        "1-1 is longer than her wake radius, so some enemy must be asleep at \
         the start; none is"
    );
}
