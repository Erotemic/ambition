//! GGRS session/input bridge shared by the harness and future network hosts.

use bevy::prelude::*;
use bevy_ggrs::ggrs::{self, PlayerType, SessionBuilder};
use bevy_ggrs::{
    ConfirmedFrameCount, GgrsConfig, GgrsSchedule, GgrsTime, LoadWorld, LocalInputs, LocalPlayers,
    PlayerInputs, ReadInputs, RollbackFrameCount, RunGgrsSystems, Session, SyncTestMismatch,
};

use ambition_platformer2d_core::{ConfirmedFrameBoundary, ControlFrame, ControlFrameLatch};

use super::RollbackRegistry;
use crate::{PreparedContentIdentity, SnapshotSchemaFingerprint};

pub type AmbitionGgrsConfig = GgrsConfig<ControlFrame>;
pub type AmbitionGgrsSession = Session<AmbitionGgrsConfig>;

#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum AmbitionReadInputsSet {
    CaptureDeviceLatch,
    PublishLocalInputs,
}

/// External input waiting to be submitted to GGRS for the next frame. This is
/// intentionally not rollback state: prediction/session logic owns the input
/// stream, while simulation state is rewound beneath it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct PendingLocalInput(pub ControlFrame);

/// The SECONDARY seats' pending input, one per slot. (queue Y1)
///
/// [`PendingLocalInput`] is seat zero's and predates local multiplayer. When the
/// session carries more than one player, every handle needs its own stream —
/// publishing seat zero's frame to all of them (which is what
/// `publish_local_inputs` did, because there was only ever one) would make four
/// pads move one fighter and checksum-compare a lie.
///
/// Slot 0 is intentionally absent: it is `PendingLocalInput`, and two homes for
/// one seat is how the two would come to disagree.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct PendingSeatInputs {
    seats: [ControlFrame; ambition_characters::brain::SlotControls::MAX_SLOTS],
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

/// Counts actual GGRS operations. It is intentionally outside rollback state so
/// tests can prove that a single harness step performed load/resimulation work.
///
/// ## Per-session versus lifetime, and why both exist (queue AC18)
///
/// A rebase installs a NEW session, and a new session legitimately starts its
/// frame numbering at zero — so the per-session counters below reset with it.
/// That is correct, and it silently invalidated the one assertion these
/// counters existed for.
///
/// The rollback exit oracle asserted `advance_runs > harness_steps`. Its route
/// happened to produce a confirmed Track-B lifecycle commit at frame 587 of
/// 600, which rebased the session atomically and by design — leaving
/// `advance_runs: 40, last_simulated_frame: 12` to be compared against 600
/// steps. The oracle went red reporting numbers that looked exactly like a GGRS
/// session that had stopped being driven at frame 12, and was read that way for
/// a day. It had in fact executed 2915 advances by frame 500.
///
/// ⚠ **so the numbers were not wrong, they were answering a different
/// question**, and nothing in the type said which. The `lifetime_*` fields
/// survive session replacement and are what a whole-run claim must be made
/// against; the unprefixed fields describe the CURRENT session only.
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
    /// ⚠ **carried, not folded.** The lifetime totals are accumulated by the
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
    /// **The status a NEW session starts from, given the outgoing one.** (AC23)
    ///
    /// Installing a session used to write `default()` unconditionally, which
    /// laundered a divergence: a sync-test mismatch reported on the old timeline
    /// vanished the moment a new session replaced it, and the replacement looked
    /// clean. Exactly one of four call sites guarded against it, and its comment
    /// explained precisely why it had to — which is a seam asking every caller to
    /// remember a rule the seam could enforce.
    ///
    /// So the diagnostic CARRIES. An unhealthy timeline hands its reason to the
    /// timeline that replaces it, and the only way to clear it is to say so
    /// (`acknowledge_and_clear`).
    ///
    /// ⚠ `mismatch_frames` does NOT carry, and that is not an oversight: frame
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
    /// How many players the session carries. (queue Y1)
    ///
    /// The FIELD arrived on 2026-07-28; the shipped host started setting it the
    /// same day. Until then it was one — invisible while the game was one-player,
    /// and a coverage gap the week C4 shipped a 2–4 player couch versus mode: the
    /// rollback oracle proved determinism for ONE input stream while the game
    /// seated four. A desync in seat two's input handling had nowhere to show up.
    ///
    /// ⚠ **every construction of this struct must set it.** `..Default::default()`
    /// silently means one player, and the proof-pulse restore path was still doing
    /// that a day after the other two paths were fixed — a four-player match came
    /// back from F9 with one handle while the roster still held four fighters
    /// (GPT 5.6, 2026-07-29). The value comes from the session's frozen
    /// `LocalSeatTopology`, never from a fresh sample of live devices.
    ///
    /// Every player is `PlayerType::Local` — a sync test has no remote peer by
    /// definition. What it buys is not networking, it is that N input streams go
    /// through save/rewind/resimulate and are checksum-compared, which is the
    /// precondition for any of them being remote later.
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
            .clamp(1, ambition_characters::brain::SlotControls::MAX_SLOTS)
    }
}

/// Who owns the currently installed GGRS session.
///
/// Local sync-test sessions may be stopped and recreated around a developer
/// content reload. External/P2P sessions require a coordinated peer barrier and
/// must never be replaced unilaterally by the local host.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollbackSessionOwnership {
    LocalSyncTest(SyncTestSettings),
    External,
}

/// Settings for `players` local streams, at the standard rollback depth.
///
/// **THE ONLY WAY to get settings without naming every field**, and it takes the
/// player count as an argument, which is the whole point of the `Default` impl
/// this replaced.
///
/// That impl guessed ONE, and the guess was wrong in production three separate
/// times: the initial session, the hot-reload rebase and the proof-pulse restore
/// each built a one-handle session while the game seated up to four (queue H3,
/// H4; GPT 5.6, 2026-07-28 and -29). Each was repaired individually and the
/// fourth site would have been free to make the same mistake, because the rule
/// keeping them right lived in a doc comment saying "every construction of this
/// struct must set it" — a load-bearing negative nothing enforced.
///
/// The split is between TUNING and TOPOLOGY. `check_distance` and
/// `max_prediction_window` are tuning: a default is a real answer, and a
/// third-party consumer should be able to name only the ones it cares about and
/// keep compiling when the engine adds another (which is exactly what
/// `fixtures/external_consumer` exists to prove). How many people are playing is
/// not tuning, and there is no honest default for it — so it is an argument, and
/// a caller that has not decided cannot type this.
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
    // The ONLY fallible step — pure GGRS construction, touches no world — runs
    // first. A caller that must not mutate the world until it knows the session
    // will exist (the atomic lifecycle commit) instead calls
    // `build_sync_test_session` before its destructive step and
    // `install_rebased_sync_test_session` after; this convenience wrapper is for
    // callers that own the whole rebase (startup / harness restart).
    let session = build_sync_test_session(settings)?;
    install_rebased_sync_test_session(world, session, settings);
    Ok(())
}

/// Construct the replacement sync-test session WITHOUT touching the world.
///
/// This is the sole fallible half of a rebase, and it depends only on
/// `settings`, so it can be built BEFORE any destructive world mutation and the
/// mutation skipped entirely if it fails — the atomicity the confirmed
/// lifecycle commit needs. Pair it with [`install_rebased_sync_test_session`].
pub fn build_sync_test_session(
    settings: SyncTestSettings,
) -> Result<AmbitionGgrsSession, ggrs::GgrsError> {
    let players = settings.player_count();
    let mut builder = SessionBuilder::<AmbitionGgrsConfig>::new()
        .with_num_players(players)?
        .with_fps(crate::SIM_TICK_HZ as usize)?
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
/// INFALLIBLE by construction: everything here is a world write. It performs the
/// destructive resets a rebase entails — frame counters and, crucially, the
/// `Time<GgrsTime>` clock — then installs the session. Because it cannot fail, a
/// caller can run it AFTER its own destructive step knowing the rebase completes.
/// Say so, loudly, when a session is being started before there is a world to
/// rewind.
///
/// A rollback session takes the CURRENT world as frame zero. If gameplay
/// construction has not run yet, the frames immediately after this one build the
/// room and the bodies through `Commands` — and a rollback cannot undo
/// construction, so every resimulated frame in that window mismatches. GGRS
/// reports that correctly and reports it as a checksum difference, which tells a
/// consumer nothing about the cause.
///
/// The engine's own harness cannot hit this: `Platformer2dSimHarness` builds the room first
/// by construction. So the only people who meet it are exactly the ones with no
/// way to diagnose it — an external consumer starting a session at boot gets
/// frames 2, 3 and 4 mismatching forever. Outlander did precisely that
/// (2026-07-27).
///
/// A warning and not a refusal: a fixture may legitimately want a session over an
/// empty world, and the engine does not get to veto a composition it cannot see
/// the purpose of. What it can do is name the cause the one time it is cheap to
/// name it.
fn warn_if_no_world_to_rewind(world: &World) {
    if has_session_world_root(world) {
        return;
    }
    bevy::log::warn!(
        target: "ambition_platformer2d::rollback",
        "starting a rollback session with no session world: frame zero is an          EMPTY world, so the construction that runs next happens inside the          rollback window. A rollback cannot undo `Commands`, so the frames that          build the room will mismatch on every resimulation and GGRS will report          it only as a checksum difference. Activate the session world first, then          start the session — it rebases onto whatever is live."
    );
}

/// Whether a gameplay session world has been constructed. `try_query` yields
/// `None` when `SessionRoot` was never registered, which is the correct "no
/// world" answer for a bare fixture.
fn has_session_world_root(world: &World) -> bool {
    world
        .try_query::<&ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>()
        .map(|mut query| query.iter(world).next().is_some())
        .unwrap_or(false)
}

/// **Replace the WHOLE input-authority cluster, atomically.**
///
/// Primary latch, per-seat latches, pending local input, pending seat inputs.
/// They describe one rollback timeline between them, so preserving some across a
/// session replacement and clearing the rest produces a session whose player one
/// starts clean while players two through four carry the previous timeline's held
/// levels — a jump or a direction edge captured just before a hot reload or a
/// proof pulse, leaking into the new baseline (GPT 5.6, 2026-07-29).
///
/// `SlotControlLatches` was the one being preserved, which is the half a
/// single-player test could never notice.
///
/// Each is reset only if the composition installed it: inserting one here would
/// make this function a second authority on which latches exist.
fn reset_input_authority(world: &mut World) {
    world.insert_resource(PendingLocalInput::default());
    world.insert_resource(PendingSeatInputs::default());
    if world.contains_resource::<ControlFrameLatch>() {
        world.insert_resource(ControlFrameLatch::default());
    }
    if world.contains_resource::<ambition_characters::brain::SlotControlLatches>() {
        world.insert_resource(ambition_characters::brain::SlotControlLatches::default());
    }
}

pub fn install_rebased_sync_test_session(
    world: &mut World,
    session: AmbitionGgrsSession,
    settings: SyncTestSettings,
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

    // GgrsTimePlugin derives deterministic elapsed time from RollbackFrameCount
    // by calling Time::advance_to. Replacing a running session resets the frame
    // counter to zero, so retaining the previous session's elapsed GGRS clock
    // would ask Bevy to move time backwards on the first AdvanceWorld and panic.
    // A session rebase is a new deterministic timeline: reset its clock along
    // with its frame identity before the first frame-zero snapshot is saved.
    world.insert_resource(Time::<GgrsTime>::new_with(GgrsTime));

    install_session_with_ownership(
        world,
        session,
        RollbackSessionOwnership::LocalSyncTest(settings),
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
    world.insert_resource(carried_status);
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
    if let Some(reason) = &status.invalidation {
        return Err(reason.clone());
    }
    if !status.mismatch_frames.is_empty() {
        return Err(format!(
            "GGRS sync-test checksum mismatch at frames {:?}",
            status.mismatch_frames
        ));
    }
    Ok(())
}

pub fn session_is_active(world: &World) -> bool {
    world.contains_resource::<AmbitionGgrsSession>()
}

/// **THE seam a driver writes input through, whichever host is running.**
///
/// A driver is anything supplying input that is not a device: a headless
/// harness, an RL agent, a replay, an integration test, a consumer's acceptance
/// walk. There are two resources underneath and picking the wrong one FAILS
/// SILENTLY — the walk runs, the body never moves, nothing says why — so this
/// exists to make the choice unnecessary rather than merely documented.
///
/// The split is not an accident and cannot be merged away. Under a fixed-tick
/// host [`ControlFrame`] is the input the sim reads. Under GGRS it is an
/// OUTPUT: `publish_ggrs_input` writes it from the session's confirmed inputs
/// every advance, so a driver writing it would be feeding resimulated input
/// back in as new input. `PendingLocalInput` is the input side there.
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
    if let Some(mut latch) = world.get_resource_mut::<ControlFrameLatch>() {
        latch.accumulate(frame);
        return;
    }
    // ⚠ this does NOT clear `PendingSeatInputs`, and an earlier version did.
    // `drive_seat_frame` is called BEFORE the step it applies to, so clearing
    // here wiped every secondary seat's input on the way past — the seam was
    // built and then emptied by its own sibling, one line later. A driver that
    // wants a seat neutral drives it neutral; silence is not a request.
    if let Some(mut pending) = world.get_resource_mut::<PendingLocalInput>() {
        pending.0 = frame;
        return;
    }
    if let Some(mut control) = world.get_resource_mut::<ControlFrame>() {
        *control = frame;
    }
}

/// **THE seam a driver writes a SECONDARY seat's input through.** (queue Y1)
///
/// The twin of [`drive_control_frame`], and it exists for the same reason that
/// one does: every driver that grew its own copy grew the same bug, because
/// writing the wrong resource is silently ignored and the sim simply never
/// moves.
///
/// Under a latching host it folds into the seat's latch, so a sub-tick press
/// survives exactly as the primary seat's does. Without one it writes the
/// pending seat input directly, which is what a headless or replay driver
/// wants — it authors per-tick frames itself and has no device to bridge.
///
/// Slot 0 is refused rather than silently redirected: it is
/// [`drive_control_frame`]'s, and a driver that meant the primary seat should
/// say so.
pub fn drive_seat_frame(
    world: &mut World,
    slot: ambition_characters::brain::PlayerSlot,
    frame: ControlFrame,
) {
    if slot.0 == 0 {
        return;
    }
    if let Some(mut latches) =
        world.get_resource_mut::<ambition_characters::brain::SlotControlLatches>()
    {
        latches.accumulate(slot, frame);
        return;
    }
    if let Some(mut seats) = world.get_resource_mut::<PendingSeatInputs>() {
        seats.set(slot.0 as usize, frame);
        return;
    }
    if let Some(mut slots) = world.get_resource_mut::<ambition_characters::brain::SlotControls>() {
        slots.set(slot, frame);
    }
}

pub(crate) fn install_session_bridge(app: &mut App) {
    // Only a speculating host quarantines external effects, so the whole
    // mechanism is installed HERE rather than in the engine group: a fixed-tick
    // or render-frame game carries none of these systems at all.
    crate::external_effects::quarantine_presentation_effects(app, LoadWorld);

    app.init_resource::<PendingLocalInput>()
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
                    .before(crate::external_effects::ExternalEffectSet::DiscardAbandoned),
                count_load_run.in_set(super::AmbitionLoadWorldSet::Reconcile),
            ),
        )
        .add_systems(
            PreUpdate,
            (
                enforce_session_contract.before(RunGgrsSystems),
                clear_historical_replay.after(RunGgrsSystems),
                // Track B: execute a confirmed deferred lifecycle op in the
                // exclusive world and rebase, after the advance batch is done.
                // Ordered AFTER the external-effect Release: the rebase bumps the
                // session generation, and the effect journal discards any pending
                // confirmed effects stamped with the OLD generation — so they must
                // be released to presentation first, or transition-adjacent
                // SFX/VFX/debris confirmed before the rebase would be dropped.
                crate::lifecycle_commit::commit_confirmed_lifecycle
                    .after(RunGgrsSystems)
                    .after(clear_historical_replay)
                    .after(crate::external_effects::ExternalEffectSet::Release),
            ),
        )
        // Effects may only be released once this render frame's advances are
        // done. Without this edge Bevy is free to release first, and the next
        // advance's outbox clear then wipes what was just handed to
        // presentation — silently, since the journal has already counted it.
        .configure_sets(
            PreUpdate,
            crate::external_effects::ExternalEffectSet::Release.after(RunGgrsSystems),
        )
        .add_observer(record_sync_test_mismatch);
}

/// Consume device input only when GGRS actually asks for the next local
/// input. Draining the latch once per rendered frame is incorrect: several
/// rendered frames may pass before a simulation tick, and a later level-only
/// sample would overwrite a short press before GGRS observed it.
fn capture_latched_local_input(
    latch: Option<ResMut<ControlFrameLatch>>,
    mut pending: ResMut<PendingLocalInput>,
    // The SECONDARY seats' half (queue Y1), drained on the same edge and for the
    // same reason: several rendered frames may pass before GGRS asks, and a
    // later level-only sample would overwrite a short press before the session
    // observed it. Optional because a composition with one seat installs
    // neither.
    seat_latches: Option<ResMut<ambition_characters::brain::SlotControlLatches>>,
    mut seats: Option<ResMut<PendingSeatInputs>>,
) {
    // ONLY when a device is actually wired to this latch. An untouched latch
    // means "nothing feeds me", not "the device said nothing" — and a
    // composition that drives `PendingLocalInput` directly (every rollback
    // harness, the equipment oracle's walker) would otherwise have its driven
    // frame replaced by a neutral default on every tick, which is how four
    // rollback oracles went red at once.
    //
    // The predicate is STICKY rather than per-frame: a tick that sampled
    // nothing must still receive the retained levels, or a held direction
    // sticks on forever.
    if let Some(mut latch) = latch {
        if latch.is_device_authority() {
            pending.0 = latch.take();
        }
    }
    if let (Some(mut seat_latches), Some(seats)) = (seat_latches, seats.as_deref_mut()) {
        for handle in 1..ambition_characters::brain::SlotControls::MAX_SLOTS {
            let slot = ambition_characters::brain::PlayerSlot(handle as u8);
            seats.set(handle, seat_latches.take(slot));
        }
    }
}

/// Hand GGRS one frame PER LOCAL HANDLE. (queue Y1)
///
/// This used to insert `pending.0` for every handle, which was correct while
/// there was exactly one and silently wrong the moment there were two: four pads
/// would drive one input stream and the sync test would checksum-compare a
/// simulation nobody was playing.
///
/// Handle 0 is [`PendingLocalInput`] — seat zero's, latched by the device layer
/// and drained when GGRS asks. Handles 1.. are [`PendingSeatInputs`], which is
/// absent in a composition with no secondary seats and reads neutral, exactly as
/// a pad nobody plugged in should.
fn publish_local_inputs(
    pending: Res<PendingLocalInput>,
    seats: Option<Res<PendingSeatInputs>>,
    local_players: Res<LocalPlayers>,
    mut commands: Commands,
) {
    let mut inputs = bevy::platform::collections::HashMap::default();
    for &handle in &local_players.0 {
        let frame = if handle == 0 {
            pending.0
        } else {
            seats
                .as_deref()
                .map(|seats| seats.get(handle))
                .unwrap_or_default()
        };
        inputs.insert(handle, frame);
    }
    commands.insert_resource(LocalInputs::<AmbitionGgrsConfig>(inputs));
}

/// Publish the session's confirmed inputs into what the simulation reads.
///
/// Handle 0 becomes [`ControlFrame`], the primary seat's. Handles 1.. become
/// `SlotControls[handle]`, which is where `tick_player_brains` already looks for
/// a secondary seat — so a rewind replays every seat's input, not just the
/// first (queue Y1).
///
/// ⚠ this is what puts seats 1.. INSIDE rollback. Before it they were written
/// on the feel clock by the host's own input path and GGRS never saw them: a
/// resimulated frame replayed seat zero faithfully and gave every other seat
/// whatever the device happened to be doing at replay time.
fn publish_ggrs_input(
    inputs: Res<PlayerInputs<AmbitionGgrsConfig>>,
    mut control: ResMut<ControlFrame>,
    mut slots: Option<ResMut<ambition_characters::brain::SlotControls>>,
) {
    for (handle, (input, _)) in inputs.iter().enumerate() {
        if handle == 0 {
            *control = *input;
            continue;
        }
        if let Some(slots) = slots.as_deref_mut() {
            slots.set(ambition_characters::brain::PlayerSlot(handle as u8), *input);
        }
    }
    if inputs.is_empty() {
        *control = ControlFrame::default();
    }
}

/// Publish the FACT "this frame number has been simulated before".
///
/// Deliberately a fact, not a policy — but note how few consumers it has left.
/// External effects (audio, VFX) no longer read it at all: "ran before" is not
/// "is settled", and answering the wrong question is what made the old
/// `SfxEmissionGate` keep phantoms and drop corrections. They now go through
/// [`crate::external_effects`], which defers rather than suppresses.
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
/// never be simulated again. `ConfirmedFrameCount` is maintained by `bevy_ggrs`
/// for both session kinds (a P2P session's confirmed frame; `current -
/// check_distance` under sync test), so this works in the harness and online.
fn count_advance_run(
    frame: Res<RollbackFrameCount>,
    confirmed: Option<Res<ConfirmedFrameCount>>,
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
        boundary.confirmed = confirmed.map_or(-1, |confirmed| confirmed.0);
    }
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
) {
    status
        .mismatch_frames
        .extend(trigger.event().mismatched_frames.iter().copied());
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
}

fn live_content_identity(world: &mut World) -> Option<PreparedContentIdentity> {
    let mut query = world.query::<&PreparedContentIdentity>();
    query.iter(world).next().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **One call reaches whichever seam the host actually reads.**
    ///
    /// The three cases are not interchangeable and getting one wrong is silent:
    /// the driver keeps driving, the sim never sees the input, and nothing
    /// reports a thing. Each arm is asserted separately because "it did not
    /// panic" is not the claim — "it landed in the resource this host consumes"
    /// is.
    #[test]
    fn the_driver_seam_writes_whichever_resource_this_host_reads() {
        let pressed = ControlFrame {
            axis_x: 1.0,
            ..Default::default()
        };

        // A driver with no device under a fixed-tick host: `ControlFrame` IS the
        // input the sim reads.
        let mut fixed = World::new();
        fixed.insert_resource(ControlFrame::default());
        drive_control_frame(&mut fixed, pressed);
        assert_eq!(fixed.resource::<ControlFrame>().axis_x, 1.0);

        // The same driver under GGRS. `ControlFrame` is an OUTPUT there —
        // `publish_ggrs_input` overwrites it from the session's confirmed inputs
        // every advance — so the input must land in `PendingLocalInput`, and
        // must NOT be written to `ControlFrame`, or a driver would be feeding
        // resimulated input back in as new input.
        let mut rollback = World::new();
        rollback.insert_resource(ControlFrame::default());
        rollback.insert_resource(PendingLocalInput::default());
        drive_control_frame(&mut rollback, pressed);
        assert_eq!(rollback.resource::<PendingLocalInput>().0.axis_x, 1.0);
        assert_eq!(
            rollback.resource::<ControlFrame>().axis_x,
            0.0,
            "a driver must not write the resource GGRS publishes into"
        );

        // A device-backed host: the latch wins, so nudging a windowed build does
        // not fight the device layer for the same resource.
        let mut windowed = World::new();
        windowed.insert_resource(ControlFrame::default());
        windowed.insert_resource(PendingLocalInput::default());
        windowed.insert_resource(ControlFrameLatch::default());
        drive_control_frame(&mut windowed, pressed);
        assert_eq!(windowed.resource::<ControlFrameLatch>().peek().axis_x, 1.0);
        assert_eq!(windowed.resource::<PendingLocalInput>().0.axis_x, 0.0);
    }

    /// The "no world to rewind" detector fires exactly when construction has not
    /// happened, and stays quiet once it has.
    ///
    /// The condition is worth a test rather than a comment because the SYMPTOM it
    /// explains is unattributable: GGRS reports a checksum difference on frames
    /// 2, 3 and 4 and cannot know that a room was being built inside the window.
    /// A detector that also fired on healthy sessions would be worse than none —
    /// a warning every correct host prints is a warning nobody reads.
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
            RollbackSessionOwnership::LocalSyncTest(SyncTestSettings {
                check_distance: 0,
                max_prediction_window: 8,
                ..SyncTestSettings::for_players(1)
            })
        );
        assert_eq!(
            world.resource::<Time<GgrsTime>>().elapsed(),
            std::time::Duration::ZERO,
            "a new frame-zero session must not retain elapsed time from the old timeline"
        );
    }

    /// **The atomicity seam.** The fallible half of a rebase
    /// (`build_sync_test_session`) takes no `World` and can therefore be
    /// completed BEFORE any destructive mutation; the world-touching half
    /// (`install_rebased_sync_test_session`) is infallible. A caller (the
    /// confirmed lifecycle commit) that builds first and only mutates + installs
    /// once the build succeeded can never half-commit. This test proves the two
    /// halves compose into the same end state the fused wrapper produces — and,
    /// by its very signature, that the build needs no world to fail against.
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

        install_rebased_sync_test_session(&mut world, session, settings);

        assert_eq!(world.resource::<RollbackFrameCount>().0, 0);
        assert_eq!(
            world.resource::<Time<GgrsTime>>().elapsed(),
            std::time::Duration::ZERO,
            "install resets the clock exactly as the fused path does"
        );
        assert_eq!(
            *world.resource::<RollbackSessionOwnership>(),
            RollbackSessionOwnership::LocalSyncTest(settings),
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
            .init_resource::<ControlFrameLatch>()
            .init_resource::<PendingLocalInput>()
            .init_resource::<LocalPlayers>();
        install_session_bridge(&mut app);

        {
            let mut latch = app.world_mut().resource_mut::<ControlFrameLatch>();
            latch.accumulate(ControlFrame {
                jump_pressed: true,
                jump_held: true,
                ..default()
            });
            // A later rendered frame sees the button released, but no GGRS
            // tick requested input between these samples.
            latch.accumulate(ControlFrame::default());
        }

        assert_eq!(
            app.world().resource::<PendingLocalInput>().0,
            ControlFrame::default(),
            "render-frame sampling must not consume the tick latch"
        );

        app.world_mut().run_schedule(ReadInputs);
        let first = app.world().resource::<PendingLocalInput>().0;
        assert!(
            first.jump_pressed,
            "the short press must reach the next GGRS tick"
        );
        assert!(!first.jump_held, "the latest button level is released");

        app.world_mut().run_schedule(ReadInputs);
        assert!(
            !app.world().resource::<PendingLocalInput>().0.jump_pressed,
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
    /// republished every advance from GGRS's own counters.
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
    use ambition_characters::brain::{PlayerSlot, SlotControls};

    fn frame_with_axis(axis_x: f32) -> ControlFrame {
        ControlFrame {
            axis_x,
            ..ControlFrame::default()
        }
    }

    /// **Every local handle gets its OWN frame.** (queue Y1)
    ///
    /// `publish_local_inputs` inserted `pending.0` for every handle, which was
    /// correct while there was exactly one and silently wrong the moment there
    /// were two: four pads would drive one input stream and the sync test would
    /// checksum-compare a simulation nobody was playing.
    #[test]
    fn each_local_handle_submits_its_own_input_stream() {
        let mut app = App::new();
        app.insert_resource(PendingLocalInput(frame_with_axis(1.0)));
        let mut seats = PendingSeatInputs::default();
        seats.set(1, frame_with_axis(-1.0));
        app.insert_resource(seats);
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
        app.insert_resource(PendingLocalInput(frame_with_axis(1.0)));
        app.insert_resource(LocalPlayers(vec![0, 1]));
        app.add_systems(Update, publish_local_inputs);
        app.update();

        let inputs = app.world().resource::<LocalInputs<AmbitionGgrsConfig>>();
        assert_eq!(inputs.0.get(&1).map(|frame| frame.axis_x), Some(0.0));
    }

    /// **The player count is what the session builds with**, clamped to the
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

    /// Seats 1.. land in `SlotControls`, which is where `tick_player_brains`
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

    /// **A new session inherits an unhealthy timeline's reason.** (AC23)
    ///
    /// The defect: installing a session wrote `RollbackSessionStatus::default()`
    /// unconditionally, so a divergence reported on the old timeline vanished
    /// the instant a new session replaced it. One of four call sites guarded
    /// against that, with a comment explaining exactly why it had to — a seam
    /// asking every caller to remember a rule the seam can enforce.
    #[test]
    fn an_invalidated_session_hands_its_reason_to_its_replacement() {
        let previous = RollbackSessionStatus {
            mismatch_frames: Vec::new(),
            invalidation: Some("room reconstructed under a live timeline".to_string()),
        };
        let carried = RollbackSessionStatus::carried_from(Some(&previous));
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
        let reason = carried.invalidation.expect("the mismatch survives as prose");
        assert!(reason.contains("41"), "the reason lost the evidence: {reason}");
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
