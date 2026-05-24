use super::*;

#[cfg(test)]
mod conversion_tests {
    use super::*;

    /// Build an NPC with a patrol radius and a player parked far
    /// outside the talk radius — used by the patrol-motion tests so
    /// the AI lands on `Patrol` mode each tick.
    fn world_with_patrolling_npc(patrol_radius: f32) -> (ae::World, NpcRuntime, ae::Player) {
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
        let interactable = ae::Interactable::new(
            id.clone(),
            String::from("Talk"),
            aabb,
            ae::InteractionKind::Npc {
                dialogue_id: Some(id.clone()),
                patrol_radius,
                patrol_path_id: None,
            },
        );
        let npc = NpcRuntime::new(id.clone(), id.clone(), aabb, interactable);
        let player = ae::Player::new_with_abilities(
            ae::Vec2::new(1500.0, 540.0),
            ae::AbilitySet::sandbox_all(),
        );
        (world, npc, player)
    }

    /// Build a brain matching the NPC's authored fields. Convenience
    /// for the conversion tests so each scenario doesn't repeat the
    /// `let mut brain = npc.build_brain();` setup boilerplate.
    fn brain_for(npc: &NpcRuntime) -> crate::brain::Brain {
        npc.build_brain()
    }

    /// Bug the user reported: NPCs floated wherever LDtk placed them
    /// because the runtime didn't tick gravity / collision on them.
    /// Pin: after a few ticks an NPC spawned in mid-air lands on the
    /// floor and `on_ground` flips true.
    #[test]
    fn npc_falls_to_floor_under_gravity() {
        let (world, mut npc, player) = world_with_patrolling_npc(0.0);
        // Lift the NPC into mid-air so gravity has work to do.
        npc.pos.y = 200.0;
        npc.spawn.y = 200.0;
        let mut brain = brain_for(&npc);
        for _ in 0..120 {
            npc.tick_via_brain(&mut brain, &world, player.pos, 0.0, 0.016);
        }
        assert!(npc.on_ground, "NPC must land on the floor under gravity");
        // Body bottom should rest on the floor's top edge (y=600).
        let body_bottom = npc.pos.y + npc.size.y * 0.5;
        assert!(
            (body_bottom - 600.0).abs() < 1.0,
            "expected body bottom near floor top (600); got {body_bottom}"
        );
    }

    /// A patrolling NPC paces left/right around its spawn within
    /// `patrol_radius`. Pin both the motion (NPC moves) and the
    /// bound (NPC reverses before exceeding the radius).
    #[test]
    fn patrolling_npc_paces_within_radius() {
        let (world, mut npc, player) = world_with_patrolling_npc(96.0);
        let mut brain = brain_for(&npc);
        // Settle gravity first so we're testing horizontal motion,
        // not the freefall.
        for _ in 0..30 {
            npc.tick_via_brain(&mut brain, &world, player.pos, 0.0, 0.016);
        }
        let spawn_x = npc.spawn.x;
        let mut min_x = npc.pos.x;
        let mut max_x = npc.pos.x;
        for _ in 0..600 {
            npc.tick_via_brain(&mut brain, &world, player.pos, 0.0, 0.016);
            min_x = min_x.min(npc.pos.x);
            max_x = max_x.max(npc.pos.x);
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
        let mut brain = brain_for(&npc);
        // Settle physics.
        for _ in 0..30 {
            npc.tick_via_brain(&mut brain, &world, player.pos, 0.0, 0.016);
        }
        // Park the player right next to the NPC — within talk_radius.
        player.pos = ae::Vec2::new(npc.pos.x + 30.0, npc.pos.y);
        // Run for a half-second of real time. Whatever momentum was
        // left from the patrol step must drain to ~0 inside the
        // talk radius.
        for _ in 0..30 {
            npc.tick_via_brain(&mut brain, &world, player.pos, 0.0, 0.016);
        }
        assert!(
            matches!(npc.ai_mode, ae::CharacterAiMode::Chase),
            "expected Chase mode (NPC interprets as hold-and-face), got {:?}",
            npc.ai_mode
        );
        assert!(
            npc.vel.x.abs() < 5.0,
            "NPC must come to rest inside talk_radius; got vel.x={}",
            npc.vel.x
        );
        // And the NPC faces the player so the dialog prompt sits on
        // the right side.
        let dx = player.pos.x - npc.pos.x;
        assert_eq!(npc.facing.signum(), dx.signum(), "NPC must face the player");
    }

    /// patrol_radius=0 is the explicit "static NPC" knob — no
    /// motion regardless of how long the simulation runs. Pin so a
    /// future tuning pass that defaults patrol_radius nonzero
    /// doesn't silently move every NPC.
    #[test]
    fn npc_with_zero_patrol_radius_stays_at_spawn_x() {
        let (world, mut npc, player) = world_with_patrolling_npc(0.0);
        let original_x = npc.pos.x;
        let mut brain = brain_for(&npc);
        for _ in 0..300 {
            npc.tick_via_brain(&mut brain, &world, player.pos, 0.0, 0.016);
        }
        assert!(
            (npc.pos.x - original_x).abs() < 1.0,
            "static NPC must not drift; was {}, now {}",
            original_x,
            npc.pos.x
        );
        assert!(matches!(
            npc.ai_mode,
            ae::CharacterAiMode::Idle | ae::CharacterAiMode::Chase
        ));
    }

    /// build_brain() picks Patrol vs StandStill based on the NPC's
    /// authored fields. Pins the spawn-time mapping the actors
    /// system depends on.
    #[test]
    fn npc_build_brain_picks_template_from_authored_fields() {
        let (_, npc_static, _) = world_with_patrolling_npc(0.0);
        match npc_static.build_brain() {
            crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::StandStill) => {}
            other => panic!("expected StandStill for zero-radius NPC, got {:?}", other),
        }
        let (_, npc_patrol, _) = world_with_patrolling_npc(64.0);
        match npc_patrol.build_brain() {
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
        let (_, npc, _) = world_with_patrolling_npc(96.0);
        let brain = npc.build_brain();
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
        let mut brain = brain_for(&npc);
        // Settle gravity.
        for _ in 0..30 {
            npc.tick_via_brain(&mut brain, &world, player.pos, 0.0, 0.016);
        }
        // Run for a long while; track that the AI mode stays in
        // patrol and never wedges in Idle.
        let mut patrol_ticks = 0;
        let mut chase_ticks = 0;
        for _ in 0..300 {
            npc.tick_via_brain(&mut brain, &world, player.pos, 0.0, 0.016);
            match npc.ai_mode {
                ae::CharacterAiMode::Patrol => patrol_ticks += 1,
                ae::CharacterAiMode::Chase => chase_ticks += 1,
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
        let mut brain = brain_for(&npc);
        for _ in 0..120 {
            npc.tick_via_brain(&mut brain, &world, player.pos, 0.0, 0.016);
        }
        // Vel along x should drain to zero (gravity has settled).
        assert!(
            npc.vel.x.abs() < 0.5,
            "static NPC should have ~zero horizontal velocity; got {}",
            npc.vel.x
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
            let brain = ae::EnemyBrain::Custom(name.to_string());
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
            assert!(archetype.chase_speed().is_finite());
            assert!(archetype.aggro_radius().is_finite());
            assert!(archetype.aggro_radius() >= 0.0);
            assert!(archetype.attack_range().is_finite());
            assert!(archetype.attack_range() >= 0.0);
            assert!(archetype.contact_strength().is_finite());
            assert!(archetype.contact_strength() >= 0.0);
            assert!(archetype.damage_amount() > 0);
            if archetype.attacks_player() {
                assert!(
                    archetype.attack_range() > 0.0,
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
            EnemyArchetype::SmallLurker.aggro_radius()
                < EnemyArchetype::SmallSkitter.aggro_radius()
        );
        assert!(
            EnemyArchetype::LargeColossus.aggro_radius()
                < EnemyArchetype::LargeBrute.aggro_radius()
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

    fn enemy_test_world() -> ae::World {
        ae::World::new(
            String::from("enemy_ai_test"),
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(100.0, 100.0),
            vec![ae::Block::solid(
                String::from("floor"),
                ae::Vec2::new(0.0, 560.0),
                ae::Vec2::new(800.0, 40.0),
            )],
        )
    }

    fn enemy_aabb(pos: ae::Vec2) -> ae::Aabb {
        ae::Aabb::new(pos, ae::Vec2::new(14.0, 23.0))
    }

    #[test]
    fn enemy_ai_output_drives_chase_motion() {
        let world = enemy_test_world();
        let aabb = enemy_aabb(ae::Vec2::new(100.0, 536.0));
        let mut enemy = EnemyRuntime::new(
            "skitter",
            "skitter",
            aabb,
            ae::EnemyBrain::Custom("small_skitter".into()),
            &[],
        );
        // Newly spawned enemies carry a short spawn grace cooldown.
        // Clear it here so this test isolates the CharacterAI Chase
        // decision rather than the recovery/spawn-grace state.
        enemy.attack_cooldown = 0.0;
        let mut player = ae::Player::new(ae::Vec2::new(240.0, 536.0));
        player.size = ae::Vec2::new(28.0, 46.0);
        enemy.update(
            &world,
            player.pos,
            FeatureCombatTuning::default(),
            Some(player.pos),
            None,

            0.05,
        );
        assert_eq!(enemy.ai_mode, ae::CharacterAiMode::Chase);
        assert!(
            enemy.vel.x > 0.0,
            "CharacterAI Chase intent should drive rightward motion"
        );
    }

    #[test]
    fn path_enemy_holds_patrol_and_starts_attack_from_character_ai_output() {
        let world = enemy_test_world();
        let aabb = enemy_aabb(ae::Vec2::new(100.0, 536.0));
        let path = ae::KinematicPath {
            points: vec![ae::Vec2::new(100.0, 536.0), ae::Vec2::new(260.0, 536.0)],
            speed: 120.0,
            mode: ae::KinematicPathMode::PingPong,
            start_offset_seconds: 0.0,
        };
        let paths = vec![("skitter_path".to_string(), path)];
        let mut enemy = EnemyRuntime::new(
            "path_skitter",
            "path_skitter",
            aabb,
            ae::EnemyBrain::Patrol {
                path_id: Some("skitter_path".into()),
            },
            &paths,
        );
        // Clear the spawn grace cooldown so the path enemy can enter
        // Telegraph immediately when the player is already in range.
        enemy.attack_cooldown = 0.0;
        let mut player = ae::Player::new(ae::Vec2::new(130.0, 536.0));
        player.size = ae::Vec2::new(28.0, 46.0);
        // Post-actor-brain-migration: `enemy.update()` returns the
        // intent frame; the EFFECTS-stage consumer
        // `start_enemy_melee_from_brain_actions` reads
        // `ActorActionMessage::Melee` and calls `begin_melee_attack`.
        // Here we drive both halves directly to keep the test as a
        // unit test rather than a full-pipeline integration test.
        let frame = enemy.update(
            &world,
            player.pos,
            FeatureCombatTuning::default(),
            Some(player.pos),
            None,
            0.10,
        );
        assert!(
            frame.melee_pressed,
            "intent frame must request a melee when the player is in range"
        );
        let started = enemy.begin_melee_attack(FeatureCombatTuning::default());
        assert!(started, "begin_melee_attack should accept the intent");
        assert_eq!(enemy.ai_mode, ae::CharacterAiMode::Telegraph);
        assert!(enemy.attack_windup_timer > 0.0);
        assert_eq!(
            enemy.pos,
            ae::Vec2::new(100.0, 536.0),
            "path patrol must hold when CharacterAI requests an attack"
        );
    }

    /// `reset_to_spawn` must restore a morphed PirateOnShark back to
    /// its original fused archetype with the right size, gravity,
    /// rider health, and choreography. Same-room reset relies on
    /// this — without it, a dismounted shark stays a grounded
    /// pirate forever even after the player respawns.
    #[test]
    fn reset_to_spawn_restores_morphed_pirate_on_shark() {
        let pos = ae::Vec2::new(400.0, 200.0);
        let aabb = ae::Aabb::new(pos, ae::Vec2::new(14.0, 23.0));
        let mut enemy = EnemyRuntime::new(
            "shark_a",
            "Burning Flying Shark",
            aabb,
            ae::EnemyBrain::Custom("pirate_on_shark".into()),
            &[],
        );
        assert_eq!(enemy.archetype, EnemyArchetype::PirateOnShark);
        assert_eq!(enemy.gravity_scale, 0.0);
        assert!(enemy.rider_health.is_some());
        let spawn_size = enemy.spawn_size;

        // Force a morph: shark dies → grounded pirate.
        // apply_damage_at with a body-overlap hit and damage >= shark hp.
        let body = enemy.aabb();
        // Skip the rider's hitbox by hitting at the bottom of the body.
        let bottom_hit = ae::Aabb::new(
            ae::Vec2::new(enemy.pos.x, enemy.pos.y + 20.0),
            ae::Vec2::new(8.0, 8.0),
        );
        // Damage the shark to death (max_health is 6).
        let outcome = enemy.apply_damage_at(bottom_hit, 99);
        assert!(
            matches!(
                outcome,
                super::super::enemies::EnemyDamageOutcome::Damaged {
                    archetype_changed: true,
                    ..
                }
            ),
            "expected dismount morph, got {outcome:?}"
        );
        assert_eq!(enemy.archetype, EnemyArchetype::PirateRaider);
        assert_eq!(enemy.gravity_scale, 1.0);
        assert!(enemy.rider_health.is_none());
        // After dismount the runtime name must reflect the new
        // archetype so the visual layer's name-based sprite lookup
        // resolves to the pirate sheet (not the shark sheet that
        // matched the spawn name). Without this rename the bug
        // surfaces as "small pirate hitbox drawn as a giant shark".
        assert_eq!(
            enemy.name, "Pirate Raider",
            "dismount_shark must rename so the sprite layer picks pirate art",
        );
        let expected_size = EnemyArchetype::PirateRaider
            .default_size()
            .expect("PirateRaider has a default size");
        assert_eq!(
            enemy.size, expected_size,
            "dismount_shark must shrink the hitbox to pirate-sized",
        );

        // Now reset to spawn — archetype must come back, size + gravity
        // restored, rider_health re-armed, timers cleared.
        enemy.pos = ae::Vec2::new(0.0, 0.0);
        enemy.attack_timer = 0.5;
        enemy.attack_windup_timer = 0.3;
        enemy.hit_flash = 0.2;
        enemy.reset_to_spawn();
        assert_eq!(enemy.archetype, EnemyArchetype::PirateOnShark);
        assert_eq!(enemy.spawn_archetype, EnemyArchetype::PirateOnShark);
        assert_eq!(enemy.gravity_scale, 0.0);
        assert!(enemy.rider_health.is_some());
        assert_eq!(enemy.size, spawn_size);
        assert_eq!(enemy.pos, pos);
        assert!(enemy.alive);
        assert_eq!(enemy.attack_timer, 0.0);
        assert_eq!(enemy.attack_windup_timer, 0.0);
        assert_eq!(enemy.hit_flash, 0.0);
        let _ = body; // silence unused warning if body becomes unused
    }

    /// Aerial enemies (flying shark + rider) used to write `self.pos`
    /// directly from the choreography's steering target, which let
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
        let mut enemy = EnemyRuntime::new(
            "shark_a",
            "Burning Flying Shark",
            aabb,
            ae::EnemyBrain::Custom("pirate_on_shark".into()),
            &[],
        );
        enemy.attack_cooldown = 0.0;
        let player_pos = ae::Vec2::new(500.0, 300.0);
        for _ in 0..120 {
            enemy.update(
                &world,
                player_pos,
                FeatureCombatTuning::default(),
                Some(player_pos),
                None,

                1.0 / 60.0,
            );
        }
        let half_w = enemy.size.x * 0.5;
        let wall_left_edge = 300.0;
        assert!(
            enemy.pos.x + half_w <= wall_left_edge + 0.5,
            "aerial enemy clipped into wall at pos {:?}; wall left edge {}",
            enemy.pos,
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
        let path = ae::KinematicPath {
            points: vec![ae::Vec2::new(100.0, 536.0), ae::Vec2::new(400.0, 536.0)],
            speed: 120.0,
            mode: ae::KinematicPathMode::PingPong,
            start_offset_seconds: 0.0,
        };
        let paths = vec![("skitter_path".to_string(), path)];
        let mut enemy = EnemyRuntime::new(
            "path_skitter",
            "path_skitter",
            aabb,
            ae::EnemyBrain::Patrol {
                path_id: Some("skitter_path".into()),
            },
            &paths,
        );
        enemy.attack_cooldown = 0.0;
        let player_pos_far = ae::Vec2::new(2000.0, 536.0);
        for _ in 0..120 {
            enemy.update(
                &world,
                player_pos_far,
                FeatureCombatTuning::default(),
                Some(player_pos_far),
                None,

                1.0 / 60.0,
            );
        }
        let half_w = enemy.size.x * 0.5;
        let wall_left_edge = 200.0;
        assert!(
            enemy.pos.x + half_w <= wall_left_edge + 0.5,
            "patrol enemy clipped into wall at pos {:?}; wall left edge {}",
            enemy.pos,
            wall_left_edge,
        );
    }

    /// When a PirateOnShark fires, the projectile spawn must:
    ///   1. carry an owner_id with the `lasersword:` prefix so the
    ///      visuals layer renders it with the laser-sword sprite, and
    ///   2. originate from the rider's hand (NOT the enemy centre)
    ///      so the muzzle flash and the projectile track the visible
    ///      gun-sword. The projectile spawn + origin + owner_id are
    ///      pinned in the EFFECTS consumer test
    ///      `spawn_enemy_projectiles_from_brain_actions::tests::*`;
    ///      here we pin the INTENT — the runtime's per-tick frame
    ///      eventually carries `fire = Some(...)`.
    #[test]
    fn pirate_on_shark_fire_intent_lands_on_actor_control_frame() {
        let world = enemy_test_world();
        let aabb = ae::Aabb::new(ae::Vec2::new(300.0, 300.0), ae::Vec2::new(14.0, 23.0));
        let mut enemy = EnemyRuntime::new(
            "shark_a",
            "Burning Flying Shark",
            aabb,
            ae::EnemyBrain::Custom("pirate_on_shark".into()),
            &[],
        );
        assert_eq!(enemy.archetype, EnemyArchetype::PirateOnShark);
        enemy.attack_cooldown = 0.0;
        let player_pos = ae::Vec2::new(500.0, 300.0);
        let mut fire_seen = false;
        // Drive long enough that the orbit-and-fire choreography
        // emits at least one shot (`fire_interval` is 1.4s).
        for _ in 0..200 {
            let frame = enemy.update(
                &world,
                player_pos,
                FeatureCombatTuning::default(),
                Some(player_pos),
                None,

                1.0 / 60.0,
            );
            if let Some(req) = frame.fire {
                // Player is to the right; fire direction should be +x.
                assert!(req.dir.x > 0.0, "fire dir should point at player (+x), got {}", req.dir.x);
                fire_seen = true;
                break;
            }
        }
        assert!(
            fire_seen,
            "no fire intent in 200 ticks — choreography may have changed"
        );
    }
}
