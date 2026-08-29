//! GGRS session/input bridge shared by the harness and future network hosts.

use bevy::prelude::*;
use bevy_ggrs::ggrs::{self, PlayerType, SessionBuilder};
use bevy_ggrs::{
    ConfirmedFrameCount, GgrsConfig, GgrsSchedule, GgrsTime, LoadWorld, LocalInputs, LocalPlayers,
    PlayerInputs, ReadInputs, RollbackFrameCount, RunGgrsSystems, Session, SyncTestMismatch,
};

use ambition_platformer2d_core::{ConfirmedFrameBoundary, ControlFrame};

use super::RollbackRegistry;
use crate::{PreparedContentIdentity, SnapshotSchemaFingerprint};

pub type AmbitionGgrsConfig = GgrsConfig<ControlFrame>;
pub type AmbitionGgrsSession = Session<AmbitionGgrsConfig>;

#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum AmbitionReadInputsSet {
    CaptureDeviceLatch,
    PublishLocalInputs,
}

/// Has the wrong-seam diagnostic fired this run?
///
///  a finding, not just a log line. The check below could have warned and
/// nothing else, and its first test did what a log-only diagnostic forces a test
/// to do: re-derive the predicate over the same resources and assert on THAT —
/// which passes just as happily when the system is never registered. Publishing
/// the answer makes the SYSTEM the thing under test, and lets a consumer or a
/// harness ask the question without scraping stderr.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputSeamMisuse(pub bool);

/// EXTERNAL INPUT WAITING TO BE SUBMITTED TO GGRS, ONE FRAME PER HANDLE.
///
/// Intentionally not rollback state: prediction and session logic own the input
/// stream, while simulation state is rewound beneath it.
///
///  one per handle, because publishing seat zero's frame to all of them —
/// which is what `publish_local_inputs` did, back when there was only ever one —
/// makes four pads move one fighter and checksum-compare a lie.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct PendingSeatInputs {
    seats: [ControlFrame; ambition_characters::control::SlotControls::MAX_SLOTS],
}

impl PendingSeatInputs {
    pub fn get(&self, handle: usize) -> ControlFrame {
        self.seats.get(handle).copied().unwrap_or_default()
    }

    pub fn set(&mut self, handle: usize, frame: ControlFrame) {
        if let Some(seat) = self.seats.get_mut(handle) {
            *seat = frame;
        }
    }
}

/// Counts GGRS execution outside rollback state.
///
/// Unprefixed counters describe the current session and reset on rebase;
/// `lifetime_*` counters span every session installed during the process run.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RollbackExecutionStats {
    pub advance_runs: u64,
    pub load_runs: u64,
    /// The frame of the most recent advance, replay or not.
    pub last_simulated_frame: i32,
    /// High-water mark across every advance. A frame at or below it is being
    /// re-simulated, which is how [`count_advance_run`] tells a replay pass from
    /// a first-time one. `None` until the first advance, so frame 0 is not
    /// mistaken for a replay of itself.
    pub highest_simulated_frame: Option<i32>,
    /// Advances across every session this process has installed.
    pub lifetime_advance_runs: u64,
    /// Loads across every session this process has installed.
    pub lifetime_load_runs: u64,
    /// How many sessions have been installed. `1` for a run that never rebased,
    /// so `sessions_installed > 1` is exactly "the counters above were reset
    /// under you".
    pub sessions_installed: u64,
}

impl RollbackExecutionStats {
    /// The stats a freshly installed session starts from: per-session counters
    /// zeroed, lifetime totals carried through untouched.
    ///
    ///  carried, not folded. The lifetime totals are accumulated by the
    /// same systems that accumulate the per-session ones, so they are correct
    /// on every frame rather than only just after a rebase. Adding the outgoing
    /// session's counts here — which is the obvious reading of "carry forward",
    /// and what this did first — double-counts every session. A teardown with no
    /// following install would also lose its work under a fold-at-install rule,
    /// and those are exactly the runs worth measuring.
    fn rebased(self) -> Self {
        Self {
            advance_runs: 0,
            load_runs: 0,
            last_simulated_frame: 0,
            highest_simulated_frame: None,
            sessions_installed: self.sessions_installed + 1,
            ..self
        }
    }
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct RollbackSessionStatus {
    pub mismatch_frames: Vec<i32>,
    pub invalidation: Option<String>,
}

impl RollbackSessionStatus {
    /// Whether this timeline may still authorize confirmed host-side effects.
    ///
    /// Keeping the predicate on the status itself gives every host-side gate the same answer
    /// instead of re-deriving a subtly different idea of "healthy".
    pub fn is_healthy(&self) -> bool {
        self.invalidation.is_none() && self.mismatch_frames.is_empty()
    }

    /// The status a NEW session starts from, given the outgoing one. (AC23)
    ///
    /// So the diagnostic CARRIES. An unhealthy timeline hands its reason to the
    /// timeline that replaces it, and the only way to clear it is to say so
    /// (`acknowledge_and_clear`).
    ///
    ///  `mismatch_frames` does NOT carry, and that is not an oversight: frame
    /// numbers restart at zero for every GGRS session, so carrying them forward
    /// would report a mismatch at frames the new timeline has not reached yet.
    /// The reason survives as prose, which is the part a reader acts on.
    pub fn carried_from(previous: Option<&Self>) -> Self {
        let Some(previous) = previous else {
            return Self::default();
        };
        let inherited = previous.invalidation.clone().or_else(|| {
            (!previous.mismatch_frames.is_empty()).then(|| {
                format!(
                    "GGRS sync-test checksum mismatch at frames {:?} on the PREVIOUS timeline",
                    previous.mismatch_frames
                )
            })
        });
        Self {
            mismatch_frames: Vec::new(),
            invalidation: inherited,
        }
    }

    /// Clear an inherited diagnostic DELIBERATELY.
    ///
    /// The escape hatch, named for what it is. A tool that has shown the
    /// divergence to a human and been told to carry on calls this; nothing on
    /// the ordinary install path does.
    pub fn acknowledge_and_clear(&mut self) {
        self.mismatch_frames.clear();
        self.invalidation = None;
    }
}

/// Monotonic identity for rollback timelines.
///
/// This resource deliberately survives session teardown. Frame numbers restart
/// at zero for every GGRS session, so deriving a generation from the optional
/// [`ConfirmedFrameBoundary`] aliases a stopped-and-restarted session with the
/// one that preceded it. Host-side journals and traces use this generation to
/// discard work from timelines that no longer exist.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RollbackSessionGeneration(u64);

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct RollbackSessionContract {
    pub content: Option<PreparedContentIdentity>,
    pub schema: SnapshotSchemaFingerprint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncTestSettings {
    pub check_distance: usize,
    pub max_prediction_window: usize,
    /// Number of local input streams in the sync-test session.
    /// Callers derive this from the session's frozen seating/topology rather than
    /// resampling connected devices.
    pub players: usize,
}

impl SyncTestSettings {
    /// The player count clamped to what the session can actually build: at
    /// least one, at most the controller slots the game supports.
    ///
    /// Clamped rather than asserted because this is settings data that reaches
    /// the builder from a dev tool and a harness option, and a session that
    /// refuses to start is worse than one that starts with a sane count.
    pub fn player_count(&self) -> usize {
        self.players
            .clamp(1, ambition_characters::control::SlotControls::MAX_SLOTS)
    }
}

/// Who owns the currently installed GGRS session.
///
/// Local sync-test sessions may be stopped and recreated around a developer
/// content reload. External/P2P sessions require a coordinated peer barrier and
/// must never be replaced unilaterally by the local host.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollbackSessionOwnership {
    LocalSyncTest {
        settings: SyncTestSettings,
        /// Starter that owns this local sync-test session.
        owner: SyncTestOwner,
    },
    External,
}

/// Which starter owns a live sync-test session.
/// The local maintainer may rebuild only sessions it started; caller-owned and
/// external sessions are left alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncTestOwner {
    /// `maintain_local_session` started it and may stop or rebuild it.
    LocalMaintainer,
    /// Somebody else did — match activation, a dev tool, a test harness. The
    /// maintainer inspects and steps aside, exactly as it does for `External`.
    Caller,
}

/// Construct standard sync-test tuning for an explicit player count.
/// Player count is topology, not tuning, so it is required instead of defaulted.
impl SyncTestSettings {
    pub fn for_players(players: usize) -> Self {
        Self {
            check_distance: 7,
            max_prediction_window: 12,
            players,
        }
    }
}

pub fn start_sync_test_session(
    world: &mut World,
    settings: SyncTestSettings,
) -> Result<(), ggrs::GgrsError> {
    start_sync_test_session_owned(world, settings, SyncTestOwner::Caller)
}

/// [`start_sync_test_session`], declaring WHO owns the result.
///
///  the owner is an argument, not a follow-up call. Stamping ownership
/// after the session exists would leave a window where the maintainer's own
/// session looks like somebody else's — and "an authority that needs a second
/// call" is the shape this repo has been bitten by before.
pub fn start_sync_test_session_owned(
    world: &mut World,
    settings: SyncTestSettings,
    owner: SyncTestOwner,
) -> Result<(), ggrs::GgrsError> {
    // The ONLY fallible step — pure GGRS construction, touches no world — runs first.
    let session = build_sync_test_session(settings)?;
    install_rebased_sync_test_session(world, session, settings, owner);
    Ok(())
}

/// Construct the replacement sync-test session WITHOUT touching the world.
///
/// Pair it with [`install_rebased_sync_test_session`].
pub fn build_sync_test_session(
    settings: SyncTestSettings,
) -> Result<AmbitionGgrsSession, ggrs::GgrsError> {
    let players = settings.player_count();
    let mut builder = SessionBuilder::<AmbitionGgrsConfig>::new()
        .with_num_players(players)?
        .with_fps(ambition_platformer2d_runtime::SIM_TICK_HZ as usize)?
        .with_max_prediction_window(settings.max_prediction_window)
        .with_check_distance(settings.check_distance);
    for handle in 0..players {
        builder = builder.add_player(PlayerType::Local, handle)?;
    }
    let session = builder.start_synctest_session()?;
    Ok(AmbitionGgrsSession::SyncTest(session))
}

/// Install an already-built sync-test session as the new frame-zero baseline.
///
/// Installation only mutates world state and cannot fail. Rebase resets frame
/// counters and `Time<GgrsTime>` before installing the session.
///
/// Warn when frame zero has no constructed session world: construction via
/// `Commands` after session start cannot be undone by rollback, so those frames
/// will checksum-mismatch on resimulation. Empty-world fixtures remain allowed.
fn warn_if_no_world_to_rewind(world: &World) {
    if has_session_world_root(world) {
        return;
    }
    bevy::log::warn!(
        target: "ambition_platformer2d::rollback",
        "starting a rollback session with no session world: frame zero is an          EMPTY world, so the construction that runs next happens inside the          rollback window. A rollback cannot undo `Commands`, so the frames that          build the room will mismatch on every resimulation and GGRS will report          it only as a checksum difference. Activate the session world first, then          start the session — it rebases onto whatever is live."
    );
}

/// Whether a gameplay session world has been constructed and is readable.
///
///  `session_world_entity` is `None` for a bare fixture too, which is the
/// same correct "no world" answer the `try_query` fallback gave, so nothing that
/// legitimately runs without a session starts warning.
fn has_session_world_root(world: &World) -> bool {
    ambition_platformer2d_shared_tangle::lifecycle::session_world_entity(world).is_some()
}

/// Replace the WHOLE input-authority cluster, atomically.
///
/// `SlotControlLatches` was the one being preserved, which is the half a
/// single-player test could never notice.
///
/// Each is reset only if the composition installed it: inserting one here would
/// make this function a second authority on which latches exist.
fn reset_input_authority(world: &mut World) {
    world.insert_resource(PendingSeatInputs::default());
    // ⭐ ONE table. This used to reset seat zero's latch and the other seats'
    // separately, which is exactly how "preserved some, cleared the rest" became
    // possible in the first place.
    if world.contains_resource::<ambition_characters::control::SlotControlLatches>() {
        world.insert_resource(ambition_characters::control::SlotControlLatches::default());
    }
}

pub fn install_rebased_sync_test_session(
    world: &mut World,
    session: AmbitionGgrsSession,
    settings: SyncTestSettings,
    // Declared by the caller for the same reason as `start_sync_test_session_owned`:
    // a rebase keeps its owner, and inferring one here would guess.
    owner: SyncTestOwner,
) {
    warn_if_no_world_to_rewind(world);
    // A newly installed GGRS session always starts from the current live world
    // as frame zero. Snapshot stores are intentionally retained here: the first
    // SaveWorld request at frame zero replaces every non-negative frame in each
    // bevy_ggrs ring, while resetting these frame resources prevents that save
    // from being mislabeled with the previous session's frame number.
    world.insert_resource(RollbackFrameCount(0));
    world.insert_resource(ConfirmedFrameCount(-1));
    reset_input_authority(world);

    // GgrsTimePlugin derives deterministic elapsed time from RollbackFrameCount by calling
    // Time::advance_to.
    world.insert_resource(Time::<GgrsTime>::new_with(GgrsTime));

    install_session_with_ownership(
        world,
        session,
        RollbackSessionOwnership::LocalSyncTest { settings, owner },
    );
}

/// Install any already-constructed GGRS session behind Ambition's exact
/// content/schema contract. Matchbox will eventually construct a P2P session
/// and hand it to this same seam; the harness uses [`start_sync_test_session`].
pub fn install_session(world: &mut World, session: AmbitionGgrsSession) {
    install_session_with_ownership(world, session, RollbackSessionOwnership::External);
}

fn install_session_with_ownership(
    world: &mut World,
    session: AmbitionGgrsSession,
    ownership: RollbackSessionOwnership,
) {
    let schema = world
        .get_resource::<RollbackRegistry>()
        .cloned()
        .unwrap_or_default()
        .schema_fingerprint();
    let content = live_content_identity(world);
    world.insert_resource(RollbackSessionContract { content, schema });
    // AC23: an unhealthy timeline hands its reason to the one replacing it. A
    // session install must never LAUNDER a divergence into a clean baseline, and
    // making that the seam's behaviour is what stops it depending on each caller
    // remembering — one of the four remembered.
    let carried_status =
        RollbackSessionStatus::carried_from(world.get_resource::<RollbackSessionStatus>());
    let confirmation = if carried_status.is_healthy() {
        ambition_platformer2d_runtime::RollbackConfirmationState::Healthy
    } else {
        ambition_platformer2d_runtime::RollbackConfirmationState::Unhealthy
    };
    world.insert_resource(carried_status);
    world.insert_resource(confirmation);
    // Per-session counters restart; lifetime totals do not. A caller measuring
    // a whole run must not have its measurement silently zeroed by a rebase it
    // did not ask for and cannot see (AC18).
    let carried = world
        .get_resource::<RollbackExecutionStats>()
        .copied()
        .unwrap_or_default();
    world.insert_resource(carried.rebased());
    world.insert_resource(ownership);
    world.insert_resource(session);

    // A new session is a new timeline. The counter lives independently of the
    // boundary because teardown removes the boundary; deriving from that optional
    // resource would make every stop/restart cycle reuse generation zero.
    let generation = {
        let mut generation =
            world.get_resource_or_insert_with::<RollbackSessionGeneration>(Default::default);
        generation.0 = generation.0.wrapping_add(1);
        generation.0
    };
    world.insert_resource(ConfirmedFrameBoundary {
        current: 0,
        confirmed: -1,
        session: generation,
    });
}

/// Remove every resource whose presence means a rollback session is active.
///
/// The generation counter intentionally survives: the next installation must
/// receive a different identity even after the boundary itself is removed.
pub fn stop_session(world: &mut World) {
    // The input-authority cluster leaves WITH the session it belonged to.
    //
    // A latch holds levels and edges captured against the timeline that is being
    // torn down. Leaving them installed means the next session's frame zero can
    // begin with a jump nobody pressed in it.
    reset_input_authority(world);
    world.remove_resource::<AmbitionGgrsSession>();
    world.remove_resource::<RollbackSessionContract>();
    world.remove_resource::<RollbackSessionOwnership>();
    // Nothing speculates any more, so external effects and persistence return
    // to their non-rollback behavior immediately. Leaving this installed would
    // strand pending effects and keep confirmed-state save gates closed forever.
    world.remove_resource::<ConfirmedFrameBoundary>();
    world.insert_resource(ambition_platformer2d_runtime::RollbackConfirmationState::Unavailable);
}

/// Queue the exact same teardown from a regular Bevy system.
pub fn stop_session_deferred(commands: &mut Commands) {
    commands.queue(|world: &mut World| stop_session(world));
}

/// Return a diagnostic error when GGRS invalidated the session contract or a
/// sync-test checksum mismatch was observed.
pub fn session_health(world: &World) -> Result<(), String> {
    let Some(status) = world.get_resource::<RollbackSessionStatus>() else {
        return Ok(());
    };
    if status.is_healthy() {
        return Ok(());
    }
    if let Some(reason) = &status.invalidation {
        return Err(reason.clone());
    }
    Err(format!(
        "GGRS sync-test checksum mismatch at frames {:?}",
        status.mismatch_frames
    ))
}

pub fn session_is_active(world: &World) -> bool {
    world.contains_resource::<AmbitionGgrsSession>()
}

/// THE seam a driver writes input through, whichever host is running.
///
/// A driver is anything supplying input that is not a device: a headless
/// harness, an RL agent, a replay, an integration test, a consumer's acceptance
/// walk. There are two resources underneath and picking the wrong one FAILS
/// SILENTLY — the walk runs, the body never moves, nothing says why — so this
/// exists to make the choice unnecessary rather than merely documented.
///
/// The split is not an accident and cannot be merged away. Under GGRS it is an OUTPUT:
/// `publish_ggrs_input` writes it from the session's confirmed inputs every advance, so a
/// driver writing it would be feeding resimulated input back in as new input. Handle zero of
/// `PendingSeatInputs` is the input side there.
///
/// A device-backed host writes neither: it accumulates into
/// [`ControlFrameLatch`], which both hosts drain at their own clock. If a latch
/// is present this defers to it, so a driver can nudge a windowed build without
/// fighting the device layer.
///
/// Found while giving the external-consumer fixture a rollback host: it had to
/// carry its own copy of this branch, which is the definition of a leak — every
/// consumer rediscovering an engine rule the engine could have stated once.
pub fn drive_control_frame(world: &mut World, frame: ControlFrame) {
    drive_slot_frame(
        world,
        ambition_characters::control::PlayerSlot::PRIMARY,
        frame,
    );
}

///  this was TWO functions with the same four-arm shape, differing only in
/// which resource each arm named — and the resources they named have since
/// become one table each (`SlotControlLatches`, `PendingSeatInputs`). What is
/// left of the fork is the last arm.
///
/// it accepts every slot, and the version that did not was a bug. `drive_seat_frame` refused
/// slot zero with a bare `return`, on the argument that the primary seat belonged to
/// [`drive_control_frame`].
pub fn drive_slot_frame(
    world: &mut World,
    slot: ambition_characters::control::PlayerSlot,
    frame: ControlFrame,
) {
    // A device-backed host: fold into the seat's latch so a sub-tick press
    // survives to the tick that drains it.
    if let Some(mut latches) =
        world.get_resource_mut::<ambition_characters::control::SlotControlLatches>()
    {
        latches.accumulate(slot, frame);
        return;
    }
    //  this does NOT clear the other handles, and an earlier version did.
    // `drive_slot_frame` is called BEFORE the step it applies to, so clearing
    // here wiped every other seat's input on the way past — the seam was built
    // and then emptied by its own sibling, one line later. A driver that wants a
    // seat neutral drives it neutral; silence is not a request.
    if let Some(mut pending) = world.get_resource_mut::<PendingSeatInputs>() {
        pending.set(slot.0 as usize, frame);
        return;
    }
    // It is that seat's output mirror now, so writing it would deliver a press to nobody — the
    // silent no-op this whole seam exists to prevent.
    //
    //  BOTH surfaces, and that is the helper's whole contract. A driver
    // says *this seat is holding this frame* and must not have to know how the
    // composition was assembled. The RAW row is what a shaping stage reads — a
    // scripted reset has to reach the reset stage, a scripted stick the portal
    // warp — and a composition that installs the shaping stages will overwrite
    // the slot below with the shaped result anyway. A composition that installs
    // NONE of them (the smallest headless fixture) has no commit either, so
    // without the second write its press would sit in a table nothing drains.
    if let Some(mut raw) = world.get_resource_mut::<ambition_characters::control::SeatRawFrames>() {
        raw.set(slot, frame);
    }
    if let Some(mut slots) = world.get_resource_mut::<ambition_characters::control::SlotControls>()
    {
        slots.set(slot, frame);
    }
}

pub(crate) fn install_session_bridge(app: &mut App) {
    // Only a speculating host quarantines external effects, so the whole
    // mechanism is installed HERE rather than in the engine group: a fixed-tick
    // or render-frame game carries none of these systems at all.
    ambition_platformer2d_runtime::external_effects::quarantine_presentation_effects(
        app, LoadWorld,
    );

    // See `local_session`.
    app.init_resource::<super::local_session::LocalSessionPolicy>()
        .init_resource::<super::local_session::LocalSessionOwnership>()
        // Present from boot so "where do this session's seats come from" always
        // has an answer to read and an owner to release. The default —
        // `Devices` — is what every composition that never decides a match
        // wants.
        .init_resource::<ambition_input::SessionSeatingSource>()
        .add_systems(
            Update,
            super::local_session::maintain_local_session
                .in_set(super::local_session::LocalSessionSet::Maintain),
        )
        // Both are in `Update` and nothing ordered them, so which authority sized the ggrs
        // session was a race — and it resolved DIFFERENTLY on the two shipped routes: measured,
        // versus took the pad count and smash took the roster's. The session is never resized
        // afterwards (see the note in `maintain_local_session` for why detect-and-restart is
        // worse), so whichever won, won for the whole match.
        //
        //  same schedule, so this is a REAL edge. A cross-schedule `.after`
        // is silently vacuous in Bevy and this repo has been bitten by one; both
        // sets live in `Update`, which is what makes the constraint bite.
        .configure_sets(
            Update,
            super::local_session::LocalSessionSet::Maintain
                .after(ambition_input::InputSet::Collect),
        );

    app.add_systems(Update, report_input_written_to_the_wrong_seam);
    app.init_resource::<InputSeamMisuse>()
        .init_resource::<PendingSeatInputs>()
        .init_resource::<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>()
        .init_resource::<RollbackExecutionStats>()
        .init_resource::<RollbackSessionStatus>()
        .configure_sets(
            ReadInputs,
            (
                AmbitionReadInputsSet::CaptureDeviceLatch,
                AmbitionReadInputsSet::PublishLocalInputs,
            )
                .chain(),
        )
        .add_systems(
            ReadInputs,
            capture_latched_local_input.in_set(AmbitionReadInputsSet::CaptureDeviceLatch),
        )
        .add_systems(
            ReadInputs,
            publish_local_inputs.in_set(AmbitionReadInputsSet::PublishLocalInputs),
        )
        .add_systems(
            GgrsSchedule,
            (publish_ggrs_input, count_advance_run)
                .chain()
                .before(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::CoreSimulation),
        )
        .add_systems(
            LoadWorld,
            (
                // Publishes the restored frame, which the abandoned-branch
                // discard reads. The edge is required, not incidental.
                mark_historical_replay
                    .before(ambition_platformer2d_runtime::external_effects::ExternalEffectSet::DiscardAbandoned),
                count_load_run.in_set(super::AmbitionLoadWorldSet::Reconcile),
            ),
        )
        .add_systems(
            PreUpdate,
            (
                enforce_session_contract.before(RunGgrsSystems),
                clear_historical_replay.after(RunGgrsSystems),
                // Track B: execute a confirmed deferred lifecycle op in the exclusive world and
                // rebase, after the advance batch is done.
                crate::lifecycle_commit::commit_confirmed_lifecycle
                    .after(RunGgrsSystems)
                    .after(clear_historical_replay)
                    .after(ambition_platformer2d_runtime::external_effects::ExternalEffectSet::Release),
            ),
        )
        // Effects may only be released once this render frame's advances are
        // done. Without this edge Bevy is free to release first, and the next
        // advance's outbox clear then wipes what was just handed to
        // presentation — silently, since the journal has already counted it.
        .configure_sets(
            PreUpdate,
            ambition_platformer2d_runtime::external_effects::ExternalEffectSet::Release.after(RunGgrsSystems),
        )
        .add_observer(record_sync_test_mismatch);
}

/// Consume device input only when GGRS actually asks for the next local
/// input. Draining the latch once per rendered frame is incorrect: several
/// rendered frames may pass before a simulation tick, and a later level-only
/// sample would overwrite a short press before GGRS observed it.
fn capture_latched_local_input(
    //  ONE table for every seat, zero included. This took seat zero's latch
    // as a separate resource beside this one and drained the two in separate
    // blocks — the same edge, the same reason, twice.
    latches: Option<ResMut<ambition_characters::control::SlotControlLatches>>,
    mut pending: ResMut<PendingSeatInputs>,
) {
    // ONLY when a device is actually wired to this latch.
    //
    // The predicate is STICKY rather than per-frame: a tick that sampled
    // nothing must still receive the retained levels, or a held direction
    // sticks on forever.
    let Some(mut latches) = latches else {
        return;
    };
    let primary = ambition_characters::control::PlayerSlot::PRIMARY;
    if latches.is_device_authority(primary) {
        pending.set(0, latches.take(primary));
    }
    //  seats 1.. are drained UNCONDITIONALLY, and seat zero is not. Only
    // seat zero has a second author to lose to — every rollback harness drives
    // `PendingLocalInput` directly, and replacing that with a neutral default is
    // how four oracles went red at once. Nothing drives `PendingSeatInputs`
    // behind this system's back.
    for handle in 1..ambition_characters::control::SlotControls::MAX_SLOTS {
        let slot = ambition_characters::control::PlayerSlot(handle as u8);
        pending.set(handle, latches.take(slot));
    }
}

/// Every handle is one row of [`PendingSeatInputs`], latched by the device layer
/// and drained when GGRS asks. A handle nobody feeds reads neutral, exactly as a
/// pad nobody plugged in should.
fn publish_local_inputs(
    pending: Res<PendingSeatInputs>,
    local_players: Res<LocalPlayers>,
    mut commands: Commands,
) {
    //  no `handle == 0` branch: one table answers for every handle.
    let inputs = local_players
        .0
        .iter()
        .map(|&handle| (handle, pending.get(handle)))
        .collect();
    commands.insert_resource(LocalInputs::<AmbitionGgrsConfig>(inputs));
}

/// Publish the session's confirmed inputs into what the simulation reads.
///
///  this is what puts seats 1.. INSIDE rollback.
///
/// Every seat lands in one table now, and `ControlFrame` is written after the loop as what it
/// has become: a MIRROR of what seat zero received, for the trace codec, the harness's action
/// encoder, and the wrong-seam diagnostic.
fn publish_ggrs_input(
    inputs: Res<PlayerInputs<AmbitionGgrsConfig>>,
    mut control: ResMut<ControlFrame>,
    mut slots: Option<ResMut<ambition_characters::control::SlotControls>>,
) {
    for (handle, (input, _)) in inputs.iter().enumerate() {
        if let Some(slots) = slots.as_deref_mut() {
            slots.set(
                ambition_characters::control::PlayerSlot(handle as u8),
                *input,
            );
        }
    }
    //  from the table, not from `inputs[0]` — so the mirror cannot disagree
    // with the seat, including in the empty-session case where nobody published
    // anything and neutral is the honest answer.
    *control = slots
        .as_deref()
        .map(|slots| slots.get(ambition_characters::control::PlayerSlot::PRIMARY))
        .filter(|_| !inputs.is_empty())
        .unwrap_or_default();
}

/// Publish the FACT "this frame number has been simulated before".
///
/// Deliberately a fact, not a policy — but note how few consumers it has left. They now go
/// through [`ambition_platformer2d_runtime::external_effects`], which defers rather than
/// suppresses.
///
/// What remains are consumers that genuinely need to know a frame is being
/// revisited: the forensic trace uses it to avoid consuming per-logical-frame
/// suppression windows twice, and the falling-sand grid uses it as a step guard.
fn publish_replay_pass(
    replay: &mut ambition_platformer2d_shared_tangle::schedule::SimulationReplayState,
    simulated_before: bool,
) {
    replay.replaying_history = simulated_before;
}

/// Decide, per advance, whether GGRS is re-simulating a frame it already ran,
/// and publish where the confirmed boundary sits.
///
/// The frame number is the exact test for the first: at or below the high-water
/// mark means this frame was simulated before. Bracketing on "a rollback
/// happened this render frame" is NOT equivalent — `clear_historical_replay`
/// runs after the whole GGRS batch, so the coarse window also covers the
/// brand-new frame at the end of a rollback.
///
/// [`ConfirmedFrameBoundary`] is the separate, stronger fact: which frames can
/// never be simulated again. It is derived from the live session rather than
/// read off `ConfirmedFrameCount` — see [`confirmed_line`] for why.
fn count_advance_run(
    frame: Res<RollbackFrameCount>,
    confirmed: Option<Res<ConfirmedFrameCount>>,
    session: Option<Res<AmbitionGgrsSession>>,
    mut stats: ResMut<RollbackExecutionStats>,
    mut replay: ResMut<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>,
    boundary: Option<ResMut<ConfirmedFrameBoundary>>,
) {
    stats.advance_runs = stats.advance_runs.saturating_add(1);
    stats.lifetime_advance_runs = stats.lifetime_advance_runs.saturating_add(1);
    stats.last_simulated_frame = frame.0;
    let simulated_before = stats
        .highest_simulated_frame
        .is_some_and(|highest| frame.0 <= highest);
    stats.highest_simulated_frame = Some(
        stats
            .highest_simulated_frame
            .map_or(frame.0, |highest| highest.max(frame.0)),
    );
    publish_replay_pass(&mut replay, simulated_before);
    if let Some(mut boundary) = boundary {
        boundary.current = frame.0;
        boundary.confirmed = confirmed_line(
            frame.0,
            session.as_deref(),
            confirmed.map(|confirmed| confirmed.0),
        );
    }
}

/// Where the confirmed line sits DURING the advance of `frame`.
///
/// ⛔⛔ NOT `ConfirmedFrameCount`, and that resource is why this function
/// exists. `bevy_ggrs` computes it from the frame counter it reads BEFORE
/// bumping it, so inside `AdvanceWorld` it always describes the PREVIOUS frame.
/// Under the shipped local session -- a sync test with `check_distance: 0`,
/// where rollback is dormant and NOTHING is ever speculative -- that published
/// `confirmed == current - 1` on every frame the game ever ran, so
/// `fully_confirmed()` was false forever and every consumer waiting for settled
/// truth stood down silently: the winner card, the return to character select,
/// and the persistence save. A match ended and the stage never went home.
///
/// ⭐ The session is the authority, asked about the frame being advanced. This
/// mirrors `bevy_ggrs`'s own rule; only the frame it is applied to differs.
fn confirmed_line(
    frame: i32,
    session: Option<&AmbitionGgrsSession>,
    published: Option<i32>,
) -> i32 {
    match session {
        // A sync test re-simulates the last `check_distance` frames, so
        // everything older than that window can never be simulated again. At
        // zero the window is empty and the frame just advanced is already final.
        Some(Session::SyncTest(session)) => frame.saturating_sub(session.check_distance() as i32),
        // Online, confirmation is a network fact and nothing local may widen it.
        Some(Session::P2P(session)) => session.confirmed_frame(),
        // A spectator only ever receives frames that are already confirmed.
        Some(Session::Spectator(_)) => frame,
        // No session in the world: a hand-built harness. Believe what it
        // published, and `-1` (GGRS's own convention) when it published nothing.
        None => published.unwrap_or(-1),
    }
    .max(-1)
}

/// `LoadWorld`: the host has restored `frame`, so the simulation now sits
/// there. Republishing it is what lets `discard_abandoned_predictions` drop the
/// branch that was just walked away from without naming a GGRS type.
fn mark_historical_replay(
    frame: Res<RollbackFrameCount>,
    mut replay: ResMut<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>,
    boundary: Option<ResMut<ConfirmedFrameBoundary>>,
) {
    publish_replay_pass(&mut replay, true);
    if let Some(mut boundary) = boundary {
        boundary.current = frame.0;
    }
}

fn clear_historical_replay(
    mut replay: ResMut<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>,
) {
    publish_replay_pass(&mut replay, false);
}

fn count_load_run(mut stats: ResMut<RollbackExecutionStats>) {
    stats.load_runs = stats.load_runs.saturating_add(1);
    stats.lifetime_load_runs = stats.lifetime_load_runs.saturating_add(1);
}

fn record_sync_test_mismatch(
    trigger: On<SyncTestMismatch>,
    mut status: ResMut<RollbackSessionStatus>,
    mut confirmation: ResMut<ambition_platformer2d_runtime::RollbackConfirmationState>,
) {
    status
        .mismatch_frames
        .extend(trigger.event().mismatched_frames.iter().copied());
    *confirmation = ambition_platformer2d_runtime::RollbackConfirmationState::Unhealthy;
}

fn enforce_session_contract(world: &mut World) {
    if !session_is_active(world) {
        return;
    }

    let current_schema = world
        .get_resource::<RollbackRegistry>()
        .cloned()
        .unwrap_or_default()
        .schema_fingerprint();
    let current_content = live_content_identity(world);

    let Some(contract) = world.get_resource::<RollbackSessionContract>().cloned() else {
        world.insert_resource(RollbackSessionContract {
            content: current_content,
            schema: current_schema,
        });
        return;
    };

    if contract.schema != current_schema {
        invalidate_session(
            world,
            format!(
                "GGRS rollback schema changed while the session was active: expected {}, observed {}",
                contract.schema, current_schema
            ),
        );
        return;
    }

    match (contract.content, current_content) {
        (None, Some(identity)) => {
            world.resource_mut::<RollbackSessionContract>().content = Some(identity);
        }
        (Some(expected), Some(observed)) if expected != observed => {
            invalidate_session(
                world,
                format!(
                    "prepared content changed while the GGRS session was active: expected {:?}, observed {:?}",
                    expected, observed
                ),
            );
        }
        (Some(expected), None) => {
            invalidate_session(
                world,
                format!(
                    "canonical prepared content {:?} disappeared while the GGRS session was active",
                    expected
                ),
            );
        }
        _ => {}
    }
}

fn invalidate_session(world: &mut World, reason: String) {
    stop_session(world);
    world
        .get_resource_or_insert_with::<RollbackSessionStatus>(Default::default)
        .invalidation = Some(reason);
    world.insert_resource(ambition_platformer2d_runtime::RollbackConfirmationState::Unhealthy);
}

fn live_content_identity(world: &mut World) -> Option<PreparedContentIdentity> {
    let mut query = world.query::<&PreparedContentIdentity>();
    query.iter(world).next().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One call reaches whichever seam the host actually reads.
    ///
    /// The three cases are not interchangeable and getting one wrong is silent: the driver keeps
    /// driving, the sim never sees the input, and nothing reports a thing.
    #[test]
    fn the_driver_seam_writes_whichever_resource_this_host_reads() {
        let pressed = ControlFrame {
            axis_x: 1.0,
            ..Default::default()
        };

        // A driver with no device and no latch: the press lands in the tables
        // the sim reads.
        let mut fixed = World::new();
        fixed.insert_resource(ControlFrame::default());
        fixed.insert_resource(ambition_characters::control::SeatRawFrames::default());
        fixed.insert_resource(ambition_characters::control::SlotControls::default());
        drive_control_frame(&mut fixed, pressed);
        assert_eq!(
            fixed
                .resource::<ambition_characters::control::SeatRawFrames>()
                .get(ambition_characters::control::PlayerSlot::PRIMARY)
                .axis_x,
            1.0,
            "the raw row is where a shaping stage would see the press"
        );
        assert_eq!(
            fixed
                .resource::<ambition_characters::control::SlotControls>()
                .get(ambition_characters::control::PlayerSlot::PRIMARY)
                .axis_x,
            1.0,
            "and the slot is where a composition with no shaping stages reads it"
        );
        assert_eq!(
            fixed.resource::<ControlFrame>().axis_x,
            0.0,
            "a driver must not write the output mirror on ANY host"
        );

        // The same driver under GGRS. `ControlFrame` is an OUTPUT there —
        // `publish_ggrs_input` overwrites it from the session's confirmed inputs
        // every advance — so the input must land in `PendingSeatInputs`, and
        // must NOT be written to `ControlFrame`, or a driver would be feeding
        // resimulated input back in as new input.
        let mut rollback = World::new();
        rollback.insert_resource(ControlFrame::default());
        rollback.insert_resource(PendingSeatInputs::default());
        drive_control_frame(&mut rollback, pressed);
        assert_eq!(rollback.resource::<PendingSeatInputs>().get(0).axis_x, 1.0);
        assert_eq!(
            rollback.resource::<ControlFrame>().axis_x,
            0.0,
            "a driver must not write the resource GGRS publishes into"
        );

        // A device-backed host: the latch wins, so nudging a windowed build does
        // not fight the device layer for the same resource.
        let mut windowed = World::new();
        windowed.insert_resource(ControlFrame::default());
        windowed.insert_resource(PendingSeatInputs::default());
        windowed.insert_resource(ambition_characters::control::SlotControlLatches::default());
        drive_control_frame(&mut windowed, pressed);
        assert_eq!(
            windowed
                .resource::<ambition_characters::control::SlotControlLatches>()
                .peek(ambition_characters::control::PlayerSlot::PRIMARY)
                .axis_x,
            1.0
        );
        assert_eq!(windowed.resource::<PendingSeatInputs>().get(0).axis_x, 0.0);
    }

    /// The "no world to rewind" detector fires exactly when construction has not
    /// happened, and stays quiet once it has.
    ///
    /// A detector that also fired on healthy sessions would be worse than none — a warning
    /// every correct host prints is a warning nobody reads.
    #[test]
    fn a_session_started_over_an_empty_world_is_the_detectable_case() {
        use ambition_platformer2d_shared_tangle::lifecycle::{SessionRoot, SessionScopeId};

        let empty = World::new();
        assert!(
            !has_session_world_root(&empty),
            "an empty world is exactly the case the warning exists for"
        );

        let mut constructed = World::new();
        constructed.spawn(SessionRoot(SessionScopeId(0)));
        assert!(
            has_session_world_root(&constructed),
            "a constructed session must NOT warn, or the warning is noise every              correct host prints and nobody reads"
        );
    }

    #[test]
    fn restarting_a_sync_test_session_rebases_ggrs_time_to_frame_zero() {
        let mut world = World::new();
        let mut old_timeline = Time::<GgrsTime>::new_with(GgrsTime);
        old_timeline.advance_to(std::time::Duration::from_secs(9));
        world.insert_resource(old_timeline);
        world.insert_resource(RollbackFrameCount(540));

        start_sync_test_session(
            &mut world,
            SyncTestSettings {
                check_distance: 0,
                max_prediction_window: 8,
                ..SyncTestSettings::for_players(1)
            },
        )
        .expect("a one-player baseline SyncTest session is valid");

        assert_eq!(world.resource::<RollbackFrameCount>().0, 0);
        assert_eq!(
            *world.resource::<RollbackSessionOwnership>(),
            RollbackSessionOwnership::LocalSyncTest {
                owner: SyncTestOwner::Caller,
                settings: SyncTestSettings {
                    check_distance: 0,
                    max_prediction_window: 8,
                    ..SyncTestSettings::for_players(1)
                },
            }
        );
        assert_eq!(
            world.resource::<Time<GgrsTime>>().elapsed(),
            std::time::Duration::ZERO,
            "a new frame-zero session must not retain elapsed time from the old timeline"
        );
    }

    /// A caller (the confirmed lifecycle commit) that builds first and only mutates + installs
    /// once the build succeeded can never half-commit. This test proves the two halves compose
    /// into the same end state the fused wrapper produces — and, by its very signature, that
    /// the build needs no world to fail against.
    #[test]
    fn a_session_can_be_built_before_the_world_and_installed_after() {
        let settings = SyncTestSettings {
            check_distance: 0,
            max_prediction_window: 8,
            ..SyncTestSettings::for_players(1)
        };

        // Build with NO world in scope at all — the fallible step is pure.
        let session = build_sync_test_session(settings)
            .expect("a one-player baseline SyncTest session is valid");

        // A destructive mutation would happen HERE in a real commit; only after
        // it succeeds do we touch the world, and that step cannot fail.
        let mut world = World::new();
        let mut old_timeline = Time::<GgrsTime>::new_with(GgrsTime);
        old_timeline.advance_to(std::time::Duration::from_secs(9));
        world.insert_resource(old_timeline);
        world.insert_resource(RollbackFrameCount(540));

        install_rebased_sync_test_session(&mut world, session, settings, SyncTestOwner::Caller);

        assert_eq!(world.resource::<RollbackFrameCount>().0, 0);
        assert_eq!(
            world.resource::<Time<GgrsTime>>().elapsed(),
            std::time::Duration::ZERO,
            "install resets the clock exactly as the fused path does"
        );
        assert_eq!(
            *world.resource::<RollbackSessionOwnership>(),
            RollbackSessionOwnership::LocalSyncTest {
                settings,
                owner: SyncTestOwner::Caller,
            },
            "the pre-built session is installed under the sync-test ownership"
        );
        assert!(
            matches!(
                world.resource::<AmbitionGgrsSession>(),
                AmbitionGgrsSession::SyncTest(_)
            ),
            "the installed session is the one that was built"
        );
    }

    #[test]
    fn stop_restart_uses_a_fresh_generation_after_the_boundary_was_removed() {
        let mut world = World::new();
        let settings = SyncTestSettings {
            check_distance: 0,
            max_prediction_window: 8,
            ..SyncTestSettings::for_players(1)
        };

        start_sync_test_session(&mut world, settings).expect("first session starts");
        let first = world.resource::<ConfirmedFrameBoundary>().session;
        stop_session(&mut world);
        assert!(
            !world.contains_resource::<ConfirmedFrameBoundary>(),
            "teardown must immediately disable quarantine and confirmation gates"
        );

        start_sync_test_session(&mut world, settings).expect("second session starts");
        let second = world.resource::<ConfirmedFrameBoundary>().session;
        assert_ne!(
            first, second,
            "frame zero in the restarted session must not alias frame zero from the old timeline"
        );
    }

    #[test]
    fn deferred_stop_removes_the_confirmed_boundary_too() {
        fn queue_stop(mut commands: Commands) {
            stop_session_deferred(&mut commands);
        }

        let mut app = App::new();
        app.world_mut().insert_resource(ConfirmedFrameBoundary {
            current: 9,
            confirmed: 4,
            session: 3,
        });
        app.add_systems(Update, queue_stop);
        app.update();

        assert!(
            !app.world().contains_resource::<ConfirmedFrameBoundary>(),
            "the deferred path must execute the same complete teardown as stop_session"
        );
    }

    /// The property AC18 turned on: a rebase restarts the per-session counters
    /// and must NOT restart the lifetime ones. A whole-run claim made against
    /// the per-session numbers is a claim about however much happened since the
    /// last rebase, which the caller cannot see and did not ask for.
    #[test]
    fn a_rebase_restarts_the_session_counters_and_carries_the_lifetime_totals() {
        let worked = RollbackExecutionStats {
            advance_runs: 2915,
            load_runs: 583,
            last_simulated_frame: 588,
            highest_simulated_frame: Some(588),
            lifetime_advance_runs: 2915,
            lifetime_load_runs: 583,
            sessions_installed: 1,
        };

        let rebased = worked.rebased();

        assert_eq!(
            rebased.advance_runs, 0,
            "a new session starts at frame zero"
        );
        assert_eq!(rebased.load_runs, 0);
        assert_eq!(rebased.last_simulated_frame, 0);
        assert_eq!(rebased.highest_simulated_frame, None);
        assert_eq!(rebased.lifetime_advance_runs, 2915);
        assert_eq!(rebased.lifetime_load_runs, 583);
        assert_eq!(
            rebased.sessions_installed, 2,
            "sessions_installed > 1 is exactly the signal that the per-session \
             counters were reset under a reader"
        );

        // Carried, NOT folded: the counting systems already advance the
        // lifetime totals every frame, so re-adding the outgoing session here
        // would double every session's work. This assertion is the one that
        // caught exactly that, first try.
        let twice = RollbackExecutionStats {
            advance_runs: 40,
            load_runs: 7,
            lifetime_advance_runs: 2955,
            lifetime_load_runs: 590,
            ..rebased
        }
        .rebased();
        assert_eq!(twice.lifetime_advance_runs, 2955);
        assert_eq!(twice.lifetime_load_runs, 590);
        assert_eq!(twice.sessions_installed, 3);
    }

    #[test]
    fn invalidation_removes_the_confirmed_boundary_but_preserves_the_reason() {
        let mut world = World::new();
        world.insert_resource(ConfirmedFrameBoundary {
            current: 7,
            confirmed: 2,
            session: 5,
        });

        invalidate_session(&mut world, "contract changed".into());

        assert!(!world.contains_resource::<ConfirmedFrameBoundary>());
        assert_eq!(
            world
                .resource::<RollbackSessionStatus>()
                .invalidation
                .as_deref(),
            Some("contract changed")
        );
    }

    #[test]
    fn device_edges_are_consumed_when_read_inputs_runs_not_each_render_frame() {
        let mut app = App::new();
        app.init_schedule(ReadInputs)
            .init_resource::<ambition_characters::control::SlotControlLatches>()
            .init_resource::<PendingSeatInputs>()
            .init_resource::<LocalPlayers>();
        install_session_bridge(&mut app);

        {
            let primary = ambition_characters::control::PlayerSlot::PRIMARY;
            let mut latches = app
                .world_mut()
                .resource_mut::<ambition_characters::control::SlotControlLatches>();
            latches.accumulate(
                primary,
                ControlFrame {
                    jump_pressed: true,
                    jump_held: true,
                    ..default()
                },
            );
            // A later rendered frame sees the button released, but no GGRS
            // tick requested input between these samples.
            latches.accumulate(primary, ControlFrame::default());
        }

        assert_eq!(
            app.world().resource::<PendingSeatInputs>().get(0),
            ControlFrame::default(),
            "render-frame sampling must not consume the tick latch"
        );

        app.world_mut().run_schedule(ReadInputs);
        let first = app.world().resource::<PendingSeatInputs>().get(0);
        assert!(
            first.jump_pressed,
            "the short press must reach the next GGRS tick"
        );
        assert!(!first.jump_held, "the latest button level is released");

        app.world_mut().run_schedule(ReadInputs);
        assert!(
            !app.world()
                .resource::<PendingSeatInputs>()
                .get(0)
                .jump_pressed,
            "the edge must be consumed exactly once"
        );
    }
}

#[cfg(test)]
mod replay_pass_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::schedule::SimulationReplayState;

    /// Runs the REAL `count_advance_run` for a frame, returning whether it
    /// judged the frame a re-simulation. Driving the actual system (not a
    /// reimplementation of its rule) is what makes this able to fail when the
    /// rule regresses.
    fn advance_to(world: &mut World, frame: i32) -> bool {
        world.insert_resource(RollbackFrameCount(frame));
        world
            .run_system_cached(count_advance_run)
            .expect("count_advance_run runs");
        world.resource::<SimulationReplayState>().replaying_history
    }

    fn rollback_world() -> World {
        let mut world = World::new();
        world.init_resource::<RollbackExecutionStats>();
        world.init_resource::<SimulationReplayState>();
        world
    }

    /// A rollback re-runs frames 3 and 4 and then simulates 5 for the first
    /// time. The distinction still matters for the consumers that legitimately
    /// want "don't do this twice" — the trace's duplicate append and the
    /// falling-sand step guard. External effects no longer read it at all.
    #[test]
    fn only_the_re_simulated_frames_are_marked_as_replay() {
        let mut world = rollback_world();
        for frame in 0..=4 {
            assert!(
                !advance_to(&mut world, frame),
                "frame {frame} is being simulated for the first time"
            );
        }

        // Rollback: GGRS reloads frame 2 and re-advances through 4.
        for frame in 3..=4 {
            assert!(advance_to(&mut world, frame), "frame {frame} ran before");
        }

        assert!(
            !advance_to(&mut world, 5),
            "frame 5 is new — the frame the player just caused"
        );
    }

    /// The confirmed boundary is the fact external effects key on, and it is
    /// republished every advance.
    ///
    /// ⚠ NO SESSION HERE, so this pins the HARNESS FALLBACK, not the rule a
    /// running game gets. `a_sync_test_with_rollback_dormant_confirms_the_frame_it_just_ran`
    /// is the one that reads the session, and it is the arm that was missing when
    /// the boundary published a permanent one-frame lag.
    #[test]
    fn each_advance_publishes_where_the_confirmed_line_sits() {
        let mut world = rollback_world();
        world.insert_resource(ConfirmedFrameBoundary::default());
        world.insert_resource(ConfirmedFrameCount(2));

        advance_to(&mut world, 6);

        let boundary = *world.resource::<ConfirmedFrameBoundary>();
        assert_eq!(boundary.current, 6);
        assert_eq!(boundary.confirmed, 2);
        assert!(
            !boundary.fully_confirmed(),
            "frames 3..=6 are still predicted"
        );
    }

    /// ⛔⛔ WITH ROLLBACK DORMANT, THE FRAME JUST RUN IS ALREADY FINAL.
    ///
    /// The shipped app runs a sync test with `check_distance: 0`: nothing is
    /// ever re-simulated, so nothing is ever speculative. Reading
    /// `ConfirmedFrameCount` instead published `current - 1` here, which made
    /// `fully_confirmed()` false on every frame the game ever ran and stood down
    /// the winner card, the return to character select and the persistence save
    /// without a word.
    ///
    /// ⭐ THE ARMS STRADDLE THE CHECK DISTANCE, everything else held still: a
    /// claim about a boundary that compares two frame numbers is worth nothing
    /// unless one arm sits on each side of the window.
    #[test]
    fn a_sync_test_with_rollback_dormant_confirms_the_frame_it_just_ran() {
        for (check_distance, expected) in [(0usize, 9i32), (7, 2)] {
            let session = build_sync_test_session(SyncTestSettings {
                check_distance,
                max_prediction_window: 12,
                players: 1,
            })
            .expect("a sync-test session builds");

            let mut world = rollback_world();
            world.insert_resource(ConfirmedFrameBoundary::default());
            world.insert_resource(session);
            // The stale value a real advance would find in the world: bevy_ggrs
            // computes it before bumping the frame, so it is one behind.
            world.insert_resource(ConfirmedFrameCount(8 - check_distance as i32));

            advance_to(&mut world, 9);

            let boundary = *world.resource::<ConfirmedFrameBoundary>();
            assert_eq!(boundary.current, 9);
            assert_eq!(
                boundary.confirmed, expected,
                "check_distance {check_distance} confirms up to frame {expected}"
            );
            assert_eq!(
                boundary.fully_confirmed(),
                check_distance == 0,
                "a dormant window leaves nothing predicted; a window of \
                 {check_distance} leaves frames still open"
            );
        }
    }

    /// `LoadWorld` moves the simulation back to the restored frame. The
    /// abandoned-branch discard reads exactly this, so it must be republished
    /// rather than left pointing at the frame the host walked away from.
    #[test]
    fn restoring_a_frame_moves_the_published_boundary_back_to_it() {
        let mut world = rollback_world();
        world.insert_resource(ConfirmedFrameBoundary::default());
        world.insert_resource(ConfirmedFrameCount(1));
        advance_to(&mut world, 9);

        world.insert_resource(RollbackFrameCount(4));
        world
            .run_system_cached(mark_historical_replay)
            .expect("mark_historical_replay runs");

        assert_eq!(
            world.resource::<ConfirmedFrameBoundary>().current,
            4,
            "the simulation now sits at the restored frame, not at 9"
        );
    }

    /// A host with no confirmed boundary installed (fixed-tick, headless, or a
    /// rollback host before its first session) must still work.
    #[test]
    fn a_host_without_a_boundary_still_tracks_replay() {
        let mut world = rollback_world();

        for frame in [0, 1, 1] {
            world.insert_resource(RollbackFrameCount(frame));
            world
                .run_system_cached(count_advance_run)
                .expect("count_advance_run runs without a boundary");
        }

        assert!(world.resource::<SimulationReplayState>().replaying_history);
    }
}

#[cfg(test)]
mod multi_seat_input_tests {
    use super::*;
    use ambition_characters::control::{PlayerSlot, SlotControls};

    fn frame_with_axis(axis_x: f32) -> ControlFrame {
        ControlFrame {
            axis_x,
            ..ControlFrame::default()
        }
    }

    /// Every local handle gets its OWN frame.
    ///
    /// `publish_local_inputs` inserted `pending.0` for every handle, which was
    /// correct while there was exactly one and silently wrong the moment there
    /// were two: four pads would drive one input stream and the sync test would
    /// checksum-compare a simulation nobody was playing.
    #[test]
    fn each_local_handle_submits_its_own_input_stream() {
        let mut app = App::new();
        let mut pending = PendingSeatInputs::default();
        pending.set(0, frame_with_axis(1.0));
        pending.set(1, frame_with_axis(-1.0));
        app.insert_resource(pending);
        app.insert_resource(LocalPlayers(vec![0, 1]));
        app.add_systems(Update, publish_local_inputs);
        app.update();

        let inputs = app.world().resource::<LocalInputs<AmbitionGgrsConfig>>();
        assert_eq!(
            inputs.0.get(&0).map(|frame| frame.axis_x),
            Some(1.0),
            "handle 0 must carry the PRIMARY seat's pending input"
        );
        assert_eq!(
            inputs.0.get(&1).map(|frame| frame.axis_x),
            Some(-1.0),
            "handle 1 was handed seat zero's frame — two pads, one input stream, \
             and a checksum comparison of a game nobody is playing"
        );
    }

    /// A composition with no secondary seats reads NEUTRAL for them rather than
    /// inheriting seat zero — a pad nobody plugged in is not a pad holding left.
    #[test]
    fn a_handle_with_no_seat_input_is_neutral_not_a_copy_of_seat_zero() {
        let mut app = App::new();
        {
            let mut pending = PendingSeatInputs::default();
            pending.set(0, frame_with_axis(1.0));
            app.insert_resource(pending);
        }
        app.insert_resource(LocalPlayers(vec![0, 1]));
        app.add_systems(Update, publish_local_inputs);
        app.update();

        let inputs = app.world().resource::<LocalInputs<AmbitionGgrsConfig>>();
        assert_eq!(inputs.0.get(&1).map(|frame| frame.axis_x), Some(0.0));
    }

    /// The player count is what the session builds with, clamped to the
    /// slots the game supports rather than asserted: this is settings data
    /// reaching the builder from a dev tool and a harness option, and a session
    /// that refuses to start is worse than one that starts with a sane count.
    #[test]
    fn the_player_count_is_clamped_into_what_a_session_can_hold() {
        let one = SyncTestSettings::for_players(1);
        assert_eq!(one.player_count(), 1, "the default is still one seat");

        let zero = SyncTestSettings {
            players: 0,
            ..SyncTestSettings::for_players(1)
        };
        assert_eq!(
            zero.player_count(),
            1,
            "a session needs at least one player"
        );

        let too_many = SyncTestSettings {
            players: 99,
            ..SyncTestSettings::for_players(1)
        };
        assert_eq!(too_many.player_count(), SlotControls::MAX_SLOTS);
    }

    /// A two-player session actually builds. The builder loops `add_player`, and
    /// GGRS rejects a handle count that disagrees with `with_num_players`, so
    /// this is the check that the loop and the count cannot drift apart.
    #[test]
    fn a_two_player_sync_test_session_builds() {
        let settings = SyncTestSettings {
            players: 2,
            ..SyncTestSettings::for_players(1)
        };
        build_sync_test_session(settings).expect("a two-seat sync test session builds");
    }

    /// Seats 1.. land in `SlotControls`, which is where `tick_controlled_brains`
    /// already looks — so a rewind replays every seat's input rather than only
    /// the first.
    #[test]
    fn confirmed_inputs_reach_the_primary_frame_and_the_secondary_slots() {
        let mut slots = SlotControls::default();
        let mut control = ControlFrame::default();

        // Stand in for `PlayerInputs`' iteration order: handle, frame.
        for (handle, frame) in [(0usize, frame_with_axis(1.0)), (1, frame_with_axis(-1.0))] {
            if handle == 0 {
                control = frame;
            } else {
                slots.set(PlayerSlot(handle as u8), frame);
            }
        }

        assert_eq!(control.axis_x, 1.0, "handle 0 is the primary seat's frame");
        assert_eq!(
            slots.get(PlayerSlot(1)).axis_x,
            -1.0,
            "handle 1 must reach the slot its brain reads"
        );
        assert_eq!(
            slots.get(PlayerSlot(0)).axis_x,
            0.0,
            "slot 0 is NOT written here — the primary seat is `ControlFrame`, and \
             two homes for one seat is how the two come to disagree"
        );
    }
}

#[cfg(test)]
mod ac23_tests {
    use super::*;

    /// A new session inherits an unhealthy timeline's reason. (AC23)
    #[test]
    fn an_invalidated_session_hands_its_reason_to_its_replacement() {
        let previous = RollbackSessionStatus {
            mismatch_frames: Vec::new(),
            invalidation: Some("room reconstructed under a live timeline".to_string()),
        };
        let carried = RollbackSessionStatus::carried_from(Some(&previous));
        assert!(
            !carried.is_healthy(),
            "an inherited invalidation was reported healthy"
        );
        assert_eq!(
            carried.invalidation.as_deref(),
            Some("room reconstructed under a live timeline"),
            "the replacement session came up clean, so the divergence was \
             laundered by the install"
        );
    }

    /// A checksum mismatch carries as PROSE, not as frame numbers.
    ///
    /// Frames restart at zero for every GGRS session, so carrying the numbers
    /// would report a mismatch at frames the new timeline has not reached.
    #[test]
    fn a_mismatch_carries_its_reason_but_not_its_frame_numbers() {
        let previous = RollbackSessionStatus {
            mismatch_frames: vec![41, 42],
            invalidation: None,
        };
        let carried = RollbackSessionStatus::carried_from(Some(&previous));
        assert!(
            carried.mismatch_frames.is_empty(),
            "frame numbers from a dead timeline were carried into a live one, so \
             the new session reports a mismatch at frames it has not reached"
        );
        let reason = carried
            .invalidation
            .expect("the mismatch survives as prose");
        assert!(
            reason.contains("41"),
            "the reason lost the evidence: {reason}"
        );
        assert!(
            reason.contains("PREVIOUS"),
            "the reason does not say the mismatch belongs to the old timeline: {reason}"
        );
    }

    /// A HEALTHY session installs clean, which is the ordinary case and must not
    /// acquire a phantom diagnostic.
    #[test]
    fn a_healthy_session_installs_clean() {
        let previous = RollbackSessionStatus::default();
        assert!(
            previous.is_healthy(),
            "the default session status is not healthy"
        );
        assert_eq!(
            RollbackSessionStatus::carried_from(Some(&previous)),
            RollbackSessionStatus::default()
        );
        assert_eq!(
            RollbackSessionStatus::carried_from(None),
            RollbackSessionStatus::default()
        );
    }

    /// Clearing is possible, but only by SAYING SO.
    #[test]
    fn a_diagnostic_can_be_cleared_only_deliberately() {
        let mut status = RollbackSessionStatus {
            mismatch_frames: vec![7],
            invalidation: Some("diverged".to_string()),
        };
        status.acknowledge_and_clear();
        assert_eq!(status, RollbackSessionStatus::default());
    }
}

/// Warn when a rollback consumer writes `ControlFrame` instead of
/// `PendingSeatInputs`. GGRS derives `ControlFrame` from confirmed seat input on
/// each simulated frame, so an external write would be discarded. The warning
/// requires sustained disagreement and runs outside rollback because its local
/// counter is diagnostic-only and must not rewind.
fn report_input_written_to_the_wrong_seam(
    mut reported: ResMut<InputSeamMisuse>,
    control: Option<Res<ControlFrame>>,
    pending: Option<Res<PendingSeatInputs>>,
    // Liveness is the PRESENCE of the session resource, which is how
    // `local_session` asks the same question.
    session: Option<Res<AmbitionGgrsSession>>,
    mut consecutive: Local<u32>,
) {
    const WRONG_SEAM_FRAMES: u32 = 45;

    let (Some(control), Some(pending)) = (control, pending) else {
        return;
    };
    // Only meaningful while a rollback session is actually the authority.
    if session.is_none() {
        *consecutive = 0;
        return;
    }
    let neutral = ControlFrame::default();
    if *control != neutral && pending.get(0) == neutral {
        *consecutive += 1;
    } else {
        *consecutive = 0;
        return;
    }
    if *consecutive >= WRONG_SEAM_FRAMES && !reported.0 {
        reported.0 = true;
        bevy::log::warn!(
            target: "ambition_platformer2d::rollback",
            "input is being written to `ControlFrame`, but this is a ROLLBACK \
             host and it reads `PendingSeatInputs`. `publish_ggrs_input` \
             overwrites `ControlFrame` from the session's confirmed inputs every \
             simulated frame, so those writes are discarded and the body will \
             never move. Write handle zero of `PendingSeatInputs` (or feed \
             `SlotControlLatches`, \
             which the device path uses). Said once per run."
        );
    }
}

/// The wrong-seam diagnostic — the one defect in this file that no in-workspace
/// caller can currently reach, which is exactly why it needs a fixture.
#[cfg(test)]
mod wrong_seam_tests {
    use super::*;

    /// Run the real system and assert on the fact it publishes rather than
    /// re-deriving the predicate in the test.
    fn reported(live_session: bool, drive_control: bool, drive_pending: bool) -> bool {
        let mut app = bevy::prelude::App::new();
        app.init_resource::<ControlFrame>();
        app.init_resource::<PendingSeatInputs>();
        app.init_resource::<InputSeamMisuse>();
        if live_session {
            app.insert_resource(
                build_sync_test_session(SyncTestSettings::for_players(1))
                    .expect("a sync-test session builds"),
            );
        }
        app.add_systems(Update, report_input_written_to_the_wrong_seam);

        for _ in 0..90 {
            if drive_control {
                app.world_mut().resource_mut::<ControlFrame>().axis_x = 1.0;
            }
            if drive_pending {
                {
                    let mut pending = app.world_mut().resource_mut::<PendingSeatInputs>();
                    let mut frame = pending.get(0);
                    frame.axis_x = 1.0;
                    pending.set(0, frame);
                }
            }
            app.update();
        }
        app.world().resource::<InputSeamMisuse>().0
    }

    /// The broken consumer. Drives `ControlFrame` under a rollback host,
    /// which reads `PendingSeatInputs` — so `publish_ggrs_input` overwrites
    /// those writes every simulated frame and the body never moves. This is the
    /// case the roadmap records as unreachable by any in-repo test.
    #[test]
    fn driving_control_frame_under_a_rollback_host_is_reported() {
        assert!(reported(true, true, false));
    }

    /// The healthy consumer drives the seam this host actually reads.
    #[test]
    fn driving_pending_local_input_is_silent() {
        assert!(!reported(true, false, true));
    }

    /// A fixed-tick host is not this diagnostic's business. `ControlFrame`
    /// IS the authority there, so the identical writes must say nothing.
    #[test]
    fn driving_control_frame_without_a_session_is_silent() {
        assert!(!reported(false, true, false));
    }

    /// The poison. An idle rollback host must stay silent, or the check is
    /// only asking "is a session running".
    #[test]
    fn an_idle_rollback_host_is_silent() {
        assert!(!reported(true, false, false));
    }
}
