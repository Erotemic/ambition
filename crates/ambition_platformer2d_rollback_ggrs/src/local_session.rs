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
