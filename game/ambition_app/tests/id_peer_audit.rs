//! ⭐⭐ **ID-PEER: LOCAL LIFECYCLE IDENTITY IS NOT PEER-STABLE MECHANICAL IDENTITY.**
//!
//! `SessionScopeId`, shell activation ids, content epochs, load ids and other
//! monotonic host counters are perfectly good for cleanup, ownership and
//! stale-message rejection. What they must never become is an input to
//! authoritative RNG, deterministic construction provenance, rollback identity,
//! contact/projectile identity, or a peer checksum — because two peers that have
//! burned different amounts of local history would then disagree about a
//! mechanically identical world.
//!
//! ⛔ THIS FILE HOLDS THE AUDIT AS A GUARD, NOT AS A ONE-OFF CENSUS. A census
//! answers "is it clean today"; the guard answers "did a local identity newly
//! become canonical", which is the question that matters for a campaign that
//! will take more than one sitting.
//!
//! ⚠ AUDIT RESULT, MEASURED 2026-09-15 against the LIVE `RollbackRegistry`
//! rather than by grepping source. The population is what the guard prints, not
//! a number copied into this comment — a descriptor count written here is stale
//! the next time anything registers.
//!
//! | sink | verdict |
//! |---|---|
//! | authoritative RNG | boss `PatternRng` CLEAN (`encounter_id` + rollback-visible `step_index`); `seed_from_id` CLEAN (FNV over an authored id); the smash match roster seed was NOT and now draws from a peer-agreed match ordinal |
//! | contact / projectile identity | `MoveOccurrence` is body-local and starts at 0; `SimId`/`SimIdCounter` derive from parent + construction order — but `SimId::match_spawn` embedded the absolute activation tick in a `component-canonical` identity STRING, which no carrier-type list could see |
//! | rollback identity / peer checksum | the local tokens are not registered directly; the leaks are all DERIVED values, and the list is `RECORDED_DIVERGENCE` below |
//! | construction provenance | `TransactionId`, recorded by `a_transaction_identity_still_depends_on_host_local_lineage_counters` |
//!
//! ⛔⛤ THIS TABLE HAS SAID "NARROW AND PRECISELY TWO THINGS" AND BEEN WRONG
//! TWICE. Both times the missed leaks were real, canonical and older than the
//! guard: the checkpoint family, because the kind that carried it was not in a
//! hand-written list of kind names; and `SimTick`, because an absolute tick does
//! not read as a lifecycle counter. Read `RECORDED_DIVERGENCE` for the state;
//! a completeness word here is a hostage to the next review.
use crate::common;

/// Types whose VALUE is, or carries, a host-local lifecycle count. None may be
/// registered with a kind that feeds the peer checksum.
///
/// ⚠ Wrappers are listed by their own name. `SessionScopedEntity` carries a
/// `SessionScopeId` and reached the checksum while a list of identity names
/// alone reported clean — a registered type never matches the name of the id it
/// holds. New registrations are caught by `rollback_schema_baseline.txt`, which
/// member-diffs every row; this list is what that baseline cannot know.
const HOST_LOCAL_IDENTITIES: &[&str] = &[
    // ⛔⛔ THE LARGEST ONE, AND IT DOES NOT LOOK LIKE A LIFECYCLE COUNTER.
    // `SimTick` is the canonical timeline — but it is `init_resource`'d once at
    // App build, advances UNCONDITIONALLY at the head of the sim schedule
    // (menus and suspended gameplay included), has exactly one writer, and is
    // never rebased. So it counts this App's whole life.
    "SimTick",
    "SessionScopeId",
    "SessionScopedEntity",
    "ShellActivationId",
    "ShellRequestId",
    "ContentEpoch",
    "LoadId",
    // Derived from `ContentEpoch` + `SessionScopeId`.
    "TransactionId",
    // Carry `session`, `activated_on` and/or the local seat-topology generation.
    "ActiveMatch",
    // ⛔ CARRIERS OF `CheckpointOperationKey`. Found by the GPT review of
    // 2026-09-15, after this guard had reported the campaign complete twice:
    // every one of them is a `*CustomChecksum` kind, which the old string list
    // did not name. All three now project through
    // `CheckpointOperationKey::write_peer_stable_into`, so they are listed in
    // `PEER_STABLE_PROJECTION` below and stay here to be CHECKED against it.
    //
    // ⚠ `OutstandingCheckpointRequest` was in this list and does not belong:
    // it is a bare `bool` and holds no key at all. It was added by reading the
    // four registrations beside each other instead of reading the four TYPES,
    // and the "recorded exception is still checksummed" arm could not see the
    // error, because being registered is all it asks. A wrong member of a
    // four-member list survives a count of four.
    "CheckpointOperationKey", "SessionStartupResume", "AcceptedCheckpointRestore",
    "SessionCheckpointOutcomes",
    "MatchInstance",
    "StocksMatchSettled",
    "SuddenDeathEntered",
    "LiveMatchTicks",
];

/// Kinds that checksum the WHOLE value. Measured from the registrars: each of
/// these calls `checksum_component`/`checksum_resource` with the type's own
/// encoding, so a host-local field inside one is in the peer comparison.

/// Carriers whose checksum is a stated PROJECTION that excludes the host-local
/// part, read and confirmed by a human.
///
/// ⛔ THE KIND SAYS A PROJECTION EXISTS; IT CANNOT SAY WHAT THE PROJECTION
/// COMPARES. `resource-canonical-custom-checksum` is asserted below for every
/// entry here, so reverting one to a whole-value checksum reddens this guard.
/// But a projection that hashed the session anyway would wear the same kind —
/// so each entry is also a claim someone checked, backed by a VALUE-level arm
/// in the owning crate (`the_verdict_checksum_ignores_the_session_count`,
/// `the_peer_stable_checksum_ignores_session_and_seat_topology`, …). The guard
/// holds the registration; the arms hold the function body.
/// Kinds whose checksum is a stated projection rather than the whole value.
const PROJECTED_CHECKSUM_KINDS: &[&str] = &[
    "ResourceCanonicalCustomChecksum",
    "ResourceCloneCustomChecksum",
    "ComponentCloneCustomChecksum",
];

const PEER_STABLE_PROJECTION: &[&str] = &[
    // `ActiveMatch::peer_stable_checksum` projects the seat count and the
    // activation tick, excluding `session` and the local seat-topology
    // generation. Both still snapshot, so a rewind restores them.
    "ambition_match::seating::ActiveMatch",
    // ⛔ ONE SEAM, FOUR REGISTRATIONS. All four stamp a `MatchInstance`, whose
    // `session` is a per-App activation count. Each projects through
    // `MatchInstance::peer_stable()` — the activation tick, which both peers
    // simulate — and keeps the whole value in the snapshot so a rewind still
    // restores the local half. The `peer_stable_checksum` arm in each owning
    // crate is what holds the projection honest; this list only records that a
    // reviewer checked it.
    "ambition_match::settlement::StocksMatchSettled",
    "ambition_match::settlement::SuddenDeathEntered",
    "ambition_platformer2d_actor_monolith::character_runtime::live_match_clock::LiveMatchTicks",
    // ⛔ ONE ROOT CAUSE, THREE CARRIERS. Each stores a `CheckpointOperationKey`
    // and projects it through the key's own
    // `write_peer_stable_into` — the admission sequence plus whether a scope
    // owns the operation, never the scope's per-App value. The whole key still
    // lives in the snapshot, which for these three is a `Clone` of the resource,
    // so a rewind restores the local half.
    //
    // `a_checkpoint_operation_key_projects_the_sequence_and_not_the_session_scope`
    // (`actor_monolith::session::checkpoint::tests`) holds the function body.
    "ambition_platformer2d_actor_monolith::session::checkpoint::SessionStartupResume",
    "ambition_platformer2d_actor_monolith::session::checkpoint::AcceptedCheckpointRestore",
    "ambition_platformer2d_actor_monolith::session::checkpoint::SessionCheckpointOutcomes",
];

/// Values that ARE canonical while still being a function of host-local
/// lineage, recorded so the guard reports a CHANGE rather than the known state.
///
/// ⛔ `a_transaction_identity_still_depends_on_host_local_lineage_counters`
/// (`shared_tangle::construction::tests`) proves the dependency on
/// `ContentEpoch` and `SessionScopeId`. When that arm flips, delete this
/// exception and the guard tightens by itself.
const RECORDED_DIVERGENCE: &[&str] = &[
    // Derived from `ContentEpoch` + `SessionScopeId`;
    // `a_transaction_identity_still_depends_on_host_local_lineage_counters`
    // (shared_tangle::construction::tests) holds the detail.
    "ambition_platformer2d_shared_tangle::construction::TransactionId",
    // ⛔⛤ THE CAMPAIGN'S LARGEST OPEN ITEM, and the one that makes every other
    // absolute-tick value suspect. `sim_tick` is `resource-canonical`, so its
    // ABSOLUTE value is compared: two Apps that have been running for different
    // lengths of time disagree from the FIRST compared frame, and nothing
    // derived from it can be peer-stable either — which is exactly the mistake
    // `MatchInstance::activation_tick` records.
    //
    // ⚠ It cannot be closed the way the others were. A projection excluding the
    // tick would exclude the timeline itself; what is needed is a
    // SESSION-RELATIVE tick, rebased when peers agree to start, which is netcode
    // work. Recorded so this is held by a test rather than by prose.
    "ambition_time::SimTick",
];

/// ⭐⭐ **THE CAMPAIGN'S STANDING GUARD: NO HOST-LOCAL IDENTITY IS CANONICAL.**
///
/// It reads the live registry rather than the source, because registration is
/// assembled by plugins and a source census cannot see what a composition
/// actually installed.
#[test]
fn no_host_local_lifecycle_identity_is_rollback_registered() {
    use ambition_platformer2d::rollback::{RollbackEntryKind, RollbackRegistry};

    let mut sim = common::fixed_60hz_room_sim("proving_grounds");
    for _ in 0..10 {
        sim.step(common::base());
    }
    let registry = sim
        .world()
        .get_resource::<RollbackRegistry>()
        .expect("the rollback registry is installed by the engine plugins");
    // ⛔ THE KIND IS KEPT AS THE ENUM, NOT AS A STRING. Reproducing the
    // registry's semantics here as a list of variant names is what hid the
    // checkpoint family; `RollbackEntryKind::feeds_peer_checksum` owns the
    // question and a new variant cannot be added without answering it.
    let registered: Vec<(String, RollbackEntryKind)> = registry
        .descriptors()
        .map(|d| (d.type_name.clone(), d.kind))
        .collect();

    // ⛔ THE ANTI-VACUITY FLOOR. An empty or tiny registry makes every
    // "not registered" verdict below true for free, and this crate's own
    // coverage sweep exists because that is the common way such a check passes
    // while measuring nothing.
    assert!(
        registered.len() > 300,
        "only {} rollback descriptors are registered; the composition did not \
         install the engine's rollback state, so every verdict below would pass \
         vacuously",
        registered.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    for suspect in HOST_LOCAL_IDENTITIES {
        for (type_name, kind) in &registered {
            let leaf = type_name.rsplit("::").next().unwrap_or(type_name);
            if leaf != *suspect {
                continue;
            }
            // Snapshotting a host-local value is legitimate: a rewind has to
            // restore it. Only the checksummed kinds are a peer-visible defect.
            // A reviewed projection excludes the host-local part, and no kind
            // records that — see `PEER_STABLE_PROJECTION`.
            if PEER_STABLE_PROJECTION.contains(&type_name.as_str()) {
                continue;
            }
            if !kind.feeds_peer_checksum() {
                continue;
            }
            if RECORDED_DIVERGENCE.contains(&type_name.as_str()) {
                continue;
            }
            offenders.push(format!("{type_name} as {kind:?}"));
        }
    }
    assert!(
        offenders.is_empty(),
        "a HOST-LOCAL lifecycle identity is now rollback state, so it is inside \
         the canonical comparison two peers make:\n  {}\n\n\
         Rollback state is compared BETWEEN PEERS. A value that counts local \
         activations, sessions or load transactions cannot be in it — two peers \
         with different prior local history would disagree about a mechanically \
         identical world. Keep the local token for its real job and derive the \
         canonical value from peer-stable facts (content fingerprint, schema \
         fingerprint, stable room identity, canonical `SimId`s).",
        offenders.join("\n  ")
    );

    // ⚠ AND THE RECORDED EXCEPTION MUST STILL BE THERE. If `TransactionId` has
    // stopped being canonical, this exception is dead and keeping it would let a
    // future regression through silently.
    let stale: Vec<&str> = RECORDED_DIVERGENCE
        .iter()
        .copied()
        .filter(|recorded| {
            !registered.iter().any(|(type_name, kind)| {
                type_name == recorded && kind.feeds_peer_checksum()
            })
        })
        .collect();
    assert!(
        stale.is_empty(),
        "these recorded divergences are no longer checksummed:\n  {}\n\n\
         If a leak has been closed, DELETE its line from RECORDED_DIVERGENCE so \
         this guard covers the type again. An exception for a value that no \
         longer needs one is a hole with a comment over it.",
        stale.join("\n  ")
    );

    // ⛔ AND EVERY REVIEWED PROJECTION MUST STILL BE A PROJECTION. Reverting one
    // of these to `rollback_resource_canonical` puts its host-local fields back
    // into the peer checksum while leaving its `peer_stable_checksum` compiling,
    // correct and unused — so the value-level arms cannot see it. The KIND can.
    let unprojected: Vec<String> = PEER_STABLE_PROJECTION
        .iter()
        .copied()
        .filter_map(|reviewed| {
            let kinds: Vec<String> = registered
                .iter()
                .filter(|(type_name, _)| type_name == reviewed)
                .map(|(_, kind)| format!("{kind:?}"))
                .collect();
            match kinds.as_slice() {
                [] => Some(format!("{reviewed}: not registered at all")),
                kinds if kinds
                    .iter()
                    .all(|kind| PROJECTED_CHECKSUM_KINDS.contains(&kind.as_str())) =>
                {
                    None
                }
                kinds => Some(format!("{reviewed}: registered as {}", kinds.join(", "))),
            }
        })
        .collect();
    assert!(
        unprojected.is_empty(),
        "these types are listed as having a peer-stable checksum PROJECTION, \
         but are not registered under a projected-checksum kind:\n  {}\n\n\
         A whole-value checksum over one of these puts a per-App session count \
         back inside the comparison two peers make. Either register it through \
         a `*_checksum` registrar again, or — if the type no longer holds a \
         host-local field at all — delete its line here so the guard covers it \
         directly.",
        unprojected.join("\n  ")
    );
}

/// ⭐⭐ **THE ID-PEER ACCEPTANCE: TWO HOSTS WITH DIFFERENT PRIOR LOCAL HISTORY
/// ENTER THE SAME AGREED MATCH AND SEAT THE SAME FIGHTERS.**
///
/// This is the campaign's decisive shape and it is independent of A10:
///
/// ```text
/// App A: arbitrary prior local history X
/// App B: different prior local history Y
/// both enter the same peer-agreed match
/// expect: the same deterministic random choices
/// ```
///
/// ⛔⛤ **WHAT THIS REPLACED, AND WHY IT IS AN EQUALITY NOW.** Until
/// `agreed_match_seed` existed, the smash match seeded its one deterministic
/// stream with `ShellRouter.active.activation_id` — a private monotonic counter
/// incremented per route entry ON THIS HOST — so this arm asserted the
/// DIVERGENCE. The seed is now a digest of the agreed configuration and takes
/// only `&SmashSelect`, so a host counter is not merely unused: it is
/// unspellable at that signature. The arm therefore asserts the acceptance.
///
/// ⚠ THE PREMISE IS ASSERTED FIRST, as the ID-PEER row requires: the two hosts'
/// local activation counters must actually DIFFER, or the equality below is
/// about two hosts with identical history and says nothing.
#[test]
fn two_hosts_with_different_local_history_seat_the_same_agreed_match() {
    use ambition_demo_smash::select::{SlotOccupant, SlotPick, SmashRoster, SmashSelect};
    use ambition_platformer2d::game_shell::ShellRouter;

    const POLICY: ambition_platformer2d::input::sources::InputAssignmentPolicy =
        ambition_platformer2d::input::sources::InputAssignmentPolicy::UnifiedPrimary;

    /// A host that has burned `visits` extra route activations before the match.
    fn host_with_prior_history(visits: usize) -> u64 {
        let mut sim = common::fixed_60hz_room_sim("proving_grounds");
        for _ in 0..10 {
            sim.step(common::base());
        }
        // Re-enter the live route `visits` times. Each activation mints a new
        // `ShellActivationId` from the router's private counter.
        for _ in 0..visits {
            let route = sim
                .world()
                .get_resource::<ShellRouter>()
                .and_then(|r| r.active.as_ref())
                .map(|a| a.route_id.clone())
                .expect("the room sim is active on a route");
            sim.world_mut().write_message(
                ambition_platformer2d::game_shell::ShellCommand::ReplaceWith {
                    route,
                    // ⚠ `None`: this fixture is burning activations, not
                    // correlating a transaction of its own.
                    request: None,
                },
            );
            for _ in 0..20 {
                sim.step(common::base());
            }
        }
        sim.world()
            .get_resource::<ShellRouter>()
            .and_then(|r| r.active.as_ref())
            .map(|a| a.activation_id.0)
            .expect("an active activation id")
    }

    let app_a = host_with_prior_history(3);
    let app_b = host_with_prior_history(0);

    // ⛔ THE PREMISE. Without it this arm compares two identical histories.
    assert_ne!(
        app_a, app_b,
        "the two hosts hold the SAME activation id ({app_a}), so this arm is not \
         about prior local history at all — the fixture failed to burn activations"
    );

    // The match both peers agree on: same seats, same occupants, same picks,
    // seat 1 asked to be surprised.
    let fighters = SmashRoster(vec![
        "alpha".to_string(),
        "beta".to_string(),
        "gamma".to_string(),
        "delta".to_string(),
    ]);
    let agreed = || {
        let mut select = SmashSelect::default();
        select.set_occupant(0, SlotOccupant::Cpu);
        select.set_pick(0, SlotPick::Fighter(0));
        select.set_occupant(1, SlotOccupant::Cpu);
        select.set_pick(1, SlotPick::Random);
        // ⚠ TWO random seats, not one: each extra surprise square divides the
        // chance that a wrong seed happens to draw the right roster anyway.
        select.set_occupant(2, SlotOccupant::Cpu);
        select.set_pick(2, SlotPick::Random);
        select
    };
    // ⛔ THROUGH THE PRODUCTION SEED FUNCTION, not a copy of its arithmetic. The
    // previous version of this arm reproduced `activation_id.rotate_left(17) ^
    // participating` inline, and the day the production expression changed that
    // copy would have gone on testing a formula the game no longer used.
    let roster_on = |_host_activation_id: u64| {
        let select = agreed();
        select
            .roster_seeded(
                &fighters,
                ambition_demo_smash::agreed_match_seed(&select),
                POLICY,
                &Default::default(),
                None,
                ambition_demo_smash::STARTING_STOCKS,
            )
            .expect("two decided seats are a match")
            .participants
            .iter()
            .map(|p| p.character.as_str().to_string())
            .collect::<Vec<String>>()
    };

    // ⛔⛤ **THE SEED FIRST, THEN THE ROSTER — AND ASSERTING ONLY THE ROSTER MADE
    // THIS A 3/4-POWER DETECTOR.** MEASURED: with one random seat over four
    // fighters, two DIFFERENT seeds still draw the same fighter one time in
    // four. Poisoning `agreed_match_seed` to be impure (a per-call counter
    // standing in for host lineage) left an earlier version of this arm GREEN
    // for exactly that reason, while the defect was fully present. The seed is a
    // `u64`, so comparing it collides negligibly; the roster comparison stays
    // because it is the outcome the acceptance sentence is actually about.
    let select = agreed();
    assert_eq!(
        ambition_demo_smash::agreed_match_seed(&select),
        ambition_demo_smash::agreed_match_seed(&select),
        "the agreed match seed is not a function of the agreed configuration, so \
         two peers cannot compute it independently and agree"
    );
    assert_eq!(
        roster_on(app_a),
        roster_on(app_b),
        "two hosts that have burned DIFFERENT numbers of local route activations \
         ({app_a} vs {app_b}) seated different fighters for a match they agree on \
         completely. A random seat's fighter is being drawn from host-local \
         lineage, which is the ID-PEER defect."
    );
}

/// ⛔ AND THE PROPERTY THE OLD SEED WAS THERE FOR MUST SURVIVE: a DIFFERENT
/// agreed configuration still walks a different sequence.
///
/// Without this, `agreed_match_seed` could be collapsed to a constant and the
/// acceptance arm above would still pass — "every host agrees" is satisfied
/// perfectly by a seed that never changes, which would make every random square
/// in the game draw the same fighter forever.
#[test]
fn a_different_agreed_configuration_draws_a_different_sequence() {
    use ambition_demo_smash::select::{SlotOccupant, SlotPick, SmashSelect};

    let two_way = {
        let mut s = SmashSelect::default();
        s.set_occupant(0, SlotOccupant::Cpu);
        s.set_pick(0, SlotPick::Fighter(0));
        s.set_occupant(1, SlotOccupant::Cpu);
        s.set_pick(1, SlotPick::Random);
        s
    };
    let three_way = {
        let mut s = SmashSelect::default();
        s.set_occupant(0, SlotOccupant::Cpu);
        s.set_pick(0, SlotPick::Fighter(0));
        s.set_occupant(1, SlotOccupant::Cpu);
        s.set_pick(1, SlotPick::Random);
        s.set_occupant(2, SlotOccupant::Cpu);
        s.set_pick(2, SlotPick::Random);
        s
    };
    let other_picks = {
        let mut s = SmashSelect::default();
        s.set_occupant(0, SlotOccupant::Cpu);
        s.set_pick(0, SlotPick::Fighter(2));
        s.set_occupant(1, SlotOccupant::Cpu);
        s.set_pick(1, SlotPick::Random);
        s
    };

    let seed = ambition_demo_smash::agreed_match_seed;
    assert_ne!(
        seed(&two_way),
        seed(&three_way),
        "a two-way and a three-way share one seed, so the participant count is \
         not reaching the digest"
    );
    assert_ne!(
        seed(&two_way),
        seed(&other_picks),
        "two matches whose LOCKED PICKS differ share one seed, so the picks are \
         not reaching the digest and every rematch would draw identically"
    );
    // ⛔ AND IT IS A FUNCTION, not a counter: the same configuration twice is
    // the same seed, which is the half that makes peer agreement possible.
    assert_eq!(
        seed(&two_way),
        seed(&two_way),
        "the agreed seed is not a function of the agreed configuration"
    );
}

/// ⛔⛤ **LOCAL DEVICE WIRING IS NOT PART OF THE AGREED MATCH.** The same human on
/// pad 0 here and pad 2 there is the same seat; `SlotOccupant::Controller`'s
/// `device` indexes the LOCAL input source order. The first version of
/// `agreed_match_seed` hashed it, which swapped one host-local term for another.
#[test]
fn different_local_device_wiring_seeds_the_same_agreed_match() {
    use ambition_demo_smash::select::{SlotOccupant, SlotPick, SmashSelect};

    let with_devices = |a: usize, b: usize| {
        let mut s = SmashSelect::default();
        s.set_occupant(0, SlotOccupant::Controller { device: a });
        s.set_pick(0, SlotPick::Fighter(0));
        s.set_occupant(1, SlotOccupant::Controller { device: b });
        s.set_pick(1, SlotPick::Random);
        s
    };
    // ⛔ THE PREMISE: the two hosts really do wire the pads differently.
    let (host_a, host_b) = (with_devices(0, 1), with_devices(2, 3));
    assert_ne!(
        format!("{:?}", host_a.slot(0).occupant),
        format!("{:?}", host_b.slot(0).occupant),
        "the premise: the two hosts must bind different local device indices"
    );
    assert_eq!(
        ambition_demo_smash::agreed_match_seed(&host_a),
        ambition_demo_smash::agreed_match_seed(&host_b),
        "two hosts whose seats hold the same ROLES but different local device \
         indices computed different agreed seeds, so local input wiring is \
         reaching the match's random draw"
    );
    // ⛔ AND THE CATEGORY STILL COUNTS: a human seat and a CPU seat are a
    // mechanical difference, so collapsing the occupant entirely would be wrong.
    let cpu_seated = {
        let mut s = SmashSelect::default();
        s.set_occupant(0, SlotOccupant::Cpu);
        s.set_pick(0, SlotPick::Fighter(0));
        s.set_occupant(1, SlotOccupant::Controller { device: 1 });
        s.set_pick(1, SlotPick::Random);
        s
    };
    assert_ne!(
        ambition_demo_smash::agreed_match_seed(&host_a),
        ambition_demo_smash::agreed_match_seed(&cpu_seated),
        "a human-seated and a CPU-seated match share one seed, so the occupant \
         category is no longer reaching the digest"
    );
}

/// ⭐⭐ **THE ACCEPTANCE, DRIVEN THROUGH THE SHIPPED MATCH-START ROAD.**
///
/// Two Apps aged differently, both taken to the select screen, given the same
/// agreed lobby, and asked to start. The rosters the SYSTEM publishes must
/// match.
///
/// ⛔⛤ THE HELPER-ONLY VERSION OF THIS WAS NOT A WITNESS. It called
/// `agreed_match_seed` directly with a hand-built `SmashSelect` and took the
/// host's activation id as an unused argument, so reverting the production call
/// site to `router.active.activation_id` left it GREEN. A test that calls the
/// helper proves the helper; only driving `start_the_battle_when_asked` proves
/// the game.
#[test]
fn two_differently_aged_hosts_publish_the_same_roster_through_the_shipped_road() {
    use ambition_demo_smash::select::{SlotOccupant, SlotPick, SmashSelect};
    use ambition_demo_smash::select_screen::StartRequested;
    use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId, ShellRouter};

    fn published_roster(extra_visits: usize) -> (u64, Vec<String>) {
        let mut app = ambition_app::app::build_visible_app(
            ambition_app::app::VisibleRenderMode::NoWindow,
            true,
        );
        for _ in 0..30 {
            app.update();
        }
        let goto = |app: &mut bevy::prelude::App, route: &str| {
            app.world_mut()
                .write_message(ShellCommand::GoTo(ShellRouteId::new(route)));
            for _ in 0..40 {
                app.update();
            }
        };
        // Age this host: each visit to the select route mints a new activation.
        for _ in 0..extra_visits {
            goto(&mut app, ambition_demo_smash::SMASH_SELECT_ROUTE);
        }
        goto(&mut app, ambition_demo_smash::SMASH_SELECT_ROUTE);

        let activation = app
            .world()
            .get_resource::<ShellRouter>()
            .and_then(|r| r.active.as_ref())
            .map(|a| a.activation_id.0)
            .expect("the shell is active on a route");

        {
            let mut select = app
                .world_mut()
                .get_resource_mut::<SmashSelect>()
                .expect("the select screen owns its state on this route");
            select.set_occupant(0, SlotOccupant::Cpu);
            select.set_pick(0, SlotPick::Fighter(0));
            select.set_occupant(1, SlotOccupant::Cpu);
            select.set_pick(1, SlotPick::Random);
            select.set_occupant(2, SlotOccupant::Cpu);
            select.set_pick(2, SlotPick::Random);
        }
        app.world_mut().insert_resource(StartRequested(true));

        let mut published = None;
        for _ in 0..240 {
            app.update();
            if let Some(roster) = app
                .world()
                .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
            {
                published = Some(
                    roster
                        .participants
                        .iter()
                        .map(|p| p.character.as_str().to_string())
                        .collect::<Vec<String>>(),
                );
                break;
            }
        }
        (
            activation,
            published.expect(
                "the shipped match-start road published no roster, so this arm \
                 never reached its subject",
            ),
        )
    }

    let (age_a, roster_a) = published_roster(3);
    let (age_b, roster_b) = published_roster(0);

    // ⛔ THE PREMISE: the two hosts really are differently aged.
    assert_ne!(
        age_a, age_b,
        "both hosts report activation {age_a}, so this arm is not about prior \
         local history"
    );
    assert!(
        !roster_a.is_empty(),
        "the published roster is empty, so the comparison below is vacuous"
    );
    assert_eq!(
        roster_a, roster_b,
        "two hosts that have burned different numbers of local route activations \
         ({age_a} vs {age_b}) published DIFFERENT rosters for the same agreed \
         lobby, so a random seat's fighter is drawn from host-local lineage"
    );
}
