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
use crate::lifecycle::SessionSpawnScope;
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
    /// The toy's relations are pure adjacency. Fact-carrying relations are
    /// proven against the real `Limb` rig in `ambition_actors`, where the slot
    /// and home offset are genuinely host-relative rather than invented for a
    /// fixture.
    type Relation = ToyRelation;
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

    fn dispatch_relation(relation: &Self::Relation) -> RelationDispatch<Self> {
        match relation {
            ToyRelation::Grudge => RelationDispatch {
                kind: grudge(),
                ops: grudge_ops(),
            },
        }
    }

    fn canonical_relation_summary(relation: &Self::Relation) -> String {
        match relation {
            ToyRelation::Grudge => "-".to_string(),
        }
    }
}

/// What one toy relation IS. A one-variant enum is still the right shape: the
/// KIND is derived from it, so a request cannot name a kind that disagrees with
/// what it carries.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ToyRelation {
    Grudge,
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

fn wire_grudge(
    from: Entity,
    to: Entity,
    _relation: &ToyRelation,
    ctx: &mut ConstructionExecCtx<'_, '_, '_, Toy>,
) {
    // Relation-side sabotage, so the adversarial tests exercise the SAME wiring
    // path ordinary construction takes. `RelationSabotage::None` is that path.
    match RELATION_SABOTAGE.with(|s| s.get()) {
        RelationSabotage::None => {
            ctx.commands.entity(from).insert(Grudge(to));
        }
        // The case a receipt cannot see: the function ran, the world is unchanged.
        RelationSabotage::NoOp => {}
        RelationSabotage::WrongTarget => {
            let elsewhere = ctx.commands.spawn_empty().id();
            ctx.commands.entity(from).insert(Grudge(elsewhere));
        }
        RelationSabotage::WrongSource => {
            let elsewhere = ctx.commands.spawn_empty().id();
            ctx.commands.entity(elsewhere).insert(Grudge(to));
        }
        RelationSabotage::RemovedAfterWiring => {
            ctx.commands.entity(from).insert(Grudge(to));
            // A later command in the same flush, exactly as a competing system
            // would issue.
            ctx.commands.entity(from).remove::<Grudge>();
        }
        RelationSabotage::OverwrittenByAnotherCommand => {
            ctx.commands.entity(from).insert(Grudge(to));
            let usurper = ctx.commands.spawn_empty().id();
            ctx.commands.entity(from).insert(Grudge(usurper));
        }
    }
}

/// The toy grudge's postcondition: read the component, do not trust the call.
fn verify_grudge(
    world: &World,
    from: Entity,
    to: Entity,
    _relation: &ToyRelation,
) -> RelationCheck {
    match world.get::<Grudge>(from) {
        None => RelationCheck::NotInstalled,
        Some(Grudge(found)) if *found == to => RelationCheck::Installed,
        Some(Grudge(found)) => RelationCheck::WrongTarget {
            found: Some(*found),
        },
    }
}

fn grudge_ops() -> RelationOps<Toy> {
    RelationOps {
        wire: wire_grudge,
        verify: verify_grudge,
    }
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
        .try_register_relation(grudge(), "toy", "tests", "v1")
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
            to: SimId::placement(second),
            relation: ToyRelation::Grudge,
        });
        let mut b = request(second);
        b.relations.push(RelationRequest {
            to: SimId::placement(first),
            relation: ToyRelation::Grudge,
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
        to: SimId::placement("b"),
        relation: ToyRelation::Grudge,
    });
    let mut b = request("b");
    b.origin = SpawnOrigin::Dynamic {
        parent: SimId::placement("a"),
        sequence: 4,
    };
    let plan = ConstructionPlan::prepare(scope(), vec![a, b], &nothing_live(), &registry).unwrap();

    assert_eq!(
        plan.deterministic_dump(),
        "construction-plan-v3\n\
         epoch:7\n\
         room\troom_a\n\
         entity\tplacement:a\ttoy.build\tauthored\troom_a\ta\ta\n\
         entity\tplacement:b\ttoy.build\tdynamic\tplacement:a\t4\tb\n\
         relation\tplacement:a\ttoy.grudge\tplacement:b\t-\n"
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
        to: SimId::placement("ghost"),
        relation: ToyRelation::Grudge,
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
        to: SimId::placement("veteran"),
        relation: ToyRelation::Grudge,
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

/// A relation whose KIND nothing declared is refused.
///
/// The kind is derived from the relation value now, so the way to reach this is
/// a registry that never declared it — which is also the only way it can still
/// happen. A request naming a kind that disagrees with what it carries is no
/// longer a state the type system permits, which is what
/// `a_relation_cannot_name_a_kind_that_disagrees_with_its_payload` documents.
#[test]
fn an_unregistered_relation_kind_is_rejected() {
    let mut registry = ConstructionRegistry::<Toy>::default();
    registry
        .try_register_recipe(recipe(), "toy", "tests", "v1")
        .expect("the recipe registers");
    let mut a = request("a");
    a.relations.push(RelationRequest {
        to: SimId::placement("a"),
        relation: ToyRelation::Grudge,
    });
    let error = ConstructionPlan::prepare(scope(), vec![a], &nothing_live(), &registry)
        .expect_err("an unregistered relation kind must not plan");
    assert_eq!(
        error,
        ConstructionError::UnknownRelationKind {
            from: SimId::placement("a"),
            kind: grudge(),
        }
    );
}

/// **The kind/payload mismatch is unrepresentable.**
///
/// This is a compile-shape test, and deliberately so: there is nothing to assert
/// at runtime because the illegal state cannot be constructed. A
/// `RelationRequest` has exactly one field describing what the relation is, and
/// [`ConstructionDomain::dispatch_relation`] derives the kind from it, so the
/// pairing is a function rather than two independent inputs.
///
/// What this replaced: `RelationRequest { kind, to, payload }`, where nothing
/// checked that `kind` and `payload` described the same relation. The registry
/// validated the kind and knew nothing of payloads; the wiring function
/// destructured the payload and `unreachable!`d on a mismatch — during COMMIT,
/// after the outgoing room had been retired. See
/// `crates/ambition_actors/src/construction/tests.rs` for the domain-level
/// version, which is where a genuine multi-variant relation enum lives.
#[test]
fn a_relation_cannot_name_a_kind_that_disagrees_with_its_payload() {
    let request = RelationRequest::<Toy> {
        to: SimId::placement("a"),
        relation: ToyRelation::Grudge,
    };
    // The kind is DERIVED. There is no second input for it to disagree with.
    assert_eq!(
        Toy::dispatch_relation(&request.relation).kind,
        grudge(),
        "the kind a relation resolves to is a function of the relation itself"
    );
}

/// Two rows declaring the same relation are refused, not silently merged.
///
/// The receipt keys wired relations on `(from, kind, to)`, so a duplicate would
/// execute twice and be recorded once — a receipt saying "wired" over a world
/// holding it applied twice. For an accumulating relation that is real
/// corruption, and every count-based check passes through it.
#[test]
fn a_duplicate_relation_is_refused_before_ordering() {
    let registry = registry();
    let mut a = request("a");
    let b = request("b");
    a.relations.push(RelationRequest {
        to: SimId::placement("b"),
        relation: ToyRelation::Grudge,
    });
    a.relations.push(RelationRequest {
        to: SimId::placement("b"),
        relation: ToyRelation::Grudge,
    });
    let error = ConstructionPlan::prepare(scope(), vec![a, b], &nothing_live(), &registry)
        .expect_err("a duplicate relation must not plan");
    assert_eq!(
        error,
        ConstructionError::DuplicateRelation {
            from: SimId::placement("a"),
            kind: grudge(),
            to: SimId::placement("b"),
        }
    );
}

/// Request arrival order reaches neither the dump nor the execution sequence,
/// **including for relations**.
///
/// Relations sort by `(from, kind, to)`, which is a TOTAL order only because
/// duplicates are refused above — with duplicates admitted, two rows could sort
/// equal and their relative order would be whatever the sort happened to do.
#[test]
fn relation_request_order_does_not_change_the_plan() {
    let registry = registry();
    let build = |reversed: bool| {
        let mut a = request("a");
        let mut b = request("b");
        a.relations.push(RelationRequest {
            to: SimId::placement("b"),
            relation: ToyRelation::Grudge,
        });
        b.relations.push(RelationRequest {
            to: SimId::placement("a"),
            relation: ToyRelation::Grudge,
        });
        let requests = if reversed { vec![b, a] } else { vec![a, b] };
        ConstructionPlan::prepare(scope(), requests, &nothing_live(), &registry)
            .expect("both orders plan")
            .deterministic_dump()
    };
    assert_eq!(
        build(false),
        build(true),
        "two callers requesting the same set in different orders must produce one plan"
    );
}

/// **Registration order cannot change committed behaviour.**
///
/// The registry holds no function pointers at all now, so there is no
/// implementation for a second registration to install or to lose a race to:
/// wiring comes from `dispatch_relation`. Registering in either order yields the
/// same dump AND the same execution, which is what the old first-wins table
/// could not promise — it accepted two registrations with identical metadata and
/// different `RelationOps` as "idempotent" and kept whichever arrived first,
/// under a dump and a fingerprint that could not tell them apart.
#[test]
fn relation_registration_order_changes_neither_the_dump_nor_behaviour() {
    let other = RelationKind::new("toy.other");
    let mut forward = ConstructionRegistry::<Toy>::default();
    forward
        .try_register_recipe(recipe(), "toy", "tests", "v1")
        .expect("recipe registers");
    forward
        .try_register_relation(grudge(), "toy", "tests", "v1")
        .expect("grudge registers");
    forward
        .try_register_relation(other.clone(), "toy", "tests", "v1")
        .expect("other registers");

    let mut reversed = ConstructionRegistry::<Toy>::default();
    reversed
        .try_register_relation(other, "toy", "tests", "v1")
        .expect("other registers");
    reversed
        .try_register_relation(grudge(), "toy", "tests", "v1")
        .expect("grudge registers");
    reversed
        .try_register_recipe(recipe(), "toy", "tests", "v1")
        .expect("recipe registers");

    assert_eq!(
        forward.deterministic_dump(),
        reversed.deterministic_dump(),
        "ordered storage means insertion order does not reach the dump"
    );

    let plan = |registry: &ConstructionRegistry<Toy>| {
        let mut a = request("a");
        let b = request("b");
        a.relations.push(RelationRequest {
            to: SimId::placement("b"),
            relation: ToyRelation::Grudge,
        });
        ConstructionPlan::prepare(scope(), vec![a, b], &nothing_live(), registry)
            .expect("both registries plan the same relation")
    };
    let services = Services::default();
    let (forward_world, forward_receipt) = commit(&plan(&forward), &services);
    let (reversed_world, reversed_receipt) = commit(&plan(&reversed), &services);
    assert_eq!(
        forward_receipt.relations_wired(),
        reversed_receipt.relations_wired()
    );
    let grudge_of = |world: &World, receipt: &ConstructionReceipt| {
        let from = receipt
            .entity(&SimId::placement("a"))
            .expect("`a` committed");
        world.get::<Grudge>(from).copied()
    };
    assert!(
        grudge_of(&forward_world, &forward_receipt).is_some(),
        "the relation was actually installed"
    );
    assert_eq!(
        grudge_of(&forward_world, &forward_receipt).map(|Grudge(target)| forward_receipt
            .committed_ids()
            .contains(&SimId::placement("b"))
            && target == reversed_receipt.entity(&SimId::placement("b")).unwrap()),
        grudge_of(&reversed_world, &reversed_receipt).map(|Grudge(target)| reversed_receipt
            .committed_ids()
            .contains(&SimId::placement("b"))
            && target == reversed_receipt.entity(&SimId::placement("b")).unwrap()),
        "both orders wire the same relation onto the same identity"
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
        to: SimId::placement("b"),
        relation: ToyRelation::Grudge,
    });
    let mut b = request("b");
    b.relations.push(RelationRequest {
        to: SimId::placement("a"),
        relation: ToyRelation::Grudge,
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
        to: SimId::placement("b"),
        relation: ToyRelation::Grudge,
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
        to: SimId::placement("b"),
        relation: ToyRelation::Grudge,
    });
    let mut b = request("b");
    b.relations.push(RelationRequest {
        to: SimId::placement("c"),
        relation: ToyRelation::Grudge,
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
        to: SimId::placement("b"),
        relation: ToyRelation::Grudge,
    });
    let mut b = request("b");
    b.relations.push(RelationRequest {
        to: SimId::placement("c"),
        relation: ToyRelation::Grudge,
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
        to: SimId::placement("b"),
        relation: ToyRelation::Grudge,
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
        type Relation = ();
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

        fn dispatch_relation(_: &Self::Relation) -> RelationDispatch<Self> {
            unreachable!("the drift fixture declares no relations")
        }

        fn canonical_relation_summary(_: &Self::Relation) -> String {
            "-".to_string()
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

/// A relation's WIRING can change while its kind and owner stay put, so the
/// schema id is what makes such a change visible. It must reach the dump.
#[test]
fn a_relation_schema_change_changes_the_registry_dump() {
    let dump_for = |schema: &str| {
        let mut registry = ConstructionRegistry::<Toy>::default();
        registry
            .try_register_relation(grudge(), "toy", "aggression", schema)
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
                .try_register_relation(kind, "toy", "tests", "v1")
                .unwrap();
        }
        registry.deterministic_dump()
    };
    assert_eq!(dump_for(false), dump_for(true));
}

/// Differing metadata is a conflict, and a rejected registration leaves the
/// registry untouched.
#[test]
fn relation_metadata_conflicts_are_rejected_and_identical_ones_are_idempotent() {
    let mut registry = ConstructionRegistry::<Toy>::default();
    registry
        .try_register_relation(grudge(), "toy", "aggression", "v1")
        .unwrap();
    registry
        .try_register_relation(grudge(), "toy", "aggression", "v1")
        .expect("byte-identical re-registration is idempotent");

    let before = registry.deterministic_dump();
    for (owner, source, schema) in [
        ("other", "aggression", "v1"),
        ("toy", "other-source", "v1"),
        ("toy", "aggression", "v2"),
    ] {
        let error = registry
            .try_register_relation(grudge(), owner, source, schema)
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

/// **A registration cannot supply executable behaviour at all.**
///
/// The first-wins hazard this replaces was concrete: `try_register_relation`
/// took a `RelationOps` and decided idempotence on METADATA alone, so two
/// registrations agreeing on owner/source/schema and disagreeing on the wiring
/// function were accepted as "the same registration" and the first one won.
/// The dump and the prepared-content fingerprint were byte-identical either way,
/// so two builds could execute different construction behaviour under the same
/// declared content identity, decided by plugin insertion order.
///
/// An earlier attempt compared `std::ptr::fn_addr_eq` instead, which a registry
/// contract cannot rest on: the compiler may merge two identical functions to
/// one address and emit one function at several addresses across codegen units,
/// so the same pair of registrations could conflict or not between builds. And
/// pointer comparison never caught the realistic case anyway — editing a
/// function's body does not move it.
///
/// Both are gone because the table holds no functions. Wiring is resolved by
/// `ConstructionDomain::dispatch_relation`, one exhaustive match in the domain
/// that defines the relation enum, so there is nothing here to race for.
/// `relation_registration_order_changes_neither_the_dump_nor_behaviour` proves
/// the consequence end-to-end.
#[test]
fn a_relation_registration_declares_identity_and_nothing_executable() {
    let mut registry = ConstructionRegistry::<Toy>::default();
    registry
        .try_register_relation(grudge(), "toy", "aggression", "v1")
        .expect("first registration succeeds");
    let dump = registry.deterministic_dump();
    registry
        .try_register_relation(grudge(), "toy", "aggression", "v1")
        .expect("identical declared metadata re-registers idempotently");
    assert_eq!(
        registry.deterministic_dump(),
        dump,
        "an idempotent re-registration changes nothing"
    );
    // A metadata disagreement is still a transactional refusal.
    assert!(matches!(
        registry.try_register_relation(grudge(), "toy", "aggression", "v2"),
        Err(ConstructionRegistrationError::ConflictingRelation { .. })
    ));
    assert_eq!(
        registry.deterministic_dump(),
        dump,
        "a rejected registration leaves the registry untouched"
    );
    // Whatever any registration said, the ops come from the domain.
    assert_eq!(Toy::dispatch_relation(&ToyRelation::Grudge).kind, grudge());
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
            registry.try_register_relation(RelationKind::new(kind), owner, source, schema),
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
    SpawnUnownedIdentity,
    SpawnForeignScopedRoot,
    PresentationChild,
    /// An entity that CLAIMS a legacy construction family by name. Whether it
    /// is tolerated depends on whether the name is enumerated.
    SpawnLegacyRoot(&'static str),
    RemoveTransactionId,
    OverwriteTransactionId,
}

/// What each adversarial toy WIRING should do, so the relation postcondition
/// checks exercise the real wiring path rather than a stand-in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RelationSabotage {
    None,
    NoOp,
    WrongTarget,
    WrongSource,
    RemovedAfterWiring,
    OverwrittenByAnotherCommand,
}

thread_local! {
    static SABOTAGE: std::cell::Cell<Sabotage> = const { std::cell::Cell::new(Sabotage::None) };
    static RELATION_SABOTAGE: std::cell::Cell<RelationSabotage> =
        const { std::cell::Cell::new(RelationSabotage::None) };
}

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
            // A root wearing THIS transaction's ownership that no plan row
            // named. The caller never lists it, which is exactly why the scope
            // is read from the world instead of from the caller.
            ctx.commands.spawn((
                SimId::placement("uninvited"),
                ctx.scope.transaction(ctx.session),
            ));
        }
        Sabotage::SpawnUnownedIdentity => {
            // A real identity, minted outside the planner, classified by
            // nothing. Indistinguishable from a recipe inventing a root, which
            // is why it is fatal.
            ctx.commands.spawn(SimId::placement("mystery_body"));
        }
        Sabotage::SpawnLegacyRoot(family) => {
            // The shape the giant hand limbs have TODAY, once they say so: a
            // real identity minted outside the planner, explicitly naming the
            // un-migrated family that minted it.
            ctx.commands.spawn((
                SimId::placement("giant/hand_left"),
                LegacyConstructionRoot::new(family),
            ));
        }
        Sabotage::RemoveTransactionId => {
            ctx.commands.entity(root.entity()).remove::<TransactionId>();
        }
        Sabotage::OverwriteTransactionId => {
            let elsewhere = ConstructionScope {
                binding: ContentBinding::Content(ambition_engine_core::ContentEpoch(9)),
                room: Some("some_other_room".into()),
            };
            ctx.commands
                .entity(root.entity())
                .insert(elsewhere.transaction(SessionSpawnScope::UNSCOPED));
        }
        Sabotage::SpawnForeignScopedRoot => {
            // Another live transaction's root. Present in the world, none of
            // this transaction's business, and must not be reported.
            let elsewhere = ConstructionScope {
                binding: ContentBinding::Content(ambition_engine_core::ContentEpoch(9)),
                room: Some("some_other_room".into()),
            };
            ctx.commands.spawn((
                SimId::placement("other_rooms_occupant"),
                elsewhere.transaction(SessionSpawnScope::UNSCOPED),
            ));
        }
        Sabotage::PresentationChild => {
            // Legal: identity-bearing, but explicitly declared non-authoritative.
            // It must carry an identity for this to prove anything — an entity
            // with no `SimId` was never in scope to begin with.
            ctx.commands
                .spawn((SimId::placement("a/visual"), PresentationOnly));
        }
    }
}

/// Runs a plan under one sabotage and verifies the resulting world, gathering
/// the scope from the world exactly as the production boundary does.
///
/// The sabotage flags are thread-local, and Rust runs tests on separate
/// threads, so these do not interfere with each other or with the ordinary toy
/// tests.
fn verify_under(
    sabotage: Sabotage,
    plan: &ConstructionPlan<Toy>,
) -> Result<(), Vec<RosterViolation>> {
    verify_under_both(sabotage, RelationSabotage::None, plan)
}

fn verify_under_both(
    sabotage: Sabotage,
    relation_sabotage: RelationSabotage,
    plan: &ConstructionPlan<Toy>,
) -> Result<(), Vec<RosterViolation>> {
    SABOTAGE.with(|s| s.set(sabotage));
    RELATION_SABOTAGE.with(|s| s.set(relation_sabotage));
    let services = Services::default();
    let mut world = World::new();
    // Captured before construction, as production captures it: at this point the
    // world is empty, so the baseline is empty and every root below is this
    // transaction's doing.
    let baseline =
        TransactionBaseline::capture(&mut world).expect("an empty world has no duplicates");
    let receipt = commit_into(&mut world, plan, &services);
    SABOTAGE.with(|s| s.set(Sabotage::None));
    RELATION_SABOTAGE.with(|s| s.set(RelationSabotage::None));

    let transaction = plan.scope().transaction(SessionSpawnScope::UNSCOPED);
    let scope = AuthoritativeScope::gather(&mut world, &transaction);
    verify_committed_roster(plan, &receipt, &baseline, &scope, &world)
}

/// **A planned relation the executor never wired is a failure, not a skip.**
///
/// The verifier used to `continue` past any relation absent from
/// `relations_wired`, which made the whole postcondition pass vacuous for
/// exactly the relations that failed hardest: a relation the executor never
/// attempted has no receipt entry, so "check the ones that were wired" checked
/// everything except the broken one. Which relations were OWED is now derived
/// from the identities actually committed.
///
/// The executor cannot be made to skip a relation from the outside — that is
/// the point of the design — so this drops the entry from the receipt directly,
/// standing in for an executor seam that failed to wire.
#[test]
fn a_planned_relation_missing_from_the_receipt_is_fatal() {
    let plan = related_plan();
    let services = Services::default();
    let mut world = World::new();
    let baseline =
        TransactionBaseline::capture(&mut world).expect("an empty world has no duplicates");
    let mut receipt = commit_into(&mut world, &plan, &services);

    let key = (SimId::placement("a"), grudge(), SimId::placement("b"));
    assert!(
        receipt.relations_wired.remove(&key),
        "the fixture must have wired the relation before we drop it"
    );

    let transaction = plan.scope().transaction(SessionSpawnScope::UNSCOPED);
    let scope = AuthoritativeScope::gather(&mut world, &transaction);
    let violations = verify_committed_roster(&plan, &receipt, &baseline, &scope, &world)
        .expect_err("an unwired planned relation must be reported");
    let missing: Vec<_> = violations
        .iter()
        .filter(|v| matches!(v, RosterViolation::RelationMissingFromReceipt { .. }))
        .collect();
    assert_eq!(missing.len(), 1, "got {violations:?}");
    assert_eq!(missing[0].severity(), Severity::Fatal);
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

/// An authoritative root wearing this transaction's ownership that no plan row
/// named. **The caller never mentions it** — the scope is read from the world.
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

/// **An arbitrary identity-bearing entity with no ownership stamp is FATAL.**
///
/// This used to be `Severity::Unmigrated` — reported, published anyway — on the
/// reasoning that "unowned" meant "a family that has not migrated yet". It did
/// not mean that. It meant nothing in the world said what the entity was, which
/// is equally the signature of a recipe inventing an authoritative root, so the
/// one failure the verifier exists to catch was the one it tolerated. A genuine
/// legacy family now claims that status by name; see
/// `the_known_legacy_exception_maps_to_unmigrated_severity`.
#[test]
fn an_arbitrary_unowned_identity_is_fatal() {
    let violations = verify_under(Sabotage::SpawnUnownedIdentity, &sabotage_plan())
        .expect_err("an unowned identity must be reported");
    let unowned: Vec<_> = violations
        .iter()
        .filter(|v| matches!(v, RosterViolation::UnownedIdentity { .. }))
        .collect();
    assert_eq!(unowned.len(), 1, "got {violations:?}");
    assert_eq!(
        unowned[0].severity(),
        Severity::Fatal,
        "nothing in the world says what built this entity, so the transaction is unpublishable"
    );
}

/// The known-legacy EXCEPTION mechanism: an enumerated family maps to
/// `Severity::Unmigrated` (reported, published), an unenumerated one to
/// `Severity::Fatal`.
///
/// **The ledger is empty as of Checkpoint B** — the giant hands were the last
/// entry — so there is no production family to drive the reported-but-not-fatal
/// path end to end. The mechanism survives for the families that have yet to
/// migrate to eager stamping, so this pins the severity MAPPING directly rather
/// than through a live family: `LegacyConstruction` is `Unmigrated`,
/// `UnknownLegacyFamily` is `Fatal`. When a family re-enters
/// `KNOWN_LEGACY_FAMILIES`, the end-to-end `Sabotage::SpawnLegacyRoot` path
/// covers it again.
#[test]
fn the_known_legacy_exception_maps_to_unmigrated_severity() {
    assert!(
        KNOWN_LEGACY_FAMILIES.is_empty(),
        "Checkpoint B emptied the ledger; if a family re-enters, restore the \
         end-to-end SpawnLegacyRoot assertion"
    );
    let known = RosterViolation::LegacyConstruction {
        sim_id: SimId::placement("x"),
        family: "some-future-family".into(),
    };
    assert_eq!(known.severity(), Severity::Unmigrated);
    let unknown = RosterViolation::UnknownLegacyFamily {
        sim_id: SimId::placement("x"),
        family: "not-enumerated".into(),
    };
    assert_eq!(unknown.severity(), Severity::Fatal);
}

/// A root claiming a legacy family nobody enumerated is FATAL.
///
/// Without this the marker would be a universal opt-out: any entity could
/// exempt itself from verification by asserting it was legacy. An unrecognised
/// claim of legacy status is not evidence of legacy status.
#[test]
fn an_unknown_legacy_family_is_fatal() {
    assert!(
        !KNOWN_LEGACY_FAMILIES.contains(&"not-a-real-family"),
        "the fixture must name a family the ledger does not list"
    );
    let violations = verify_under(
        Sabotage::SpawnLegacyRoot("not-a-real-family"),
        &sabotage_plan(),
    )
    .expect_err("an unrecognised legacy claim must be reported");
    let unknown: Vec<_> = violations
        .iter()
        .filter(|v| matches!(v, RosterViolation::UnknownLegacyFamily { .. }))
        .collect();
    assert_eq!(unknown.len(), 1, "got {violations:?}");
    assert_eq!(unknown[0].severity(), Severity::Fatal);
}

/// **A planned root that lost its ownership stamp is FATAL.**
///
/// The executor stamps identity, provenance, and ownership in one insert, and
/// verification checked the first two. The third is the one that DRIVES scope
/// classification, so an unstamped planned root is invisible to the next
/// transaction's gathering — it becomes somebody else's problem permanently.
#[test]
fn a_planned_root_stripped_of_its_ownership_is_fatal() {
    let violations = verify_under(Sabotage::RemoveTransactionId, &sabotage_plan())
        .expect_err("a planned root without ownership must be detected");
    // The sabotage runs in the recipe, so it strips EVERY planned row: both are
    // reported, which is the point — the check is per-root, not a spot check.
    let lost: Vec<_> = violations
        .iter()
        .filter(|v| matches!(v, RosterViolation::OwnershipLost { found: None, .. }))
        .collect();
    assert_eq!(lost.len(), 2, "got {violations:?}");
    assert!(lost.iter().all(|v| v.severity() == Severity::Fatal));
}

/// Ownership OVERWRITTEN, rather than removed, is equally fatal — and the
/// finding names what it found, so the two are diagnosable apart.
#[test]
fn a_planned_root_restamped_by_another_transaction_is_fatal() {
    let violations = verify_under(Sabotage::OverwriteTransactionId, &sabotage_plan())
        .expect_err("a restamped planned root must be detected");
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, RosterViolation::OwnershipLost { found: Some(_), .. })),
        "got {violations:?}"
    );
}

/// A root belonging to a DIFFERENT live transaction is out of jurisdiction, and
/// must not be reported at all.
#[test]
fn a_root_from_another_transaction_scope_is_not_reported() {
    assert_eq!(
        verify_under(Sabotage::SpawnForeignScopedRoot, &sabotage_plan()),
        Ok(())
    );
}

/// Presentation-only helpers stay legal even though they carry an identity —
/// they are classified out by an explicit component, never by their spelling.
#[test]
fn a_presentation_only_child_is_permitted() {
    assert_eq!(
        verify_under(Sabotage::PresentationChild, &sabotage_plan()),
        Ok(())
    );
}

// ── Baseline identity and multiplicity ───────────────────────────────────────
//
// An identity-only baseline could not tell an original from a replacement, a
// survivor from a look-alike, or a pre-existing duplicate from a clean start.
// These are the cases that motivated capturing entities.

/// A world with a live entity, ready to be a baseline.
fn world_with(ids: &[(&str, Entity)]) -> World {
    let mut world = World::new();
    for (id, _) in ids {
        world.spawn(SimId::placement(*id));
    }
    world
}

fn baseline_of(world: &mut World) -> TransactionBaseline {
    TransactionBaseline::capture(world).expect("no duplicates in this fixture")
}

fn verify_world(
    world: &mut World,
    plan: &ConstructionPlan<Toy>,
    receipt: &ConstructionReceipt,
    baseline: &TransactionBaseline,
) -> Result<(), Vec<RosterViolation>> {
    let transaction = plan.scope().transaction(SessionSpawnScope::UNSCOPED);
    let scope = AuthoritativeScope::gather(world, &transaction);
    verify_committed_roster(plan, receipt, baseline, &scope, world)
}

fn empty_plan() -> ConstructionPlan<Toy> {
    ConstructionPlan::prepare(scope(), Vec::new(), &nothing_live(), &registry()).unwrap()
}

/// (1) A second entity spawned with a BASELINE identity. The identity set is
/// unchanged, so a set comparison sees nothing.
#[test]
fn a_second_entity_wearing_a_baseline_identity_is_detected() {
    let mut world = world_with(&[("survivor", Entity::PLACEHOLDER)]);
    let baseline = baseline_of(&mut world);
    world.spawn(SimId::placement("survivor"));

    let violations = verify_world(
        &mut world,
        &empty_plan(),
        &ConstructionReceipt::default(),
        &baseline,
    )
    .expect_err("a duplicated baseline identity must be detected");
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, RosterViolation::Duplicated { count, .. } if *count == 2)),
        "got {violations:?}"
    );
}

/// (2) A baseline entity despawned and replaced by another entity carrying the
/// same identity. **Exactly one occupant, exactly the right identity, wrong
/// body.** This is the case an identity-only baseline is structurally blind to.
#[test]
fn a_baseline_entity_replaced_by_a_look_alike_is_detected() {
    let mut world = World::new();
    let original = world.spawn(SimId::placement("survivor")).id();
    let baseline = baseline_of(&mut world);
    world.despawn(original);
    let replacement = world.spawn(SimId::placement("survivor")).id();

    let violations = verify_world(
        &mut world,
        &empty_plan(),
        &ConstructionReceipt::default(),
        &baseline,
    )
    .expect_err("a replaced baseline entity must be detected");
    assert!(
        violations.iter().any(|v| matches!(
            v,
            RosterViolation::BaselineReplaced { expected, found, .. }
                if *expected == original && *found == replacement
        )),
        "got {violations:?}"
    );
}

/// (3) Duplicate authoritative identities present BEFORE the transaction. The
/// capture refuses rather than collapsing them, because every later
/// multiplicity check would otherwise be measured against a baseline that had
/// already lost the duplicate.
#[test]
fn a_baseline_that_already_contains_duplicates_is_refused_at_capture() {
    let mut world = World::new();
    let first = world.spawn(SimId::placement("twin")).id();
    let second = world.spawn(SimId::placement("twin")).id();
    let error = TransactionBaseline::capture(&mut world)
        .expect_err("a pre-existing duplicate identity must refuse capture");
    let BaselineCaptureError::DuplicateIdentity { sim_id, entities } = error;
    assert_eq!(sim_id, SimId::placement("twin"));
    let mut expected = vec![first, second];
    expected.sort();
    assert_eq!(entities, expected);
}

/// (4) An untouched baseline entity whose provenance is rewritten under it.
#[test]
fn mutating_an_untouched_baseline_entitys_provenance_is_detected() {
    let mut world = World::new();
    let survivor = world
        .spawn((
            SimId::placement("survivor"),
            SpawnOrigin::Authored {
                source: "room_a".into(),
                instance: "survivor".into(),
            },
        ))
        .id();
    let baseline = baseline_of(&mut world);
    world.entity_mut(survivor).insert(SpawnOrigin::Dynamic {
        parent: SimId::placement("nobody"),
        sequence: 1,
    });

    let violations = verify_world(
        &mut world,
        &empty_plan(),
        &ConstructionReceipt::default(),
        &baseline,
    )
    .expect_err("rewriting an untouched entity's provenance must be detected");
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, RosterViolation::BaselineProvenanceChanged { .. })),
        "got {violations:?}"
    );
}

/// (5) A DECLARED reconstruction that changes only the permitted entity
/// generation verifies clean — and the same world without the declaration does
/// not. Permission is declared, never inferred from the plan naming the id.
#[test]
fn a_declared_reconstruction_changes_exactly_one_generation() {
    let services = Services::default();
    let mut world = World::new();
    let original = world.spawn(SimId::placement("a")).id();
    let bystander = world.spawn(SimId::placement("untouched")).id();
    let baseline = baseline_of(&mut world);

    // Retire the old body and rebuild the identity through the plan.
    world.despawn(original);
    let plan = ConstructionPlan::prepare(scope(), vec![request("a")], &nothing_live(), &registry())
        .unwrap();
    let receipt = commit_into(&mut world, &plan, &services);
    let rebuilt = receipt.entity(&SimId::placement("a")).expect("committed");
    assert_ne!(rebuilt, original, "reconstruction must mint a new body");

    let declared = baseline.clone().reconstructing([SimId::placement("a")]);
    assert_eq!(
        verify_world(&mut world, &plan, &receipt, &declared),
        Ok(()),
        "a declared reconstruction with one new generation is clean"
    );
    assert!(
        world.get_entity(bystander).is_ok(),
        "the bystander's generation must be untouched"
    );

    // Without the declaration, the very same world is a violation: the plan
    // naming the identity is NOT permission to have destroyed what held it.
    let violations = verify_world(&mut world, &plan, &receipt, &baseline)
        .expect_err("an undeclared reconstruction must be rejected");
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, RosterViolation::PlannedOverBaseline { .. })),
        "got {violations:?}"
    );
}

/// A declared reconstruction that leaves the OLD body alive is still wrong:
/// two generations of one identity, and dependants may hold either.
#[test]
fn a_reconstruction_that_leaves_the_old_body_alive_is_detected() {
    let services = Services::default();
    let mut world = World::new();
    let original = world.spawn(SimId::placement("a")).id();
    let baseline = baseline_of(&mut world).reconstructing([SimId::placement("a")]);

    // Note: `original` is deliberately NOT despawned.
    let plan = ConstructionPlan::prepare(scope(), vec![request("a")], &nothing_live(), &registry())
        .unwrap();
    let receipt = commit_into(&mut world, &plan, &services);

    let violations = verify_world(&mut world, &plan, &receipt, &baseline)
        .expect_err("the stale generation must be detected");
    assert!(
        violations.iter().any(|v| matches!(
            v,
            RosterViolation::ReconstructedOldSurvived { stale, .. } if *stale == original
        )),
        "got {violations:?}"
    );
}

/// Identities that were already live and are untouched are not "unplanned" —
/// that is what the baseline is for.
#[test]
fn a_baseline_identity_is_not_reported_as_unplanned() {
    let mut world = world_with(&[("survivor", Entity::PLACEHOLDER)]);
    let baseline = baseline_of(&mut world);
    let violations = verify_world(
        &mut world,
        &empty_plan(),
        &ConstructionReceipt::default(),
        &baseline,
    );
    assert_eq!(
        violations,
        Ok(()),
        "a pre-existing, untouched identity is not a finding"
    );
}

/// A declared retirement that did not happen.
#[test]
fn an_identity_declared_retired_that_survived_is_detected() {
    let mut world = world_with(&[("doomed", Entity::PLACEHOLDER)]);
    let baseline = baseline_of(&mut world).retiring([SimId::placement("doomed")]);
    let violations = verify_world(
        &mut world,
        &empty_plan(),
        &ConstructionReceipt::default(),
        &baseline,
    )
    .expect_err("a survivor of a declared retirement must be detected");
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, RosterViolation::RetiredSurvived { .. })),
        "got {violations:?}"
    );
}

// ── Relation postconditions ──────────────────────────────────────────────────
//
// The receipt records that a wiring function was CALLED. Every case below
// produces a receipt identical to the clean one, so nothing here is detectable
// without reading the committed components.

fn related_plan() -> ConstructionPlan<Toy> {
    let mut a = request("a");
    a.relations.push(RelationRequest {
        to: SimId::placement("b"),
        relation: ToyRelation::Grudge,
    });
    ConstructionPlan::prepare(scope(), vec![a, request("b")], &nothing_live(), &registry()).unwrap()
}

#[test]
fn a_correctly_wired_relation_verifies() {
    assert_eq!(
        verify_under_both(Sabotage::None, RelationSabotage::None, &related_plan()),
        Ok(())
    );
}

/// Each of these produces a receipt that says the relation was wired.
#[test]
fn relation_wiring_defects_are_detected_against_the_committed_world() {
    for sabotage in [
        RelationSabotage::NoOp,
        RelationSabotage::WrongTarget,
        RelationSabotage::WrongSource,
        RelationSabotage::RemovedAfterWiring,
        RelationSabotage::OverwrittenByAnotherCommand,
    ] {
        let plan = related_plan();
        let violations = verify_under_both(Sabotage::None, sabotage, &plan)
            .expect_err("a relation defect must be detected");
        assert!(
            violations
                .iter()
                .any(|v| matches!(v, RosterViolation::RelationNotEstablished { .. })),
            "got {violations:?}"
        );
    }
}

/// A no-op wiring function reads as absent, and an overwrite reads as pointing
/// somewhere else. The distinction is what makes a failure diagnosable.
#[test]
fn a_relation_defect_reports_what_the_world_actually_holds() {
    let plan = related_plan();
    let violations = verify_under_both(Sabotage::None, RelationSabotage::NoOp, &plan).unwrap_err();
    assert!(
        violations.iter().any(|v| matches!(
            v,
            RosterViolation::RelationNotEstablished { check, .. }
                if *check == RelationCheck::NotInstalled
        )),
        "got {violations:?}"
    );

    let violations =
        verify_under_both(Sabotage::None, RelationSabotage::WrongTarget, &plan).unwrap_err();
    assert!(
        violations.iter().any(|v| matches!(
            v,
            RosterViolation::RelationNotEstablished { check, .. }
                if matches!(check, RelationCheck::WrongTarget { found: Some(_) })
        )),
        "got {violations:?}"
    );
}

/// A relation left pointing at the PRE-reconstruction body. The endpoints are
/// live, the identity roster is perfect, and the source holds a handle to a
/// corpse — the exact stale-handle class `RelationCutBySubset` refuses at plan
/// time, here proven detectable after the fact.
#[test]
fn a_relation_onto_a_stale_generation_is_detected() {
    let services = Services::default();
    let mut world = World::new();
    let baseline = baseline_of(&mut world);
    let plan = related_plan();
    let receipt = commit_into(&mut world, &plan, &services);

    let a = receipt.entity(&SimId::placement("a")).unwrap();
    let stale_b = receipt.entity(&SimId::placement("b")).unwrap();
    // Rebuild `b`'s body and repoint the identity, WITHOUT rewiring the grudge.
    world.despawn(stale_b);
    let fresh_b = world
        .spawn((
            SimId::placement("b"),
            SpawnOrigin::Authored {
                source: "room_a".into(),
                instance: "b".into(),
            },
            plan.scope().transaction(SessionSpawnScope::UNSCOPED),
        ))
        .id();
    assert_ne!(fresh_b, stale_b);
    assert_eq!(
        world.get::<Grudge>(a).map(|g| g.0),
        Some(stale_b),
        "the source still points at the body that died"
    );

    let violations = verify_world(&mut world, &plan, &receipt, &baseline)
        .expect_err("a grudge onto a dead generation must be detected");
    assert!(
        violations.iter().any(
            |v| matches!(v, RosterViolation::RelationNotEstablished { .. })
                || matches!(v, RosterViolation::MovedRoot { .. })
        ),
        "got {violations:?}"
    );
}

/// Rebuilding a relation's dependency closure rewires it correctly — the
/// positive counterpart to the stale-generation case above.
#[test]
fn rebuilding_a_relation_closure_rewires_it() {
    let services = Services::default();
    let mut world = World::new();
    let plan = related_plan();
    let receipt = commit_into(&mut world, &plan, &services);

    let closure = plan.relation_closure(&BTreeSet::from([SimId::placement("b")]));
    assert_eq!(
        closure,
        BTreeSet::from([SimId::placement("a"), SimId::placement("b")]),
        "the closure must pull in the source that points at `b`"
    );
    for id in &closure {
        world.despawn(receipt.entity(id).unwrap());
    }
    let baseline = baseline_of(&mut world);
    let rebuilt = {
        let scope_value = plan.scope().clone();
        let mut commands = world.commands();
        let mut ctx = ConstructionExecCtx::<Toy> {
            commands: &mut commands,
            scope: &scope_value,
            session: crate::lifecycle::SessionSpawnScope::UNSCOPED,
            services: &services,
        };
        plan.commit_subset(&closure, &mut ctx)
            .expect("the closure encloses the relation, so it cannot be refused")
    };
    world.flush();

    assert_eq!(
        verify_world(&mut world, &plan, &rebuilt, &baseline),
        Ok(()),
        "a closure rebuild wires the relation onto the new generation"
    );
}

/// A presentation-only entity that was ALREADY alive when the transaction
/// opened must not be reported as a lost baseline identity.
///
/// The baseline and the occupant count have to apply the same filter. When
/// `capture` admitted presentation-only entities and the verifier's occupant
/// count excluded them, every pre-existing visual double reported
/// `BaselineLost` on every subsequent room load — an entity looked for among
/// occupants that structurally could not contain it.
#[test]
fn a_pre_existing_presentation_only_entity_is_not_a_lost_baseline_identity() {
    let mut world = World::new();
    world.spawn((SimId::placement("hero/afterimage"), PresentationOnly));
    world.spawn(SimId::placement("hero"));
    let baseline = baseline_of(&mut world);
    assert!(
        !baseline.contains(&SimId::placement("hero/afterimage")),
        "a presentation-only entity is not part of the authoritative baseline"
    );
    assert!(baseline.contains(&SimId::placement("hero")));

    assert_eq!(
        verify_world(
            &mut world,
            &empty_plan(),
            &ConstructionReceipt::default(),
            &baseline
        ),
        Ok(()),
        "nothing changed, so nothing should be reported"
    );
}
