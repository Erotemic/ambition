use super::*;

#[cfg(test)]
mod conversion_tests {
    use super::*;

    fn world_with_npc(id: &str) -> (ae::World, NpcRuntime) {
        let world = ae::World::new(
            String::from("npc_test"),
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(100.0, 100.0),
            vec![ae::Block::solid(
                String::from("floor"),
                ae::Vec2::new(0.0, 560.0),
                ae::Vec2::new(800.0, 40.0),
            )],
        );
        let aabb = ae::Aabb::new(ae::Vec2::new(200.0, 540.0), ae::Vec2::new(11.0, 19.0));
        let object = ae::RoomObject::new(
            id.to_string(),
            id.to_string(),
            aabb,
            ae::RoomObjectKind::Interactable(ae::Interactable::new(
                id.to_string(),
                String::from("Talk"),
                aabb,
                ae::InteractionKind::Npc {
                    dialogue_id: Some(id.to_string()),
                    patrol_radius: 0.0,
                },
            )),
        );
        let interactable = match object.kind.clone() {
            ae::RoomObjectKind::Interactable(it) => it,
            _ => unreachable!(),
        };
        let npc = NpcRuntime::new(&object, interactable);
        (world, npc)
    }

    #[test]
    fn striking_npc_three_times_flips_them_hostile() {
        let mut features = FeatureRuntime {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: vec![world_with_npc("guide").1],
            switches: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };
        let attack = ae::Aabb::new(ae::Vec2::new(200.0, 540.0), ae::Vec2::new(20.0, 20.0));
        for _ in 0..3 {
            let _ = features.apply_player_attack(attack, 1, 0.0);
        }
        assert_eq!(features.npcs.len(), 1);
        assert!(features.npcs[0].hostile);
    }

    /// Slash damage applies a horizontal AND upward velocity nudge on
    /// enemies (the existing slash-feel signature). Pinning this so a
    /// future change to `apply_damage_event` doesn't silently drop
    /// the slash-only knockback.
    #[test]
    fn slash_source_applies_knockback_to_hit_enemy() {
        let mut features = FeatureRuntime {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: Vec::new(),
            switches: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };
        features.spawn_enemy(
            "victim".into(),
            ae::EnemyBrain::Custom("medium_striker".into()),
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(28.0, 46.0),
        );
        features.enemies[0].vel = ae::Vec2::ZERO;
        let attack = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(40.0, 40.0));
        let report = features.apply_damage_event(&DamageEvent {
            volume: attack,
            damage: 1,
            source: DamageSource::PlayerSlash { knock_x: 300.0 },
            ignored_targets: Vec::new(),
        });
        assert_eq!(report.enemies_hit, 1);
        assert!(report.events.impacts.len() >= 1);
        let enemy = &features.enemies[0];
        assert!(enemy.vel.x > 0.0, "slash knock_x must push enemy right");
        assert!(enemy.vel.y < 0.0, "slash must nudge enemy upward");
    }

    /// Projectile damage source hits the same enemies the slash does
    /// but does NOT apply knockback — the projectile's own visual
    /// motion communicates the impact, and we don't want fireballs
    /// pushing enemies around like a melee swing.
    #[test]
    fn projectile_source_damages_without_knockback() {
        let mut features = FeatureRuntime {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: Vec::new(),
            switches: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };
        features.spawn_enemy(
            "target".into(),
            ae::EnemyBrain::Custom("medium_striker".into()),
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(28.0, 46.0),
        );
        let starting_health = features.enemies[0].health.current;
        features.enemies[0].vel = ae::Vec2::ZERO;
        let volume = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(20.0, 20.0));
        let report = features.apply_damage_event(&DamageEvent {
            volume,
            damage: 1,
            source: DamageSource::PlayerProjectile {
                kind: ae::ProjectileKind::Fireball,
            },
            ignored_targets: Vec::new(),
        });
        assert_eq!(report.enemies_hit, 1);
        assert!(features.enemies[0].health.current < starting_health);
        let enemy = &features.enemies[0];
        assert_eq!(
            enemy.vel,
            ae::Vec2::ZERO,
            "projectile damage must not apply knockback (got {:?})",
            enemy.vel
        );
    }

    /// A wide damage volume that overlaps multiple enemies hits each
    /// of them and the report tallies the count. This is the
    /// future-proof behavior for AOE-style sources.
    #[test]
    fn damage_event_reports_multi_hit_count() {
        let mut features = FeatureRuntime {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: Vec::new(),
            switches: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };
        for (i, x) in [80.0_f32, 120.0, 160.0].iter().enumerate() {
            features.spawn_enemy(
                format!("aoe_target_{i}"),
                ae::EnemyBrain::Custom("medium_striker".into()),
                ae::Vec2::new(*x, 100.0),
                ae::Vec2::new(28.0, 46.0),
            );
        }
        let volume = ae::Aabb::new(ae::Vec2::new(120.0, 100.0), ae::Vec2::new(80.0, 40.0));
        let report = features.apply_damage_event(&DamageEvent {
            volume,
            damage: 1,
            source: DamageSource::PlayerProjectile {
                kind: ae::ProjectileKind::Hadouken,
            },
            ignored_targets: Vec::new(),
        });
        assert_eq!(report.enemies_hit, 3);
        assert!(!report.any_actor_hit() == false);
    }

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
        let object = ae::RoomObject::new(
            id.clone(),
            id.clone(),
            aabb,
            ae::RoomObjectKind::Interactable(ae::Interactable::new(
                id.clone(),
                String::from("Talk"),
                aabb,
                ae::InteractionKind::Npc {
                    dialogue_id: Some(id.clone()),
                    patrol_radius,
                },
            )),
        );
        let interactable = match object.kind.clone() {
            ae::RoomObjectKind::Interactable(it) => it,
            _ => unreachable!(),
        };
        let npc = NpcRuntime::new(&object, interactable);
        let player = ae::Player::new_with_abilities(
            ae::Vec2::new(1500.0, 540.0),
            ae::AbilitySet::sandbox_all(),
        );
        (world, npc, player)
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
        for _ in 0..120 {
            npc.update(&world, &player, 0.016);
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
        // Settle gravity first so we're testing horizontal motion,
        // not the freefall.
        for _ in 0..30 {
            npc.update(&world, &player, 0.016);
        }
        let spawn_x = npc.spawn.x;
        let mut min_x = npc.pos.x;
        let mut max_x = npc.pos.x;
        for _ in 0..600 {
            npc.update(&world, &player, 0.016);
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
        // Settle physics.
        for _ in 0..30 {
            npc.update(&world, &player, 0.016);
        }
        // Park the player right next to the NPC — within talk_radius.
        player.pos = ae::Vec2::new(npc.pos.x + 30.0, npc.pos.y);
        // Run for a half-second of real time. Whatever momentum was
        // left from the patrol step must drain to ~0 inside the
        // talk radius.
        for _ in 0..30 {
            npc.update(&world, &player, 0.016);
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
        for _ in 0..300 {
            npc.update(&world, &player, 0.016);
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

    #[test]
    fn apply_save_with_hostile_flag_replaces_npc_with_enemy() {
        let mut features = FeatureRuntime {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: vec![world_with_npc("guide").1],
            switches: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };
        let mut save = ae::SandboxSaveData::new();
        save.set_flag("npc_guide_hostile", true);
        features.apply_save(&save);
        assert!(features.npcs.is_empty(), "NPC should be removed");
        assert_eq!(features.enemies.len(), 1, "An enemy should replace the NPC");
        assert_eq!(features.enemies[0].id, "guide");
    }

    #[test]
    fn apply_save_with_dead_flag_keeps_npc_dead_no_respawn() {
        let mut features = FeatureRuntime {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: vec![world_with_npc("guide").1],
            switches: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };
        let mut save = ae::SandboxSaveData::new();
        save.set_flag("npc_guide_hostile", true);
        save.set_flag("enemy_guide_dead", true);
        features.apply_save(&save);
        assert!(features.npcs.is_empty(), "NPC was hostile, removed");
        assert!(
            features.enemies.is_empty(),
            "Dead flag should suppress the conversion respawn"
        );
    }

    #[test]
    fn spawn_chest_appends_a_chest_runtime() {
        let mut features = FeatureRuntime {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: Vec::new(),
            switches: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };
        features.spawn_chest(
            "encounter_chest_mob_lab".into(),
            Some(ae::PickupKind::Health { amount: 2 }),
            ae::Vec2::new(400.0, 300.0),
            ae::Vec2::new(28.0, 28.0),
        );
        assert_eq!(features.chests.len(), 1);
        assert_eq!(features.chests[0].id, "encounter_chest_mob_lab");
        // Same id again → no double-spawn.
        features.spawn_chest(
            "encounter_chest_mob_lab".into(),
            None,
            ae::Vec2::new(0.0, 0.0),
            ae::Vec2::new(28.0, 28.0),
        );
        assert_eq!(features.chests.len(), 1, "should not double-spawn");
    }

    #[test]
    fn apply_save_marks_authored_enemy_dead_when_save_says_so() {
        // Authored enemy from a regular EnemySpawn (no encounter
        // prefix). The save flag should mark it dead on load.
        let world = ae::World::new(
            String::from("enemy_test"),
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(100.0, 100.0),
            vec![ae::Block::solid(
                String::from("floor"),
                ae::Vec2::new(0.0, 560.0),
                ae::Vec2::new(800.0, 40.0),
            )],
        )
        .with_objects(vec![ae::RoomObject::new(
            String::from("spider"),
            String::from("spider"),
            ae::Aabb::new(ae::Vec2::new(400.0, 540.0), ae::Vec2::new(11.0, 19.0)),
            ae::RoomObjectKind::EnemySpawn(ae::EnemyBrain::Custom(String::from("medium_striker"))),
        )]);
        let mut features = FeatureRuntime::from_world(&world);
        assert_eq!(features.enemies.len(), 1);
        assert!(features.enemies[0].alive);
        let mut save = ae::SandboxSaveData::new();
        save.set_flag("enemy_spider_dead", true);
        features.apply_save(&save);
        assert!(!features.enemies[0].alive);
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
    #[test]
    fn enemy_archetype_tunings_are_finite() {
        for archetype in EnemyArchetype::COMBAT_ALL {
            assert!(archetype.max_health() > 0);
            assert!(archetype.patrol_speed().is_finite());
            assert!(archetype.chase_speed().is_finite());
            assert!(archetype.aggro_radius().is_finite());
            assert!(archetype.aggro_radius() >= 0.0);
            assert!(archetype.attack_range().is_finite());
            assert!(archetype.attack_range() > 0.0);
            assert!(archetype.contact_strength().is_finite());
            assert!(archetype.contact_strength() > 0.0);
            assert!(archetype.damage_amount() > 0);
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
}
