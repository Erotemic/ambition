//! **Rollback, as a supported promise.**
//!
//! ADR 0031 deferred this deliberately, and the campaign's Deferred section
//! says why: rollback is "a far larger promise than a clock: frozen schema,
//! complete authoritative baseline, stable participants, deterministic
//! activation, lifecycle rebasing, confirmation boundaries. Its own slice, its
//! own acceptance tests."
//!
//! This is that slice. The six properties are not documented here — they are
//! the reason this module exists at all, because each one is a way a consumer
//! could hold rollback wrong, and every one of them was held wrong first by
//! the engine's own fixture. [`start`] performs the sequence Outlander used to
//! perform by hand, so the orderings that produce a desync are unreachable
//! rather than warned about:
//!
//! | Property | How this module keeps it |
//! |---|---|
//! | frozen schema | `rollback-wire-format-is-frozen` in `check_absence_contracts.py`; 63 encoded types across 9 crates, and a type joining or leaving is a ratchet failure |
//! | complete authoritative baseline | [`start`] refuses a host whose registry is empty — see [`RollbackRefused::NoAuthoritativeState`] |
//! | stable participants | the count is declared at COMPOSITION and cannot be passed per-session, so a restart reuses it — see [`crate::app::PlatformerApp::rollback`] |
//! | deterministic activation | [`start`] drives the host to `Running` itself; a consumer cannot begin before construction finishes |
//! | lifecycle rebasing | the session rebases onto the CURRENT live world as frame zero |
//! | confirmation boundaries | [`start`] settles past activation before frame zero, because activation completing is not the same fact as the next tick being quiet |
//!
//! ## Why a function and not a builder flag
//!
//! The composition half IS a builder flag — [`crate::app::PlatformerApp::rollback`].
//! But a session cannot be started at build time: the world has to be
//! CONSTRUCTED first. Preparation and the session-world commit build the room
//! and the body through `Commands`, and a rollback cannot undo construction —
//! rewinding across it is a guaranteed divergence, and a sync test reports it
//! immediately and correctly.
//!
//! The engine's own fixture learned that by starting the session on update #1
//! and watching GGRS report a checksum mismatch on frames 2, 3 and 4 forever.
//! That is why the entry point is a function that takes a built `App`: the step
//! it performs — construct, settle, then rebase frame zero onto the result —
//! cannot be expressed as a flag, and a flag that pretended otherwise would
//! hand every consumer the same three-frame mismatch.

use bevy::prelude::App;

use crate::app::{host_status, HostStatus};

pub use ambition_engine_core::snapshot::{
    checksum_bytes, cursor_checksum, decode_state, encode_state, put_bool, put_f32, put_i32,
    put_opt_str, put_str, put_u32, put_u64, put_u8, put_vec2, resolved_checksum, state_checksum,
    Reader, SnapshotCursor, SnapshotResolve, SnapshotState, StateHasher,
};
pub use ambition_runtime::rollback::{
    AmbitionRollbackApp, RollbackRegistrationDescriptor, RollbackRegistry,
};

/// How a rollback session should be brought up.
///
/// ⚠ **The participant count is deliberately NOT here.** It is declared at
/// composition, by [`crate::app::PlatformerApp::rollback`], because a restart
/// must reuse the frozen value rather than re-sample it: proof pulses,
/// hot-reload rebases and lifecycle commits are all the same session
/// RESTARTED, and a count that moved between them would seat a different
/// number of players into what claims to be the same match. Putting it on the
/// plan would make it an argument to every restart — three chances to disagree
/// where the engine went to some trouble to have one answer.
///
/// The engine shipped the weaker version of this bug already:
/// `..Default::default()` silently meant one player, and a rollback oracle
/// proved determinism for ONE input stream the same week a 2–4 player couch
/// versus mode landed. A desync in seat two had nowhere to show up.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RollbackPlan {
    check_distance: usize,
    prediction_window: usize,
    activation_budget: usize,
    settle_ticks: usize,
}

impl RollbackPlan {
    /// The default plan: four frames of comparison, ten of prediction.
    ///
    /// Every participant is local — a sync test has no remote peer by
    /// definition. What it buys is not networking: it is that N input streams
    /// go through save/rewind/resimulate and are checksum-compared, which is
    /// the precondition for any of them being remote later.
    pub fn new() -> Self {
        Self {
            check_distance: 4,
            prediction_window: 10,
            // 600 ticks is ten seconds at 60Hz. It is a budget for a host that
            // will never activate, not an expectation — the engine's own
            // fixture activates in well under a hundred.
            activation_budget: 600,
            settle_ticks: 8,
        }
    }

    /// How many frames back the session re-simulates and compares.
    pub fn check_distance(mut self, frames: usize) -> Self {
        self.check_distance = frames;
        self
    }

    /// How far ahead the session may predict before it must stall.
    pub fn prediction_window(mut self, frames: usize) -> Self {
        self.prediction_window = frames;
        self
    }

    /// How many ticks [`start`] may spend waiting for the host to activate.
    pub fn activation_budget(mut self, ticks: usize) -> Self {
        self.activation_budget = ticks;
        self
    }

    /// How many quiet ticks to run after activation before frame zero.
    ///
    /// ⚠ Lowering this to zero is how a consumer reintroduces the hazard the
    /// campaign names: seating completes on the session's first frame, so
    /// activation would land on GGRS frame 1 where nothing can rewind across
    /// it. The knob exists because a game with a different activation shape
    /// may need MORE, not because zero is a supported choice.
    pub fn settle_ticks(mut self, ticks: usize) -> Self {
        self.settle_ticks = ticks;
        self
    }
}

impl Default for RollbackPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// The participant count a rollback composition declared.
///
/// Inserted by [`crate::app::PlatformerApp::rollback`]. It is the consumer's
/// STATEMENT about its own topology, which is a different fact from how many
/// devices happen to be plugged in — [`start`] refuses when the two disagree
/// rather than picking one.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeclaredParticipants(pub usize);

/// Why a rollback session could not start.
///
/// Every variant names the thing the author must change. A refusal that said
/// only "failed" would send them into `crates/`, which is the failure ADR
/// 0031's blind-agent gate measures.
///
/// ⚠ **A fifth variant was written and deleted before this shipped.**
/// `ParticipantsDisagree` compared the declared count against
/// `LocalSeatTopology` and refused when they differed. It had a good story —
/// the shipped host reads the seat topology, a consumer declaring
/// "single-player" states something the engine cannot check — and it could
/// never fire: a probe found that on a host composed through
/// `PlatformerApp::rollback`, `LocalSeatTopology` is never inserted at all.
/// The resource is populated by the dev observatory's own session path, not by
/// the builder.
///
/// It was deleted rather than kept as harmless. An unreachable refusal reads
/// as protection, and the property it claimed to defend is enforced by
/// something real instead: the count is not passable per session, so there is
/// no second value to disagree with.
#[derive(Clone, Debug)]
pub enum RollbackRefused {
    /// The host was not composed for rollback.
    NotComposedForRollback,
    /// The host never reached `Running` within the plan's budget.
    NeverActivated {
        /// Ticks spent waiting.
        ticks: usize,
        /// What the host was doing when the budget ran out — a `Refused` host
        /// carries the reasons preparation rejected it, which is the whole
        /// diagnosis.
        status: String,
    },
    /// Nothing registered any authoritative state.
    ///
    /// A session over an empty registry saves nothing, rewinds nothing, and
    /// compares nothing — it passes. That is the shape this repo calls an
    /// instrument that measures nothing and reports the success condition, and
    /// it is worth a refusal rather than a green run.
    NoAuthoritativeState,
    /// GGRS rejected the session.
    SessionRejected(String),
}

impl core::fmt::Display for RollbackRefused {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotComposedForRollback => write!(
                f,
                "this host was not composed for rollback — build it with \
                 `PlatformerApp::rollback(participants)` rather than the \
                 fixed-step default"
            ),
            Self::NeverActivated { ticks, status } => write!(
                f,
                "the host never started in {ticks} ticks; it was {status}. A \
                 rollback session rebases frame zero onto the live world, so \
                 the world has to exist first"
            ),
            Self::NoAuthoritativeState => write!(
                f,
                "nothing registered authoritative state, so a session would \
                 save, rewind and compare nothing and pass. Register with \
                 `rollback_component_canonical` (or a sibling) before starting"
            ),
            Self::SessionRejected(why) => write!(f, "GGRS refused the session: {why}"),
        }
    }
}

impl std::error::Error for RollbackRefused {}

/// A started rollback session.
///
/// Returned rather than dropped so a caller can assert on the facts that make
/// the session meaningful — chiefly [`Self::encoded_types`], because a session
/// over nothing is the failure mode that looks most like success.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RollbackSession {
    ticks_to_activation: usize,
    participants: usize,
    encoded_types: usize,
}

impl RollbackSession {
    /// Ticks the host took to reach `Running`, before settling.
    pub fn ticks_to_activation(&self) -> usize {
        self.ticks_to_activation
    }

    /// How many participants the session seated.
    pub fn participants(&self) -> usize {
        self.participants
    }

    /// How many distinct kinds of authoritative state the session carries.
    pub fn encoded_types(&self) -> usize {
        self.encoded_types
    }
}

/// Bring a composed rollback host up to a running session.
///
/// Construct, wait for activation, settle, then rebase frame zero onto the
/// result. Each step is here because skipping it produces a desync that
/// reports as a checksum mismatch several frames later, where it reads like a
/// bug in the game rather than a bug in the startup order.
pub fn start(app: &mut App, plan: RollbackPlan) -> Result<RollbackSession, RollbackRefused> {
    // The host kind is a resource the engine sets when the plugin group is
    // chosen, so this reads the composition's actual decision rather than a
    // flag the caller repeats.
    let is_rollback_host = app
        .world()
        .get_resource::<ambition_runtime::SimulationHost>()
        .is_some_and(|host| host.is_ggrs());
    let declared = app.world().get_resource::<DeclaredParticipants>().copied();
    let (true, Some(DeclaredParticipants(participants))) = (is_rollback_host, declared) else {
        return Err(RollbackRefused::NotComposedForRollback);
    };

    let mut ticks_to_activation = None;
    for tick in 0..plan.activation_budget {
        app.update();
        if host_status(app).is_running() {
            ticks_to_activation = Some(tick + 1);
            break;
        }
    }
    let ticks_to_activation = ticks_to_activation.ok_or_else(|| {
        let status = match host_status(app) {
            HostStatus::Refused { reasons } => format!("refused: {}", reasons.join("; ")),
            other => format!("{other:?}"),
        };
        RollbackRefused::NeverActivated {
            ticks: plan.activation_budget,
            status,
        }
    })?;

    // Activation completing is not the same fact as the tick after it being
    // quiet.
    for _ in 0..plan.settle_ticks {
        app.update();
    }

    let encoded_types = app
        .world()
        .get_resource::<RollbackRegistry>()
        .map(|registry| registry.descriptors().count())
        .unwrap_or(0);
    if encoded_types == 0 {
        return Err(RollbackRefused::NoAuthoritativeState);
    }

    ambition_runtime::rollback::start_sync_test_session(
        app.world_mut(),
        ambition_runtime::rollback::SyncTestSettings {
            check_distance: plan.check_distance,
            max_prediction_window: plan.prediction_window,
            ..ambition_runtime::rollback::SyncTestSettings::for_players(participants)
        },
    )
    .map_err(|error| RollbackRefused::SessionRejected(error.to_string()))?;
    app.update();

    Ok(RollbackSession {
        ticks_to_activation,
        participants,
        encoded_types,
    })
}
