//! Headless movement + collision tests for the actor simulation: NPC gravity
//! settle / patrol / talk-stop / possession and enemy aerial / patrol / wall /
//! sideways-gravity / moving-platform-ride behaviour, plus archetype-tuning
//! invariants — all driven through the cluster scratch views without a renderer.

use super::*;

/// Build a peaceful actor (the unified cluster) with a patrol radius and a
/// player parked far outside the talk radius, plus the catalog Brain that
/// drives it. Peaceful actors are the SAME cluster as enemies now, so these
/// tests drive `ActorMut::update` (via `update_for_test`) with a frame the
/// catalog brain produced — exactly what `update_ecs_actors` does per tick.
fn world_with_patrolling_npc(
    patrol_radius: f32,
) -> (
    ae::World,
    super::ecs::actor_clusters::ActorClusterSeed,
    ambition_characters::brain::Brain,
    ae::BodyClusterScratch,
) {
    let world = ae::World::new(
        String::from("patrol_test"),
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(100.0, 100.0),
        vec![ae::Block::solid(
            String::from("floor"),
            ae::Vec2::new(0.0, 600.0),
            ae::Vec2::new(2000.0, 40.0),
        )],
    );
    let aabb = ae::Aabb::new(ae::Vec2::new(800.0, 540.0), ae::Vec2::new(11.0, 19.0));
    let id = String::from("patrol_kira");
    let interactable = ambition_interaction::Interactable::new(
        id.clone(),
        String::from("Talk"),
        aabb,
        ambition_interaction::InteractionKind::Npc {
            character_id: None,
            dialogue_id: Some(id.clone()),
            patrol_radius,
            patrol_path_id: None,
            brain_override: None,
        },
    );
    let (seed, _render) = super::ecs::actor_clusters::ActorClusterSeed::new_peaceful_npc(
        id.clone(),
        id.clone(),
        aabb,
        &interactable,
        &[],
    );
    // This is a patrol MOVEMENT test: it needs a patrol brain to drive the body,
    // not the brain-SELECTION logic (which now lives in `resolve_npc_brain` and is
    // tested there). Build the patrol brain directly from the placement lane so the
    // test exercises patrol integration independent of catalog selection.
    let brain = {
        let mut cfg = ambition_characters::brain::PatrolCfg::NPC_DEFAULT;
        cfg.lane = ambition_characters::brain::AuthoredWorldPatrolLane::new(
            seed.spawn.pos.x,
            patrol_radius.max(0.0),
        );
        cfg.aggro_radius = crate::features::NPC_TALK_RADIUS;
        ambition_characters::brain::Brain::StateMachine(
            ambition_characters::brain::StateMachineCfg::Patrol {
                cfg,
                state: ambition_characters::brain::PatrolState::default(),
            },
        )
    };
    let player = crate::avatar::primary_player_scratch(
        ae::Vec2::new(1500.0, 540.0),
        ae::AbilitySet::sandbox_all(),
    );
    (world, seed, brain, player)
}

/// Tick a peaceful actor one frame the way `update_ecs_actors` does: build a
/// brain snapshot, tick the catalog brain into a frame, then integrate the
/// body through the unified `ActorMut::update`.
fn tick_peaceful(
    seed: &mut super::ecs::actor_clusters::ActorClusterSeed,
    brain: &mut ambition_characters::brain::Brain,
    world: &ae::World,
    target: ae::Vec2,
    dt: f32,
    gravity: ae::Vec2,
) {
    let snapshot = ambition_characters::brain::BrainSnapshot {
        captured: false,
        captured_for: 0.0,
        holding_captive: false,
        pummels_landed: 0,
        // A fixture body: unattributed facts are the honest answer here.
        subject: None,
        actor_pos: seed.kin.pos,
        actor_vel: seed.kin.vel,
        actor_facing: seed.kin.facing,
        control_down: gravity,
        movement_frame_mode: ae::InputFrameMode::BodyRelativeAssist,
        aim_frame_mode: ae::InputFrameMode::ScreenRelative,
        actor_on_ground: seed.body.0.ground.on_ground,
        side_contact_normal: seed
            .body
            .0
            .wall
            .on_wall
            .then_some(seed.body.0.wall.wall_normal_x.signum()),
        turns_at_walls: seed.config.brain_profile.turns_at_walls
            && !seed.config.tuning.surface_walker,
        attack_kit: Vec::new(),
        actor_aerial: seed.surface.gravity_scale <= 0.001,
        alive: true,
        target_pos: target,
        target_alive: true,
        health_fraction: 1.0,
        sim_time: 0.0,
        dt,
        // The snapshot's `max_run_speed` MUST be the body's actual physical capability (what
        // the integrator scales `locomotion` by), so the brain's `locomotion_for(patrol_speed)`
        // normalization round-trips to the intended patrol speed.
        max_run_speed: seed.config.tuning.max_run_speed,
        // A fixture body on default tuning: `None` resolves to the engine's
        // canonical movement table, which is what this seed is built from.
        movement_tuning: None,
        abilities: None,
        attack_cooldown_remaining: 0.0,
        attack_windup_remaining: 0.0,
        attack_active_remaining: 0.0,
        attack_recover_remaining: 0.0,
        stun_remaining: 0.0,
        boss_encounter_phase: None,
        world_size: ambition_platformer2d_core::Vec2::ZERO,
        front_wall_clearance: None,
        player_input: None,
        crowding: None,
        terrain: None,
        air_jumps_remaining: 0,
    };
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    brain.tick(&snapshot, &mut frame);
    let mut model = crate::features::MotionModel::default();
    seed.update_for_test(
        world,
        target,
        FeatureCombatTuning::default(),
        dt,
        false,
        frame,
        &mut model,
        ae::MotionFrame::from_direction(gravity, ae::GRAVITY),
    );
}

/// Pin: after a few ticks an NPC spawned in mid-air lands on the floor and `on_ground` flips
/// true.
#[test]
fn npc_falls_to_floor_under_gravity() {
    let (world, mut npc, mut brain, player) = world_with_patrolling_npc(0.0);
    npc.kin.pos.y = 200.0;
    npc.spawn.pos.y = 200.0;
    for _ in 0..120 {
        tick_peaceful(
            &mut npc,
            &mut brain,
            &world,
            player.kinematics.pos,
            0.016,
            ae::Vec2::new(0.0, 1.0),
        );
    }
    assert!(
        npc.body.0.ground.on_ground,
        "NPC must land on the floor under gravity"
    );
    let body_bottom = npc.kin.pos.y + npc.kin.size.y * 0.5;
    assert!(
        (body_bottom - 600.0).abs() < 1.0,
        "expected body bottom near floor top (600); got {body_bottom}"
    );
}

/// A patrolling NPC paces left/right around its spawn within `patrol_radius`.
/// Pin both the motion (NPC moves) and the bound (reverses before exceeding
/// the radius).
#[test]
fn patrolling_npc_paces_within_radius() {
    let (world, mut npc, mut brain, player) = world_with_patrolling_npc(96.0);
    for _ in 0..30 {
        tick_peaceful(
            &mut npc,
            &mut brain,
            &world,
            player.kinematics.pos,
            0.016,
            ae::Vec2::new(0.0, 1.0),
        );
    }
    let spawn_x = npc.spawn.pos.x;
    let mut min_x = npc.kin.pos.x;
    let mut max_x = npc.kin.pos.x;
    for _ in 0..600 {
        tick_peaceful(
            &mut npc,
            &mut brain,
            &world,
            player.kinematics.pos,
            0.016,
            ae::Vec2::new(0.0, 1.0),
        );
        min_x = min_x.min(npc.kin.pos.x);
        max_x = max_x.max(npc.kin.pos.x);
    }
    assert!(
        max_x - min_x > 50.0,
        "patrolling NPC must move; range was {min_x}-{max_x}"
    );
    assert!(
        min_x >= spawn_x - 96.0 - 6.0,
        "NPC went too far left: {min_x} < {} - 6",
        spawn_x - 96.0
    );
    assert!(
        max_x <= spawn_x + 96.0 + 6.0,
        "NPC went too far right: {max_x} > {} + 6",
        spawn_x + 96.0
    );
}

/// patrol_radius=0 is the explicit "static NPC" knob — no motion regardless
/// of how long the simulation runs.
#[test]
fn npc_with_zero_patrol_radius_stays_at_spawn_x() {
    let (world, mut npc, mut brain, player) = world_with_patrolling_npc(0.0);
    let original_x = npc.kin.pos.x;
    for _ in 0..300 {
        tick_peaceful(
            &mut npc,
            &mut brain,
            &world,
            player.kinematics.pos,
            0.016,
            ae::Vec2::new(0.0, 1.0),
        );
    }
    assert!(
        (npc.kin.pos.x - original_x).abs() < 1.0,
        "static NPC must not drift; was {original_x}, now {}",
        npc.kin.pos.x
    );
}

/// Pre-hostile NPC's catalog brain reports not-hostile; the EFFECTS-stage
/// attack gate uses this to skip melee. Locks in "aggressiveness in the brain".
#[test]
fn peaceful_npc_brain_is_not_hostile() {
    let (_, _npc, brain, _) = world_with_patrolling_npc(96.0);
    assert!(
        !brain.is_hostile(),
        "peaceful NPC brain must report !is_hostile"
    );
}
// Bodies are constructed from `CharacterDefinition`; unresolved or incomplete
// definitions are construction errors rather than fallbacks to generic archetypes.
// Per-character durability and contact tuning are tested beside each character.

// The remaining patrol-collision test builds its own collision world inline.

fn enemy_aabb(pos: ae::Vec2) -> ae::Aabb {
    ae::Aabb::new(pos, ae::Vec2::new(14.0, 23.0))
}

// Chase motion now comes from the brain's tick output, not from `evaluate_character_ai_output`;
// brain-side tick equivalence lives in `ambition_characters::brain::state_machine` tests.

// Path patrol + melee-pressed routing now comes from the brain frame; the integration's job is
// just to react to whatever frame the brain emits. Brain-side coverage for path patrol lives in
// `ambition_characters::brain::state_machine::tick_patrol` tests.

// Tests for the legacy fused PirateOnShark archetype (rider+shark
// share one entity, `apply_damage_at` routes hits to rider vs
// body AABB, dismount morphs the archetype) deleted with the
// mount/rider split. The composite is now two linked entities;
// coverage lives in
// `crate::features::ecs::mount::tests`.

/// With the brain→sim seam (`ActorControlFrame` + uniform `step_motion`) the wall blocks them, so
/// the position must stay on the safe side of the wall after one tick of forced chase.
#[test]
fn aerial_enemy_respects_world_collision_against_a_wall() {
    let world = ae::World::new(
        String::from("aerial_collision_test"),
        ae::Vec2::new(800.0, 600.0),
        ae::Vec2::new(100.0, 100.0),
        vec![
            ae::Block::solid(
                String::from("floor"),
                ae::Vec2::new(0.0, 560.0),
                ae::Vec2::new(800.0, 40.0),
            ),
            ae::Block::solid(
                String::from("wall"),
                ae::Vec2::new(300.0, 200.0),
                ae::Vec2::new(40.0, 320.0),
            ),
        ],
    );
    let aabb = ae::Aabb::new(ae::Vec2::new(200.0, 300.0), ae::Vec2::new(20.0, 16.0));
    let mut enemy = super::ecs::actor_clusters::ActorClusterSeed::new(
        "shark_a",
        "Burning Flying Shark",
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom("burning_flying_shark".into()),
        &[],
    );
    enemy.attack.cooldown = 0.0;
    let player_pos = ae::Vec2::new(500.0, 300.0);
    // Drive the enemy directly with a brain-shaped frame
    // requesting rightward motion at chase speed — the test
    // verifies the integration step blocks the body against
    // the wall, not just the steering code that picks velocity.
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.velocity_target = ae::WorldVec2::new(enemy.config.tuning.chase_speed, 0.0);
    for _ in 0..120 {
        let mut model = crate::features::MotionModel::default();
        enemy.update_for_test(
            &world,
            player_pos,
            FeatureCombatTuning::default(),
            1.0 / 60.0,
            false,
            frame,
            &mut model,
            ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
        );
    }
    let half_w = enemy.kin.size.x * 0.5;
    let wall_left_edge = 300.0;
    assert!(
        enemy.kin.pos.x + half_w <= wall_left_edge + 0.5,
        "aerial enemy clipped into wall at pos {:?}; wall left edge {}",
        enemy.kin.pos,
        wall_left_edge,
    );
}

#[test]
fn patrol_enemy_respects_world_collision_against_a_wall() {
    let world = ae::World::new(
        String::from("patrol_collision_test"),
        ae::Vec2::new(800.0, 600.0),
        ae::Vec2::new(100.0, 100.0),
        vec![
            ae::Block::solid(
                String::from("floor"),
                ae::Vec2::new(0.0, 560.0),
                ae::Vec2::new(800.0, 40.0),
            ),
            ae::Block::solid(
                String::from("wall"),
                ae::Vec2::new(200.0, 480.0),
                ae::Vec2::new(40.0, 80.0),
            ),
        ],
    );
    let aabb = enemy_aabb(ae::Vec2::new(100.0, 536.0));
    let path = ambition_platformer2d_core::KinematicPath {
        points: vec![ae::Vec2::new(100.0, 536.0), ae::Vec2::new(400.0, 536.0)],
        speed: 120.0,
        mode: ambition_platformer2d_core::KinematicPathMode::PingPong,
        start_offset_seconds: 0.0,
    };
    let paths = vec![("skitter_path".to_string(), path)];
    let mut enemy = super::ecs::actor_clusters::ActorClusterSeed::new(
        "path_skitter",
        "path_skitter",
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Patrol {
            path_id: Some("skitter_path".into()),
        },
        &paths,
    );
    enemy.attack.cooldown = 0.0;
    let player_pos_far = ae::Vec2::new(2000.0, 536.0);
    // Drive directly with a brain-shaped frame requesting
    // rightward patrol motion — the test verifies the
    // integration step blocks the body against the wall.
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    // Full-throttle rightward run intent; the enemy's tuning owns the px/s scale.
    frame.locomotion = ae::LocalAxes::new(1.0, 0.0);
    for _ in 0..120 {
        let mut model = crate::features::MotionModel::default();
        enemy.update_for_test(
            &world,
            player_pos_far,
            FeatureCombatTuning::default(),
            1.0 / 60.0,
            false,
            frame,
            &mut model,
            ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
        );
    }
    let half_w = enemy.kin.size.x * 0.5;
    let wall_left_edge = 200.0;
    assert!(
        enemy.kin.pos.x + half_w <= wall_left_edge + 0.5,
        "patrol enemy clipped into wall at pos {:?}; wall left edge {}",
        enemy.kin.pos,
        wall_left_edge,
    );
}

/// Side contacts are a semantic movement fact even under sideways gravity.
/// The integrator reports the contact in the body's local side axis; it does
/// not decide that the body should reverse.
#[test]
fn sideways_wall_contact_is_reported_without_mutating_facing() {
    let gravity = ae::Vec2::new(1.0, 0.0);
    let world = ae::World::new(
        String::from("sideways_wall_contact"),
        ae::Vec2::new(800.0, 600.0),
        ae::Vec2::new(100.0, 300.0),
        vec![
            ae::Block::solid(
                String::from("ground_wall"),
                ae::Vec2::new(300.0, 80.0),
                ae::Vec2::new(60.0, 440.0),
            ),
            ae::Block::solid(
                String::from("cap_bottom"),
                ae::Vec2::new(250.0, 450.0),
                ae::Vec2::new(60.0, 90.0),
            ),
        ],
    );
    let aabb = enemy_aabb(ae::Vec2::new(286.0, 300.0));
    let mut enemy = super::ecs::actor_clusters::ActorClusterSeed::new(
        "sideways_walker",
        "sideways_walker",
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Passive,
        &[],
    );
    enemy.kin.facing = 1.0;
    let mut model = enemy.config.tuning.motion_model();
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.facing = 1.0;
    // With gravity pointing world-right, local +side points world-up. The
    // side obstacle in this fixture is below the body, so drive local -side
    // into it; the resulting top-face normal is a semantic local-side contact.
    frame.locomotion = ae::LocalAxes::new(-1.0, 0.0);

    for _ in 0..240 {
        enemy.update_for_test(
            &world,
            ae::Vec2::new(2000.0, 300.0),
            FeatureCombatTuning::default(),
            1.0 / 60.0,
            false,
            frame,
            &mut model,
            ae::MotionFrame::from_direction(gravity, ae::GRAVITY),
        );
        if enemy.body.0.wall.on_wall {
            break;
        }
    }

    assert!(
        enemy.body.0.wall.on_wall,
        "the shared kernel should report a side contact"
    );
    assert!(
        enemy.body.0.wall.wall_normal_x.abs() > 0.5,
        "side contact should carry a local-side normal"
    );
    assert_eq!(
        enemy.kin.facing, 1.0,
        "collision reports contact; the controller/brain owns facing"
    );
}

// Fire intent now comes from the brain's tick output, not the legacy orbit-and- fire branch
// that lived inside `build_control_frame`. The EFFECTS-consumer test
// `spawn_projectiles_from_brain_actions::tests::*` still covers the projectile spawn shape;
// brain-side fire-intent generation belongs in the relevant brain backend's tests.

/// A surface-walking enemy (PuppySlug) GLUED to a moving platform rides it by
/// the full platform velocity — the emergent-riding fix for "slugs behave weird
/// on moving platforms". Isolated by comparing a moving platform against an
/// identical static one: the surface-crawl is the same in both, so the extra
/// displacement is exactly the carry.
fn slug_step_on_platform(platform_velocity: ae::Vec2) -> f32 {
    // A platform-shaped solid (BlinkWall, like real moving platforms) carrying
    // `platform_velocity`. Slug stands on its top.
    let mut platform = ae::Block::blink_wall(
        String::from("platform"),
        ae::Vec2::new(0.0, 500.0),
        ae::Vec2::new(400.0, 40.0),
        ae::BlinkWallTier::Soft,
    );
    platform.velocity = platform_velocity;
    let world = ae::World::new(
        String::from("slug_platform"),
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(100.0, 100.0),
        vec![platform],
    );
    let aabb = ae::Aabb::new(ae::Vec2::new(200.0, 492.0), ae::Vec2::new(10.0, 8.0));
    let mut enemy = super::ecs::actor_clusters::ActorClusterSeed::new(
        "slug",
        "PuppySlug",
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Passive,
        &[],
    );
    // The crawler POLICY with a live attachment (independent of which
    // archetype the brain resolves to): glued to the platform top. This is
    // exactly the model `ActorTuning::motion_model` selects for a
    // `surface_walker` archetype at spawn.
    enemy.config.tuning.surface_walker = true;
    enemy.body.0.ground.on_ground = true;
    enemy.surface.surface_normal = ae::Vec2::new(0.0, -1.0);
    let x0 = enemy.kin.pos.x;
    let mut model = enemy.config.tuning.motion_model();
    let crate::features::MotionModel::AdhesiveCrawler(crawler) = &mut model else {
        panic!("a surface_walker archetype must select the crawler policy at spawn");
    };
    crawler.state = ae::CrawlerState::attached(ae::Vec2::new(0.0, -1.0));
    enemy.update_for_test(
        &world,
        ae::Vec2::new(1500.0, 492.0),
        FeatureCombatTuning::default(),
        1.0 / 60.0,
        false,
        ambition_characters::actor::control::ActorControlFrame::neutral(),
        &mut model,
        ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
    );
    enemy.kin.pos.x - x0
}

#[test]
fn a_surface_walker_rides_a_moving_platform() {
    let static_dx = slug_step_on_platform(ae::Vec2::ZERO);
    let moving_dx = slug_step_on_platform(ae::Vec2::new(5.0, 0.0));
    // The crawl is identical in both; the difference is the +5px platform carry.
    assert!(
        (moving_dx - static_dx - 5.0).abs() < 0.01,
        "slug should ride +5px with the platform: moving_dx={moving_dx}, static_dx={static_dx}"
    );
}

/// Fable review §B2: a NON-surface-walker's published
/// `surface_normal` must track its live gravity (anti-gravity at its
/// position), not its spawn constant — the shield-block side, slash
/// knockback, and ranged muzzle all derive the body frame from it.
#[test]
fn a_normal_actor_surface_normal_tracks_live_gravity() {
    for gravity_dir in [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(-1.0, 0.0),
    ] {
        let world = ae::World::new(
            String::from("normal_frame"),
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(100.0, 100.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 900.0),
                ae::Vec2::new(2000.0, 100.0),
            )],
        );
        let aabb = ae::Aabb::new(ae::Vec2::new(500.0, 500.0), ae::Vec2::new(14.0, 23.0));
        let mut enemy = super::ecs::actor_clusters::ActorClusterSeed::new(
            "walker",
            "Goblin",
            aabb,
            ambition_entity_catalog::placements::CharacterBrain::Passive,
            &[],
        );
        // Spawn constant is (0,-1); the update must overwrite it with the
        let mut model = crate::features::MotionModel::default();
        // live frame for every cardinal.
        enemy.update_for_test(
            &world,
            ae::Vec2::new(600.0, 500.0),
            FeatureCombatTuning::default(),
            1.0 / 60.0,
            false,
            ambition_characters::actor::control::ActorControlFrame::neutral(),
            &mut model,
            ae::MotionFrame::from_direction(gravity_dir, ae::GRAVITY),
        );
        assert_eq!(
            enemy.surface.surface_normal, -gravity_dir,
            "surface_normal must be anti-gravity under {gravity_dir:?}"
        );
    }
}

/// Movement integration never turns a body around on its controller's behalf.
/// A real wall contact is reported through `BodyWallState`; policy above the
/// movement kernel decides whether that should change facing.
#[test]
fn movement_integration_does_not_auto_turn_at_a_wall() {
    let world = ae::World::new(
        String::from("wall"),
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(100.0, 100.0),
        vec![
            ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 500.0),
                ae::Vec2::new(2000.0, 100.0),
            ),
            ae::Block::solid(
                "wall",
                ae::Vec2::new(600.0, 300.0),
                ae::Vec2::new(40.0, 200.0),
            ),
        ],
    );
    let aabb = ae::Aabb::new(ae::Vec2::new(500.0, 476.0), ae::Vec2::new(14.0, 23.0));
    let mut body = super::ecs::actor_clusters::ActorClusterSeed::new(
        "walker",
        "Snake",
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Passive,
        &[],
    );
    body.config.brain_profile.turns_at_walls = true;
    body.kin.facing = 1.0;
    let mut model = body.config.tuning.motion_model();

    for _ in 0..240 {
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        frame.facing = 1.0;
        frame.locomotion = ae::LocalAxes::new(1.0, 0.0);
        body.update_for_test(
            &world,
            ae::Vec2::new(1500.0, 476.0),
            FeatureCombatTuning::default(),
            1.0 / 60.0,
            false,
            frame,
            &mut model,
            ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
        );
    }

    assert!(body.body.0.wall.on_wall);
    assert_eq!(body.kin.facing, 1.0);
}

#[test]
fn stopping_in_open_space_preserves_facing() {
    let world = ae::World::new(
        String::from("open_floor"),
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(100.0, 100.0),
        vec![ae::Block::solid(
            "floor",
            ae::Vec2::new(0.0, 500.0),
            ae::Vec2::new(2000.0, 100.0),
        )],
    );
    let aabb = ae::Aabb::new(ae::Vec2::new(500.0, 476.0), ae::Vec2::new(14.0, 23.0));
    let mut body = super::ecs::actor_clusters::ActorClusterSeed::new(
        "fighter",
        "fighter",
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Passive,
        &[],
    );
    body.config.brain_profile.turns_at_walls = true;
    body.kin.facing = 1.0;
    body.kin.vel.x = 120.0;
    let mut model = body.config.tuning.motion_model();

    for _ in 0..240 {
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        frame.facing = body.kin.facing;
        body.update_for_test(
            &world,
            ae::Vec2::new(1500.0, 476.0),
            FeatureCombatTuning::default(),
            1.0 / 60.0,
            false,
            frame,
            &mut model,
            ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
        );
    }

    assert!(!body.body.0.wall.on_wall);
    assert_eq!(body.kin.facing, 1.0);
}
