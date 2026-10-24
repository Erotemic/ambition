//! The construction tests that commit through a publication: a room is
//! built, verified and published or refused as one operation.

use ambition_characters::actor::limb::{Limb, LimbIntents, LimbRig, LimbRouteState, LimbSlot};
use ambition_platformer2d_shared_tangle::construction::{
    ConstructionPlan, ConstructionScope, SpawnOrigin,
};
use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::{App, Commands, Entity, Update, World};
use ambition_platformer2d_construction::*;
use ambition_platformer2d_actor_monolith::features::{
    ActorConstructionContext, RoomFeatureConstructionPlan,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::construction::{
    ConstructionReceipt, RelationCheck,
    RelationRequest, RosterViolation, TransactionBaseline,
};
use ambition_platformer2d_actor_monolith::construction_fixtures::*;

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
///
/// ⛔ It must commit on the road production takes. There is one road — the
/// candidate road — and a published room's observable end state is the same as
/// the old live road's, because publication is a marker REMOVAL. An arm that
/// drifts onto a second road therefore diverges SILENTLY.
fn commit_over(plan: RoomFeatureConstructionPlan, seed: impl FnOnce(&mut World)) -> App {
    commit_bracketed(plan, |world| {
            // ⛔ THE COMPOSITION PRODUCTION BUILDS, installed HERE rather than
            // inside `commit_bracketed`: the pair of arms that asks what happens
            // when the filter is MISSING seeds it themselves, and a harness that
            // always installed it would answer their question for them.
            ambition_platformer2d_shared_tangle::construction::register_inactive_candidate_filter(
                world,
            );
        seed(world);
    })
}

/// As [`commit_over`], WITHOUT installing the disabling filter — the one
/// difference between the two, and the subject of the arms that ask what a world
/// unable to hide a candidate does.
fn commit_bracketed(plan: RoomFeatureConstructionPlan, seed: impl FnOnce(&mut World)) -> App {
    let mut app = App::new();
    app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
    seed(app.world_mut());
    app.add_systems(Update, move |mut commands: Commands| {
        // ⛔ THE SAME DECLARATION `spawn_contents` MAKES. A harness that opened
        // an undeclared baseline would verify these rooms against a claim the
        // production road no longer makes, and would go on passing after the
        // production road's declaration broke.
        let publication = crate::transaction::begin_publication(
            &mut commands,
            plan.room().id.clone(),
            plan.construction_transactions(SessionSpawnScope::UNSCOPED),
            crate::transaction::PublicationRetention::UntilOwnerRetires,
        );
        crate::transaction::open(
            &mut commands,
            publication,
            &plan,
            SessionSpawnScope::UNSCOPED,
        );
        // ⛔ THE PRODUCTION CONSTRUCTION BOUNDARY, NOT A COPY OF IT. Calling
        // `spawn_room_feature_entities_from_plan` directly here would leave every
        // arm below certifying a road production does not take.
        crate::stage::construct_room_candidate(
            &mut commands,
            publication,
            &plan,
            SessionSpawnScope::UNSCOPED,
            None,
            None,
            crate::CommitFactsSource::TheWorldAtTheCommit,
        );
        crate::transaction::close(
            &mut commands,
            publication,
            &plan,
            SessionSpawnScope::UNSCOPED,
        );
    });
    app.update();
    app
}

// ── All three origins, one planner ───────────────────────────────────────────

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
            &ambition_combat::components::ActorIdentity,
            &ambition_combat::components::ActorAggression,
        )>()
        .iter(app.world())
        .filter(|(identity, _)| identity.id.starts_with("duel_"))
        .map(|(_, aggression)| aggression.grudge.is_some())
        .collect();
    assert_eq!(grudges.len(), 2, "both duellists spawned");
    assert!(
        grudges.iter().all(|has| *has),
        "each duellist holds a grudge against the other"
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
        .query::<&ambition_held_items::GroundItem>()
        .iter(app.world())
        .count();
    assert_eq!(items, 1, "the authored pickup reached the world");
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
    room.boss_spawns
        .push(ambition_platformer2d_world::rooms::Authored::new(
            "warden",
            "clockwork_warden",
            ae::Aabb::new(ae::Vec2::new(100.0, 20.0), ae::Vec2::splat(30.0)),
            ambition_entity_catalog::placements::BossBrain::PhaseScript {
                script_id: "clockwork_warden".into(),
            },
        ));
    let plan = prepare(
        &room,
        &ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry::default(),
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
                pos: ae::Vec2::ZERO,
                half_size: ae::Vec2::splat(8.0),
                character_id: "puppy_slug".into(),
                encounter_id: "enc".into(),
                faction: ambition_characters::actor::ActorFaction::Enemy,
                calling: ambition_platformer2d_construction::SummonCalling::Fight,
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
        ConstructionScope::in_generation(ambition_platformer2d_shared_tangle::construction::ContentBinding::RuntimeDynamic, None),
        requests,
        &live,
        &recipes,
    )
    .expect("every variant plans");

    let mut world = World::new();
    world.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    world.init_resource::<ambition_platformer2d_construction::mechanics::SessionMechanics>();
    let services = ActorConstructionServices {
        context: ambition_platformer2d_construction::placements::ActorPlacementContext::new(
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
            facts: &ambition_platformer2d_construction::PersistedFates::unrecorded(),
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

/// A summon states no name, so both of its names are its character's.
///
/// Every summoner used to restate the name beside the character id, and one
/// copy disagreed ("Slop Lurker" for the character "Ai Slop"). The fixture
/// cast's display name is the character id, and the feature id is a
/// different string, so a body named from either the request or its id
/// fails here.
#[test]
fn a_summoned_minion_takes_its_characters_name() {
    let recipes = engine_construction_registry();
    let summoner = SimId::placement("boss_1");
    let request = summoned_minion_request(
        &summoner,
        0,
        SummonedMinionParams {
            health: None,
            keeps_contact_damage: true,
            feature_id: "slop_add".into(),
            pos: ae::Vec2::ZERO,
            half_size: ae::Vec2::splat(8.0),
            character_id: "puppy_slug".into(),
            encounter_id: "enc".into(),
            faction: ambition_characters::actor::ActorFaction::Enemy,
            calling: ambition_platformer2d_construction::SummonCalling::Fight,
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

    let mut world = World::new();
    let services = ActorConstructionServices {
        context: ambition_platformer2d_construction::placements::ActorPlacementContext::new(
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &Default::default(),
        )
        .with_prepared(fixture_cast()),
        boss_catalog: ambition_boss_encounter::test_boss_catalog().clone(),
    };
    {
        let mut commands = world.commands();
        let scope = plan.scope().clone();
        let mut ctx = ambition_platformer2d_shared_tangle::construction::ConstructionExecCtx {
            commands: &mut commands,
            scope: &scope,
            session: SessionSpawnScope::UNSCOPED,
            services: &services,
            facts: &ambition_platformer2d_construction::PersistedFates::unrecorded(),
        };
        plan.commit(&mut ctx);
    }
    world.flush();

    let minion = SimId::spawned(&summoner, 0);
    let (feature_name, identity) = world
        .query::<(
            &SimId,
            &ambition_combat::components::FeatureName,
            &ambition_combat::components::ActorIdentity,
        )>()
        .iter(&world)
        .find(|(id, ..)| **id == minion)
        .map(|(_, feature_name, identity)| (feature_name.0.clone(), identity.name.clone()))
        .expect("the summon was built with both names");
    assert_eq!(identity, "puppy_slug", "the body's identity is its character's");
    assert_eq!(
        feature_name, identity,
        "the feature name is a second copy that disagrees with the character"
    );
}

// ── Summon counter preconditions ─────────────────────────────────────────────

/// ⛔⛤ **A ROOM THAT FAILS VERIFICATION IS NOT PUBLISHED — ON THE ROAD THE GAME
/// TAKES.**
///
/// Nothing test-only is wired into the construction path to produce the failure.
/// The room transaction compares the generation its plan was prepared under
/// against the session's live `ActiveContentBinding` and refuses a stale room;
/// the seed states a generation the plan was not prepared against, which is the
/// *"a live shell session lost its canonical content authority"* case that
/// comparison exists for.
///
/// ⛔ Do not seed a live `placement:duel_blue` here expecting
/// `ReconstructedOldSurvived`: that is not a failure on this road. A planned
/// identity the baseline still holds is declared a SUPERSESSION, not a
/// reconstruction — the coexistence is A10's premise, the projection removes the
/// predecessor, and the room publishes. A planned id with a live body is never
/// declared `reconstructing` in the first place. Its meaning is pinned at the
/// verifier that owns it
/// (`a_reconstruction_that_leaves_the_old_body_alive_is_detected`), rather than
/// by an arm here certifying a road production no longer uses.
#[test]
fn a_room_that_fails_verification_is_not_published() {
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &engine_construction_registry())
        .expect("the room plans: the defect is in the world, not in the plan");
    let mut app = commit_over(plan, |world| {
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            world,
            crate::ActiveContentBinding::content(ambition_platformer2d_core::ContentEpoch(7), Default::default()));
    });

    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>()
        .clone();
    assert!(
        !verification.published,
        "a room that failed verification must not publish: {verification:?}"
    );
    assert!(
        verification.violations.iter().any(|violation| matches!(
            violation,
            ambition_platformer2d_shared_tangle::construction::RosterViolation::ContentBindingMismatch {
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
        .resource::<crate::LastConstructionVerification>()
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
            facts: &ambition_platformer2d_construction::PersistedFates::unrecorded(),
        };
        plan.commit(&mut ctx)
    };
    world.flush();
    (world, receipt, baseline)
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
/// `attach_mount_role_from` never inserts `MountSlot`, and
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
        seat: ae::Vec2::ZERO,
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

/// The giant rows commit into a correctly wired rig, and the boundary verifier
/// sees no violation — no legacy warning, because the hands are owned rows now.
#[test]
fn a_committed_giant_has_a_verified_two_hand_rig() {
    let requests =
        ambition_platformer2d_construction::authored_actor_requests(&giant_room(), &[], Some(fixture_cast()));
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

/// End to end: a provider STAGES a giant, the room commits, the boundary
/// verifier publishes, and the world holds a fully wired two-hand rig.
#[test]
fn a_staged_giant_commits_into_a_published_room_with_a_wired_rig() {
    let room = empty_room("hall");
    let mut staging = ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry::default();
    staging
        .register("hall", "test_provider", "boss", "boss.v1", |_room| {
            vec![staged_giant("gnu")]
        })
        .expect("stager registers");
    let plan = prepare(&room, &staging, &engine_construction_registry()).expect("the room plans");
    let mut app = commit(plan);

    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>();
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

fn committed_giant() -> (
    ActorConstructionPlan,
    World,
    ConstructionReceipt,
    TransactionBaseline,
) {
    let requests =
        ambition_platformer2d_construction::authored_actor_requests(&giant_room(), &[], Some(fixture_cast()));
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
            &ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry::default(),
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
            plan.respawn_authoritative_sim_id(&mut commands, SessionSpawnScope::UNSCOPED, &ambition_platformer2d_construction::PersistedFates::unrecorded(), seed)
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

/// End to end: the pair commits, the room publishes, and BOTH ends of the weld
/// are live at the boundary — no frame-later resolution.
#[test]
fn a_committed_mount_pair_is_welded_both_ways_and_published() {
    let plan = prepare(
        &mounted_pair_room(),
        &ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the mounted room plans");
    let mut app = commit(plan);

    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>();
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
    room.boss_spawns
        .push(ambition_platformer2d_world::rooms::Authored::new(
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
        &ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry::default(),
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
        .resource::<crate::LastConstructionVerification>();
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

/// The migrated actors are VISIBLE to the boundary verifier: identity, provenance, and transaction
/// ownership are stamped at construction, not assigned by `ensure_sim_id` after verification has
/// already run.
#[test]
fn a_committed_actor_room_is_fully_visible_at_the_boundary() {
    let mut room = empty_room("field");
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
        &ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the room plans");
    let mut app = commit(plan);

    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>();
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
    room.boss_spawns
        .push(ambition_platformer2d_world::rooms::Authored::new(
            "warden",
            "clockwork warden",
            ae::Aabb::new(ae::Vec2::new(400.0, 100.0), ae::Vec2::splat(40.0)),
            ambition_entity_catalog::placements::BossBrain::Dormant,
        ));
    let plan = prepare(
        &room,
        &ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry::default(),
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
        plan.respawn_authoritative_entity(&mut commands, SessionSpawnScope::UNSCOPED, &ambition_platformer2d_construction::PersistedFates::unrecorded(), "warden")
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

/// The committed placement is stamped and verified at the boundary like every
/// plan row — the placement family leaves the invisible list.
#[test]
fn a_committed_placement_room_publishes_with_a_stamped_pickup() {
    let plan = prepare_with_placements(&placement_room()).expect("the gallery plans");
    let mut app = commit(plan);
    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>();
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

/// A placement reconstructs through the planner — the family-specific respawn
/// branches are gone, and so, as of 2026-09-05, is the `lower_one` fallback
/// itself: this comment was the only thing in the workspace that still named it.
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
        plan.respawn_authoritative_entity(&mut commands, SessionSpawnScope::UNSCOPED, &ambition_platformer2d_construction::PersistedFates::unrecorded(), "ring_1")
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
    room.shrines
        .push(ambition_platformer2d_world::rooms::ShrineSpec {
            id: "rest_1".into(),
            name: "Rest".into(),
            pos: ae::Vec2::new(64.0, 32.0),
            half_extent: ae::Vec2::splat(16.0),
        });
    room.gravity_zones
        .push(ambition_platformer2d_world::rooms::GravityZoneSpec {
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
        &ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry::default(),
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
        .resource::<crate::LastConstructionVerification>();
    assert!(verification.published, "{:?}", verification.violations);
    assert_eq!(verification.violations, Vec::new());

    let world = app.world_mut();
    let mut shrines = world.query::<(&SimId, &ambition_shrine::HealShrine)>();
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
        &ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the room plans (epoch 4)");
    let app = commit_over(plan, |world| {
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            world,
            crate::ActiveContentBinding::content(ae::ContentEpoch(9), Default::default()));
    });
    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>();
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
        &ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry::default(),
        &engine_construction_registry(),
    )
    .expect("the room plans (epoch 4)");
    let app = commit_over(plan, |world| {
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            world,
            crate::ActiveContentBinding::content(ae::ContentEpoch(4), Default::default()));
    });
    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>();
    assert!(verification.published, "{:?}", verification.violations);
}

/// A staged actor that names a CHARACTER takes that character's label and art
/// identity — its request's own `name` reaches neither.
///
/// ⭐ MEASURED, and it corrects a comment rather than confirming one.
/// `game/ambition_content/src/duel_arena.rs` asked its two fighters to keep
/// `name` in sync with the character catalog's `display_name` under a "MUST",
/// on the stated ground that the sprite sheet and the authored hitbox metadata
/// both resolve from it. Neither half survived measurement:
///
/// * `new_character_in` destructures `display_name` out of the
///   `CharacterBodyBlueprint` — the CHARACTER's answer. The request's `name`
///   is never passed to that road at all, so it cannot be the label. (Only the
///   peaceful road, which takes no character, forwards a request's `name`.)
/// * the sheet binder reads the worn character
///   (`an_actor_binds_the_sheet_of_its_character_id_not_its_display_name`), and
///   `authored_attack_volume_resolver` branches on that id with NO name road —
///   its `None` arm falls to the PLAYER's hitbox, not to a name lookup.
///
/// ⛔ THE CATALOG HERE IS EMPTY, which is what makes the answer attributable: an
/// empty catalog can join no display name, so an id that still arrives was
/// STATED. The request authors `name: "test_walker"` and that string appears
/// nowhere below.
#[test]
fn a_staged_actor_naming_a_character_takes_the_characters_label_not_its_requests() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");
    let mut app = commit(plan);

    let rows: Vec<(String, Option<String>)> = app
        .world_mut()
        .query::<(
            &ambition_combat::components::ActorIdentity,
            Option<&ambition_characters::actor::WornCharacter>,
        )>()
        .iter(app.world())
        .filter(|(identity, _)| identity.id.starts_with("duel_"))
        .map(|(identity, worn)| (identity.name.clone(), worn.map(|w| w.id().to_string())))
        .collect();

    assert_eq!(rows.len(), 2, "both staged fighters spawned");
    for (name, sprite_character_id) in rows {
        assert_ne!(
            name, "test_walker",
            "the request's own `name` reached the body; a caller keeping that \
             string in sync with the character catalog would be maintaining a \
             field the character road drops"
        );
        assert_eq!(name, "medium_striker", "the character supplies the label");
        assert_eq!(
            sprite_character_id.as_deref(),
            Some("medium_striker"),
            "the art identity is STATED by the request's character, not joined \
             from a display name — this catalog is empty and could join nothing"
        );
    }
}

/// ⛔⛤ **A ROOM BUILT AS A CANDIDATE IN A WORLD THAT CANNOT HIDE ONE IS REFUSED
/// — AND UNTIL 2026-09-13 NOTHING ANYWHERE ASKED.**
///
/// `InactiveCandidate` only hides an entity because
/// `register_inactive_candidate_filter` made it a bevy DISABLING component.
/// Without that call the marker is an ordinary inert component: every root the
/// bracket stamps is stamped, and every one of them is LIVE — a half-built room
/// participating in the running world while it is being validated, which is the
/// exact outcome the bracket exists to prevent, and it is SILENT.
///
/// ⛔ `ConstructionPlan::commit_inactive` refuses on exactly this, and its doc
/// said the deferred road's caller *"owes that check"*. MEASURED at HEAD:
/// `inactive_candidate_filter_installed` had ONE caller — inside
/// `commit_inactive` itself — and `commit_inactive` had ZERO production callers.
/// ⇒ **The check lived only on the road with no traffic. Nobody owed it.**
///
/// ⇒ The room transaction opens with `&mut World` and asks there.
#[test]
fn a_candidate_room_is_refused_by_a_world_that_cannot_hide_a_candidate() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");

    let mut app = commit_bracketed(plan, |_| {});
    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>();
    assert!(
        !verification.published,
        "a room was published as a CANDIDATE into a world where `InactiveCandidate` \
         is not a disabling component, so every root it called invisible was live \
         the whole time"
    );
    // ⛔⛤ **`!published` IS TOO WEAK ON ITS OWN — 2026-09-15 AUDIT, FINDING 4.**
    // `spawn_contents_for` queues `open`, then the candidate construction
    // commands, then `close`. A refusal DISCOVERED INSIDE `open` cannot unqueue
    // the commands behind it, so by the time the verdict is recorded the roots
    // EXIST — and in this exact case they are not hidden at all, because the
    // whole refusal is that this world cannot hide them. A verdict of `false`
    // over a world full of live debris is not a refusal, it is a report.
    assert_eq!(
        ambition_platformer2d_shared_tangle::construction::outstanding_candidates(
            app.world_mut()
        ),
        0,
        "⛔ A REFUSED CANDIDATE ROOM LEFT ITS ROOTS STANDING. The transaction \
         refused before it could hide anything, and every root it had already \
         queued is in the world — VISIBLE, because the marker is inert here"
    );
    let stamped = {
        let world = app.world_mut();
        let mut query = world
            .query::<&ambition_platformer2d_shared_tangle::construction::TransactionId>();
        query.iter(world).count()
    };
    assert_eq!(
        stamped, 0,
        "⛔ CANDIDATE TRANSACTION STATE OUTLIVED ITS REFUSAL: {stamped} \
         entit(ies) still carry a `TransactionId` for a room that was refused, \
         so a later transaction gathering by scope can still find them"
    );
}

/// ⛔⛤ **A WORLD THAT CANNOT HIDE A CANDIDATE CONSTRUCTS NONE — 2026-09-15
/// AUDIT, FINDING 4, AND THIS IS THE ASSERTION THE CLEANUP ARM CANNOT MAKE.**
///
/// `a_candidate_room_is_refused_by_a_world_that_cannot_hide_a_candidate` proves
/// that after the flush nothing is left standing. It cannot prove that nothing
/// was ever BUILT — and the difference is not academic here, because
/// `commit_inactive`'s own note records that component hooks and lifecycle
/// observers run during `queue.apply` even though no scheduled system does. An
/// entity that exists for the length of a command flush, unhidden, is observable
/// by exactly that mechanism.
///
/// ⭐ **SO THE INSTRUMENT IS THAT MECHANISM.** A hook on `TransactionId` — the
/// stamp every constructed root carries — counts insertions as they happen
/// rather than inspecting the settled world. Zero is the claim.
///
/// ⭐⭐ **AND THE POSITIVE CONTROL IS THE SAME PLAN IN THE SAME ARM.** A count of
/// zero from a harness that constructs nothing for an unrelated reason reads
/// identically to the fix working; the second half installs the filter and
/// requires the count to be non-zero.
#[test]
fn a_world_that_cannot_hide_a_candidate_constructs_no_candidate_at_all() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");

    *CANDIDATE_INSERTIONS.lock().expect("insertions") = 0;
    let mut refused = commit_bracketed(plan.clone(), count_candidate_insertions);
    let built_while_unhideable = *CANDIDATE_INSERTIONS.lock().expect("insertions");
    assert!(
        !refused
            .world()
            .resource::<crate::LastConstructionVerification>()
            .published,
        "the room PUBLISHED in a world that cannot hide a candidate, so this arm \
         is not about a refusal at all"
    );
    assert_eq!(
        built_while_unhideable, 0,
        "⛔ A WORLD THAT CANNOT HIDE A CANDIDATE STILL CONSTRUCTED {built_while_unhideable} \
         OF THEM. The roots existed — VISIBLY, because the refusal IS that \
         nothing here is hidden — and were removed afterwards. Component hooks \
         and lifecycle observers run during that flush, so \
         \"cleaned up later\" is not the same statement as \"never built\""
    );
    // ⚠ The refusal must still settle its own state; that is the sibling arm's
    // subject, asserted here only so a fix that constructs nothing by CRASHING
    // cannot pass.
    assert_eq!(
        ambition_platformer2d_shared_tangle::construction::outstanding_candidates(
            refused.world_mut()
        ),
        0,
        "the refused world holds candidate roots"
    );

    // ⭐⭐ THE POSITIVE CONTROL: same plan, same bracket, one difference.
    *CANDIDATE_INSERTIONS.lock().expect("insertions") = 0;
    let control = commit_bracketed(plan, |world| {
        ambition_platformer2d_shared_tangle::construction::register_inactive_candidate_filter(
            world,
        );
        count_candidate_insertions(world);
    });
    let built_when_hideable = *CANDIDATE_INSERTIONS.lock().expect("insertions");
    assert!(
        built_when_hideable > 0,
        "⛔ THE INSTRUMENT IS BLIND. The same plan, in a world that DOES hide \
         candidates, constructed nothing the hook could see — so the zero \
         asserted above is a claim about the hook rather than about the refusal"
    );
    assert!(
        control
            .world()
            .resource::<crate::LastConstructionVerification>()
            .published,
        "the control room was refused in a world that hides candidates, so the \
         refusal above is about something other than the filter"
    );
}

/// ⭐ THE CONTROL, and without it the arm above is satisfied by a harness that
/// refuses every bracketed room for some unrelated reason.
///
/// The SAME plan, the SAME bracket, one difference: the world was taught what
/// `InactiveCandidate` means.
#[test]
fn the_same_candidate_room_publishes_once_the_filter_is_installed() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");

    let app = commit_bracketed(plan, |world| {
        ambition_platformer2d_shared_tangle::construction::register_inactive_candidate_filter(
            world,
        );
    });
    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>();
    assert!(
        verification.published,
        "a bracketed room was refused in a world that DOES hide candidates, so the \
         refusal above is about something other than the filter: {:?}",
        verification.violations
    );
}

/// ⛔⛤ **A SHELL-ROUTED COMPOSITION THAT HAS LOST ITS CONTENT BINDING MUST REFUSE
/// THE ROOM, NOT PUBLISH IT — AND THE OLD CODE COULD NOT TELL THAT CASE FROM A
/// FIXTURE.**
///
/// The commit boundary compared `plan.construction_binding()` against
/// `ActiveContentBinding` *if the resource was there*, and the resource's own doc
/// called the absent case *"an honest gap, not a waiver: a fixture with no content
/// authority has nothing to be stale against."* True of a fixture. The `Option`
/// meant BOTH that and *"a live shell session missing the canonical authority it
/// owes"*, and in the second case a room publishes into a generation nobody can
/// name — the fail-open shape a 2026-09-13 review flagged.
///
/// ⭐ **THE DISCRIMINATOR IS AN EXISTING TYPE, NOT A NEW ONE.**
/// `SessionGatedSimulation` is installed only by `ambition_game_shell`'s session
/// plugin and says so: *"never inserted by direct-entry apps or headless
/// harnesses."* Two other sites already branch on it.
#[test]
fn a_shell_composition_missing_its_content_binding_refuses_the_room() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");

    let app = commit_over(plan, |world| {
        world.init_resource::<
            ambition_platformer2d_shared_tangle::lifecycle::SessionGatedSimulation,
        >();
    });
    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>();
    assert!(
        !verification.published,
        "a shell-routed composition published a room with NO `ActiveContentBinding` \
         at all, so nothing checked which generation it was publishing into"
    );
}

/// ⭐ THE CONTROL, and without it the arm above is satisfied by a boundary that
/// refuses every room with no binding — which is every fixture in this file.
///
/// The SAME plan and the SAME absent binding, differing only in whether the
/// composition claims to route gameplay through a shell session.
#[test]
fn a_direct_entry_fixture_with_no_content_binding_still_publishes() {
    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let plan = prepare(&room, &staging, &recipes).expect("the room plans");

    let app = commit(plan);
    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>();
    assert!(
        verification.published,
        "a direct-entry fixture was refused for stating no content binding, which \
         is the one composition entitled to state none: {:?}",
        verification.violations
    );
}

/// ⛔⛤ **THE ROAD A HOT RELOAD TAKES, MINUS THE DISK: A ROOM PREPARED FOR N+1
/// AGAINST A WORLD STILL RUNNING N KEEPS BOTH FACTS.**
///
/// This is where the two could re-merge. `for_room_construction` used to take
/// `content_epoch` and `active_binding` as two parameters and let the second
/// CLOBBER the first into one field — which is how a materially changed reload
/// came to stamp N+1's roots with **N**. The unit arm in `shared_tangle` proves
/// `ConstructionScope` carries the pair; this proves the pair survives the road
/// that builds one.
///
/// ⛔⛤ **AND THE TWO-PARAMETER SIGNATURE THAT ARM DESCRIBES IS GONE, 2026-09-16,
/// BECAUSE IT WENT WRONG AGAIN IN THE OTHER DIRECTION.** Once a caller could
/// state the two halves independently, three ORDINARY roads — the door
/// transition, the reset and the neighbour prefetch — filled the incoming half
/// with `content_unstated` and the boundary half correctly, so every root they
/// built named no prepared content. ⇒ A replacement now asks for two bindings
/// by the name [`ActorConstructionContext::for_content_replacement`], which is
/// this arm's road, and every other road takes `for_live_room_construction` and
/// cannot express a split at all.
///
/// ⭐ **THE TWO ASSERTIONS PULL IN OPPOSITE DIRECTIONS ON PURPOSE**: the roots'
/// transaction must follow the INCOMING generation and the boundary comparison
/// must follow the LIVE one, so a fix that simply propagated one value everywhere
/// cannot satisfy both.
#[test]
fn a_room_prepared_for_the_next_generation_still_expects_the_live_one() {
    use ambition_platformer2d_shared_tangle::construction::ContentBinding;

    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();

    let live = crate::ActiveContentBinding::content(ae::ContentEpoch(4), Default::default());
    let plan = RoomFeatureConstructionPlan::prepare(
        &room,
        &Default::default(),
        &staging,
        &ambition_boss_encounter::test_boss_catalog(),
        ActorConstructionContext::for_content_replacement(
            &recipes,
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &ambition_platformer2d_construction::mechanics::GenerationMechanics::of(replacement_mechanics(), replacement_cast()),
            // The world it is being committed into, which is still N.
            live.0,
            // The INCOMING generation — what a reload is publishing.
            ContentBinding::content_unstated(ae::ContentEpoch(5)),
            None,
            None,
        ),
    )
    .expect("the room plans");

    assert_eq!(
        plan.construction_binding(),
        ContentBinding::content_unstated(ae::ContentEpoch(4)),
        "the commit boundary would compare this plan against generation 5 while \
         the live world is still 4, and refuse the very reload introducing 5"
    );

    let session = ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED;
    let stamped = plan.construction().scope().transaction(session);
    let expected = ambition_platformer2d_shared_tangle::construction::ConstructionScope::in_generation(
        ContentBinding::content_unstated(ae::ContentEpoch(5)),
        Some(room.id.clone()),
    )
    .transaction(session);
    assert_eq!(
        stamped, expected,
        "the roots this plan mints are stamped with the generation being REPLACED \
         rather than the one they are made of — the live world ends at 5 with every \
         root claiming 4, inside canonical rollback state"
    );
}

/// ⛔⛤ **THE RACE THE TWO-BINDING CONTRACT OWES: A REPLACEMENT WHOSE WORLD MOVED
/// UNDER IT MUST REFUSE — AND THE COMPARISON MUST USE THE *EXPECTED-LIVE* HALF.**
///
/// `ConstructionScope` carries `expected_live` and `incoming` because a
/// replacement is two generations. The staleness arm above covers the case where
/// they coincide. This covers the one that made the split necessary: a plan built
/// FROM generation 5 to be committed INTO a world running 4, arriving at a world
/// that has since advanced to 9.
///
/// ⭐ **IT IS ALSO THE ARM THAT SAYS WHICH HALF THE BOUNDARY READS.** If `close`
/// compared `incoming` the violation would name 5, and a reload would be refused
/// by the very generation it is introducing — which is why the field could not
/// simply be re-pointed.
#[test]
fn a_replacement_refuses_a_world_that_moved_under_it_and_names_the_binding_it_expected() {
    use ambition_platformer2d_shared_tangle::construction::ContentBinding;

    let recipes = engine_construction_registry();
    let (room, staging) = duelling_room();
    let expected_live = crate::ActiveContentBinding::content(ae::ContentEpoch(4), Default::default());
    let replacement = |expected: &crate::ActiveContentBinding| {
        RoomFeatureConstructionPlan::prepare(
            &room,
            &Default::default(),
            &staging,
            &ambition_boss_encounter::test_boss_catalog(),
            ActorConstructionContext::for_content_replacement(
                &recipes,
                &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
                &ambition_platformer2d_construction::mechanics::GenerationMechanics::of(replacement_mechanics(), replacement_cast()),
                // … to be committed INTO a world running 4.
                expected.0,
                // built FROM generation 5 …
                ContentBinding::content_unstated(ae::ContentEpoch(5)),
                None,
                None,
            ),
        )
        .expect("the room plans")
    };

    // The world moved to 9 while this replacement was in flight.
    let app = commit_over(replacement(&expected_live), |world| {
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            world,
            crate::ActiveContentBinding::content(ae::ContentEpoch(9), Default::default()));
    });
    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>()
        .clone();
    assert!(
        !verification.published,
        "a replacement published into a world that had advanced past the one its \
         preflight ran against"
    );
    let named = verification.violations.iter().find_map(|violation| match violation {
        RosterViolation::ContentBindingMismatch { planned, live } => Some((*planned, *live)),
        _ => None,
    });
    assert_eq!(
        named,
        Some((
            ContentBinding::content_unstated(ae::ContentEpoch(4)),
            ContentBinding::content_unstated(ae::ContentEpoch(9)),
        )),
        "the boundary compared the wrong half of the scope. It must name the \
         EXPECTED-LIVE generation (4) against the live one (9); naming the \
         INCOMING generation (5) would refuse every reload by the generation it \
         is introducing. Violations: {:?}",
        verification.violations
    );

    // ⭐ THE CONTROL: the SAME replacement, committed into the world it expected,
    // publishes. Staleness discriminates; it does not blanket-refuse a plan for
    // carrying two generations.
    let app = commit_over(replacement(&expected_live), |world| {
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            world,
            crate::ActiveContentBinding::content(ae::ContentEpoch(4), Default::default()));
    });
    let verification = app
        .world()
        .resource::<crate::LastConstructionVerification>();
    assert!(
        verification.published,
        "a replacement arriving at exactly the world it expected was refused, so \
         the assertion above is about refusing replacements rather than about \
         staleness: {:?}",
        verification.violations
    );
}
