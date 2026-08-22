//! Match-seat binding and the rollback-safe receipt for a live prepared match.

use bevy::prelude::*;

/// Stable roster seat carried by the fighter body.
/// Driving participant, character id, and entity order cannot substitute for seat identity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchSeat(pub usize);

/// Derive live fighter entities from rollback-restored [`MatchSeat`] components, sorted by seat.
/// No resource stores live entity handles for the cast.
pub fn match_participants(seated: &Query<(Entity, &MatchSeat)>) -> Vec<Entity> {
    let mut by_seat: Vec<(usize, Entity)> = seated
        .iter()
        .map(|(entity, seat)| (seat.0, entity))
        .collect();
    by_seat.sort_by_key(|(seat, _)| *seat);
    by_seat.into_iter().map(|(_, entity)| entity).collect()
}

/// Receipt for a fully activated match.
/// Stores activation facts only; the live cast is derived from [`MatchSeat`] components.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ActiveMatch {
    /// How many seats this match activated with. Compare it against
    /// [`match_participants`] to ask whether the cast is still whole.
    seats: usize,
    /// The frozen seat topology this match was activated against, copied from
    /// the roster so the two can be COMPARED rather than assumed equal.
    seat_topology: Option<u64>,
    /// Whose plan this is a receipt for. `None` in a composition with no
    /// session lifecycle at all, which is the same answer `PreparedMatch` stamps
    /// there, so the two still compare equal.
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    /// Simulation tick of activation, used to derive opening-ceremony phase without mutable timer state.
    /// `None` means the composition has no simulation clock.
    activated_on: Option<u64>,
}

/// Stable activation identity used by ruleset-local per-match state.
/// It derives from rollback-restored session and activation tick, so stale state fails identity match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MatchInstance {
    /// The gameplay session the cast was built in.
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    /// The sim tick it was built on.
    activated_on: Option<u64>,
}

impl MatchInstance {
    /// The two facts, for the wire format. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn parts(
        &self,
    ) -> (
        Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        Option<u64>,
    ) {
        (self.session, self.activated_on)
    }

    /// Rebuild a present activation from rollback state; resource snapshotting separately restores absence.
    #[doc(hidden)]
    pub fn from_snapshot(
        session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        activated_on: Option<u64>,
    ) -> Self {
        Self {
            session,
            activated_on,
        }
    }
}

impl ActiveMatch {
    /// Publish the receipt after the full cast has been activated.
    pub fn activated(
        seats: usize,
        seat_topology: Option<u64>,
        session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        activated_on: Option<u64>,
    ) -> Self {
        Self {
            seats,
            seat_topology,
            session,
            activated_on,
        }
    }

    /// How many ticks the match has been live, or `None` when the composition
    /// has no clock to measure against.
    pub fn ticks_since_activation(&self, now: u64) -> Option<u64> {
        self.activated_on.map(|then| now.saturating_sub(then))
    }

    /// Session whose prepared plan this activation receipts.
    pub fn session(
        &self,
    ) -> Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId> {
        self.session
    }

    /// Number of seats activated; live participants are derived from the world.
    pub fn seats(&self) -> usize {
        self.seats
    }

    /// Identity rulesets use to key per-match state.
    pub fn instance(&self) -> MatchInstance {
        MatchInstance {
            session: self.session,
            activated_on: self.activated_on,
        }
    }

    /// Which frozen topology decided this match's seating, if a session had
    /// frozen one when the roster was built.
    pub fn seat_topology(&self) -> Option<u64> {
        self.seat_topology
    }

    /// Record the frozen topology that already agrees with this unchanged seating.
    pub fn adopt_seat_topology(&mut self, generation: u64) {
        self.seat_topology = Some(generation);
    }

    /// Test-only constructor for a live match without preparation.
    #[doc(hidden)]
    pub fn for_test(seats: usize, seat_topology: Option<u64>) -> Self {
        Self {
            seats,
            seat_topology,
            session: None,
            // No clock means no opening-ceremony hold.
            activated_on: None,
        }
    }

    /// The sim tick the cast was built on, when the composition had a clock.
    pub fn activated_on(&self) -> Option<u64> {
        self.activated_on
    }

    /// Rebuild an activation from a rollback snapshot.
    ///
    /// what makes registering this correct is that `bevy_ggrs` restores ABSENCE:
    /// `ResourceSnapshotPlugin::load` maps `(Some(_), None)` to `remove_resource`. Registration
    /// would have been decorative if the plugin only overwrote a present value, which is worth
    /// stating because that is the assumption the fix rests on.
    #[doc(hidden)]
    pub fn from_snapshot(
        seats: usize,
        seat_topology: Option<u64>,
        session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        activated_on: Option<u64>,
    ) -> Self {
        Self {
            seats,
            seat_topology,
            session,
            activated_on,
        }
    }
}
