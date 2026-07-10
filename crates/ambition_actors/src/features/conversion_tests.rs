//! Headless movement + collision tests for the actor simulation: NPC gravity
//! settle / patrol / talk-stop / possession and enemy aerial / patrol / wall /
//! sideways-gravity / moving-platform-ride behaviour, plus archetype-tuning
//! invariants — all driven through the cluster scratch views without a renderer.

use super::*;

#[cfg(test)]
mod conversion_tests {
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
            },
        );
        let (seed, _render) = super::ecs::actor_clusters::ActorClusterSeed::new_peaceful_npc(
            id.clone(),
            id.clone(),
            aabb,
            &interactable,
            &[],
        );
        let brain = crate::features::npcs::npc_brain_from_catalog(
            &interactable,
            seed.config.spawn.pos.x,
            patrol_radius.max(0.0),
            crate::features::NPC_TALK_RADIUS,
            false,
        );
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
            actor_pos: seed.kin.pos,
            actor_vel: seed.kin.vel,
            actor_facing: seed.kin.facing,
            control_down: gravity,
            movement_frame_mode: ae::InputFrameMode::BodyRelativeAssist,
            aim_frame_mode: ae::InputFrameMode::ScreenRelative,
            actor_on_ground: seed.body.0.ground.on_ground,
            actor_aerial: seed.surface.gravity_scale <= 0.001,
            alive: true,
            target_pos: target,
            target_alive: true,
            health_fraction: 1.0,
            sim_time: 0.0,
            dt,
            // The snapshot's `max_run_speed` MUST be the body's actual physical
            // capability (what the integrator scales `locomotion` by), so the
            // brain's `locomotion_for(patrol_speed)` normalization round-trips to
            // the intended patrol speed. (Was hardcoded NPC_PATROL_SPEED, which
            // only round-tripped while the body's max_run_speed happened to equal
            // it — now decoupled: capability 270 vs patrol policy 60.)
            max_run_speed: seed.config.tuning.max_run_speed,
            attack_cooldown_remaining: 0.0,
            attack_windup_remaining: 0.0,
            attack_active_remaining: 0.0,
            attack_recover_remaining: 0.0,
            stun_remaining: 0.0,
            wall_contact: None,
            boss_encounter_phase: None,
            world_size: ambition_engine_core::Vec2::ZERO,
            front_wall_clearance: None,
            player_input: None,
            crowding: None,
            terrain: None,
            air_jumps_remaining: 0,
        };
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        brain.tick(&snapshot, &mut frame);
        seed.update_for_test(
            world,
            target,
            FeatureCombatTuning::default(),
            None,
            dt,
            false,
            frame,
            gravity,
        );
    }

    /// Bug the user reported: NPCs floated wherever LDtk placed them because the
    /// runtime didn't tick gravity / collision on them. Pin: after a few ticks an
    /// NPC spawned in mid-air lands on the floor and `on_ground` flips true.
    #[test]
    fn npc_falls_to_floor_under_gravity() {
        let (world, mut npc, mut brain, player) = world_with_patrolling_npc(0.0);
        npc.kin.pos.y = 200.0;
        npc.config.spawn.pos.y = 200.0;
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
        let spawn_x = npc.config.spawn.pos.x;
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

    /// `npc_brain_from_catalog` picks Patrol vs StandStill from the authored
    /// fields. Pins the spawn-time mapping the unified actor tick depends on.
    #[test]
    fn npc_brain_from_catalog_picks_template_from_authored_fields() {
        let interactable = |radius: f32| {
            ambition_interaction::Interactable::new(
                "kira",
                "Talk",
                ae::Aabb::new(ae::Vec2::new(800.0, 540.0), ae::Vec2::new(11.0, 19.0)),
                ambition_interaction::InteractionKind::Npc {
                    character_id: None,
                    dialogue_id: Some("kira".into()),
                    patrol_radius: radius,
                    patrol_path_id: None,
                },
            )
        };
        match crate::features::npcs::npc_brain_from_catalog(
            &interactable(0.0),
            800.0,
            0.0,
            crate::features::NPC_TALK_RADIUS,
            false,
        ) {
            ambition_characters::brain::Brain::StateMachine(
                ambition_characters::brain::StateMachineCfg::StandStill,
            ) => {}
            other => panic!("expected StandStill for zero-radius NPC, got {other:?}"),
        }
        match crate::features::npcs::npc_brain_from_catalog(
            &interactable(64.0),
            800.0,
            64.0,
            crate::features::NPC_TALK_RADIUS,
            false,
        ) {
            ambition_characters::brain::Brain::StateMachine(
                ambition_characters::brain::StateMachineCfg::Patrol { cfg, .. },
            ) => {
                assert_eq!(cfg.lane.radius_px, 64.0);
                assert_eq!(cfg.aggressiveness, 0.0);
                assert!(cfg.aggro_radius > 0.0);
            }
            other => panic!("expected Patrol for nonzero-radius NPC, got {other:?}"),
        }
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

    /// The Perfect Cell-ular Automaton resolves to its dedicated boss
    /// archetype when provoked — by catalog id, display name, or the
    /// encounter's dialogue node. Pins the dialogue-gated boss wiring so a
    /// rename in any of the three doesn't silently drop the PCA to the
    /// generic `combatant` grunt.
    #[test]
    fn perfect_cellular_automaton_provokes_to_its_boss_archetype() {
        use crate::features::ecs::hostile_brain_id_for_actor;
        use crate::features::enemies::test_spec;
        assert_eq!(
            hostile_brain_id_for_actor(
                "perfect_cellular_automaton",
                "Perfect Cellular Automaton",
                None
            ),
            "cellular_automaton_fighter",
        );
        assert_eq!(
            hostile_brain_id_for_actor(
                "npc_unknown",
                "Mystery",
                Some("perfect_cellular_automaton")
            ),
            "cellular_automaton_fighter",
        );
        // The boss archetype is beefier than a grunt (60 HP — bumped so the duel
        // bout breathes — vs the combatant's 4) and is a hostile Smash fighter.
        let spec = test_spec("cellular_automaton_fighter");
        assert_eq!(spec.max_health, 60);
        // It is a grounded-base HYBRID (S3b): it prefers to fight on the ground
        // (so on provoke its gravity re-syncs to grounded and it descends), but
        // carries the `can_fly` kit to take to the air and cover a long traversal
        // gap. Flight is a brain *preference* (I4), not a fixed body mode.
        assert!(
            !spec.is_aerial,
            "the PCA boss is grounded-base (prefers grounded), not a permanent flyer"
        );
        assert!(
            spec.smash_can_fly,
            "the PCA boss has the fly kit so it can traverse long distances airborne"
        );
        // It carries the glider as its ranged zoning tool.
        assert!(
            spec.ranged.is_some(),
            "the PCA should have a ranged glider poke"
        );
        // S3c: it carries the reactive-block kit. One authored flag projects into
        // BOTH the brain's attempt (`smash_can_shield` -> `SmashCfg::can_shield`)
        // AND the body's enforce gate (`CombatCapabilities::can_shield`).
        assert!(
            spec.smash_can_shield,
            "the PCA boss has the reactive-block kit so it can guard a lunge it won't blink"
        );
        assert!(
            spec.combat_capabilities().can_shield,
            "the authored shield kit must project onto the body's enforce gate"
        );
        assert!(
            spec.brain_spec().smash_can_shield,
            "the authored shield kit must project onto the brain's attempt"
        );
        // S3d: it carries the dash kit (the body turns its dash-to-close decision
        // into a real burst). Projects onto the body's enforce gate.
        assert!(
            spec.smash_can_dash,
            "the PCA boss has the dash kit so its dash-to-close becomes a real burst"
        );
        assert!(
            spec.combat_capabilities().can_dash,
            "the authored dash kit must project onto the body's enforce gate"
        );
    }

    #[test]
    fn enemy_brain_keys_resolve_to_their_rows() {
        use crate::features::enemies::test_spec;
        // A known spawn brain key resolves to its own authored row...
        assert_eq!(test_spec("small_skitter").max_health, 2);
        assert_eq!(test_spec("large_brute").max_health, 9);
        assert_eq!(test_spec("sandbag_infinite").max_health, 9999);
        // ...and an unknown / non-roster key falls back to the combatant row.
        assert_eq!(
            test_spec("unknown_brain").max_health,
            test_spec("combatant").max_health,
        );
    }

    /// Every combat archetype reports finite, non-NaN tunings. A regression
    /// here would mean a numerical typo in the authored `character_archetypes.ron`
    /// row (most likely an `f32::NAN` slipped in). Hostile archetypes
    /// additionally must have positive `attack_range` + `contact_strength`;
    /// peaceful rows (puppy_slug, pirate_heavy) may have `attack_range == 0.0`
    /// because they don't emit a melee windup.
    #[test]
    fn enemy_archetype_tunings_are_finite() {
        use crate::features::enemies::{test_spec, COMBAT_BRAIN_KEYS};
        for key in COMBAT_BRAIN_KEYS {
            let spec = test_spec(key);
            assert!(spec.max_health > 0);
            assert!(spec.patrol_speed.is_finite());
            assert!(spec.tuning().chase_speed.is_finite());
            assert!(spec.tuning().aggro_radius.is_finite());
            assert!(spec.tuning().aggro_radius >= 0.0);
            assert!(spec.tuning().attack_range.is_finite());
            assert!(spec.tuning().attack_range >= 0.0);
            assert!(spec.contact_strength.is_finite());
            assert!(spec.contact_strength >= 0.0);
            assert!(spec.damage_amount > 0);
            if spec.attacks_player {
                assert!(
                    spec.tuning().attack_range > 0.0,
                    "{key} reports it attacks but has zero attack_range",
                );
                assert!(
                    spec.contact_strength > 0.0,
                    "{key} reports it attacks but has zero contact_strength",
                );
            }
        }
    }

    /// Cross-archetype invariants for the S/M/L × low/med/high
    /// aggression matrix. Locks in the design contract that:
    /// - "Large" archetypes have more HP than "Small" ones.
    /// - High-aggression archetypes have wider aggro radii than
    ///   their low-aggression siblings of the same size.
    /// - Damage scales with size class.
    #[test]
    fn enemy_archetype_size_and_aggression_invariants() {
        // HP: small < medium < large.
        assert!(
            crate::features::enemies::test_spec("small_skitter").max_health
                < crate::features::enemies::test_spec("medium_striker").max_health
        );
        assert!(
            crate::features::enemies::test_spec("small_lurker").max_health
                < crate::features::enemies::test_spec("medium_striker").max_health
        );
        assert!(
            crate::features::enemies::test_spec("medium_striker").max_health
                < crate::features::enemies::test_spec("large_brute").max_health
        );
        assert!(
            crate::features::enemies::test_spec("large_brute").max_health
                < crate::features::enemies::test_spec("large_colossus").max_health
        );

        // Aggro radius: low-aggression < high-aggression at same size.
        assert!(
            crate::features::enemies::test_spec("small_lurker")
                .tuning()
                .aggro_radius
                < crate::features::enemies::test_spec("small_skitter")
                    .tuning()
                    .aggro_radius
        );
        assert!(
            crate::features::enemies::test_spec("large_colossus")
                .tuning()
                .aggro_radius
                < crate::features::enemies::test_spec("large_brute")
                    .tuning()
                    .aggro_radius
        );

        // Damage: large > medium / small (LargeColossus is the heaviest hitter).
        assert!(
            crate::features::enemies::test_spec("large_colossus").damage_amount
                >= crate::features::enemies::test_spec("large_brute").damage_amount
        );
        assert!(
            crate::features::enemies::test_spec("large_brute").damage_amount
                > crate::features::enemies::test_spec("small_skitter").damage_amount
        );

        // Patrol speed: lurker / colossus visibly slower than their
        // higher-aggression siblings.
        assert!(
            crate::features::enemies::test_spec("small_lurker").patrol_speed
                < crate::features::enemies::test_spec("small_skitter").patrol_speed
        );
        assert!(
            crate::features::enemies::test_spec("large_colossus").patrol_speed
                < crate::features::enemies::test_spec("large_brute").patrol_speed
        );
    }

    // `enemy_test_world` was deleted alongside the legacy AI tests
    // that consumed it. The remaining patrol-collision test builds
    // its own collision world inline.

    fn enemy_aabb(pos: ae::Vec2) -> ae::Aabb {
        ae::Aabb::new(pos, ae::Vec2::new(14.0, 23.0))
    }

    // `enemy_ai_output_drives_chase_motion` was deleted with the
    // brain-authority GC pass that dropped the legacy
    // `build_control_frame` path. Chase motion now
    // comes from the brain's tick output, not from
    // `evaluate_character_ai_output`; brain-side tick equivalence
    // lives in `ambition_characters::brain::state_machine` tests.

    // `path_enemy_holds_patrol_and_starts_attack_from_character_ai_output`
    // was deleted with the brain-authority GC pass. Path patrol +
    // melee-pressed routing now comes from the brain frame; the
    // integration's job is just to react to whatever frame the
    // brain emits. Brain-side coverage for path patrol lives in
    // `ambition_characters::brain::state_machine::tick_patrol` tests.

    // Tests for the legacy fused PirateOnShark archetype (rider+shark
    // share one entity, `apply_damage_at` routes hits to rider vs
    // body AABB, dismount morphs the archetype) deleted with the
    // mount/rider split. The composite is now two linked entities;
    // coverage lives in
    // `crate::features::ecs::mount::tests`.

    /// Aerial enemies (flying shark + rider) used to write `self.pos`
    /// directly from a steering target, which let
    /// them clip straight through solid walls. With the brain→sim
    /// seam (`ActorControlFrame` + uniform `step_kinematic`) the
    /// wall blocks them, so the position must stay on the safe side
    /// of the wall after one tick of forced chase.
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
            ambition_entity_catalog::placements::CharacterBrain::Custom(
                "burning_flying_shark".into(),
            ),
            &[],
        );
        enemy.attack.cooldown = 0.0;
        let player_pos = ae::Vec2::new(500.0, 300.0);
        // Drive the enemy directly with a brain-shaped frame
        // requesting rightward motion at chase speed — the test
        // verifies the integration step blocks the body against
        // the wall, not just the steering code that picks velocity.
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        frame.velocity_target = ae::Vec2::new(enemy.config.tuning.chase_speed, 0.0);
        for _ in 0..120 {
            enemy.update_for_test(
                &world,
                player_pos,
                FeatureCombatTuning::default(),
                None,
                1.0 / 60.0,
                false,
                frame,
                ae::Vec2::new(0.0, 1.0),
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

    /// Path-patrol enemies used to write `self.pos = motion.advance(...)`
    /// directly, bypassing world collision. With the brain→sim seam
    /// the path lookahead becomes a desired velocity that `step_kinematic`
    /// blocks against solids — so a wall placed on the patrol curve
    /// stops the body short of the wall instead of letting it clip.
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
        let path = ambition_engine_core::KinematicPath {
            points: vec![ae::Vec2::new(100.0, 536.0), ae::Vec2::new(400.0, 536.0)],
            speed: 120.0,
            mode: ambition_engine_core::KinematicPathMode::PingPong,
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
        frame.locomotion = ae::Vec2::new(1.0, 0.0);
        for _ in 0..120 {
            enemy.update_for_test(
                &world,
                player_pos_far,
                FeatureCombatTuning::default(),
                None,
                1.0 / 60.0,
                false,
                frame,
                ae::Vec2::new(0.0, 1.0),
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

    /// Under SIDEWAYS gravity a patrolling enemy walks along the gravity-
    /// PERPENDICULAR axis (vertical), so the wall-stop "reverse facing" detection
    /// must watch that axis — not screen-x. The old `vel.x` read never fired here
    /// (x is the near-zero gravity axis when grounded), so a patroller would push
    /// into a wall forever. This pins the gravity-relative fix: driven into a
    /// blocking wall along its run axis, the enemy flips facing exactly once.
    #[test]
    fn patrol_enemy_reverses_facing_at_a_wall_under_sideways_gravity() {
        // Gravity points +x (right); the enemy rests against the +x "ground" wall
        // and patrols along the perpendicular (vertical) axis inside a corridor
        // capped top and bottom by blockers.
        let gravity = ae::Vec2::new(1.0, 0.0);
        let world = ae::World::new(
            String::from("sideways_patrol_test"),
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(100.0, 300.0),
            vec![
                // The surface the enemy is pushed onto (its "floor" under +x gravity).
                ae::Block::solid(
                    String::from("ground_wall"),
                    ae::Vec2::new(300.0, 80.0),
                    ae::Vec2::new(60.0, 440.0),
                ),
                // Corridor caps in the vertical run path.
                ae::Block::solid(
                    String::from("cap_top"),
                    ae::Vec2::new(250.0, 60.0),
                    ae::Vec2::new(60.0, 90.0),
                ),
                ae::Block::solid(
                    String::from("cap_bottom"),
                    ae::Vec2::new(250.0, 450.0),
                    ae::Vec2::new(60.0, 90.0),
                ),
            ],
        );
        // Right edge (center.x + 14) touches the ground wall at x = 300.
        let aabb = enemy_aabb(ae::Vec2::new(286.0, 300.0));
        let paths: Vec<(String, ambition_engine_core::KinematicPath)> = vec![];
        let mut enemy = super::ecs::actor_clusters::ActorClusterSeed::new(
            "sideways_patroller",
            "sideways_patroller",
            aabb,
            ambition_entity_catalog::placements::CharacterBrain::Patrol { path_id: None },
            &paths,
        );
        enemy.attack.cooldown = 0.0;
        // Force the AI into Patrol: no aggro/attack reach, patrol enabled, so the
        // far player can't pull it into Chase (the flip only fires for Patrol).
        enemy.config.tuning.aggro_radius = 0.0;
        enemy.config.tuning.attack_range = 0.0;
        enemy.config.tuning.is_sandbag = false;
        let initial_facing = enemy.kin.facing;
        let player_pos_far = ae::Vec2::new(2000.0, 300.0);
        // Constant run intent along the perpendicular axis (sign maps to ±vertical);
        // the enemy travels until a cap stops it, then the detection flips facing.
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        // Full-throttle run intent along the local side axis; tuning owns px/s.
        frame.locomotion = ae::Vec2::new(1.0, 0.0);
        // Count facing reversals: with the OLD screen-x detection, `vel.x` is the
        // (zeroed, grounded) gravity axis so the wall-stop NEVER triggers → zero
        // flips and the enemy grinds into the wall forever. With the gravity-
        // perpendicular detection the vertical stall is seen and facing reverses.
        // (The constant test frame keeps driving INTO the cap, so facing re-flips
        // on re-contact — we assert it reverses at all, not an exact parity.)
        let mut flips = 0u32;
        let mut prev_facing = enemy.kin.facing;
        for _ in 0..240 {
            enemy.update_for_test(
                &world,
                player_pos_far,
                FeatureCombatTuning::default(),
                None,
                1.0 / 60.0,
                false,
                frame,
                gravity,
            );
            if enemy.kin.facing != prev_facing {
                flips += 1;
                prev_facing = enemy.kin.facing;
            }
        }
        let _ = initial_facing;
        assert!(
            flips >= 1,
            "a patroller that stalls against a wall under sideways gravity must \
             reverse facing — the wall-stop detection has to watch the vertical \
             run axis, not screen-x (which is the zeroed gravity axis here); got \
             {flips} flips, mode={:?}",
            enemy.status.ai_mode,
        );
    }

    // `pirate_on_shark_fire_intent_lands_on_actor_control_frame`
    // was deleted with the brain-authority GC pass. Fire intent now
    // comes from the brain's tick output, not the legacy orbit-and-
    // fire branch that lived inside `build_control_frame`.
    // The EFFECTS-consumer test
    // `spawn_enemy_projectiles_from_brain_actions::tests::*` still
    // covers the projectile spawn shape; brain-side fire-intent
    // generation belongs in the relevant brain backend's tests.

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
        // Force the surface-walker grounded state directly (independent of which
        // archetype the brain resolves to): glued to the platform top.
        enemy.config.tuning.surface_walker = true;
        enemy.body.0.ground.on_ground = true;
        enemy.surface.surface_normal = ae::Vec2::new(0.0, -1.0);
        let x0 = enemy.kin.pos.x;
        enemy.update_for_test(
            &world,
            ae::Vec2::new(1500.0, 492.0),
            FeatureCombatTuning::default(),
            None,
            1.0 / 60.0,
            false,
            ambition_characters::actor::control::ActorControlFrame::neutral(),
            ae::Vec2::new(0.0, 1.0),
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

    /// Fable review 2026-07-02 §B2: a NON-surface-walker's published
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
            // live frame for every cardinal.
            enemy.update_for_test(
                &world,
                ae::Vec2::new(600.0, 500.0),
                FeatureCombatTuning::default(),
                None,
                1.0 / 60.0,
                false,
                ambition_characters::actor::control::ActorControlFrame::neutral(),
                gravity_dir,
            );
            assert_eq!(
                enemy.surface.surface_normal, -gravity_dir,
                "surface_normal must be anti-gravity under {gravity_dir:?}"
            );
        }
    }
}
