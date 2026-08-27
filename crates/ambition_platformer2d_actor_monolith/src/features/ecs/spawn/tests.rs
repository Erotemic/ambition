//! Tests for the ECS feature spawn paths: authored actors/bosses, dynamic
//! encounter mobs, and mounted-rider archetypes (ADR 0020).

use super::super::brain_builders::enemy_default_brain;
use super::super::spawn_actors::spawn_boss_with_overrides_into;
use super::*;
use ambition_characters::actor::{BodyCombat, BodyHealth};
use ambition_characters::brain::{
    ActionSet, Brain, MeleeActionSpec, MoveStyleSpec, StateMachineCfg,
};
use ambition_characters::control::ActorControl;
use ambition_combat::actor_tuning::ActorConfig;
use ambition_combat::components::{
    ActorAggression, ActorDisposition, ActorIdentity, AggressionMode, CombatKit,
};
use ambition_platformer2d_core as ae;
use bevy::prelude::{App, Commands, Update};

/// A BODY REMEMBERS WHICH CHARACTER IT IS, which is the precondition the
/// character-first provocation branch keys on.
///
/// If a peaceful NPC built by the archetype road did not carry that field, every pirate's
/// authored policy would be dead content and deleting the rows it replaces would return them
/// all to `combatant` — silently, because a generic brawler looks like a working provoke.
///
/// Two terms, both observed: the id survives construction when the placement
/// names one, AND it is the id that was named rather than whatever the display
/// name happened to resolve to.
#[test]
fn a_body_built_from_a_named_character_remembers_which_one() {
    let cast = crate::character_runtime::fixture_cast(&["npc_pirate_quartermaster"]);
    let seed = crate::features::ecs::actor_clusters::ActorClusterSeed::new_character_in(
        &Default::default(),
        &crate::character_roster::catalog(),
        "cove_pirate".to_string(),
        cast.get("npc_pirate_quartermaster")
            .expect("the fixture cast holds the character it was asked for")
            .body_blueprint()
            .expect("a fixture character states everything a body needs"),
        ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
        ambition_entity_catalog::placements::CharacterBrain::Passive,
        &[],
    );
    assert_eq!(
        seed.config.sprite_character_id.as_deref(),
        Some("npc_pirate_quartermaster"),
        "a body built from a named character forgot which one, so provocation \
         cannot ask it what it becomes and falls back to matching its name"
    );
}

/// A body whose CHARACTER declares `template` as its controller policy.
///
/// `make_enemy(brain_key)` stood here and built through an archetype row
/// resolved from the key, so a test named a creature-ish string and got whatever
/// the fixture roster said that creature was. The rows are deleted (AC6) and a
/// body's policy comes from the character that states it — which is what these
/// tests were always really asserting about.
fn body_driven_by(
    template: ambition_characters::brain::CharacterBrainTemplate,
) -> (ActorConfig, ambition_platformer2d_core::AbilitySet) {
    let mut definition =
        crate::character_runtime::CharacterDefinition::new("fixture_body", "Fixture Body", "test")
            .with_locomotion(ambition_characters::actor::CharacterLocomotion {
                run_speed: 155.0,
                move_style: MoveStyleSpec::Walk,
                ..Default::default()
            })
            .with_autonomous_profile(ambition_characters::brain::BrainProfile {
                template,
                aggro_radius: 460.0,
                attack_range: 150.0,
                patrol_effort: 0.6774,
                chase_effort: 1.0,
                ..Default::default()
            });
    definition.vitals.max_health = Some(4);
    let finalized = crate::character_runtime::prepare_and_finalize_for_test(
        definition,
        &crate::character_runtime::CharacterBindings::default(),
    );
    let seed = crate::features::ecs::actor_clusters::ActorClusterSeed::new_character_in(
        &Default::default(),
        &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        "test".to_string(),
        finalized
            .prepared
            .body_blueprint()
            .expect("the fixture character states everything a body needs"),
        ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
        ambition_entity_catalog::placements::CharacterBrain::Custom("fixture_body".to_string()),
        &[],
    );
    let abilities = seed.body.0.abilities.abilities;
    (seed.config, abilities)
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

    let mut room = ambition_platformer2d_world::rooms::RoomSpec::new(
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
    let boss_catalog = ambition_boss_encounter::test_boss_catalog();

    let plan = RoomFeatureConstructionPlan::prepare(
        &room,
        &registry,
        &Default::default(),
        &catalog,
        &Default::default(),
        &boss_catalog,
        crate::features::ActorConstructionContext::new(
            &crate::construction::engine_construction_registry(),
            Default::default(),
        ),
    )
    .expect("the caller-supplied registry should prepare the room");

    let mut app = App::new();
    app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
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

/// The row is deleted (AC6); the claim survives because it was never really about the row: a body's
/// driver is the profile SOMETHING published, and now the only thing that can publish one is a
/// character or a placement.
#[test]
fn encounter_mob_brain_comes_from_its_characters_profile() {
    use ambition_characters::brain::{Brain, StateMachineCfg};
    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    app.add_systems(
        Update,
        |mut commands: Commands,
         catalog: bevy::prelude::Res<
            ambition_characters::actor::character_catalog::CharacterCatalog,
        >| {
            spawn_encounter_mob(
                &mut commands,
                &catalog,
                &Default::default(),
                &smash_fixture_cast(),
                ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                "test_encounter",
                crate::features::EncounterMobSeed {
                    id: "test_mob".to_string(),
                    character: Some("fixture_striker"),
                    brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                        "fixture_striker".into(),
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
    assert!(
        matches!(brain, Brain::StateMachine(StateMachineCfg::Smash { .. })),
        "the character declares a Smash policy and its body is driven by \
         something else"
    );
}

/// A body-complete fixture character that declares a SMASH controller policy —
/// the shape the deleted `medium_striker` row carried.
fn smash_fixture_cast() -> crate::character_runtime::PreparedCharacterRegistry {
    let mut definition = crate::character_runtime::CharacterDefinition::new(
        "fixture_striker",
        "Fixture Striker",
        "test",
    )
    .with_locomotion(ambition_characters::actor::CharacterLocomotion {
        run_speed: 155.0,
        move_style: ambition_characters::brain::MoveStyleSpec::Walk,
        ..Default::default()
    })
    .with_autonomous_profile(ambition_characters::brain::BrainProfile {
        template: ambition_characters::brain::CharacterBrainTemplate::Smash,
        aggro_radius: 460.0,
        attack_range: 150.0,
        patrol_effort: 0.6774,
        chase_effort: 1.0,
        ..Default::default()
    });
    definition.vitals.max_health = Some(4);
    let finalized = crate::character_runtime::prepare_and_finalize_for_test(
        definition,
        &crate::character_runtime::CharacterBindings::default(),
    );
    let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
    registry.insert_prepared(finalized.prepared);
    registry
}

/// Pins the parallel-shape invariant. (Bosses are plan rows now, so the recipe calls this same
/// `_into` on an executor-allocated root.)
#[test]
fn boss_spawn_attaches_brain_components() {
    use ambition_characters::brain::{ActionSet, Brain, StateMachineCfg};
    let mut app = App::new();
    app.add_systems(Update, |mut commands: Commands| {
        let authored = ambition_platformer2d_world::rooms::Authored {
            id: "test_boss".to_string(),
            name: "Test Warden".to_string(),
            aabb: ae::Aabb::new(ae::Vec2::new(200.0, 100.0), ae::Vec2::new(40.0, 50.0)),
            payload: ambition_entity_catalog::placements::BossBrain::Dormant,
        };
        let root = commands.spawn_empty().id();
        spawn_boss_with_overrides_into(
            &mut commands,
            &ambition_boss_encounter::test_boss_catalog(),
            ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
            root,
            &authored,
            &ambition_boss_encounter::BossOverrides::default(),
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
        &CombatKit,
        &ActorAggression,
    )>();
    let (identity, disposition, health, combat, kit, aggression) = shared_q
        .iter(app.world())
        .next()
        .expect("boss shared components");
    assert_eq!(identity.id(), "test_boss");
    assert_eq!(*disposition, ActorDisposition::Hostile);
    assert!(health.alive());
    assert_eq!(
        combat.hit_flash, 0.0,
        "a freshly spawned boss is not blinking"
    );
    assert!(kit.can_ranged(None));
    assert_eq!(aggression.mode, AggressionMode::Hostile);
}

/// Pins the parallel-shape invariant so a future spawn-site refactor can't silently lose the brain.
#[test]
fn encounter_mob_spawns_with_brain_components() {
    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    app.add_systems(
        Update,
        |mut commands: Commands,
         catalog: bevy::prelude::Res<
            ambition_characters::actor::character_catalog::CharacterCatalog,
        >| {
            spawn_encounter_mob(
                &mut commands,
                &catalog,
                &Default::default(),
                &crate::character_runtime::fixture_cast(&["fixture_striker"]),
                ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                "test_encounter",
                crate::features::EncounterMobSeed {
                    id: "test_mob".to_string(),
                    character: Some("fixture_striker"),
                    brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                        "fixture_striker".into(),
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

/// `enemy_default_brain` picks the brain FAMILY its controller policy names.
///
/// The mapping under test was never the creature's — it is `template → StateMachineCfg`, and a
/// character states the template.
#[test]
fn enemy_default_brain_picks_the_family_its_policy_names() {
    use ambition_characters::brain::CharacterBrainTemplate as Template;

    let (sandbag, abilities) = body_driven_by(Template::StandStill);
    assert!(matches!(
        enemy_default_brain(&sandbag, abilities),
        Brain::StateMachine(StateMachineCfg::StandStill)
    ));

    let (diver, abilities) = body_driven_by(Template::ChargeCrash);
    assert!(matches!(
        enemy_default_brain(&diver, abilities),
        Brain::StateMachine(StateMachineCfg::ChargeCrash { .. })
    ));

    let (brute, abilities) = body_driven_by(Template::MeleeBrute);
    assert!(matches!(
        enemy_default_brain(&brute, abilities),
        Brain::StateMachine(StateMachineCfg::MeleeBrute { .. })
    ));

    let (striker, abilities) = body_driven_by(Template::Smash);
    match enemy_default_brain(&striker, abilities) {
        Brain::StateMachine(StateMachineCfg::Smash { cfg, .. }) => {
            assert!(cfg.aggro_radius > 0.0);
            assert!((cfg.chase_speed - 155.0).abs() < 0.01);
        }
        other => panic!("expected Smash for a Smash policy, got {other:?}"),
    }
}

// Each swept `COMBAT_BRAIN_KEYS` or a `fixture_spec(..)` row and asserted that the table's rows
// produced sensible kits — a coverage lint over a table that no longer exists, and whose
// replacement is not another sweep: what a body fights with is authored on its character and
// reaches it through one persona writer, and a character that cannot state a body refuses to
// build one.

/// A body forced hostile must be able to SWING, not merely to approach — the
/// "walks toward you but never swings" state where only movement was made
/// hostile.
#[test]
fn a_body_forced_hostile_swings_when_its_kit_can() {
    let (enemy, abilities) =
        body_driven_by(ambition_characters::brain::CharacterBrainTemplate::MeleeBrute);
    let mut brain = enemy_default_brain(&enemy, abilities);
    match &mut brain {
        Brain::StateMachine(StateMachineCfg::MeleeBrute { cfg, .. }) => {
            cfg.aggressiveness = 1.0;
            cfg.aggro_radius = 500.0;
            cfg.attack_range = 160.0;
        }
        other => panic!("expected a MeleeBrute policy to build a MeleeBrute, got {other:?}"),
    }
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Lunge(
            ambition_characters::brain::action_set::LungeSpec::BRUTE_DEFAULT,
        )),
        ..ActionSet::peaceful()
    };

    let snapshot = ambition_characters::brain::BrainSnapshot {
        captured: false,
        captured_for: 0.0,
        holding_captive: false,
        pummels_landed: 0,
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
    crate::brain_tick::tick_brain_with_actions(&mut brain, &actions, &snapshot, None, &mut frame);
    assert!(
        frame.melee_pressed,
        "a body forced hostile with a melee in its kit should commit a swing \
         when in range",
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

    let authored = ambition_platformer2d_world::rooms::Authored::new(
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

    let spawn = move |mut commands: Commands,
                      catalog: bevy::prelude::Res<
        ambition_characters::actor::character_catalog::CharacterCatalog,
    >| {
        let root = commands.spawn_empty().id();
        super::super::spawn_actors::spawn_interactable_into(
            &mut commands,
            &catalog,
            &Default::default(),
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

/// Guard against inferring gameplay identity from presentation identity.
mod authored_enemy_reads_its_character {
    use super::*;

    // It published two rows with deliberately different pools so "the character's HP won" could
    // be told apart from "the archetype's did" — the comparison this module was built to make.
    // There is no second authority to lose to: a body's pool is its character's, and a
    // placement naming no buildable character is refused rather than handed a generic body with
    // a plausible pool.

    /// `npc_busy_beaver` is a REAL catalog row with the real display name
    /// "Busy Beaver", authoring 9 HP as a character. Using a real row is what
    /// makes the name-fallback case in the second test reachable at all.
    /// A CATALOG-BACKED NPC WEARS ITS CHARACTER, which is what makes
    /// provocation able to ask what it becomes.
    ///
    /// the anonymous case is asserted too. A synthetic placement names no
    /// character, and inventing a worn id to satisfy a lookup would be worse
    /// than the absence.
    #[test]
    fn a_catalog_backed_npc_wears_the_character_its_placement_names() {
        let npc = |character_id: Option<&str>| ambition_interaction::Interactable {
            id: "cove_pirate".to_string(),
            prompt: String::new(),
            aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
            kind: ambition_interaction::InteractionKind::Npc {
                character_id: character_id.map(str::to_string),
                dialogue_id: None,
                brain_override: None,
                patrol_path_id: None,
                patrol_radius: 0.0,
            },
            requires_facing: false,
            enabled: true,
        };
        let named = npc(Some("npc_pirate_quartermaster"));
        assert_eq!(
            crate::features::ecs::spawn_actors::npc_character_id(&named),
            Some("npc_pirate_quartermaster"),
            "a placement that names a character produced no gameplay identity, so \
             provocation cannot ask that creature what it becomes when struck"
        );

        let anonymous = npc(None);
        assert_eq!(
            crate::features::ecs::spawn_actors::npc_character_id(&anonymous),
            None,
            "a placement that names nobody was given an identity anyway"
        );
    }

    /// A CONSTRUCTED CHARACTER IS STAMPED AS APPLIED ON ITS OWN FRAME.
    ///
    /// Construction wrote only `ProjectedCharacterKit`, so a body built COMPLETE still had no
    /// baseline, `stale_cast` was true, and the character was applied a SECOND time on the next
    /// pass.
    ///
    /// this fixture cannot lie about it, which is why it lives here: the
    /// app it builds registers the spawn system and NOTHING ELSE.
    /// `apply_worn_character_gameplay` is not in it, so a `PersonaBaseline` on
    /// the body can only have come from construction.
    ///
    /// and `displaced` must be EMPTY — that is the Construction boundary's
    /// meaning. Nothing was taken from this body; it was built as this character.
    /// A stamp carrying displacements would claim a replacement had happened and
    /// hand a later re-wear something wrong to retract to.
    #[test]
    fn construction_stamps_the_applied_template_so_nothing_reapplies_it() {
        let spec = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom(
                "medium_striker".to_string(),
            ),
            "npc_busy_beaver",
        );
        let authored = ambition_platformer2d_world::rooms::Authored::new(
            "EnemySpawn-1",
            "Some Room Enemy",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
            spec,
        );

        let mut app = App::new();
        app.insert_resource(crate::character_roster::catalog());
        app.insert_resource(prepared_complete());
        app.add_systems(
            Update,
            move |mut commands: Commands,
                  catalog: bevy::prelude::Res<
                ambition_characters::actor::character_catalog::CharacterCatalog,
            >,
                  prepared: bevy::prelude::Res<
                crate::character_runtime::PreparedCharacterRegistry,
            >| {
                let root = commands.spawn_empty().id();
                crate::features::spawn_enemy_with_faction_into(
                    &mut commands,
                    &catalog,
                    &Default::default(),
                    &prepared,
                    &Default::default(),
                    SessionSpawnScope::UNSCOPED,
                    root,
                    &authored,
                    &[],
                    ambition_combat::components::ActorFaction::Enemy,
                );
            },
        );
        app.update();

        let generation = app
            .world()
            .resource::<crate::character_runtime::PreparedCharacterRegistry>()
            .generation();
        let world = app.world_mut();
        let mut q = world.query::<&crate::avatar::PersonaBaseline>();
        let baseline = q
            .iter(world)
            .next()
            .expect(
                "a character-first body carries no applied-template stamp, so the \
                 persona derive will apply the character a SECOND time on its next \
                 pass — the body is not complete, whatever construction granted",
            )
            .clone();
        assert_eq!(baseline.id, "npc_busy_beaver");
        assert_eq!(
            baseline.generation, generation,
            "the stamp names a different cast than the one that built the body, so \
             a hot-reload check reads it as stale immediately"
        );
        assert_eq!(
            baseline.displaced,
            Default::default(),
            "construction recorded a DISPLACEMENT, which claims a replacement \
             happened: a later re-wear would retract to values this body never had"
        );
    }

    /// Initial orientation is carried by the authored occurrence and lands on the authoritative
    /// body before its first controller tick.
    #[test]
    fn a_placement_sets_initial_body_facing_on_the_construction_frame() {
        let mut spec = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom(
                "medium_striker".to_string(),
            ),
            "npc_busy_beaver",
        );
        spec.facing = ambition_platformer2d_world::rooms::SpawnFacing::Left;
        let authored = ambition_platformer2d_world::rooms::Authored::new(
            "EnemySpawn-facing",
            "Left-facing beaver",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
            spec,
        );

        let mut app = App::new();
        app.insert_resource(crate::character_roster::catalog());
        app.insert_resource(prepared_complete());
        app.add_systems(
            Update,
            move |mut commands: Commands,
                  catalog: bevy::prelude::Res<
                ambition_characters::actor::character_catalog::CharacterCatalog,
            >,
                  prepared: bevy::prelude::Res<
                crate::character_runtime::PreparedCharacterRegistry,
            >| {
                let root = commands.spawn_empty().id();
                crate::features::spawn_enemy_with_faction_into(
                    &mut commands,
                    &catalog,
                    &Default::default(),
                    &prepared,
                    &Default::default(),
                    SessionSpawnScope::UNSCOPED,
                    root,
                    &authored,
                    &[],
                    ambition_combat::components::ActorFaction::Enemy,
                );
            },
        );
        app.update();

        let world = app.world_mut();
        let mut q = world.query::<&crate::features::BodyKinematics>();
        let body = q
            .iter(world)
            .next()
            .expect("the authored enemy was not constructed");
        assert!(
            body.facing < 0.0,
            "Left placement facing did not reach BodyKinematics on construction: {}",
            body.facing
        );
    }

    /// THE SAME BODY, TWO PLACEMENTS, TWO DRIVERS.
    ///
    /// Two terms, both observed: the placement's policy REACHES the body's
    /// tuning, and a placement that names nothing keeps the character's own — so
    /// this cannot pass by both answers happening to agree.
    #[test]
    fn a_placement_may_name_the_policy_that_drives_this_body() {
        let profiles = policy_registry();
        let unplaced = spawn_with_placement_policy(None, &profiles);
        let guarding = spawn_with_placement_policy(Some("door_guard"), &profiles);
        assert!(
            (unplaced - 77.0 * 0.5).abs() < 0.01,
            "a placement that names no policy keeps the character's own, whose \
             patrol effort is the 0.5 default: {unplaced}"
        );
        assert!(
            (guarding - 77.0 * 0.1).abs() < 0.01,
            "the PLACEMENT's policy did not reach the body — it patrols at the \
             character's pace instead of the door guard's: {guarding}"
        );
    }

    /// A placement that names a policy nobody published is a REFUSAL.
    ///
    /// the same contract `CharacterDefinition::autonomous_profile_ref`
    /// carries, one authority over: an explicit reference that misses must never
    /// read as silence, or the level says "guard this door" and the body
    /// patrols, with everything green.
    #[test]
    #[should_panic(expected = "is not published")]
    fn a_placement_naming_an_unpublished_policy_is_refused() {
        let profiles = policy_registry();
        let _ = spawn_with_placement_policy(Some("no_such_policy"), &profiles);
    }

    /// One published policy: a door guard that barely ambles.
    fn policy_registry() -> ambition_characters::actor::character_catalog::BrainProfileRegistry {
        use ambition_characters::actor::character_catalog::CharacterCatalog;
        // NAMESPACED, because assembly namespaces every fragment key — a
        // fixture keying the bare local name models a catalog that cannot exist.
        const CATALOG: &str = r#"(
            autonomous_profiles: {
                "test::door_guard": (
                    template: MeleeBrute,
                    aggro_radius: 120.0,
                    attack_range: 40.0,
                    patrol_effort: 0.1,
                ),
            },
            brain_presets: {},
            action_set_presets: {},
            characters: {},
        )"#;
        ambition_characters::actor::character_catalog::BrainProfileRegistry::from_catalog_for_test(
            // The provider these fixtures' own characters name.
            "test",
            &CharacterCatalog::from_data(
                ambition_characters::actor::character_catalog::parse_catalog(CATALOG),
            ),
        )
    }

    /// Spawn the complete character with an optional placement-authored policy,
    /// and report the patrol speed its tuning ended up with.
    fn spawn_with_placement_policy(
        policy: Option<&'static str>,
        profiles: &ambition_characters::actor::character_catalog::BrainProfileRegistry,
    ) -> f32 {
        let mut spec = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom(
                "medium_striker".to_string(),
            ),
            "npc_busy_beaver",
        );
        spec.brain_profile = policy.map(ambition_entity_catalog::BrainProfileRef::new);
        let authored = ambition_platformer2d_world::rooms::Authored::new(
            "EnemySpawn-1",
            "Some Room Enemy",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
            spec,
        );
        let profiles = profiles.clone();

        let mut app = App::new();
        app.insert_resource(crate::character_roster::catalog());
        app.insert_resource(prepared_complete());
        app.add_systems(
            Update,
            move |mut commands: Commands,
                  catalog: bevy::prelude::Res<
                ambition_characters::actor::character_catalog::CharacterCatalog,
            >,
                  prepared: bevy::prelude::Res<
                crate::character_runtime::PreparedCharacterRegistry,
            >| {
                let root = commands.spawn_empty().id();
                crate::features::spawn_enemy_with_faction_into(
                    &mut commands,
                    &catalog,
                    &Default::default(),
                    &prepared,
                    &profiles,
                    SessionSpawnScope::UNSCOPED,
                    root,
                    &authored,
                    &[],
                    ambition_combat::components::ActorFaction::Enemy,
                );
            },
        );
        app.update();

        let world = app.world_mut();
        let mut q = world.query::<&ActorConfig>();
        q.iter(world)
            .next()
            .expect("one enemy body")
            .tuning
            .patrol_speed
    }

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

    /// A CHARACTER-FIRST BODY CARRIES THE WEAPON ITS CHARACTER HOLDS.
    ///
    /// the spawn plan resolves a held item from `enemy.spec`, and a
    /// character-first seed's spec is INERT — so a migrated raider spawned
    /// empty-handed and dropped nothing when it died, which is most of what a
    /// raider is. This is the gap between `pirate_shark_rider` being authorable
    /// and being migratable (its row's `held_item: Some("gun_sword")`).
    ///
    /// the control is the same character with no weapon authored: a body that
    /// holds nothing must carry no `HeldItem` at all, or "authored nothing" and
    /// "authored a weapon" would be the same state.
    #[test]
    fn a_built_body_holds_the_weapon_its_character_authors() {
        assert!(
            ambition_characters::brain::held_item_by_id("gun_sword").is_some(),
            "the fixture names a REGISTERED item, or this test passes on a warning"
        );
        assert_eq!(
            spawn_held_item(Some("gun_sword")).as_deref(),
            Some("gun_sword"),
            "the character's weapon did not reach the body"
        );
        assert_eq!(
            spawn_held_item(None),
            None,
            "a character that authors no weapon must hold nothing"
        );
    }

    /// The held-item id on the body a complete character builds, if any.
    fn spawn_held_item(item: Option<&'static str>) -> Option<String> {
        let mut definition =
            crate::character_runtime::CharacterDefinition::new("npc_raider", "Cove Raider", "test")
                .with_locomotion(ambition_characters::actor::CharacterLocomotion {
                    run_speed: 230.0,
                    move_style: ambition_characters::brain::MoveStyleSpec::Walk,
                    ..Default::default()
                });
        if let Some(item) = item {
            definition = definition.with_held_item(item);
        }
        definition.vitals.max_health = Some(4);
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &crate::character_runtime::CharacterBindings::default(),
        );
        let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
        registry.insert_prepared(finalized.prepared);

        let spec = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom(
                "medium_striker".to_string(),
            ),
            "npc_raider",
        );
        let authored = ambition_platformer2d_world::rooms::Authored::new(
            "EnemySpawn-1",
            "Cove Raider",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
            spec,
        );

        let mut app = App::new();
        app.insert_resource(crate::character_roster::catalog());
        app.insert_resource(registry);
        app.add_systems(
            Update,
            move |mut commands: Commands,
                  catalog: bevy::prelude::Res<
                ambition_characters::actor::character_catalog::CharacterCatalog,
            >,
                  prepared: bevy::prelude::Res<
                crate::character_runtime::PreparedCharacterRegistry,
            >| {
                let root = commands.spawn_empty().id();
                crate::features::spawn_enemy_with_faction_into(
                    &mut commands,
                    &catalog,
                    &Default::default(),
                    &prepared,
                    // These fixtures author no placement-side policy, so an
                    // empty registry is the state they model.
                    &Default::default(),
                    SessionSpawnScope::UNSCOPED,
                    root,
                    &authored,
                    &[],
                    ambition_combat::components::ActorFaction::Enemy,
                );
            },
        );
        app.update();

        let world = app.world_mut();
        let mut q = world.query::<&crate::features::HeldItem>();
        q.iter(world).next().map(|held| held.spec.id.clone())
    }

    /// A complete character is built from its definition rather than patched
    /// onto an archetype body. The assertions distinguish archetype-owned facts
    /// from values authored only by the character definition.
    #[test]
    fn a_complete_character_is_built_from_itself_rather_than_from_an_archetype() {
        let (max, speed, contact) = spawn_with(prepared_complete(), "npc_busy_beaver");
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

        // That road is the body-assist seam, which is gone: every registered character can
        // build its own body, so the state that control modelled cannot occur.
    }

    /// A SPAWN NOTHING CAN BUILD IS REFUSED, LOUDLY.
    ///
    /// authored `character_id = IronMary` must never silently produce a
    /// shark-rider body because Iron Mary was accidentally omitted from some
    /// registration list."*
    ///
    /// the condition WAS "nothing can build it", and is now simply "it cannot be built"
    /// (AC6). The distinction existed because a demo that BORROWS another provider's character
    /// could legitimately run without it, falling back to a roster row that still described the
    /// body — Mary-O's plane swarms were the case.
    ///
    /// What this keeps is the assertion that the enemy road itself does not quietly build something
    /// — a caller reaching it unplanned must still fail loudly.
    #[test]
    #[should_panic(expected = "which this composition has not registered")]
    fn a_spawn_naming_nothing_buildable_is_refused() {
        // A character nobody registered: the body would be a generic
        // `combatant` wearing Iron Mary's name.
        //
        // the registry is NON-EMPTY here on purpose — an empty one is the
        // shape a host with no cast has, and this asserts the content rule
        // rather than that shape.
        spawn_with_prepared_and_brain(prepared(), "iron_mary", "no_such_archetype");
    }
    // It asserted that a composition publishing no characters still built SOMETHING, because
    // refusing *"would take down the multi-game shell over a registry that publishes no
    // characters"*. Every composition in the repository publishes one now; a body with nothing
    // to build it from is a construction error in all of them, which is what makes the road
    // deletable rather than merely unused.

    /// A PLACEMENT DECIDES WHEN ITS BODY COMES BACK.
    ///
    /// ADR 0022's rule, finally authorable where it belongs. Respawn is the
    /// one fact in an enemy archetype row that is neither the character's nor
    /// the controller's — the same creature is a permanent casualty in a story
    /// room and a repopulating trash mob in a corridor — and it lived on the row
    /// only because a placement had no field for it.
    ///
    /// the reason it became urgent: a MIGRATED character has no row, so its
    /// respawn arrived through the `combatant` fallback. That worked by luck.
    #[test]
    fn a_placement_authors_its_own_respawn_and_outranks_the_default() {
        use ambition_entity_catalog::placements::RespawnPolicy;

        // the two answers must DIFFER, or neither assertion can tell "the
        // placement was read" from "the default stood". The unauthored answer
        // is `UNDESCRIBED_BODY_RESPAWN` — the engine's stated policy for a body
        // nobody described — so the placement asks for the opposite.
        let authored = spawn_respawn(Some(RespawnPolicy::DeadStaysDead));
        assert_eq!(
            authored,
            RespawnPolicy::DeadStaysDead,
            "the placement's policy did not reach the body"
        );
        let unauthored = spawn_respawn(None);
        assert_eq!(
            unauthored,
            crate::features::ecs::spawn_actors::UNDESCRIBED_BODY_RESPAWN,
            "a placement that says nothing must take the engine's stated answer \
             for an undescribed body — it used to inherit whatever archetype row \
             its brain key happened to name (AC6)"
        );
    }

    /// Spawn one enemy and report the respawn policy its body ended up with.
    fn spawn_respawn(
        respawn: Option<ambition_entity_catalog::placements::RespawnPolicy>,
    ) -> ambition_entity_catalog::placements::RespawnPolicy {
        let mut spec = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom(
                "medium_striker".to_string(),
            ),
            "npc_busy_beaver",
        );
        spec.respawn = respawn;
        let authored = ambition_platformer2d_world::rooms::Authored::new(
            "EnemySpawn-1",
            "Some Room Enemy",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
            spec,
        );

        let mut app = App::new();
        app.insert_resource(crate::character_roster::catalog());
        // The character must be able to BUILD a body: respawn is a placement
        // fact, and reading it requires a body to write it onto.
        app.insert_resource(prepared_complete());
        app.add_systems(
            Update,
            move |mut commands: Commands,
                  catalog: bevy::prelude::Res<
                ambition_characters::actor::character_catalog::CharacterCatalog,
            >,
                  prepared: bevy::prelude::Res<
                crate::character_runtime::PreparedCharacterRegistry,
            >| {
                let root = commands.spawn_empty().id();
                crate::features::spawn_enemy_with_faction_into(
                    &mut commands,
                    &catalog,
                    &Default::default(),
                    &prepared,
                    // These fixtures author no placement-side policy, so an
                    // empty registry is the state they model.
                    &Default::default(),
                    SessionSpawnScope::UNSCOPED,
                    root,
                    &authored,
                    &[],
                    ambition_combat::components::ActorFaction::Enemy,
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
        character_id: &'static str,
    ) -> (i32, f32, i32) {
        spawn_with_prepared_and_brain(prepared, character_id, "medium_striker")
    }

    fn spawn_with_prepared_and_brain(
        prepared: crate::character_runtime::PreparedCharacterRegistry,
        // was `Option`, and both callers always passed `Some`. The placement
        // type requires a character now, so the axis this could vary no longer
        // exists.
        character_id: &'static str,
        brain_key: &'static str,
    ) -> (i32, f32, i32) {
        let spec = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom(brain_key.to_string()),
            character_id,
        );
        let authored = ambition_platformer2d_world::rooms::Authored::new(
            "EnemySpawn-1",
            "Some Room Enemy",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
            spec,
        );

        let mut app = App::new();
        app.insert_resource(crate::character_roster::catalog());
        app.insert_resource(prepared);
        app.add_systems(
            Update,
            move |mut commands: Commands,
                  catalog: bevy::prelude::Res<
                ambition_characters::actor::character_catalog::CharacterCatalog,
            >,
                  prepared: bevy::prelude::Res<
                crate::character_runtime::PreparedCharacterRegistry,
            >| {
                let root = commands.spawn_empty().id();
                crate::features::spawn_enemy_with_faction_into(
                    &mut commands,
                    &catalog,
                    &Default::default(),
                    &prepared,
                    // These fixtures author no placement-side policy, so an
                    // empty registry is the state they model.
                    &Default::default(),
                    SessionSpawnScope::UNSCOPED,
                    root,
                    &authored,
                    &[],
                    ambition_combat::components::ActorFaction::Enemy,
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

    fn spawn(name: &'static str, character_id: &'static str) -> (i32, Option<String>) {
        let spec = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom(
                "medium_striker".to_string(),
            ),
            character_id,
        );
        let authored = ambition_platformer2d_world::rooms::Authored::new(
            "EnemySpawn-1",
            name,
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0)),
            spec,
        );

        let mut app = App::new();
        app.insert_resource(crate::character_roster::catalog());
        app.insert_resource(prepared_complete());
        app.add_systems(
            Update,
            move |mut commands: Commands,
                  catalog: bevy::prelude::Res<
                ambition_characters::actor::character_catalog::CharacterCatalog,
            >,
                  prepared: bevy::prelude::Res<
                crate::character_runtime::PreparedCharacterRegistry,
            >| {
                let root = commands.spawn_empty().id();
                crate::features::spawn_enemy_with_faction_into(
                    &mut commands,
                    &catalog,
                    &Default::default(),
                    &prepared,
                    // These fixtures author no placement-side policy, so an
                    // empty registry is the state they model.
                    &Default::default(),
                    SessionSpawnScope::UNSCOPED,
                    root,
                    &authored,
                    &[],
                    ambition_combat::components::ActorFaction::Enemy,
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
        //
        // the fixture registers a body-COMPLETE character now (AC5): the
        // health arrives because the body is BUILT from the definition, not
        // because a patch corrected an archetype's. That is what production does
        // — every registered character can build its own body — and a fixture
        // modelling the vanished middle state would pin a road that no longer
        // exists.
        let (max, sprite) = spawn("Some Room Enemy", "npc_busy_beaver");
        assert_eq!(
            max, 9,
            "the character authored 9 and it must outrank the archetype's 3"
        );
        assert_eq!(sprite.as_deref(), Some("npc_busy_beaver"));
    }
    // It spawned a placement whose DISPLAY NAME matched a catalog character and asserted the
    // body kept the archetype's 3 HP rather than the character's 9, because gameplay identity
    // must not be inferred from presentation identity. A body is built from
    // `gameplay_character_id()` — the placement's `character_id` and nothing else — and a
    // placement that names none has no second road to be built on, so the display name cannot
    // supply one.
}
