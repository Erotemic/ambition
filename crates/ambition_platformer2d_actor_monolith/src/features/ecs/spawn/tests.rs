//! Tests for the ECS feature spawn paths: authored actors/bosses, dynamic
//! encounter mobs, and mounted-rider archetypes (ADR 0020).

use super::super::brain_builders::{enemy_default_action_set, enemy_default_brain};
use super::super::spawn_actors::spawn_boss_with_overrides_into;
use super::*;
use crate::features::enemies::ArchetypeSpecExt;
use crate::features::{
    ActorAggression, ActorConfig, ActorCooldowns, ActorDisposition, ActorIdentity, ActorIntent,
    AggressionMode, CombatKit,
};
use ambition_characters::actor::{BodyCombat, BodyHealth};
use ambition_characters::brain::{
    ActionSet, ActorControl, Brain, MeleeActionSpec, MoveStyleSpec, StateMachineCfg,
};
use ambition_platformer2d_core as ae;
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
    use ambition_entity_catalog::placements::{
        DamageKind, DamageTeam, HazardRespawn, HazardSpec, PlacementKind, PlacementSchema,
    };
    use ambition_platformer2d_core::Vec2;
    use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;

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
        &Default::default(),
        &roster,
        &boss_catalog,
        crate::features::ActorConstructionContext::new(
            &crate::construction::engine_construction_registry(),
            Default::default(),
        ),
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
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
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
                &Default::default(),
                &roster,
                ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                "test_encounter",
                crate::features::EncounterMobSeed {
                    id: "test_mob".to_string(),
                    character: None,
                    brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                        "medium_striker".into(),
                    ),
                    pos: ae::Vec2::new(100.0, 100.0),
                    size: ae::Vec2::new(20.0, 30.0),
                },
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

/// Regression net: the boss populate function attaches Brain (BossPattern) +
/// ActionSet + ActorControl alongside BossFeature. Pins the
/// parallel-shape invariant. (Bosses are plan rows now, so the recipe calls
/// this same `_into` on an executor-allocated root.)
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
        let root = commands.spawn_empty().id();
        spawn_boss_with_overrides_into(
            &mut commands,
            &crate::boss_encounter::test_boss_catalog(),
            ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
            root,
            &authored,
            &super::super::spawn_actors::BossOverrides::default(),
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
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
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
                &Default::default(),
                &roster,
                ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                "test_encounter",
                crate::features::EncounterMobSeed {
                    id: "test_mob".to_string(),
                    character: None,
                    brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                        "medium_striker".into(),
                    ),
                    pos: ae::Vec2::new(100.0, 100.0),
                    size: ae::Vec2::new(20.0, 30.0),
                },
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
        // A fixture body: unattributed facts are the honest answer here.
        subject: None,
        actor_pos: ae::Vec2::ZERO,
        actor_vel: ae::Vec2::ZERO,
        actor_facing: 1.0,
        control_down: ae::Vec2::new(0.0, 1.0),
        movement_frame_mode: ae::InputFrameMode::BodyRelativeAssist,
        aim_frame_mode: ae::InputFrameMode::ScreenRelative,
        actor_on_ground: true,
        side_contact_normal: None,
        turns_at_walls: false,
        attack_kit: Vec::new(),
        actor_aerial: false,
        alive: true,
        target_pos: ae::Vec2::new(72.0, 0.0),
        target_alive: true,
        health_fraction: 1.0,
        sim_time: 0.0,
        dt: 1.0 / 60.0,
        max_run_speed: 120.0,
        // A fixture body on default tuning: `None` resolves to the engine's
        // canonical movement table.
        movement_tuning: None,
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
/// is never the character's label — `ambition_platformer2d_ldtk` has no catalog
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
            let root = commands.spawn_empty().id();
            super::super::spawn_actors::spawn_interactable_into(
                &mut commands,
                &catalog,
                &Default::default(),
                &roster,
                // No prepared cast in this fixture: the catalog default stands,
                // which is what this test is about.
                &Default::default(),
                SessionSpawnScope::UNSCOPED,
                root,
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

/// **The caller-level guard for D73's identity inversion.**
///
/// `spawn_enemy_with_faction_into` used to reach the prepared character through
/// `config.sprite_character_id`, which `presentation_identity` →
/// `id_for_authored_identity` produces WITH A DISPLAY-NAME FALLBACK. Reading a
/// body's health off that chain infers gameplay identity from presentation
/// identity, which is the arrow the character-template campaign exists to
/// reverse. Nothing in the tree noticed; this module is where it would have.
///
/// See `docs/planning/character-template-architecture-2026-08-10.md`, appendix C.
mod authored_enemy_reads_its_character {
    use super::*;

    /// A roster whose `medium_striker` gives a body 3 HP. Deliberately NOT the
    /// engine default, so "the archetype's pool stood" is distinguishable from
    /// "something wrote an ambient default".
    ///
    /// ⚠ `combatant` is required (the roster panics without it) and is given a
    /// DIFFERENT pool on purpose: `spec_for_brain` silently answers `combatant`
    /// for an unknown key, so identical pools would hide a spawn that never
    /// found `medium_striker` at all.
    fn roster() -> crate::features::CharacterRoster {
        crate::features::CharacterRoster::from_ron(
            r#"{
                "medium_striker": (
                    max_health: 3, run_speed: 0.0, patrol_effort: 0.0, chase_effort: 0.0,
                    aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
                    damage_amount: 0, brain_template: StandStill, move_style: Walk,
                    attacks_player: false, body_contact_damage: false,
                ),
                "combatant": (
                    max_health: 42, run_speed: 0.0, patrol_effort: 0.0, chase_effort: 0.0,
                    aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
                    damage_amount: 0, brain_template: StandStill, move_style: Walk,
                    attacks_player: false, body_contact_damage: false,
                ),
            }"#,
        )
    }

    /// `npc_busy_beaver` is a REAL catalog row with the real display name
    /// "Busy Beaver", authoring 9 HP as a character. Using a real row is what
    /// makes the name-fallback case in the second test reachable at all.
    fn prepared() -> crate::character_runtime::PreparedCharacterRegistry {
        let mut definition = crate::character_runtime::CharacterDefinition::new(
            "npc_busy_beaver",
            "Busy Beaver",
            "test",
        );
        definition.vitals.max_health = Some(9);
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &crate::character_runtime::CharacterBindings::default(),
        );
        let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
        registry.insert_prepared(finalized.prepared);
        registry
    }

    /// A character COMPLETE enough to build a body from: it states how fast it
    /// runs, which is the discriminator `is_complete_body` uses.
    fn prepared_complete() -> crate::character_runtime::PreparedCharacterRegistry {
        let mut definition = crate::character_runtime::CharacterDefinition::new(
            "npc_busy_beaver",
            "Busy Beaver",
            "test",
        )
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 77.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_contact_damage(ambition_characters::actor::ContactDamage {
            strength: 0.25,
            amount: 3,
        });
        definition.vitals.max_health = Some(9);
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &crate::character_runtime::CharacterBindings::default(),
        );
        let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
        registry.insert_prepared(finalized.prepared);
        registry
    }

    /// **A COMPLETE character is BUILT, not patched.**
    ///
    /// ⭐ the difference this asserts is the campaign's central one. Both spawns
    /// below name the same character; the first is half-migrated and receives
    /// the archetype's body with the character's health written over it, and the
    /// second states how it moves and therefore gets a body made of nothing but
    /// itself.
    ///
    /// The tell is a fact the ARCHETYPE authors and the character does not
    /// mention: `medium_striker` says `attacks_player: false`. A patched body
    /// carries that; a built one carries the constructor's own answer. Run speed
    /// is the positive half — 77 px/s exists nowhere but the definition.
    #[test]
    fn a_complete_character_is_built_from_itself_rather_than_from_an_archetype() {
        let (max, speed, contact) = spawn_with(prepared_complete(), Some("npc_busy_beaver"));
        assert_eq!(max, 9, "the character's pool");
        assert_eq!(
            speed, 77.0,
            "the body runs at the archetype's speed, so it was built from the \
             archetype and the character was merely patched over it"
        );
        assert_eq!(
            contact, 3,
            "the character's contact damage did not reach the body"
        );

        // ⚠ the control: the SAME character, minus the locomotion that makes it
        // buildable, still takes the legacy road — and must, or every
        // half-migrated character in the tree becomes a body that cannot move.
        let (max, speed, contact) = spawn_with(prepared(), Some("npc_busy_beaver"));
        assert_eq!(
            max, 9,
            "the character's pool still outranks the archetype's"
        );
        assert_eq!(
            speed, 0.0,
            "the archetype's run speed, which is what it authors"
        );
        assert_eq!(contact, 0);
    }

    /// **A PLACEMENT DECIDES WHEN ITS BODY COMES BACK.**
    ///
    /// ⭐ ADR 0022's rule, finally authorable where it belongs. Respawn is the
    /// one fact in an enemy archetype row that is neither the character's nor
    /// the controller's — the same creature is a permanent casualty in a story
    /// room and a repopulating trash mob in a corridor — and it lived on the row
    /// only because a placement had no field for it.
    ///
    /// ⛔ the reason it became urgent: a MIGRATED character has no row, so its
    /// respawn arrived through the `combatant` fallback. That worked by luck.
    #[test]
    fn a_placement_authors_its_own_respawn_and_outranks_the_archetypes() {
        use ambition_entity_catalog::placements::RespawnPolicy;

        // The fixture archetype says nothing, so it defaults to DeadStaysDead —
        // and the placement asks for the opposite, which is the only way to tell
        // "the placement was read" from "the default stood".
        let authored = spawn_respawn(Some(RespawnPolicy::OnRoomReenter));
        assert_eq!(
            authored,
            RespawnPolicy::OnRoomReenter,
            "the placement's policy did not reach the body"
        );
        let unauthored = spawn_respawn(None);
        assert_eq!(
            unauthored,
            RespawnPolicy::DeadStaysDead,
            "a placement that says nothing must keep the archetype's answer, or \
             every level authored before the field existed changes meaning"
        );
    }

    /// Spawn one enemy and report the respawn policy its body ended up with.
    fn spawn_respawn(
        respawn: Option<ambition_entity_catalog::placements::RespawnPolicy>,
    ) -> ambition_entity_catalog::placements::RespawnPolicy {
        let mut spec = crate::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom(
                "medium_striker".to_string(),
            ),
        );
        spec.respawn = respawn;
        let authored = crate::rooms::Authored::new(
            "EnemySpawn-1",
            "Some Room Enemy",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
            spec,
        );

        let mut app = App::new();
        app.insert_resource(crate::character_roster::catalog());
        app.insert_resource(roster());
        app.insert_resource(prepared());
        app.add_systems(
            Update,
            move |mut commands: Commands,
                  catalog: bevy::prelude::Res<
                ambition_characters::actor::character_catalog::CharacterCatalog,
            >,
                  roster: bevy::prelude::Res<crate::features::CharacterRoster>,
                  prepared: bevy::prelude::Res<
                crate::character_runtime::PreparedCharacterRegistry,
            >| {
                let root = commands.spawn_empty().id();
                crate::features::spawn_enemy_with_faction_into(
                    &mut commands,
                    &catalog,
                    &Default::default(),
                    &roster,
                    &prepared,
                    SessionSpawnScope::UNSCOPED,
                    root,
                    &authored,
                    &[],
                    crate::features::ActorFaction::Enemy,
                );
            },
        );
        app.update();

        let world = app.world_mut();
        let mut q = world.query::<&ActorConfig>();
        q.iter(world).next().expect("one enemy body").tuning.respawn
    }

    /// The two roads, sharing one harness: `(max health, run speed, contact damage)`.
    fn spawn_with(
        prepared: crate::character_runtime::PreparedCharacterRegistry,
        character_id: Option<&'static str>,
    ) -> (i32, f32, i32) {
        let mut spec = crate::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom(
                "medium_striker".to_string(),
            ),
        );
        spec.character_id = character_id.map(ambition_entity_catalog::CharacterId::from);
        let authored = crate::rooms::Authored::new(
            "EnemySpawn-1",
            "Some Room Enemy",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
            spec,
        );

        let mut app = App::new();
        app.insert_resource(crate::character_roster::catalog());
        app.insert_resource(roster());
        app.insert_resource(prepared);
        app.add_systems(
            Update,
            move |mut commands: Commands,
                  catalog: bevy::prelude::Res<
                ambition_characters::actor::character_catalog::CharacterCatalog,
            >,
                  roster: bevy::prelude::Res<crate::features::CharacterRoster>,
                  prepared: bevy::prelude::Res<
                crate::character_runtime::PreparedCharacterRegistry,
            >| {
                let root = commands.spawn_empty().id();
                crate::features::spawn_enemy_with_faction_into(
                    &mut commands,
                    &catalog,
                    &Default::default(),
                    &roster,
                    &prepared,
                    SessionSpawnScope::UNSCOPED,
                    root,
                    &authored,
                    &[],
                    crate::features::ActorFaction::Enemy,
                );
            },
        );
        app.update();

        let world = app.world_mut();
        let mut q = world.query::<(&BodyHealth, &ActorConfig)>();
        let (health, config) = q.iter(world).next().expect("one enemy body");
        (
            health.health.max,
            config.tuning.max_run_speed,
            config.tuning.damage_amount,
        )
    }

    fn spawn(name: &'static str, character_id: Option<&'static str>) -> (i32, Option<String>) {
        let mut spec = crate::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom(
                "medium_striker".to_string(),
            ),
        );
        spec.character_id = character_id.map(ambition_entity_catalog::CharacterId::from);
        let authored = crate::rooms::Authored::new(
            "EnemySpawn-1",
            name,
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
            spec,
        );

        let mut app = App::new();
        app.insert_resource(crate::character_roster::catalog());
        app.insert_resource(roster());
        app.insert_resource(prepared());
        app.add_systems(
            Update,
            move |mut commands: Commands,
                  catalog: bevy::prelude::Res<
                ambition_characters::actor::character_catalog::CharacterCatalog,
            >,
                  roster: bevy::prelude::Res<crate::features::CharacterRoster>,
                  prepared: bevy::prelude::Res<
                crate::character_runtime::PreparedCharacterRegistry,
            >| {
                let root = commands.spawn_empty().id();
                crate::features::spawn_enemy_with_faction_into(
                    &mut commands,
                    &catalog,
                    &Default::default(),
                    &roster,
                    &prepared,
                    SessionSpawnScope::UNSCOPED,
                    root,
                    &authored,
                    &[],
                    crate::features::ActorFaction::Enemy,
                );
            },
        );
        app.update();

        let world = app.world_mut();
        let mut q = world.query::<(&BodyHealth, &ActorConfig)>();
        let (health, config) = q.iter(world).next().expect("one enemy body");
        (health.health.max, config.sprite_character_id.clone())
    }

    #[test]
    fn a_spawn_that_names_a_registered_character_takes_its_health() {
        // The display name is deliberately NOT the character's, so the only
        // route to 9 HP is the authored id.
        let (max, sprite) = spawn("Some Room Enemy", Some("npc_busy_beaver"));
        assert_eq!(
            max, 9,
            "the character authored 9 and it must outrank the archetype's 3"
        );
        assert_eq!(sprite.as_deref(), Some("npc_busy_beaver"));
    }

    /// ⛔⛔ **THE POISON THIS MODULE EXISTS FOR.** Re-wire the lookup back
    /// through `config.sprite_character_id` and this reds with 9, because the
    /// display name DOES resolve to the registered character — which the second
    /// assertion proves rather than assumes. Without that assertion the test
    /// could pass because the name never resolved at all, which would be a
    /// check that cannot fail.
    #[test]
    fn a_spawn_that_only_looks_like_a_character_does_not_become_one() {
        let (max, sprite) = spawn("Busy Beaver", None);
        assert_eq!(
            sprite.as_deref(),
            Some("npc_busy_beaver"),
            "the PRESENTATION join must still resolve the display name, or the \
             gameplay assertion below is vacuous"
        );
        assert_eq!(
            max, 3,
            "it wears the beaver's sheet and it is not a beaver: the archetype's \
             3 must stand. 9 here means gameplay identity is being inferred from \
             presentation identity again"
        );
    }
}
