//! **The Phase-3 exit criteria, proven against the three real families.**
//!
//! The pure planner's own properties are proven in
//! `ambition_platformer_primitives::construction`. These tests are about the
//! actor domain: that a real authored room, a real provider stager, and a real
//! summon all go through that planner; that the failures which used to be
//! silent skips now stop the room before it is torn down; and that
//! reconstruction runs the same recipe ordinary construction does.

use ambition_platformer_primitives::construction::{
    ConstructionError, ConstructionPlan, ConstructionScope, SpawnOrigin,
};
use ambition_platformer_primitives::lifecycle::SessionSpawnScope;
use ambition_platformer_primitives::sim_id::SimId;
use bevy::prelude::{App, Commands, Update, World};

use super::*;
use crate::features::{
    ActorConstructionContext, RoomFeatureConstructionError, RoomFeatureConstructionPlan,
};
use ambition_engine_core as ae;

const REAL_HELD_ITEM: &str = "gun_sword";

fn empty_room(id: &str) -> crate::rooms::RoomSpec {
    crate::rooms::RoomSpec::new(
        id,
        ae::World::new(id, ae::Vec2::splat(1000.0), ae::Vec2::ZERO, Vec::new()),
    )
}

fn ground_item(id: &str, held_item: &str) -> crate::rooms::GroundItemSpec {
    crate::rooms::GroundItemSpec {
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
        },
    }
}

/// A room with both planned authored + staged families, staged by a named
/// provider so the resulting provenance is real rather than a placeholder.
fn duelling_room() -> (
    crate::rooms::RoomSpec,
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
    room: &crate::rooms::RoomSpec,
    staging: &crate::features::RoomContentStagingRegistry,
    recipes: &ActorConstructionRegistry,
) -> Result<RoomFeatureConstructionPlan, RoomFeatureConstructionError> {
    RoomFeatureConstructionPlan::prepare(
        room,
        &Default::default(),
        staging,
        &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        &crate::features::enemies::test_roster(),
        &crate::boss_encounter::test_boss_catalog(),
        ActorConstructionContext::new(recipes, ae::ContentEpoch(4)),
    )
}

/// Commit a prepared room plan into a real `App` and hand back the world.
fn commit(plan: RoomFeatureConstructionPlan) -> App {
    let mut app = App::new();
    app.add_message::<crate::rooms::RoomLoaded>();
    app.add_systems(Update, move |mut commands: Commands| {
        crate::features::spawn_room_feature_entities_from_plan(
            &mut commands,
            &plan,
            SessionSpawnScope::UNSCOPED,
        );
    });
    app.update();
    app
}

// ── All three origins, one planner ───────────────────────────────────────────

/// The authored ground item and the provider-staged actors land in ONE plan,
/// each stating the origin category it actually has. Before this, the staged
/// pair went out as deferred `SpawnActorRequest` messages and neither family
/// recorded where it came from at all.
#[test]
fn a_room_plans_its_authored_and_provider_staged_families_with_real_provenance() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");

    assert_eq!(
        plan.construction().deterministic_dump(),
        "construction-plan-v2\n\
         epoch:4\n\
         room\thall\n\
         entity\tplacement:duel_blue\tambition.staged-actor\tprovider-staged\ttest_provider\thall\tduel_blue\tstaged-actor duel_blue test_walker enemy\n\
         entity\tplacement:duel_red\tambition.staged-actor\tprovider-staged\ttest_provider\thall\tduel_red\tstaged-actor duel_red test_walker enemy\n\
         entity\tplacement:pickup_a\tambition.authored-ground-item\tauthored\thall\tpickup_a\tground-item pickup_a gun_sword\n\
         relation\tplacement:duel_blue\tambition.grudge\tplacement:duel_red\n\
         relation\tplacement:duel_red\tambition.grudge\tplacement:duel_blue\n",
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
    app.add_message::<crate::rooms::RoomLoaded>();
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

/// The staged duel's mutual grudge is wired from the plan's relations. The old
/// path did this in a post-spawn `wire_staged_grudges` pass over one message
/// batch; a foe in a different batch, or misspelled, was silently dropped.
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

// ── Failures that used to be silent ──────────────────────────────────────────

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
    assert_eq!(
        error,
        RoomFeatureConstructionError::ActorConstruction(ActorConstructionError::UnknownHeldItem {
            authored_id: "pickup_a".into(),
            item: "no_such_item".into(),
        })
    );
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

/// Exit criterion: *unresolved relations fail before mutation.* A
/// `grudge_against` naming nobody used to leave the fighter with no grudge and
/// no complaint.
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
    app.add_message::<crate::rooms::RoomLoaded>();
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
            feature_id: "slop_add".into(),
            name: "slop".into(),
            pos: ae::Vec2::ZERO,
            half_size: ae::Vec2::splat(8.0),
            archetype_id: "puppy_slug".into(),
            encounter_id: "enc_1".into(),
            faction: crate::features::ActorFaction::Enemy,
        },
    );

    let live: std::collections::BTreeSet<SimId> = [summoner.clone()].into_iter().collect();
    let plan = ConstructionPlan::<ActorConstruction>::prepare(
        ConstructionScope {
            binding: ambition_platformer_primitives::construction::ContentBinding::RuntimeDynamic,
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

/// Two summons from the same summoner take distinct identities even when the
/// authored summon spec reuses one feature id. Under the old path both landed
/// on `placement:{feature_id}` and collided outright.
#[test]
fn two_summons_from_one_summoner_do_not_collide() {
    let recipes = engine_construction_registry();
    let summoner = SimId::placement("boss_1");
    let params = || SummonedMinionParams {
        feature_id: "slop_add".into(),
        name: "slop".into(),
        pos: ae::Vec2::ZERO,
        half_size: ae::Vec2::splat(8.0),
        archetype_id: "puppy_slug".into(),
        encounter_id: "enc_1".into(),
        faction: crate::features::ActorFaction::Enemy,
    };
    let live: std::collections::BTreeSet<SimId> = [summoner.clone()].into_iter().collect();
    let plan = ConstructionPlan::<ActorConstruction>::prepare(
        ConstructionScope {
            binding: ambition_platformer_primitives::construction::ContentBinding::RuntimeDynamic,
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
            binding: ambition_platformer_primitives::construction::ContentBinding::RuntimeDynamic,
            room: None,
        },
        vec![summoned_minion_request(
            &summoner,
            0,
            SummonedMinionParams {
                feature_id: "slop_add".into(),
                name: "slop".into(),
                pos: ae::Vec2::ZERO,
                half_size: ae::Vec2::splat(8.0),
                archetype_id: "puppy_slug".into(),
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
// `apply_summon_effects` had no test at all before this. It is the only place
// the runtime-dynamic family actually reaches the world, so a change there
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

fn summon_world() -> World {
    let mut world = World::new();
    world.init_resource::<bevy::ecs::message::Messages<ambition_vfx::EffectRequest>>();
    world.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    world.insert_resource(crate::features::enemies::test_roster());
    world.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    world.insert_resource(engine_construction_registry());
    world
}

fn summon_spec(id: &str) -> ambition_vfx::SummonSpec {
    ambition_vfx::SummonSpec {
        id: id.to_string(),
        name: "slop".into(),
        pos: ae::Vec2::ZERO,
        half_size: ae::Vec2::splat(8.0),
        archetype_id: "puppy_slug".into(),
        encounter_id: "enc_1".into(),
        faction: ambition_vfx::HitSide::Enemy,
    }
}

/// A real summon reaches the world with a dynamic identity under its summoner
/// and provenance naming that summoner.
#[test]
fn a_summoned_minion_reaches_the_world_as_a_dynamic_child() {
    let mut world = summon_world();
    let boss = world
        .spawn((
            SimId::placement("boss_1"),
            ambition_platformer_primitives::sim_id::SimIdCounter::default(),
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
            ambition_platformer_primitives::sim_id::SimIdCounter::default(),
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
            .get::<ambition_platformer_primitives::sim_id::SimIdCounter>(boss)
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
/// and no longer hunts its rival. Refused rather than half-applied.
#[test]
fn rebuilding_one_duellist_alone_is_refused_because_its_grudge_would_be_lost() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");

    let mut app = App::new();
    app.add_message::<crate::rooms::RoomLoaded>();
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
        Some(false),
        "a relation-bearing row does not silently come back without its relation"
    );
    assert_eq!(
        app.world_mut().query::<&SimId>().iter(app.world()).count(),
        0,
        "the refusal happened before anything was built"
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
    app.add_message::<crate::rooms::RoomLoaded>();
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
/// This is what replaced the removed `AcceptsFn` tests. The recipe is derived from the
/// payload and construction is one exhaustive match, so a variant with no arm is
/// a compile error rather than a mid-commit panic — but "every arm actually
/// builds something" is still a behavioural claim, and this is it. A new
/// `ActorConstructionParams` variant that is planned but forgotten here shows up
/// as a missing identity, not as a green suite.
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
        staged_actor_requests("hall", "prov", &[staged_enemy("staged", None)])
            .pop()
            .expect("one request"),
        summoned_minion_request(
            &summoner,
            0,
            SummonedMinionParams {
                feature_id: "slop".into(),
                name: "slop".into(),
                pos: ae::Vec2::ZERO,
                half_size: ae::Vec2::splat(8.0),
                archetype_id: "puppy_slug".into(),
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
            binding: ambition_platformer_primitives::construction::ContentBinding::RuntimeDynamic,
            room: None,
        },
        requests,
        &live,
        &recipes,
    )
    .expect("every variant plans");

    let mut world = World::new();
    world.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    world.insert_resource(crate::features::enemies::test_roster());
    let services = ActorConstructionServices {
        context: crate::world::placements::ActorPlacementContext::new(
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &crate::features::enemies::test_roster(),
        ),
        boss_catalog: crate::boss_encounter::test_boss_catalog().clone(),
    };
    let planned = plan.planned_ids();
    {
        let mut commands = world.commands();
        let scope = plan.scope().clone();
        let mut ctx = ambition_platformer_primitives::construction::ConstructionExecCtx {
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
/// **Demonstrated against the pre-repair implementation** (which called
/// `counter.next()` while assembling requests): it failed there with `Some(1)`
/// where the contract requires `Some(0)`.
#[test]
fn a_rejected_summon_batch_spends_no_identity() {
    use ambition_platformer_primitives::sim_id::SimIdCounter;

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

/// A summoner with no `SimIdCounter` at all is refused before anything is built
/// — not discovered afterwards, when the minions already exist.
#[test]
fn a_summoner_without_a_counter_is_refused_before_spawning() {
    let mut world = summon_world();
    // Identified, but carrying no counter to reserve from.
    let boss = world.spawn(SimId::placement("boss_1")).id();

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
    use ambition_platformer_primitives::sim_id::SimIdCounter;

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
    use ambition_platformer_primitives::construction::ConstructionDomain;

    let mut room = empty_room("hall");
    room.ground_items
        .push(ground_item("pickup", REAL_HELD_ITEM));
    let ground = authored_ground_item_requests(&room)
        .expect("resolves")
        .pop()
        .expect("one request");
    let staged = staged_actor_requests("hall", "prov", &[staged_enemy("staged", None)])
        .pop()
        .expect("one request");
    let summoned = summoned_minion_request(
        &SimId::placement("boss_1"),
        0,
        SummonedMinionParams {
            feature_id: "slop".into(),
            name: "slop".into(),
            pos: ae::Vec2::ZERO,
            half_size: ae::Vec2::splat(8.0),
            archetype_id: "puppy_slug".into(),
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

/// The counter check happens INSIDE the same exclusive-world command that
/// builds the minions, so a mutation landing between the system running and its
/// command applying is caught with nothing built — rather than discovered after
/// the identities have already been handed out.
///
/// The window is real here, not simulated: `apply_summon_effects` queues its
/// commit, a second system writes the counter DIRECTLY (no commands, so no sync
/// point), and only then does the schedule reach the barrier where the commit
/// applies.
#[test]
fn a_counter_mutation_before_the_commit_applies_refuses_with_nothing_built() {
    use ambition_platformer_primitives::sim_id::SimIdCounter;
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
    // Bevy auto-inserts a sync point between a command-queueing system and a
    // later one, which would apply the summon's commit before the interloper
    // runs and close the very window under test. Turned off here deliberately:
    // the point is to reproduce the interleaving, not to rely on the scheduler
    // preventing it.
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

/// The engine's real recipes, but with the grudge's WIRING replaced by a no-op
/// while its metadata and its verifier stay exactly as production declares them.
///
/// This is a genuine injection into the production path rather than a
/// simulation of one: `prepare` reads whichever registry it is handed, and
/// `install_actor_construction_recipes` is idempotent under identical metadata,
/// so registering the broken ops FIRST leaves them in place while every other
/// engine recipe registers normally.
///
/// It also demonstrates the registration policy from the other side: identical
/// declared metadata makes these two ops interchangeable to the registry, which
/// is precisely why behaviour is governed by `schema_id` and why a silent
/// behaviour swap is the thing postcondition verification has to catch.
fn recipes_with_a_grudge_that_never_lands() -> ActorConstructionRegistry {
    fn wire_nothing(_from: Entity, _to: Entity, _ctx: &mut Ctx<'_, '_, '_>) {}
    let mut registry = ActorConstructionRegistry::default();
    registry
        .try_register_relation(
            relation_grudge(),
            OWNER,
            "aggression",
            SCHEMA,
            ambition_platformer_primitives::construction::RelationOps {
                wire: wire_nothing,
                verify: grudge_ops_for_tests().verify,
            },
        )
        .expect("the broken ops register first");
    install_actor_construction_recipes(&mut registry)
        .expect("the engine's recipes register idempotently over identical metadata");
    registry
}

fn room_loaded_count(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<crate::rooms::RoomLoaded>>()
        .drain()
        .count()
}

/// **The room is not published when its relations did not land.**
///
/// The receipt says the grudge was wired — the wiring function ran — and every
/// identity is present and correct. Only reading the committed components
/// catches this.
#[test]
fn a_room_whose_relation_never_lands_is_not_published() {
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes_with_a_grudge_that_never_lands())
        .expect("the room plans: the defect is in wiring, not in planning");
    let mut app = commit(plan);

    let verification = app
        .world()
        .resource::<crate::features::LastConstructionVerification>()
        .clone();
    assert!(
        !verification.published,
        "a room whose relations did not land must not publish: {verification:?}"
    );
    assert!(
        verification.fatal().any(|violation| matches!(
            violation,
            ambition_platformer_primitives::construction::RosterViolation::RelationNotEstablished {
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
        verification.fatal().next().is_none(),
        "a correctly wired room has no fatal violations: {:?}",
        verification.violations
    );
    assert!(verification.published, "{verification:?}");
    assert_eq!(
        room_loaded_count(&mut app),
        1,
        "a verified room publishes exactly once"
    );
}
