//! Engine ownership of the local GGRS session.
//!
//! A GGRS host simulates only while a session resource exists. This module owns
//! the default local session; developer verification only adjusts its policy.
//! Sessions installed by another owner are never replaced. Compositions that
//! construct and install their own session can disable autostart through
//! [`LocalSessionPolicy::autostart`].

use bevy::prelude::*;

/// Ordering point for policy writers before the local session owner runs.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LocalSessionSet {
    /// [`maintain_local_session`] runs here.
    Maintain,
}

use super::session::{AmbitionGgrsSession, SyncTestSettings};

// the declaration is the INPUT layer's — a host consumes who is playing, it
// does not define it. See `ambition_input::seating`.
use ambition_input::SessionSeatingSource;

/// Configuration read by the local session owner and writable by verification tools.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalSessionPolicy {
    /// Frames of resimulation the session verifies each tick. `0` = rollback
    /// dormant.
    pub check_distance: usize,
    /// The furthest the session may speculate ahead of confirmation.
    pub max_prediction_window: usize,
    /// Whether this owner may start a session once gameplay is active. Disable
    /// this when the composition installs its own session.
    pub autostart: bool,
}

impl Default for LocalSessionPolicy {
    fn default() -> Self {
        Self {
            check_distance: 0,
            max_prediction_window: 8,
            autostart: true,
        }
    }
}

/// What the owner currently has running, so a restart can tell "nothing yet"
/// from "the policy changed" from "somebody else's session".
#[derive(Resource, Default, Debug)]
pub struct LocalSessionOwnership {
    /// Policy used to start the live locally owned session. Ownership itself is
    /// authoritative in [`RollbackSessionOwnership`].
    started: Option<LocalSessionPolicy>,
    pub last_error: Option<String>,
}

impl LocalSessionOwnership {
    /// The policy the live session was started with. not an ownership
    /// test — ask [`locally_owned`] first.
    pub fn running_policy(&self) -> Option<LocalSessionPolicy> {
        self.started
    }

    /// Force the locally owned session to rebuild on the next frame without
    /// changing frozen seating, for example after world content is invalidated.
    pub fn release(&mut self) {
        self.started = None;
    }
}

/// May THIS host stop and rebuild the live rollback timeline?
///
/// ⭐ **OWNERSHIP, NOT LIVENESS, AND IT IS PUBLISHED BECAUSE TWO SUBSYSTEMS ASK
/// IT.** A locally maintained sync test is one this process started and may stop;
/// an `External` session belongs to peers, and a `Caller`-owned one to a match
/// activation or a harness that did not ask for a rebase. `ambition_content` had
/// its own copy of this predicate for a day, which is how `Q118`'s lease came to
/// re-ask half its own question — see [`crate::session::mechanical_mutation_boundary`].
pub fn locally_rebasable_timeline(world: &World) -> bool {
    maintained_settings(world).is_some()
}

/// Return settings only for sync-test sessions owned by the local maintainer.
/// Caller- or peer-owned sessions are not eligible for maintenance here.
fn maintained_settings(world: &World) -> Option<super::session::SyncTestSettings> {
    match world.get_resource::<super::session::RollbackSessionOwnership>() {
        Some(super::session::RollbackSessionOwnership::LocalSyncTest {
            settings,
            owner: super::session::SyncTestOwner::LocalMaintainer,
        }) => Some(*settings),
        _ => None,
    }
}

/// Keep exactly one local session alive for as long as gameplay is.
///
/// Exclusive-world because starting and stopping a GGRS session is a whole-world
/// operation (it rebases snapshot storage), which is also why this cannot be an
/// ordinary system with resource params.
/// The seat count this session installs with: the roster's when one was decided,
/// the connected devices' otherwise.
fn decided_or_device_seating(world: &mut World) -> usize {
    let decided = world
        .get_resource::<SessionSeatingSource>()
        .and_then(|source| source.channel_plan().cloned());
    let frozen = freeze_local_seating(world);
    match decided {
        Some(plan) => {
            let seats = plan.channels();
            // Record the roster's count ON the topology, so the handle count,
            // the per-seat latches and the roster all cite one number rather
            // than agreeing by coincidence.
            let order = ambition_input::LocalDeviceOrder::from_devices(
                world
                    .get_resource::<ambition_input::LocalDeviceOrder>()
                    .map(|order| order.devices().to_vec())
                    .unwrap_or_default(),
            );
            let generation = world
                .get_resource_mut::<ambition_input::LocalSeatTopology>()
                .map(|mut topology| {
                    topology.capture_for_roster(&order, plan);
                    topology.generation()
                });
            // the topology the session was frozen from, recorded ON the
            // claim. Without it "the roster decided two seats" and "the
            // session is running two handles" are two assertions that usually
            // agree; with it they are one fact anything can cite.
            if let Some(SessionSeatingSource::Decided {
                frozen_topology, ..
            }) = world
                .get_resource_mut::<SessionSeatingSource>()
                .as_deref_mut()
            {
                *frozen_topology = generation;
            }
            seats.max(1)
        }
        None => frozen.players(),
    }
}

/// ⛔⛤ **A DEVELOPER EDIT IS A PROPOSAL UNTIL THE TIMELINE'S OWNER ADMITS IT —
/// `Q120`, REBUILT 2026-09-13 AFTER THE FIRST VERSION GOT THE ORDER BACKWARDS.**
///
/// MEASURED 2026-09-12 against the real GGRS sync-test canary (save every frame,
/// rewind 4, resimulate the same inputs, compare checksums): editing
/// `ActiveMovementTuning` mid-timeline **DESYNCS**, while the same forty frames
/// with no edit stay healthy. `Q120`'s own row calls the state that produces it
/// *"NOT COHERENT"* — a mutable mechanical input outside rollback history that
/// resimulation reads at its latest value.
///
/// ⛔⛤ **THE FIRST FIX LET THE EDIT LAND AND WATCHED FOR IT AFTERWARDS, AND THE
/// SCHEDULE NEVER SUPPORTED THAT.** The editor adapter ran in the SIM schedule,
/// which under this host is `GgrsSchedule`, advanced from `PreUpdate` by
/// `RunGgrsSystems`; the watcher ran in `Update`. ⇒ The old timeline simulated —
/// and could RESIMULATE HISTORY — with the new value before anything noticed.
/// The comment claimed *"the same frame the edit was observed"*; nothing
/// established it. ⭐ An invariant asserted in a doc comment is a claim about a
/// schedule, and it has to be asked of the schedule.
///
/// ⭐⭐ **SO THE DECISION MOVED IN FRONT OF THE ADVANCE.** This runs in
/// [`MechanicalEditSet::Admit`], which the host configures
/// `.before(RunGgrsSystems)`, and the adapters that write the authoritative
/// values run after it in [`MechanicalEditSet::Publish`] — still before the
/// advance. No frame exists in which a timeline reads a value nobody admitted.
///
/// ## The three answers, and why they are these
///
/// * **No live session** → `Publish`. There is no history an edit could
///   contradict. This is every non-rollback composition, always.
/// * **A session THIS maintainer owns** → stop it, release the policy memo, then
///   `Publish`. `maintain_local_session` starts the next baseline against the
///   edited mechanics in the same frame's `Update`. This is `Q120`'s model 2,
///   chosen by precedent: the content publication road adopted exactly this
///   stop-and-release on the same day (`Q118`), reusing what the LDtk reload has
///   shipped for months.
/// * **`External` or `Caller`-owned** → `Refuse`, **and the proposal is
///   retained**. `RollbackSessionOwnership` says an external session *"must
///   never be replaced unilaterally by the local host"*, and a caller's session
///   was not started for this.
///
/// ⛔⛤ **`Refuse` MEANS THE AUTHORITATIVE VALUE DOES NOT MOVE.** That is what
/// `Q120`'s model 1 says — *"refuse mechanical live edits while a rollback
/// timeline is active"* — and the first version of this fix wrote the opposite
/// into both the code and the row: *"the edit lands and the timeline is left
/// alone"*. That sentence describes the INCOHERENT state the row exists to
/// remove, not a policy. The proposal stays pending, so the moment the session
/// ends or ownership becomes local the edit publishes on its own.
pub fn decide_mechanical_edit_admission(world: &mut World) {
    use ambition_platformer2d_core::{MechanicalEditAdmission, PendingMechanicalEdits};

    use crate::session::MechanicalMutationBoundary;

    // ⛔ ANY domain, and the decider does not care WHICH. It answers for the
    // whole batch — that is the half of the protocol that is genuinely shared.
    // Which values are in the batch is each domain's own business, and a
    // publisher drains only its own proposal.
    let pending = world
        .get_resource::<PendingMechanicalEdits>()
        .is_some_and(PendingMechanicalEdits::any_pending);
    if !pending {
        world.insert_resource(MechanicalEditAdmission::Publish);
        return;
    }
    // ⛔⛤ **THE SHARED PROJECTION, NOT A SECOND CLASSIFICATION.** This used to
    // branch on `session_is_active` + ownership and never consult HEALTH, while
    // `Q118`'s publication road refused an unhealthy authority outright — two
    // subsystems independently defining what "mechanical mutation is legal around
    // rollback" means, which had already produced one defect. See
    // `crate::session::mechanical_mutation_boundary`.
    let admission = match crate::session::mechanical_mutation_boundary(world) {
        // Nothing to protect. The next baseline — if one is ever started —
        // starts with whatever the edit leaves behind.
        MechanicalMutationBoundary::NoTimeline => MechanicalEditAdmission::Publish,
        MechanicalMutationBoundary::LocallyRebasable => {
            crate::session::stop_session(world);
            if let Some(mut state) = world.get_resource_mut::<LocalSessionOwnership>() {
                state.release();
            }
            bevy::log::info!(
                target: "ambition_platformer2d::rollback",
                "a pending mechanical edit stopped the local rollback baseline; the \
                 session owner will rebase it onto the edited mechanics"
            );
            MechanicalEditAdmission::Publish
        }
        MechanicalMutationBoundary::ForeignTimeline => MechanicalEditAdmission::Refuse,
        // ⛔⛤ **AND THIS ARM IS NEW: AN UNHEALTHY TIMELINE REFUSES TOO.** A
        // recorded divergence must not be crossed by a mechanical mutation under
        // any model either row lists. ⚠ It was SAFE before this arm existed, for
        // a reason neither feature stated — `stop_session` stands the authority
        // DOWN rather than removing it, and `ActiveRollbackAuthority::installed`
        // refuses to launder a divergence into a fresh timeline — but "safe
        // because something else happens to hold" is not a policy.
        MechanicalMutationBoundary::Unhealthy(reason) => {
            bevy::log::warn!(
                target: "ambition_platformer2d::rollback",
                "a pending mechanical edit was refused: the rollback authority has \
                 recorded a divergence ({reason}). Rebasing across it would carry \
                 the edit into a timeline that is already known to disagree."
            );
            MechanicalEditAdmission::Refuse
        }
    };
    world.insert_resource(admission);
}

pub fn maintain_local_session(world: &mut World) {
    let gameplay_active =
        ambition_platformer2d_shared_tangle::lifecycle::session_world_entity(world).is_some();
    let session_live = world.contains_resource::<AmbitionGgrsSession>();
    // ONE authority. `RollbackSessionOwnership` says whose session this is;
    // `LocalSessionOwnership.started` only says which policy it was started with,
    // and is meaningless unless the first has already said "mine".
    // ONE authority, at last. `RollbackSessionOwnership` now names the
    // OWNER, not just the session kind, so "is this mine" is a question the
    // authority can answer — and `LocalSessionOwnership.started` is free to be
    // what it always should have been: the POLICY memo, read only once ownership
    // has said yes.
    let owned_settings = maintained_settings(world);
    let owned = owned_settings.and(
        world
            .get_resource::<LocalSessionOwnership>()
            .and_then(|state| state.started),
    );

    // THE CALLER STARTS THIS ONE, so this owner must not. `PlatformerApp::rollback(n)` sets
    // `autostart = false` because its own contract says it does not start a session — the
    // caller does, with `rollback::start`, which activates, settles and THEN rebases frame
    // zero. It cost the external consumer one tick — gate opened on 179 against 180 — which is
    // the whole point of having that fixture.
    //
    // This guard is exactly that window.
    let autostart = world
        .get_resource::<LocalSessionPolicy>()
        .copied()
        .unwrap_or_default()
        .autostart;
    if !autostart && owned_settings.is_none() {
        return;
    }

    if !gameplay_active {
        if owned_settings.is_some() && session_live {
            super::session::stop_session(world);
        }
        if owned_settings.is_some() {
            // THE TOPOLOGY BELONGS TO THE GAMEPLAY SESSION, so it ends with
            // it. Left standing it is the previous match's seating presented to
            // the next one as a frozen fact — and the versus roster reads any
            // frozen topology without asking whether a session owns it, so two
            // people who played, quit and came back with one controller would
            // still seat two fighters.
            world.remove_resource::<ambition_input::LocalSeatTopology>();
        }
        if let Some(mut state) = world.get_resource_mut::<LocalSessionOwnership>() {
            state.started = None;
        }
        return;
    }

    // ⛔⛤ **THE TIMELINE DOES NOT START OVER A WORLD WHOSE DURABLE STATE IS
    // STILL BEING LOADED.** Measured 2026-09-16: with nothing ordering the
    // durable-restore chain against this maintainer, whether a simulation tick
    // runs before `SaveRestored` rises was decided by unrelated `Update`
    // membership. Two worlds differing only by one extra `Update` system gave
    // "no tick ever runs unrestored" and "tick 0 runs unrestored" — and a
    // conversation opening on that tick has its visit dropped by
    // `count_the_dialogue_visit_when_a_conversation_opens`, whose `!restored.0`
    // guard is correct and has nowhere to put the write.
    //
    // ⭐ ONE ROAD, NOT A MIRROR. The latch is read where it LIVES, through the
    // `actor_monolith` dependency this crate already has. A readiness flag
    // mirrored into a lower layer would be a second copy of one fact, which is
    // the duplicate authority this repair exists to remove — and the ordering
    // cannot be expressed as a schedule edge, because the condition is "the
    // file has been applied", not "a system has run once".
    //
    // ⚠ **THE QUESTION IS "PENDING", NEVER "`!restored`" — MEASURED AT THE COST
    // OF 66 TESTS.** `SaveRestored` is not a latch that always rises: it is a
    // completion fact about one domain in one experience, and a smash match
    // never satisfies the singleton body `complete_durable_restore` needs, so
    // its latch reads false for the whole process. Gating on the bare latch hung
    // every smash composition at session start. `durable_hydration_is_pending`
    // is three-valued for exactly that reason and owns the distinction; see its
    // doc comment.
    //
    // ⚠ AND IT GATES THE START ONLY. A live session is never torn down or left
    // stopped by this: the condition is `!session_live`, so the one transition
    // it can refuse is "no session -> session". `SaveRestored` has no
    // mid-session `true -> false` transition left to create a stall, and the
    // `debug_assert` in `reset_inventory_on_new_game` is what keeps that true.
    if !session_live
        && ambition_platformer2d_actor_monolith::session::durable_horizon::durable_hydration_is_pending(
            world,
        )
    {
        return;
    }

    // a session this module did not start is AUTHORITATIVE. A Matchbox/P2P
    // session installed through `install_session` outranks the local one; the
    // owner inspects and steps aside rather than replacing it.
    if session_live && owned_settings.is_none() {
        return;
    }

    let policy = world
        .get_resource::<LocalSessionPolicy>()
        .copied()
        .unwrap_or_default();
    // Already running exactly what is asked for — and "exactly" includes HOW
    // MANY PEOPLE.
    //
    // the policy alone was not enough. The roster-aware seating freeze and this maintainer both
    // run in `Update` with no ordering contract between them, so on the first gameplay frame this
    // could freeze a topology from connected DEVICES before the roster published its decided
    // PARTICIPANTS — which differ for a keyboard seat, a spare pad, or a CPU seat.
    //
    // The running `SyncTestSettings` are right there in the ownership resource, so comparing
    // them costs nothing and closes the hole. Removing the comparison alone makes it pass,
    // which isolates it.
    //
    // Detect-and-restart trades a wrong player count for a dead seat. Recorded as G2's residue
    // rather than half-applied.
    if session_live && owned == Some(policy) {
        return;
    }
    if session_live {
        super::session::stop_session(world);
    }

    // HOW MANY PEOPLE ARE PLAYING, asked once and frozen.
    //
    // The roster and the session both need to agree. Sampling `LocalDeviceOrder`
    // independently means a controller connecting between the two samples makes
    // them disagree while both cite "the same source": the roster seats three
    // fighters into a two-handle session and nothing says so. Deciding it once at
    // session start is what makes them the same answer rather than two answers
    // that usually match.
    //
    // captured on the FIRST call of a gameplay session and never again while it
    // lasts — a policy change restarts the GGRS session but must NOT recapture,
    // or the topology would be stable only per sub-session, which is not the
    // lifetime anything else uses.
    // A HOST THAT WILL DECIDE A ROSTER MUST SAY SO, AND WAIT.
    //
    // `freeze_local_seating` captures from connected DEVICES, and devices are not participants: a
    // keyboard seat has no controller entity, a spare pad may not be playing, a CPU seat has no
    // device at all. That is the right answer for a host with no match to decide and the wrong one
    // for a host whose roster arrives a frame later — and the maintainer had no way to tell those
    // apart. Ordering it after `InputSet::Collect` narrowed the race; it could not remove it,
    // because *"usually published in the same Update"* is not an initialization contract.
    //
    // A host that says nothing keeps device-derived seating, which is what every single-player
    // composition wants and what the tests rely on.
    if matches!(
        world.get_resource::<SessionSeatingSource>(),
        Some(SessionSeatingSource::Pending { .. })
    ) {
        return;
    }
    let players = decided_or_device_seating(world);
    let settings = SyncTestSettings {
        check_distance: policy.check_distance,
        max_prediction_window: policy.max_prediction_window,
        players,
    };
    match super::session::start_sync_test_session_owned(
        world,
        settings,
        super::session::SyncTestOwner::LocalMaintainer,
    ) {
        Ok(()) => {
            let mut state = world.resource_mut::<LocalSessionOwnership>();
            state.started = Some(policy);
            state.last_error = None;
        }
        Err(error) => {
            error!("failed to start the local GGRS session: {error}");
            let mut state = world.resource_mut::<LocalSessionOwnership>();
            state.started = None;
            state.last_error = Some(format!("failed to start the local GGRS session: {error}"));
        }
    }
}

/// The frozen seating for this gameplay session, captured once.
fn freeze_local_seating(world: &mut World) -> ambition_input::LocalSeatTopology {
    if let Some(frozen) = world.get_resource::<ambition_input::LocalSeatTopology>() {
        if frozen.is_frozen() {
            return frozen.clone();
        }
    }
    let order = world
        .get_resource::<ambition_input::LocalDeviceOrder>()
        .map(|devices| devices.devices().to_vec())
        .unwrap_or_default();
    let order = ambition_input::LocalDeviceOrder::from_devices(order);
    let mut topology =
        world.get_resource_or_insert_with(ambition_input::LocalSeatTopology::default);
    topology.capture(&order);
    topology.clone()
}

#[cfg(test)]
mod seating_readiness_tests {
    use super::*;
    use bevy::prelude::World;

    /// A host that says nothing seats from devices — the common case.
    ///
    /// this is asserted FIRST because it is what the gate must not break.
    #[test]
    fn a_host_with_no_roster_declaration_is_not_gated() {
        let mut world = World::new();
        world.init_resource::<ambition_input::LocalSeatTopology>();
        world.init_resource::<ambition_input::LocalDeviceOrder>();
        assert_eq!(
            world.get_resource::<SessionSeatingSource>(),
            None,
            "the fixture claimed roster-driven seating, so it is not testing the \
             ungated path"
        );
        // Device-derived: no devices connected, so the floor of one seat.
        assert_eq!(decided_or_device_seating(&mut world), 1);
    }

    /// A claim with no decision yet HOLDS the session.
    ///
    /// this gate shipped disarmed: `SeatingComesFromARoster` was defined and
    /// never inserted anywhere, so every roster-driven host still raced its own
    /// roster and the losing side froze a topology from connected DEVICES — for
    /// a whole match, because the session is never resized afterwards.
    #[test]
    fn a_pending_roster_holds_the_session() {
        let mut world = World::new();
        world.insert_resource(SessionSeatingSource::pending("smash"));
        assert_eq!(
            world.resource::<SessionSeatingSource>().seat_count(),
            None,
            "a pending claim reported a seat count, so the maintainer would size \
             a session from a roster nobody has decided"
        );
    }

    /// A claim is released by the experience that made it, and by nobody else.
    #[test]
    fn only_the_owner_releases_its_claim() {
        let mut seating = SessionSeatingSource::decided(
            "ambition_versus",
            ambition_input::LocalChannelPlan::from_sources(
                [0, 1].map(ambition_input::LocalInputSource::Pad),
            ),
        );
        assert!(!seating.release("smash"), "a stranger released the claim");
        assert_eq!(seating.seat_count(), Some(2));
        assert!(seating.release("ambition_versus"));
        assert_eq!(seating, SessionSeatingSource::Devices);
        assert!(
            !seating.release("ambition_versus"),
            "releasing device-derived seating reported that it had released a claim"
        );
    }

    /// A DECIDED seat count wins over what is plugged in.
    ///
    /// devices are not participants: a keyboard seat has no controller
    /// entity, a spare pad may not be playing, a CPU seat has none at all. A
    /// two-fighter match against one connected pad is the ordinary case, and
    /// freezing from devices sized that session for one.
    #[test]
    fn a_decided_roster_sizes_the_session_not_the_devices() {
        let mut world = World::new();
        world.init_resource::<ambition_input::LocalSeatTopology>();
        world.init_resource::<ambition_input::LocalDeviceOrder>();
        world.insert_resource(SessionSeatingSource::decided(
            "ambition_versus",
            ambition_input::LocalChannelPlan::from_sources(
                [0, 1].map(ambition_input::LocalInputSource::Pad),
            ),
        ));

        assert_eq!(
            decided_or_device_seating(&mut world),
            2,
            "the session was sized from connected devices while a roster had \
             decided two seats"
        );
        // and the claim records WHICH topology it was frozen from, so the
        // roster and the running session cite one fact.
        assert_eq!(
            world.resource::<SessionSeatingSource>().frozen_topology(),
            Some(
                world
                    .resource::<ambition_input::LocalSeatTopology>()
                    .generation()
            ),
            "the claim did not record the topology the session was built from"
        );
        // and the TOPOLOGY carries it too, so the handle count, the per-seat
        // latches and the roster all cite one number rather than agreeing by
        // coincidence.
        let topology = world.resource::<ambition_input::LocalSeatTopology>();
        assert_eq!(topology.declared_seats(), Some(2));
        assert!(
            topology.is_frozen(),
            "a decided seating must still FREEZE, or a hot reload resamples live \
             devices — which is the bug the freeze exists for"
        );
    }
}


#[cfg(test)]
mod mechanical_edit_admission_tests {
    use super::*;
    use ambition_platformer2d_core::{MechanicalEditAdmission, PendingMechanicalEdits};
    use crate::session::{RollbackSessionOwnership, SyncTestOwner, SyncTestSettings};

    /// A world with a LIVE session of the given ownership and one edit waiting.
    ///
    /// ⚠ `session_is_active` is what the decider branches on, so the fixture
    /// asserts it rather than assuming the ownership resource implies it — an
    /// ownership stamp with no session would make every arm below take the
    /// "nothing to protect" road and agree for the wrong reason.
    fn world_with_live_session(ownership: RollbackSessionOwnership) -> World {
        let mut world = World::new();
        world.init_resource::<LocalSessionOwnership>();
        world.insert_resource({
            let mut pending = PendingMechanicalEdits::default();
            pending.propose(fixture_domain());
            pending
        });
        match ownership {
            RollbackSessionOwnership::External => {
                let session =
                    crate::session::build_sync_test_session(SyncTestSettings::for_players(1))
                        .expect("the fixture could not build a GGRS session");
                crate::session::install_session(&mut world, session);
            }
            RollbackSessionOwnership::LocalSyncTest { settings, owner } => {
                crate::session::start_sync_test_session_owned(&mut world, settings, owner)
                    .expect("the fixture could not start a GGRS session");
            }
        }
        assert_eq!(
            world.get_resource::<RollbackSessionOwnership>().copied(),
            Some(ownership),
            "the fixture installed a session under different ownership than it \
             asked for, so the arm below is testing the wrong policy"
        );
        assert!(
            crate::session::session_is_active(&world),
            "the fixture has no live session, so every arm would answer Publish \
             for want of a timeline rather than by policy"
        );
        world
    }

    struct FixtureDomain;
    fn fixture_domain() -> ambition_platformer2d_core::MechanicalDomain {
        ambition_platformer2d_core::MechanicalDomain::of::<FixtureDomain>("fixture")
    }

    fn local() -> RollbackSessionOwnership {
        RollbackSessionOwnership::LocalSyncTest {
            settings: SyncTestSettings::for_players(1),
            owner: SyncTestOwner::LocalMaintainer,
        }
    }

    /// ⭐ **NO PROPOSAL, NO INTERFERENCE.** The overwhelmingly common frame.
    #[test]
    fn a_frame_with_nothing_pending_never_touches_a_live_session() {
        let mut world = world_with_live_session(local());
        world.insert_resource(PendingMechanicalEdits::default());
        decide_mechanical_edit_admission(&mut world);
        assert!(
            crate::session::session_is_active(&world),
            "a frame with no pending edit stopped the rollback session"
        );
        assert_eq!(
            *world.resource::<MechanicalEditAdmission>(),
            MechanicalEditAdmission::Publish
        );
    }

    /// ⭐ **NO TIMELINE, NOTHING TO PROTECT.** Every composition without a
    /// rollback host lives here, and the developer tools must keep working.
    #[test]
    fn an_edit_with_no_live_session_publishes() {
        let mut world = World::new();
        world.insert_resource({
            let mut pending = PendingMechanicalEdits::default();
            pending.propose(fixture_domain());
            pending
        });
        decide_mechanical_edit_admission(&mut world);
        assert_eq!(
            *world.resource::<MechanicalEditAdmission>(),
            MechanicalEditAdmission::Publish,
            "a developer edit was refused by a host that has no timeline to \
             desync, which would break live editing in every non-rollback build"
        );
    }

    /// ⛔⛤ **MODEL 2, AND THE STOP HAPPENS BEFORE THE ANSWER IS `Publish`.**
    ///
    /// `Q120` measured that editing `ActiveMovementTuning` mid-timeline desyncs
    /// the sync-test canary. The baseline this host owns is therefore stopped
    /// and its policy memo released, so `maintain_local_session` starts the next
    /// one against the edited mechanics.
    #[test]
    fn an_edit_rebases_a_baseline_this_host_owns() {
        let mut world = world_with_live_session(local());
        world.resource_mut::<LocalSessionOwnership>().started = Some(LocalSessionPolicy::default());

        decide_mechanical_edit_admission(&mut world);

        assert!(
            !crate::session::session_is_active(&world),
            "a live mechanical edit left this host's own baseline running, so \
             resimulation keeps reading the latest value — the state `Q120` \
             calls not coherent"
        );
        assert_eq!(
            world.resource::<LocalSessionOwnership>().running_policy(),
            None,
            "the policy memo still names a session that was just stopped, so the \
             maintainer would see 'already running exactly this' and never \
             rebuild"
        );
        assert_eq!(
            *world.resource::<MechanicalEditAdmission>(),
            MechanicalEditAdmission::Publish
        );
    }

    /// ⛔⛤ **AN UNHEALTHY TIMELINE REFUSES THE EDIT TOO — THE ARM `Q118` AND
    /// `Q120` EACH ASSUMED THE OTHER HAD.**
    ///
    /// Before the two rows shared one boundary projection, this decision branched
    /// on *session active + ownership* and never consulted HEALTH, while the
    /// content publication road refused an unhealthy authority outright. A
    /// recorded divergence is a state no model in either row permits a mechanical
    /// mutation across.
    ///
    /// ⚠ **IT WAS SAFE WITHOUT THIS ARM, FOR A REASON NEITHER FEATURE STATED**,
    /// which is exactly why the arm exists rather than a comment: `stop_session`
    /// STANDS THE AUTHORITY DOWN rather than removing it, and
    /// `ActiveRollbackAuthority::installed` refuses to launder a divergence into
    /// a fresh timeline — so a rebase could not have healed one. *"Safe because
    /// something else happens to hold"* is a coincidence one refactor away from
    /// being false.
    #[test]
    fn an_unhealthy_timeline_refuses_a_mechanical_edit_rather_than_rebasing_across_it() {
        let mut world = world_with_live_session(local());
        // ⚠ THE PREMISE, and it is the load-bearing half: this ownership is one
        // this host COULD rebase, so a refusal below is about HEALTH and not
        // about ownership.
        assert!(
            crate::local_session::locally_rebasable_timeline(&world),
            "the fixture's session is not locally rebasable, so refusing it says \
             nothing about health"
        );
        world
            .resource_mut::<ambition_platformer2d_runtime::rollback::ActiveRollbackAuthority>()
            .invalidate("a deliberate desync, for this test".to_string());

        decide_mechanical_edit_admission(&mut world);

        assert_eq!(
            *world.resource::<MechanicalEditAdmission>(),
            MechanicalEditAdmission::Refuse,
            "a mechanical edit was admitted across a RECORDED DIVERGENCE. The \
             content publication road refuses the same world, and two features \
             defining one policy is what produced `Q118`'s half-question lease."
        );
        assert!(
            crate::session::session_is_active(&world),
            "the unhealthy baseline was STOPPED for a developer edit, which is a \
             rebase across a divergence"
        );
        assert!(
            world.resource::<PendingMechanicalEdits>().any_pending(),
            "the refused edit was discarded rather than staged"
        );
    }

    /// ⛔⛤ **MODEL 1 MEANS THE AUTHORITATIVE VALUE DOES NOT MOVE — WHICH IS NOT
    /// WHAT THE FIRST VERSION OF THIS FIX GUARDED.**
    ///
    /// That version's arm was called `a_session_this_host_does_not_own_is_left
    /// _alone` and asserted only that no rebase fired. It passed while the edit
    /// still reached `ActiveMovementTuning` — i.e. while the timeline resimulated
    /// against a value it had never seen. Leaving the session alone is the
    /// EASY half; refusing the edit is the half that makes the state coherent,
    /// and it is what `Q120`'s model 1 actually says.
    ///
    /// ⚠ And the proposal is RETAINED, not dropped: an edit the developer made
    /// must not silently vanish because a peer session happened to be live. It
    /// publishes by itself the moment the refusal stops applying.
    #[test]
    fn a_session_this_host_does_not_own_refuses_the_edit_and_keeps_it() {
        for (what, ownership) in [
            ("an EXTERNAL/P2P session", RollbackSessionOwnership::External),
            (
                "a CALLER-owned sync test",
                RollbackSessionOwnership::LocalSyncTest {
                    settings: SyncTestSettings::for_players(1),
                    owner: SyncTestOwner::Caller,
                },
            ),
        ] {
            let mut world = world_with_live_session(ownership);

            decide_mechanical_edit_admission(&mut world);

            assert_eq!(
                *world.resource::<MechanicalEditAdmission>(),
                MechanicalEditAdmission::Refuse,
                "{what} admitted a local developer edit, so its timeline \
                 resimulates against mechanics it never ran with"
            );
            assert!(
                crate::session::session_is_active(&world),
                "{what} was stopped by this host for a local developer edit"
            );
            assert!(
                world
                    .resource::<PendingMechanicalEdits>()
                    .is_pending(fixture_domain()),
                "{what} DISCARDED the developer's edit instead of staging it, so \
                 the value the inspector shows is not the value that will ever \
                 be published"
            );

            // ⭐ AND THE REFUSAL EXPIRES WITH ITS REASON. The same retained
            // proposal publishes as soon as the session it was protecting ends,
            // which is what makes staging honest rather than a quiet drop.
            crate::session::stop_session(&mut world);
            decide_mechanical_edit_admission(&mut world);
            assert_eq!(
                *world.resource::<MechanicalEditAdmission>(),
                MechanicalEditAdmission::Publish,
                "{what} kept refusing a staged edit after its timeline ended"
            );
        }
    }
}
