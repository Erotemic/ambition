//! The pure planner's contract: canonical ordering, identity and reference
//! validation before any mutation, one constructor shared by ordinary
//! construction and reconstruction, and a byte-stable dump.
//!
//! These tests use a toy domain rather than a real actor family on purpose —
//! the properties proven here are the planner's, and a domain that could fail
//! for its own reasons would make a failure ambiguous. The three real families
//! are proven against the real world in `ambition_actors`.

use std::collections::BTreeSet;

use bevy::prelude::{Component, Entity, World};

use super::*;
use crate::sim_id::SimId;

#[derive(Component, Clone, Debug, PartialEq, Eq)]
struct Built(String);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct Grudge(Entity);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Params {
    label: String,
}

/// Counts how many times each recipe ran, so a test can prove that
/// reconstruction went through the same function ordinary construction uses.
#[derive(Default)]
struct Services {
    ordinary_runs: std::cell::Cell<usize>,
}

struct Toy;

impl ConstructionDomain for Toy {
    type Parameters = Params;
    type Services = Services;

    fn canonical_summary(parameters: &Self::Parameters) -> String {
        parameters.label.clone()
    }
}

fn build(planned: &PlannedEntity<Toy>, ctx: &mut ConstructionExecCtx<'_, '_, '_, Toy>) -> Entity {
    ctx.services
        .ordinary_runs
        .set(ctx.services.ordinary_runs.get() + 1);
    ctx.commands
        .spawn(Built(planned.parameters().label.clone()))
        .id()
}

/// The toy domain has one parameter shape, so its recipes accept everything.
/// Rejection is exercised by `a_recipe_that_rejects_the_parameters_fails_the_plan`,
/// which registers a picky recipe of its own.
fn accepts_any(_: &Params) -> bool {
    true
}

fn wire_grudge(from: Entity, to: Entity, ctx: &mut ConstructionExecCtx<'_, '_, '_, Toy>) {
    ctx.commands.entity(from).insert(Grudge(to));
}

fn recipe() -> RecipeId {
    RecipeId::new("toy.build")
}

fn grudge() -> RelationKind {
    RelationKind::new("toy.grudge")
}

fn registry() -> ConstructionRegistry<Toy> {
    let mut registry = ConstructionRegistry::<Toy>::default();
    registry
        .try_register_recipe(recipe(), "toy", "tests", "v1", accepts_any, build)
        .expect("first registration succeeds");
    registry
        .try_register_relation(grudge(), "toy", wire_grudge)
        .expect("first registration succeeds");
    registry
}

fn scope() -> ConstructionScope {
    ConstructionScope {
        content_epoch: ambition_engine_core::ContentEpoch(7),
        room: Some("room_a".into()),
    }
}

fn request(id: &str) -> ConstructionRequest<Toy> {
    ConstructionRequest {
        sim_id: SimId::placement(id),
        recipe: recipe(),
        origin: SpawnOrigin::Authored {
            source: "room_a".into(),
            instance: id.into(),
        },
        parameters: Params { label: id.into() },
        relations: Vec::new(),
    }
}

fn nothing_live() -> BTreeSet<SimId> {
    BTreeSet::new()
}

// ── Canonical ordering ───────────────────────────────────────────────────────

/// Exit criterion: *reordered plan input does not change deterministic output*.
#[test]
fn request_order_does_not_change_the_plan() {
    let registry = registry();
    let forward = ConstructionPlan::prepare(
        scope(),
        vec![request("a"), request("b"), request("c")],
        &nothing_live(),
        &registry,
    )
    .expect("valid plan");
    let reversed = ConstructionPlan::prepare(
        scope(),
        vec![request("c"), request("b"), request("a")],
        &nothing_live(),
        &registry,
    )
    .expect("valid plan");

    assert_eq!(
        forward.deterministic_dump(),
        reversed.deterministic_dump(),
        "two orderings of the same requests must produce identical plans"
    );
    // And the dump is actually ordered, not merely equal to itself.
    let ids: Vec<&str> = forward
        .entities()
        .iter()
        .map(|entity| entity.sim_id().as_str())
        .collect();
    assert_eq!(ids, ["placement:a", "placement:b", "placement:c"]);
}

/// Relations are ordered too — a mutual pair declared in either direction
/// yields the same plan.
#[test]
fn relation_order_does_not_change_the_plan() {
    let registry = registry();
    let pair = |first: &str, second: &str| {
        let mut a = request(first);
        a.relations.push(RelationRequest {
            kind: grudge(),
            to: SimId::placement(second),
        });
        let mut b = request(second);
        b.relations.push(RelationRequest {
            kind: grudge(),
            to: SimId::placement(first),
        });
        vec![a, b]
    };
    let forward =
        ConstructionPlan::prepare(scope(), pair("a", "b"), &nothing_live(), &registry).unwrap();
    let reversed =
        ConstructionPlan::prepare(scope(), pair("b", "a"), &nothing_live(), &registry).unwrap();
    assert_eq!(forward.deterministic_dump(), reversed.deterministic_dump());
}

/// The dump is the inspection surface, so its exact shape is pinned. A change
/// here is a compatibility decision, not an incidental edit.
#[test]
fn the_plan_dump_has_a_stable_shape() {
    let registry = registry();
    let mut a = request("a");
    a.relations.push(RelationRequest {
        kind: grudge(),
        to: SimId::placement("b"),
    });
    let mut b = request("b");
    b.origin = SpawnOrigin::Dynamic {
        parent: SimId::placement("a"),
        sequence: 4,
    };
    let plan = ConstructionPlan::prepare(scope(), vec![a, b], &nothing_live(), &registry).unwrap();

    assert_eq!(
        plan.deterministic_dump(),
        "construction-plan-v2\n\
         epoch:7\n\
         room\troom_a\n\
         entity\tplacement:a\ttoy.build\tauthored\troom_a\ta\ta\n\
         entity\tplacement:b\ttoy.build\tdynamic\tplacement:a\t4\tb\n\
         relation\tplacement:a\ttoy.grudge\tplacement:b\n"
    );
}

// ── Validation before mutation ───────────────────────────────────────────────

/// Exit criterion: *duplicate identities fail before mutation*.
#[test]
fn a_duplicate_identity_is_rejected() {
    let registry = registry();
    let error = ConstructionPlan::prepare(
        scope(),
        vec![request("a"), request("a")],
        &nothing_live(),
        &registry,
    )
    .expect_err("a duplicate identity must not plan");
    assert_eq!(
        error,
        ConstructionError::DuplicateIdentity {
            sim_id: SimId::placement("a")
        }
    );
}

/// A plan may not mint an identity a live entity already holds — the collision
/// that would make two rows in a snapshot claim the same body.
#[test]
fn an_identity_a_live_entity_already_holds_is_rejected() {
    let registry = registry();
    let live: BTreeSet<SimId> = [SimId::placement("a")].into_iter().collect();
    let error = ConstructionPlan::prepare(scope(), vec![request("a")], &live, &registry)
        .expect_err("a live collision must not plan");
    assert_eq!(
        error,
        ConstructionError::IdentityAlreadyLive {
            sim_id: SimId::placement("a")
        }
    );
}

/// Exit criterion: *unresolved relations fail before mutation*. Today's grudge
/// wiring silently skips an unresolvable foe id; planning refuses it instead.
#[test]
fn an_unresolved_relation_is_rejected() {
    let registry = registry();
    let mut a = request("a");
    a.relations.push(RelationRequest {
        kind: grudge(),
        to: SimId::placement("ghost"),
    });
    let error = ConstructionPlan::prepare(scope(), vec![a], &nothing_live(), &registry)
        .expect_err("an unresolvable relation target must not plan");
    assert_eq!(
        error,
        ConstructionError::UnresolvedRelation {
            from: SimId::placement("a"),
            kind: grudge(),
            to: SimId::placement("ghost"),
        }
    );
}

/// A relation onto an entity that is merely LIVE — not a row in this plan — is
/// rejected, not accepted-and-skipped.
///
/// Commit wires relations between the identities it just constructed, so it
/// holds no entity for an outsider. Accepting the plan and then quietly not
/// wiring it would be a brand-new silent drop, in the machinery built to delete
/// silent drops. Relating to a live entity is a real need; it belongs with the
/// commit boundary that Phase 4 gives a live identity index.
#[test]
fn a_relation_onto_a_merely_live_entity_is_rejected() {
    let registry = registry();
    let live: BTreeSet<SimId> = [SimId::placement("veteran")].into_iter().collect();
    let mut a = request("a");
    a.relations.push(RelationRequest {
        kind: grudge(),
        to: SimId::placement("veteran"),
    });
    let error = ConstructionPlan::prepare(scope(), vec![a], &live, &registry)
        .expect_err("a relation onto a non-planned identity must not plan");
    assert_eq!(
        error,
        ConstructionError::UnresolvedRelation {
            from: SimId::placement("a"),
            kind: grudge(),
            to: SimId::placement("veteran"),
        }
    );
}

/// A PARENT, unlike a relation target, may be live — a summoner outlives the
/// summon it plans. The two rules are deliberately different, so both are
/// pinned.
#[test]
fn a_parent_that_is_merely_live_is_accepted() {
    let registry = registry();
    let live: BTreeSet<SimId> = [SimId::placement("summoner")].into_iter().collect();
    let mut a = request("a");
    a.origin = SpawnOrigin::Dynamic {
        parent: SimId::placement("summoner"),
        sequence: 0,
    };
    ConstructionPlan::prepare(scope(), vec![a], &live, &registry)
        .expect("a parent may be an already-live identity");
}

#[test]
fn an_unresolved_parent_is_rejected() {
    let registry = registry();
    let mut a = request("a");
    a.origin = SpawnOrigin::Dynamic {
        parent: SimId::placement("ghost"),
        sequence: 0,
    };
    let error = ConstructionPlan::prepare(scope(), vec![a], &nothing_live(), &registry)
        .expect_err("an unresolvable parent must not plan");
    assert_eq!(
        error,
        ConstructionError::UnresolvedParent {
            sim_id: SimId::placement("a"),
            parent: SimId::placement("ghost"),
        }
    );
}

#[test]
fn an_unregistered_recipe_is_rejected() {
    let registry = registry();
    let mut a = request("a");
    a.recipe = RecipeId::new("toy.missing");
    let error = ConstructionPlan::prepare(scope(), vec![a], &nothing_live(), &registry)
        .expect_err("an unknown recipe must not plan");
    assert_eq!(
        error,
        ConstructionError::UnknownRecipe {
            sim_id: SimId::placement("a"),
            recipe: RecipeId::new("toy.missing"),
        }
    );
}

#[test]
fn an_unregistered_relation_kind_is_rejected() {
    let registry = registry();
    let mut a = request("a");
    a.relations.push(RelationRequest {
        kind: RelationKind::new("toy.unknown"),
        to: SimId::placement("a"),
    });
    let error = ConstructionPlan::prepare(scope(), vec![a], &nothing_live(), &registry)
        .expect_err("an unknown relation kind must not plan");
    assert_eq!(
        error,
        ConstructionError::UnknownRelationKind {
            from: SimId::placement("a"),
            kind: RelationKind::new("toy.unknown"),
        }
    );
}

// ── Commit ───────────────────────────────────────────────────────────────────

/// Commit a plan into a bare `World` and apply the queued commands, the same
/// exclusive-world shape `RoomConstructionPlan::apply_to_world` uses.
fn commit(plan: &ConstructionPlan<Toy>, services: &Services) -> (World, ConstructionReceipt) {
    let mut world = World::new();
    let receipt = commit_into(&mut world, plan, services);
    (world, receipt)
}

fn commit_into(
    world: &mut World,
    plan: &ConstructionPlan<Toy>,
    services: &Services,
) -> ConstructionReceipt {
    let scope = plan.scope().clone();
    let receipt = {
        let mut commands = world.commands();
        let mut ctx = ConstructionExecCtx::<Toy> {
            commands: &mut commands,
            scope: &scope,
            session: crate::lifecycle::SessionSpawnScope::UNSCOPED,
            services,
        };
        plan.commit(&mut ctx)
    };
    world.flush();
    receipt
}

fn construct_one_into(
    world: &mut World,
    plan: &ConstructionPlan<Toy>,
    services: &Services,
    sim_id: &SimId,
) -> Result<Entity, ConstructionError> {
    let scope = plan.scope().clone();
    let result = {
        let mut commands = world.commands();
        let mut ctx = ConstructionExecCtx::<Toy> {
            commands: &mut commands,
            scope: &scope,
            session: crate::lifecycle::SessionSpawnScope::UNSCOPED,
            services,
        };
        plan.construct_one(sim_id, &mut ctx)
    };
    world.flush();
    result
}

/// Exit criterion: *planned and committed `SimId` rosters match exactly*.
#[test]
fn the_committed_roster_is_exactly_the_planned_roster() {
    let registry = registry();
    let plan = ConstructionPlan::prepare(
        scope(),
        vec![request("a"), request("b"), request("c")],
        &nothing_live(),
        &registry,
    )
    .unwrap();
    let services = Services::default();
    let (mut world, receipt) = commit(&plan, &services);

    assert_eq!(receipt.committed_ids(), plan.planned_ids());
    assert_eq!(receipt.len(), 3);

    // And the world really holds them — a receipt that agreed with the plan
    // while nothing spawned would prove nothing.
    let mut built = world.query::<&Built>();
    assert_eq!(
        built.iter(&world).count(),
        3,
        "every planned row produced a live entity"
    );
}

/// The executor stamps identity and provenance, so no recipe can forget them.
#[test]
fn every_committed_entity_carries_its_identity_and_provenance() {
    let registry = registry();
    let plan =
        ConstructionPlan::prepare(scope(), vec![request("a")], &nothing_live(), &registry).unwrap();
    let services = Services::default();
    let (world, receipt) = commit(&plan, &services);

    let entity = receipt.entity(&SimId::placement("a")).expect("committed");
    assert_eq!(world.get::<SimId>(entity), Some(&SimId::placement("a")));
    assert_eq!(
        world.get::<SpawnOrigin>(entity),
        Some(&SpawnOrigin::Authored {
            source: "room_a".into(),
            instance: "a".into(),
        })
    );
}

/// Relations wire after every row exists, which is what lets a mutual pair
/// resolve without either half needing to be constructed first.
#[test]
fn a_mutual_relation_wires_both_directions() {
    let registry = registry();
    let mut a = request("a");
    a.relations.push(RelationRequest {
        kind: grudge(),
        to: SimId::placement("b"),
    });
    let mut b = request("b");
    b.relations.push(RelationRequest {
        kind: grudge(),
        to: SimId::placement("a"),
    });
    let plan = ConstructionPlan::prepare(scope(), vec![a, b], &nothing_live(), &registry).unwrap();
    let services = Services::default();
    let (world, receipt) = commit(&plan, &services);

    let (ea, eb) = (
        receipt.entity(&SimId::placement("a")).unwrap(),
        receipt.entity(&SimId::placement("b")).unwrap(),
    );
    assert_eq!(world.get::<Grudge>(ea), Some(&Grudge(eb)));
    assert_eq!(world.get::<Grudge>(eb), Some(&Grudge(ea)));
    assert_eq!(receipt.relations_wired().len(), 2);
}

/// Exit criterion: *the slice has no separate normal-spawn and reconstruction
/// constructor*. Rebuilding one row runs the same recipe, and the service
/// counter proves it rather than the absence of a second symbol.
#[test]
fn reconstructing_one_entity_runs_the_ordinary_recipe() {
    let registry = registry();
    let plan = ConstructionPlan::prepare(
        scope(),
        vec![request("a"), request("b")],
        &nothing_live(),
        &registry,
    )
    .unwrap();

    let services = Services::default();
    let (mut world, _) = commit(&plan, &services);
    assert_eq!(services.ordinary_runs.get(), 2, "two rows constructed");

    // Now rebuild exactly one, the way a same-room restore does.
    let rebuilt = construct_one_into(&mut world, &plan, &services, &SimId::placement("a"));

    assert!(rebuilt.is_ok());
    assert_eq!(
        services.ordinary_runs.get(),
        3,
        "reconstruction went through the same recipe, not a parallel constructor"
    );
}

#[test]
fn reconstructing_an_unplanned_identity_is_an_error_not_a_silent_skip() {
    let registry = registry();
    let plan =
        ConstructionPlan::prepare(scope(), vec![request("a")], &nothing_live(), &registry).unwrap();
    let services = Services::default();
    let mut world = World::new();
    let result = construct_one_into(&mut world, &plan, &services, &SimId::placement("nope"));
    assert_eq!(
        result,
        Err(ConstructionError::NotInPlan {
            sim_id: SimId::placement("nope")
        })
    );
}

// ── Registration lifecycle ───────────────────────────────────────────────────

#[test]
fn identical_re_registration_is_idempotent_and_a_conflict_is_rejected() {
    let mut registry = registry();
    registry
        .try_register_recipe(recipe(), "toy", "tests", "v1", accepts_any, build)
        .expect("byte-identical re-registration is idempotent");

    fn other(_: &PlannedEntity<Toy>, _: &mut ConstructionExecCtx<'_, '_, '_, Toy>) -> Entity {
        unreachable!("never executed")
    }
    let before = registry.deterministic_dump();
    let error = registry
        .try_register_recipe(recipe(), "other", "tests", "v1", accepts_any, other)
        .expect_err("a second owner for one recipe must be rejected");
    assert!(matches!(
        error,
        ConstructionRegistrationError::ConflictingRecipe { .. }
    ));
    assert_eq!(
        registry.deterministic_dump(),
        before,
        "a rejected registration leaves the registry untouched"
    );
}

#[test]
fn empty_identity_fields_are_rejected() {
    let mut registry = ConstructionRegistry::<Toy>::default();
    assert_eq!(
        registry.try_register_recipe(RecipeId::new(" "), "toy", "tests", "v1", accepts_any, build),
        Err(ConstructionRegistrationError::EmptyIdentity { field: "id" })
    );
    assert_eq!(
        registry.try_register_recipe(recipe(), "", "tests", "v1", accepts_any, build),
        Err(ConstructionRegistrationError::EmptyIdentity { field: "owner" })
    );
}

#[test]
fn the_registry_dump_does_not_depend_on_registration_order() {
    fn second(
        planned: &PlannedEntity<Toy>,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, Toy>,
    ) -> Entity {
        build(planned, ctx)
    }
    let mut forward = ConstructionRegistry::<Toy>::default();
    forward
        .try_register_recipe(
            RecipeId::new("toy.a"),
            "toy",
            "tests",
            "v1",
            accepts_any,
            build,
        )
        .unwrap();
    forward
        .try_register_recipe(
            RecipeId::new("toy.b"),
            "toy",
            "tests",
            "v1",
            accepts_any,
            second,
        )
        .unwrap();

    let mut reversed = ConstructionRegistry::<Toy>::default();
    reversed
        .try_register_recipe(
            RecipeId::new("toy.b"),
            "toy",
            "tests",
            "v1",
            accepts_any,
            second,
        )
        .unwrap();
    reversed
        .try_register_recipe(
            RecipeId::new("toy.a"),
            "toy",
            "tests",
            "v1",
            accepts_any,
            build,
        )
        .unwrap();

    assert_eq!(forward.deterministic_dump(), reversed.deterministic_dump());
}

// ── The recipe/parameter pairing ─────────────────────────────────────────────

fn accepts_nothing(_: &Params) -> bool {
    false
}

fn picky() -> RecipeId {
    RecipeId::new("toy.picky")
}

fn registry_with_picky_recipe() -> ConstructionRegistry<Toy> {
    let mut registry = registry();
    registry
        .try_register_recipe(picky(), "toy", "tests", "v1", accepts_nothing, build)
        .expect("first registration succeeds");
    registry
}

/// A request names its recipe and carries its parameters as two independent
/// public fields, so nothing but a check stops a caller pairing them wrongly.
/// Without the check the mismatch reaches the recipe, which can only panic —
/// mid-commit, after earlier rows have already mutated the world.
#[test]
fn a_recipe_that_cannot_build_from_the_parameters_fails_the_plan() {
    let registry = registry_with_picky_recipe();
    let mut a = request("a");
    a.recipe = picky();
    let error = ConstructionPlan::prepare(scope(), vec![a], &nothing_live(), &registry)
        .expect_err("a recipe that cannot build these parameters must not plan");
    assert_eq!(
        error,
        ConstructionError::ParametersRejected {
            sim_id: SimId::placement("a"),
            recipe: picky(),
        }
    );
}

/// And the rejection happens before ANY row runs — including rows that sort
/// ahead of the bad one and would otherwise already be in the world.
#[test]
fn a_mispaired_row_stops_the_rows_that_sort_before_it() {
    let registry = registry_with_picky_recipe();
    let good = request("a");
    let mut bad = request("b");
    bad.recipe = picky();
    assert!(
        ConstructionPlan::prepare(scope(), vec![good, bad], &nothing_live(), &registry).is_err(),
        "one unbuildable row rejects the whole plan"
    );
    // Nothing to assert about the world: `prepare` never receives one. That is
    // the property — the refusal happens where mutation is not yet possible.
}

// ── Partial commits ──────────────────────────────────────────────────────────

fn feuding_pair(registry: &ConstructionRegistry<Toy>) -> ConstructionPlan<Toy> {
    let mut a = request("a");
    a.relations.push(RelationRequest {
        kind: grudge(),
        to: SimId::placement("b"),
    });
    let b = request("b");
    ConstructionPlan::prepare(scope(), vec![a, b], &nothing_live(), registry).unwrap()
}

/// Rebuilding a relation-bearing row on its own would put the body back without
/// its wiring — an entity count that looks right and a behaviour that is not.
/// Refused, loudly, rather than best-effort.
#[test]
fn a_row_whose_relation_leaves_the_subset_cannot_be_rebuilt_alone() {
    let registry = registry();
    let plan = feuding_pair(&registry);
    let services = Services::default();
    let mut world = World::new();

    let error = construct_one_into(&mut world, &plan, &services, &SimId::placement("a"))
        .expect_err("rebuilding a grudge-bearing row alone must be refused");
    assert_eq!(
        error,
        ConstructionError::RelationOutsideSubset {
            from: SimId::placement("a"),
            kind: grudge(),
            to: SimId::placement("b"),
        }
    );
    assert_eq!(
        services.ordinary_runs.get(),
        0,
        "the refusal happened before the recipe ran"
    );
    let mut built = world.query::<&Built>();
    assert_eq!(
        built.iter(&world).count(),
        0,
        "a refused rebuild leaves the world untouched"
    );
}

/// The rule is directional. `b` is grudged BY `a`; that relation belongs to
/// `a`'s row, which is not being rebuilt and still holds it. Rebuilding `b`
/// alone is therefore ordinary, not a partial relation.
#[test]
fn a_relation_pointing_into_the_subset_does_not_block_it() {
    let registry = registry();
    let plan = feuding_pair(&registry);
    let services = Services::default();
    let mut world = World::new();

    construct_one_into(&mut world, &plan, &services, &SimId::placement("b"))
        .expect("a row that declares no relation of its own rebuilds alone");
    assert_eq!(services.ordinary_runs.get(), 1);
}

/// A subset that encloses both ends wires the relation between them, so the
/// pair comes back whole.
#[test]
fn a_subset_that_encloses_a_relation_wires_it() {
    let registry = registry();
    let plan = feuding_pair(&registry);
    let services = Services::default();
    let mut world = World::new();

    let receipt = {
        let mut commands = world.commands();
        let scope = scope();
        let mut ctx = ConstructionExecCtx {
            commands: &mut commands,
            scope: &scope,
            session: crate::lifecycle::SessionSpawnScope::UNSCOPED,
            services: &services,
        };
        plan.commit_subset(&plan.planned_ids(), &mut ctx)
            .expect("the whole roster encloses every relation")
    };
    world.flush();

    let (ea, eb) = (
        receipt.entity(&SimId::placement("a")).unwrap(),
        receipt.entity(&SimId::placement("b")).unwrap(),
    );
    assert_eq!(world.get::<Grudge>(ea), Some(&Grudge(eb)));
    assert_eq!(receipt.relations_wired().len(), 1);
}

// ── The executor's own guard ─────────────────────────────────────────────────

/// A recipe returns an arbitrary `Entity` and the executor has no way to know it
/// was freshly created. A defective one that hands back a body which is already
/// identified would have its identity silently overwritten — two `SimId`s on one
/// entity, which is a desync — while the receipt still reported clean parity.
#[test]
#[should_panic(expected = "already holds identity")]
fn a_recipe_that_returns_an_already_identified_entity_is_caught() {
    fn steal(
        _planned: &PlannedEntity<Toy>,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, Toy>,
    ) -> Entity {
        // Whatever it was asked to build, it hands back the same squatted body.
        ctx.services.ordinary_runs.set(1);
        Entity::from_raw_u32(0).expect("entity 0 is representable")
    }

    let mut registry = ConstructionRegistry::<Toy>::default();
    registry
        .try_register_recipe(recipe(), "toy", "tests", "v1", accepts_any, steal)
        .unwrap();
    let plan = ConstructionPlan::prepare(
        scope(),
        vec![request("a"), request("b")],
        &nothing_live(),
        &registry,
    )
    .unwrap();

    let services = Services::default();
    let mut world = World::new();
    // Give entity 0 an identity of its own, the way any live body has one.
    let squatted = world.spawn(SimId::placement("already-here")).id();
    assert_eq!(squatted, Entity::from_raw_u32(0).unwrap());
    let _ = commit_into(&mut world, &plan, &services);
}
