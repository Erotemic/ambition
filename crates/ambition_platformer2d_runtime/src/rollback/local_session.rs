//! **Who owns the local GGRS session** — the engine, not a developer tool.
//!
//! A GGRS host advances the simulation only while a `Session` resource exists:
//! with none, `GgrsSchedule` runs zero times and the app composes, boots,
//! renders and never simulates. Measured 2026-08-03.
//!
//! ⛔ **and until this module the ONLY thing that installed one was
//! `RollbackObservatoryPlugin`, behind `#[cfg(feature = "dev_tools")]`.** That is
//! why `build_visible_app` chose the GGRS host inside the same `cfg`: not a
//! developer convenience, but a coupling — the host choice depended on the only
//! thing that could supply a session, so a build without dev tooling could not
//! safely pick it. It is also why the shipped rollback schema was exercised by
//! nothing outside the test harnesses.
//!
//! # The split this module is
//!
//! The dev observatory did two jobs in one 120-line function. They are different
//! jobs and only one of them is a developer's:
//!
//! | | owns | lives |
//! |---|---|---|
//! | **the session OWNER** (here) | whether a session exists at all, and for how many players | the engine |
//! | **the proof instrument** (`dev::rollback_observatory`) | how many frames it verifies, on request | dev tooling |
//!
//! So the observatory is now a KNOB on this owner rather than a second owner —
//! it raises [`LocalSessionPolicy::check_distance`] for a pulse and lowers it
//! again. ⛔ two owners of one session is the shape this repo has been bitten by
//! before (three sites learned a roster's `published_by` separately), and a
//! relocation avoids it by construction: there is still exactly one owner.
//!
//! ⚠ **a session this module did not start is never replaced.** A future
//! Matchbox/P2P session is authoritative; the owner steps aside for it, which is
//! the same rule the observatory had and the reason `install_session` exists as a
//! separate seam.

use bevy::prelude::*;

/// Where the local session is decided, so a developer instrument can order its
/// policy write BEFORE the owner acts on it in the same frame.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LocalSessionSet {
    /// [`maintain_local_session`] runs here.
    Maintain,
}

use super::session::{AmbitionGgrsSession, SyncTestSettings};

/// How the local session is configured. The OWNER reads it; a developer
/// instrument may write it to ask for deeper verification.
///
/// ⭐ **`check_distance = 0` is the shipped default and it is not a stub.** A
/// zero check distance makes GGRS issue one `AdvanceFrame` and no save/load
/// requests: the simulation is driven deterministically and rollback stays
/// dormant, which is what a single-player session wants. The measured cost of
/// the alternative is real — a save-and-checksum floor of ~1.8 ms/frame against
/// a 2.17 ms simulation step (release, 2026-08-03).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalSessionPolicy {
    /// Frames of resimulation the session verifies each tick. `0` = rollback
    /// dormant.
    pub check_distance: usize,
    /// The furthest the session may speculate ahead of confirmation.
    pub max_prediction_window: usize,
}

impl Default for LocalSessionPolicy {
    fn default() -> Self {
        Self {
            check_distance: 0,
            max_prediction_window: 8,
        }
    }
}

/// What the owner currently has running, so a restart can tell "nothing yet"
/// from "the policy changed" from "somebody else's session".
#[derive(Resource, Default, Debug)]
pub struct LocalSessionOwnership {
    /// `Some` when THIS module started the live session, carrying the policy it
    /// was started with.
    started: Option<LocalSessionPolicy>,
    /// Set once a start fails, so the failure is reportable rather than a
    /// silently sessionless app.
    pub last_error: Option<String>,
}

impl LocalSessionOwnership {
    /// True when the live session is one this module started.
    pub fn owns_session(&self) -> bool {
        self.started.is_some()
    }

    /// The policy the live session was started with, if this module started it.
    pub fn running_policy(&self) -> Option<LocalSessionPolicy> {
        self.started
    }

    /// Give up ownership so the owner rebuilds on the next frame.
    ///
    /// For a caller that has just invalidated the world under the session — a
    /// content hot-reload — and needs it rebased. ⚠ it does NOT touch the frozen
    /// seating: a reload changes the level, not who is playing.
    pub fn release(&mut self) {
        self.started = None;
    }
}

/// Keep exactly one local session alive for as long as gameplay is.
///
/// Exclusive-world because starting and stopping a GGRS session is a whole-world
/// operation (it rebases snapshot storage), which is also why this cannot be an
/// ordinary system with resource params.
pub fn maintain_local_session(world: &mut World) {
    let gameplay_active =
        ambition_platformer2d_shared_tangle::lifecycle::session_world_entity(world).is_some();
    let session_live = world.contains_resource::<AmbitionGgrsSession>();
    let owned = world
        .get_resource::<LocalSessionOwnership>()
        .and_then(|state| state.started);

    if !gameplay_active {
        if owned.is_some() && session_live {
            super::session::stop_session(world);
        }
        if owned.is_some() {
            // ⛔ **THE TOPOLOGY BELONGS TO THE GAMEPLAY SESSION, so it ends with
            // it.** Left standing it is the previous match's seating presented to
            // the next one as a frozen fact — and the versus roster reads any
            // frozen topology without asking whether a session owns it, so two
            // people who played, quit and came back with one controller would
            // still seat two fighters (GPT 5.6, 2026-07-29).
            world.remove_resource::<ambition_input::LocalSeatTopology>();
        }
        if let Some(mut state) = world.get_resource_mut::<LocalSessionOwnership>() {
            state.started = None;
        }
        return;
    }

    // ⚠ **a session this module did not start is AUTHORITATIVE.** A Matchbox/P2P
    // session installed through `install_session` outranks the local one; the
    // owner inspects and steps aside rather than replacing it.
    if session_live && owned.is_none() {
        return;
    }

    let policy = world
        .get_resource::<LocalSessionPolicy>()
        .copied()
        .unwrap_or_default();
    // Already running exactly what is asked for.
    if session_live && owned == Some(policy) {
        return;
    }
    if session_live {
        super::session::stop_session(world);
    }

    // ⭐ **HOW MANY PEOPLE ARE PLAYING, asked once and frozen.**
    //
    // The roster and the session both need to agree. Sampling `LocalDeviceOrder`
    // independently means a controller connecting between the two samples makes
    // them disagree while both cite "the same source": the roster seats three
    // fighters into a two-handle session and nothing says so. Deciding it once at
    // session start is what makes them the same answer rather than two answers
    // that usually match (GPT 5.6, 2026-07-28).
    //
    // ⚠ captured on the FIRST call of a gameplay session and never again while it
    // lasts — a policy change restarts the GGRS session but must NOT recapture,
    // or the topology would be stable only per sub-session, which is not the
    // lifetime anything else uses.
    let players = freeze_local_seating(world).players();
    let settings = SyncTestSettings {
        check_distance: policy.check_distance,
        max_prediction_window: policy.max_prediction_window,
        players,
    };
    match super::session::start_sync_test_session(world, settings) {
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
