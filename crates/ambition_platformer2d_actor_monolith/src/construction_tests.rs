//! The Phase-3 exit criteria, proven against the three real families.
//!
//! The pure planner's own properties are proven in
//! `ambition_platformer2d_shared_tangle::construction`.

use ambition_characters::actor::limb::LimbSlot;
use ambition_platformer2d_shared_tangle::construction::{
    ConstructionError, ConstructionPlan, ConstructionScope, SpawnOrigin,
};
use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::{App, Commands, Update, World};
use ambition_platformer2d_construction::*;
// What the planner's module brought in when these lived inside it.
use ambition_platformer2d_actor_spawn::{SpawnActorKind, SpawnActorRequest};
use ambition_platformer2d_shared_tangle::construction::ContentBinding;
use crate::features::{
    ActorConstructionContext, RoomFeatureConstructionError, RoomFeatureConstructionPlan,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::construction::RelationRequest;
use crate::construction_fixtures::*;

/// The authored ground item and the provider-staged actors land in ONE plan, each stating the
/// origin category it actually has.
#[test]
fn a_room_plans_its_authored_and_provider_staged_families_with_real_provenance() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");

    assert_eq!(
        plan.construction().deterministic_dump(),
        "construction-plan-v5\n\
         epoch:4\n\
         expects\tepoch:4\n\
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
        // ⚠ NO BRACKET AT EITHER END, AND THE ROOTS ARE STILL VISIBLE BELOW: this
        // rig never calls `register_inactive_candidate_filter`, so the
        // `InactiveCandidate` stamp is an inert component here. That is the state
        // `OpenRefused::CandidateFilterNotInstalled` refuses in production, used
        // deliberately.
        let receipt = crate::features::spawn_room_feature_entities_from_plan(
            &mut commands,
            &plan,
            SessionSpawnScope::UNSCOPED,
            &ambition_platformer2d_construction::PersistedFates::unrecorded(),
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
        .query::<&ambition_held_items::GroundItem>()
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
            &ambition_platformer2d_construction::PersistedFates::unrecorded(),
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
        &ambition_platformer2d_construction::PersistedFates::unrecorded(),
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
            pos: ae::Vec2::ZERO,
            half_size: ae::Vec2::splat(8.0),
            character_id: "puppy_slug".into(),
            encounter_id: "enc_1".into(),
            faction: ambition_characters::actor::ActorFaction::Enemy,
        },
    );

    let live: std::collections::BTreeSet<SimId> = [summoner.clone()].into_iter().collect();
    let plan = ConstructionPlan::<ActorConstruction>::prepare(
        ConstructionScope::in_generation(ambition_platformer2d_shared_tangle::construction::ContentBinding::RuntimeDynamic, None),
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
        pos: ae::Vec2::ZERO,
        half_size: ae::Vec2::splat(8.0),
        character_id: "puppy_slug".into(),
        encounter_id: "enc_1".into(),
        faction: ambition_characters::actor::ActorFaction::Enemy,
    };
    let live: std::collections::BTreeSet<SimId> = [summoner.clone()].into_iter().collect();
    let plan = ConstructionPlan::<ActorConstruction>::prepare(
        ConstructionScope::in_generation(ambition_platformer2d_shared_tangle::construction::ContentBinding::RuntimeDynamic, None),
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
        ConstructionScope::in_generation(ambition_platformer2d_shared_tangle::construction::ContentBinding::RuntimeDynamic, None),
        vec![summoned_minion_request(
            &summoner,
            0,
            SummonedMinionParams {
                // A fixture keeps the character's authored vitals.
                health: None,
                // A boss minion keeps its character's hazard.
                keeps_contact_damage: true,
                feature_id: "slop_add".into(),
                pos: ae::Vec2::ZERO,
                half_size: ae::Vec2::splat(8.0),
                character_id: "puppy_slug".into(),
                encounter_id: "enc_1".into(),
                faction: ambition_characters::actor::ActorFaction::Enemy,
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

fn summon_world() -> World {
    let mut world = World::new();
    insert_summon_resources(&mut world);
    world
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
            &ambition_platformer2d_construction::PersistedFates::unrecorded(),
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
            &ambition_platformer2d_construction::PersistedFates::unrecorded(),
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

/// ⭐⭐ A WORLD MAY AUTHOR AN AXE ON THE GROUND, which it could not before
/// 2026-09-05.
///
/// ⛔⛔ THIS WAS A CONSTRUCTION REFUSAL THAT GOT RECORDED AS A DESIGN DECISION.
/// `authored_ground_item_requests` resolves `held_item` through
/// `ambition_characters::brain::held_item_by_id` and refuses an unknown id with
/// `UnknownHeldItem` -- and until the registries were merged, `axe` and
/// `javelin` were built in `ambition_held_items`, a crate this one DEPENDS on,
/// so they were unknown here. Placing an axe in an LDtk world was a refusal.
/// The custody page wrote that up as "axe and javelin reach a hand only through
/// the menu/grant road", which reads like an authoring choice and was a
/// consequence of where two tables happened to live.
///
/// ⚠ `gun_sword` is deliberately NOT the subject: it was a row in the narrow
/// table all along, so it would have passed before the merge and proves nothing
/// about it. The subjects are the two ids that could not resolve.
#[test]
fn a_room_may_author_the_weapons_that_used_to_live_in_the_other_registry() {
    for held in ["axe", "javelin"] {
        let mut room = empty_room("cove");
        room.ground_items.push(ground_item("dropped", held));

        let requests = authored_ground_item_requests(&room).unwrap_or_else(|error| {
            panic!(
                "a world authoring `{held}` on the ground must construct, and was \
                 refused: {error:?}"
            )
        });
        assert_eq!(
            requests.len(),
            1,
            "one authored ground item must yield one construction request"
        );
    }
}

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
            pos: ae::Vec2::ZERO,
            half_size: ae::Vec2::splat(8.0),
            character_id: "puppy_slug".into(),
            encounter_id: "enc".into(),
            faction: ambition_characters::actor::ActorFaction::Enemy,
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
            pos: ae::Vec2::ZERO,
            half_size: ae::Vec2::splat(10.0),
            character_id: archetype.to_string(),
            encounter_id: "e".into(),
            faction: ambition_characters::actor::ActorFaction::Enemy,
        },
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

/// A giant whose character says its rider conducts it is planned with hands
/// that are the seated rider's. With no rider seated, or on a giant that does
/// not say so, the hands are the giant's own.
#[test]
fn a_seated_conductor_is_planned_as_the_owner_of_the_hands() {
    let planned = |character: &str, seated: bool| {
        let mut room = giant_room();
        room.enemy_spawns[0].payload.character_id = character.into();
        if seated {
            room.boss_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
                "the_rider",
                "gnu_ton_rider",
                ae::Aabb::new(ae::Vec2::new(100.0, 20.0), ae::Vec2::splat(30.0)),
                ambition_entity_catalog::placements::BossBrain::PhaseScript {
                    script_id: "gnu_ton_rider".into(),
                },
            ));
            room.mount_links.push(("the_rider".to_string(), "boss_mount".to_string()));
        }
        let plan = prepare(
            &room,
            &crate::features::RoomContentStagingRegistry::default(),
            &engine_construction_registry(),
        )
        .expect("the giant room plans");
        let host_sim = SimId::placement("boss_mount");
        let ActorConstructionParams::GiantHost { conducted, .. } =
            plan.construction().get(&host_sim).expect("the host row").parameters()
        else {
            panic!("the host is a giant host row");
        };
        let hands: Vec<_> = (0..2)
            .map(|ordinal| {
                let row = plan.construction().get(&SimId::spawned(&host_sim, ordinal)).expect("a hand row");
                let ActorConstructionParams::GiantHand { conductor, .. } = row.parameters() else {
                    panic!("a hand is a giant hand row");
                };
                *conductor
            })
            .collect();
        (*conducted, hands)
    };
    let boss = Some(ambition_characters::actor::ActorFaction::Boss);
    assert_eq!(planned("fixture_conducted_giant", true), (true, vec![boss, boss]));
    assert_eq!(planned("fixture_conducted_giant", false), (false, vec![None, None]), "no rider is seated");
    assert_eq!(planned("fixture_giant", true), (false, vec![None, None]), "this giant's hands are its own");
}

/// A CHARACTER makes a limbed host by stating its hands, and names the
/// character each hand is: no archetype row, no mount class and no name in the
/// planner says it.
///
/// the fixture's roster deliberately answers `combatant` for this brain, so
/// the ONLY thing that can produce three rows here is the character. The hand
/// character is not the gnu's, so a planner that wrote a hand character of its
/// own is caught.
#[test]
fn a_character_states_which_character_its_hands_are() {
    let mut room = empty_room("arena");
    let mut authored: ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::EnemySpawnSpec,
    > = ambition_platformer2d_world::rooms::Authored::new(
        "boss_mount",
        "Giant GNU",
        ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::splat(60.0)),
        ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom("no_such_archetype".into()),
            "no_such_archetype",
        ),
    );
    authored.payload.character_id = ambition_entity_catalog::CharacterId::new("npc_giant");
    room.enemy_spawns.push(authored);

    let finalized = crate::character_runtime::prepare_and_finalize_for_test(
        ambition_characters::actor::definition::CharacterDefinition::new(
            "npc_giant",
            "Giant GNU",
            "test",
        )
        .with_mount(ambition_characters::actor::CharacterMount {
            class: Some("giant".to_string()),
            ..Default::default()
        }),
        &ambition_characters::prepared::CharacterBindings::default(),
    );
    // The class alone is a piloting fact: it plans one ordinary row.
    let mut classed = ambition_characters::prepared::PreparedCharacterRegistry::default();
    classed.insert_prepared(finalized.prepared.clone());
    assert_eq!(
        ambition_platformer2d_construction::authored_actor_requests(&room, &[], Some(&classed)).len(),
        1,
        "a mount class does not give a body hands"
    );
    let mut handed = finalized.prepared;
    handed.hands = Some(ambition_characters::actor::CharacterHands {
        character: "fixture_walker".to_string(),
        rest: None,
        more: Vec::new(),
    });
    let mut cast = ambition_characters::prepared::PreparedCharacterRegistry::default();
    cast.insert_prepared(handed);

    let without_cast = ambition_platformer2d_construction::authored_actor_requests(&room, &[], None);
    assert_eq!(
        without_cast.len(),
        1,
        "the control: with no cast prepared, the unknown brain is an ordinary \
         enemy — so three rows below cannot come from the roster"
    );

    let requests = ambition_platformer2d_construction::authored_actor_requests(&room, &[], Some(&cast));
    assert_eq!(
        requests.len(),
        3,
        "host + two hands, decided by the CHARACTER: {:?}",
        requests
            .iter()
            .map(|r| r.sim_id.clone())
            .collect::<Vec<_>>()
    );
    let hands: Vec<_> = requests
        .iter()
        .filter_map(|request| match &request.parameters {
            ActorConstructionParams::GiantHand { authored, .. } => {
                Some((authored.payload.character_id.as_str().to_string(), authored.name.clone()))
            }
            _ => None,
        })
        .collect();
    let hand = ("fixture_walker".to_string(), "Giant GNU Hand".to_string());
    assert_eq!(hands, vec![hand.clone(), hand], "each hand is the character its host names");
}

/// The giant host and both hands are explicit plan rows joined by limb
/// relations. They used to be minted inside the enemy spawn helper as
/// authoritative roots no plan named — the last legacy family.
#[test]
fn a_giant_enemy_becomes_a_host_row_and_two_hand_rows() {
    let requests =
        ambition_platformer2d_construction::authored_actor_requests(&giant_room(), &[], Some(fixture_cast()));

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

/// Reconstruction closure: asking to rebuild any one of the three rebuilds all
/// three. The giant host is a relation target and each hand a source, so no
/// one of them can be rebuilt alone — the closure holds the cluster together.
#[test]
fn the_giant_reconstruction_closure_is_the_whole_cluster() {
    let requests =
        ambition_platformer2d_construction::authored_actor_requests(&giant_room(), &[], Some(fixture_cast()));
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
            ae::Vec2::ZERO,
            ae::Vec2::splat(60.0),
            "fixture_giant",
            "enc",
            ambition_characters::actor::ActorFaction::Enemy,
            ambition_combat::components::ActorAggression::hostile(),
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
            ambition_encounter::mob_seed::EncounterMobSeed {
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

/// A cut-rope arena that does not author the anvil its boss's script drops.
fn cut_rope_room(props: &[&str]) -> ambition_platformer2d_world::rooms::RoomSpec {
    let mut room = empty_room("arena");
    room.boss_spawns
        .push(ambition_platformer2d_world::rooms::Authored::new(
            "behemoth",
            "smirking_behemoth_boss",
            ae::Aabb::new(ae::Vec2::new(100.0, 20.0), ae::Vec2::splat(60.0)),
            ambition_entity_catalog::placements::BossBrain::PhaseScript {
                script_id: "smirking_behemoth_boss".into(),
            },
        ));
    room.props = props
        .iter()
        .map(|kind| ambition_platformer2d_world::rooms::PropSpec {
            id: format!("iid-{kind}"),
            name: kind.to_string(),
            kind: kind.to_string(),
            pos: ae::Vec2::new(300.0, 200.0),
            size: ae::Vec2::new(40.0, 30.0),
            flip_y: false,
            draw: Default::default(),
        })
        .collect();
    room
}

/// A boss script that names a prop the room does not author fails the room
/// while it is whole. At run time the drop found no anvil, did nothing, and
/// still consumed its beat, so the fight waited for an impact that could not
/// come.
#[test]
fn a_boss_script_naming_a_prop_the_room_lacks_fails_the_room_while_it_is_whole() {
    let prepare_arena = |props: &[&str]| {
        prepare(
            &cut_rope_room(props),
            &crate::features::RoomContentStagingRegistry::default(),
            &engine_construction_registry(),
        )
    };
    // Premise: the same arena with its anvil plans, so the refusal below is
    // about the anvil and nothing else.
    prepare_arena(&["cut_rope_rope", "cut_rope_anvil"]).expect("the whole arena plans");
    let error = prepare_arena(&["cut_rope_rope"]).expect_err("an arena without its anvil cannot plan");
    assert!(
        matches!(
            error,
            RoomFeatureConstructionError::ActorConstruction(
                ActorConstructionError::EncounterScriptCannotRun {
                    error: ambition_encounter::EncounterScriptError::NoSuchProp { ref kind, .. },
                    ..
                }
            ) if kind == "cut_rope_anvil"
        ),
        "{error:?}"
    );
}

/// A staged boss is checked the same way. One built without an encounter runs
/// no script, so its room plans whatever props it authors.
#[test]
fn a_staged_boss_script_is_checked_unless_the_boss_has_no_encounter() {
    let prepare_staged = |no_encounter: bool| {
        let mut staging = crate::features::RoomContentStagingRegistry::default();
        staging
            .register("hall", "test_provider", "behemoth", "behemoth.v1", move |_room| {
                vec![SpawnActorRequest {
                    id: "behemoth".to_string(),
                    name: "smirking_behemoth_boss".to_string(),
                    pos: ae::Vec2::new(100.0, 20.0),
                    half_size: ae::Vec2::splat(60.0),
                    faction: ambition_characters::actor::ActorFaction::Boss,
                    grudge_against: None,
                    kind: SpawnActorKind::Boss {
                        brain: ambition_entity_catalog::placements::BossBrain::PhaseScript {
                            script_id: "smirking_behemoth_boss".into(),
                        },
                        overrides: ambition_boss_encounter::BossOverrides {
                            no_encounter,
                            ..Default::default()
                        },
                    },
                }]
            })
            .expect("stager registers");
        prepare(&empty_room("hall"), &staging, &engine_construction_registry())
    };
    let error = prepare_staged(false).expect_err("a staged behemoth needs its anvil");
    assert!(
        matches!(
            error,
            RoomFeatureConstructionError::ActorConstruction(
                ActorConstructionError::EncounterScriptCannotRun {
                    error: ambition_encounter::EncounterScriptError::NoSuchProp { ref kind, .. },
                    ..
                }
            ) if kind == "cut_rope_anvil"
        ),
        "{error:?}"
    );
    prepare_staged(true).expect("a behemoth with no encounter runs no script");
}

/// Two links claiming one mount are refused at preparation — the domain
/// preflight sees the planned relations, not a frame-later race.
#[test]
fn two_riders_claiming_one_authored_mount_are_refused() {
    let mut room = mounted_pair_room();
    room.enemy_spawns
        .push(ambition_platformer2d_world::rooms::Authored::new(
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
    room.enemy_spawns
        .push(ambition_platformer2d_world::rooms::Authored::new(
            "walker",
            "Ordinary Walker",
            ae::Aabb::new(ae::Vec2::new(300.0, 40.0), ae::Vec2::new(22.0, 39.0)),
            ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
                "combatant",
            ),
        ));
    room.boss_spawns
        .push(ambition_platformer2d_world::rooms::Authored::new(
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

/// A room with one person, `npc_1`, who names `character` (or nobody).
fn person_room(character: Option<&str>) -> ambition_platformer2d_world::rooms::RoomSpec {
    use ambition_entity_catalog::placements::{InteractableSpec, InteractionKindSpec, PlacementSchema};
    let mut room = empty_room("parlour");
    room.placements.push(
        ambition_platformer2d_world::placements::PlacementRecord::new(
            "npc_1",
            PlacementSchema::Interactable(InteractableSpec::new(
                "Talk",
                InteractionKindSpec::Npc {
                    character_id: character.map(str::to_owned),
                    dialogue_id: None,
                    patrol_radius: 0.0,
                    patrol_path_id: None,
                    brain_override: None,
                },
            )),
            ae::Aabb::new(ae::Vec2::new(96.0, 32.0), ae::Vec2::splat(16.0)),
        ),
    );
    room
}

/// ⛔ A PERSON WHO NAMES A CHARACTER THAT NOBODY REGISTERED IS REFUSED WHEN THE
/// ROOM IS PLANNED.
///
/// The plan prepared, and the recipe panicked when the room was built
/// (`report_unprepared_character`): a world reload of an `NpcSpawn` with a
/// wrong `character_id` stopped the game. An enemy with the same fault was
/// already refused here.
///
/// The controls are the three cases the NPC road CAN build, which stay
/// plans: a character of the cast, a person who names nobody, a character
/// that only the catalog has (its body is the catalog row's), and a
/// composition that published no cast (a warning, not a refusal).
#[test]
fn a_person_who_names_an_unregistered_character_is_refused_when_the_room_is_planned() {
    let catalog = crate::character_roster::catalog();
    let nobody = ambition_characters::actor::character_catalog::CharacterCatalog::empty();
    let prepare = |room: &ambition_platformer2d_world::rooms::RoomSpec,
                   catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
                   cast: Option<&'static ambition_characters::prepared::PreparedCharacterRegistry>| {
        let registry = engine_construction_registry();
        let sheets = Default::default();
        let context = ActorConstructionContext::new(
            &registry,
            catalog,
            &sheets,
            ContentBinding::content_unstated(ae::ContentEpoch(4)),
        );
        RoomFeatureConstructionPlan::prepare(
            room,
            &placement_registry(),
            &crate::features::RoomContentStagingRegistry::default(),
            &ambition_boss_encounter::test_boss_catalog(),
            match cast {
                Some(cast) => context.with_prepared(cast),
                None => context,
            },
        )
        .map(|_| ())
    };

    // ⛔ THE PREMISES of the controls: the character the catalog arm names is
    // in the catalog and is NOT in the cast, and the cast is not empty.
    const CATALOG_ONLY: &str = "npc_kernel_guide";
    assert!(
        catalog.display_name(CATALOG_ONLY).is_some()
            && fixture_cast().get(CATALOG_ONLY).is_none()
            && !fixture_cast().is_empty(),
        "the fixture does not have a character that only the catalog has"
    );

    let refused = prepare(&person_room(Some("a_stranger")), &nobody, Some(fixture_cast()))
        .expect_err("a person who names a character in no cast and no catalog was planned");
    assert!(
        matches!(
            &refused,
            RoomFeatureConstructionError::ActorConstruction(
                ActorConstructionError::BodyCharacterNotRegistered { sim_id, character },
            ) if *sim_id == SimId::placement("npc_1") && character == "a_stranger"
        ),
        "the refusal does not name the person and the character: {refused}"
    );
    // The catalog that HAS characters does not have this one either.
    assert!(
        prepare(&person_room(Some("a_stranger")), &catalog, Some(fixture_cast())).is_err(),
        "a catalog that does not have the character let the person through"
    );

    for (what, room, catalog, cast) in [
        ("a character of the cast", person_room(Some("fixture_walker")), &nobody, Some(fixture_cast())),
        ("a person who names nobody", person_room(None), &nobody, Some(fixture_cast())),
        ("a character that only the catalog has", person_room(Some(CATALOG_ONLY)), &catalog, Some(fixture_cast())),
        ("a composition with no cast", person_room(Some("a_stranger")), &nobody, None),
    ] {
        if let Err(error) = prepare(&room, catalog, cast) {
            panic!("control, {what}: the NPC road can build this person and the plan was refused: {error}");
        }
    }
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
    let walker: ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::EnemySpawnSpec,
    > = ambition_platformer2d_world::rooms::Authored::new(
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
        let enemy: ambition_platformer2d_world::rooms::Authored<
            ambition_platformer2d_world::rooms::EnemySpawnSpec,
        > = ambition_platformer2d_world::rooms::Authored::new(
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
    let mut cast = ambition_characters::prepared::PreparedCharacterRegistry::default();
    cast.insert_prepared(
        crate::character_runtime::prepare_and_finalize_for_test(
            ambition_characters::actor::definition::CharacterDefinition::new(
                "npc_mute", "Mute", "test",
            ),
            &ambition_characters::prepared::CharacterBindings::default(),
        )
        .prepared,
    );
    let incomplete = RoomFeatureConstructionPlan::prepare(
        &room_naming("npc_mute"),
        &Default::default(),
        &crate::features::RoomContentStagingRegistry::default(),
        &ambition_boss_encounter::test_boss_catalog(),
        ActorConstructionContext::new(
            &engine_construction_registry(),
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &Default::default(),
            ContentBinding::content_unstated(ae::ContentEpoch(4)),
        )
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
        ambition_characters::actor::definition::CharacterDefinition::new(
            "npc_test_shark",
            "Shark",
            "test",
        )
        .with_mount(ambition_characters::actor::CharacterMount {
            class: Some("shark".to_string()),
            ..Default::default()
        }),
        &ambition_characters::prepared::CharacterBindings::default(),
    );
    let mut cast = ambition_characters::prepared::PreparedCharacterRegistry::default();
    cast.insert_prepared(finalized.prepared);

    let bosses = ambition_boss_encounter::BossCatalog::default();
    let params = ActorConstructionParams::StagedActor(request.clone());

    let planned = ambition_platformer2d_construction::mount_capabilities_of(&params, &bosses, Some(&cast));
    assert_eq!(
        planned.mount_class.as_deref(),
        Some("shark"),
        "the character authors a mount class and the plan must carry it"
    );

    // the control, and it is what makes the assertion above mean anything:
    // with no cast the same request falls to the archetype, which states no
    // mount. If this ALSO said `shark`, the roster would be answering and the
    // character would be decorative.
    let without_cast = ambition_platformer2d_construction::mount_capabilities_of(&params, &bosses, None);
    assert_eq!(
        without_cast.mount_class, None,
        "the fixture roster must state no mount for this brain, or this test is \
         not measuring the character"
    );
}

/// A boss in a PRACTICE room (`RoomMetadata::practice`, the Hall of Bosses' own
/// arenas) is planned as a practice copy, and one anywhere else is not: the
/// row carries what the room says, and the boss's `BossConfig::practice` is
/// set from it at spawn.
#[test]
fn a_practice_room_plans_its_bosses_as_practice_copies() {
    let practice_of = |practice: bool| {
        let mut room = giant_room();
        room.metadata.practice = practice;
        room.boss_spawns
            .push(ambition_platformer2d_world::rooms::Authored::new(
                "hall_copy",
                "mockingbird",
                ae::Aabb::new(ae::Vec2::new(100.0, 20.0), ae::Vec2::splat(30.0)),
                ambition_entity_catalog::placements::BossBrain::PhaseScript {
                    script_id: "mockingbird".into(),
                },
            ));
        ambition_platformer2d_construction::authored_actor_requests(&room, &[], None)
            .into_iter()
            .find_map(|request| match request.parameters {
                ActorConstructionParams::AuthoredBoss { authored, practice, .. } if authored.id == "hall_copy" => {
                    Some(practice)
                }
                _ => None,
            })
            .expect("the boss is a plan row")
    };
    assert!(practice_of(true), "a practice room's boss is a practice copy");
    assert!(!practice_of(false), "a boss anywhere else is not");
}

/// A placement is planned where its body starts (`SpawnGrounding`): a walker
/// on the ground under it, a flyer and an `Exact` placement where they are
/// placed, a rider where it is placed (it starts on its mount).
///
/// Jon, 2026-10-08: each actor was built above its floor and fell to it, so a
/// room started with a burst of landing sounds.
#[test]
fn a_placement_is_planned_where_its_body_starts() {
    use ambition_platformer2d_world::rooms::SpawnGrounding;

    let mut flyer = ambition_characters::prepared::PreparedCharacterRegistry::default();
    flyer.insert_prepared(
        crate::character_runtime::prepare_and_finalize_for_test(
            ambition_characters::actor::definition::CharacterDefinition::new("fixture_flyer", "Flyer", "test")
                .with_locomotion(ambition_characters::actor::CharacterLocomotion {
                    baseline_free_flight: Some(true),
                    ..Default::default()
                }),
            &ambition_characters::prepared::CharacterBindings::default(),
        )
        .prepared,
    );
    // The floor's top is at 500, and each placement's feet are at 460.
    let placed_feet = 460.0;
    let floor_top = 500.0;
    let planned_feet = |character: &str, grounding: SpawnGrounding, rides: bool, cast| {
        let mut room = empty_room("room");
        room.world.blocks.push(ae::Block::solid(
            "floor",
            ae::Vec2::new(0.0, floor_top),
            ae::Vec2::new(1000.0, 100.0),
        ));
        let mut payload = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Passive,
            character,
        );
        payload.grounding = grounding;
        room.enemy_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
            "placed",
            "Placed",
            ae::aabb_from_min_size(ae::Vec2::new(100.0, placed_feet - 28.0), ae::Vec2::splat(28.0)),
            payload,
        ));
        if rides {
            room.mount_links.push(("placed".to_string(), "its_mount".to_string()));
        }
        let requests = ambition_platformer2d_construction::authored_actor_requests(&room, &[], cast);
        let [request] = requests.as_slice() else {
            panic!("one placement plans one row, not {}", requests.len());
        };
        let ActorConstructionParams::AuthoredEnemy { authored, .. } = &request.parameters else {
            panic!("the placement plans an ordinary enemy row");
        };
        use ae::AabbExt;
        authored.aabb.bottom()
    };
    let walkers = Some(fixture_cast());

    assert_eq!(planned_feet("fixture_walker", SpawnGrounding::Auto, false, walkers), floor_top);
    assert_eq!(planned_feet("fixture_walker", SpawnGrounding::Ground, false, walkers), floor_top);
    assert_eq!(planned_feet("fixture_walker", SpawnGrounding::Exact, false, walkers), placed_feet);
    assert_eq!(planned_feet("fixture_walker", SpawnGrounding::Auto, true, walkers), placed_feet, "a rider");

    assert_eq!(planned_feet("fixture_flyer", SpawnGrounding::Auto, false, Some(&flyer)), placed_feet);
    assert_eq!(planned_feet("fixture_flyer", SpawnGrounding::Ground, false, Some(&flyer)), floor_top);
}
