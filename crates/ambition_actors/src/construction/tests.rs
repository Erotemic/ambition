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
///
/// Brackets the work with the SAME transaction open/close
/// `RoomConstructionPlan::spawn_contents` uses, because that is where the
/// boundary lives: the feature plan does not publish, and a harness that called
/// it alone would verify nothing.
fn commit(plan: RoomFeatureConstructionPlan) -> App {
    commit_over(plan, |_| {})
}

/// As [`commit`], with `seed` run against the world FIRST — before the
/// transaction opens, so whatever it spawns is part of the opening baseline.
fn commit_over(plan: RoomFeatureConstructionPlan, seed: impl FnOnce(&mut World)) -> App {
    let mut app = App::new();
    app.add_message::<crate::rooms::RoomLoaded>();
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
            plan.construction(),
            receipt.construction(),
            plan.room().id.clone(),
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
        "construction-plan-v3\n\
         epoch:4\n\
         room\thall\n\
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
/// and no longer hunts its rival. `respawn_authoritative_entity` rebuilds the
/// RELATION CLOSURE, so asking for one duellist rebuilds **both** and the grudge
/// is wired.
///
/// This used to REFUSE (the closure was not expanded, so `construct_one` hit
/// `RelationCutBySubset`). Rebuilding the closure is the better contract: it is
/// the only way to bring a related row back correctly, and it is exactly what the
/// giant host + hands need. Both duellists come back with the grudge intact.
#[test]
fn rebuilding_one_duellist_rebuilds_its_grudge_partner_too() {
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
        staged_actor_requests(
            "hall",
            "prov",
            &[staged_enemy("staged", None)],
            &crate::features::enemies::test_roster(),
        )
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
    let staged = staged_actor_requests(
        "hall",
        "prov",
        &[staged_enemy("staged", None)],
        &crate::features::enemies::test_roster(),
    )
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
fn room_loaded_count(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<crate::rooms::RoomLoaded>>()
        .drain()
        .count()
}

/// **The room is not published when its relations did not land.**
///
/// **A room that fails verification does not publish, and does not write
/// `RoomLoaded`.**
///
/// The failure here is a real production shape rather than an injected one: an
/// entity already holds an identity this room plans, so committing the room
/// creates a second body for it — `PlannedOverBaseline`. Nothing test-only is
/// wired into the construction path to produce it.
///
/// It used to be produced by registering deliberately broken `RelationOps` into
/// the registry ahead of the engine's own, which worked only because the
/// registry stored executable behaviour and treated identical metadata as
/// idempotent — the first-wins hazard itself. That hazard is gone, so the seam
/// is gone with it; relation-postcondition detection is proven against the toy
/// domain in `ambition_platformer_primitives` and, for the real limb and mount
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
        verification.fatal().any(|violation| matches!(
            violation,
            ambition_platformer_primitives::construction::RosterViolation::PlannedOverBaseline {
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

// ── Bidirectional relations (Phase 4, first migration) ───────────────────────
//
// `Limb`/`LimbRig` and `RidingOn`/`MountSlot` are each TWO components that must
// agree. Every test here checks both sides, because the way these pairs have
// historically broken is one side landing and the other not — a failure that
// every forward-only assertion passes straight through.

use ambition_platformer_primitives::construction::{
    verify_committed_roster, AuthoritativeScope, ConstructionReceipt, RelationCheck,
    RelationRequest, RosterViolation, TransactionBaseline,
};

fn dynamic_scope() -> ConstructionScope {
    ConstructionScope {
        binding: ambition_platformer_primitives::construction::ContentBinding::RuntimeDynamic,
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
            &crate::features::enemies::test_roster(),
        ),
        boss_catalog: crate::boss_encounter::test_boss_catalog().clone(),
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
        let mut ctx = ambition_platformer_primitives::construction::ConstructionExecCtx {
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
    let transaction = plan.scope().transaction(SessionSpawnScope::UNSCOPED);
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
    world.entity_mut(mount).insert(crate::features::Mountable {
        rider_offset: ae::Vec2::ZERO,
        class: crate::features::MountClass("giant".into()),
        control_grant: crate::features::ControlGrant::Total,
        death_impact: crate::features::MountDeathImpact::Dismount,
    });
    world.entity_mut(rider).insert(crate::features::CanPilot {
        classes: vec![crate::features::MountClass("giant".into())],
    });
}

fn hand(slot: crate::features::LimbSlot) -> ActorRelation {
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
        hand(crate::features::LimbSlot::HandLeft),
    );
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let limb = receipt.entity(&SimId::placement("hand")).expect("built");
    let host = receipt.entity(&SimId::placement("giant")).expect("built");

    let attached = world
        .get::<crate::features::Limb>(limb)
        .expect("the limb side landed");
    assert_eq!(attached.of, host);
    assert_eq!(attached.slot, crate::features::LimbSlot::HandLeft);
    assert_eq!(attached.home_offset, ae::Vec2::new(12.0, -4.0));

    let rig = world
        .get::<crate::features::LimbRig>(host)
        .expect("the host side landed");
    assert_eq!(
        rig.get(crate::features::LimbSlot::HandLeft),
        Some(limb),
        "the rig files the limb under exactly its planned slot"
    );
    assert_eq!(rig.limbs.len(), 1, "and drives no other limb");

    assert_eq!(verify_bare(&mut world, &plan, &receipt, &baseline), Ok(()));
}

/// **A limb the host's rig does not contain is inert but looks attached.**
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
        hand(crate::features::LimbSlot::HandRight),
    );
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let host = receipt.entity(&SimId::placement("giant")).expect("built");

    // Exactly the half-write: strip the reverse side, leave the forward one.
    world.entity_mut(host).remove::<crate::features::LimbRig>();

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
        hand(crate::features::LimbSlot::HandLeft),
    );
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let limb = receipt.entity(&SimId::placement("hand")).expect("built");
    let host = receipt.entity(&SimId::placement("giant")).expect("built");

    world.entity_mut(limb).insert(crate::features::Limb {
        of: host,
        slot: crate::features::LimbSlot::HandRight,
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
///
/// The rig is a `Vec` and `fan_out_limb_intents` reads it positionally, so the
/// order is content, not incident. Canonical order sorts by the limb's `SimId`,
/// which is why the two hands file under their two distinct slots regardless of
/// declaration order.
#[test]
fn two_limbs_accumulate_into_one_rig_keyed_by_slot() {
    let giant = SimId::placement("giant");
    let mut host = bare_request("giant");
    host.relations.clear();
    let mut left = bare_request("giant/0");
    left.relations.push(RelationRequest {
        to: giant.clone(),
        relation: hand(crate::features::LimbSlot::HandLeft),
    });
    let mut right = bare_request("giant/1");
    right.relations.push(RelationRequest {
        to: giant.clone(),
        relation: hand(crate::features::LimbSlot::HandRight),
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
        .get::<crate::features::LimbRig>(host_entity)
        .expect("the rig accumulated");
    assert_eq!(
        rig.get(crate::features::LimbSlot::HandLeft),
        Some(left_entity)
    );
    assert_eq!(
        rig.get(crate::features::LimbSlot::HandRight),
        Some(right_entity)
    );
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
            .get::<crate::features::RidingOn>(rider)
            .expect("the rider side landed")
            .mount,
        mount
    );
    assert!(
        world.get::<crate::features::Mounted>(rider).is_some(),
        "the rider is marked mounted"
    );
    assert_eq!(
        world
            .get::<crate::features::MountSlot>(mount)
            .expect("the mount side landed")
            .rider,
        Some(rider)
    );
    equip_mount_pair(&mut world, rider, mount);
    assert_eq!(verify_bare(&mut world, &plan, &receipt, &baseline), Ok(()));
}

/// **The half-write that exists in the tree today.**
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
        .remove::<crate::features::MountSlot>();

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

    world.entity_mut(mount).insert(crate::features::MountSlot {
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

/// **A limb wired into the wrong slot is detected — the slot is verified on
/// both sides.**
#[test]
fn a_limb_filed_under_the_wrong_slot_is_detected() {
    let plan = related_actor_plan(
        &["giant", "hand"],
        "hand",
        "giant",
        hand(crate::features::LimbSlot::HandLeft),
    );
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let host = receipt.entity(&SimId::placement("giant")).expect("built");
    let limb = receipt.entity(&SimId::placement("hand")).expect("built");

    // File the same limb under the OTHER slot, leaving `Limb.slot` right.
    let mut rig = world
        .get_mut::<crate::features::LimbRig>(host)
        .expect("the rig landed");
    rig.limbs.clear();
    rig.limbs.insert(crate::features::LimbSlot::HandRight, limb);

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

/// **A limb whose home offset was overwritten after wiring is detected.**
///
/// The offset is the limb's entire idle behaviour; a corrupted one station-keeps
/// to the wrong place forever, which no structural check would ever notice. This
/// is the poison counterpart to `a_limb_relation_wires_the_limb_and_the_hosts_rig`,
/// which asserts the offset lands: it did not fail before this check existed,
/// because nothing read the offset back.
#[test]
fn a_limb_with_a_corrupted_home_offset_is_detected() {
    let plan = related_actor_plan(
        &["giant", "hand"],
        "hand",
        "giant",
        hand(crate::features::LimbSlot::HandLeft),
    );
    let (mut world, receipt, baseline) = commit_bare(&plan);
    let limb = receipt.entity(&SimId::placement("hand")).expect("built");

    world
        .get_mut::<crate::features::Limb>(limb)
        .unwrap()
        .home_offset = ae::Vec2::new(999.0, 999.0);

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

/// **A mount link missing `Mounted` is detected.**
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

    world.entity_mut(rider).remove::<crate::features::Mounted>();

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

/// **A mount link whose rider cannot pilot the mount's class is detected.**
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
    world.entity_mut(mount).insert(crate::features::Mountable {
        rider_offset: ae::Vec2::ZERO,
        class: crate::features::MountClass("shark".into()),
        control_grant: crate::features::ControlGrant::Total,
        death_impact: crate::features::MountDeathImpact::Dismount,
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
            feature_id: id.to_string(),
            name: id.to_string(),
            pos: ae::Vec2::ZERO,
            half_size: ae::Vec2::splat(10.0),
            archetype_id: archetype.to_string(),
            encounter_id: "e".into(),
            faction: crate::features::ActorFaction::Enemy,
        },
    )
}

fn preflight(requests: Vec<ActorConstructionRequest>) -> Result<(), ActorConstructionError> {
    preflight_actor_relations(
        &requests,
        &crate::features::enemies::test_roster(),
        &crate::boss_encounter::test_boss_catalog(),
    )
}

/// Two limbs claiming one host slot is refused before any spawn.
#[test]
fn two_limbs_in_one_slot_are_rejected() {
    let host = minion_request("giant", "giant_gnu");
    let mut a = minion_request("hand_a", "giant_gnu_hands");
    let mut b = minion_request("hand_b", "giant_gnu_hands");
    a.relations.push(RelationRequest {
        to: host.sim_id.clone(),
        relation: hand(crate::features::LimbSlot::HandLeft),
    });
    b.relations.push(RelationRequest {
        to: host.sim_id.clone(),
        relation: hand(crate::features::LimbSlot::HandLeft),
    });
    assert!(matches!(
        preflight(vec![host, a, b]),
        Err(ActorConstructionError::LimbSlotTaken { .. })
    ));
}

/// One limb naming two hosts is refused.
#[test]
fn a_limb_with_two_hosts_is_rejected() {
    let host_a = minion_request("giant_a", "giant_gnu");
    let host_b = minion_request("giant_b", "giant_gnu");
    let mut limb = minion_request("hand", "giant_gnu_hands");
    limb.relations.push(RelationRequest {
        to: host_a.sim_id.clone(),
        relation: hand(crate::features::LimbSlot::HandLeft),
    });
    limb.relations.push(RelationRequest {
        to: host_b.sim_id.clone(),
        relation: hand(crate::features::LimbSlot::HandRight),
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
    let mount = minion_request("shark", "burning_flying_shark");
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
    let mount_a = minion_request("shark_a", "burning_flying_shark");
    let mount_b = minion_request("shark_b", "burning_flying_shark");
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
    let mount = minion_request("giant", "giant_gnu");
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
    let mount = minion_request("shark", "burning_flying_shark");
    rider.relations.push(RelationRequest {
        to: mount.sim_id.clone(),
        relation: ActorRelation::Mount,
    });
    assert_eq!(preflight(vec![rider, mount]), Ok(()));
}

// ── Giant hands are explicit plan rows (Checkpoint B) ─────────────────────────

fn giant_room() -> crate::rooms::RoomSpec {
    let mut room = empty_room("arena");
    room.enemy_spawns.push(crate::rooms::Authored::new(
        "boss_mount",
        "Giant GNU",
        ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::splat(60.0)),
        ambition_entity_catalog::placements::CharacterBrain::Custom("giant_gnu".into()),
    ));
    room
}

/// **The giant host and both hands are explicit plan rows joined by limb
/// relations.** They used to be minted inside the enemy spawn helper as
/// authoritative roots no plan named — the last legacy family.
#[test]
fn a_giant_enemy_becomes_a_host_row_and_two_hand_rows() {
    let roster = crate::features::enemies::test_roster();
    let requests = crate::construction::authored_giant_requests(&giant_room(), &roster, &[]);

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
    let roster = crate::features::enemies::test_roster();
    let requests = crate::construction::authored_giant_requests(&giant_room(), &roster, &[]);
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
        .get::<crate::features::LimbRig>(host_entity)
        .expect("the host carries a rig");
    assert_eq!(rig.get(crate::features::LimbSlot::HandLeft), Some(hand_l));
    assert_eq!(rig.get(crate::features::LimbSlot::HandRight), Some(hand_r));
    assert_eq!(rig.limbs.len(), 2);
    // The host owns the router's scratch state.
    assert!(world
        .get::<crate::features::LimbIntents>(host_entity)
        .is_some());
    assert!(world
        .get::<crate::features::LimbRouteState>(host_entity)
        .is_some());

    assert_eq!(verify_bare(&mut world, &plan, &receipt, &baseline), Ok(()));
}

/// **Reconstruction closure: asking to rebuild any one of the three rebuilds all
/// three.** The giant host is a relation target and each hand a source, so no
/// one of them can be rebuilt alone — the closure holds the cluster together.
#[test]
fn the_giant_reconstruction_closure_is_the_whole_cluster() {
    let roster = crate::features::enemies::test_roster();
    let requests = crate::construction::authored_giant_requests(&giant_room(), &roster, &[]);
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

/// No family is enumerated as legacy any more — the list emptied when the giant
/// hands migrated. Deletes with `Severity::Unmigrated` at Phase 4's end.
#[test]
fn the_legacy_family_list_is_empty() {
    assert!(
        ambition_platformer_primitives::construction::KNOWN_LEGACY_FAMILIES.is_empty(),
        "the giant hands were the last legacy family"
    );
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
            brain: ambition_entity_catalog::placements::CharacterBrain::Custom("giant_gnu".into()),
        },
    }
}

/// **A provider-staged giant lowers to the SAME three-row cluster an authored
/// one does.** Before this, `staged_actor_requests` emitted a single
/// `StagedActor` row whose recipe routed through the enemy spawn helper — which
/// no longer spawns hands — so a staged giant was a handless host.
#[test]
fn a_staged_giant_becomes_a_host_row_and_two_hand_rows() {
    let roster = crate::features::enemies::test_roster();
    let requests = staged_actor_requests("hall", "prov", &[staged_giant("gnu")], &roster);

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
    let roster = crate::features::enemies::test_roster();
    let requests = staged_actor_requests("hall", "prov", &[staged_enemy("npc", None)], &roster);
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
        .get::<crate::features::LimbRig>(host_entity)
        .expect("the staged host carries a rig");
    assert_eq!(rig.get(crate::features::LimbSlot::HandLeft), Some(hand_l));
    assert_eq!(rig.get(crate::features::LimbSlot::HandRight), Some(hand_r));
}

/// **The authored giant host carries the room's frozen kinematic paths** — the
/// same seed data an ordinary authored enemy receives. The first migration
/// passed `Vec::new()`, silently un-pathing every giant.
#[test]
fn an_authored_giant_host_carries_the_rooms_frozen_paths() {
    let mut room = giant_room();
    room.kinematic_paths
        .push(ambition_world::rooms::KinematicPathSpec::new(
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
    let roster = crate::features::enemies::test_roster();
    let root = {
        let mut commands = world.commands();
        crate::features::ecs::spawn_runtime_minion(
            &mut commands,
            &catalog,
            &roster,
            SessionSpawnScope::UNSCOPED,
            "runaway",
            "Giant GNU",
            ae::Vec2::ZERO,
            ae::Vec2::splat(60.0),
            "giant_gnu",
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
    let roster = crate::features::enemies::test_roster();
    {
        let mut commands = world.commands();
        crate::features::spawn_encounter_mob(
            &mut commands,
            &catalog,
            &roster,
            SessionSpawnScope::UNSCOPED,
            "enc",
            "wave_gnu".to_string(),
            ambition_entity_catalog::placements::CharacterBrain::Custom("giant_gnu".into()),
            ae::Vec2::ZERO,
            ae::Vec2::splat(120.0),
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
    let roster = crate::features::enemies::test_roster();
    let requests = crate::construction::authored_giant_requests(&giant_room(), &roster, &[]);
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
        assert_eq!(
            fault.severity(),
            ambition_platformer_primitives::construction::Severity::Fatal,
            "a composition fault is unpublishable"
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
        .get_mut::<crate::features::LimbRig>(host_entity)
        .expect("rig")
        .limbs
        .insert(crate::features::LimbSlot::HandRight, interloper);
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
        .get_mut::<crate::features::LimbRig>(host_entity)
        .expect("rig")
        .limbs
        .insert(crate::features::LimbSlot::HandRight, hand_l);
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
        .get_mut::<crate::features::LimbRig>(host_entity)
        .expect("rig")
        .limbs
        .remove(&crate::features::LimbSlot::HandLeft);
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
        let mut rig = world
            .get_mut::<crate::features::LimbRig>(host_entity)
            .expect("rig");
        rig.limbs
            .insert(crate::features::LimbSlot::HandLeft, hand_r);
        rig.limbs
            .insert(crate::features::LimbSlot::HandRight, hand_l);
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
        .get_mut::<crate::features::Limb>(hand_l)
        .expect("the hand carries a Limb")
        .of = stale;
    assert!(rig_faults(&plan, &receipt, &world) > 0);
}

// ── Reconstruction from a stable identity ─────────────────────────────────────

/// **Production reconstruction can start from ANY cluster member — host, left
/// hand, or right hand.** The authored-id entry point spells only
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
            .get::<crate::features::LimbRig>(new_host)
            .expect("the rebuilt host carries a rig");
        assert_eq!(rig.get(crate::features::LimbSlot::HandLeft), Some(new_l));
        assert_eq!(rig.get(crate::features::LimbSlot::HandRight), Some(new_r));
        for (hand, slot) in [
            (new_l, crate::features::LimbSlot::HandLeft),
            (new_r, crate::features::LimbSlot::HandRight),
        ] {
            let limb = world
                .get::<crate::features::Limb>(hand)
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
        dump.contains("relation\tambition.limb\tambition_actors\tlimb-rig\t"),
        "{dump}"
    );
    assert!(
        dump.contains("relation\tambition.mount\tambition_actors\tmount-link\t"),
        "{dump}"
    );
}
