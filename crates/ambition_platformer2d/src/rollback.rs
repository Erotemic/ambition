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
//! | frozen schema | `rollback-wire-format-is-frozen` in `check_absence_contracts.py`; 81 encoded types across 11 crates, and a type joining or leaving is a ratchet failure |
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

pub use ambition_platformer2d_core::snapshot::{
    checksum_bytes, cursor_checksum, decode_state, encode_state, put_bool, put_f32, put_i32,
    put_opt_str, put_str, put_u32, put_u64, put_u8, put_vec2, resolved_checksum, state_checksum,
    Reader, SnapshotCursor, SnapshotResolve, SnapshotState, StateHasher,
};
pub use ambition_platformer2d_runtime::rollback::{
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
    ///
    /// ⚠ **and a tick count is not a readiness CONTRACT.** No number here
    /// proves a host is settled; it buys frames, and the default buys enough
    /// for every activation shape in this repo. Do not read a passing run as
    /// "eight ticks is the requirement", do not tune it to make a flaky test
    /// pass, and do not build a claim on top of it — the durable version is a
    /// semantic barrier that waits for the activation to be CONFIRMED, and it
    /// belongs with atomic match/session activation rather than here. Until
    /// then this is a harness detail that happens to be public because
    /// [`start`] needs it.
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
/// ⚠ **A fifth variant was written and deleted before this shipped, and its
/// PREMISE HAS SINCE EXPIRED.** `ParticipantsDisagree` compared the declared
/// count against `LocalSeatTopology` and refused when they differed. It was
/// deleted as unreachable: a probe found that on a host composed through
/// `PlatformerApp::rollback`, `LocalSeatTopology` was never inserted at all,
/// being populated only by the dev observatory's own session path.
///
/// ⛔ **that is no longer true.** `freeze_local_seating_for_the_decided_match`
/// is registered by `PlatformerHostPlugins`, which this builder adds
/// unconditionally, so a topology IS frozen here the moment a match publishes a
/// roster — which is precisely the situation the refusal was about. The reason
/// for deleting it has expired even though the deletion may still be right.
///
/// ⚠ **so this is a live question, not a settled one**, and it is recorded
/// rather than acted on: restoring a refusal is a design decision, and the
/// original argument for removing it ("an unreachable refusal reads as
/// protection") no longer applies to a reachable one. What has NOT changed is
/// the other half of that argument — the count is not passable per session, so
/// there is still no second value for a consumer to disagree with. Which of
/// those two facts governs is the thing to decide.
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
    /// Activation never produced a session world.
    ///
    /// ⛔ **a rollback session must not begin before activation has settled**,
    /// and until 2026-08-06 nothing said so — because nothing could notice. A
    /// direct-entry host spawns its session root at PLUGIN-BUILD time, so
    /// "the world exists" was true before the first frame and the rule had no
    /// teeth to grow.
    ///
    /// Under a shell-routed host it is not true: the root arrives several frames
    /// in, behind a load barrier and eight preparation work items. A sync test
    /// started before then compares frames containing ACTIVATION work, and the
    /// two runs diverge on the world being built rather than on gameplay —
    /// measured, when the sim harness was pointed at the shell: *"GGRS sync-test
    /// checksum mismatch at frames [2, 3, 4]"*, with the desync canary reporting
    /// no controlled subject beside it.
    ///
    /// ⭐ **so the fix is not a longer budget, it is a stated precondition.** A
    /// session that begins over a world that does not exist yet is measuring
    /// construction; refusing is the honest answer, and it names the thing to
    /// wait for.
    NoSessionWorld {
        /// Ticks spent activating before the check.
        ticks: usize,
    },
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
            Self::NoSessionWorld { ticks } => write!(
                f,
                "the host reached Running in {ticks} ticks but no session world \
                 exists yet, so a session opened here would compare ACTIVATION \
                 rather than gameplay and its checksum mismatch would read as a \
                 desync in the game. Wait for the activation to produce a world \
                 — `settle_until_session_world` is the helper — before starting"
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

/// Whether a rollback session is still doing its job.
///
/// ⚠ **A started session is not a running one, and nothing in this SDK could
/// tell the difference until now.** Blind run 7 watched `host_status` report
/// `Running { prepared: true }` for 4300 consecutive updates while its sim was
/// frozen and its player body had not moved by a single float. GGRS reports a
/// desync through a `warn!` and a message a headless consumer never sees, so
/// the only honest answer available to that author was "the engine is broken".
///
/// [`RollbackSession`] reports STARTUP facts — participants, encoded types,
/// ticks to activation — and all three were healthy while the session was not.
/// This is the liveness half.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RollbackHealth {
    /// No session has been started on this host, or the one that was has been
    /// stopped.
    NoSession,
    /// Simulating, with no mismatch reported.
    Healthy {
        /// The session's current frame. Compare it across updates: a frame
        /// that stops advancing is a stalled session even when nothing has
        /// reported a mismatch.
        frame: i32,
        /// Which timeline this is. Frames restart at zero on every session, so
        /// "frame 3" alone cannot distinguish a restarted session from the one
        /// it replaced; the generation can, and it is the only fact here that
        /// survives a rebase.
        generation: u64,
    },
    /// The session re-simulated a frame and got a different answer.
    ///
    /// This is a determinism bug in the game or the engine, and it is the
    /// whole reason to run a sync test. The frames are the ones that differed.
    Desynced {
        /// Frames whose re-simulation disagreed.
        frames: Vec<i32>,
        /// The frame the session had reached.
        frame: i32,
        /// Which timeline disagreed. See [`Self::Healthy`].
        generation: u64,
    },
    /// The session was invalidated and will not continue.
    Invalidated {
        /// Why, in the engine's words.
        reason: String,
    },
}

impl RollbackHealth {
    /// Simulating and undesynced.
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy { .. })
    }

    /// The session's current frame, if there is a session.
    ///
    /// ⚠ A frame that does not ADVANCE between updates is a stalled session,
    /// and no variant here reports that on its own — liveness is a property of
    /// two observations, not one. Sample it twice.
    pub fn frame(&self) -> Option<i32> {
        match self {
            Self::Healthy { frame, .. } | Self::Desynced { frame, .. } => Some(*frame),
            Self::NoSession | Self::Invalidated { .. } => None,
        }
    }

    /// Which timeline this health is about, if a session is running.
    ///
    /// A stop-and-restart produces a DIFFERENT generation over the same world,
    /// which is the only way a consumer can tell "the session I started" from
    /// "a session started since" — both report frame numbers from zero.
    pub fn generation(&self) -> Option<u64> {
        match self {
            Self::Healthy { generation, .. } | Self::Desynced { generation, .. } => {
                Some(*generation)
            }
            Self::NoSession | Self::Invalidated { .. } => None,
        }
    }
}

/// Ask a rollback host how its session is doing.
///
/// Cheap enough to call every update. See [`RollbackHealth`] for why a
/// consumer needs it and what [`RollbackSession`] does not tell you.
///
/// ⚠ **The LIVE session decides first, and it did not used to.** `health` read
/// `RollbackSessionStatus` and `RollbackFrameCount`, both of which are read
/// models that OUTLIVE the session that wrote them — teardown removes the
/// session, its ownership and the confirmed-frame boundary, and leaves those
/// two behind. So a stopped host reported `Healthy { frame }` at whatever frame
/// it had reached, forever, from a public API (GPT 5.6, 2026-07-31). The frame
/// even looked plausible: it was the last real one.
///
/// What survives teardown deliberately is the DIAGNOSIS. A session that
/// desynced and was then stopped still reports [`RollbackHealth::Invalidated`]
/// carrying why, because a divergence that disappears when the timeline is torn
/// down is exactly the laundering `RollbackSessionStatus::carried_from` exists
/// to prevent — the reason is the part a reader acts on, and it is the same
/// prose the NEXT session would inherit.
pub fn health(app: &App) -> RollbackHealth {
    use ambition_platformer2d_runtime::rollback::{RollbackFrameCount, RollbackSessionStatus};

    let world = app.world();
    let status = world.get_resource::<RollbackSessionStatus>();
    if !ambition_platformer2d_runtime::rollback::session_is_active(world) {
        // No live session: the only honest report is what is left to say about
        // the timeline that ended, which is its diagnosis or nothing.
        return match RollbackSessionStatus::carried_from(status).invalidation {
            Some(reason) => RollbackHealth::Invalidated { reason },
            None => RollbackHealth::NoSession,
        };
    }
    let Some(status) = status else {
        return RollbackHealth::NoSession;
    };
    if let Some(reason) = &status.invalidation {
        return RollbackHealth::Invalidated {
            reason: reason.clone(),
        };
    }
    let frame = world
        .get_resource::<RollbackFrameCount>()
        .map(|count| count.0)
        .unwrap_or(0);
    // The generation is stamped on the boundary the session install writes, so
    // it is present exactly while a session is.
    let generation = world
        .get_resource::<ambition_platformer2d_core::confirmed_frame::ConfirmedFrameBoundary>()
        .map(|boundary| boundary.session)
        .unwrap_or(0);
    if status.mismatch_frames.is_empty() {
        RollbackHealth::Healthy { frame, generation }
    } else {
        RollbackHealth::Desynced {
            frames: status.mismatch_frames.clone(),
            frame,
            generation,
        }
    }
}

/// Stop the rollback session this host is running.
///
/// The other half of [`start`], and it exists because a consumer that can start
/// a session and cannot stop one has no way to observe the lifecycle it is
/// being promised: [`health`] reporting [`RollbackHealth::NoSession`] after a
/// stop, and a restart being a NEW generation over the same world, are both
/// facts about teardown.
///
/// Teardown takes the session, its ownership, the confirmed-frame boundary and
/// the input-authority latch — the latch because it holds edges captured
/// against a timeline that no longer exists, and leaving it installed lets the
/// next session's frame zero begin with a jump nobody pressed in it.
///
/// Calling this on a host with no session is a no-op, not an error.
pub fn stop(app: &mut App) {
    ambition_platformer2d_runtime::rollback::stop_session(app.world_mut());
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
        .get_resource::<ambition_platformer2d_runtime::SimulationHost>()
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

    // ⛔ **the world must EXIST before the session does.** See
    // [`RollbackRefused::NoSessionWorld`]: a rollback session opened over an
    // unbuilt world compares activation rather than gameplay, and the checksum
    // mismatch that produces reads as a desync in the game.
    if ambition_platformer2d_shared_tangle::lifecycle::session_world_entity(app.world()).is_none() {
        return Err(RollbackRefused::NoSessionWorld {
            ticks: ticks_to_activation + plan.settle_ticks,
        });
    }

    let encoded_types = app
        .world()
        .get_resource::<RollbackRegistry>()
        .map(|registry| registry.descriptors().count())
        .unwrap_or(0);
    if encoded_types == 0 {
        return Err(RollbackRefused::NoAuthoritativeState);
    }

    let settings = ambition_platformer2d_runtime::rollback::SyncTestSettings {
        check_distance: plan.check_distance,
        max_prediction_window: plan.prediction_window,
        ..ambition_platformer2d_runtime::rollback::SyncTestSettings::for_players(participants)
    };
    // ⚠ The EFFECTIVE count, not the requested one. `player_count` clamps into
    // what a session can build, and reporting the request back would let this
    // struct describe a topology GGRS did not seat. `PlatformerApp::rollback`
    // refuses out-of-range counts so the clamp should now be a no-op — reading
    // it here is what keeps that true rather than assumed.
    let participants = settings.player_count();
    ambition_platformer2d_runtime::rollback::start_sync_test_session(app.world_mut(), settings)
        .map_err(|error| RollbackRefused::SessionRejected(error.to_string()))?;
    app.update();

    Ok(RollbackSession {
        ticks_to_activation,
        participants,
        encoded_types,
    })
}
