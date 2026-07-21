//! Tests for the ECS feature spawn paths: authored actors/bosses, dynamic
//! encounter mobs, and mounted-rider archetypes (ADR 0020).

use super::super::brain_builders::{enemy_default_action_set, enemy_default_brain};
use super::super::spawn_actors::spawn_boss;
use super::*;
use crate::features::{
    ActorAggression, ActorConfig, ActorCooldowns, ActorDisposition, ActorIdentity, ActorIntent,
    AggressionMode, CombatKit,
};
use ambition_characters::actor::{BodyCombat, BodyHealth};
use ambition_characters::brain::{
    ActionSet, ActorControl, Brain, MeleeActionSpec, MoveStyleSpec, StateMachineCfg,
};
use ambition_engine_core as ae;
use bevy::prelude::{App, Commands, Update};

fn make_enemy(brain_key: &str) -> ActorConfig {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0));
    crate::features::ecs::actor_clusters::ActorClusterSeed::new(
        "test".to_string(),
        "test".to_string(),
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom(brain_key.to_string()),
        &[],
    )
    .config
}

/// The room-construction choke point lowers authored placements through EXACTLY
/// the registry it is handed — not a locally reconstructed one. This is the
/// invariant behind the setup/reset unification: setup, same-room reset, room
/// transition, and snapshot restore all call
/// `RoomFeatureConstructionPlan` with the ONE installed
/// `PlacementLoweringRegistry`. Here we hand it a registry whose Hazard
/// interpreter is a marker (not the built-in hazard spawn); a hazard placement
/// then yields the marker, which the deleted default-six helper never could.
#[test]
fn room_features_lower_through_the_caller_supplied_registry() {
    use crate::world::placements::{LoweringCtx, PlacementLoweringRegistry, PlacementRecord};
    use ambition_engine_core::Vec2;
    use ambition_entity_catalog::placements::{
        DamageKind, DamageTeam, HazardRespawn, HazardSpec, PlacementKind, PlacementSchema,
    };
    use ambition_platformer_primitives::lifecycle::SessionSpawnScope;

    #[derive(bevy::prelude::Component)]
    struct TestLoweredMarker;

    // A stand-in interpreter that leaves an observable trace only the passed
    // registry could produce.
    fn marker_hazard_lowering(_record: &PlacementRecord, ctx: &mut LoweringCtx<'_, '_, '_>) {
        ctx.commands.spawn(TestLoweredMarker);
    }

    let mut registry = PlacementLoweringRegistry::default();
    registry
        .try_register(
            PlacementKind::Hazard,
            "test",
            "spawn_test",
            "hazard.v1",
            marker_hazard_lowering,
        )
        .unwrap();

    let mut room = crate::rooms::RoomSpec::new(
        "test_room",
        ae::World::new("test_room", Vec2::splat(1000.0), Vec2::ZERO, Vec::new()),
    );
    room.placements.push(PlacementRecord::new(
        "haz_1",
        PlacementSchema::Hazard(HazardSpec {
            damage: 1,
            knockback: [0.0, 0.0],
            kind: DamageKind::Hazard,
            team: DamageTeam::Environment,
            hitstop_seconds: 0.0,
            respawn: HazardRespawn::Never,
            path_id: None,
        }),
        ae::Aabb::new(Vec2::ZERO, Vec2::splat(4.0)),
    ));

    let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::empty();
    let roster = crate::features::enemies::test_roster();
    let boss_catalog = crate::boss_encounter::test_boss_catalog();

    let plan = RoomFeatureConstructionPlan::prepare(
        &room,
        &registry,
        &Default::default(),
        &catalog,
        &roster,
        &boss_catalog,
    )
    .expect("the caller-supplied registry should prepare the room");

    let mut app = App::new();
    app.add_message::<crate::rooms::RoomLoaded>();
    app.add_systems(Update, move |mut commands: Commands| {
        spawn_room_feature_entities_from_plan(&mut commands, &plan, SessionSpawnScope::UNSCOPED);
    });
    app.update();

    let marker_count = app
        .world_mut()
        .query::<&TestLoweredMarker>()
        .iter(app.world())
        .count();
    assert_eq!(
        marker_count, 1,
        "the hazard placement must lower through the supplied registry's marker \
         interpreter — proving the room build uses the registry it is handed"
    );
}

/// Regression net: spawning an encounter mob attaches a
/// per-archetype Brain. `medium_striker` migrated from
/// `MeleeBrute` to `Smash` in `character_archetypes.ron`; the test
/// follows that and pins the Smash variant instead.
#[test]
fn encounter_mob_brain_is_per_archetype_melee_brute() {
    use ambition_characters::brain::{Brain, StateMachineCfg};
    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.insert_resource(crate::features::enemies::test_roster());
    app.add_systems(
        Update,
        |mut commands: Commands,
         catalog: bevy::prelude::Res<
            ambition_characters::actor::character_catalog::CharacterCatalog,
        >,
         roster: bevy::prelude::Res<crate::features::CharacterRoster>| {
            spawn_encounter_mob(
                &mut commands,
                &catalog,
                &roster,
                ambition_platformer_primitives::lifecycle::SessionSpawnScope::UNSCOPED,
                "test_encounter",
                "test_mob".to_string(),
                ambition_entity_catalog::placements::CharacterBrain::Custom(
                    "medium_striker".into(),
                ),
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(20.0, 30.0),
            );
        },
    );
    app.update();
    let mut q = app.world_mut().query::<&Brain>();
    let brain = q.iter(app.world()).next().expect("encounter mob exists");
    // medium_striker is a hostile archetype with Smash brain.
    assert!(matches!(
        brain,
        Brain::StateMachine(StateMachineCfg::Smash { .. })
    ));
}

/// Regression net: spawn_boss attaches Brain (BossPattern) +
/// ActionSet + ActorControl alongside BossFeature. Pins the
/// parallel-shape invariant.
#[test]
fn boss_spawn_attaches_brain_components() {
    use ambition_characters::brain::{ActionSet, ActorControl, Brain, StateMachineCfg};
    let mut app = App::new();
    app.add_systems(Update, |mut commands: Commands| {
        let authored = crate::rooms::Authored {
            id: "test_boss".to_string(),
            name: "Test Warden".to_string(),
            aabb: ae::Aabb::new(ae::Vec2::new(200.0, 100.0), ae::Vec2::new(40.0, 50.0)),
            payload: ambition_entity_catalog::placements::BossBrain::Dormant,
        };
        spawn_boss(
            &mut commands,
            &crate::boss_encounter::test_boss_catalog(),
            ambition_platformer_primitives::lifecycle::SessionSpawnScope::UNSCOPED,
            &authored,
        );
    });
    app.update();
    let mut q = app
        .world_mut()
        .query::<(&Brain, &ActionSet, &ActorControl)>();
    let count = q.iter(app.world()).count();
    assert_eq!(
        count, 1,
        "boss should carry Brain + ActionSet + ActorControl"
    );
    let (brain, action_set, _) = q.iter(app.world()).next().expect("boss exists");
    // Brain is BossPattern with the real encounter id derived
    // from the boss name.
    match brain {
        Brain::StateMachine(StateMachineCfg::BossPattern { cfg, .. }) => {
            assert_eq!(cfg.encounter_id, "test_warden");
        }
        other => panic!("expected BossPattern brain, got {:?}", other),
    }
    // ActionSet carries an ordinary Bolt ranged baseline. Profile-driven boss
    // strikes use the separate per-profile ActorMoveset and BossAttackIntent,
    // so the generic one-slot special route stays empty and cannot double-fire.
    assert!(
        matches!(
            action_set.ranged,
            Some(ambition_characters::brain::RangedActionSpec {
                style: ambition_characters::brain::action_set::RangedStyle::Bolt,
                ..
            })
        ),
        "boss ActionSet should default to Bolt ranged",
    );
    assert!(
        action_set.special.is_none(),
        "boss ActionSet.special should be None — profile-driven boss attacks \
             route through BossAttackIntent and the shared moveset; got {:?}",
        action_set.special,
    );

    let mut shared_q = app.world_mut().query::<(
        &ActorIdentity,
        &ActorDisposition,
        &BodyHealth,
        &BodyCombat,
        &ActorIntent,
        &ActorCooldowns,
        &CombatKit,
        &ActorAggression,
    )>();
    let (identity, disposition, health, combat, intent, cooldowns, kit, aggression) = shared_q
        .iter(app.world())
        .next()
        .expect("boss shared components");
    assert_eq!(identity.id(), "test_boss");
    assert_eq!(*disposition, ActorDisposition::Hostile);
    assert!(health.alive());
    assert!(combat.alive);
    assert_eq!(
        intent.mode(),
        ambition_characters::actor::ai::CharacterAiMode::Chase
    );
    assert_eq!(cooldowns.attack_cooldown, 0.0);
    assert!(kit.can_ranged(None));
    assert_eq!(aggression.mode, AggressionMode::Hostile);
}

/// Regression net: every encounter-spawned hostile actor lands
/// with the universal-brain components attached. Pins the
/// parallel-shape invariant so a future spawn-site refactor
/// can't silently lose the brain.
#[test]
fn encounter_mob_spawns_with_brain_components() {
    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.insert_resource(crate::features::enemies::test_roster());
    app.add_systems(
        Update,
        |mut commands: Commands,
         catalog: bevy::prelude::Res<
            ambition_characters::actor::character_catalog::CharacterCatalog,
        >,
         roster: bevy::prelude::Res<crate::features::CharacterRoster>| {
            spawn_encounter_mob(
                &mut commands,
                &catalog,
                &roster,
                ambition_platformer_primitives::lifecycle::SessionSpawnScope::UNSCOPED,
                "test_encounter",
                "test_mob".to_string(),
                ambition_entity_catalog::placements::CharacterBrain::Custom(
                    "medium_striker".into(),
                ),
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(20.0, 30.0),
            );
        },
    );
    app.update();
    let mut q = app
        .world_mut()
        .query::<(&Brain, &ActionSet, &ActorControl)>();
    let count = q.iter(app.world()).count();
    assert_eq!(
        count, 1,
        "encounter mob should carry Brain + ActionSet + ActorControl"
    );
}

/// enemy_default_brain picks a per-archetype template — pins
/// the mapping so a future refactor that re-keys archetypes
/// can't silently lose the Wanderer/StandStill assignments
/// PuppySlug and the training dummy rely on.
#[test]
fn enemy_default_brain_picks_per_archetype_template() {
    let slug = make_enemy("puppy_slug");
    assert!(matches!(
        enemy_default_brain(&slug),
        Brain::StateMachine(StateMachineCfg::Wanderer { .. })
    ));

    let sandbag = make_enemy("sandbag_infinite");
    assert!(matches!(
        enemy_default_brain(&sandbag),
        Brain::StateMachine(StateMachineCfg::StandStill)
    ));

    let shark = make_enemy("burning_flying_shark");
    assert!(matches!(
        enemy_default_brain(&shark),
        Brain::StateMachine(StateMachineCfg::ChargeCrash { .. })
    ));

    // `MediumStriker` was migrated to the Smash brain template in
    // `character_archetypes.ron` — assert against the live data path
    // rather than reverting to MeleeBrute. The chase_speed pin
    // moves over to the `SmashCfg` row.
    let striker = make_enemy("medium_striker");
    match enemy_default_brain(&striker) {
        Brain::StateMachine(StateMachineCfg::Smash { cfg, .. }) => {
            assert!(cfg.aggro_radius > 0.0);
            assert!(
                (cfg.chase_speed
                    - crate::features::enemies::test_spec("medium_striker")
                        .tuning()
                        .chase_speed)
                    .abs()
                    < 0.01
            );
        }
        other => panic!("expected Smash for MediumStriker, got {:?}", other),
    }
}

/// Coverage lint: every combat brain key maps to a
/// usable Brain (no panic, non-empty per design). Catches a
/// future archetype addition that forgets to update
/// enemy_default_brain.
#[test]
fn enemy_default_brain_covers_every_combat_archetype() {
    for key in crate::features::enemies::COMBAT_BRAIN_KEYS {
        let enemy = make_enemy(key);
        let brain = enemy_default_brain(&enemy);
        // Aggressiveness should match the row's attacks_player.
        // (Wanderer / StandStill / peaceful Patrol all return
        // !is_hostile; everyone else returns is_hostile.)
        assert_eq!(
            brain.is_hostile(),
            crate::features::enemies::test_spec(key).attacks_player,
            "{key} brain.is_hostile mismatch with attacks_player",
        );
    }
}

/// ADR 0020 parity: the mounted rider archetype (`pirate_shark_rider`) carries
/// its own orbit-and-fire kit — a ranged Bolt (the gun-sword) — so that, under a
/// mount's Total control grant, its Skirmisher brain drives the shark's orbit
/// and it fires. The fused `pirate_on_shark` row is gone; the loadout now lives
/// on the standalone rider archetype, spawned as a solo actor linked to the
/// shark by a `mounted_on` ref.
#[test]
fn mounted_rider_archetype_carries_a_ranged_kit() {
    let set = enemy_default_action_set(&crate::features::enemies::test_spec("pirate_shark_rider"));
    assert!(
        set.ranged.is_some(),
        "the shark rider fires a Bolt (gun_sword) — its mounted attack",
    );
    assert!(matches!(set.move_style, MoveStyleSpec::Walk));

    let heavy = enemy_default_action_set(&crate::features::enemies::test_spec(
        "pirate_heavy_shark_rider",
    ));
    assert!(
        heavy.ranged.is_some(),
        "the heavy shark rider also fires a Bolt"
    );
    assert!(matches!(heavy.move_style, MoveStyleSpec::WalkHeavy));
}

/// Coverage lint: every hostile-by-default combat archetype gets at least one
/// offensive ActionSet verb. Peaceful-by-default archetypes may still carry a
/// dormant verb when another system explicitly forces them hostile (PirateHeavy
/// after provocation / dismount); default hostility remains controlled by the
/// brain's aggressiveness, not by stripping the capability out of the ActionSet.
#[test]
fn enemy_default_action_set_covers_every_combat_archetype() {
    for key in crate::features::enemies::COMBAT_BRAIN_KEYS {
        let spec = crate::features::enemies::test_spec(key);
        let set = enemy_default_action_set(&spec);
        if spec.attacks_player {
            assert!(
                set.melee.is_some() || set.ranged.is_some(),
                "{key} attacks_player but ActionSet has no melee or ranged",
            );
        }
    }
}

/// enemy_default_action_set picks a per-archetype concrete
/// attack spec — the EFFECTS consumers read these to spawn
/// distinct hitboxes / projectiles per archetype.
#[test]
fn enemy_default_action_set_picks_per_archetype_specs() {
    let set = enemy_default_action_set(&crate::features::enemies::test_spec("puppy_slug"));
    assert!(set.melee.is_none(), "peaceful PuppySlug has no melee");
    assert!(matches!(set.move_style, MoveStyleSpec::Slither));

    let set = enemy_default_action_set(&crate::features::enemies::test_spec("pirate_heavy"));
    assert!(matches!(set.melee, Some(MeleeActionSpec::Lunge(_))));
    assert!(matches!(set.move_style, MoveStyleSpec::WalkHeavy));

    let set = enemy_default_action_set(&crate::features::enemies::test_spec("large_brute"));
    assert!(matches!(set.melee, Some(MeleeActionSpec::Lunge(_))));
    assert!(matches!(set.move_style, MoveStyleSpec::WalkHeavy));

    let set = enemy_default_action_set(&crate::features::enemies::test_spec("medium_striker"));
    assert!(matches!(set.melee, Some(MeleeActionSpec::Swipe(_))));

    let set = enemy_default_action_set(&crate::features::enemies::test_spec("pirate_shark_rider"));
    assert!(set.ranged.is_some(), "pirate_shark_rider has ranged");
    assert!(matches!(set.move_style, MoveStyleSpec::Walk));
}

/// PirateHeavy is peaceful by default via brain aggressiveness, but once a
/// cove heavy is explicitly provoked the same archetype/action data must be
/// capable of producing a melee request. This prevents the "walks toward you
/// but never swings" state where only movement was made hostile.
#[test]
fn pirate_heavy_action_set_swings_when_brain_is_forced_hostile() {
    let enemy = make_enemy("pirate_heavy");
    let mut brain = enemy_default_brain(&enemy);
    match &mut brain {
        Brain::StateMachine(StateMachineCfg::MeleeBrute { cfg, .. }) => {
            cfg.aggressiveness = 1.0;
            cfg.aggro_radius = 500.0;
            cfg.attack_range = 160.0;
        }
        other => panic!("expected PirateHeavy to use MeleeBrute, got {other:?}"),
    }
    let actions = enemy_default_action_set(&crate::features::enemies::test_spec("pirate_heavy"));
    assert!(matches!(actions.melee, Some(MeleeActionSpec::Lunge(_))));

    let snapshot = ambition_characters::brain::BrainSnapshot {
        actor_pos: ae::Vec2::ZERO,
        actor_vel: ae::Vec2::ZERO,
        actor_facing: 1.0,
        control_down: ae::Vec2::new(0.0, 1.0),
        movement_frame_mode: ae::InputFrameMode::BodyRelativeAssist,
        aim_frame_mode: ae::InputFrameMode::ScreenRelative,
        actor_on_ground: true,
        actor_aerial: false,
        alive: true,
        target_pos: ae::Vec2::new(72.0, 0.0),
        target_alive: true,
        health_fraction: 1.0,
        sim_time: 0.0,
        dt: 1.0 / 60.0,
        max_run_speed: 120.0,
        attack_cooldown_remaining: 0.0,
        attack_windup_remaining: 0.0,
        attack_active_remaining: 0.0,
        attack_recover_remaining: 0.0,
        stun_remaining: 0.0,
        boss_encounter_phase: None,
        world_size: ambition_engine_core::Vec2::ZERO,
        front_wall_clearance: None,
        player_input: None,
        crowding: None,
        terrain: None,
        air_jumps_remaining: 0,
    };
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    brain.tick_with_actions(&actions, &snapshot, None, &mut frame);
    assert!(
        frame.melee_pressed,
        "provoked PirateHeavy should commit a melee swing when in range",
    );
}

/// The Hall pedestal label must be the catalog `display_name`, not the
/// `character_id`.
///
/// Every LDtk `NpcSpawn` shares the identifier "NpcSpawn", so `Authored.name`
/// is never the character's label — `ambition_ldtk_map` has no catalog
/// dependency and cannot resolve one. `spawn_interactable` is the first seam
/// that can, and everything downstream reads the result: nameplates
/// (`ActorIdentity.name` -> `NameplateFact.label`), the interaction banner,
/// the dialogue speaker fallback, speech-SFX keying, and the
/// `id_for_display_name` sprite-size lookup — which silently returns `None`
/// for every catalog character when the label is an id.
///
/// Poisoned deliberately: the authored name here is the literal LDtk
/// identifier and the character_id is a real catalog row, so BOTH degenerate
/// answers ("NpcSpawn" from passing the authored name straight through, and
/// "npc_architect" from substituting the id) fail this assertion.
#[test]
fn authored_npc_takes_its_label_from_the_catalog_display_name() {
    use ambition_entity_catalog::placements::{InteractableSpec, InteractionKindSpec};

    let mut app = App::new();
    app.insert_resource(crate::character_roster::catalog());
    app.insert_resource(crate::features::enemies::CharacterRoster::default());

    let authored = crate::rooms::Authored::new(
        "NpcSpawn-107741",
        // What the LDtk converter actually produces for every NpcSpawn.
        "NpcSpawn",
        ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(32.0, 48.0)),
        InteractableSpec::new(
            "Inspect",
            InteractionKindSpec::Npc {
                character_id: Some("npc_architect".to_string()),
                dialogue_id: Some("hall_architect".to_string()),
                patrol_radius: 0.0,
                patrol_path_id: None,
                brain_override: Some("stand_still".to_string()),
            },
        ),
    );

    let spawn =
        move |mut commands: Commands,
              catalog: bevy::prelude::Res<
            ambition_characters::actor::character_catalog::CharacterCatalog,
        >,
              roster: bevy::prelude::Res<crate::features::enemies::CharacterRoster>| {
            super::super::spawn_actors::spawn_interactable(
                &mut commands,
                &catalog,
                &roster,
                SessionSpawnScope::UNSCOPED,
                &authored,
                &[],
            );
        };
    app.add_systems(Update, spawn);
    app.update();

    let mut q = app.world_mut().query::<&ActorIdentity>();
    let identity = q
        .iter(app.world())
        .next()
        .expect("spawn_interactable should spawn an NPC actor");

    assert_eq!(
        identity.name(),
        "Architect NPC",
        "NPC label should resolve through the catalog; got {:?} (an id or the \
         raw LDtk identifier here means the catalog join was dropped)",
        identity.name(),
    );
}
