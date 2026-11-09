//! Each authored Mary-O enemy placement must build exactly one actor and receive
//! the mechanics implied by its authored brain archetype.

use bevy::prelude::*;

use ambition_demo_mary_o::ai_slop::is_ai_slop_brain;
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

/// Every snake wears its shell state.
///
/// This is the half that actually bit: the count being right is worth nothing if
/// the surviving actors are copies without it. `SnakeShell` comes from the
/// Solid Snake character and from nothing else, so its presence on every
/// snake-brained actor says the placements build that character. (An AI Slop
/// carries no state of its own: its stomp reads the authored brain on the
/// body.)
#[test]
fn every_snake_wears_the_shell_state_its_brain_promises() {
    let mut app = booted();
    let built = mary_o_enemies(&mut app);

    let mut shells = app.world_mut().query::<&SnakeShell>();
    let tagged_snakes = shells.iter(app.world()).count();

    let want_snakes = built.iter().filter(|(_, snake)| *snake).count();
    assert!(
        want_snakes > 0,
        "1-1 has snakes; without one this test measures nothing"
    );
    assert_eq!(
        tagged_snakes, want_snakes,
        "{want_snakes} actors have a snake brain but {tagged_snakes} carry SnakeShell — \
         an untagged snake is an enemy that cannot be stomped and does not report it"
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
    use ambition_platformer2d::actors::body::dormancy::wake_radius;
use ambition_platformer2d::actors::body::dormancy::Dormant;
    use ambition_platformer2d::combat::dormancy_rule::DormancyRule;
    use ambition_platformer2d::characters::actor::limb::Limb;
    use ambition_platformer2d::characters::control::DrivingParticipant;
    use ambition_platformer2d::combat::components::EncounterMob;
    use ambition_platformer2d::actor::ActorFaction;
    use ambition_platformer2d::combat::scoped_rules::DeclaredRules;
    use ambition_platformer2d::engine_core::BodyKinematics;
    use ambition_platformer2d::mount::Mountable;

    let mut app = booted();
    // Her rooms carry her mode; the rule declared for it governs them.
    let rule = app
        .world()
        .get_resource::<DeclaredRules<DormancyRule>>()
        .and_then(|rules| rules.governing(ambition_platformer2d::combat::scoped_rules::ActiveRoom::live(Some(ambition_demo_mary_o::MARY_O_MODE))).copied())
        .expect("Mary-O states a dormancy rule for her rooms");
    assert_eq!(rule.hostile_wake_radius, ambition_demo_mary_o::pack::room_rules().dormancy.expect("her rooms state a wake radius").hostile_wake_radius);

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

/// A Solid Snake has its shell state the first time the simulation sees it.
///
/// The sampler runs in every sim tick before `WorldPrepSet::AfterIntegrate`,
/// and records, for each Solid Snake body, whether it had `SnakeShell` the
/// first time the sampler saw it. A pass that adds the shell to a built body
/// runs later than the build, so the first sample of every snake is without
/// it, on whichever tick the room built the snake.
///
/// ⛔ THE SHELL WAS ADDED ON A LATER PASS. `tag_mary_o_snakes` gave a snake its
/// shell in `AfterIntegrate`, keyed on the brain, so a snake existed for part
/// of a tick without the state its rules read. Her cast now states that a
/// Solid Snake carries it, and the body is built with it.
#[test]
fn a_snake_is_built_with_its_shell_state() {
    use ambition_platformer2d::platformer::schedule::{SimScheduleExt, WorldPrepSet};

    #[derive(Resource, Default)]
    struct FirstSeen(Vec<(Entity, bool)>);
    fn sample(
        bodies: Query<(Entity, &ambition_platformer2d::characters::actor::WornCharacter, Has<SnakeShell>)>,
        mut seen: ResMut<FirstSeen>,
    ) {
        for (body, worn, shelled) in &bodies {
            if worn.id() == ambition_demo_mary_o::snake::SNAKE_SHEET_TARGET
                && !seen.0.iter().any(|(first, _)| *first == body)
            {
                seen.0.push((body, shelled));
            }
        }
    }

    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.init_resource::<FirstSeen>();
    let sim = app.sim_schedule();
    app.add_systems(sim, sample.before(WorldPrepSet::AfterIntegrate));
    for _ in 0..400 {
        app.update();
    }
    let seen = &app.world().resource::<FirstSeen>().0;
    assert!(!seen.is_empty(), "1-1 has snakes; without one this test measures nothing");
    let bare = seen.iter().filter(|(_, shelled)| !shelled).count();
    assert_eq!(
        bare,
        0,
        "{bare} of {} Solid Snakes were first seen without their shell state",
        seen.len()
    );
}

#[test]
fn two_walkers_meet_without_overlap_and_turn_away() {
    use ambition_platformer2d::characters::actor::WornCharacter;
    use ambition_platformer2d::engine_core::BodyKinematics;
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)));
    for _ in 0..120 { app.update(); }
    let mut q = app.world_mut().query::<(Entity, &WornCharacter)>();
    let mut slops: Vec<_> = q.iter(app.world()).filter(|(_, worn)| worn.id() == "ai_slop").map(|(entity, _)| entity).collect();
    slops.sort();
    assert!(slops.len() >= 2, "the real level must supply the pair");
    let pair = [slops[0], slops[1]];
    for (entity, x, facing) in [(pair[0], 350.0, 1.0), (pair[1], 410.0, -1.0)] {
        let mut kin = app.world_mut().get_mut::<BodyKinematics>(entity).unwrap();
        kin.pos = ambition_platformer2d::engine_core::Vec2::new(x, 416.0 - kin.size().y * 0.5);
        kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
        kin.facing = facing;
    }
    let mut touched = false;
    let mut turned = false;
    for _ in 0..120 {
        app.update();
        let left = app.world().get::<BodyKinematics>(pair[0]).unwrap();
        let right = app.world().get::<BodyKinematics>(pair[1]).unwrap();
        let gap = right.aabb().min.x - left.aabb().max.x;
        assert!(gap >= -0.001, "the pair overlaps by {}", -gap);
        touched |= gap < 0.01;
        turned |= left.facing < 0.0 && right.facing > 0.0;
        if touched && turned { break; }
    }
    assert!(touched, "the pair never reached contact");
    assert!(turned, "the pair did not turn away");
}

#[test]
fn authored_enemy_starts_do_not_overlap_their_constructed_bodies() {
    use ambition_platformer2d::engine_core::{Aabb, AabbExt};
    use ambition_platformer2d::platformer::body::SpawnBaseline;
    let mut overlaps = Vec::new();
    for room in ambition_demo_mary_o::authored_levels() {
        if room.enemy_spawns.is_empty() { continue; }
        let mut app = ambition_demo_mary_o_app::build_demo_app();
        app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(room.id.clone()));
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)));
        let mut ready = false;
        for _ in 0..120 {
            app.update();
            let mut q = app.world_mut().query::<(&FeatureId, &SpawnBaseline, &ActorConfig)>();
            if q.iter(app.world()).filter(|(id, ..)| room.enemy_spawns.iter().any(|spawn| spawn.id == id.0)).count() == room.enemy_spawns.len() {
                ready = true;
                break;
            }
        }
        assert!(ready, "{} did not construct all authored enemies", room.id);
        let mut q = app.world_mut().query::<(&FeatureId, &SpawnBaseline, &ActorConfig)>();
        let starts: Vec<_> = q.iter(app.world())
            .filter(|(id, _, _)| room.enemy_spawns.iter().any(|spawn| spawn.id == id.0))
            .map(|(id, spawn, _)| (id.0.clone(), Aabb::new(spawn.pos, spawn.size * 0.5)))
            .collect();
        for (i, (left_id, left)) in starts.iter().enumerate() {
            for (right_id, right) in &starts[i + 1..] {
                if left.strict_intersects(*right) {
                    overlaps.push(format!("{}: {left_id} {left:?} overlaps {right_id} {right:?}", room.id));
                }
            }
        }
    }
    assert!(overlaps.is_empty(), "{}", overlaps.join("\n"));
}

#[test]
fn a_walker_hurts_mary_o_without_turning_away() {
    use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
    use ambition_platformer2d::engine_core::BodyKinematics;
    use ambition_platformer2d::platformer::markers::PrimaryPlayer;
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)));
    for _ in 0..120 { app.update(); }
    let mut q = app.world_mut().query_filtered::<Entity, With<PrimaryPlayer>>();
    let player = q.single(app.world()).unwrap();
    let mut q = app.world_mut().query::<(Entity, &WornCharacter, &BodyKinematics)>();
    let enemy = q.iter(app.world()).filter(|(_, worn, _)| worn.id() == "ai_slop")
        .min_by(|a, b| a.2.pos.x.total_cmp(&b.2.pos.x)).unwrap().0;
    retain_contact_walkers(&mut app, &[enemy]);
    let player_size = app.world().get::<BodyKinematics>(player).unwrap().size();
    let enemy_size = app.world().get::<BodyKinematics>(enemy).unwrap().size();
    for (entity, x, facing) in [(player, 350.0, 1.0), (enemy, 350.0 + (player_size.x + enemy_size.x) * 0.5, -1.0)] {
        let mut kin = app.world_mut().get_mut::<BodyKinematics>(entity).unwrap();
        kin.pos = ambition_platformer2d::engine_core::Vec2::new(x, 416.0 - kin.size().y * 0.5);
        kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
        kin.facing = facing;
    }
    let mut others = app.world_mut().query::<(Entity, &mut ActorConfig)>();
    for (entity, mut config) in others.iter_mut(app.world_mut()) {
        if entity != enemy { config.tuning.body_contact_damage = false; }
    }
    let before = app.world().get::<BodyHealth>(player).unwrap().current();
    assert!(before > 0);
    for _ in 0..4 { app.update(); }
    assert!(app.world().get::<BodyKinematics>(enemy).unwrap().facing < 0.0, "Mary-O must not turn a walker away");
    assert!(app.world().get::<BodyHealth>(player).unwrap().current() < before, "the walker crossed Mary-O without dealing its contact hit");
}

#[test]
fn walking_into_a_boxed_snake_still_kicks_it() {
    use ambition_platformer2d::characters::actor::WornCharacter;
    use ambition_platformer2d::engine_core::BodyKinematics;
    use ambition_platformer2d::platformer::markers::PrimaryPlayer;
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(ambition_demo_mary_o::test_course::TEST_COURSE_ROOM_ID.into()));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)));
    for _ in 0..120 { app.update(); }
    let mut q = app.world_mut().query_filtered::<Entity, With<PrimaryPlayer>>();
    let player = q.single(app.world()).unwrap();
    let mut q = app.world_mut().query::<(Entity, &WornCharacter)>();
    let snake = q.iter(app.world()).find(|(_, worn)| worn.id() == "solid_snake").unwrap().0;
    *app.world_mut().get_mut::<SnakeShell>(snake).unwrap() = SnakeShell::Boxed(100.0);
    app.world_mut().entity_mut(snake).insert(
        ambition_platformer2d::characters::control::ControlHolds::only(
            ambition_platformer2d::characters::control::ControlHold::Sequence));
    for _ in 0..4 { app.update(); }
    for (entity, x) in [(player, 350.0), (snake, 420.0)] {
        let mut kin = app.world_mut().get_mut::<BodyKinematics>(entity).unwrap();
        kin.pos = ambition_platformer2d::engine_core::Vec2::new(x, 416.0 - kin.size().y * 0.5);
        kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
    }
    app.world_mut().resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>().0 = ambition_platformer2d::input::ControlFrame { axis_x: 1.0, ..Default::default() };
    let mut kicked = false;
    for _ in 0..90 {
        app.update();
        kicked |= matches!(app.world().get::<SnakeShell>(snake), Some(SnakeShell::Sliding { .. }));
        if kicked { break; }
    }
    app.world().resource::<ambition_platformer2d::scripted_input::ScriptedControlsObserved>()
        .assert_the_script_reached_the_simulation();
    assert!(kicked, "solid-body contact prevented the side kick: player={:?}, snake={:?}, shell={:?}",
        app.world().get::<BodyKinematics>(player), app.world().get::<BodyKinematics>(snake),
        app.world().get::<SnakeShell>(snake));
}

fn contact_course() -> (App, Entity, Vec<Entity>) {
    use ambition_platformer2d::characters::actor::WornCharacter;
    use ambition_platformer2d::platformer::markers::PrimaryPlayer;
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..120 { app.update(); }
    let player = app.world_mut().query_filtered::<Entity, With<PrimaryPlayer>>().single(app.world()).unwrap();
    let mut walkers: Vec<_> = app.world_mut().query::<(Entity, &WornCharacter)>().iter(app.world())
        .filter(|(_, worn)| matches!(worn.id(), "ai_slop" | "solid_snake"))
        .map(|(entity, _)| entity).collect();
    walkers.sort();
    assert!(walkers.len() >= 2);
    (app, player, walkers)
}

fn retain_contact_walkers(app: &mut App, keep: &[Entity]) {
    let removed: Vec<_> = app.world_mut().query::<(Entity, &ambition_platformer2d::characters::actor::WornCharacter)>()
        .iter(app.world()).filter(|(entity, worn)| matches!(worn.id(), "ai_slop" | "solid_snake") && !keep.contains(entity))
        .map(|(entity, _)| entity).collect();
    for entity in removed { app.world_mut().despawn(entity); }
}

fn place_contact_body(app: &mut App, body: Entity, x: f32, facing: f32) {
    let mut kin = app.world_mut().get_mut::<ambition_platformer2d::engine_core::BodyKinematics>(body).unwrap();
    kin.pos = ambition_platformer2d::engine_core::Vec2::new(x, 416.0 - kin.size().y * 0.5);
    kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
    kin.facing = facing;
    if let Some(mut ground) = app.world_mut().get_mut::<ambition_platformer2d::engine_core::BodyGroundState>(body) {
        ground.on_ground = true;
    }
}

#[test]
fn a_dead_walker_does_not_block_or_turn_a_live_walker() {
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::engine_core::BodyKinematics;
    let (mut app, _, walkers) = contact_course();
    let slops: Vec<_> = walkers.into_iter().filter(|body| app.world().get::<ambition_platformer2d::characters::actor::WornCharacter>(*body).unwrap().id() == "ai_slop").collect();
    let (live, dead) = (slops[0], slops[1]);
    retain_contact_walkers(&mut app, &[live, dead]);
    place_contact_body(&mut app, live, 350.0, 1.0);
    let gap = (app.world().get::<BodyKinematics>(live).unwrap().size().x + app.world().get::<BodyKinematics>(dead).unwrap().size().x) * 0.5 + 8.0;
    place_contact_body(&mut app, dead, 350.0 + gap, -1.0);
    let far_edge = app.world().get::<BodyKinematics>(dead).unwrap().aabb().max.x;
    app.world_mut().get_mut::<BodyHealth>(dead).unwrap().damage(i32::MAX);
    assert!(!app.world().get::<BodyHealth>(dead).unwrap().alive());
    for _ in 0..180 {
        app.update();
        let kin = app.world().get::<BodyKinematics>(live).unwrap();
        assert!(kin.facing > 0.0, "a corpse turned the living walker away");
        if kin.aabb().min.x > far_edge { return; }
    }
    panic!("the living walker did not pass the corpse");
}

#[test]
fn cosmic_and_quasar_contact_kill_walkers_that_approach_or_walk_away() {
    use ambition_platformer2d::actors::body::empowerment::Empowered;
    use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
    use ambition_platformer2d::engine_core::BodyKinematics;
    use ambition_platformer2d::input::ControlFrame;
    for (cosmic, character, facing) in [(true, "ai_slop", -1.0), (true, "solid_snake", 1.0), (false, "ai_slop", 1.0), (false, "solid_snake", -1.0)] {
        let (mut app, player, walkers) = contact_course();
        let enemy = *walkers.iter().find(|body| app.world().get::<WornCharacter>(**body).unwrap().id() == character).unwrap();
        retain_contact_walkers(&mut app, &[enemy]);
        if cosmic {
            app.world_mut().entity_mut(player).insert(WornCharacter::new(ambition_demo_mary_o::cosmic::COSMIC_CHARACTER_ID));
        } else {
            app.world_mut().entity_mut(player).insert(Empowered::for_seconds(ambition_demo_mary_o::star::COSMIC_QUASAR_SUPER_STATE, 10.0));
        }
        app.update();
        place_contact_body(&mut app, player, 350.0, 1.0);
        place_contact_body(&mut app, enemy, 400.0, facing);
        let mut defeated = false;
        for _ in 0..60 {
            app.world_mut().resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>().0 = ControlFrame { axis_x: 1.0, ..Default::default() };
            app.update();
            if app.world().get::<BodyHealth>(enemy).is_none_or(|health| !health.alive()) {
                defeated = true;
                break;
            }
            assert_eq!(app.world().get::<BodyKinematics>(enemy).unwrap().facing.signum(), facing, "Mary-O turned {character} away: cosmic={cosmic}");
        }
        assert!(defeated, "contact did not defeat {character}: cosmic={cosmic}, facing={facing}");
        assert!(app.world().get::<BodyHealth>(player).unwrap().alive());
    }
}
