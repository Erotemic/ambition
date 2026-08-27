//! The Phase-3 exit criteria, proven against the three real families.
//!
//! The pure planner's own properties are proven in
//! `ambition_platformer2d_shared_tangle::construction`.

use ambition_characters::actor::limb::{Limb, LimbIntents, LimbRig, LimbRouteState, LimbSlot};
use ambition_platformer2d_shared_tangle::construction::{
    ConstructionError, ConstructionPlan, ConstructionScope, SpawnOrigin,
};
use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::{App, Commands, Update, World};

use super::*;
use crate::features::{
    ActorConstructionContext, RoomFeatureConstructionError, RoomFeatureConstructionPlan,
};
use ambition_platformer2d_core as ae;

const REAL_HELD_ITEM: &str = "gun_sword";

fn empty_room(id: &str) -> ambition_platformer2d_world::rooms::RoomSpec {
    ambition_platformer2d_world::rooms::RoomSpec::new(
        id,
        ae::World::new(id, ae::Vec2::splat(1000.0), ae::Vec2::ZERO, Vec::new()),
    )
}

/// THE FIXTURE CAST every construction test builds bodies from.
///
/// `'static` because `ActorConstructionContext` BORROWS the cast, and a
/// per-test local would not outlive the plan it is handed to.
fn fixture_cast() -> &'static crate::character_runtime::PreparedCharacterRegistry {
    static CAST: std::sync::OnceLock<crate::character_runtime::PreparedCharacterRegistry> =
        std::sync::OnceLock::new();
    CAST.get_or_init(|| {
        let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
        for (id, mount) in [
            ("medium_striker", None),
            ("puppy_slug", None),
            ("combatant", None),
            ("fixture_walker", None),
            // The giant's HANDS: `giant_cluster_rows` mints two limb rows naming
            // this character, so a cast without it refuses the whole cluster.
            ("npc_giant_gnu_hands", None),
            // The limbed host: a `"giant"`-class mount lowers to host + two hand
            // rows, and a CHARACTER is the only thing that says so now.
            ("fixture_giant", Some(("giant", &[][..]))),
            // The rideable body and its rider, the ADR 0020 pair.
            ("fixture_mount", Some(("shark", &[][..]))),
        ] {
            let mut definition = crate::character_runtime::CharacterDefinition::new(id, id, "test")
                .with_locomotion(ambition_characters::actor::CharacterLocomotion {
                    run_speed: 155.0,
                    move_style: ambition_characters::brain::MoveStyleSpec::Walk,
                    ..Default::default()
                });
            definition.vitals.max_health = Some(4);
            if let Some((class, pilotable)) = mount {
                definition.mount = Some(ambition_characters::actor::CharacterMount {
                    class: Some(class.to_string()),
                    pilotable_classes: pilotable.iter().map(|c: &&str| (*c).to_string()).collect(),
                    ..Default::default()
                });
            }
            let finalized = crate::character_runtime::prepare_and_finalize_for_test(
                definition,
                &crate::character_runtime::CharacterBindings::default(),
            );
            registry.insert_prepared(finalized.prepared);
        }
        // The RIDER: it pilots a `"shark"`-class mount and is not itself one.
        let mut rider = crate::character_runtime::CharacterDefinition::new(
            "pirate_raider",
            "pirate_raider",
            "test",
        )
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 155.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        });
        rider.vitals.max_health = Some(4);
        rider.mount = Some(ambition_characters::actor::CharacterMount {
            class: None,
            pilotable_classes: vec!["shark".to_string()],
            ..Default::default()
        });
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            rider,
            &crate::character_runtime::CharacterBindings::default(),
        );
        registry.insert_prepared(finalized.prepared);
        registry
    })
}

fn ground_item(id: &str, held_item: &str) -> ambition_platformer2d_world::rooms::GroundItemSpec {
    ambition_platformer2d_world::rooms::GroundItemSpec {
        id: id.to_string(),
        name: format!("{id} display"),
        held_item: held_item.to_string(),
        pos: ae::Vec2::ZERO,
        half_extent: ae::Vec2::splat(8.0),
    }
}

fn staged_enemy(id: &str, grudge_against: Option<&str>) -> SpawnActorRequest {
    SpawnActorRequest {
        id: id.to_string(),
        name: "test_walker".to_string(),
        pos: ae::Vec2::ZERO,
        half_size: ae::Vec2::splat(10.0),
        faction: crate::features::ActorFaction::Npc,
        grudge_against: grudge_against.map(str::to_string),
        kind: SpawnActorKind::Enemy {
            brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                "medium_striker".into(),
            ),
            character: ambition_entity_catalog::CharacterId::from("medium_striker"),
        },
    }
}

/// A room with both planned authored + staged families, staged by a named
/// provider so the resulting provenance is real rather than a placeholder.
fn duelling_room() -> (
    ambition_platformer2d_world::rooms::RoomSpec,
    crate::features::RoomContentStagingRegistry,
) {
    let mut room = empty_room("hall");
    room.ground_items
        .push(ground_item("pickup_a", REAL_HELD_ITEM));
    let mut staging = crate::features::RoomContentStagingRegistry::default();
    staging
        .register("hall", "test_provider", "duel", "duel.v1", |_room| {
            vec![
                staged_enemy("duel_red", Some("duel_blue")),
                staged_enemy("duel_blue", Some("duel_red")),
            ]
        })
        .expect("stager registers");
    (room, staging)
}

fn prepare(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
    staging: &crate::features::RoomContentStagingRegistry,
    recipes: &ActorConstructionRegistry,
) -> Result<RoomFeatureConstructionPlan, RoomFeatureConstructionError> {
    RoomFeatureConstructionPlan::prepare(
        room,
        &Default::default(),
        staging,
        &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        &Default::default(),
        &ambition_boss_encounter::test_boss_catalog(),
        ActorConstructionContext::new(recipes, ae::ContentEpoch(4)).with_prepared(fixture_cast()),
    )
}

/// Commit a prepared room plan into a real `App` and hand back the world.
///
/// Brackets the work with the SAME transaction open/close
/// `RoomConstructionPlan::spawn_contents` uses, because that is where the
/// boundary lives: the feature plan does not publish, and a harness that called
/// it alone would verify nothing.
fn commit(plan: RoomFeatureConstructionPlan) -> App {
    commit_over(plan, |_| {})
}

/// As [`commit`], with `seed` run against the world FIRST
fn commit_over(plan: RoomFeatureConstructionPlan, seed: impl FnOnce(&mut World)) -> App {
    let mut app = App::new();
    app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
    seed(app.world_mut());
    app.add_systems(Update, move |mut commands: Commands| {
        crate::world::rooms::transaction::open(&mut commands);
        let receipt = crate::features::spawn_room_feature_entities_from_plan(
            &mut commands,
            &plan,
            SessionSpawnScope::UNSCOPED,
        );
        crate::world::rooms::transaction::close(
            &mut commands,
            &plan,
            &receipt,
            plan.room().id.clone(),
            SessionSpawnScope::UNSCOPED,
        );
    });
    app.update();
    app
}

// ── All three origins, one planner ───────────────────────────────────────────

/// The authored ground item and the provider-staged actors land in ONE plan, each stating the
/// origin category it actually has.
#[test]
fn a_room_plans_its_authored_and_provider_staged_families_with_real_provenance() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");

    assert_eq!(
        plan.construction().deterministic_dump(),
        "construction-plan-v4\n\
         epoch:4\n\
         room\thall\n\
         lane\tprimary\n\
         entity\tplacement:duel_blue\tambition.staged-actor\tprovider-staged\ttest_provider\thall\tduel_blue\tstaged-actor duel_blue test_walker enemy\n\
         entity\tplacement:duel_red\tambition.staged-actor\tprovider-staged\ttest_provider\thall\tduel_red\tstaged-actor duel_red test_walker enemy\n\
         entity\tplacement:pickup_a\tambition.authored-ground-item\tauthored\thall\tpickup_a\tground-item pickup_a gun_sword\n\
         relation\tplacement:duel_blue\tambition.grudge\tplacement:duel_red\t-\n\
         relation\tplacement:duel_red\tambition.grudge\tplacement:duel_blue\t-\n",
        "the plan states each family's real origin, in canonical order"
    );
}

/// Exit criterion: *planned and committed `SimId` rosters match exactly.*
///
/// Asserted against the WORLD, not just the receipt. The receipt is written by
/// the executor one row at a time, so comparing it to the plan compares the
/// executor's bookkeeping with itself and would stay green even if a recipe
/// built nothing, built something else, or handed back a body that already
/// existed. What the criterion means is that the identities the plan declared
/// are the identities alive afterwards — which only the world can say.
#[test]
fn the_committed_roster_is_exactly_the_planned_roster() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");
    let planned = plan.construction().planned_ids();

    let mut app = App::new();
    app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
    let committed = std::sync::Arc::new(std::sync::Mutex::new(None));
    let sink = committed.clone();
    app.add_systems(Update, move |mut commands: Commands| {
        let receipt = crate::features::spawn_room_feature_entities_from_plan(
            &mut commands,
            &plan,
            SessionSpawnScope::UNSCOPED,
        );
        *sink.lock().unwrap() = Some(receipt.construction().committed_ids());
    });
    app.update();

    let in_world: std::collections::BTreeSet<SimId> = app
        .world_mut()
        .query::<&SimId>()
        .iter(app.world())
        .cloned()
        .collect();
    assert_eq!(
        in_world, planned,
        "every planned identity is alive in the world, and no identity is alive that the plan did \
         not declare"
    );
    assert_eq!(planned.len(), 3);

    // The receipt agrees with the world, so downstream callers may trust it.
    assert_eq!(
        committed
            .lock()
            .unwrap()
            .clone()
            .expect("the plan committed"),
        in_world,
        "the executor's receipt reports what actually reached the world"
    );

    // Each identity is on exactly ONE entity: a recipe that returned a body
    // another row had already claimed would show up here as a short count.
    assert_eq!(
        app.world_mut().query::<&SimId>().iter(app.world()).count(),
        3,
        "three identities on three distinct entities"
    );
}

/// Provenance reaches the live entity, so a restore can read it. Identity does
/// too — both are stamped by the executor rather than by each recipe.
#[test]
fn committed_entities_carry_their_identity_and_provenance() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");
    let mut app = commit(plan);

    let mut found: Vec<(String, String)> = app
        .world_mut()
        .query::<(&SimId, &SpawnOrigin)>()
        .iter(app.world())
        .map(|(id, origin)| (id.as_str().to_string(), origin.canonical_kind().to_string()))
        .collect();
    found.sort();
    assert_eq!(
        found,
        vec![
            (
                "placement:duel_blue".to_string(),
                "provider-staged".to_string()
            ),
            (
                "placement:duel_red".to_string(),
                "provider-staged".to_string()
            ),
            ("placement:pickup_a".to_string(), "authored".to_string()),
        ]
    );
}

#[test]
fn the_staged_duels_mutual_grudge_is_wired_from_the_plan() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");
    let mut app = commit(plan);

    let grudges: Vec<bool> = app
        .world_mut()
        .query::<(
            &crate::features::ActorConfig,
            &crate::features::ActorAggression,
        )>()
        .iter(app.world())
        .filter(|(config, _)| config.id.starts_with("duel_"))
        .map(|(_, aggression)| aggression.grudge.is_some())
        .collect();
    assert_eq!(grudges.len(), 2, "both duellists spawned");
    assert!(
        grudges.iter().all(|has| *has),
        "each duellist holds a grudge against the other"
    );
}


/// Exit criterion: *a failed plan leaves the active world unchanged* — and the
/// specific failure is one that used to be a bare `return` inside the spawner.
#[test]
fn an_authored_ground_item_naming_an_unknown_held_item_fails_the_plan() {
    let recipes = engine_construction_registry();
    let mut room = empty_room("hall");
    room.ground_items
        .push(ground_item("pickup_a", "no_such_item"));

    let error = prepare(&room, &Default::default(), &recipes)
        .expect_err("an unresolvable held item must not plan");
    let RoomFeatureConstructionError::ActorConstruction(ActorConstructionError::UnknownHeldItem(
        unresolved,
    )) = &error
    else {
        panic!("expected the held-item refusal, got {error:?}");
    };
    assert_eq!(unresolved.namespace, "held item");
    assert_eq!(unresolved.id, "no_such_item");
    assert_eq!(unresolved.declared_by, "ground item `pickup_a`");
    // The refusal is the only authority on this defect now, so it is also the
    // one that has to say what the author could have written instead.
    assert!(
        unresolved.available.contains(&"gun_sword".to_owned()),
        "the refusal names what the registry does provide: {unresolved}"
    );
}

/// THE TWO SIDES OF ONE LIST.
///
/// [`reinstatable_authored_requests`] is what a room hands to a NEIGHBOUR that
/// has to rebuild an occurrence lying in it, and [`relocate_request`] is what
/// that neighbour uses to put it back where it was left. A family that joins
/// one without the other is either an occurrence offered up and then built at
/// the wrong coordinates, or a `Placed` row nothing can ever answer — so the
/// pairing is asserted rather than remembered.
#[test]
fn every_reinstatable_record_can_be_relocated() {
    let mut room = empty_room("hall");
    room.ground_items
        .push(ground_item("pickup_a", REAL_HELD_ITEM));
    let requests = reinstatable_authored_requests(&room).expect("the fixture resolves");
    assert!(
        !requests.is_empty(),
        "a fixture that offers nothing to relocate proves nothing about the pairing"
    );

    let somewhere_else = ae::Vec2::new(123.0, 456.0);
    for mut request in requests {
        let sim_id = request.sim_id.clone();
        assert!(
            relocate_request(&mut request, somewhere_else),
            "`{sim_id:?}` is offered for reinstatement but cannot be relocated: a \
             room that owed it would build it at the coordinates its own record \
             names instead of where the occurrence was left",
        );
        // and it MOVED — `relocate_request` answering true while ignoring the
        // position is the same defect wearing the other mask.
        let ActorConstructionParams::GroundItem { spec, .. } = &request.parameters else {
            panic!("a relocated ground item is still a ground item");
        };
        assert_eq!(spec.pos, somewhere_else);
    }
}

/// Poison test for the above: with the item resolvable the SAME room plans and
/// commits, so the rejection is about the held item and not about ground items
/// being unplannable in general.
#[test]
fn the_same_room_plans_once_its_held_item_resolves() {
    let recipes = engine_construction_registry();
    let mut room = empty_room("hall");
    room.ground_items
        .push(ground_item("pickup_a", REAL_HELD_ITEM));
    let plan = prepare(&room, &Default::default(), &recipes).expect("the room plans");
    let mut app = commit(plan);

    let items = app
        .world_mut()
        .query::<&crate::items::pickup::GroundItem>()
        .iter(app.world())
        .count();
    assert_eq!(items, 1, "the authored pickup reached the world");
}

#[test]
fn a_grudge_against_nobody_fails_the_plan() {
    let recipes = engine_construction_registry();
    let mut staging = crate::features::RoomContentStagingRegistry::default();
    staging
        .register("hall", "test_provider", "duel", "duel.v1", |_room| {
            vec![staged_enemy("duel_red", Some("a_fighter_who_is_not_here"))]
        })
        .expect("stager registers");

    let error = prepare(&empty_room("hall"), &staging, &recipes)
        .expect_err("an unresolvable grudge target must not plan");
    assert_eq!(
        error,
        RoomFeatureConstructionError::Construction(ConstructionError::UnresolvedRelation {
            from: SimId::placement("duel_red"),
            kind: relation_grudge(),
            to: SimId::placement("a_fighter_who_is_not_here"),
        })
    );
}

/// Preparation is pure, so a room that fails to plan cannot have half-built
/// itself. This asserts the property directly rather than trusting the type.
#[test]
fn a_rejected_plan_spawns_nothing() {
    let recipes = engine_construction_registry();
    let mut room = empty_room("hall");
    room.ground_items.push(ground_item("ok", REAL_HELD_ITEM));
    room.ground_items.push(ground_item("bad", "no_such_item"));

    let mut app = App::new();
    app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
    let result = prepare(&room, &Default::default(), &recipes);
    assert!(result.is_err(), "the room must not plan");
    app.update();

    let items = app
        .world_mut()
        .query::<&crate::items::pickup::GroundItem>()
        .iter(app.world())
        .count();
    assert_eq!(
        items, 0,
        "the resolvable sibling did not sneak into the world ahead of the failure"
    );
}

// ── One constructor for construction and reconstruction ──────────────────────

/// Exit criterion: *the slice has no separate normal-spawn and reconstruction
/// constructor.* `respawn_authoritative_entity` — the same call a same-room
/// snapshot restore makes — routes a planned family through
/// `ConstructionPlan::construct_one`, producing an entity with the identity and
/// provenance the plan declared, not a bare re-spawn.
#[test]
fn rebuilding_one_planned_entity_reproduces_its_identity_and_provenance() {
    let recipes = engine_construction_registry();
    let mut room = empty_room("hall");
    room.ground_items
        .push(ground_item("pickup_a", REAL_HELD_ITEM));
    let plan = prepare(&room, &Default::default(), &recipes).expect("the room plans");

    let mut app = App::new();
    app.add_systems(Update, move |mut commands: Commands| {
        let rebuilt = plan.respawn_authoritative_entity(
            &mut commands,
            SessionSpawnScope::UNSCOPED,
            "pickup_a",
        );
        assert!(rebuilt, "the planned ground item is rebuildable by id");
    });
    app.update();

    let found: Vec<(String, SpawnOrigin)> = app
        .world_mut()
        .query::<(&SimId, &SpawnOrigin)>()
        .iter(app.world())
        .map(|(id, origin)| (id.as_str().to_string(), origin.clone()))
        .collect();
    assert_eq!(
        found,
        vec![(
            "placement:pickup_a".to_string(),
            SpawnOrigin::Authored {
                source: "hall".into(),
                instance: "pickup_a".into(),
            }
        )],
        "reconstruction produced the planned identity and provenance, not a bare respawn"
    );
}

/// A room plan will not rebuild an id it never planned, rather than quietly
/// doing nothing that looks like success.
#[test]
fn rebuilding_an_unplanned_id_reports_failure() {
    let recipes = engine_construction_registry();
    let plan = prepare(&empty_room("hall"), &Default::default(), &recipes).expect("plans");
    let mut world = World::new();
    let mut commands_queue = world.commands();
    assert!(!plan.respawn_authoritative_entity(
        &mut commands_queue,
        SessionSpawnScope::UNSCOPED,
        "never_authored",
    ));
}

// ── The runtime-dynamic family ───────────────────────────────────────────────

/// A summoned minion gets a dynamic identity under its summoner and a
/// `SpawnOrigin::Dynamic` naming that summoner — the two facts that let it be
/// reconstructed without reading anything out of its id string.
#[test]
fn a_summoned_minion_is_planned_as_a_dynamic_child_of_its_summoner() {
    let recipes = engine_construction_registry();
    let summoner = SimId::placement("boss_1");
    let request = summoned_minion_request(
        &summoner,
        7,
        SummonedMinionParams {
            // A fixture keeps the character's authored vitals.
            health: None,
                    // A boss minion keeps its character's hazard.
                    keeps_contact_damage: true,
            feature_id: "slop_add".into(),
            name: "slop".into(),
            pos: ae::Vec2::ZERO,
            half_size: ae::Vec2::splat(8.0),
            character_id: "puppy_slug".into(),
            encounter_id: "enc_1".into(),
            faction: crate::features::ActorFaction::Enemy,
        },
    );

    let live: std::collections::BTreeSet<SimId> = [summoner.clone()].into_iter().collect();
    let plan = ConstructionPlan::<ActorConstruction>::prepare(
        ConstructionScope {
            binding:
                ambition_platformer2d_shared_tangle::construction::ContentBinding::RuntimeDynamic,
            room: None,
        },
        vec![request],
        &live,
        &recipes,
    )
    .expect("the summon plans");

    let row = plan
        .get(&SimId::spawned(&summoner, 7))
        .expect("the minion is planned under its summoner");
    assert_eq!(row.sim_id().as_str(), "placement:boss_1/7");
    assert_eq!(
        row.origin(),
        &SpawnOrigin::Dynamic {
            parent: summoner.clone(),
            sequence: 7,
        }
    );
    assert_eq!(
        row.origin().parent(),
        Some(&summoner),
        "the parent is readable as data, not recoverable by splitting the id"
    );
}

#[test]
fn two_summons_from_one_summoner_do_not_collide() {
    let recipes = engine_construction_registry();
    let summoner = SimId::placement("boss_1");
    let params = || SummonedMinionParams {
        // A fixture keeps the character's authored vitals.
        health: None,
                    // A boss minion keeps its character's hazard.
                    keeps_contact_damage: true,
        feature_id: "slop_add".into(),
        name: "slop".into(),
        pos: ae::Vec2::ZERO,
        half_size: ae::Vec2::splat(8.0),
        character_id: "puppy_slug".into(),
        encounter_id: "enc_1".into(),
        faction: crate::features::ActorFaction::Enemy,
    };
    let live: std::collections::BTreeSet<SimId> = [summoner.clone()].into_iter().collect();
    let plan = ConstructionPlan::<ActorConstruction>::prepare(
        ConstructionScope {
            binding:
                ambition_platformer2d_shared_tangle::construction::ContentBinding::RuntimeDynamic,
            room: None,
        },
        vec![
            summoned_minion_request(&summoner, 0, params()),
            summoned_minion_request(&summoner, 1, params()),
        ],
        &live,
        &recipes,
    )
    .expect("two summons from one summoner plan");
    assert_eq!(plan.planned_ids().len(), 2);
}

/// A summon whose summoner is not a live identity cannot plan. This is the
/// guard behind `apply_summon_effects` skipping an unidentified spawner: a
/// parentless dynamic id is exactly the ambiguity the origin replaced.
#[test]
fn a_summon_under_an_unknown_summoner_is_rejected() {
    let recipes = engine_construction_registry();
    let summoner = SimId::placement("ghost_boss");
    let error = ConstructionPlan::<ActorConstruction>::prepare(
        ConstructionScope {
            binding:
                ambition_platformer2d_shared_tangle::construction::ContentBinding::RuntimeDynamic,
            room: None,
        },
        vec![summoned_minion_request(
            &summoner,
            0,
            SummonedMinionParams {
                // A fixture keeps the character's authored vitals.
                health: None,
                    // A boss minion keeps its character's hazard.
                    keeps_contact_damage: true,
                feature_id: "slop_add".into(),
                name: "slop".into(),
                pos: ae::Vec2::ZERO,
                half_size: ae::Vec2::splat(8.0),
                character_id: "puppy_slug".into(),
                encounter_id: "enc_1".into(),
                faction: crate::features::ActorFaction::Enemy,
            },
        )],
        &Default::default(),
        &recipes,
    )
    .expect_err("a summon under an unknown summoner must not plan");
    assert_eq!(
        error,
        ConstructionError::UnresolvedParent {
            sim_id: SimId::spawned(&summoner, 0),
            parent: summoner,
        }
    );
}

// ── Determinism over real content ────────────────────────────────────────────

/// The planner sorts, so two stagers registered in either order produce the
/// same room plan. Registration order is a plugin-composition accident and must
/// not reach the world.
#[test]
fn stager_registration_order_does_not_change_the_room_plan() {
    let recipes = engine_construction_registry();
    let room = empty_room("hall");

    let dump_for = |first: bool| {
        let mut staging = crate::features::RoomContentStagingRegistry::default();
        let (a, b): (&str, &str) = if first {
            ("alpha", "beta")
        } else {
            ("beta", "alpha")
        };
        staging
            .register("hall", a, "src", "v1", move |_room| {
                vec![staged_enemy(
                    if a == "alpha" { "red" } else { "blue" },
                    None,
                )]
            })
            .unwrap();
        staging
            .register("hall", b, "src", "v1", move |_room| {
                vec![staged_enemy(
                    if b == "alpha" { "red" } else { "blue" },
                    None,
                )]
            })
            .unwrap();
        prepare(&room, &staging, &recipes)
            .expect("plans")
            .construction()
            .deterministic_dump()
    };

    assert_eq!(dump_for(true), dump_for(false));
}

// ── The summon executor, end to end ──────────────────────────────────────────
//
// It is the only place the runtime-dynamic family actually reaches the world, so a change there
// could otherwise ride a fully green suite.

/// Drive the real `apply_summon_effects` system over one summon request.
fn run_summon(world: &mut World, summoner: Entity, spec: ambition_vfx::SummonSpec) {
    world.write_message(ambition_vfx::EffectRequest {
        owner: summoner,
        effect: ambition_vfx::Effect::Summon(spec),
    });
    world
        .run_system_cached(crate::features::apply_summon_effects)
        .expect("the summon executor runs");
    world.flush();
}

/// Everything `apply_summon_effects` reads, into a world that already exists.
///
/// Separate from [`summon_world`] so a summoner the REAL construction executor
/// built can be handed to the SAME executor — see
/// [`a_boss_the_construction_executor_built_can_summon`].
fn insert_summon_resources(world: &mut World) {
    world.init_resource::<bevy::ecs::message::Messages<ambition_vfx::EffectRequest>>();
    world.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    world.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    world.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    world.insert_resource(engine_construction_registry());
    world.insert_resource(fixture_cast().clone());
}

fn summon_world() -> World {
    let mut world = World::new();
    insert_summon_resources(&mut world);
    world
}

fn summon_spec(id: &str) -> ambition_vfx::SummonSpec {
    ambition_vfx::SummonSpec {
        id: id.to_string(),
        name: "slop".into(),
        pos: ae::Vec2::ZERO,
        half_size: ae::Vec2::splat(8.0),
        character_id: "puppy_slug".into(),
        encounter_id: "enc_1".into(),
        faction: ambition_vfx::HitSide::Enemy,
        // A minion nobody rides, which is every summon these fixtures are about.
        ridden_by_summoner: None,
                    // The sentinel's minions keep the vitals their character authors.
                    health: None,
                    // A boss minion keeps its character's hazard.
                    keeps_contact_damage: true,
    }
}

/// The summoner is built by the REAL construction executor, not by hand.
///
/// Every other test in this section spawns it as
/// `(SimId::placement(..), SimIdCounter::default())` — the fixture hand-supplying
/// exactly the component the shipped path omitted. So the section was thorough
/// about the executor and green over a DEAD FEATURE: the gradient sentinel's
/// Minima Trap summons a "Puppy Slug", and on a construction-built boss
/// `apply_summon_effects` read `counter=None`, warned, and built nothing.
///
/// the general rule this pins: *a test that constructs its subject's
/// preconditions by hand cannot detect that production never establishes them.*
/// The only assertion that can is one whose subject came out of the shipped
/// builder — here `RoomFeatureConstructionPlan::prepare` →
/// `spawn_room_feature_entities_from_plan` → `ConstructionPlan::commit_entity`,
/// which is the one and only path an authored boss reaches the world by.
#[test]
fn a_boss_the_construction_executor_built_can_summon() {
    let mut room = empty_room("hall");
    room.boss_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
        "warden",
        "clockwork_warden",
        ae::Aabb::new(ae::Vec2::new(100.0, 20.0), ae::Vec2::splat(30.0)),
        ambition_entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "clockwork_warden".into(),
        },
    ));
    let plan = prepare(
        &room,
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the boss room plans");

    let mut app = commit(plan);
    insert_summon_resources(app.world_mut());
    let world = app.world_mut();

    let boss = {
        let mut query = world.query::<(Entity, &SimId)>();
        query
            .iter(world)
            .find(|(_, sim)| sim.as_str() == "placement:warden")
            .map(|(entity, _)| entity)
            .expect("the construction executor built the boss")
    };

    run_summon(world, boss, summon_spec("minima_trap_add"));

    let mut minions = world.query::<&SimId>();
    let minted: Vec<String> = minions
        .iter(world)
        .map(|id| id.as_str().to_string())
        .filter(|id| id.starts_with("placement:warden/"))
        .collect();
    assert_eq!(
        minted,
        vec!["placement:warden/0".to_string()],
        "a boss the construction executor built can summon: its identity comes \
         with the counter its descendants' identities are minted from"
    );
}

/// A real summon reaches the world with a dynamic identity under its summoner
/// and provenance naming that summoner.
#[test]
fn a_summoned_minion_reaches_the_world_as_a_dynamic_child() {
    let mut world = summon_world();
    let boss = world
        .spawn((
            SimId::placement("boss_1"),
            ambition_platformer2d_shared_tangle::sim_id::SimIdCounter::default(),
        ))
        .id();

    run_summon(&mut world, boss, summon_spec("slop_add"));

    let mut minions = world.query::<(&SimId, &SpawnOrigin)>();
    let found: Vec<(String, SpawnOrigin)> = minions
        .iter(&world)
        .filter(|(id, _)| id.as_str() != "placement:boss_1")
        .map(|(id, origin)| (id.as_str().to_string(), origin.clone()))
        .collect();
    assert_eq!(
        found,
        vec![(
            "placement:boss_1/0".to_string(),
            SpawnOrigin::Dynamic {
                parent: SimId::placement("boss_1"),
                sequence: 0,
            }
        )],
        "the minion is a dynamic child of its summoner, not an authored placement"
    );
}

/// Two summons in one batch take successive sequence numbers from the
/// summoner's own counter — the per-spawner stream N3.1 requires — rather than
/// colliding on one authored id.
#[test]
fn successive_summons_advance_the_summoners_own_counter() {
    let mut world = summon_world();
    let boss = world
        .spawn((
            SimId::placement("boss_1"),
            ambition_platformer2d_shared_tangle::sim_id::SimIdCounter::default(),
        ))
        .id();

    run_summon(&mut world, boss, summon_spec("slop_add"));
    run_summon(&mut world, boss, summon_spec("slop_add"));

    let mut ids = world.query::<&SimId>();
    let mut seen: Vec<String> = ids
        .iter(&world)
        .map(|id| id.as_str().to_string())
        .filter(|id| id != "placement:boss_1")
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec!["placement:boss_1/0", "placement:boss_1/1"],
        "the same authored summon id twice yields two distinct identities"
    );
    assert_eq!(
        world
            .get::<ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>(boss)
            .map(|counter| counter.0),
        Some(2),
        "minting advanced the summoner's snapshot-visible counter"
    );
}

/// An emitter with no simulation identity cannot lend one, so its summon is
/// refused rather than given a parentless dynamic id. This is a deliberate
/// behaviour change and is pinned so it cannot regress silently in either
/// direction.
#[test]
fn a_summon_from_an_unidentified_emitter_is_refused() {
    let mut world = summon_world();
    let anonymous = world.spawn_empty().id();

    run_summon(&mut world, anonymous, summon_spec("slop_add"));

    let mut ids = world.query::<&SimId>();
    assert_eq!(
        ids.iter(&world).count(),
        0,
        "nothing was spawned for an emitter that cannot be descended from"
    );
}

// ── Partial reconstruction of a real family ──────────────────────────────────

/// The duellists' grudge is a planned relation, so rebuilding one of them alone
/// would put the fighter back without it — a body that looks right in the roster
/// and no longer hunts its rival. `respawn_authoritative_entity` rebuilds the
/// RELATION CLOSURE, so asking for one duellist rebuilds both and the grudge
/// is wired.
///
/// Rebuilding the closure is the better contract: it is the only way to bring a related row back
/// correctly, and it is exactly what the giant host + hands need. Both duellists come back with the
/// grudge intact.
#[test]
fn rebuilding_one_duellist_rebuilds_its_grudge_partner_too() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");

    let mut app = App::new();
    app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
    let outcome = std::sync::Arc::new(std::sync::Mutex::new(None));
    let sink = outcome.clone();
    app.add_systems(Update, move |mut commands: Commands| {
        *sink.lock().unwrap() = Some(plan.respawn_authoritative_entity(
            &mut commands,
            SessionSpawnScope::UNSCOPED,
            "duel_red",
        ));
    });
    app.update();

    assert_eq!(
        *outcome.lock().unwrap(),
        Some(true),
        "asking for one duellist rebuilds its grudge closure"
    );
    // Both duellists came back, and the grudge is wired between them.
    let ids: std::collections::BTreeSet<String> = app
        .world_mut()
        .query::<&SimId>()
        .iter(app.world())
        .map(|id| id.as_str().to_owned())
        .collect();
    assert!(
        ids.contains("placement:duel_red") && ids.contains("placement:duel_blue"),
        "the whole grudge closure was rebuilt: {ids:?}"
    );
}

/// The refusal is specific to relation-bearing rows, not a blanket ban on
/// single-entity rebuilds: the authored pickup in the same plan still rebuilds
/// on its own, which is what the same-room restore path depends on.
#[test]
fn a_relation_free_row_in_the_same_plan_still_rebuilds_alone() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");

    let mut app = App::new();
    app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
    let outcome = std::sync::Arc::new(std::sync::Mutex::new(None));
    let sink = outcome.clone();
    app.add_systems(Update, move |mut commands: Commands| {
        *sink.lock().unwrap() = Some(plan.respawn_authoritative_entity(
            &mut commands,
            SessionSpawnScope::UNSCOPED,
            "pickup_a",
        ));
    });
    app.update();

    assert_eq!(*outcome.lock().unwrap(), Some(true));
    let ids: Vec<String> = app
        .world_mut()
        .query::<&SimId>()
        .iter(app.world())
        .map(|id| id.as_str().to_string())
        .collect();
    assert_eq!(ids, vec!["placement:pickup_a".to_string()]);
}

/// Every parameter variant reaches a construction arm and produces its root.
///
/// The recipe is derived from the payload and construction is one exhaustive match, so a variant
/// with no arm is a compile error rather than a mid-commit panic — but "every arm actually builds
/// something" is still a behavioural claim, and this is it. A new `ActorConstructionParams` variant
/// that is planned but forgotten here shows up as a missing identity, not as a green suite.
#[test]
fn every_parameter_variant_constructs_its_root() {
    let recipes = engine_construction_registry();
    let summoner = SimId::placement("boss_1");

    let requests = vec![
        authored_ground_item_requests(&{
            let mut room = empty_room("hall");
            room.ground_items
                .push(ground_item("pickup", REAL_HELD_ITEM));
            room
        })
        .expect("the ground item resolves")
        .pop()
        .expect("one request"),
        staged_actor_requests(
            "hall",
            "prov",
            &[staged_enemy("staged", None)],
            Some(fixture_cast()),
        )
        .pop()
        .expect("one request"),
        summoned_minion_request(
            &summoner,
            0,
            SummonedMinionParams {
                // A fixture keeps the character's authored vitals.
                health: None,
                    // A boss minion keeps its character's hazard.
                    keeps_contact_damage: true,
                feature_id: "slop".into(),
                name: "slop".into(),
                pos: ae::Vec2::ZERO,
                half_size: ae::Vec2::splat(8.0),
                character_id: "puppy_slug".into(),
                encounter_id: "enc".into(),
                faction: crate::features::ActorFaction::Enemy,
            },
        ),
    ];
    assert_eq!(
        requests.len(),
        3,
        "one request per ActorConstructionParams variant"
    );

    let live: std::collections::BTreeSet<SimId> = [summoner].into_iter().collect();
    let plan = ConstructionPlan::<ActorConstruction>::prepare(
        ConstructionScope {
            binding:
                ambition_platformer2d_shared_tangle::construction::ContentBinding::RuntimeDynamic,
            room: None,
        },
        requests,
        &live,
        &recipes,
    )
    .expect("every variant plans");

    let mut world = World::new();
    world.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    world.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    let services = ActorConstructionServices {
        context: crate::world::placements::ActorPlacementContext::new(
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &Default::default(),
        )
        .with_prepared(fixture_cast()),
        boss_catalog: ambition_boss_encounter::test_boss_catalog().clone(),
    };
    let planned = plan.planned_ids();
    {
        let mut commands = world.commands();
        let scope = plan.scope().clone();
        let mut ctx = ambition_platformer2d_shared_tangle::construction::ConstructionExecCtx {
            commands: &mut commands,
            scope: &scope,
            session: SessionSpawnScope::UNSCOPED,
            services: &services,
        };
        plan.commit(&mut ctx);
    }
    world.flush();

    let in_world: std::collections::BTreeSet<SimId> =
        world.query::<&SimId>().iter(&world).cloned().collect();
    assert_eq!(
        in_world, planned,
        "all three variants built exactly their planned roots"
    );
}

// ── Summon counter preconditions ─────────────────────────────────────────────

/// Reserving is not spending. A batch that cannot plan leaves the counter where
/// it found it, and the very next summon takes the identity the refused batch
/// had reserved.
///
/// Demonstrated against the pre-repair implementation (which called
/// `counter.next()` while assembling requests): it failed there with `Some(1)`
/// where the contract requires `Some(0)`.
#[test]
fn a_rejected_summon_batch_spends_no_identity() {
    use ambition_platformer2d_shared_tangle::sim_id::SimIdCounter;

    let mut world = summon_world();
    let boss = world
        .spawn((SimId::placement("boss_1"), SimIdCounter::default()))
        .id();
    // Squat the identity this summon would take, so preparation refuses it.
    let squatter = world
        .spawn(SimId::from_snapshot("placement:boss_1/0".to_string()))
        .id();

    run_summon(&mut world, boss, summon_spec("slop_add"));

    assert_eq!(
        world.get::<SimIdCounter>(boss).map(|counter| counter.0),
        Some(0),
        "a refused batch leaves the counter exactly where it found it"
    );

    world.despawn(squatter);
    run_summon(&mut world, boss, summon_spec("slop_add"));
    assert_eq!(
        world.get::<SimIdCounter>(boss).map(|counter| counter.0),
        Some(1),
        "the retried summon took the sequence the refused batch had reserved"
    );
}

/// the state is now UNREACHABLE by ordinary means: `SimId` requires
/// `SimIdCounter`, so an identified entity is born able to be descended from.
/// It is stripped explicitly here because the refusal is still the correct
/// answer if something ever tears the pair apart, and a branch nothing can enter
/// is a branch nothing checks.
#[test]
fn a_summoner_without_a_counter_is_refused_before_spawning() {
    let mut world = summon_world();
    // Identified, but carrying no counter to reserve from.
    let boss = world.spawn(SimId::placement("boss_1")).id();
    world
        .entity_mut(boss)
        .remove::<ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>();

    run_summon(&mut world, boss, summon_spec("slop_add"));

    let built = world
        .query::<&SimId>()
        .iter(&world)
        .filter(|id| id.as_str().starts_with("placement:boss_1/"))
        .count();
    assert_eq!(built, 0, "nothing was built for an unreservable summoner");
    let _ = boss;
}

/// One successful batch advances the counter exactly once per summon, and the
/// identities it hands out do not overlap.
#[test]
fn successive_summons_allocate_non_overlapping_identities() {
    use ambition_platformer2d_shared_tangle::sim_id::SimIdCounter;

    let mut world = summon_world();
    let boss = world
        .spawn((SimId::placement("boss_1"), SimIdCounter::default()))
        .id();

    // Two summons in ONE batch: the reservation advances within the batch.
    world.write_message(ambition_vfx::EffectRequest {
        owner: boss,
        effect: ambition_vfx::Effect::Summon(summon_spec("a")),
    });
    world.write_message(ambition_vfx::EffectRequest {
        owner: boss,
        effect: ambition_vfx::Effect::Summon(summon_spec("b")),
    });
    world
        .run_system_cached(crate::features::apply_summon_effects)
        .expect("the summon executor runs");
    world.flush();

    let mut minted: Vec<String> = world
        .query::<&SimId>()
        .iter(&world)
        .map(|id| id.as_str().to_string())
        .filter(|id| id.starts_with("placement:boss_1/"))
        .collect();
    minted.sort();
    assert_eq!(
        minted,
        vec![
            "placement:boss_1/0".to_string(),
            "placement:boss_1/1".to_string()
        ],
        "two summons in one batch take distinct successive identities"
    );
    assert_eq!(
        world.get::<SimIdCounter>(boss).map(|counter| counter.0),
        Some(2),
        "the counter advanced exactly once per summon, not once per batch"
    );
}

// ── Recipe descriptor and execution cannot drift ─────────────────────────────

/// Every parameter variant reports the recipe descriptor it is supposed to, AND
/// constructs successfully through that same descriptor.
///
/// One exhaustive `dispatch` yields both the identity and the executor, so they
/// are chosen in the same arm. This asserts the pairing per variant so a future
/// arm that names one recipe and calls another's code is caught behaviourally
/// rather than only by reading.
#[test]
fn every_parameter_variant_matches_its_descriptor() {
    use ambition_platformer2d_shared_tangle::construction::ConstructionDomain;

    let mut room = empty_room("hall");
    room.ground_items
        .push(ground_item("pickup", REAL_HELD_ITEM));
    let ground = authored_ground_item_requests(&room)
        .expect("resolves")
        .pop()
        .expect("one request");
    let staged = staged_actor_requests(
        "hall",
        "prov",
        &[staged_enemy("staged", None)],
        Some(fixture_cast()),
    )
    .pop()
    .expect("one request");
    let summoned = summoned_minion_request(
        &SimId::placement("boss_1"),
        0,
        SummonedMinionParams {
            // A fixture keeps the character's authored vitals.
            health: None,
                    // A boss minion keeps its character's hazard.
                    keeps_contact_damage: true,
            feature_id: "slop".into(),
            name: "slop".into(),
            pos: ae::Vec2::ZERO,
            half_size: ae::Vec2::splat(8.0),
            character_id: "puppy_slug".into(),
            encounter_id: "enc".into(),
            faction: crate::features::ActorFaction::Enemy,
        },
    );

    for (params, expected) in [
        (&ground.parameters, recipe_authored_ground_item()),
        (&staged.parameters, recipe_staged_actor()),
        (&summoned.parameters, recipe_summoned_minion()),
    ] {
        assert_eq!(
            ActorConstruction::dispatch(params).recipe,
            expected,
            "each variant reports its own recipe identity"
        );
    }
}

/// The window is real here, not simulated: `apply_summon_effects` queues its
/// commit, a second system writes the counter DIRECTLY (no commands, so no sync
/// point), and only then does the schedule reach the barrier where the commit
/// applies.
#[test]
fn a_counter_mutation_before_the_commit_applies_refuses_with_nothing_built() {
    use ambition_platformer2d_shared_tangle::sim_id::SimIdCounter;
    use bevy::prelude::{IntoScheduleConfigs, Query, Schedule};

    fn interlope(mut counters: Query<&mut SimIdCounter>) {
        for mut counter in &mut counters {
            counter.0 = 5;
        }
    }

    let mut world = summon_world();
    let boss = world
        .spawn((SimId::placement("boss_1"), SimIdCounter::default()))
        .id();
    world.write_message(ambition_vfx::EffectRequest {
        owner: boss,
        effect: ambition_vfx::Effect::Summon(summon_spec("slop_add")),
    });

    let mut schedule = Schedule::default();
    // Turned off here deliberately: the point is to reproduce the interleaving, not to rely on
    // the scheduler preventing it.
    schedule.set_build_settings(bevy::ecs::schedule::ScheduleBuildSettings {
        auto_insert_apply_deferred: false,
        ..Default::default()
    });
    schedule.add_systems((
        crate::features::apply_summon_effects,
        interlope.after(crate::features::apply_summon_effects),
    ));
    schedule.run(&mut world);

    let built = world
        .query::<&SimId>()
        .iter(&world)
        .filter(|id| id.as_str().starts_with("placement:boss_1/"))
        .count();
    assert_eq!(built, 0, "the refusal happened before anything was built");
    assert_eq!(
        world.get::<SimIdCounter>(boss).map(|counter| counter.0),
        Some(5),
        "the interloper's value stands — there is no max() recovery path"
    );
}

// ── The production boundary publishes only what it verified ──────────────────
//
// These run the REAL path: `RoomFeatureConstructionPlan::prepare` →
// `spawn_room_feature_entities_from_plan` → the queued capture/verify pair →
// `RoomLoaded`. Nothing here reaches into the verifier directly.

fn room_loaded_count(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ambition_platformer2d_world::rooms::RoomLoaded>>()
        .drain()
        .count()
}

/// The room is not published when its relations did not land.
///
/// A room that fails verification does not publish, and does not write
/// `RoomLoaded`.
///
/// Nothing test-only is wired into the construction path to produce it.
///
/// That hazard is gone, so the seam is gone with it; relation-postcondition detection is proven
/// against the toy domain in `ambition_platformer2d_shared_tangle` and, for the real limb and mount
/// wiring, by the poison tests further down this file.
#[test]
fn a_room_that_fails_verification_is_not_published() {
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &engine_construction_registry())
        .expect("the room plans: the defect is in the world, not in the plan");
    let mut app = commit_over(plan, |world| {
        // A live body already wearing an identity the room is about to build.
        world.spawn(SimId::placement("duel_blue"));
    });

    let verification = app
        .world()
        .resource::<crate::features::LastConstructionVerification>()
        .clone();
    assert!(
        !verification.published,
        "a room that failed verification must not publish: {verification:?}"
    );
    assert!(
        verification.violations.iter().any(|violation| matches!(
            violation,
            ambition_platformer2d_shared_tangle::construction::RosterViolation::PlannedOverBaseline {
                ..
            }
        )),
        "got {:?}",
        verification.violations
    );
    assert_eq!(
        room_loaded_count(&mut app),
        0,
        "RoomLoaded must not be written when verification failed"
    );
}

/// Poison counterpart: the SAME room, the same code path, the real registry.
/// Without this the test above would also pass if rooms never published at all.
#[test]
fn the_same_room_publishes_once_its_relation_lands() {
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &engine_construction_registry()).expect("the room plans");
    let mut app = commit(plan);

    let verification = app
        .world()
        .resource::<crate::features::LastConstructionVerification>()
        .clone();
    assert!(
        verification.violations.is_empty(),
        "a correctly wired room has no construction violations: {:?}",
        verification.violations
    );
    assert!(verification.published, "{verification:?}");
    assert_eq!(
        room_loaded_count(&mut app),
        1,
        "a verified room publishes exactly once"
    );
}

// ── Bidirectional relations (Phase 4, first migration) ───────────────────────
//
// `Limb`/`LimbRig` and `RidingOn`/`MountSlot` are each TWO components that must
// agree. Every test here checks both sides, because the way these pairs have
// historically broken is one side landing and the other not — a failure that
// every forward-only assertion passes straight through.

use ambition_platformer2d_shared_tangle::construction::{
    verify_committed_roster, AuthoritativeScope, ConstructionReceipt, RelationCheck,
    RelationRequest, RosterViolation, TransactionBaseline,
};

fn dynamic_scope() -> ConstructionScope {
    ConstructionScope {
        binding: ambition_platformer2d_shared_tangle::construction::ContentBinding::RuntimeDynamic,
        room: None,
    }
}

fn bare_request(id: &str) -> ActorConstructionRequest {
    ActorConstructionRequest {
        sim_id: SimId::placement(id),
        origin: SpawnOrigin::ProviderStaged {
            provider: "test_provider".into(),
            room: "hall".into(),
            instance: id.into(),
        },
        parameters: ActorConstructionParams::StagedActor(staged_enemy(id, None)),
        relations: Vec::new(),
    }
}

fn test_services() -> ActorConstructionServices {
    ActorConstructionServices {
        context: crate::world::placements::ActorPlacementContext::new(
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &Default::default(),
        )
        .with_prepared(fixture_cast()),
        boss_catalog: ambition_boss_encounter::test_boss_catalog().clone(),
    }
}

/// Commit a bare construction plan into a fresh world and hand back everything
/// verification needs.
fn commit_bare(plan: &ActorConstructionPlan) -> (World, ConstructionReceipt, TransactionBaseline) {
    let mut world = World::new();
    let baseline =
        TransactionBaseline::capture(&mut world).expect("an empty world has no duplicates");
    let services = test_services();
    let receipt = {
        let mut commands = world.commands();
        let scope = plan.scope().clone();
        let mut ctx = ambition_platformer2d_shared_tangle::construction::ConstructionExecCtx {
            commands: &mut commands,
            scope: &scope,
            session: SessionSpawnScope::UNSCOPED,
            services: &services,
        };
        plan.commit(&mut ctx)
    };
    world.flush();
    (world, receipt, baseline)
}

fn verify_bare(
    world: &mut World,
    plan: &ActorConstructionPlan,
    receipt: &ConstructionReceipt,
    baseline: &TransactionBaseline,
) -> Result<(), Vec<RosterViolation>> {
    let transaction = plan.transaction(SessionSpawnScope::UNSCOPED);
    let scope = AuthoritativeScope::gather(world, &transaction);
    verify_committed_roster(plan, receipt, baseline, &scope, world)
}

/// A plan of `rows`, with `from` declaring `kind`/`payload` onto `to`.
fn related_actor_plan(
    rows: &[&str],
    from: &str,
    to: &str,
    relation: ActorRelation,
) -> ActorConstructionPlan {
    let requests: Vec<_> = rows
        .iter()
        .map(|id| {
            let mut request = bare_request(id);
            if *id == from {
                request.relations.push(RelationRequest {
                    to: SimId::placement(to),
                    relation: relation.clone(),
                });
            }
            request
        })
        .collect();
    ActorConstructionPlan::prepare(
        dynamic_scope(),
        requests,
        &Default::default(),
        &engine_construction_registry(),
    )
    .expect("the plan is valid")
}

/// Give a committed rider/mount pair the capability components their archetypes
/// would carry, so the mount postcondition's capability checks have something to
/// read. The bare fixtures build generic enemy bodies, which are neither mounts
/// nor pilots; `verify_mount` legitimately requires `Mountable` on the mount and
/// a compatible `CanPilot` on the rider, so a wiring test must equip the pair for
/// the same reason a real room's archetypes do.
fn equip_mount_pair(world: &mut World, rider: Entity, mount: Entity) {
    world.entity_mut(mount).insert(ambition_mount::Mountable {
        rider_offset: ae::Vec2::ZERO,
        class: ambition_mount::MountClass("giant".into()),
        control_grant: ambition_mount::ControlGrant::Total,
        death_impact: ambition_mount::MountDeathImpact::Dismount,
    });
    world.entity_mut(rider).insert(ambition_mount::CanPilot {
        classes: vec![ambition_mount::MountClass("giant".into())],
    });
}

fn hand(slot: LimbSlot) -> ActorRelation {
    ActorRelation::Limb {
        slot,
        home_offset: ae::Vec2::new(12.0, -4.0),
    }
}

/// One limb relation writes BOTH ends: `Limb` on the limb, an entry in the
/// host's `LimbRig` going back.
#[test]
fn a_limb_relation_wires_the_limb_and_the_hosts_rig() {
    let plan = related_actor_plan(
        &["giant", "hand"],
        "hand",
        "giant",
        hand(LimbSlot::HAND_LEFT),
    );
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let limb = receipt.entity(&SimId::placement("hand")).expect("built");
    let host = receipt.entity(&SimId::placement("giant")).expect("built");

    let attached = world.get::<Limb>(limb).expect("the limb side landed");
    assert_eq!(attached.of, host);
    assert_eq!(attached.slot, LimbSlot::HAND_LEFT);
    assert_eq!(attached.home_offset, ae::Vec2::new(12.0, -4.0));

    let rig = world.get::<LimbRig>(host).expect("the host side landed");
    assert_eq!(
        rig.get(LimbSlot::HAND_LEFT),
        Some(limb),
        "the rig files the limb under exactly its planned slot"
    );
    assert_eq!(rig.limbs.len(), 1, "and drives no other limb");

    assert_eq!(verify_bare(&mut world, &plan, &receipt, &baseline), Ok(()));
}

/// A limb the host's rig does not contain is inert but looks attached.
///
/// `fan_out_limb_intents` iterates the RIG, so a limb missing from it receives
/// nothing — while `Limb.of` still names the right host and every forward-only
/// check passes. This is the half-write the reverse verification exists for.
#[test]
fn a_limb_missing_from_its_hosts_rig_is_detected() {
    let plan = related_actor_plan(
        &["giant", "hand"],
        "hand",
        "giant",
        hand(LimbSlot::HAND_RIGHT),
    );
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let host = receipt.entity(&SimId::placement("giant")).expect("built");

    // Exactly the half-write: strip the reverse side, leave the forward one.
    world.entity_mut(host).remove::<LimbRig>();

    let violations = verify_bare(&mut world, &plan, &receipt, &baseline)
        .expect_err("a limb outside its host's rig must be detected");
    assert!(
        violations.iter().any(|violation| matches!(
            violation,
            RosterViolation::RelationNotEstablished { check, .. }
                if *check == RelationCheck::ReverseMismatch { found: None }
        )),
        "got {violations:?}"
    );
}

/// The slot is part of the relation, so a rewritten slot is a defect: the
/// router would drive this limb from the wrong intent stream.
#[test]
fn a_limb_whose_slot_was_rewritten_is_detected() {
    let plan = related_actor_plan(
        &["giant", "hand"],
        "hand",
        "giant",
        hand(LimbSlot::HAND_LEFT),
    );
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let limb = receipt.entity(&SimId::placement("hand")).expect("built");
    let host = receipt.entity(&SimId::placement("giant")).expect("built");

    world.entity_mut(limb).insert(Limb {
        of: host,
        slot: LimbSlot::HAND_RIGHT,
        home_offset: ae::Vec2::new(12.0, -4.0),
    });

    let violations = verify_bare(&mut world, &plan, &receipt, &baseline)
        .expect_err("a rewritten slot must be detected");
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, RosterViolation::RelationNotEstablished { .. })),
        "got {violations:?}"
    );
}

/// Two limbs ACCUMULATE into one rig, in the plan's canonical relation order
/// rather than in whatever order anything happened to spawn.
#[test]
fn two_limbs_accumulate_into_one_rig_keyed_by_slot() {
    let giant = SimId::placement("giant");
    let mut host = bare_request("giant");
    host.relations.clear();
    let mut left = bare_request("giant/0");
    left.relations.push(RelationRequest {
        to: giant.clone(),
        relation: hand(LimbSlot::HAND_LEFT),
    });
    let mut right = bare_request("giant/1");
    right.relations.push(RelationRequest {
        to: giant.clone(),
        relation: hand(LimbSlot::HAND_RIGHT),
    });

    // Declared right-first on purpose: canonical ordering, not arrival order,
    // must decide the rig's contents.
    let plan = ActorConstructionPlan::prepare(
        dynamic_scope(),
        vec![right, host, left],
        &Default::default(),
        &engine_construction_registry(),
    )
    .expect("the plan is valid");
    let (mut world, receipt, baseline) = commit_bare(&plan);

    let host_entity = receipt.entity(&SimId::placement("giant")).expect("built");
    let left_entity = receipt.entity(&SimId::placement("giant/0")).expect("built");
    let right_entity = receipt.entity(&SimId::placement("giant/1")).expect("built");
    let rig = world
        .get::<LimbRig>(host_entity)
        .expect("the rig accumulated");
    assert_eq!(rig.get(LimbSlot::HAND_LEFT), Some(left_entity));
    assert_eq!(rig.get(LimbSlot::HAND_RIGHT), Some(right_entity));
    assert_eq!(rig.limbs.len(), 2, "exactly the two declared limbs");
    assert_eq!(verify_bare(&mut world, &plan, &receipt, &baseline), Ok(()));
}

/// A mount relation writes both ends: `RidingOn` + `Mounted` on the rider,
/// `MountSlot` on the mount going back.
#[test]
fn a_mount_relation_wires_the_rider_and_the_mounts_slot() {
    let plan = related_actor_plan(&["rider", "mount"], "rider", "mount", ActorRelation::Mount);
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let rider = receipt.entity(&SimId::placement("rider")).expect("built");
    let mount = receipt.entity(&SimId::placement("mount")).expect("built");

    assert_eq!(
        world
            .get::<ambition_mount::RidingOn>(rider)
            .expect("the rider side landed")
            .mount,
        mount
    );
    assert!(
        world.get::<ambition_mount::Mounted>(rider).is_some(),
        "the rider is marked mounted"
    );
    assert_eq!(
        world
            .get::<ambition_mount::MountSlot>(mount)
            .expect("the mount side landed")
            .rider,
        Some(rider)
    );
    equip_mount_pair(&mut world, rider, mount);
    assert_eq!(verify_bare(&mut world, &plan, &receipt, &baseline), Ok(()));
}

/// The half-write that exists in the tree today.
///
/// `attach_mount_role` never inserts `MountSlot`, and
/// `reconcile_autonomous_actors` re-establishes the link with
/// `world.get_mut::<MountSlot>(..)` — a mutation that silently does nothing when
/// the component is absent — while inserting `RidingOn` unconditionally. The
/// result is a rider pointing at a mount that does not point back, and
/// `steer_mount_from_rider` queries `With<MountSlot>`, so the mount quietly
/// stops obeying while every rider-side assertion still passes.
#[test]
fn a_mount_that_does_not_point_back_at_its_rider_is_detected() {
    let plan = related_actor_plan(&["rider", "mount"], "rider", "mount", ActorRelation::Mount);
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let rider = receipt.entity(&SimId::placement("rider")).expect("built");
    let mount = receipt.entity(&SimId::placement("mount")).expect("built");
    equip_mount_pair(&mut world, rider, mount);

    world
        .entity_mut(mount)
        .remove::<ambition_mount::MountSlot>();

    let violations = verify_bare(&mut world, &plan, &receipt, &baseline)
        .expect_err("a mount that does not point back must be detected");
    assert!(
        violations.iter().any(|violation| matches!(
            violation,
            RosterViolation::RelationNotEstablished { check, .. }
                if *check == RelationCheck::ReverseMismatch { found: None }
        )),
        "got {violations:?}"
    );
}

/// A mount whose slot points at somebody ELSE — two riders claiming one saddle.
#[test]
fn a_mount_holding_a_different_rider_is_detected() {
    let plan = related_actor_plan(&["rider", "mount"], "rider", "mount", ActorRelation::Mount);
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let rider = receipt.entity(&SimId::placement("rider")).expect("built");
    let mount = receipt.entity(&SimId::placement("mount")).expect("built");
    equip_mount_pair(&mut world, rider, mount);
    let usurper = world.spawn_empty().id();

    world.entity_mut(mount).insert(ambition_mount::MountSlot {
        rider: Some(usurper),
    });

    let violations = verify_bare(&mut world, &plan, &receipt, &baseline)
        .expect_err("a saddle holding the wrong rider must be detected");
    assert!(
        violations.iter().any(|violation| matches!(
            violation,
            RosterViolation::RelationNotEstablished { check, .. }
                if matches!(check, RelationCheck::ReverseMismatch { found: Some(_) })
        )),
        "got {violations:?}"
    );
}

/// A limb wired into the wrong slot is detected — the slot is verified on
/// both sides.
#[test]
fn a_limb_filed_under_the_wrong_slot_is_detected() {
    let plan = related_actor_plan(
        &["giant", "hand"],
        "hand",
        "giant",
        hand(LimbSlot::HAND_LEFT),
    );
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let host = receipt.entity(&SimId::placement("giant")).expect("built");
    let limb = receipt.entity(&SimId::placement("hand")).expect("built");

    // File the same limb under the OTHER slot, leaving `Limb.slot` right.
    let mut rig = world.get_mut::<LimbRig>(host).expect("the rig landed");
    rig.limbs.clear();
    rig.limbs.insert(LimbSlot::HAND_RIGHT, limb);

    let violations = verify_bare(&mut world, &plan, &receipt, &baseline)
        .expect_err("a limb filed under the wrong slot must be detected");
    assert!(
        violations.iter().any(|violation| matches!(
            violation,
            RosterViolation::RelationNotEstablished {
                check: RelationCheck::PayloadMismatch { field: "rig_slot" },
                ..
            }
        )),
        "got {violations:?}"
    );
}

/// A limb whose home offset was overwritten after wiring is detected.
///
/// The offset is the limb's entire idle behaviour; a corrupted one station-keeps to the wrong
/// place forever, which no structural check would ever notice.
#[test]
fn a_limb_with_a_corrupted_home_offset_is_detected() {
    let plan = related_actor_plan(
        &["giant", "hand"],
        "hand",
        "giant",
        hand(LimbSlot::HAND_LEFT),
    );
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let limb = receipt.entity(&SimId::placement("hand")).expect("built");

    world.get_mut::<Limb>(limb).unwrap().home_offset = ae::Vec2::new(999.0, 999.0);

    let violations = verify_bare(&mut world, &plan, &receipt, &baseline)
        .expect_err("a corrupted home offset must be detected");
    assert!(
        violations.iter().any(|violation| matches!(
            violation,
            RosterViolation::RelationNotEstablished {
                check: RelationCheck::PayloadMismatch {
                    field: "home_offset"
                },
                ..
            }
        )),
        "got {violations:?}"
    );
}

/// A mount link missing `Mounted` is detected.
///
/// `steer_mount_from_rider` queries `With<Mounted>`, so a rider linked without it
/// sits on a mount that never receives its intent. Every `RidingOn`/`MountSlot`
/// assertion passes.
#[test]
fn a_mount_link_missing_the_mounted_marker_is_detected() {
    let plan = related_actor_plan(&["rider", "mount"], "rider", "mount", ActorRelation::Mount);
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let rider = receipt.entity(&SimId::placement("rider")).expect("built");
    let mount = receipt.entity(&SimId::placement("mount")).expect("built");
    equip_mount_pair(&mut world, rider, mount);

    world.entity_mut(rider).remove::<ambition_mount::Mounted>();

    let violations = verify_bare(&mut world, &plan, &receipt, &baseline)
        .expect_err("a rider without Mounted must be detected");
    assert!(
        violations.iter().any(|violation| matches!(
            violation,
            RosterViolation::RelationNotEstablished {
                check: RelationCheck::MissingCapability {
                    component: "Mounted"
                },
                ..
            }
        )),
        "got {violations:?}"
    );
}

/// A mount link whose rider cannot pilot the mount's class is detected.
///
/// The preflight rejects this before construction; this is the runtime
/// counterpart, for a pair that somehow reached the world incompatible.
#[test]
fn a_mount_link_with_an_incompatible_class_is_detected() {
    let plan = related_actor_plan(&["rider", "mount"], "rider", "mount", ActorRelation::Mount);
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let rider = receipt.entity(&SimId::placement("rider")).expect("built");
    let mount = receipt.entity(&SimId::placement("mount")).expect("built");
    equip_mount_pair(&mut world, rider, mount);
    // The rider can pilot "giant" but the mount is now a "shark".
    world.entity_mut(mount).insert(ambition_mount::Mountable {
        rider_offset: ae::Vec2::ZERO,
        class: ambition_mount::MountClass("shark".into()),
        control_grant: ambition_mount::ControlGrant::Total,
        death_impact: ambition_mount::MountDeathImpact::Dismount,
    });

    let violations = verify_bare(&mut world, &plan, &receipt, &baseline)
        .expect_err("an incompatible mount class must be detected");
    assert!(
        violations.iter().any(|violation| matches!(
            violation,
            RosterViolation::RelationNotEstablished {
                check: RelationCheck::PayloadMismatch {
                    field: "mount_class"
                },
                ..
            }
        )),
        "got {violations:?}"
    );
}

// ── Preflight: illegal relation configurations rejected before mutation ───────

/// Build one summon request per row for a preflight fixture, so the relation
/// rules can be exercised without a whole room.
fn minion_request(id: &str, archetype: &str) -> ActorConstructionRequest {
    summoned_minion_request(
        &SimId::placement("summoner"),
        id.bytes().map(u64::from).sum(),
        SummonedMinionParams {
            // A fixture keeps the character's authored vitals.
            health: None,
                    // A boss minion keeps its character's hazard.
                    keeps_contact_damage: true,
            feature_id: id.to_string(),
            name: id.to_string(),
            pos: ae::Vec2::ZERO,
            half_size: ae::Vec2::splat(10.0),
            character_id: archetype.to_string(),
            encounter_id: "e".into(),
            faction: crate::features::ActorFaction::Enemy,
        },
    )
}

fn preflight(requests: Vec<ActorConstructionRequest>) -> Result<(), ActorConstructionError> {
    preflight_actor_relations(
        &requests,
        &ambition_boss_encounter::test_boss_catalog(),
        Some(fixture_cast()),
    )
}

/// Two limbs claiming one host slot is refused before any spawn.
#[test]
fn two_limbs_in_one_slot_are_rejected() {
    let host = minion_request("giant", "fixture_giant");
    let mut a = minion_request("hand_a", "giant_gnu_hands");
    let mut b = minion_request("hand_b", "giant_gnu_hands");
    a.relations.push(RelationRequest {
        to: host.sim_id.clone(),
        relation: hand(LimbSlot::HAND_LEFT),
    });
    b.relations.push(RelationRequest {
        to: host.sim_id.clone(),
        relation: hand(LimbSlot::HAND_LEFT),
    });
    assert!(matches!(
        preflight(vec![host, a, b]),
        Err(ActorConstructionError::LimbSlotTaken { .. })
    ));
}

/// One limb naming two hosts is refused.
#[test]
fn a_limb_with_two_hosts_is_rejected() {
    let host_a = minion_request("giant_a", "fixture_giant");
    let host_b = minion_request("giant_b", "fixture_giant");
    let mut limb = minion_request("hand", "giant_gnu_hands");
    limb.relations.push(RelationRequest {
        to: host_a.sim_id.clone(),
        relation: hand(LimbSlot::HAND_LEFT),
    });
    limb.relations.push(RelationRequest {
        to: host_b.sim_id.clone(),
        relation: hand(LimbSlot::HAND_RIGHT),
    });
    assert!(matches!(
        preflight(vec![host_a, host_b, limb]),
        Err(ActorConstructionError::LimbHasTwoHosts { .. })
    ));
}

/// Two riders claiming one mount is refused before mutation.
#[test]
fn two_riders_on_one_mount_are_rejected() {
    let mut rider_a = minion_request("rider_a", "pirate_raider");
    let mut rider_b = minion_request("rider_b", "pirate_raider");
    let mount = minion_request("shark", "fixture_mount");
    rider_a.relations.push(RelationRequest {
        to: mount.sim_id.clone(),
        relation: ActorRelation::Mount,
    });
    rider_b.relations.push(RelationRequest {
        to: mount.sim_id.clone(),
        relation: ActorRelation::Mount,
    });
    assert!(matches!(
        preflight(vec![rider_a, rider_b, mount]),
        Err(ActorConstructionError::MountHasTwoRiders { .. })
    ));
}

/// One rider naming two mounts is refused.
#[test]
fn one_rider_on_two_mounts_is_rejected() {
    let mut rider = minion_request("rider", "pirate_raider");
    let mount_a = minion_request("shark_a", "fixture_mount");
    let mount_b = minion_request("shark_b", "fixture_mount");
    rider.relations.push(RelationRequest {
        to: mount_a.sim_id.clone(),
        relation: ActorRelation::Mount,
    });
    rider.relations.push(RelationRequest {
        to: mount_b.sim_id.clone(),
        relation: ActorRelation::Mount,
    });
    assert!(matches!(
        preflight(vec![rider, mount_a, mount_b]),
        Err(ActorConstructionError::RiderOnTwoMounts { .. })
    ));
}

/// A self-mount is refused.
#[test]
fn a_self_mount_is_rejected() {
    let mut rider = minion_request("rider", "pirate_raider");
    rider.relations.push(RelationRequest {
        to: rider.sim_id.clone(),
        relation: ActorRelation::Mount,
    });
    assert!(matches!(
        preflight(vec![rider]),
        Err(ActorConstructionError::SelfMount { .. })
    ));
}

/// A rider whose class list does not include the mount's class is refused
/// before mutation — where the live path drops the link silently.
#[test]
fn an_incompatible_pilot_and_mount_class_are_rejected() {
    // A shark-rider cannot pilot a `giant`-class mount.
    let mut rider = minion_request("rider", "pirate_raider");
    let mount = minion_request("giant", "fixture_giant");
    rider.relations.push(RelationRequest {
        to: mount.sim_id.clone(),
        relation: ActorRelation::Mount,
    });
    assert!(matches!(
        preflight(vec![rider, mount]),
        Err(ActorConstructionError::IncompatibleMountClass { .. })
    ));
}

/// A mount relation whose "mount" end is not a mount at all is refused.
#[test]
fn a_mount_relation_onto_a_non_mount_is_rejected() {
    let mut rider = minion_request("rider", "pirate_raider");
    // A shark-rider ridden by nothing — but here we point it at another rider,
    // which has no `mount_class`.
    let not_a_mount = minion_request("also_rider", "pirate_raider");
    rider.relations.push(RelationRequest {
        to: not_a_mount.sim_id.clone(),
        relation: ActorRelation::Mount,
    });
    assert!(matches!(
        preflight(vec![rider, not_a_mount]),
        Err(ActorConstructionError::WrongFamilyForRelation { end: "mount", .. })
    ));
}

/// A compatible pair passes the preflight — the poison counterpart, so the
/// rejections above are not merely "everything is rejected".
#[test]
fn a_compatible_rider_and_mount_pass_preflight() {
    let mut rider = minion_request("rider", "pirate_raider");
    let mount = minion_request("shark", "fixture_mount");
    rider.relations.push(RelationRequest {
        to: mount.sim_id.clone(),
        relation: ActorRelation::Mount,
    });
    assert_eq!(preflight(vec![rider, mount]), Ok(()));
}

// ── Giant hands are explicit plan rows (Checkpoint B) ─────────────────────────

fn giant_room() -> ambition_platformer2d_world::rooms::RoomSpec {
    let mut room = empty_room("arena");
    room.enemy_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
        "boss_mount",
        "Giant GNU",
        ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::splat(60.0)),
        ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom("fixture_giant".into()),
            "fixture_giant",
        ),
    ));
    room
}

/// A CHARACTER can make a limbed host, with no archetype row saying so.
///
/// the fixture's roster deliberately answers `combatant` for this brain, so
/// the ONLY thing that can produce three rows here is the character.
#[test]
fn a_character_that_authors_a_giant_mount_plans_its_hands_without_a_row() {
    let mut room = empty_room("arena");
    let mut authored: ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::EnemySpawnSpec> =
        ambition_platformer2d_world::rooms::Authored::new(
            "boss_mount",
            "Giant GNU",
            ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::splat(60.0)),
            ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                ambition_entity_catalog::placements::CharacterBrain::Custom(
                    "no_such_archetype".into(),
                ),
                "no_such_archetype",
            ),
        );
    authored.payload.character_id = ambition_entity_catalog::CharacterId::new("npc_giant");
    room.enemy_spawns.push(authored);

    let finalized = crate::character_runtime::prepare_and_finalize_for_test(
        crate::character_runtime::CharacterDefinition::new("npc_giant", "Giant GNU", "test")
            .with_mount(ambition_characters::actor::CharacterMount {
                class: Some("giant".to_string()),
                ..Default::default()
            }),
        &crate::character_runtime::CharacterBindings::default(),
    );
    let mut cast = crate::character_runtime::PreparedCharacterRegistry::default();
    cast.insert_prepared(finalized.prepared);

    let without_cast = crate::construction::authored_actor_requests(&room, &[], None);
    assert_eq!(
        without_cast.len(),
        1,
        "the control: with no cast prepared, the unknown brain is an ordinary \
         enemy — so three rows below cannot come from the roster"
    );

    let requests = crate::construction::authored_actor_requests(&room, &[], Some(&cast));
    assert_eq!(
        requests.len(),
        3,
        "host + two hands, decided by the CHARACTER: {:?}",
        requests
            .iter()
            .map(|r| r.sim_id.clone())
            .collect::<Vec<_>>()
    );
}

/// The giant host and both hands are explicit plan rows joined by limb
/// relations. They used to be minted inside the enemy spawn helper as
/// authoritative roots no plan named — the last legacy family.
#[test]
fn a_giant_enemy_becomes_a_host_row_and_two_hand_rows() {
    let requests =
        crate::construction::authored_actor_requests(&giant_room(), &[], Some(fixture_cast()));

    // One host + two hands.
    assert_eq!(requests.len(), 3, "host + two hands");
    let host = SimId::placement("boss_mount");
    let hand_l = SimId::spawned(&host, 0);
    let hand_r = SimId::spawned(&host, 1);
    let ids: std::collections::BTreeSet<_> = requests.iter().map(|r| r.sim_id.clone()).collect();
    assert!(ids.contains(&host) && ids.contains(&hand_l) && ids.contains(&hand_r));

    // Each hand declares one limb relation back onto the host, and the host
    // declares none.
    let host_relations = requests
        .iter()
        .find(|r| r.sim_id == host)
        .expect("host row")
        .relations
        .len();
    assert_eq!(
        host_relations, 0,
        "the host carries no relations; the hands do"
    );
    for hand in [&hand_l, &hand_r] {
        let row = requests
            .iter()
            .find(|r| &r.sim_id == hand)
            .expect("hand row");
        assert_eq!(row.relations.len(), 1);
        assert_eq!(row.relations[0].to, host);
        assert!(matches!(
            row.relations[0].relation,
            ActorRelation::Limb { .. }
        ));
    }
}

/// The giant rows commit into a correctly wired rig, and the boundary verifier
/// sees no violation — no legacy warning, because the hands are owned rows now.
#[test]
fn a_committed_giant_has_a_verified_two_hand_rig() {
    let requests =
        crate::construction::authored_actor_requests(&giant_room(), &[], Some(fixture_cast()));
    let plan = ActorConstructionPlan::prepare(
        dynamic_scope(),
        requests,
        &Default::default(),
        &engine_construction_registry(),
    )
    .expect("the giant plan is valid");

    let host = SimId::placement("boss_mount");
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let host_entity = receipt.entity(&host).expect("host built");
    let hand_l = receipt
        .entity(&SimId::spawned(&host, 0))
        .expect("left built");
    let hand_r = receipt
        .entity(&SimId::spawned(&host, 1))
        .expect("right built");

    let rig = world
        .get::<LimbRig>(host_entity)
        .expect("the host carries a rig");
    assert_eq!(rig.get(LimbSlot::HAND_LEFT), Some(hand_l));
    assert_eq!(rig.get(LimbSlot::HAND_RIGHT), Some(hand_r));
    assert_eq!(rig.limbs.len(), 2);
    // The host owns the router's scratch state.
    assert!(world.get::<LimbIntents>(host_entity).is_some());
    assert!(world.get::<LimbRouteState>(host_entity).is_some());

    assert_eq!(verify_bare(&mut world, &plan, &receipt, &baseline), Ok(()));
}

/// Reconstruction closure: asking to rebuild any one of the three rebuilds all
/// three. The giant host is a relation target and each hand a source, so no
/// one of them can be rebuilt alone — the closure holds the cluster together.
#[test]
fn the_giant_reconstruction_closure_is_the_whole_cluster() {
    let requests =
        crate::construction::authored_actor_requests(&giant_room(), &[], Some(fixture_cast()));
    let plan = ActorConstructionPlan::prepare(
        dynamic_scope(),
        requests,
        &Default::default(),
        &engine_construction_registry(),
    )
    .expect("valid");

    let host = SimId::placement("boss_mount");
    let hand_l = SimId::spawned(&host, 0);
    let hand_r = SimId::spawned(&host, 1);
    for seed in [&host, &hand_l, &hand_r] {
        let closure = plan.relation_closure(&std::collections::BTreeSet::from([seed.clone()]));
        assert_eq!(
            closure,
            std::collections::BTreeSet::from([host.clone(), hand_l.clone(), hand_r.clone()]),
            "the closure of {seed} is the whole giant cluster"
        );
    }
}

// ── Giants for every construction origin ─────────────────────────────────────

fn staged_giant(id: &str) -> SpawnActorRequest {
    SpawnActorRequest {
        id: id.to_string(),
        name: "Giant GNU".to_string(),
        pos: ae::Vec2::new(100.0, 100.0),
        half_size: ae::Vec2::splat(60.0),
        faction: crate::features::ActorFaction::Enemy,
        grudge_against: None,
        kind: SpawnActorKind::Enemy {
            brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                "fixture_giant".into(),
            ),
            character: ambition_entity_catalog::CharacterId::from("fixture_giant"),
        },
    }
}

/// A provider-staged giant lowers to the SAME three-row cluster an authored
/// one does. Before this, `staged_actor_requests` emitted a single
/// `StagedActor` row whose recipe routed through the enemy spawn helper — which
/// no longer spawns hands — so a staged giant was a handless host.
#[test]
fn a_staged_giant_becomes_a_host_row_and_two_hand_rows() {
    let requests =
        staged_actor_requests("hall", "prov", &[staged_giant("gnu")], Some(fixture_cast()));

    assert_eq!(requests.len(), 3, "host + two hands");
    let host = SimId::placement("gnu");
    for (sim_id, ordinal) in [(SimId::spawned(&host, 0), 0), (SimId::spawned(&host, 1), 1)] {
        let row = requests
            .iter()
            .find(|r| r.sim_id == sim_id)
            .unwrap_or_else(|| panic!("hand row {ordinal} exists"));
        assert_eq!(row.relations.len(), 1);
        assert_eq!(row.relations[0].to, host);
        assert!(matches!(
            row.relations[0].relation,
            ActorRelation::Limb { .. }
        ));
        assert!(
            matches!(&row.origin, SpawnOrigin::ProviderStaged { provider, room, .. }
                if provider == "prov" && room == "hall"),
            "a staged hand keeps its staged provenance: {:?}",
            row.origin
        );
    }
    let host_row = requests
        .iter()
        .find(|r| r.sim_id == host)
        .expect("host row");
    assert!(
        matches!(
            &host_row.parameters,
            ActorConstructionParams::GiantHost { .. }
        ),
        "the staged giant is a GiantHost row, not a StagedActor row"
    );
}

/// The giant expansion does not leak onto ordinary staged actors.
#[test]
fn a_staged_non_giant_stays_a_single_staged_actor_row() {
    let requests = staged_actor_requests(
        "hall",
        "prov",
        &[staged_enemy("npc", None)],
        Some(fixture_cast()),
    );
    assert_eq!(requests.len(), 1);
    assert!(matches!(
        &requests[0].parameters,
        ActorConstructionParams::StagedActor(_)
    ));
}

/// End to end: a provider STAGES a giant, the room commits, the boundary
/// verifier publishes, and the world holds a fully wired two-hand rig.
#[test]
fn a_staged_giant_commits_into_a_published_room_with_a_wired_rig() {
    let room = empty_room("hall");
    let mut staging = crate::features::RoomContentStagingRegistry::default();
    staging
        .register("hall", "test_provider", "boss", "boss.v1", |_room| {
            vec![staged_giant("gnu")]
        })
        .expect("stager registers");
    let plan = prepare(&room, &staging, &engine_construction_registry()).expect("the room plans");
    let mut app = commit(plan);

    let verification = app
        .world()
        .resource::<crate::world::rooms::LastConstructionVerification>();
    assert!(
        verification.published,
        "the staged-giant room publishes: {:?}",
        verification.violations
    );

    let host = SimId::placement("gnu");
    let world = app.world_mut();
    let find = |world: &mut World, wanted: &SimId| {
        let mut query = world.query::<(bevy::prelude::Entity, &SimId)>();
        query
            .iter(world)
            .find(|(_, sim)| *sim == wanted)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("`{wanted}` is live"))
    };
    let host_entity = find(world, &host);
    let hand_l = find(world, &SimId::spawned(&host, 0));
    let hand_r = find(world, &SimId::spawned(&host, 1));
    let rig = world
        .get::<LimbRig>(host_entity)
        .expect("the staged host carries a rig");
    assert_eq!(rig.get(LimbSlot::HAND_LEFT), Some(hand_l));
    assert_eq!(rig.get(LimbSlot::HAND_RIGHT), Some(hand_r));
}

/// The authored giant host carries the room's frozen kinematic paths — the
/// same seed data an ordinary authored enemy receives. The first migration
/// passed `Vec::new()`, silently un-pathing every giant.
#[test]
fn an_authored_giant_host_carries_the_rooms_frozen_paths() {
    let mut room = giant_room();
    room.kinematic_paths
        .push(ambition_platformer2d_world::rooms::KinematicPathSpec::new(
            "patrol",
            "patrol",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)),
            ae::KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(64.0, 0.0), 24.0),
        ));
    let plan = prepare(
        &room,
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the pathed giant room plans");

    let host = SimId::placement("boss_mount");
    let row = plan
        .construction()
        .get(&host)
        .expect("the giant host is a plan row");
    let ActorConstructionParams::GiantHost { paths, .. } = row.parameters() else {
        panic!("the host row is a GiantHost");
    };
    assert_eq!(
        paths.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
        vec!["patrol"],
        "the host row froze the room's paths at planning"
    );
}

/// A runtime-dynamic origin cannot lower a giant into plan rows, so it REFUSES
/// the spec instead of spawning a handless host. The root allocated for the
/// minion stays unpopulated.
#[test]
fn a_runtime_minion_giant_is_refused_before_it_spawns() {
    let mut world = World::new();
    let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::empty();
    let root = {
        let mut commands = world.commands();
        crate::features::ecs::spawn_runtime_minion(
            &mut commands,
            &catalog,
            &Default::default(),
            // the giant IS in the cast, so the refusal under test is the
            // limb-rig one rather than "this names no character" (AC6).
            fixture_cast(),
            SessionSpawnScope::UNSCOPED,
            "runaway",
            "Giant GNU",
            ae::Vec2::ZERO,
            ae::Vec2::splat(60.0),
            "fixture_giant",
            "enc",
            crate::features::ActorFaction::Enemy,
            crate::features::ActorAggression::hostile(),
        )
    };
    world.flush();
    assert!(
        world
            .get::<ambition_combat::components::FeatureId>(root)
            .is_none(),
        "the refused giant populated nothing"
    );
}

/// The encounter-wave origin refuses a giant the same way.
#[test]
fn an_encounter_wave_giant_is_refused_before_it_spawns() {
    let mut world = World::new();
    let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::empty();
    {
        let mut commands = world.commands();
        crate::features::spawn_encounter_mob(
            &mut commands,
            &catalog,
            &Default::default(),
            // See the minion twin: the cast holds the giant, so the refusal
            // under test is the limb-rig one.
            fixture_cast(),
            SessionSpawnScope::UNSCOPED,
            "enc",
            crate::features::EncounterMobSeed {
                id: "wave_gnu".to_string(),
                character: Some("fixture_giant"),
                brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                    "fixture_giant".into(),
                ),
                pos: ae::Vec2::ZERO,
                size: ae::Vec2::splat(120.0),
            },
        );
    }
    world.flush();
    let mut features = world.query::<&ambition_combat::components::FeatureId>();
    assert_eq!(
        features.iter(&world).count(),
        0,
        "the refused wave giant spawned no body"
    );
}

// ── Exact rig composition ─────────────────────────────────────────────────────

fn committed_giant() -> (
    ActorConstructionPlan,
    World,
    ConstructionReceipt,
    TransactionBaseline,
) {
    let requests =
        crate::construction::authored_actor_requests(&giant_room(), &[], Some(fixture_cast()));
    let plan = ActorConstructionPlan::prepare(
        dynamic_scope(),
        requests,
        &Default::default(),
        &engine_construction_registry(),
    )
    .expect("the giant plan is valid");
    let (world, receipt, baseline) = commit_bare(&plan);
    (plan, world, receipt, baseline)
}

fn rig_faults(plan: &ActorConstructionPlan, receipt: &ConstructionReceipt, world: &World) -> usize {
    let faults = crate::construction::verify_rig_composition(plan, receipt, world);
    for fault in &faults {
        assert!(
            matches!(fault, RosterViolation::RigComposition { .. }),
            "the composition pass only speaks RigComposition: {fault:?}"
        );
    }
    faults.len()
}

/// The composition pass is quiet on a correctly committed cluster — the poison
/// tests below are meaningful only if this baseline holds.
#[test]
fn a_clean_giant_rig_has_no_composition_faults() {
    let (plan, world, receipt, _) = committed_giant();
    assert_eq!(rig_faults(&plan, &receipt, &world), 0);
}

/// An EXTRA limb the plan never described: every planned relation still
/// verifies, so only the composition pass can see the surplus.
#[test]
fn an_extra_unplanned_rig_entry_is_fatal() {
    let (plan, mut world, receipt, baseline) = committed_giant();
    let host = SimId::placement("boss_mount");
    let host_entity = receipt.entity(&host).expect("host");
    let interloper = world.spawn_empty().id();
    // A one-hand plan would leave HandRight free; here we sabotage by pointing
    // an occupied slot's entry at a THIRD body while both planned relations
    // keep their own components intact — the shape a second intent stream
    // leaves behind.
    world
        .get_mut::<LimbRig>(host_entity)
        .expect("rig")
        .limbs
        .insert(LimbSlot::HAND_RIGHT, interloper);
    assert!(rig_faults(&plan, &receipt, &world) > 0);
    // And the outer roster pass still passes its OWN checks minus the rig —
    // proving the composition pass is the one that catches this.
    let per_relation = verify_bare(&mut world, &plan, &receipt, &baseline);
    assert!(
        per_relation.is_err(),
        "the reverse-membership check also notices the displaced hand"
    );
}

/// One limb body answering to BOTH slots: each slot individually resolves to a
/// committed hand, so only the duplicate scan sees one body wearing two names.
#[test]
fn a_duplicated_limb_entity_across_slots_is_fatal() {
    let (plan, mut world, receipt, _) = committed_giant();
    let host = SimId::placement("boss_mount");
    let host_entity = receipt.entity(&host).expect("host");
    let hand_l = receipt.entity(&SimId::spawned(&host, 0)).expect("left");
    world
        .get_mut::<LimbRig>(host_entity)
        .expect("rig")
        .limbs
        .insert(LimbSlot::HAND_RIGHT, hand_l);
    assert!(rig_faults(&plan, &receipt, &world) > 0);
}

/// A planned slot with nothing in it. The limb's own components survive, so the
/// forward checks pass; the hole is only visible slot-by-slot.
#[test]
fn a_missing_planned_slot_is_fatal() {
    let (plan, mut world, receipt, _) = committed_giant();
    let host = SimId::placement("boss_mount");
    let host_entity = receipt.entity(&host).expect("host");
    world
        .get_mut::<LimbRig>(host_entity)
        .expect("rig")
        .limbs
        .remove(&LimbSlot::HAND_LEFT);
    assert!(rig_faults(&plan, &receipt, &world) > 0);
}

/// Correct forward `Limb` data on both hands, corrupted HOST rig: the two rig
/// entries are swapped, so each slot holds a real committed hand — the wrong
/// one.
#[test]
fn a_swapped_rig_with_correct_forward_limbs_is_fatal() {
    let (plan, mut world, receipt, _) = committed_giant();
    let host = SimId::placement("boss_mount");
    let host_entity = receipt.entity(&host).expect("host");
    let hand_l = receipt.entity(&SimId::spawned(&host, 0)).expect("left");
    let hand_r = receipt.entity(&SimId::spawned(&host, 1)).expect("right");
    {
        let mut rig = world.get_mut::<LimbRig>(host_entity).expect("rig");
        rig.limbs.insert(LimbSlot::HAND_LEFT, hand_r);
        rig.limbs.insert(LimbSlot::HAND_RIGHT, hand_l);
    }
    assert!(rig_faults(&plan, &receipt, &world) > 0);
}

/// Correct host rig, STALE limb forward pointer: the hand answers to an entity
/// that is not its host's current body. `Limb.of` carries a full `Entity` —
/// index AND generation — so a stale generation compares unequal.
#[test]
fn a_stale_limb_host_pointer_is_fatal() {
    let (plan, mut world, receipt, _) = committed_giant();
    let host = SimId::placement("boss_mount");
    let hand_l = receipt.entity(&SimId::spawned(&host, 0)).expect("left");
    let stale = world.spawn_empty().id();
    world.despawn(stale);
    world
        .get_mut::<Limb>(hand_l)
        .expect("the hand carries a Limb")
        .of = stale;
    assert!(rig_faults(&plan, &receipt, &world) > 0);
}

// ── Reconstruction from a stable identity ─────────────────────────────────────

/// Production reconstruction can start from ANY cluster member — host, left
/// hand, or right hand. The authored-id entry point spells only
/// `SimId::placement`, which can never name a hand; the `SimId` entry point can.
/// Each rebuild produces three FRESH bodies with the rig and both forward limb
/// pointers rewired onto the new generation.
#[test]
fn reconstructing_from_any_giant_cluster_member_rebuilds_all_three_fresh() {
    let host = SimId::placement("boss_mount");
    let hand_l = SimId::spawned(&host, 0);
    let hand_r = SimId::spawned(&host, 1);
    for seed in [&host, &hand_l, &hand_r] {
        let plan = prepare(
            &giant_room(),
            &crate::features::RoomContentStagingRegistry::default(),
            &engine_construction_registry(),
        )
        .expect("the giant room plans");
        let mut app = commit(plan.clone());
        let world = app.world_mut();

        let find = |world: &mut World, wanted: &SimId| {
            let mut query = world.query::<(bevy::prelude::Entity, &SimId)>();
            query
                .iter(world)
                .find(|(_, sim)| *sim == wanted)
                .map(|(entity, _)| entity)
                .unwrap_or_else(|| panic!("`{wanted}` is live"))
        };
        let old: Vec<_> = [&host, &hand_l, &hand_r]
            .into_iter()
            .map(|id| find(world, id))
            .collect();
        for entity in &old {
            world.despawn(*entity);
        }

        let rebuilt = {
            let mut commands = world.commands();
            plan.respawn_authoritative_sim_id(&mut commands, SessionSpawnScope::UNSCOPED, seed)
        };
        assert!(rebuilt, "the closure of `{seed}` rebuilds");
        world.flush();

        let new_host = find(world, &host);
        let new_l = find(world, &hand_l);
        let new_r = find(world, &hand_r);
        for (fresh, stale) in [new_host, new_l, new_r].iter().zip(&old) {
            assert_ne!(fresh, stale, "seed `{seed}` produced a fresh body");
        }
        let rig = world
            .get::<LimbRig>(new_host)
            .expect("the rebuilt host carries a rig");
        assert_eq!(rig.get(LimbSlot::HAND_LEFT), Some(new_l));
        assert_eq!(rig.get(LimbSlot::HAND_RIGHT), Some(new_r));
        for (hand, slot) in [(new_l, LimbSlot::HAND_LEFT), (new_r, LimbSlot::HAND_RIGHT)] {
            let limb = world
                .get::<Limb>(hand)
                .expect("the rebuilt hand carries a Limb");
            assert_eq!(limb.of, new_host, "rewired onto the NEW host generation");
            assert_eq!(limb.slot, slot);
        }
    }
}

/// Both new relations are in the registry dump, so a change to either one's
/// schema moves the prepared-content fingerprint.
#[test]
fn the_limb_and_mount_relations_reach_the_registry_dump() {
    let dump = engine_construction_registry().deterministic_dump();
    assert!(
        dump.contains("relation\tambition.limb\tambition_platformer2d_actor_monolith\tlimb-rig\t"),
        "{dump}"
    );
    assert!(
        dump.contains(
            "relation\tambition.mount\tambition_platformer2d_actor_monolith\tmount-link\t"
        ),
        "{dump}"
    );
}

// ── Authored mount links are planned relations ────────────────────────────────

/// A room with a shark mount and its pirate rider, linked the way LDtk's
/// `mounted_on` entity-ref lowers: `RoomSpec.mount_links = [(rider, mount)]`.
fn mounted_pair_room() -> ambition_platformer2d_world::rooms::RoomSpec {
    let mut room = empty_room("cove");
    room.enemy_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
        "sky_shark",
        "Burning Flying Shark",
        ae::Aabb::new(ae::Vec2::new(200.0, 100.0), ae::Vec2::new(63.0, 26.0)),
        ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom("fixture_mount".into()),
            "fixture_mount",
        ),
    ));
    room.enemy_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
        "sky_rider",
        "Pirate Raider",
        ae::Aabb::new(ae::Vec2::new(200.0, 40.0), ae::Vec2::new(22.0, 39.0)),
        ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom("pirate_raider".into()),
            "pirate_raider",
        ),
    ));
    room.mount_links
        .push(("sky_rider".to_string(), "sky_shark".to_string()));
    room
}

/// An authored mount link is a planned relation between two plan rows. The
/// deleted resolver matched the pair by `FeatureId` a frame after spawn; here
/// both actors are pulled into the planner and the rider row declares
/// `ambition.mount` before anything exists.
#[test]
fn an_authored_mount_link_becomes_planned_rows_with_a_mount_relation() {
    let plan = prepare(
        &mounted_pair_room(),
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the mounted room plans");

    let rider = SimId::placement("sky_rider");
    let mount = SimId::placement("sky_shark");
    let rider_row = plan
        .construction()
        .get(&rider)
        .expect("the rider is a plan row");
    assert!(matches!(
        rider_row.parameters(),
        ActorConstructionParams::AuthoredEnemy { .. }
    ));
    assert!(plan.construction().get(&mount).is_some());
    let dump = plan.construction().deterministic_dump();
    assert!(
        dump.contains("relation\tplacement:sky_rider\tambition.mount\tplacement:sky_shark\t-"),
        "{dump}"
    );
}

/// End to end: the pair commits, the room publishes, and BOTH ends of the weld
/// are live at the boundary — no frame-later resolution.
#[test]
fn a_committed_mount_pair_is_welded_both_ways_and_published() {
    let plan = prepare(
        &mounted_pair_room(),
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the mounted room plans");
    let mut app = commit(plan);

    let verification = app
        .world()
        .resource::<crate::world::rooms::LastConstructionVerification>();
    assert!(
        verification.published,
        "the mounted room publishes: {:?}",
        verification.violations
    );

    let world = app.world_mut();
    let find = |world: &mut World, wanted: &SimId| {
        let mut query = world.query::<(bevy::prelude::Entity, &SimId)>();
        query
            .iter(world)
            .find(|(_, sim)| *sim == wanted)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("`{wanted}` is live"))
    };
    let rider = find(world, &SimId::placement("sky_rider"));
    let mount = find(world, &SimId::placement("sky_shark"));
    assert_eq!(
        world
            .get::<ambition_mount::RidingOn>(rider)
            .map(|riding| riding.mount),
        Some(mount)
    );
    assert!(world.get::<ambition_mount::Mounted>(rider).is_some());
    assert_eq!(
        world
            .get::<ambition_mount::MountSlot>(mount)
            .and_then(|slot| slot.rider),
        Some(rider),
        "the mount points back — the half-write this campaign kept finding"
    );
}

/// The gnu_ton_rider pattern: a BOSS rider on a giant mount. The boss
/// becomes a planned row (`AuthoredBoss`, CanPilot from its profile), its
/// relation targets the giant HOST row the giant expansion already planned —
/// one row per identity, no duplicate.
#[test]
fn a_boss_rider_on_a_giant_becomes_a_planned_pair() {
    let mut room = giant_room();
    room.boss_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
        "the_rider",
        "gnu_ton_rider",
        ae::Aabb::new(ae::Vec2::new(100.0, 20.0), ae::Vec2::splat(30.0)),
        ambition_entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "gnu_ton_rider".into(),
        },
    ));
    room.mount_links
        .push(("the_rider".to_string(), "boss_mount".to_string()));
    let plan = prepare(
        &room,
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the boss-rider room plans");

    let rider = SimId::placement("the_rider");
    let host = SimId::placement("boss_mount");
    assert!(matches!(
        plan.construction()
            .get(&rider)
            .expect("the boss is a plan row")
            .parameters(),
        ActorConstructionParams::AuthoredBoss { .. }
    ));
    assert!(matches!(
        plan.construction()
            .get(&host)
            .expect("the giant host row exists ONCE")
            .parameters(),
        ActorConstructionParams::GiantHost { .. }
    ));

    // Commit: the whole cluster (host + hands + boss) publishes with the boss
    // welded onto the giant.
    let mut app = commit(plan);
    let verification = app
        .world()
        .resource::<crate::world::rooms::LastConstructionVerification>();
    assert!(
        verification.published,
        "the boss-rider room publishes: {:?}",
        verification.violations
    );
    let world = app.world_mut();
    let find = |world: &mut World, wanted: &SimId| {
        let mut query = world.query::<(bevy::prelude::Entity, &SimId)>();
        query
            .iter(world)
            .find(|(_, sim)| *sim == wanted)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("`{wanted}` is live"))
    };
    let rider_entity = find(world, &rider);
    let host_entity = find(world, &host);
    assert_eq!(
        world
            .get::<ambition_mount::RidingOn>(rider_entity)
            .map(|riding| riding.mount),
        Some(host_entity)
    );
    assert_eq!(
        world
            .get::<ambition_mount::MountSlot>(host_entity)
            .and_then(|slot| slot.rider),
        Some(rider_entity)
    );
}

/// A link naming nobody fails the room while it is whole. The deleted resolver
/// retried such a pair silently forever.
#[test]
fn a_mount_link_naming_nobody_fails_the_room_while_it_is_whole() {
    let mut room = mounted_pair_room();
    room.mount_links
        .push(("sky_rider_typo".to_string(), "sky_shark".to_string()));
    let error = prepare(
        &room,
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect_err("a dangling link cannot plan");
    assert!(
        matches!(
            error,
            RoomFeatureConstructionError::ActorConstruction(
                ActorConstructionError::MountLinkNamesNobody { ref id, .. }
            ) if id == "sky_rider_typo"
        ),
        "{error:?}"
    );
}

/// Two links claiming one mount are refused at preparation — the domain
/// preflight sees the planned relations, not a frame-later race.
#[test]
fn two_riders_claiming_one_authored_mount_are_refused() {
    let mut room = mounted_pair_room();
    room.enemy_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
        "second_rider",
        "Pirate Raider",
        ae::Aabb::new(ae::Vec2::new(260.0, 40.0), ae::Vec2::new(22.0, 39.0)),
        ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom("pirate_raider".into()),
            "pirate_raider",
        ),
    ));
    room.mount_links
        .push(("second_rider".to_string(), "sky_shark".to_string()));
    let error = prepare(
        &room,
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect_err("a double-claimed mount cannot plan");
    assert!(
        matches!(
            error,
            RoomFeatureConstructionError::ActorConstruction(
                ActorConstructionError::MountHasTwoRiders { .. }
            )
        ),
        "{error:?}"
    );
}

/// The reconstruction closure keeps the pair together: rebuilding either end
/// rebuilds both, so neither can strand the other on a dead entity handle.
#[test]
fn the_mount_pair_reconstruction_closure_is_both_actors() {
    let plan = prepare(
        &mounted_pair_room(),
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the mounted room plans");
    let rider = SimId::placement("sky_rider");
    let mount = SimId::placement("sky_shark");
    for seed in [&rider, &mount] {
        let closure = plan
            .construction()
            .relation_closure(&std::collections::BTreeSet::from([(*seed).clone()]));
        assert_eq!(
            closure,
            std::collections::BTreeSet::from([rider.clone(), mount.clone()]),
            "the closure of {seed} is the whole pair"
        );
    }
}

// ── Phase 4a/4b: the enemy and boss FAMILIES are plan rows ────────────────────

/// Every authored enemy and boss is a plan row. Ordinary enemy →
/// `AuthoredEnemy`; giant → host + two hands; boss → `AuthoredBoss`. The family
/// loops are deleted, so this is the only way a room's actors exist.
#[test]
fn every_authored_enemy_and_boss_is_a_plan_row() {
    let mut room = giant_room();
    room.enemy_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
        "walker",
        "Ordinary Walker",
        ae::Aabb::new(ae::Vec2::new(300.0, 40.0), ae::Vec2::new(22.0, 39.0)),
        ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
            "combatant",
        ),
    ));
    room.boss_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
        "warden",
        "clockwork warden",
        ae::Aabb::new(ae::Vec2::new(400.0, 100.0), ae::Vec2::splat(40.0)),
        ambition_entity_catalog::placements::BossBrain::Dormant,
    ));
    let plan = prepare(
        &room,
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the mixed room plans");

    let expect_kind = |id: &str, check: fn(&ActorConstructionParams) -> bool| {
        let row = plan
            .construction()
            .get(&SimId::placement(id))
            .unwrap_or_else(|| panic!("`{id}` is a plan row"));
        assert!(check(row.parameters()), "`{id}` has the right family");
    };
    expect_kind("walker", |parameters| {
        matches!(parameters, ActorConstructionParams::AuthoredEnemy { .. })
    });
    expect_kind("boss_mount", |parameters| {
        matches!(parameters, ActorConstructionParams::GiantHost { .. })
    });
    expect_kind("warden", |parameters| {
        matches!(parameters, ActorConstructionParams::AuthoredBoss { .. })
    });

    // The whole roster is planned: no enemy/boss id lives outside the plan.
    let planned: std::collections::BTreeSet<String> = plan
        .construction()
        .planned_ids()
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    for id in ["walker", "boss_mount", "warden"] {
        assert!(planned.contains(&SimId::placement(id).to_string()));
    }
}

/// The migrated actors are VISIBLE to the boundary verifier: identity, provenance, and transaction
/// ownership are stamped at construction, not assigned by `ensure_sim_id` after verification has
/// already run.
#[test]
fn a_committed_actor_room_is_fully_visible_at_the_boundary() {
    let mut room = empty_room("field");
    room.enemy_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
        "walker",
        "Ordinary Walker",
        ae::Aabb::new(ae::Vec2::new(300.0, 40.0), ae::Vec2::new(22.0, 39.0)),
        ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
            "combatant",
        ),
    ));
    room.boss_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
        "warden",
        "clockwork warden",
        ae::Aabb::new(ae::Vec2::new(400.0, 100.0), ae::Vec2::splat(40.0)),
        ambition_entity_catalog::placements::BossBrain::Dormant,
    ));
    let plan = prepare(
        &room,
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the room plans");
    let mut app = commit(plan);

    let verification = app
        .world()
        .resource::<crate::world::rooms::LastConstructionVerification>();
    assert!(verification.published, "{:?}", verification.violations);
    assert_eq!(verification.violations, Vec::new());

    let world = app.world_mut();
    let mut query = world.query::<(
        &SimId,
        &ambition_platformer2d_shared_tangle::construction::SpawnOrigin,
    )>();
    let stamped: std::collections::BTreeSet<String> =
        query.iter(world).map(|(sim, _)| sim.to_string()).collect();
    for id in ["walker", "warden"] {
        assert!(
            stamped.contains(&SimId::placement(id).to_string()),
            "`{id}` carries SimId + SpawnOrigin at the boundary, not after it"
        );
    }
}

/// A planned BOSS reconstructs through the planner like any row — the
/// family-specific respawn fallbacks are deleted.
#[test]
fn a_boss_respawns_through_the_planner() {
    let mut room = empty_room("lair");
    room.boss_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
        "warden",
        "clockwork warden",
        ae::Aabb::new(ae::Vec2::new(400.0, 100.0), ae::Vec2::splat(40.0)),
        ambition_entity_catalog::placements::BossBrain::Dormant,
    ));
    let plan = prepare(
        &room,
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the room plans");
    let mut app = commit(plan.clone());
    let world = app.world_mut();
    let find = |world: &mut World, wanted: &SimId| {
        let mut query = world.query::<(bevy::prelude::Entity, &SimId)>();
        query
            .iter(world)
            .find(|(_, sim)| *sim == wanted)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("`{wanted}` is live"))
    };
    let warden = SimId::placement("warden");
    let old = find(world, &warden);
    world.despawn(old);
    let rebuilt = {
        let mut commands = world.commands();
        plan.respawn_authoritative_entity(&mut commands, SessionSpawnScope::UNSCOPED, "warden")
    };
    assert!(rebuilt, "the planned boss row rebuilds");
    world.flush();
    let fresh = find(world, &warden);
    assert_ne!(fresh, old);
    assert!(
        world
            .get::<ambition_platformer2d_shared_tangle::construction::SpawnOrigin>(fresh)
            .is_some(),
        "the rebuilt boss carries provenance"
    );
}

// ── Phase 4c: authored placements are plan rows ───────────────────────────────

fn placement_registry() -> crate::world::placements::PlacementLoweringRegistry {
    let mut registry = crate::world::placements::PlacementLoweringRegistry::default();
    registry
        .try_register(
            ambition_entity_catalog::placements::PlacementKind::Pickup,
            "ambition_platformer2d_actor_monolith",
            "test",
            "placement.pickup.v1",
            crate::features::ecs::spawn_static::lower_pickup_placement,
        )
        .unwrap();
    registry
        .try_register(
            ambition_entity_catalog::placements::PlacementKind::Interactable,
            "ambition_platformer2d_actor_monolith",
            "test",
            "placement.interactable.v1",
            crate::features::ecs::spawn_static::lower_interactable_placement,
        )
        .unwrap();
    registry
}

fn placement_room() -> ambition_platformer2d_world::rooms::RoomSpec {
    // `PickupKind` comes from the crate ROOT, not from `placements` — it is
    // public there and re-exported privately here, which `d3bd6e95a` exposed
    // when it deleted `PickupKindSpec`.
    use ambition_entity_catalog::placements::{
        HazardRespawn, InteractableSpec, InteractionKindSpec, PickupSpec, PlacementSchema,
    };
    use ambition_entity_catalog::PickupKind;
    let mut room = empty_room("gallery");
    room.placements
        .push(crate::world::placements::PlacementRecord::new(
            "ring_1",
            PlacementSchema::Pickup(PickupSpec {
                kind: PickupKind::Health { amount: 1 },
                respawn: HazardRespawn::Never,
                collected: false,
                sprite: None,
            }),
            ae::Aabb::new(ae::Vec2::new(64.0, 32.0), ae::Vec2::splat(8.0)),
        ));
    room.placements
        .push(crate::world::placements::PlacementRecord::new(
            "door_1",
            PlacementSchema::Interactable(InteractableSpec::new(
                "Enter",
                InteractionKindSpec::Door { target: None },
            )),
            ae::Aabb::new(ae::Vec2::new(128.0, 32.0), ae::Vec2::splat(16.0)),
        ));
    room
}

fn prepare_with_placements(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
) -> Result<RoomFeatureConstructionPlan, RoomFeatureConstructionError> {
    RoomFeatureConstructionPlan::prepare(
        room,
        &placement_registry(),
        &crate::features::RoomContentStagingRegistry::default(),
        &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        &Default::default(),
        &ambition_boss_encounter::test_boss_catalog(),
        ActorConstructionContext::new(&engine_construction_registry(), ae::ContentEpoch(4))
            .with_prepared(fixture_cast()),
    )
}

/// A spawning placement is a plan row; an inert one (a Door) is not. The
/// row carries the frozen interpreter; the Door record keeps its historical
/// no-entity behavior instead of becoming a fatal missing row.
#[test]
fn a_spawning_placement_is_a_plan_row_and_an_inert_one_is_skipped() {
    let plan = prepare_with_placements(&placement_room()).expect("the gallery plans");
    let ring = plan
        .construction()
        .get(&SimId::placement("ring_1"))
        .expect("the pickup placement is a plan row");
    assert!(matches!(
        ring.parameters(),
        ActorConstructionParams::Placement { .. }
    ));
    assert!(
        plan.construction()
            .get(&SimId::placement("door_1"))
            .is_none(),
        "a Door interactable spawns nothing and is not planned"
    );
}

/// The committed placement is stamped and verified at the boundary like every
/// plan row — the placement family leaves the invisible list.
#[test]
fn a_committed_placement_room_publishes_with_a_stamped_pickup() {
    let plan = prepare_with_placements(&placement_room()).expect("the gallery plans");
    let mut app = commit(plan);
    let verification = app
        .world()
        .resource::<crate::world::rooms::LastConstructionVerification>();
    assert!(verification.published, "{:?}", verification.violations);
    assert_eq!(verification.violations, Vec::new());

    let world = app.world_mut();
    let mut query = world.query::<(
        &SimId,
        &ambition_platformer2d_shared_tangle::construction::SpawnOrigin,
        &ambition_combat::components::FeatureId,
    )>();
    let ring = query
        .iter(world)
        .find(|(sim, _, _)| **sim == SimId::placement("ring_1"))
        .expect("the pickup body is live");
    assert_eq!(
        ring.2.as_str(),
        "ring_1",
        "identity and the interpreter-populated body are the SAME entity"
    );
}

/// A placement reconstructs through the planner — the `lower_one` fallback is
/// deleted with the rest of the family-specific respawn branches.
#[test]
fn a_placement_respawns_through_the_planner() {
    let plan = prepare_with_placements(&placement_room()).expect("the gallery plans");
    let mut app = commit(plan.clone());
    let world = app.world_mut();
    let find = |world: &mut World, wanted: &SimId| {
        let mut query = world.query::<(bevy::prelude::Entity, &SimId)>();
        query
            .iter(world)
            .find(|(_, sim)| *sim == wanted)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("`{wanted}` is live"))
    };
    let ring = SimId::placement("ring_1");
    let old = find(world, &ring);
    world.despawn(old);
    let rebuilt = {
        let mut commands = world.commands();
        plan.respawn_authoritative_entity(&mut commands, SessionSpawnScope::UNSCOPED, "ring_1")
    };
    assert!(rebuilt, "the planned placement row rebuilds");
    world.flush();
    assert_ne!(find(world, &ring), old);
}


/// Shrines and gravity zones always had stable authored iids; the entities now
/// wear them as plan rows, verified at the boundary like everything else.
///
/// and they are now plan rows in DIFFERENT LANES, which is the whole
/// point of the second extraction: the shrine is actor-domain vocabulary and the
/// gravity zone is the gravity capability's. One room, two independently typed
/// lanes, one transaction — so this test asks each lane for its own row rather
/// than asking the actor domain about a zone it no longer owns.
#[test]
fn shrines_and_gravity_zones_are_stamped_plan_rows() {
    let mut room = empty_room("garden");
    room.shrines.push(ambition_platformer2d_world::rooms::ShrineSpec {
        id: "rest_1".into(),
        name: "Rest".into(),
        pos: ae::Vec2::new(64.0, 32.0),
        half_extent: ae::Vec2::splat(16.0),
    });
    room.gravity_zones.push(ambition_platformer2d_world::rooms::GravityZoneSpec {
        id: "flip_1".into(),
        name: "Flip".into(),
        center: ae::Vec2::new(160.0, 96.0),
        half_extent: ae::Vec2::new(48.0, 96.0),
        dir: ae::Vec2::new(0.0, 1.0),
        oscillate_amplitude: 0.0,
        oscillate_freq: 0.0,
    });
    let plan = prepare(
        &room,
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the garden plans");
    assert!(
        plan.construction()
            .get(&SimId::placement("rest_1"))
            .is_some(),
        "the shrine is an actor-domain plan row"
    );
    assert!(
        plan.gravity_construction()
            .get(&SimId::placement("flip_1"))
            .is_some(),
        "the gravity zone is a plan row in the gravity capability's own lane"
    );
    assert!(
        plan.construction()
            .get(&SimId::placement("flip_1"))
            .is_none(),
        "the gravity zone is still planned by the ACTOR domain, so the extraction left \
         two owners for one identity"
    );

    let mut app = commit(plan);
    let verification = app
        .world()
        .resource::<crate::world::rooms::LastConstructionVerification>();
    assert!(verification.published, "{:?}", verification.violations);
    assert_eq!(verification.violations, Vec::new());

    let world = app.world_mut();
    let mut shrines = world.query::<(&SimId, &crate::shrine::HealShrine)>();
    let (sim, _) = shrines.iter(world).next().expect("the shrine is live");
    assert_eq!(
        *sim,
        SimId::placement("rest_1"),
        "identity and the populated shrine are the SAME entity"
    );

    // the other lane actually BUILT something, checked the same way: the
    // identity and the populated zone are one entity. A lane that planned a row
    // and constructed nothing would satisfy every assertion above.
    let mut zones = world.query::<(
        &SimId,
        &ambition_platformer2d_shared_tangle::gravity::GravityZone,
    )>();
    let (sim, zone) = zones.iter(world).next().expect("the gravity zone is live");
    assert_eq!(
        *sim,
        SimId::placement("flip_1"),
        "identity and the populated gravity zone are the SAME entity"
    );
    assert_eq!(
        zone.dir,
        ae::Vec2::new(0.0, 1.0),
        "the authored direction survived the lane"
    );
    assert_eq!(
        ae::AabbExt::half_size(zone.aabb),
        ae::Vec2::new(48.0, 96.0),
        "the authored region survived the lane"
    );
}


/// A plan prepared against one content generation must not publish a room into
/// a session live under another. Detection, not prevention — the world has
/// already been mutated — but the room never announces itself.
#[test]
fn a_stale_plans_room_is_refused_publication() {
    let plan = prepare(
        &giant_room(),
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the room plans (epoch 4)");
    let app = commit_over(plan, |world| {
        world.insert_resource(crate::world::rooms::ActiveContentBinding::content(
            ae::ContentEpoch(9),
        ));
    });
    let verification = app
        .world()
        .resource::<crate::world::rooms::LastConstructionVerification>();
    assert!(!verification.published, "a stale plan cannot publish");
    assert!(
        verification
            .violations
            .iter()
            .any(|violation| matches!(violation, RosterViolation::ContentBindingMismatch { .. })),
        "{:?}",
        verification.violations
    );
}

/// The matching binding publishes — the staleness check discriminates, it does
/// not blanket-refuse.
#[test]
fn a_plan_matching_the_live_binding_publishes() {
    let plan = prepare(
        &giant_room(),
        &crate::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the room plans (epoch 4)");
    let app = commit_over(plan, |world| {
        world.insert_resource(crate::world::rooms::ActiveContentBinding::content(
            ae::ContentEpoch(4),
        ));
    });
    let verification = app
        .world()
        .resource::<crate::world::rooms::LastConstructionVerification>();
    assert!(verification.published, "{:?}", verification.violations);
}

/// The room sweep is not merely available — the REAL prepare path runs it, on
/// both channels, and hands the result to the plan.
///
/// The patrol path is the one reference with no failure mode of its own: an
/// authored path that matches nothing leaves the enemy passive, silently, and
/// nothing else objects. That is what the sweep is for.
///
/// Held items are deliberately not part of this: `authored_static_requests`
/// already REFUSES an unknown `held_item` (`UnknownHeldItem`), which is stronger
/// than reporting it. The sweep keeps the namespace for callers outside
/// construction, but construction's own rule stays the authority.
#[test]
fn prepare_hands_the_plan_what_the_room_could_not_bind() {
    let mut room = empty_room("hall");
    room.kinematic_paths
        .push(ambition_platformer2d_world::rooms::KinematicPathSpec::new(
            "ledge_patrol",
            "Ledge Patrol",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)),
            ae::KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(64.0, 0.0), 30.0),
        ));
    let walker: ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::EnemySpawnSpec> = ambition_platformer2d_world::rooms::Authored::new(
        "walker_authored",
        "Walker",
        ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)),
        ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Patrol {
                path_id: Some("ledge_patrl".into()),
            },
            "fixture_walker",
        ),
    );
    room.enemy_spawns.push(walker);

    let mut staging = crate::features::RoomContentStagingRegistry::default();
    staging
        .register("hall", "test_provider", "typo", "typo.v1", |_room| {
            vec![staged_enemy("walker_a", None)]
        })
        .expect("stager registers");

    let plan = prepare(&room, &staging, &engine_construction_registry())
        .expect("an unbound reference is a content defect, not a reason to refuse the room");

    // ONE channel now (AC6). Construction now REFUSES an identifier that names no
    // character, which is a stronger statement than a report — so the patrol path is the
    // namespace this sweep still owns, and it is the one whose failure mode is still silent (a
    // bad path goes passive).
    let report = plan.binding_report();
    assert_eq!(report.len(), 1, "the patrol channel is swept:\n{report}");

    let found: Vec<_> = report
        .unresolved()
        .iter()
        .map(|u| (u.namespace, u.id.as_str()))
        .collect();
    assert_eq!(found, vec![("kinematic path", "ledge_patrl")]);
    assert_eq!(
        report.unresolved()[0].did_you_mean.as_deref(),
        Some("ledge_patrol"),
    );

    // The clean room next door reports nothing — the sweep is discriminating,
    // not just noisy.
    let (clean_room, clean_staging) = duelling_room();
    let clean =
        prepare(&clean_room, &clean_staging, &engine_construction_registry()).expect("prepares");
    assert!(
        clean.binding_report().is_empty(),
        "a room whose references all resolve says nothing:\n{}",
        clean.binding_report(),
    );
}

/// A body nothing can build REFUSES THE PLAN — it does not panic mid-commit.
///
/// the half AC6 left late. Deleting the archetype ontology made an
/// unresolvable character honest: there is no generic body left to settle for.
/// But the refusal it became lived inside `spawn_enemy_with_faction_into`, which
/// runs as a construction RECIPE — so the honest answer arrived as a panic after
/// the transaction had begun, with the outgoing room already retired and earlier
/// rows already spawned. `ConstructionDomain::dispatch`'s own doc says the
/// opposite: *"every lookup that could miss resolved in the request builder"*.
///
/// The three shapes, each a distinct fix for whoever authored it: names no
/// character at all, names one nobody registered, names one that is registered
/// and cannot build a body.
///
/// poison, and it is the half that matters: an enemy naming a REGISTERED
/// character must still prepare. A preflight that refused everything would pass
/// the three assertions above and refuse every shipping room.
#[test]
fn an_unbuildable_body_refuses_the_plan_before_anything_is_built() {
    let room_naming = |character: &str| {
        let mut room = empty_room("hall");
        let enemy: ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::EnemySpawnSpec> =
            ambition_platformer2d_world::rooms::Authored::new(
                "walker_authored",
                "Walker",
                ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)),
                ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                    ambition_entity_catalog::placements::CharacterBrain::Custom("wanderer".into()),
                    character,
                ),
            );
        room.enemy_spawns.push(enemy);
        room
    };
    let prepare_room = |room: &ambition_platformer2d_world::rooms::RoomSpec| {
        prepare(
            room,
            &crate::features::RoomContentStagingRegistry::default(),
            &engine_construction_registry(),
        )
    };

    // the first of the three shapes is GONE, and that is the point of the change rather than
    // a hole in this test. It asserted that a placement naming NO character was refused with
    // `BodyNamesNoCharacter`; `EnemySpawnSpec::character_id` is required now, so that placement
    // cannot be constructed at all, and the LDtk lowering refuses the authored entity by name
    // (`convert_enemy_spawn`, pinned in `conversion::mod`'s tests).
    assert!(
        matches!(
            prepare_room(&room_naming("iron_mary")),
            Err(RoomFeatureConstructionError::ActorConstruction(
                ActorConstructionError::BodyCharacterNotRegistered { .. }
            ))
        ),
        "a character this composition never registered would spawn a stranger \
         wearing her name — the defect D73 exists to end",
    );

    // Registered, and deliberately incomplete: no locomotion, so it cannot say
    // how it moves and `body_blueprint` refuses it.
    let mut cast = crate::character_runtime::PreparedCharacterRegistry::default();
    cast.insert_prepared(
        crate::character_runtime::prepare_and_finalize_for_test(
            crate::character_runtime::CharacterDefinition::new("npc_mute", "Mute", "test"),
            &crate::character_runtime::CharacterBindings::default(),
        )
        .prepared,
    );
    let incomplete = RoomFeatureConstructionPlan::prepare(
        &room_naming("npc_mute"),
        &Default::default(),
        &crate::features::RoomContentStagingRegistry::default(),
        &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        &Default::default(),
        &ambition_boss_encounter::test_boss_catalog(),
        ActorConstructionContext::new(&engine_construction_registry(), ae::ContentEpoch(4))
            .with_prepared(&cast),
    );
    assert!(
        matches!(
            incomplete,
            Err(RoomFeatureConstructionError::ActorConstruction(
                ActorConstructionError::BodyCharacterIsIncomplete { .. }
            ))
        ),
        "a registered character missing the facts a body needs is a DIFFERENT \
         fix from an unregistered one, and the diagnostic has to say which",
    );

    prepare_room(&room_naming("fixture_walker")).expect(
        "and a registered, body-complete character prepares — or the three above are \
                 a preflight that refuses everything",
    );
}

/// A STAGED actor takes its mount facts from its CHARACTER, exactly as an
/// authored one does.
///
/// `SpawnActorKind: Enemy` has carried a `character` since P1.12 and its own doc names the
/// consequence: *"A shark spawned this way stopped being rideable and started falling out of
/// the sky."* That was repaired at the SPAWN — so the body was built rideable and PLANNED
/// un-rideable, and the plan is what the mount-link legality rules are checked against.
///
/// the fixture's roster answers `combatant` for this brain — a row with no
/// mount at all — so the only thing that can make this request rideable is the
/// character, and the control below proves it by removing the cast.
#[test]
fn a_staged_actor_takes_its_mount_from_the_character_it_names() {
    let mut request = staged_enemy("staged_shark", None);
    let SpawnActorKind::Enemy { character, .. } = &mut request.kind else {
        unreachable!("staged_enemy builds an Enemy request")
    };
    *character = ambition_entity_catalog::CharacterId::new("npc_test_shark");

    let finalized = crate::character_runtime::prepare_and_finalize_for_test(
        crate::character_runtime::CharacterDefinition::new("npc_test_shark", "Shark", "test")
            .with_mount(ambition_characters::actor::CharacterMount {
                class: Some("shark".to_string()),
                ..Default::default()
            }),
        &crate::character_runtime::CharacterBindings::default(),
    );
    let mut cast = crate::character_runtime::PreparedCharacterRegistry::default();
    cast.insert_prepared(finalized.prepared);

    let bosses = ambition_boss_encounter::BossCatalog::default();
    let params = ActorConstructionParams::StagedActor(request.clone());

    let planned = crate::construction::mount_capabilities_of(&params, &bosses, Some(&cast));
    assert_eq!(
        planned.mount_class.as_deref(),
        Some("shark"),
        "the character authors a mount class and the plan must carry it"
    );

    // the control, and it is what makes the assertion above mean anything:
    // with no cast the same request falls to the archetype, which states no
    // mount. If this ALSO said `shark`, the roster would be answering and the
    // character would be decorative.
    let without_cast = crate::construction::mount_capabilities_of(&params, &bosses, None);
    assert_eq!(
        without_cast.mount_class, None,
        "the fixture roster must state no mount for this brain, or this test is \
         not measuring the character"
    );
}
