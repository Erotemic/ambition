use super::*;

#[cfg(test)]
mod conversion_tests {
    use super::*;

    /// Build an NPC with a patrol radius and a player parked far
    /// outside the talk radius — used by the patrol-motion tests so
    /// the AI lands on `Patrol` mode each tick.
    fn world_with_patrolling_npc(
        patrol_radius: f32,
    ) -> (
        ae::World,
        crate::features::ecs::npc_clusters::NpcClusterScratch,
        ae::PlayerClusterScratch,
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
        let interactable = crate::interaction::Interactable::new(
            id.clone(),
            String::from("Talk"),
            aabb,
            crate::interaction::InteractionKind::Npc {
                dialogue_id: Some(id.clone()),
                patrol_radius,
                patrol_path_id: None,
            },
        );
        let npc = crate::features::ecs::npc_clusters::NpcClusterScratch::new_with_paths(
            id.clone(),
            id.clone(),
            aabb,
            interactable,
            &[],
        );
        let player = crate::player::primary_player_scratch(
            ae::Vec2::new(1500.0, 540.0),
            ae::AbilitySet::sandbox_all(),
        );
        (world, npc, player)
    }

    /// Build a brain matching the NPC's authored fields. Convenience
    /// for the conversion tests so each scenario doesn't repeat the
    /// `let mut brain = npc.build_brain();` setup boilerplate.
    fn brain_for(
        npc: &mut crate::features::ecs::npc_clusters::NpcClusterScratch,
    ) -> crate::brain::Brain {
        npc.as_mut().build_brain()
    }

    /// Bug the user reported: NPCs floated wherever LDtk placed them
    /// because the runtime didn't tick gravity / collision on them.
    /// Pin: after a few ticks an NPC spawned in mid-air lands on the
    /// floor and `on_ground` flips true.
    #[test]
    fn npc_falls_to_floor_under_gravity() {
        let (world, mut npc, player) = world_with_patrolling_npc(0.0);
        // Lift the NPC into mid-air so gravity has work to do.
        npc.kin.pos.y = 200.0;
        npc.config.spawn.y = 200.0;
        let mut brain = brain_for(&mut npc);
        for _ in 0..120 {
            npc.as_mut()
                .tick_via_brain(&mut brain, &world, player.kinematics.pos, 0.0, 0.016, 1.0);
        }
        assert!(
            npc.surface.on_ground,
            "NPC must land on the floor under gravity"
        );
        // Body bottom should rest on the floor's top edge (y=600).
        let body_bottom = npc.kin.pos.y + npc.kin.size.y * 0.5;
        assert!(
            (body_bottom - 600.0).abs() < 1.0,
            "expected body bottom near floor top (600); got {body_bottom}"
        );
    }

    /// Possession-on-NPCs (the #92 "drive any actor via the human path" flex):
    /// `update_ecs_npcs` drives a possessed NPC through `integrate_velocity` with
    /// the player's axis scaled by `POSSESSED_MOVE_SPEED`. Pin that this moves
    /// the body *meaningfully* (the latent crawl bug — a `[-1,1]` direction fed
    /// straight to the integration moved it ~1 px/s — is fixed by the scale).
    #[test]
    fn possessed_npc_walks_at_a_real_speed_not_a_crawl() {
        let (world, mut npc, _player) = world_with_patrolling_npc(0.0);
        // Settle on the floor first so x-motion is the only variable.
        let mut brain = brain_for(&mut npc);
        for _ in 0..120 {
            npc.as_mut()
                .tick_via_brain(&mut brain, &world, ae::Vec2::ZERO, 0.0, 0.016, 1.0);
        }
        let start_x = npc.kin.pos.x;
        // "Possess" → drive right: axis_x = 1 scaled to a real walk speed.
        for _ in 0..60 {
            npc.as_mut().integrate_velocity(
                1.0 * crate::abilities::traversal::possession::POSSESSED_MOVE_SPEED,
                &world,
                0.016,
                1.0,
            );
        }
        let moved = npc.kin.pos.x - start_x;
        assert!(
            moved > 100.0,
            "possessed NPC should walk right meaningfully (not crawl); moved {moved}"
        );
    }

    /// A patrolling NPC paces left/right around its spawn within
    /// `patrol_radius`. Pin both the motion (NPC moves) and the
    /// bound (NPC reverses before exceeding the radius).
    #[test]
    fn patrolling_npc_paces_within_radius() {
        let (world, mut npc, player) = world_with_patrolling_npc(96.0);
        let mut brain = brain_for(&mut npc);
        // Settle gravity first so we're testing horizontal motion,
        // not the freefall.
        for _ in 0..30 {
            npc.as_mut()
                .tick_via_brain(&mut brain, &world, player.kinematics.pos, 0.0, 0.016, 1.0);
        }
        let spawn_x = npc.config.spawn.x;
        let mut min_x = npc.kin.pos.x;
        let mut max_x = npc.kin.pos.x;
        for _ in 0..600 {
            npc.as_mut()
                .tick_via_brain(&mut brain, &world, player.kinematics.pos, 0.0, 0.016, 1.0);
            min_x = min_x.min(npc.kin.pos.x);
            max_x = max_x.max(npc.kin.pos.x);
        }
        // The NPC actually moved some distance — not stuck.
        assert!(
            max_x - min_x > 50.0,
            "patrolling NPC must move; range was {}-{}",
            min_x,
            max_x
        );
        // And stayed inside its patrol bounds (with a small slack
        // for one tick of overshoot before the bound flip kicks in).
        assert!(
            min_x >= spawn_x - 96.0 - 4.0,
            "NPC went too far left: {min_x} < {} - 4",
            spawn_x - 96.0
        );
        assert!(
            max_x <= spawn_x + 96.0 + 4.0,
            "NPC went too far right: {max_x} > {} + 4",
            spawn_x + 96.0
        );
    }

    /// When the player walks within `talk_radius`, a patrolling NPC
    /// must STOP (so the player can interact). This is the inverse
    /// of an enemy "chase" — the shared character_ai vocabulary
    /// flagging "player in range" maps to "hold position" for
    /// peaceful NPCs.
    #[test]
    fn patrolling_npc_stops_when_player_is_within_talk_radius() {
        let (world, mut npc, mut player) = world_with_patrolling_npc(120.0);
        let mut brain = brain_for(&mut npc);
        // Settle physics.
        for _ in 0..30 {
            npc.as_mut()
                .tick_via_brain(&mut brain, &world, player.kinematics.pos, 0.0, 0.016, 1.0);
        }
        // Park the player right next to the NPC — within talk_radius.
        player.kinematics.pos = ae::Vec2::new(npc.kin.pos.x + 30.0, npc.kin.pos.y);
        // Run for a half-second of real time. Whatever momentum was
        // left from the patrol step must drain to ~0 inside the
        // talk radius.
        for _ in 0..30 {
            npc.as_mut()
                .tick_via_brain(&mut brain, &world, player.kinematics.pos, 0.0, 0.016, 1.0);
        }
        assert!(
            matches!(npc.status.ai_mode, crate::actor::ai::CharacterAiMode::Chase),
            "expected Chase mode (NPC interprets as hold-and-face), got {:?}",
            npc.status.ai_mode
        );
        assert!(
            npc.kin.vel.x.abs() < 5.0,
            "NPC must come to rest inside talk_radius; got vel.x={}",
            npc.kin.vel.x
        );
        // And the NPC faces the player so the dialog prompt sits on
        // the right side.
        let dx = player.kinematics.pos.x - npc.kin.pos.x;
        assert_eq!(
            npc.kin.facing.signum(),
            dx.signum(),
            "NPC must face the player"
        );
    }

    /// patrol_radius=0 is the explicit "static NPC" knob — no
    /// motion regardless of how long the simulation runs. Pin so a
    /// future tuning pass that defaults patrol_radius nonzero
    /// doesn't silently move every NPC.
    #[test]
    fn npc_with_zero_patrol_radius_stays_at_spawn_x() {
        let (world, mut npc, player) = world_with_patrolling_npc(0.0);
        let original_x = npc.kin.pos.x;
        let mut brain = brain_for(&mut npc);
        for _ in 0..300 {
            npc.as_mut()
                .tick_via_brain(&mut brain, &world, player.kinematics.pos, 0.0, 0.016, 1.0);
        }
        assert!(
            (npc.kin.pos.x - original_x).abs() < 1.0,
            "static NPC must not drift; was {}, now {}",
            original_x,
            npc.kin.pos.x
        );
        assert!(matches!(
            npc.status.ai_mode,
            crate::actor::ai::CharacterAiMode::Idle | crate::actor::ai::CharacterAiMode::Chase
        ));
    }

    /// build_brain() picks Patrol vs StandStill based on the NPC's
    /// authored fields. Pins the spawn-time mapping the actors
    /// system depends on.
    #[test]
    fn npc_build_brain_picks_template_from_authored_fields() {
        let (_, mut npc_static, _) = world_with_patrolling_npc(0.0);
        match npc_static.as_mut().build_brain() {
            crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::StandStill) => {}
            other => panic!("expected StandStill for zero-radius NPC, got {:?}", other),
        }
        let (_, mut npc_patrol, _) = world_with_patrolling_npc(64.0);
        match npc_patrol.as_mut().build_brain() {
            crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::Patrol {
                cfg,
                ..
            }) => {
                assert_eq!(cfg.radius, 64.0);
                // Peaceful NPC: aggressiveness zero.
                assert_eq!(cfg.aggressiveness, 0.0);
                // talk_radius mirrors into aggro_radius so the
                // engine evaluator returns Chase when in range.
                assert!(cfg.aggro_radius > 0.0);
            }
            other => panic!("expected Patrol for nonzero-radius NPC, got {:?}", other),
        }
    }

    /// Pre-hostile NPC's brain reports not-hostile; the EFFECTS-stage
    /// attack gate uses this to skip melee even if the brain ever
    /// emitted `melee_pressed=true`. Locks in the "aggressiveness in
    /// the brain" decision.
    #[test]
    fn peaceful_npc_brain_is_not_hostile() {
        let (_, mut npc, _) = world_with_patrolling_npc(96.0);
        let brain = npc.as_mut().build_brain();
        assert!(
            !brain.is_hostile(),
            "peaceful NPC brain must report !is_hostile"
        );
    }

    /// NPC brain dispatch over many ticks doesn't accumulate ghost
    /// state — patrol mode flips at the right bound times even when
    /// the brain re-uses the same PatrolState across many ticks.
    /// Regresses against any future patrol-mode-cache bug where
    /// the brain state gets out of sync with the actor body.
    #[test]
    fn npc_brain_patrol_mode_tracks_bounds_across_many_ticks() {
        let (world, mut npc, player) = world_with_patrolling_npc(96.0);
        let mut brain = brain_for(&mut npc);
        // Settle gravity.
        for _ in 0..30 {
            npc.as_mut()
                .tick_via_brain(&mut brain, &world, player.kinematics.pos, 0.0, 0.016, 1.0);
        }
        // Run for a long while; track that the AI mode stays in
        // patrol and never wedges in Idle.
        let mut patrol_ticks = 0;
        let mut chase_ticks = 0;
        for _ in 0..300 {
            npc.as_mut()
                .tick_via_brain(&mut brain, &world, player.kinematics.pos, 0.0, 0.016, 1.0);
            match npc.status.ai_mode {
                crate::actor::ai::CharacterAiMode::Patrol => patrol_ticks += 1,
                crate::actor::ai::CharacterAiMode::Chase => chase_ticks += 1,
                _ => {}
            }
        }
        // Player is far away (1500), so we expect mostly Patrol.
        assert!(
            patrol_ticks > 200,
            "NPC should be patrolling most ticks; got {patrol_ticks}"
        );
        // No chase (player far).
        assert_eq!(chase_ticks, 0, "Player far → no Chase mode");
    }

    /// NPC with no patrol_radius + no motion path → brain emits
    /// neutral frame every tick + the NPC stays exactly at spawn
    /// (no jitter, no drift, no ghost velocity).
    #[test]
    fn static_npc_brain_emits_neutral_each_tick() {
        let (world, mut npc, player) = world_with_patrolling_npc(0.0);
        let mut brain = brain_for(&mut npc);
        for _ in 0..120 {
            npc.as_mut()
                .tick_via_brain(&mut brain, &world, player.kinematics.pos, 0.0, 0.016, 1.0);
        }
        // Vel along x should drain to zero (gravity has settled).
        assert!(
            npc.kin.vel.x.abs() < 0.5,
            "static NPC should have ~zero horizontal velocity; got {}",
            npc.kin.vel.x
        );
    }

    #[test]
    fn enemy_archetype_brain_round_trip() {
        for (name, expected) in [
            ("small_skitter", EnemyArchetype::SmallSkitter),
            ("small_lurker", EnemyArchetype::SmallLurker),
            ("medium_striker", EnemyArchetype::MediumStriker),
            ("large_brute", EnemyArchetype::LargeBrute),
            ("large_colossus", EnemyArchetype::LargeColossus),
            ("gradient_seeker", EnemyArchetype::AggressiveSeeker),
            ("sandbag_infinite", EnemyArchetype::InfiniteSandbag),
            ("sandbag_finite", EnemyArchetype::FiniteSandbag),
            ("unknown_brain", EnemyArchetype::Combatant),
        ] {
            let brain = crate::actor::EnemyBrain::Custom(name.to_string());
            assert_eq!(EnemyArchetype::from_brain(&brain), expected);
        }
    }

    /// Every combat archetype reports finite, non-NaN tunings. A
    /// regression here would mean a numerical typo in the per-archetype
    /// match arms (most likely an `f32::NAN` literal slipped in).
    ///
    /// Hostile archetypes additionally must have positive
    /// `attack_range` + `contact_strength`. Peaceful archetypes
    /// (PuppySlug, PirateHeavy — see `EnemyArchetype::attacks_player`)
    /// are allowed to have `attack_range == 0.0` because they don't
    /// emit a melee windup; the universal-brain refactor moves this
    /// into `Brain::is_hostile()` long-term, but the per-archetype
    /// signal is the source of truth for now.
    #[test]
    fn enemy_archetype_tunings_are_finite() {
        for archetype in EnemyArchetype::COMBAT_ALL {
            assert!(archetype.max_health() > 0);
            assert!(archetype.patrol_speed().is_finite());
            assert!(archetype.tuning().chase_speed.is_finite());
            assert!(archetype.tuning().aggro_radius.is_finite());
            assert!(archetype.tuning().aggro_radius >= 0.0);
            assert!(archetype.tuning().attack_range.is_finite());
            assert!(archetype.tuning().attack_range >= 0.0);
            assert!(archetype.contact_strength().is_finite());
            assert!(archetype.contact_strength() >= 0.0);
            assert!(archetype.damage_amount() > 0);
            if archetype.attacks_player() {
                assert!(
                    archetype.tuning().attack_range > 0.0,
                    "{:?} reports it attacks but has zero attack_range",
                    archetype,
                );
                assert!(
                    archetype.contact_strength() > 0.0,
                    "{:?} reports it attacks but has zero contact_strength",
                    archetype,
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
            EnemyArchetype::SmallSkitter.max_health() < EnemyArchetype::MediumStriker.max_health()
        );
        assert!(
            EnemyArchetype::SmallLurker.max_health() < EnemyArchetype::MediumStriker.max_health()
        );
        assert!(
            EnemyArchetype::MediumStriker.max_health() < EnemyArchetype::LargeBrute.max_health()
        );
        assert!(
            EnemyArchetype::LargeBrute.max_health() < EnemyArchetype::LargeColossus.max_health()
        );

        // Aggro radius: low-aggression < high-aggression at same size.
        assert!(
            EnemyArchetype::SmallLurker.tuning().aggro_radius
                < EnemyArchetype::SmallSkitter.tuning().aggro_radius
        );
        assert!(
            EnemyArchetype::LargeColossus.tuning().aggro_radius
                < EnemyArchetype::LargeBrute.tuning().aggro_radius
        );

        // Damage: large > medium / small (LargeColossus is the heaviest hitter).
        assert!(
            EnemyArchetype::LargeColossus.damage_amount()
                >= EnemyArchetype::LargeBrute.damage_amount()
        );
        assert!(
            EnemyArchetype::LargeBrute.damage_amount()
                > EnemyArchetype::SmallSkitter.damage_amount()
        );

        // Patrol speed: lurker / colossus visibly slower than their
        // higher-aggression siblings.
        assert!(
            EnemyArchetype::SmallLurker.patrol_speed()
                < EnemyArchetype::SmallSkitter.patrol_speed()
        );
        assert!(
            EnemyArchetype::LargeColossus.patrol_speed()
                < EnemyArchetype::LargeBrute.patrol_speed()
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
    // lives in `crate::brain::state_machine` tests.

    // `path_enemy_holds_patrol_and_starts_attack_from_character_ai_output`
    // was deleted with the brain-authority GC pass. Path patrol +
    // melee-pressed routing now comes from the brain frame; the
    // integration's job is just to react to whatever frame the
    // brain emits. Brain-side coverage for path patrol lives in
    // `crate::brain::state_machine::tick_patrol` tests.

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
        let mut enemy = super::ecs::enemy_clusters::EnemyClusterSeed::new(
            "shark_a",
            "Burning Flying Shark",
            aabb,
            crate::actor::EnemyBrain::Custom("pirate_on_shark".into()),
            &[],
        );
        enemy.attack.cooldown = 0.0;
        let player_pos = ae::Vec2::new(500.0, 300.0);
        // Drive the enemy directly with a brain-shaped frame
        // requesting rightward motion at chase speed — the test
        // verifies the integration step blocks the body against
        // the wall, not just the steering code that picks velocity.
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        frame.desired_vel = ae::Vec2::new(enemy.config.tuning.chase_speed, 0.0);
        for _ in 0..120 {
            enemy.update_for_test(
                &world,
                player_pos,
                FeatureCombatTuning::default(),
                None,
                1.0 / 60.0,
                false,
                frame,
                1.0,
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
        let path = crate::actor::KinematicPath {
            points: vec![ae::Vec2::new(100.0, 536.0), ae::Vec2::new(400.0, 536.0)],
            speed: 120.0,
            mode: crate::actor::KinematicPathMode::PingPong,
            start_offset_seconds: 0.0,
        };
        let paths = vec![("skitter_path".to_string(), path)];
        let mut enemy = super::ecs::enemy_clusters::EnemyClusterSeed::new(
            "path_skitter",
            "path_skitter",
            aabb,
            crate::actor::EnemyBrain::Patrol {
                path_id: Some("skitter_path".into()),
            },
            &paths,
        );
        enemy.attack.cooldown = 0.0;
        let player_pos_far = ae::Vec2::new(2000.0, 536.0);
        // Drive directly with a brain-shaped frame requesting
        // rightward patrol motion — the test verifies the
        // integration step blocks the body against the wall.
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        frame.desired_vel = ae::Vec2::new(enemy.config.tuning.patrol_speed, 0.0);
        for _ in 0..120 {
            enemy.update_for_test(
                &world,
                player_pos_far,
                FeatureCombatTuning::default(),
                None,
                1.0 / 60.0,
                false,
                frame,
                1.0,
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

    // `pirate_on_shark_fire_intent_lands_on_actor_control_frame`
    // was deleted with the brain-authority GC pass. Fire intent now
    // comes from the brain's tick output, not the legacy orbit-and-
    // fire branch that lived inside `build_control_frame`.
    // The EFFECTS-consumer test
    // `spawn_enemy_projectiles_from_brain_actions::tests::*` still
    // covers the projectile spawn shape; brain-side fire-intent
    // generation belongs in the relevant brain backend's tests.
}
