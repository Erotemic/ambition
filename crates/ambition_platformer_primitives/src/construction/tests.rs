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

    fn dispatch(_: &Self::Parameters) -> RecipeDispatch<Self> {
        RecipeDispatch {
            recipe: recipe(),
            construct: build,
        }
    }

    fn canonical_summary(parameters: &Self::Parameters) -> String {
        parameters.label.clone()
    }
}

fn build(
    parameters: &Params,
    root: ConstructionRoot,
    ctx: &mut ConstructionExecCtx<'_, '_, '_, Toy>,
) {
    ctx.services
        .ordinary_runs
        .set(ctx.services.ordinary_runs.get() + 1);
    ctx.commands
        .entity(root.entity())
        .insert(Built(parameters.label.clone()));
    // Adversarial behaviour for the roster-verification tests. `Sabotage::None`
    // is the ordinary path every other test runs on.
    apply_sabotage(root, ctx);
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
        .try_register_recipe(recipe(), "toy", "tests", "v1")
        .expect("first registration succeeds");
    registry
        .try_register_relation(grudge(), "toy", "tests", "v1", wire_grudge)
        .expect("first registration succeeds");
    registry
}

fn scope() -> ConstructionScope {
    ConstructionScope {
        binding: ContentBinding::Content(ambition_engine_core::ContentEpoch(7)),
        room: Some("room_a".into()),
    }
}

fn request(id: &str) -> ConstructionRequest<Toy> {
    ConstructionRequest {
        sim_id: SimId::placement(id),
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
    // The recipe is derived from the payload, so an "unknown recipe" is now a
    // registry that never declared the one this domain routes to.
    let registry = ConstructionRegistry::<Toy>::default();
    let error = ConstructionPlan::prepare(scope(), vec![request("a")], &nothing_live(), &registry)
        .expect_err("an unregistered recipe must not plan");
    assert_eq!(
        error,
        ConstructionError::UnknownRecipe {
            sim_id: SimId::placement("a"),
            recipe: recipe(),
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
        .try_register_recipe(recipe(), "toy", "tests", "v1")
        .expect("byte-identical re-registration is idempotent");

    let before = registry.deterministic_dump();
    let error = registry
        .try_register_recipe(recipe(), "other", "tests", "v1")
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
        registry.try_register_recipe(RecipeId::new(" "), "toy", "tests", "v1"),
        Err(ConstructionRegistrationError::EmptyIdentity { field: "id" })
    );
    assert_eq!(
        registry.try_register_recipe(recipe(), "", "tests", "v1"),
        Err(ConstructionRegistrationError::EmptyIdentity { field: "owner" })
    );
}

#[test]
fn the_registry_dump_does_not_depend_on_registration_order() {
    let mut forward = ConstructionRegistry::<Toy>::default();
    forward
        .try_register_recipe(RecipeId::new("toy.a"), "toy", "tests", "v1")
        .unwrap();
    forward
        .try_register_recipe(RecipeId::new("toy.b"), "toy", "tests", "v1")
        .unwrap();

    let mut reversed = ConstructionRegistry::<Toy>::default();
    reversed
        .try_register_recipe(RecipeId::new("toy.b"), "toy", "tests", "v1")
        .unwrap();
    reversed
        .try_register_recipe(RecipeId::new("toy.a"), "toy", "tests", "v1")
        .unwrap();

    assert_eq!(forward.deterministic_dump(), reversed.deterministic_dump());
}

// ── The recipe/parameter pairing ─────────────────────────────────────────────
//
// There are no tests here any more, and their absence is the point. A request
// used to carry a `RecipeId` beside its parameters, so `a.recipe = picky()` was
// a thing a test could write and a caller could ship; an `AcceptsFn` checked the
// pairing at preparation and a wrong `true` still reached the constructor's
// `unreachable!` mid-commit. The recipe is now derived from the payload by
// `ConstructionDomain::dispatch`, one exhaustive match that yields the recipe
// identity and its constructor together, so the mispairing is a state that
// cannot be written down and a missing arm is a compile error. `every_parameter_variant_constructs` in `ambition_actors`
// covers the real domain's arms behaviourally.

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
        ConstructionError::RelationCutBySubset {
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

// ── The executor owns the root ───────────────────────────────────────────────

/// A recipe cannot commandeer a body that already exists.
///
/// The previous design ran the recipe and trusted whatever `Entity` it handed
/// back, guarded only by a deferred check that the entity did not already hold a
/// `SimId`. A pre-existing entity WITHOUT one — a presentation node, a helper,
/// anything not yet identified — sailed through that check and had the planned
/// identity stamped onto it. The executor allocates the root now, so there is no
/// return value to distrust: the toy domain below cannot even express the
/// attempt.
#[test]
fn a_recipe_cannot_commandeer_a_pre_existing_entity() {
    #[derive(Component)]
    struct Bystander;

    let registry = registry();
    let plan = ConstructionPlan::prepare(
        scope(),
        vec![request("a"), request("b")],
        &nothing_live(),
        &registry,
    )
    .unwrap();

    let services = Services::default();
    let mut world = World::new();
    // An unidentified entity sitting in the world, exactly the kind the old
    // guard could not protect.
    let bystander = world.spawn(Bystander).id();

    let receipt = commit_into(&mut world, &plan, &services);

    assert!(
        world.get::<SimId>(bystander).is_none(),
        "the pre-existing entity was not given a planned identity"
    );
    assert!(
        world.get::<Bystander>(bystander).is_some(),
        "and it was not otherwise disturbed"
    );
    for id in plan.planned_ids() {
        assert_ne!(
            receipt.entity(&id),
            Some(bystander),
            "no planned row resolved to the pre-existing entity"
        );
    }
}

/// One planned row, one distinct new root — and the identities in the world are
/// exactly the plan's, on exactly as many entities.
#[test]
fn each_planned_row_gets_its_own_fresh_root() {
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

    let roots: BTreeSet<Entity> = plan
        .planned_ids()
        .iter()
        .map(|id| receipt.entity(id).expect("every row committed"))
        .collect();
    assert_eq!(
        roots.len(),
        3,
        "three rows produced three distinct entities"
    );

    let in_world: BTreeSet<SimId> = world
        .query::<&SimId>()
        .iter(&world)
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        in_world,
        plan.planned_ids(),
        "the world holds exactly the planned identities"
    );
    assert_eq!(
        world.query::<&SimId>().iter(&world).count(),
        3,
        "and each identity is on exactly one entity — two rows cannot collapse"
    );
}

// ── Relation cuts, both directions ───────────────────────────────────────────
//
// These were written once, verified against the pre-fix implementation, and then
// silently lost: an edit that replaced from a marker to end-of-file took the
// whole block with it, and the commit reported a test count nobody re-derived.
// They are restored and extended here, and the load-bearing one is called out
// below.

/// **The poison test.** A relation is an `Entity` handle, so rebuilding the
/// TARGET of one is not a private matter for the target's row: `a` grudges `b`,
/// and if `b` is despawned and rebuilt alone then `a` still holds the dead
/// handle. The roster looks right — both identities present — and only the
/// wiring is silently wrong.
///
/// **Demonstrated against `896bfb1`**, which permitted this case on the
/// reasoning that the relation belonged to the untouched source. It failed
/// there with `left: Some(Grudge(1v0))` (the corpse) against
/// `right: Some(Grudge(1v1))` (the rebuilt target). It is not regression-only.
#[test]
fn reconstructing_a_relation_target_alone_must_not_strand_its_source() {
    let registry = registry();
    let plan = feuding_pair(&registry);
    let services = Services::default();
    let mut world = World::new();

    let receipt = commit_into(&mut world, &plan, &services);
    let ea = receipt.entity(&SimId::placement("a")).expect("a committed");
    let old_b = receipt.entity(&SimId::placement("b")).expect("b committed");
    assert_eq!(
        world.get::<Grudge>(ea),
        Some(&Grudge(old_b)),
        "the pair starts correctly wired"
    );

    world.despawn(old_b);
    let result = construct_one_into(&mut world, &plan, &services, &SimId::placement("b"));

    match result {
        Err(error) => assert_eq!(
            error,
            ConstructionError::RelationCutBySubset {
                from: SimId::placement("a"),
                kind: grudge(),
                to: SimId::placement("b"),
            },
            "a refusal must name the relation it would have stranded"
        ),
        Ok(new_b) => assert_eq!(
            world.get::<Grudge>(ea),
            Some(&Grudge(new_b)),
            "a rebuild that SUCCEEDS must leave `a` on the new `b`, not the corpse"
        ),
    }
}

/// The source direction, stated separately so a future one-sided rule cannot
/// pass by covering only the obvious half.
#[test]
fn reconstructing_a_relation_source_alone_is_refused() {
    let registry = registry();
    let plan = feuding_pair(&registry);
    let services = Services::default();
    let mut world = World::new();

    let error = construct_one_into(&mut world, &plan, &services, &SimId::placement("a"))
        .expect_err("rebuilding the source alone must be refused");
    assert_eq!(
        error,
        ConstructionError::RelationCutBySubset {
            from: SimId::placement("a"),
            kind: grudge(),
            to: SimId::placement("b"),
        }
    );
    assert_eq!(
        services.ordinary_runs.get(),
        0,
        "refused before any recipe ran"
    );
}

/// Closure pulls in the target when seeded with the source.
#[test]
fn relation_closure_of_a_source_includes_its_target() {
    let registry = registry();
    let plan = feuding_pair(&registry);
    assert_eq!(
        plan.relation_closure(&BTreeSet::from([SimId::placement("a")])),
        BTreeSet::from([SimId::placement("a"), SimId::placement("b")])
    );
}

/// And the source when seeded with the target — the direction the disproved
/// rule assumed was safe to ignore.
#[test]
fn relation_closure_of_a_target_includes_its_source() {
    let registry = registry();
    let plan = feuding_pair(&registry);
    assert_eq!(
        plan.relation_closure(&BTreeSet::from([SimId::placement("b")])),
        BTreeSet::from([SimId::placement("a"), SimId::placement("b")])
    );
}

/// Closure is transitive: seeding `c` in `a -> b -> c` must pull in `b` and then
/// `a`, or a chain would be rebuilt in stranded fragments.
#[test]
fn relation_closure_is_transitive_across_a_chain() {
    let registry = registry();
    let mut a = request("a");
    a.relations.push(RelationRequest {
        kind: grudge(),
        to: SimId::placement("b"),
    });
    let mut b = request("b");
    b.relations.push(RelationRequest {
        kind: grudge(),
        to: SimId::placement("c"),
    });
    let plan = ConstructionPlan::prepare(
        scope(),
        vec![a, b, request("c"), request("d")],
        &nothing_live(),
        &registry,
    )
    .unwrap();

    assert_eq!(
        plan.relation_closure(&BTreeSet::from([SimId::placement("c")])),
        BTreeSet::from([
            SimId::placement("a"),
            SimId::placement("b"),
            SimId::placement("c"),
        ]),
        "seeding the far end of a chain pulls the whole chain"
    );
    // `d` is in no relation, so it neither pulls nor is pulled.
    assert_eq!(
        plan.relation_closure(&BTreeSet::from([SimId::placement("d")])),
        BTreeSet::from([SimId::placement("d")])
    );
}

/// Rebuilding the closure produces FRESH entity generations and rewires every
/// relation onto them. This is the property the whole rule exists to protect:
/// not merely "nothing is stranded" but "the new wiring names the new bodies".
#[test]
fn rebuilding_a_closure_rewires_relations_onto_the_new_generations() {
    let registry = registry();
    let mut a = request("a");
    a.relations.push(RelationRequest {
        kind: grudge(),
        to: SimId::placement("b"),
    });
    let mut b = request("b");
    b.relations.push(RelationRequest {
        kind: grudge(),
        to: SimId::placement("c"),
    });
    let plan = ConstructionPlan::prepare(
        scope(),
        vec![a, b, request("c")],
        &nothing_live(),
        &registry,
    )
    .unwrap();
    let services = Services::default();
    let mut world = World::new();

    let first = commit_into(&mut world, &plan, &services);
    let old: Vec<Entity> = ["a", "b", "c"]
        .iter()
        .map(|id| first.entity(&SimId::placement(id)).expect("committed"))
        .collect();

    let closure = plan.relation_closure(&BTreeSet::from([SimId::placement("c")]));
    for entity in &old {
        world.despawn(*entity);
    }
    let second = {
        let mut commands = world.commands();
        let plan_scope = scope();
        let mut ctx = ConstructionExecCtx {
            commands: &mut commands,
            scope: &plan_scope,
            session: crate::lifecycle::SessionSpawnScope::UNSCOPED,
            services: &services,
        };
        plan.commit_subset(&closure, &mut ctx)
            .expect("a closed subset is never cut")
    };
    world.flush();

    let new: Vec<Entity> = ["a", "b", "c"]
        .iter()
        .map(|id| second.entity(&SimId::placement(id)).expect("rebuilt"))
        .collect();
    for (before, after) in old.iter().zip(&new) {
        assert_ne!(before, after, "every row really was rebuilt");
    }
    assert_eq!(
        world.get::<Grudge>(new[0]),
        Some(&Grudge(new[1])),
        "a -> b points at the NEW b"
    );
    assert_eq!(
        world.get::<Grudge>(new[1]),
        Some(&Grudge(new[2])),
        "b -> c points at the NEW c"
    );
}

/// A row in no relation at all still rebuilds alone: the rule is about cuts, not
/// a blanket ban on partial commits.
#[test]
fn a_row_in_no_relation_rebuilds_alone() {
    let registry = registry();
    let mut a = request("a");
    a.relations.push(RelationRequest {
        kind: grudge(),
        to: SimId::placement("b"),
    });
    let plan = ConstructionPlan::prepare(
        scope(),
        vec![a, request("b"), request("c")],
        &nothing_live(),
        &registry,
    )
    .unwrap();
    let services = Services::default();
    let mut world = World::new();

    construct_one_into(&mut world, &plan, &services, &SimId::placement("c"))
        .expect("a row outside every relation rebuilds on its own");
    assert_eq!(services.ordinary_runs.get(), 1);
}

// ── The prepared plan is frozen ──────────────────────────────────────────────

/// A domain whose `dispatch` answers differently over time.
///
/// `dispatch` is *expected* to be a pure function of the parameters, but nothing
/// makes it one — an implementation may read an atomic, a config resource, a
/// feature flag flipped by a hot reload. This domain makes that concrete so the
/// freeze can be proven rather than assumed.
mod drifting {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    pub(super) static USE_B: AtomicBool = AtomicBool::new(false);

    #[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) struct BuiltByA;
    #[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) struct BuiltByB;

    /// Counts how many times `dispatch` was consulted, so the test can prove
    /// commit did not consult it again.
    pub(super) static DISPATCHES: std::sync::atomic::AtomicUsize =
        std::sync::atomic::AtomicUsize::new(0);

    pub(super) struct Drifting;

    pub(super) fn recipe_a() -> RecipeId {
        RecipeId::new("drift.a")
    }
    pub(super) fn recipe_b() -> RecipeId {
        RecipeId::new("drift.b")
    }

    fn construct_a(
        _: &(),
        root: ConstructionRoot,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, Self_>,
    ) {
        ctx.commands.entity(root.entity()).insert(BuiltByA);
    }
    fn construct_b(
        _: &(),
        root: ConstructionRoot,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, Self_>,
    ) {
        ctx.commands.entity(root.entity()).insert(BuiltByB);
    }
    type Self_ = Drifting;

    impl ConstructionDomain for Drifting {
        type Parameters = ();
        type Services = ();

        fn dispatch(_: &Self::Parameters) -> RecipeDispatch<Self> {
            DISPATCHES.fetch_add(1, Ordering::SeqCst);
            if USE_B.load(Ordering::SeqCst) {
                RecipeDispatch {
                    recipe: recipe_b(),
                    construct: construct_b,
                }
            } else {
                RecipeDispatch {
                    recipe: recipe_a(),
                    construct: construct_a,
                }
            }
        }

        fn canonical_summary(_: &Self::Parameters) -> String {
            "drift".to_string()
        }
    }
}

/// **Preparation freezes the executable decision, not just its label.**
///
/// Before the constructor was stored on the row, commit called `dispatch` again
/// — so a plan could validate recipe A, dump recipe A, contribute recipe A to
/// the content fingerprint, and then execute constructor B. This fails against
/// that implementation: it builds `BuiltByB` while every canonical surface says
/// `drift.a`.
#[test]
fn commit_runs_the_constructor_preparation_resolved_not_a_fresh_one() {
    use drifting::{BuiltByA, BuiltByB, Drifting, DISPATCHES, USE_B};
    use std::sync::atomic::Ordering;

    USE_B.store(false, Ordering::SeqCst);
    DISPATCHES.store(0, Ordering::SeqCst);

    let mut registry = ConstructionRegistry::<Drifting>::default();
    registry
        .try_register_recipe(drifting::recipe_a(), "drift", "tests", "v1")
        .unwrap();
    registry
        .try_register_recipe(drifting::recipe_b(), "drift", "tests", "v1")
        .unwrap();

    let plan = ConstructionPlan::<Drifting>::prepare(
        scope(),
        vec![ConstructionRequest {
            sim_id: SimId::placement("x"),
            origin: SpawnOrigin::Authored {
                source: "room_a".into(),
                instance: "x".into(),
            },
            parameters: (),
            relations: Vec::new(),
        }],
        &nothing_live(),
        &registry,
    )
    .expect("plans against constructor A");

    let dispatches_after_prepare = DISPATCHES.load(Ordering::SeqCst);
    assert!(
        plan.deterministic_dump().contains("drift.a"),
        "the plan named recipe A"
    );

    // The world changes its mind between preparing and committing.
    USE_B.store(true, Ordering::SeqCst);

    let mut world = World::new();
    let receipt = {
        let mut commands = world.commands();
        let plan_scope = scope();
        let services = ();
        let mut ctx = ConstructionExecCtx::<Drifting> {
            commands: &mut commands,
            scope: &plan_scope,
            session: crate::lifecycle::SessionSpawnScope::UNSCOPED,
            services: &services,
        };
        plan.commit(&mut ctx)
    };
    world.flush();

    let root = receipt
        .entity(&SimId::placement("x"))
        .expect("the row committed");
    assert!(
        world.get::<BuiltByA>(root).is_some(),
        "commit ran the constructor preparation froze"
    );
    assert!(
        world.get::<BuiltByB>(root).is_none(),
        "commit did NOT run the constructor the domain would answer with now"
    );
    assert!(
        plan.deterministic_dump().contains("drift.a"),
        "and every canonical surface still names recipe A"
    );
    assert_eq!(
        DISPATCHES.load(Ordering::SeqCst),
        dispatches_after_prepare,
        "commit consulted the domain zero further times"
    );
}

// ── Relation registrations carry canonical schema metadata ───────────────────

fn other_wire(_: Entity, _: Entity, _: &mut ConstructionExecCtx<'_, '_, '_, Toy>) {}

/// A relation's WIRING can change while its kind and owner stay put, so the
/// schema id is what makes such a change visible. It must reach the dump.
#[test]
fn a_relation_schema_change_changes_the_registry_dump() {
    let dump_for = |schema: &str| {
        let mut registry = ConstructionRegistry::<Toy>::default();
        registry
            .try_register_relation(grudge(), "toy", "aggression", schema, wire_grudge)
            .unwrap();
        registry.deterministic_dump()
    };
    assert_ne!(dump_for("v1"), dump_for("v2"));
}

/// Registration order must not move it, because that dump is hashed into the
/// prepared-content fingerprint.
#[test]
fn relation_registration_order_does_not_change_the_dump() {
    let dump_for = |reverse: bool| {
        let mut registry = ConstructionRegistry::<Toy>::default();
        let kinds = [RelationKind::new("toy.a"), RelationKind::new("toy.b")];
        let kinds: Vec<_> = if reverse {
            kinds.into_iter().rev().collect()
        } else {
            kinds.into_iter().collect()
        };
        for kind in kinds {
            registry
                .try_register_relation(kind, "toy", "tests", "v1", wire_grudge)
                .unwrap();
        }
        registry.deterministic_dump()
    };
    assert_eq!(dump_for(false), dump_for(true));
}

/// Identical metadata AND identical wiring is idempotent; anything else is a
/// conflict, and a rejected registration leaves the registry untouched.
#[test]
fn relation_metadata_conflicts_are_rejected_and_identical_ones_are_idempotent() {
    let mut registry = ConstructionRegistry::<Toy>::default();
    registry
        .try_register_relation(grudge(), "toy", "aggression", "v1", wire_grudge)
        .unwrap();
    registry
        .try_register_relation(grudge(), "toy", "aggression", "v1", wire_grudge)
        .expect("byte-identical re-registration is idempotent");

    let before = registry.deterministic_dump();
    for (owner, source, schema, wire) in [
        ("other", "aggression", "v1", wire_grudge as RelationFn<Toy>),
        ("toy", "other-source", "v1", wire_grudge),
        ("toy", "aggression", "v2", wire_grudge),
        // Same metadata, DIFFERENT behaviour: still a conflict, because the
        // dump could not otherwise tell the two apart.
        ("toy", "aggression", "v1", other_wire),
    ] {
        let error = registry
            .try_register_relation(grudge(), owner, source, schema, wire)
            .expect_err("a differing relation registration must be rejected");
        assert!(matches!(
            error,
            ConstructionRegistrationError::ConflictingRelation { .. }
        ));
    }
    assert_eq!(
        registry.deterministic_dump(),
        before,
        "rejected registrations leave the registry untouched"
    );
}

#[test]
fn empty_relation_metadata_fields_are_rejected() {
    let mut registry = ConstructionRegistry::<Toy>::default();
    for (kind, owner, source, schema, field) in [
        (" ", "toy", "tests", "v1", "id"),
        ("toy.k", "", "tests", "v1", "owner"),
        ("toy.k", "toy", "", "v1", "source"),
        ("toy.k", "toy", "tests", "", "schema id"),
    ] {
        assert_eq!(
            registry.try_register_relation(
                RelationKind::new(kind),
                owner,
                source,
                schema,
                wire_grudge
            ),
            Err(ConstructionRegistrationError::EmptyIdentity { field })
        );
    }
}

// ── Boundary roster verification ─────────────────────────────────────────────
//
// A recipe holds raw `Commands` and the root `Entity`, so every violation below
// is expressible TODAY. These are not hypotheticals guarded by
// `ConstructionRoot` — that type only stops a recipe NOMINATING a pre-existing
// entity as a row's root.

/// What each adversarial toy recipe should do to its root.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sabotage {
    None,
    StripIdentity,
    OverwriteProvenance,
    DespawnRoot,
    DuplicateIdentity,
    SpawnExtraAuthoritativeRoot,
    PresentationChild,
}

thread_local! {
    static SABOTAGE: std::cell::Cell<Sabotage> = const { std::cell::Cell::new(Sabotage::None) };
}

#[derive(Component)]
struct PresentationOnly;

fn apply_sabotage(root: ConstructionRoot, ctx: &mut ConstructionExecCtx<'_, '_, '_, Toy>) {
    match SABOTAGE.with(|s| s.get()) {
        Sabotage::None => {}
        Sabotage::StripIdentity => {
            ctx.commands.entity(root.entity()).remove::<SimId>();
        }
        Sabotage::OverwriteProvenance => {
            ctx.commands
                .entity(root.entity())
                .insert(SpawnOrigin::Dynamic {
                    parent: SimId::placement("nobody"),
                    sequence: 99,
                });
        }
        Sabotage::DespawnRoot => {
            ctx.commands.entity(root.entity()).despawn();
        }
        Sabotage::DuplicateIdentity => {
            // A second body answering to the SAME planned identity. A
            // `BTreeSet<SimId>` comparison cannot see this at all.
            ctx.commands.spawn(SimId::placement("a"));
        }
        Sabotage::SpawnExtraAuthoritativeRoot => {
            // The shape the giant hand limbs already have.
            ctx.commands.spawn(SimId::placement("uninvited"));
        }
        Sabotage::PresentationChild => {
            // Legal: a helper with no authoritative identity.
            ctx.commands.spawn(PresentationOnly);
        }
    }
}

/// Runs a plan under one sabotage and verifies the resulting world.
///
/// The sabotage flag is thread-local, and Rust runs tests on separate threads,
/// so these do not interfere with each other or with the ordinary toy tests.
fn verify_under(
    sabotage: Sabotage,
    plan: &ConstructionPlan<Toy>,
) -> Result<(), Vec<RosterViolation>> {
    SABOTAGE.with(|s| s.set(sabotage));
    let services = Services::default();
    let mut world = World::new();
    let receipt = commit_into(&mut world, plan, &services);
    SABOTAGE.with(|s| s.set(Sabotage::None));

    // The transaction's authoritative scope: every live identity. Nothing here
    // parses an id or infers authority from a name.
    let authoritative: Vec<(SimId, Entity)> = world
        .query::<(Entity, &SimId)>()
        .iter(&world)
        .map(|(entity, id)| (id.clone(), entity))
        .collect();
    let live: std::collections::BTreeSet<Entity> = world.query::<Entity>().iter(&world).collect();
    let origins: BTreeMap<Entity, SpawnOrigin> = world
        .query::<(Entity, &SpawnOrigin)>()
        .iter(&world)
        .map(|(entity, origin)| (entity, origin.clone()))
        .collect();

    verify_committed_roster(
        plan,
        &receipt,
        &TransactionBaseline::default(),
        &authoritative,
        &|entity| live.contains(&entity),
        &|entity| origins.get(&entity).cloned(),
    )
}

fn sabotage_plan() -> ConstructionPlan<Toy> {
    let registry = registry();
    ConstructionPlan::prepare(
        scope(),
        vec![request("a"), request("b")],
        &nothing_live(),
        &registry,
    )
    .unwrap()
}

#[test]
fn a_clean_commit_verifies() {
    assert_eq!(verify_under(Sabotage::None, &sabotage_plan()), Ok(()));
}

#[test]
fn a_recipe_that_strips_its_roots_identity_is_detected() {
    let violations = verify_under(Sabotage::StripIdentity, &sabotage_plan())
        .expect_err("stripping a planned identity must be detected");
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, RosterViolation::Missing { .. })),
        "got {violations:?}"
    );
}

#[test]
fn a_recipe_that_overwrites_its_roots_provenance_is_detected() {
    let violations = verify_under(Sabotage::OverwriteProvenance, &sabotage_plan())
        .expect_err("rewriting provenance must be detected");
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, RosterViolation::ProvenanceChanged { .. })),
        "got {violations:?}"
    );
}

#[test]
fn a_recipe_that_despawns_its_root_is_detected() {
    let violations = verify_under(Sabotage::DespawnRoot, &sabotage_plan())
        .expect_err("despawning a planned root must be detected");
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, RosterViolation::Missing { .. })),
        "got {violations:?}"
    );
}

/// The case set comparison is blind to: the identity SET is exactly right and
/// two bodies answer to one of them.
#[test]
fn a_duplicated_planned_identity_is_detected() {
    let plan = sabotage_plan();
    let violations = verify_under(Sabotage::DuplicateIdentity, &plan)
        .expect_err("a duplicated identity must be detected");
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, RosterViolation::Duplicated { count, .. } if *count > 1)),
        "got {violations:?}"
    );
}

/// The shape the giant hand limbs already have today.
#[test]
fn an_unplanned_authoritative_root_is_detected() {
    let violations = verify_under(Sabotage::SpawnExtraAuthoritativeRoot, &sabotage_plan())
        .expect_err("an authoritative root no row named must be detected");
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, RosterViolation::Unplanned { .. })),
        "got {violations:?}"
    );
}

/// Presentation-only helpers stay legal — they carry no authoritative identity,
/// so they are simply not in the transaction's authoritative scope.
#[test]
fn a_presentation_only_child_is_permitted() {
    assert_eq!(
        verify_under(Sabotage::PresentationChild, &sabotage_plan()),
        Ok(())
    );
}

/// Identities that were already live when the transaction began are not
/// "unplanned" — that is what the baseline is for.
#[test]
fn a_baseline_identity_is_not_reported_as_unplanned() {
    let plan = sabotage_plan();
    let receipt = ConstructionReceipt::default();
    let baseline = TransactionBaseline::new(BTreeSet::from([SimId::placement("survivor")]));
    let violations = verify_committed_roster(
        &plan,
        &receipt,
        &baseline,
        &[(
            SimId::placement("survivor"),
            Entity::from_raw_u32(7).unwrap(),
        )],
        &|_| true,
        &|_| None,
    )
    .expect_err("the plan's own rows are still missing");
    assert!(
        !violations
            .iter()
            .any(|v| matches!(v, RosterViolation::Unplanned { .. })),
        "a pre-existing identity is not an unplanned root: {violations:?}"
    );
}
