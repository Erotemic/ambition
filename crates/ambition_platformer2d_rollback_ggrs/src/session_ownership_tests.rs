//! Rollback authority belongs to ONE gameplay session, and a stranger's is inert.
//!
//! ```text
//! same SessionScopeId, new rollback generation  ->  health/invalidation CARRIES
//! different SessionScopeId                      ->  FRESH rollback authority
//! ```
//!
//! These arms exist because the second half of that rule was missing, with a
//! user-visible symptom: quit a Smash match to the title, start Ambition, and
//! every door refused. `shell_host_lifecycle::a_smash_session_does_not_take_
//! ambitions_doors_with_it` is the acceptance walk through the real host; this
//! file is the mechanism underneath it, where a poison can be applied exactly
//! and the world is small enough to say what is being measured.
//!
//! ⛔⛔ NOTHING HERE RESETS THE PREVIOUS SESSION'S STATE BEFORE THE NEXT ONE
//! STARTS. That is the whole point: cleanup is hygiene, and ownership is what
//! makes a survivor harmless. Several arms deliberately leave scope A's state
//! allocated and prove B is unaffected anyway.

use bevy::prelude::*;

use ambition_platformer2d_runtime::rollback::RollbackRegistry;
use ambition_platformer2d_runtime::{
    ActiveRollbackAuthority, ContentEpoch, ContentFingerprint, ContentFingerprintSchemaVersion,
    PreparedContentIdentity, RollbackConfirmationState, RollbackDiagnosticHistory,
    SnapshotSchemaFingerprint,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    live_session_scope, ActiveSessionScope, SessionGatedSimulation, SessionRoot, SessionScopeId,
    SessionScopeRetired, SessionScopedEntity,
};

use crate::session::{
    enforce_session_contract, session_is_active, start_sync_test_session, SyncTestSettings,
};

const A: SessionScopeId = SessionScopeId(0);
const B: SessionScopeId = SessionScopeId(1);

fn identity(epoch: u64) -> PreparedContentIdentity {
    PreparedContentIdentity {
        fingerprint_schema: ContentFingerprintSchemaVersion::CURRENT,
        fingerprint: ContentFingerprint::from_bytes([1u8; 32]),
        snapshot_schema: SnapshotSchemaFingerprint::from_bytes([2u8; 32]),
        epoch: ContentEpoch(epoch),
    }
}

/// A world shaped like the shell host: session-gated, with an activation scope.
fn shell_shaped_world() -> World {
    let mut world = World::new();
    world.init_resource::<RollbackRegistry>();
    world.init_resource::<RollbackDiagnosticHistory>();
    // Composing the bridge IS the declaration that gameplay belongs to
    // shell-routed sessions; without it a root is readable regardless of scope,
    // which is the direct-entry contract and a different fixture.
    world.init_resource::<SessionGatedSimulation>();
    world.init_resource::<ActiveSessionScope>();
    world
}

/// Activate `scope`: mint it current and publish its canonical root.
fn activate(world: &mut World, scope: SessionScopeId, epoch: u64) -> Entity {
    let minted = world.resource_mut::<ActiveSessionScope>().begin();
    assert_eq!(minted, scope, "the fixture's scopes are minted in order");
    world
        .spawn((
            Name::new(format!("session world {}", scope.0)),
            SessionRoot(scope),
            SessionScopedEntity(scope),
            identity(epoch),
        ))
        .id()
}

/// Retire `scope` the way the shell does: clear the pointer, then despawn what
/// it owned.
fn retire(world: &mut World, scope: SessionScopeId) {
    world
        .resource_mut::<ActiveSessionScope>()
        .clear_if_current(scope);
    let doomed: Vec<Entity> = world
        .query::<(Entity, &SessionScopedEntity)>()
        .iter(world)
        .filter(|(_, owner)| owner.0 == scope)
        .map(|(entity, _)| entity)
        .collect();
    for entity in doomed {
        world.despawn(entity);
    }
}

fn confirmation(world: &World) -> RollbackConfirmationState {
    world
        .get_resource::<ActiveRollbackAuthority>()
        .map(|authority| authority.confirmation_for(live_session_scope(world)))
        .unwrap_or(RollbackConfirmationState::Unavailable)
}

fn install(world: &mut World) {
    start_sync_test_session(world, SyncTestSettings::for_players(1)).expect("session starts");
}

/// ⭐⭐ TEST 2 — A POISONED SESSION A CANNOT REACH SESSION B.
///
/// Nothing between the two activations resets anything. B is healthy because
/// the authority A poisoned does not name B, not because somebody remembered to
/// clear it.
#[test]
fn a_poisoned_session_hands_nothing_to_the_next_one() {
    let mut world = shell_shaped_world();
    activate(&mut world, A, 1);
    install(&mut world);
    assert_eq!(confirmation(&world), RollbackConfirmationState::Healthy);

    // Poison A through the ordinary invalidation path: its content vanishes
    // while its timeline is still live and still the active scope.
    let root = live_session_scope(&world).expect("A is live");
    assert_eq!(root, A);
    let a_root = world
        .query_filtered::<Entity, With<SessionRoot>>()
        .iter(&world)
        .next()
        .expect("A owns a root");
    world.entity_mut(a_root).remove::<PreparedContentIdentity>();
    enforce_session_contract(&mut world);
    assert_eq!(
        confirmation(&world),
        RollbackConfirmationState::Unhealthy,
        "A must actually be poisoned, or this test proves nothing about B"
    );

    // ── retire A, activate B. No reset in between, deliberately. ───────
    retire(&mut world, A);
    activate(&mut world, B, 2);
    install(&mut world);

    assert_ne!(A, B);
    assert_eq!(
        world.resource::<ActiveRollbackAuthority>().owner(),
        Some(B),
        "the authority B reads is B's own"
    );
    assert_eq!(
        world
            .resource::<ActiveRollbackAuthority>()
            .status()
            .invalidation,
        None,
        "nothing A's timeline discovered is a fact about B's world"
    );
    assert_eq!(confirmation(&world), RollbackConfirmationState::Healthy);
}

/// ⭐⭐ THE SAME PROOF WITH A'S AUTHORITY LEFT DELIBERATELY ALLOCATED.
///
/// The strongest form: A's poisoned authority is still the resource in the
/// world when B goes looking, and B is unaffected because the owner does not
/// match. This is what proves cleanup is not carrying correctness.
#[test]
fn a_stale_authority_left_allocated_is_inert_for_the_next_session() {
    let mut world = shell_shaped_world();
    activate(&mut world, A, 1);
    install(&mut world);
    world
        .resource_mut::<ActiveRollbackAuthority>()
        .invalidate("smash desynced".to_owned());

    // Retire A's WORLD but leave its authority resource exactly where it is —
    // as if the retirement sweep had been skipped entirely.
    retire(&mut world, A);
    assert_eq!(
        world.resource::<ActiveRollbackAuthority>().owner(),
        Some(A),
        "the fixture must still hold A's authority, or it is testing cleanup"
    );

    activate(&mut world, B, 2);
    assert_eq!(
        confirmation(&world),
        RollbackConfirmationState::Unavailable,
        "B reads a stranger's authority as ABSENT, never as its own ill health — \
         `Unavailable` resolves on the next install, `Unhealthy` never resolves"
    );
}

/// ⭐⭐ TEST 3 — AC23 SURVIVES: same scope, a new timeline still refuses.
///
/// This must fail if somebody later makes installation return a default status.
/// It is the guard on the half of the rule the cross-session fix could most
/// easily have broken.
#[test]
fn a_rebase_without_retiring_the_scope_keeps_its_poison() {
    let mut world = shell_shaped_world();
    activate(&mut world, A, 1);
    install(&mut world);
    let first = world.resource::<ActiveRollbackAuthority>().generation();
    world
        .resource_mut::<ActiveRollbackAuthority>()
        .invalidate("checksum diverged at frame 12".to_owned());

    // Restart the timeline WITHOUT retiring the gameplay session.
    install(&mut world);

    let authority = world.resource::<ActiveRollbackAuthority>();
    assert_ne!(authority.generation(), first, "a rebase is a new timeline");
    assert_eq!(
        authority.owner(),
        Some(A),
        "the same gameplay session owns it"
    );
    assert_eq!(
        authority.status().invalidation.as_deref(),
        Some("checksum diverged at frame 12"),
        "a desync must not launder itself by restarting GGRS"
    );
    assert_eq!(confirmation(&world), RollbackConfirmationState::Unhealthy);
}

/// ⭐⭐ TEST 4 — DELIBERATE RETIREMENT IS NOT A CONTENT DISAPPEARANCE.
///
/// The exact ordering edge that produced the bug: the local-session owner has
/// already decided this frame, so retirement removes the canonical root with the
/// GGRS session still installed. The contract check runs next, and must read
/// teardown as teardown.
#[test]
fn retiring_a_scope_is_teardown_not_corruption() {
    let mut world = shell_shaped_world();
    activate(&mut world, A, 1);
    install(&mut world);

    retire(&mut world, A);
    assert!(
        session_is_active(&world),
        "the fixture must reach the contract check with the timeline STILL \
         INSTALLED, which is the misordering that made this reachable"
    );

    enforce_session_contract(&mut world);

    assert!(!session_is_active(&world), "the timeline stood down");
    assert!(
        world.get_resource::<ActiveRollbackAuthority>().is_none(),
        "a retired scope's authority is gone, not left ownerless and unhealthy"
    );
    assert!(
        world.resource::<RollbackDiagnosticHistory>().is_empty(),
        "and nothing was recorded as a failure, because nothing failed"
    );
}

/// The same edge through the EXPLICIT signal rather than the polling fallback.
///
/// ⚠ THE ORDERING HERE IS HYGIENE AND THIS TEST SAYS ONLY WHAT IT PROVES: the
/// authority is gone by the end of the update the message arrived in, which is
/// before any `PreUpdate` contract check can observe it. Moving the system out
/// of `SessionScopeSet::RetireAuthority` and into `Cleanup` leaves this GREEN,
/// measured — the retirement reads the MESSAGE, not the world, so its position
/// within the update does not change the outcome. What actually holds when
/// scheduling regresses is the ownership check in `enforce_session_contract`,
/// and `retiring_a_scope_is_teardown_not_corruption` above is the arm that goes
/// red when it is removed.
#[test]
fn the_retirement_message_stands_the_authority_down_within_its_own_update() {
    let mut app = App::new();
    app.add_plugins(ambition_platformer2d_shared_tangle::lifecycle::SessionScopePlugin);
    app.add_systems(
        Update,
        crate::session::retire_rollback_authority_with_its_scope.in_set(
            ambition_platformer2d_shared_tangle::lifecycle::SessionScopeSet::RetireAuthority,
        ),
    );
    let world = app.world_mut();
    world.init_resource::<RollbackRegistry>();
    world.init_resource::<RollbackDiagnosticHistory>();
    world.init_resource::<SessionGatedSimulation>();
    activate(world, A, 1);
    install(world);
    assert!(session_is_active(app.world()));

    app.world_mut().write_message(SessionScopeRetired(A));
    app.update();

    assert!(
        !session_is_active(app.world()),
        "the authority governing a retired scope does not outlive the message"
    );
    assert!(app
        .world()
        .get_resource::<ActiveRollbackAuthority>()
        .is_none());
    let world = app.world_mut();
    assert_eq!(
        world.query::<&SessionRoot>().iter(world).count(),
        0,
        "and the world it governed was cleaned up in the same update"
    );
}

/// ⭐⭐ TEST 5 — A STALE ROOT CANNOT SATISFY ANOTHER SCOPE'S CONTRACT.
///
/// Both roots are present at once, which a delayed despawn can genuinely
/// produce. A contract that took the first `PreparedContentIdentity` it found
/// would resolve whichever the archetype happened to yield.
#[test]
fn a_contract_resolves_only_the_root_its_own_scope_owns() {
    let mut world = shell_shaped_world();
    activate(&mut world, A, 1);
    // B activates while A's root is still being despawned.
    activate(&mut world, B, 2);
    install(&mut world);

    let authority = world.resource::<ActiveRollbackAuthority>();
    assert_eq!(authority.owner(), Some(B), "the live scope is B");
    assert_eq!(
        authority.contract().content,
        Some(identity(2)),
        "B's contract binds B's content epoch, not the stale root's"
    );

    // And A's lingering root cannot keep B's contract satisfied once B's own
    // content goes: that is a real violation and must still be caught.
    let b_root = world
        .query::<(Entity, &SessionRoot)>()
        .iter(&world)
        .find(|(_, root)| root.0 == B)
        .map(|(entity, _)| entity)
        .expect("B owns a root");
    world.entity_mut(b_root).remove::<PreparedContentIdentity>();
    enforce_session_contract(&mut world);
    assert_eq!(
        confirmation(&world),
        RollbackConfirmationState::Unhealthy,
        "the valuable contract check is NOT weakened: content vanishing from the \
         LIVE session's own root is still a violation"
    );
    assert_eq!(
        world.resource::<RollbackDiagnosticHistory>().len(),
        1,
        "and it is remembered where it authorizes nothing"
    );
}

/// A direct-entry app has no shell scope pointer, and its root is still its
/// owner. One ownership rule covers both hosts.
#[test]
fn an_ungated_direct_entry_world_owns_its_own_root() {
    let mut world = World::new();
    world.init_resource::<RollbackRegistry>();
    world.init_resource::<RollbackDiagnosticHistory>();
    world.spawn((SessionRoot(A), identity(1)));

    install(&mut world);

    assert_eq!(
        world.resource::<ActiveRollbackAuthority>().owner(),
        Some(A),
        "no ActiveSessionScope resource, and the timeline still names an owner"
    );
    assert_eq!(confirmation(&world), RollbackConfirmationState::Healthy);
}

/// A timeline installed before its world adopts the first one it sees.
///
/// `warn_if_no_world_to_rewind` documents this fixture: rebase frame zero, then
/// build. Adoption is the ownership sibling of the contract's content adoption,
/// and it happens once — an authority that already names an owner never
/// re-adopts, because that would be a different session.
#[test]
fn a_timeline_installed_before_its_world_adopts_the_first_one() {
    let mut world = World::new();
    world.init_resource::<RollbackRegistry>();
    world.init_resource::<RollbackDiagnosticHistory>();

    install(&mut world);
    assert_eq!(world.resource::<ActiveRollbackAuthority>().owner(), None);

    world.spawn((SessionRoot(A), identity(1)));
    enforce_session_contract(&mut world);

    assert!(session_is_active(&world), "adoption is not a stand-down");
    let authority = world.resource::<ActiveRollbackAuthority>();
    assert_eq!(authority.owner(), Some(A));
    assert_eq!(authority.contract().content, Some(identity(1)));
}
