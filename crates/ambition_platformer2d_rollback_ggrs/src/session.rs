//! GGRS session/input bridge shared by the harness and future network hosts.

use bevy::prelude::*;
use bevy_ggrs::ggrs::{self, PlayerType, SessionBuilder};
use bevy_ggrs::{
    ConfirmedFrameCount, GgrsConfig, GgrsSchedule, GgrsTime, LoadWorld, LocalInputs, LocalPlayers,
    PlayerInputs, ReadInputs, RollbackFrameCount, RunGgrsSystems, Session, SyncTestMismatch,
};

use ambition_platformer2d_core::{ConfirmedFrameBoundary, ControlFrame};

use super::RollbackRegistry;
use crate::PreparedContentIdentity;

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

/// Ambition's rollback authority vocabulary, re-exported at the seam that
/// installs it. The types themselves are backend-neutral and live in the
/// runtime beside the schema registry — see
/// [`ambition_platformer2d_runtime::rollback::authority`] for the lifetime
/// model they encode.
pub use ambition_platformer2d_runtime::{
    ActiveRollbackAuthority, RollbackDiagnostic, RollbackDiagnosticHistory,
    RollbackTimelineContract, RollbackTimelineGeneration, RollbackTimelineStatus,
};

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
) -> Result<(), StartSyncTestError> {
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
) -> Result<(), StartSyncTestError> {
    // GGRS construction touches no world, so it runs first and a rejected
    // setting cannot leave a half-installed timeline.
    let session = build_sync_test_session(settings)?;
    // ⛔ AND THE SECOND FAILURE IS ABOUT THE WORLD, NOT THE SETTINGS. Both are
    // asked before anything is installed, so a refusal leaves the world exactly
    // as it was.
    let eligibility = FrameZeroEligibility::check(world)?;
    install_rebased_sync_test_session(world, session, settings, owner, eligibility);
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
/// Installation returns nothing and has no recoverable failure — ⚠ **but it is
/// not unconditionally safe, and this sentence has now been wrong twice.** For a
/// day it read "cannot fail" while the function returned `Result`. Then it read
/// that the [`FrameZeroEligibility`] precondition made the work infallible,
/// which the 2026-09-17 review took apart: the token proves the check RAN, and a
/// caller can invalidate it before getting here — the lifecycle road rebuilds a
/// whole room between the two calls. ⇒ What is true is that the fallible,
/// RECOVERABLE step happens in [`FrameZeroEligibility::check`], one call
/// earlier, while the old session is still alive; and that this function takes
/// the census AGAIN, unconditionally, before its first write, where a hidden
/// candidate is an invariant failure. Rebase resets frame counters and
/// `Time<GgrsTime>` before installing the session.
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
        "starting a rollback session with no session world: frame zero is an EMPTY world, so \
         the construction that runs next happens inside the rollback window. A rollback \
         cannot undo `Commands`, so the frames that build the room will mismatch on every \
         resimulation and GGRS will report it only as a checksum difference. Activate the \
         session world first, then start the session — it rebases onto whatever is live."
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

/// What a carrier-order rebase found and did. Returned so a caller can assert on
/// it rather than read a log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollbackOrderRebase {
    /// Live rollback carriers at the moment of the rebase.
    pub carriers: usize,
    /// How many of them carry a canonical [`SimId`](ambition_platformer2d_shared_tangle::sim_id::SimId).
    ///
    /// ⚠ Zero and MEANINGLESS when `hidden_candidates` is non-zero: nothing was
    /// enumerated, because nothing was rebased.
    pub identified: usize,
    /// Orders this App had handed out before the rebase — including every one
    /// belonging to an entity that has since been despawned. The gap between
    /// this and `carriers` IS the host-local history being discarded.
    pub discarded_history: usize,
    /// Rollback carriers hidden from ordinary queries as construction
    /// candidates. ⛔ NON-ZERO MEANS NOTHING WAS REBASED — see
    /// [`rebase_rollback_carrier_order`].
    pub hidden_candidates: usize,
}

/// How many rollback carriers a frame-zero enumeration can and cannot see.
///
/// ⛔ ONE COUNT, TWO READERS. The frame-zero PRECONDITION and the rebase itself
/// both need this, and two spellings of "how many are hidden" is how the two
/// disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarrierCensus {
    /// What an ordinary `With<Rollback>` query sees.
    pub visible: usize,
    /// What a query that also allows `InactiveCandidate` sees.
    pub with_candidates: usize,
    /// The difference — carriers an ordinary enumeration cannot see.
    pub hidden_candidates: usize,
}

pub fn census_rollback_carriers(world: &mut World) -> CarrierCensus {
    use bevy_ggrs::Rollback;
    let visible = world
        .query_filtered::<Entity, With<Rollback>>()
        .iter(world)
        .count();
    // `Allow<T>` is "with AND without", so the difference is exactly the carriers
    // an ordinary query cannot see.
    let with_candidates =
        ambition_platformer2d_shared_tangle::construction::count_matching_including_hidden_candidates::<
            With<Rollback>,
        >(world);
    CarrierCensus {
        visible,
        with_candidates,
        hidden_candidates: with_candidates.saturating_sub(visible),
    }
}

/// Why a frame-zero installation refused, with the numbers that decided it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameZeroRefused {
    pub hidden_candidates: usize,
    pub carriers: usize,
}

impl std::fmt::Display for FrameZeroRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "refusing to declare frame zero: {} of {} rollback carriers are hidden \
             construction candidates, so a rebased order cannot describe the whole \
             population",
            self.hidden_candidates, self.carriers
        )
    }
}

impl std::error::Error for FrameZeroRefused {}

/// Proof that this world can declare frame zero, obtained BEFORE anything
/// destructive runs.
///
/// ⛔⛤ **IT IS A TOKEN BECAUSE A REPORTED-BUT-UNREACHABLE FAILURE IS STILL A
/// FAILURE SOMEBODY HAS TO HANDLE, AND EVERY HANDLER OF IT WAS WRONG.** Replacing
/// a session is destroy-then-install. When the install itself could return
/// `Err`, both callers had a branch for a refusal arriving AFTER the destructive
/// half — and in that state the room is authoritative with the previous
/// timeline's order history installed, or there is no session at all. The
/// 2026-09-17 architecture review put it plainly: *"Continuing execution from
/// there is worse than terminating."*
///
/// ⇒ So the refusal moved to where it can be acted on. [`Self::check`] is the
/// only way to build one, its field is private, and
/// [`install_rebased_sync_test_session`] takes one by value — which makes
/// "install without having checked" unspellable, and deletes both callers'
/// unactionable post-destructive branches. ⛔ It does NOT make the install
/// unconditionally safe — see the paragraph below, and the unconditional census
/// at the top of [`install_rebased_sync_test_session`] that does.
///
/// ⛔⛤ **WHAT IT DOES NOT PROVE IS THAT THE WORLD HAS NOT CHANGED SINCE, AND AN
/// EARLIER VERSION OF THIS PARAGRAPH CLAIMED OTHERWISE.** It said `&mut World` is
/// exclusive so nothing can intervene. That is true of other SYSTEMS and false
/// of the caller: `maintain_lifecycle_commit` checks eligibility, runs
/// `execute_lifecycle_commit` — which rebuilds a room — and only then installs.
/// Exclusive access says nothing about that middle step. The 2026-09-17 review
/// put it exactly: *"it does not mean the caller cannot invalidate the predicate
/// between those two calls."*
///
/// ⇒ So this token is a RECOVERABLE PREFLIGHT and nothing more. It exists so a
/// caller can find out while its old session is still alive and keep it on a
/// refusal. [`install_rebased_sync_test_session`] takes the census again,
/// unconditionally, before its first destructive write, and a hidden candidate
/// there is an invariant failure rather than a `Result` — see the comment at that
/// assertion for why `debug_assert` was the wrong instrument.
///
/// ⚠ It stays `Copy`, deliberately. Making it move-only would stop a caller
/// re-using one token for two installs, which is a real mistake, but it would
/// read as *"holding this makes the install safe"* — and that is the claim the
/// review found to be false. The unconditional census is what makes the install
/// safe; this type only makes the refusal reachable.
#[derive(Debug, Clone, Copy)]
pub struct FrameZeroEligibility {
    carriers: usize,
}

impl FrameZeroEligibility {
    /// Ask whether frame zero can describe the whole carrier population.
    ///
    /// Reads the world and writes nothing, so a caller may ask while a live
    /// session is still authoritative and keep it on a refusal.
    pub fn check(world: &mut World) -> Result<Self, FrameZeroRefused> {
        let census = census_rollback_carriers(world);
        if census.hidden_candidates > 0 {
            return Err(FrameZeroRefused {
                hidden_candidates: census.hidden_candidates,
                carriers: census.with_candidates,
            });
        }
        Ok(Self {
            carriers: census.with_candidates,
        })
    }

    /// The population the check was taken over, for a caller's own logging.
    pub fn carriers(self) -> usize {
        self.carriers
    }
}

/// Starting a local sync-test session has two ways to fail and they are not the
/// same kind of thing.
///
/// ⛔⛤ **GGRS CONSTRUCTION FAILING AND THE WORLD BEING UNFIT FOR FRAME ZERO ARE
/// DIFFERENT ANSWERS, AND FOLDING THE SECOND INTO `GgrsError` WOULD HAVE MEANT
/// INVENTING A VARIANT FOR IT.** The first is about the arguments; the second is
/// about the world, is recoverable by waiting for the candidate to publish, and
/// a caller that wants to retry needs to be able to tell them apart.
#[derive(Debug)]
pub enum StartSyncTestError {
    /// GGRS refused the settings. Touches no world.
    Ggrs(ggrs::GgrsError),
    /// The world is not in a state that can declare frame zero. Nothing was
    /// mutated and no session was installed.
    FrameZeroRefused(FrameZeroRefused),
}

impl std::fmt::Display for StartSyncTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ggrs(error) => write!(f, "{error}"),
            Self::FrameZeroRefused(refusal) => write!(f, "{refusal}"),
        }
    }
}

impl std::error::Error for StartSyncTestError {}

impl From<ggrs::GgrsError> for StartSyncTestError {
    fn from(error: ggrs::GgrsError) -> Self {
        Self::Ggrs(error)
    }
}

impl From<FrameZeroRefused> for StartSyncTestError {
    fn from(refusal: FrameZeroRefused) -> Self {
        Self::FrameZeroRefused(refusal)
    }
}

/// Rebase the GGRS carrier ORDER onto the live rollback population.
///
/// ⛔⛔ **THE COMPONENT CHECKSUM TWO PEERS COMPARE CONTAINS AN APP-LIFETIME
/// INSERTION INDEX, AND WITHOUT THIS IT NEVER RESETS.** `ComponentChecksumPlugin`
/// hashes `RollbackOrdered.order(rollback_id)` together with the value projection
/// before XORing carriers together, and `RollbackOrdered` assigns each
/// `RollbackId` an index the first time `Rollback` is added and keeps every index
/// it ever handed out — despawned entities included. MEASURED 2026-09-17 on two
/// hosts that reach the shipped Ambition route by different shell histories: the
/// same 22 canonical identities at orders `0..21` against `74..95`, and **59 of
/// 146 real `ChecksumPart`s disagreeing** while every value agreed.
///
/// ⇒ A session declaring frame zero is exactly where that history must stop
/// contributing, which is why this runs beside the frame counters rather than at
/// teardown: the rebase is a property of the timeline being STARTED.
///
/// ⚠ **THE KEY IS THE CANONICAL IDENTITY, NOT `RollbackId`.** `RollbackId` is the
/// Bevy `Entity` that first received `Rollback`, so ordering by it would replace
/// insertion-history dependence with allocation-order dependence — the same
/// defect one layer down, which is the trap the architecture review named. `SimId`
/// is the peer-stable key the live population already carries. The previous order
/// is the TIE-BREAK.
///
/// ⛔⛤ **THE REASON GIVEN FOR THAT TIE-BREAK WAS WRONG AND IS WORTH RECORDING:**
/// *"a candidate world and the live world may legitimately carry the same
/// canonical identity at the same moment."* True of the WORLD and false of this
/// query, which cannot see a candidate at all — see the refusal below. The
/// tie-break is what makes the sort total on a population that should not
/// contain a duplicate; `rollback_populated_timeline.rs` measures that every
/// visible anchor's `SimId` is unique, so the tie-break is a fallback for a
/// state that arm says does not occur, and BOTH of its keys are host-local.
///
/// ⛔⛤ **AND IT REFUSES WHILE A CANDIDATE WORLD IS IN FLIGHT, BECAUSE THE
/// ENUMERATION BELOW CANNOT SEE ONE.** `InactiveCandidate` is a registered
/// DISABLING component, so an ordinary `With<Rollback>` query skips every
/// candidate root — and this repository already proves GGRS-shaped ordinary
/// queries cannot see them
/// (`construction/tests.rs::a_ggrs_shaped_ordinary_query_cannot_see_a_candidate`).
/// A rebase taken over the visible half would leave a hidden carrier holding its
/// `RollbackId` and absent from the rebuilt table, and `RollbackOrdered::order`
/// PANICS for an id it does not know — so the crash would arrive later, on the
/// checksum after that candidate was published.
///
/// ⇒ **The fix is not to include them.** The sort key is peer-stable
/// (`SimId`), but an INDEX is positional: a candidate present on one peer and
/// absent on the other shifts every order after it, which is the same
/// host-local dependence this function exists to remove, re-entered through the
/// front door. A timeline declaring frame zero mid-construction is the thing
/// that is wrong, so this reports it and changes nothing.
///
/// ⚠ **IT REBUILDS THE RESOURCE THROUGH THE ONLY DOOR UPSTREAM LEAVES OPEN.**
/// `RollbackOrdered::push` is private and there is no rebase API, so the order is
/// re-established by removing `RollbackId` and `Rollback` and re-adding
/// `Rollback`, whose `on_add` hook mints the id and pushes. The minted
/// `RollbackId` is `RollbackId::new(entity)` — the SAME value, for the same
/// entity — so nothing keyed on it moves; only the ordering changes. A small
/// upstream primitive would be better and this is what exists.
pub fn rebase_rollback_carrier_order(world: &mut World) -> RollbackOrderRebase {
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    use bevy_ggrs::{Rollback, RollbackId, RollbackOrdered};

    let previous = world.get_resource::<RollbackOrdered>().cloned();
    let discarded_history = previous.as_ref().map_or(0, RollbackOrdered::len);

    let CarrierCensus {
        visible,
        with_candidates,
        hidden_candidates,
    } = census_rollback_carriers(world);
    if hidden_candidates > 0 {
        bevy::log::error!(
            "REFUSING to rebase the rollback carrier order: {hidden_candidates} of \
             {with_candidates} rollback carriers are hidden construction candidates, which \
             this enumeration cannot see. Rebasing over the visible {visible} would leave \
             each hidden carrier holding a `RollbackId` absent from the rebuilt order, and \
             `RollbackOrdered::order` panics for an unknown id. A session must declare frame \
             zero with no candidate world in flight."
        );
        return RollbackOrderRebase {
            carriers: with_candidates,
            identified: 0,
            discarded_history,
            hidden_candidates,
        };
    }
    // ⚠ `RollbackId` IS OPTIONAL HERE ON PURPOSE. The `on_add` hook inserts it
    // through `Commands`, so a carrier spawned in the same frame can hold
    // `Rollback` with the id still queued. Requiring it would leave that carrier
    // out of the rebuilt ordering entirely, and `RollbackOrdered::order` PANICS
    // for an id it does not know — a crash on the next checksum rather than a
    // wrong number.
    let mut carriers: Vec<(Option<String>, u64, Entity)> = world
        .query_filtered::<(Entity, Option<&SimId>, Option<&RollbackId>), With<Rollback>>()
        .iter(world)
        .map(|(entity, sim_id, rollback)| {
            let previous_order = match (previous.as_ref(), rollback) {
                (Some(order), Some(rollback)) => order.order(*rollback),
                _ => 0,
            };
            (
                sim_id.map(|id| id.as_str().to_string()),
                previous_order,
                entity,
            )
        })
        .collect();
    let total = carriers.len();
    let identified = carriers.iter().filter(|(id, _, _)| id.is_some()).count();
    if identified != total {
        // ⚠ NOT A REFUSAL. Rebasing on a partly-unnamed population is still
        // strictly better than carrying another App's history, and refusing here
        // would leave the defect in place for the one composition that needs it
        // most. The gap is reported because a carrier the sim cannot name is the
        // same finding `ensure_sim_id` and `collect_perception_peers` report.
        //
        // ⛔⛤ **AND THAT ARGUMENT EXPIRES WHEN A SECOND PEER EXISTS.** A 2026-09-17
        // review put it plainly: this manufactures a result that cannot be
        // peer-stable, on the very population whose peer-stability is the point.
        // It is right about the destination and the trade is different today —
        // the alternative is not a refusal, it is KEEPING the App-lifetime
        // history, which is wrong in the same direction and by more. ⇒ The
        // condition that flips it is a real remote peer, not a date:
        // `rollback_populated_timeline.rs` already measures that every VISIBLE
        // rollback anchor in the populated world carries a unique `SimId`, so
        // when an external session road exists this branch should refuse to
        // install rather than fall back, and the duplicate-`SimId` tie-break
        // below (previous order, then `Entity` — both host-local) should go with
        // it. Doing that now would turn a diagnostic into a crash in the only
        // lane that runs.
        bevy::log::error!(
            "{} of {total} live rollback carriers have no canonical `SimId`, so \
             their place in the peer-compared carrier order falls back to this \
             App's construction order. Their spawn sites must mint an identity.",
            total - identified
        );
    }
    carriers.sort();
    world.insert_resource(RollbackOrdered::default());
    for (_, _, entity) in &carriers {
        let mut carrier = world.entity_mut(*entity);
        carrier.remove::<RollbackId>();
        carrier.remove::<Rollback>();
        carrier.insert(Rollback);
    }
    // The hook pushes into `RollbackOrdered` immediately and inserts the id
    // through `Commands`, so the ordering is already correct here and the ids
    // land on the entities at this flush.
    world.flush();
    RollbackOrderRebase {
        carriers: total,
        identified,
        discarded_history,
        hidden_candidates: 0,
    }
}

/// ⛔⛤ **IT REFUSES THE INSTALLATION, NOT MERELY THE REBASE — AND THE REFUSAL
/// NOW HAPPENS BEFORE YOU CAN CALL THIS AT ALL.** The first version of this
/// checked candidates inside [`rebase_rollback_carrier_order`], logged that the
/// session *"starts on this App's earlier order history"*, and installed it
/// anyway: it stopped building a partial order table that would panic later, and
/// in exchange declared a new frame-zero timeline carrying every rollback order
/// this App ever handed out — the ID-PEER defect the rebase exists to remove (59
/// of 146 differing GGRS checksum parts between equivalent hosts). The
/// architecture review of 2026-09-17 named it: *"It refuses the rebase, not the
/// session installation."*
///
/// The repair after that made this return `Result`, and a second review pass
/// found the cost: both callers then had a branch for a refusal arriving after
/// their destructive half, and neither could do anything useful in it. ⇒ The
/// check is [`FrameZeroEligibility::check`], the token it returns is the only way
/// to reach this function, and there is no RECOVERABLE failure here: a caller
/// that wants the session asks first and keeps what it has on a refusal.
///
/// ⛔⛤ **AND THE TOKEN IS NOT WHAT MAKES THAT SAFE, WHICH IS THE 2026-09-17
/// REVIEW'S FINDING.** It proves the check ran, not that its answer still holds
/// — so this function re-censuses unconditionally before its first destructive
/// write, and fails the invariant there. MEASURED with that gate deleted:
/// `cargo test --release` installs the session with a hidden carrier present
/// and no diagnostic at all, because the `debug_assert` beside the rebase is
/// compiled out. In debug the same deletion trips that `debug_assert` instead,
/// which is why the arm for this is read in BOTH profiles.
pub fn install_rebased_sync_test_session(
    world: &mut World,
    session: AmbitionGgrsSession,
    settings: SyncTestSettings,
    // Declared by the caller for the same reason as `start_sync_test_session_owned`:
    // a rebase keeps its owner, and inferring one here would guess.
    owner: SyncTestOwner,
    // ⛔ THE PRECONDITION, AS A VALUE. Its field is private, so holding one is
    // proof `FrameZeroEligibility::check` ran — and ran before whatever the
    // caller did to get here.
    eligibility: FrameZeroEligibility,
) {
    // ⛔⛔ **THE TOKEN IS A PREFLIGHT, NOT A CAPABILITY, AND THIS LINE IS WHERE
    // THAT DISTINCTION IS ENFORCED.** A `FrameZeroEligibility` proves the check
    // HAPPENED and happened before whatever the caller did next; it cannot prove
    // the caller did not then invalidate it. `&mut World` stops another SYSTEM
    // intervening — it does not stop this caller. The lifecycle road is the
    // proof: it checks eligibility, runs `execute_lifecycle_commit` (which
    // rebuilds a room), and only then installs. Exclusive access says nothing
    // about that middle step.
    //
    // ⇒ So the census is taken again HERE, unconditionally, BEFORE the first
    // destructive write below. It is an invariant failure and not a `Result`:
    // every road to this function has already reported `Committed`, so a hidden
    // candidate at this point is a broken promise by the caller and not a
    // recoverable condition. Recovering is what the token is for, one call
    // earlier, while the old session is still alive.
    //
    // ⚠ AND IT MUST BE UNCONDITIONAL, NOT A `debug_assert`. The failure mode in
    // release was the ID-PEER defect returning silently: the frame counters
    // reset, `rebase_rollback_carrier_order` declines to rebuild the order
    // because it cannot describe the hidden carriers, and the session installs
    // carrying every rollback order this App ever handed out — 59 of 146 GGRS
    // checksum parts differing between equivalent hosts. A guard that vanishes
    // in the build that ships is not guarding the case that matters.
    let precommit = census_rollback_carriers(world);
    assert_eq!(
        precommit.hidden_candidates, 0,
        "frame zero cannot be declared: {} of {} rollback carrier(s) are hidden \
         construction candidates. The caller holds a `FrameZeroEligibility` taken \
         over {} carrier(s), so the world was invalidated between that check and \
         this install. Nothing has been mutated yet — do every fallible step, and \
         every step that can publish or retire a candidate, BEFORE asking for the \
         install.",
        precommit.hidden_candidates,
        precommit.with_candidates,
        eligibility.carriers(),
    );
    warn_if_no_world_to_rewind(world);
    // A newly installed GGRS session always starts from the current live world
    // as frame zero. Snapshot stores are intentionally retained here: the first
    // SaveWorld request at frame zero replaces every non-negative frame in each
    // bevy_ggrs ring, while resetting these frame resources prevents that save
    // from being mislabeled with the previous session's frame number.
    world.insert_resource(RollbackFrameCount(0));
    world.insert_resource(ConfirmedFrameCount(-1));
    reset_input_authority(world);
    // ⛔ THE CARRIER ORDER IS PART OF WHAT FRAME ZERO REBASES. Without this the
    // checksum two peers compare carries every rollback entity this App ever
    // registered, retired sessions included — see `rebase_rollback_carrier_order`.
    let rebase = rebase_rollback_carrier_order(world);
    // ⚠ Unreachable now that the census above is unconditional and runs before
    // the first write: nothing between the two can add a carrier. Kept as a
    // `debug_assert` because the two readings bracket every mutation this
    // function makes, so a disagreement would mean one of those mutations
    // CREATES a hidden candidate — a different defect from the one above, and
    // the only one this position can still see.
    debug_assert_eq!(
        rebase.hidden_candidates, 0,
        "the unconditional census at the top of this function read 0 hidden \
         candidates and the rebase reads {}, so declaring frame zero is itself \
         creating construction candidates",
        rebase.hidden_candidates
    );
    bevy::log::debug!(
        target: "ambition_platformer2d::rollback",
        "carrier order rebased onto {} live carrier(s) ({} named), discarding {} \
         order(s) from this App's earlier history",
        rebase.carriers,
        rebase.identified,
        rebase.discarded_history
    );

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
///
/// ⛔⛔ **THIS ROAD DOES NOT DECLARE FRAME ZERO, SO IT DOES NOT REBASE — AND
/// WHOEVER BUILDS THE P2P ROAD MUST DECIDE WHICH IT IS.** A session installed
/// here inherits the frame counters and the carrier ordering as they stand,
/// deliberately: rebasing either one under a session that continues somebody
/// else's timeline would move the ground under the frames already agreed. ⇒ If
/// the P2P path starts a NEW synchronised timeline — which is what negotiating
/// a start tick means — it wants
/// [`install_rebased_sync_test_session`]'s shape, including
/// [`rebase_rollback_carrier_order`]. Without that call the peer-compared
/// checksum carries every rollback entity this App ever registered: measured
/// 2026-09-17 at 59 of 146 differing parts between two hosts whose canonical
/// identities and values were identical.
pub fn install_session(world: &mut World, session: AmbitionGgrsSession) {
    install_session_with_ownership(world, session, RollbackSessionOwnership::External);
}

fn install_session_with_ownership(
    world: &mut World,
    session: AmbitionGgrsSession,
    ownership: RollbackSessionOwnership,
) {
    // Borrowed for the same reason `enforce_session_contract` borrows: a clone's
    // memo starts empty, so reading through one leaves the world's registry
    // permanently uncached and every later frame pays the full hash.
    let schema = world
        .get_resource::<RollbackRegistry>()
        .map(RollbackRegistry::schema_fingerprint)
        .unwrap_or_else(|| RollbackRegistry::default().schema_fingerprint());
    // ⭐⭐ THE TIMELINE NAMES ITS OWNER AT INSTALL. Everything else follows from
    // this one line: which content it may bind to, whose health it carries, and
    // who is allowed to read the answer.
    let owner = ambition_platformer2d_shared_tangle::lifecycle::live_session_scope(world);
    let content = content_identity_of(world, owner);
    // AC23 and the cross-session rule are ONE rule, and it lives in
    // `ActiveRollbackAuthority::installed`. A caller supplies who and what,
    // never whether — a session install must not LAUNDER a divergence into a
    // clean baseline, and it must not INHERIT one from a game that has ended.
    let authority = ActiveRollbackAuthority::installed(
        world.get_resource::<ActiveRollbackAuthority>(),
        owner,
        RollbackTimelineContract { content, schema },
    );
    let generation = authority.generation().0;
    world.insert_resource(authority);
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

    // A new session is a new timeline. The generation lives on the authority
    // because it identifies the timeline the authority governs; the boundary
    // merely carries it to consumers, and teardown removes the boundary.
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
    world.remove_resource::<RollbackSessionOwnership>();
    // Nothing speculates any more, so external effects and persistence return
    // to their non-rollback behavior immediately. Leaving this installed would
    // strand pending effects and keep confirmed-state save gates closed forever.
    world.remove_resource::<ConfirmedFrameBoundary>();
    // ⭐ THE AUTHORITY STAYS, STOOD DOWN. The gameplay session that owned this
    // timeline has NOT ended — only the timeline has — so a divergence recorded
    // before the stop must keep refusing confirmed work. Removing the authority
    // here is what would let a desync launder itself through a teardown.
    // Retirement of the SCOPE is the thing that removes it; see
    // `retire_rollback_authority_with_its_scope`.
    if let Some(mut authority) = world.get_resource_mut::<ActiveRollbackAuthority>() {
        authority.stand_down_timeline();
    }
}

/// Retire the rollback authority governing `scope`, whether or not its timeline
/// is still running.
///
/// ⛔⛔ THIS IS TEARDOWN, NOT INVALIDATION. The gameplay session is over; its
/// timeline's health is a fact about a world that no longer exists. The
/// diagnosis is preserved in [`RollbackDiagnosticHistory`], which has no
/// gameplay authority, and the resource itself goes — so nothing can be
/// inherited even by accident.
fn retire_rollback_authority(world: &mut World) {
    stop_session(world);
    world.remove_resource::<ActiveRollbackAuthority>();
}

/// Queue the exact same teardown from a regular Bevy system.
pub fn stop_session_deferred(commands: &mut Commands) {
    commands.queue(|world: &mut World| stop_session(world));
}

/// Return a diagnostic error when GGRS invalidated the session contract or a
/// sync-test checksum mismatch was observed.
pub fn session_health(world: &World) -> Result<(), String> {
    let Some(authority) = world.get_resource::<ActiveRollbackAuthority>() else {
        return Ok(());
    };
    let status = authority.status();
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

    // ⭐⭐ THE AUTHORITY DIES BEFORE THE WORLD IT GOVERNS. Named ordering, not an
    // ad-hoc edge: `SessionScopeSet::RetireAuthority` exists to say that an
    // authority stands down before `SessionScopeSet::Cleanup` removes the
    // entities it was governing.
    app.add_systems(
        Update,
        retire_rollback_authority_with_its_scope
            .in_set(
                ambition_platformer2d_shared_tangle::lifecycle::SessionScopeSet::RetireAuthority,
            )
            // ⛔⛔ THE MESSAGE IS A PREREQUISITE THIS PLUGIN DOES NOT SUPPLY, and
            // in Bevy 0.19 a `MessageReader` for an unregistered channel is a HARD
            // FAILURE that takes the whole `App` down — not a system that skips.
            // `SessionScopeRetired` is registered by the shared tangle's
            // session-lifecycle plugin
            // (`lifecycle/session.rs`), and a host may install the rollback
            // backend without it: `examples/capability_demo`'s
            // `rollback_round_trip` does exactly that, and died on frame one with
            // *"Parameter `…::messages` failed validation: Message not
            // initialized"* — a message that names neither the system nor the
            // channel.
            //
            // ⭐ DECLINING IS THE RIGHT ANSWER, NOT REGISTERING. A host with no
            // session-scope lifecycle retires no scopes, so there is nothing for
            // this system to do; adding the channel here would make this plugin a
            // second registrar of somebody else's vocabulary and give the message
            // two cleanup systems in a host that has both.
            //
            // ⚠ NOTHING IN THIS CRATE OBSERVES THIS REGISTRATION EITHER WAY, and
            // that is worth knowing before trusting a green suite about it.
            // `session_ownership_tests` schedules the system DIRECTLY, so it
            // tests the function rather than this line; `host_invariant_tests`
            // builds the plugin but only asserts schedule and resource facts. So
            // a future edit that dropped this system entirely would keep every
            // test in the crate green, and so would a `run_if` that never
            // becomes true.
            .run_if(bevy::prelude::resource_exists::<
                bevy::ecs::message::Messages<
                    ambition_platformer2d_shared_tangle::lifecycle::SessionScopeRetired,
                >,
            >),
    );

    app.add_systems(Update, report_input_written_to_the_wrong_seam);
    app.init_resource::<InputSeamMisuse>()
        .init_resource::<PendingSeatInputs>()
        .init_resource::<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>()
        .init_resource::<RollbackExecutionStats>()
        .init_resource::<RollbackDiagnosticHistory>()
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
        // ⛔⛤ **THE EDGE `Q120`'s FIRST FIX ASSERTED IN A COMMENT AND NEVER
        // DECLARED.** A developer edit to a value the simulation reads must be
        // decided and published BEFORE this host advances the timeline —
        // otherwise the old session simulates, and RESIMULATES HISTORY, against
        // mechanics it never ran with. The first version watched for the edit in
        // `Update`, after `RunGgrsSystems` had already advanced `GgrsSchedule`
        // with the new value.
        //
        // ⭐ The SETS come from `ambition_platformer2d_core`, beside the
        // resources, so the developer-tools crate can register proposers and
        // publishers into them without depending on this crate — and so a
        // composition with no rollback host still runs the same chain and
        // publishes by default.
        .configure_sets(
            PreUpdate,
            (
                ambition_platformer2d_core::MechanicalEditSet::Propose,
                ambition_platformer2d_core::MechanicalEditSet::Admit,
                ambition_platformer2d_core::MechanicalEditSet::Publish,
            )
                .chain()
                .before(RunGgrsSystems),
        )
        .init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>()
        .init_resource::<ambition_platformer2d_core::MechanicalEditAdmission>()
        .add_systems(
            PreUpdate,
            crate::local_session::decide_mechanical_edit_admission
                .in_set(ambition_platformer2d_core::MechanicalEditSet::Admit),
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

/// What the GGRS DRIVER itself costs, bracketed from outside it.
///
/// ⭐⭐ THE MEASUREMENT THAT SPLITS AN UNATTRIBUTED MILLISECOND. Measured
/// 2026-08-29 on a Smash match: the seventeen sim phases sum to ~0.93ms of a
/// 2.12ms `PreUpdate`, and the remaining ~1.2ms is in no sim phase at all. Two
/// families of explanation fit, and one measurement separates them:
///
/// - `driver - sum(sim_phases)` is time INSIDE this exclusive system but outside
///   the phase chain — the `ReadInputs` schedule, ggrs's own advance
///   bookkeeping, and any sim system registered outside the chain;
/// - `PreUpdate - driver` is time in `PreUpdate` but outside the driver
///   entirely — the `DefaultPlugins` population (ui focus, leafwing input,
///   picking, asset events).
///
/// ⛔ IT LIVES HERE, NOT IN THE CENSUS CRATE. `ambition_dev_tools` does not
/// depend on `bevy_ggrs` and must not start: bracketing `RunGgrsSystems` needs
/// that dependency, and this crate already has it. The cost is that the row is
/// emitted on a frame COUNT rather than the census's wall-clock interval, which
/// is the honest trade rather than a new dependency edge.
///
/// ⚠ NOT ROLLBACK STATE, deliberately — a rewind leaves last-branch timings in
/// it. Correct for an instrument, and it must never gate behaviour.
#[derive(Resource, Default)]
pub struct GgrsDriverCensus {
    entered: Option<std::time::Instant>,
    inside_ms: f64,
    frames: u32,
}

/// How many frames a driver row covers. Wall-clock intervals belong to the
/// census crate; this one counts frames because it cannot see that clock.
const GGRS_DRIVER_REPORT_EVERY: u32 = 300;

fn enter_ggrs_driver(mut census: ResMut<GgrsDriverCensus>) {
    census.entered = Some(std::time::Instant::now());
}

fn leave_ggrs_driver(mut census: ResMut<GgrsDriverCensus>) {
    let Some(entered) = census.entered.take() else {
        return;
    };
    census.inside_ms += entered.elapsed().as_secs_f64() * 1000.0;
    census.frames = census.frames.saturating_add(1);
    if census.frames < GGRS_DRIVER_REPORT_EVERY {
        return;
    }
    let per_frame = census.inside_ms / census.frames as f64;
    eprintln!(
        "[census] ggrs_driver frames={} inside_ms_per_frame={per_frame:.3}",
        census.frames
    );
    census.inside_ms = 0.0;
    census.frames = 0;
}

/// Install the driver bracket when the workload census is switched on.
///
/// ⛔ Registered only when asked, for the same reason the census's own phase
/// marks are: an instrument must not join the population it measures.
pub fn install_ggrs_driver_census(app: &mut App) {
    if std::env::var("AMBITION_PROFILE_CENSUS").is_err() {
        return;
    }
    eprintln!("[census] config ggrs_driver=installed");
    app.init_resource::<GgrsDriverCensus>();
    app.add_systems(
        bevy::app::PreUpdate,
        enter_ggrs_driver.before(RunGgrsSystems),
    );
    app.add_systems(
        bevy::app::PreUpdate,
        leave_ggrs_driver.after(RunGgrsSystems),
    );
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
    authority: Option<ResMut<ActiveRollbackAuthority>>,
    mut history: ResMut<RollbackDiagnosticHistory>,
) {
    let Some(mut authority) = authority else {
        return;
    };
    let frames: Vec<i32> = trigger.event().mismatched_frames.to_vec();
    authority.record_mismatch(frames.iter().copied());
    // The active authority refuses work; the history only remembers. See
    // `RollbackDiagnosticHistory` for why those are two values now.
    history.record(RollbackDiagnostic {
        scope: authority.owner(),
        generation: authority.generation(),
        reason: format!("GGRS sync-test checksum mismatch at frames {frames:?}"),
    });
}

/// Enforce the live timeline's contract against the world it actually governs.
///
/// ⭐⭐ THREE QUESTIONS, IN THIS ORDER, and the first one is the one that was
/// missing: *whose world is this?* A timeline whose gameplay session is no
/// longer live is not enforcing anything — it is standing down. Only once the
/// owner is confirmed live do "the schema changed" and "the content
/// disappeared" mean what they say.
///
/// ⛔⛔ RETIREMENT IS NOT CORRUPTION. Quitting a match to the title retires the
/// scope and removes its canonical root, which a contract that looked only at
/// "is a GGRS session installed?" read as an illegal mid-session content
/// disappearance — poisoning a timeline that had done nothing wrong, and
/// (before ownership) handing that poison to the next game. The scheduling
/// order that produced it is fixed too (`SessionScopeSet::RetireAuthority`),
/// but this is the half that holds when scheduling regresses.
pub(crate) fn enforce_session_contract(world: &mut World) {
    if !session_is_active(world) {
        return;
    }

    let live_scope = ambition_platformer2d_shared_tangle::lifecycle::live_session_scope(world);
    let Some(authority) = world.get_resource::<ActiveRollbackAuthority>() else {
        // A live timeline with no authority governs nobody and can authorize
        // nothing. Stand it down rather than let it run ownerless.
        retire_rollback_authority(world);
        return;
    };
    match (authority.owner(), live_scope) {
        // The ordinary case: this timeline governs the live session.
        (owner, live) if owner == live => {}
        // A timeline installed before its world was built adopts the first one
        // it sees — the ownership sibling of the content adoption below, and
        // permitted for the same reason (`warn_if_no_world_to_rewind` documents
        // the empty-world fixture that rebases frame zero onto a world it is
        // about to construct).
        (None, Some(adopted)) => {
            world
                .resource_mut::<ActiveRollbackAuthority>()
                .adopt_owner(adopted);
        }
        // The session this timeline governed is over, or another one is live.
        // Neither is a contract violation; both mean this authority is done.
        _ => {
            retire_rollback_authority(world);
            return;
        }
    }
    let owner = world.resource::<ActiveRollbackAuthority>().owner();

    // ⛔⛔ BORROWED, NOT CLONED. `RollbackRegistry`'s hand-written `Clone` starts
    // the memo `OnceLock` EMPTY on purpose, so `.cloned()` here handed
    // `schema_fingerprint()` a registry that could never be memoised: this frame
    // paid a ~450-entry BTreeMap clone AND rebuilt+hashed the whole ~40KB schema
    // dump, every frame, which is the 292us the memo was added to remove. The
    // cache only pays off on the resource that actually lives in the world.
    let current_schema = world
        .get_resource::<RollbackRegistry>()
        .map(RollbackRegistry::schema_fingerprint)
        .unwrap_or_else(|| RollbackRegistry::default().schema_fingerprint());
    let current_content = content_identity_of(world, owner);
    let contract = world
        .resource::<ActiveRollbackAuthority>()
        .contract()
        .clone();

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
            world
                .resource_mut::<ActiveRollbackAuthority>()
                .adopt_content(identity);
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
                    "canonical prepared content {:?} disappeared from the LIVE session root {:?} \
                     while the GGRS session was active",
                    expected, owner
                ),
            );
        }
        _ => {}
    }
}

/// Retire the authority governing a scope the shell has just retired.
///
/// ⭐ THE EXPLICIT SIGNAL, consumed where it belongs: an authority stands down
/// BEFORE the world it governs is cleaned up ([`SessionScopeSet::RetireAuthority`]
/// is ordered before [`SessionScopeSet::Cleanup`]). Polling for the absence of a
/// world is what made deliberate teardown look like corruption.
///
/// [`SessionScopeSet::RetireAuthority`]: ambition_platformer2d_shared_tangle::lifecycle::SessionScopeSet::RetireAuthority
/// [`SessionScopeSet::Cleanup`]: ambition_platformer2d_shared_tangle::lifecycle::SessionScopeSet::Cleanup
pub(crate) fn retire_rollback_authority_with_its_scope(
    mut retired: MessageReader<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeRetired>,
    authority: Option<Res<ActiveRollbackAuthority>>,
    mut commands: Commands,
) {
    let owner = authority
        .as_deref()
        .and_then(ActiveRollbackAuthority::owner);
    // Read the WHOLE batch, never `any()`: a short-circuit leaves later
    // retirements on the cursor to be re-read next frame.
    let mut retiring = false;
    for scope in retired.read() {
        retiring |= Some(scope.0) == owner;
    }
    if retiring {
        commands.queue(|world: &mut World| retire_rollback_authority(world));
    }
}

/// Record a divergence on the live timeline and stop it.
///
/// The timeline stops but its authority stays STOOD DOWN, so the diagnosis keeps
/// refusing confirmed work for the rest of this gameplay session. The same
/// reason is also copied into the process-lifetime
/// [`RollbackDiagnosticHistory`], which outlives the session and authorizes
/// nothing.
fn invalidate_session(world: &mut World, reason: String) {
    let record = world
        .get_resource::<ActiveRollbackAuthority>()
        .map(|authority| RollbackDiagnostic {
            scope: authority.owner(),
            generation: authority.generation(),
            reason: reason.clone(),
        });
    stop_session(world);
    if let Some(mut authority) = world.get_resource_mut::<ActiveRollbackAuthority>() {
        authority.invalidate(reason);
    }
    if let Some(record) = record {
        world
            .get_resource_or_insert_with::<RollbackDiagnosticHistory>(Default::default)
            .record(record);
    }
}

/// The prepared content identity of the canonical root belonging to `owner`.
///
/// ⛔⛔ NOT "THE FIRST `PreparedContentIdentity` IN THE WORLD". A global
/// first-match let a delayed root from a retired activation satisfy the current
/// session's contract, and let the current session's root go unseen while a
/// stale one was still being despawned. The contract belongs to a scope, so it
/// inspects that scope's root and no other.
fn content_identity_of(
    world: &mut World,
    owner: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
) -> Option<PreparedContentIdentity> {
    let owner = owner?;
    let mut query = world.query::<(
        &ambition_platformer2d_shared_tangle::lifecycle::SessionRoot,
        &PreparedContentIdentity,
    )>();
    query
        .iter(world)
        .find(|(root, _)| root.0 == owner)
        .map(|(_, identity)| *identity)
}

#[cfg(test)]
mod carrier_order_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    use bevy_ggrs::{Rollback, RollbackId, RollbackOrdered};

    /// A world holding `live` named carriers, after `retired` carriers have come
    /// and gone — which is what a shell host looks like on its third route.
    fn world_with(retired: usize, live: &[&str]) -> World {
        let mut world = World::new();
        world.init_resource::<RollbackOrdered>();
        for index in 0..retired {
            let entity = world.spawn((Rollback, SimId::placement(&format!("gone{index}")))).id();
            world.flush();
            world.despawn(entity);
        }
        // Spawned in REVERSE name order, so an arm that happens to agree with
        // spawn order is not agreeing by accident.
        for name in live.iter().rev() {
            world.spawn((Rollback, SimId::placement(name)));
        }
        world.flush();
        world
    }

    fn orders(world: &mut World) -> Vec<(String, u64)> {
        let pairs: Vec<(String, RollbackId)> = world
            .query::<(&SimId, &RollbackId)>()
            .iter(&world)
            .map(|(id, rollback)| (id.as_str().to_string(), *rollback))
            .collect();
        let ordered = world.resource::<RollbackOrdered>().clone();
        let mut out: Vec<(String, u64)> = pairs
            .into_iter()
            .map(|(name, rollback)| (name, ordered.order(rollback)))
            .collect();
        out.sort();
        out
    }

    /// ⛔ THE WHOLE POINT: two Apps that hold the same live carriers must hold
    /// the same ORDERS, however much history each of them threw away first.
    #[test]
    fn two_histories_reach_the_same_carrier_order() {
        let live = ["alpha", "beta", "gamma"];
        let mut fresh = world_with(0, &live);
        let mut veteran = world_with(74, &live);

        // The premise, and it is about HISTORY rather than about the defect: the
        // veteran has handed out orders the fresh App never will.
        assert_ne!(
            fresh.resource::<RollbackOrdered>().len(),
            veteran.resource::<RollbackOrdered>().len()
        );
        assert_ne!(orders(&mut fresh), orders(&mut veteran));

        rebase_rollback_carrier_order(&mut fresh);
        let report = rebase_rollback_carrier_order(&mut veteran);

        assert_eq!(report.carriers, 3);
        assert_eq!(report.identified, 3);
        assert_eq!(
            report.discarded_history, 77,
            "the veteran had handed out 74 retired orders plus its 3 live ones"
        );
        assert_eq!(
            orders(&mut fresh),
            vec![
                ("placement:alpha".to_string(), 0),
                ("placement:beta".to_string(), 1),
                ("placement:gamma".to_string(), 2),
            ],
            "the rebased order is the canonical-identity order, not the spawn order"
        );
        assert_eq!(orders(&mut fresh), orders(&mut veteran));
        assert_eq!(veteran.resource::<RollbackOrdered>().len(), 3);
    }

    /// ⛔ A REBASE MUST NOT CHANGE ROLLBACK IDENTITY. Every snapshot in the
    /// engine is keyed on `RollbackId`, so if the rebuild minted new ones it
    /// would silently detach every stored component from its carrier.
    #[test]
    fn a_rebase_keeps_every_rollback_id() {
        let mut world = world_with(2, &["alpha", "beta"]);
        let before: std::collections::BTreeMap<String, RollbackId> = world
            .query::<(&SimId, &RollbackId)>()
            .iter(&world)
            .map(|(id, rollback)| (id.as_str().to_string(), *rollback))
            .collect();
        rebase_rollback_carrier_order(&mut world);
        let after: std::collections::BTreeMap<String, RollbackId> = world
            .query::<(&SimId, &RollbackId)>()
            .iter(&world)
            .map(|(id, rollback)| (id.as_str().to_string(), *rollback))
            .collect();
        assert_eq!(before, after);
        assert_eq!(before.len(), 2);
    }

    /// ⛔⛤ **TWO CARRIERS SHARING A `SimId` FALL BACK TO THIS APP'S SPAWN ORDER,
    /// AND THAT IS STABLE BY COINCIDENCE RATHER THAN BY CONSTRUCTION.** The sort
    /// is `(SimId, previous order, Entity)`: the first key is peer-stable and the
    /// other two are not. On a duplicate the result is decided entirely by the
    /// two host-local keys.
    ///
    /// ⚠ **I FIRST WROTE THIS AS A DISAGREEMENT ARM AND IT WAS VACUOUS.** It
    /// compared raw `Entity` values from two different `World`s, which differ
    /// whatever the ordering does — and the claim was wrong as well as
    /// unmeasured: `previous order` is monotonic in spawn order, so a fresh App
    /// and a veteran one put the same duplicate first. The disagreement needs a
    /// history where the relative previous order INVERTS, which plain spawning
    /// cannot produce.
    ///
    /// ⇒ So this pins what is actually true: the tie-break is TOTAL (nothing is
    /// dropped, no index is shared) and it reproduces the local spawn order. Two
    /// peers running the same construction agree because their construction
    /// order matches, not because anything compared here is peer-stable — the
    /// same shape as an invariant held by coincidence, recorded so the next
    /// reader does not mistake the agreement for a guarantee.
    ///
    /// ⚠ `rollback_populated_timeline.rs` measures that every visible rollback
    /// anchor in the populated world carries a UNIQUE `SimId`, so this state does
    /// not occur today. When frame-zero installation refuses on a duplicate
    /// instead of tie-breaking, this arm is what says what is being replaced.
    #[test]
    fn two_carriers_sharing_one_sim_id_keep_this_app_s_spawn_order() {
        /// Returns the two same-named carriers in SPAWN order, which is the only
        /// fact that distinguishes them inside one world.
        fn world_with_duplicates(retired: usize) -> (World, Entity, Entity) {
            let mut world = World::new();
            world.init_resource::<RollbackOrdered>();
            for index in 0..retired {
                let entity = world
                    .spawn((Rollback, SimId::placement(&format!("gone{index}"))))
                    .id();
                world.flush();
                world.despawn(entity);
            }
            let first = world.spawn((Rollback, SimId::placement("alpha"))).id();
            world.flush();
            let second = world.spawn((Rollback, SimId::placement("alpha"))).id();
            world.flush();
            (world, first, second)
        }

        fn index_of(world: &mut World, entity: Entity) -> u64 {
            let table = world.resource::<RollbackOrdered>().clone();
            let id = *world
                .get::<RollbackId>(entity)
                .expect("the rebase re-adds `Rollback`, whose hook mints the id");
            table.order(id)
        }

        let (mut fresh, fresh_first, fresh_second) = world_with_duplicates(0);
        let (mut veteran, veteran_first, veteran_second) = world_with_duplicates(7);

        let report = rebase_rollback_carrier_order(&mut fresh);
        assert_eq!(report.carriers, 2);
        assert_eq!(
            report.identified, 2,
            "both carriers are named; the ambiguity is that they are named the SAME"
        );
        rebase_rollback_carrier_order(&mut veteran);

        // TOTAL: nothing dropped, and the two indices are distinct.
        assert_eq!(fresh.resource::<RollbackOrdered>().len(), 2);
        assert_eq!(veteran.resource::<RollbackOrdered>().len(), 2);
        assert_eq!(
            (
                index_of(&mut fresh, fresh_first),
                index_of(&mut fresh, fresh_second)
            ),
            (0, 1),
            "the duplicate pair must occupy distinct indices; sharing one would \
             make two carriers one row of the peer checksum"
        );
        assert_eq!(
            (
                index_of(&mut veteran, veteran_first),
                index_of(&mut veteran, veteran_second)
            ),
            (0, 1),
            "the veteran App reproduces the SAME relative order — and it does so \
             because `previous order` is monotonic in spawn order, not because \
             either key is peer-stable"
        );
    }

    /// ⛔⛤ **THE ARM THE OTHER ONES COULD NOT FAIL: `world_with` NEVER REGISTERS
    /// THE DISABLING FILTER, so every carrier it builds is visible and no arm in
    /// this module could tell a candidate-blind enumeration from a complete
    /// one.** With the filter installed and one carrier hidden, an ordinary
    /// `With<Rollback>` query returns 1 where the world holds 2 — and a rebase
    /// taken over that 1 would leave the hidden carrier holding a `RollbackId`
    /// absent from the rebuilt order, which `RollbackOrdered::order` panics on.
    ///
    /// ⛔ POISONED: with the `hidden_candidates > 0` refusal disabled the
    /// rebuilt table holds ONE entry where the world has two carriers, which is
    /// the state that panics later, on the checksum after the candidate is
    /// published. ⚠ The assertions are ordered so THAT is what fails: checking
    /// the report first made the poison fail on `hidden_candidates: 0`, which is
    /// the field announcing the property rather than the property.
    #[test]
    fn a_hidden_candidate_carrier_stops_the_rebase_instead_of_being_dropped() {
        use ambition_platformer2d_shared_tangle::construction::{
            hide_candidate_session_root, register_inactive_candidate_filter,
        };

        let mut world = world_with(0, &["alpha", "beta"]);
        register_inactive_candidate_filter(&mut world);
        let hidden = world
            .query_filtered::<Entity, With<Rollback>>()
            .iter(&world)
            .next()
            .expect("the fixture built carriers");
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = bevy::prelude::Commands::new(&mut queue, &world);
        hide_candidate_session_root(&mut commands, hidden);
        queue.apply(&mut world);

        // The premise: the ordinary query the rebase uses cannot see it.
        assert_eq!(
            world
                .query_filtered::<Entity, With<Rollback>>()
                .iter(&world)
                .count(),
            1,
            "the disabling filter is not installed, so this arm would pass \
             against a candidate-blind rebase as well as a candidate-aware one"
        );

        let before = world.resource::<RollbackOrdered>().len();
        let report = rebase_rollback_carrier_order(&mut world);
        // ⛔ THE DEFECT-DESCRIBING ASSERTION GOES FIRST, so the poison fails on
        // the PROPERTY rather than on the report field that announces it.
        assert_eq!(
            world.resource::<RollbackOrdered>().len(),
            before,
            "a refused rebase must leave the order alone; replacing it drops \
             the hidden carrier's entry, and `RollbackOrdered::order` panics \
             for an unknown id on the next checksum"
        );
        assert_eq!(report.hidden_candidates, 1);
        assert_eq!(report.carriers, 2, "the report names the WHOLE population");
    }

    /// ⛔⛤ **AND THE ARM ABOVE IS ABOUT THE REBASE, WHICH IS NOT THE ROAD A
    /// SESSION TAKES.** The architecture review of 2026-09-17 named the gap
    /// exactly: *"It refuses the rebase, not the session installation."* The
    /// install road reset `RollbackFrameCount`, the confirmation counter, the
    /// input authority and `GgrsTime`, saw the refusal, logged that the session
    /// *"starts on this App's earlier order history"*, and installed it — trading
    /// a later panic for a new frame-zero timeline carrying every order this App
    /// ever handed out, which is the ID-PEER defect the rebase exists to remove.
    ///
    /// ⇒ This arm calls the road a session actually starts on and asserts the
    /// five things a refusal must leave alone. ⚠ The counters are asserted
    /// against values that are NOT the post-install ones, so a refusal that
    /// reset them would fail here rather than coincide with a plausible number:
    /// the fixture sets frame 77 and confirmed 41, and the install road's own
    /// values are 0 and −1.
    ///
    /// ⚠ **AND THE RECOVERABLE REFUSAL IS NO LONGER IN THE INSTALL**, which is
    /// why this arm drives `start_sync_test_session_owned`. A second review pass
    /// found that a fallible install left both callers with an unactionable
    /// post-destructive branch, so the recoverable half moved into
    /// `FrameZeroEligibility::check`. What this arm tests is that the ROAD
    /// declines and the world is untouched.
    ///
    /// ⇒ The install still refuses, as an INVARIANT rather than a `Result`, for
    /// the case this arm cannot reach: a caller that checks, invalidates its own
    /// answer, and then installs. That is
    /// `an_install_whose_preflight_was_invalidated_panics_before_touching_frame_zero`.
    #[test]
    fn a_hidden_candidate_refuses_the_installation_and_mutates_nothing() {
        use ambition_platformer2d_shared_tangle::construction::{
            hide_candidate_session_root, register_inactive_candidate_filter,
        };

        let mut world = world_with(0, &["alpha", "beta"]);
        register_inactive_candidate_filter(&mut world);
        let hidden = world
            .query_filtered::<Entity, With<Rollback>>()
            .iter(&world)
            .next()
            .expect("the fixture built carriers");
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = bevy::prelude::Commands::new(&mut queue, &world);
        hide_candidate_session_root(&mut commands, hidden);
        queue.apply(&mut world);

        // Distinctive values, so a reset is visible as a reset rather than as a
        // number that could have been there all along.
        world.insert_resource(RollbackFrameCount(77));
        world.insert_resource(ConfirmedFrameCount(41));
        let order_before = world.resource::<RollbackOrdered>().len();

        let refused = start_sync_test_session_owned(
            &mut world,
            SyncTestSettings::for_players(1),
            SyncTestOwner::Caller,
        );

        // ⛔ THE DEFECT-DESCRIBING ASSERTIONS FIRST. Reading the `Err` first would
        // let a poison fail on the return value — the thing that ANNOUNCES the
        // property — instead of on the property.
        assert!(
            !session_is_active(&world),
            "a refused start installed a session anyway. That session's frame \
             zero carries every rollback order this App ever handed out, which is \
             the ID-PEER defect the rebase exists to remove"
        );
        assert_eq!(
            world.resource::<RollbackFrameCount>().0,
            77,
            "the refusal reset the frame counter. The check has to come BEFORE \
             every mutation, not beside the rebase"
        );
        assert_eq!(
            world.resource::<ConfirmedFrameCount>().0,
            41,
            "the refusal reset the confirmation counter"
        );
        assert_eq!(
            world.resource::<RollbackOrdered>().len(),
            order_before,
            "the refusal rebuilt the order table, which drops the hidden \
             carrier's entry and panics on the next checksum"
        );
        assert!(
            world.get_resource::<Time<GgrsTime>>().is_none(),
            "the refusal installed a GGRS clock for a session that does not exist"
        );
        match refused.expect_err("the start road must refuse") {
            StartSyncTestError::FrameZeroRefused(refusal) => {
                assert_eq!(refusal.hidden_candidates, 1);
                assert_eq!(refusal.carriers, 2, "the refusal names the WHOLE population");
            }
            other => panic!(
                "the refusal came back as a GGRS construction error, so this arm \
                 is reporting on the settings rather than on the world: {other}"
            ),
        }
    }

    /// ⛔ **AND THE TOKEN CANNOT BE HAD WHILE A CANDIDATE IS HIDDEN, WHICH IS
    /// WHAT MAKES THE INSTALL'S INFALLIBILITY MEAN ANYTHING.** Without this, the
    /// arm above could pass because `start_sync_test_session_owned` declines for
    /// its own reasons while `FrameZeroEligibility::check` waves the world
    /// through — and every other caller takes the token road, not that one.
    #[test]
    fn the_frame_zero_token_is_refused_while_a_candidate_is_hidden_and_granted_when_it_is_not() {
        use ambition_platformer2d_shared_tangle::construction::{
            hide_candidate_session_root, register_inactive_candidate_filter,
        };

        let mut world = world_with(0, &["alpha", "beta"]);
        // Granted first, so "refused" below is attributable to the candidate
        // rather than to anything else about the fixture.
        let granted = FrameZeroEligibility::check(&mut world)
            .expect("a world with no candidate in flight can declare frame zero");
        assert_eq!(granted.carriers(), 2);

        register_inactive_candidate_filter(&mut world);
        let hidden = world
            .query_filtered::<Entity, With<Rollback>>()
            .iter(&world)
            .next()
            .expect("the fixture built carriers");
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = bevy::prelude::Commands::new(&mut queue, &world);
        hide_candidate_session_root(&mut commands, hidden);
        queue.apply(&mut world);

        let refusal = FrameZeroEligibility::check(&mut world)
            .expect_err("a hidden candidate must refuse the token");
        assert_eq!(refusal.hidden_candidates, 1);
        assert_eq!(refusal.carriers, 2);
    }

    /// ⚠ An unnamed carrier does not stop the rebase, and the report says so —
    /// refusing would leave the whole population carrying another App's history
    /// because one body's spawn site forgot to mint an identity.
    #[test]
    fn an_unnamed_carrier_is_counted_and_still_ordered() {
        let mut world = world_with(1, &["alpha"]);
        world.spawn(Rollback);
        world.flush();
        let report = rebase_rollback_carrier_order(&mut world);
        assert_eq!(report.carriers, 2);
        assert_eq!(report.identified, 1);
        assert_eq!(world.resource::<RollbackOrdered>().len(), 2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema_entry(name: &str) -> crate::RollbackRegistrationDescriptor {
        crate::RollbackRegistrationDescriptor {
            name: name.to_owned(),
            owner: "test".to_owned(),
            kind: crate::RollbackEntryKind::Derived,
            type_name: format!("test::{name}"),
            detail: String::new(),
        }
    }

    /// ⭐⭐ THE PER-FRAME SCHEMA HASH MUST BE THE MEMOISED ONE.
    ///
    /// `schema_fingerprint` was memoised because rebuilding the ~40KB schema dump
    /// and blake3ing it cost 292us of EVERY frame. The memo then did nothing,
    /// because this system read it through `.cloned()` — and `RollbackRegistry`'s
    /// hand-written `Clone` starts the `OnceLock` empty on purpose. Every value
    /// assertion still passed; the trace still showed the cost.
    ///
    /// ⛔ SO THIS TEST ASKS THE WORLD'S OWN REGISTRY, NOT THE VALUE. A test that
    /// compares fingerprints across frames is exactly the test that agreed with
    /// the bug: the recomputed value is correct, it is just recomputed.
    #[test]
    fn the_session_contract_hashes_the_schema_once_per_registry_not_once_per_frame() {
        let mut world = World::new();
        let mut registry = RollbackRegistry::default();
        for name in ["alpha", "beta", "gamma"] {
            registry
                .try_register(schema_entry(name))
                .expect("fixture registration");
        }
        assert!(
            !registry.fingerprint_is_memoised(),
            "the fixture must start uncached, or this proves nothing"
        );
        world.insert_resource(registry);

        start_sync_test_session(&mut world, SyncTestSettings::for_players(1))
            .expect("session starts");

        // ⛔⛔ RE-ARM THE INSTRUMENT. Installing the session reads the fingerprint
        // too, so by here the memo is already populated and "is it cached?" can
        // no longer tell us who cached it. A `clone()` is precisely "same entries,
        // empty memo" — swap it in, and from this point ONLY the per-frame system
        // can populate it, and only by reading the world's registry directly.
        let uncached = world.resource::<RollbackRegistry>().clone();
        assert!(
            !uncached.fingerprint_is_memoised(),
            "a clone must start uncached, or this test observes nothing"
        );
        world.insert_resource(uncached);

        for _ in 0..8 {
            enforce_session_contract(&mut world);
        }

        assert!(
            world
                .resource::<RollbackRegistry>()
                .fingerprint_is_memoised(),
            "the world's registry never cached its schema fingerprint, so every \
             frame rebuilt and re-hashed the whole schema dump — the memo is \
             being read through a clone again"
        );
        assert!(
            world.contains_resource::<AmbitionGgrsSession>(),
            "the contract check must not have invalidated the session it was \
             asked to police"
        );
        assert_eq!(
            world
                .resource::<ActiveRollbackAuthority>()
                .status()
                .invalidation,
            None,
            "a stable registry must produce a stable schema across frames"
        );
    }

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

        let eligibility = FrameZeroEligibility::check(&mut world)
            .expect("no candidate world is in flight in this fixture");
        install_rebased_sync_test_session(
            &mut world,
            session,
            settings,
            SyncTestOwner::Caller,
            eligibility,
        );

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

    /// ⛔⛤ **A CALLER THAT INVALIDATES ITS OWN PREFLIGHT MUST NOT SILENTLY
    /// INSTALL, AND BEFORE 2026-09-17 IT DID.**
    ///
    /// The token made "install without having checked" unspellable, and this arm
    /// is the case that shape does not cover: the caller DID check, got a token,
    /// and then changed the world. The lifecycle road has exactly that structure
    /// — check, `execute_lifecycle_commit`, install — and `&mut World` does not
    /// forbid the middle step, only other systems. The 2026-09-17 review said it
    /// plainly: *"it does not mean the caller cannot invalidate the predicate
    /// between those two calls."*
    ///
    /// ⚠ WHAT THE OLD BEHAVIOUR WAS, WHICH IS WHY THIS IS AN `assert` AND NOT A
    /// `debug_assert`: in release the frame counters reset, the rebase declined
    /// to rebuild the order because it cannot describe a hidden carrier, and the
    /// session installed carrying every rollback order this App ever handed out.
    /// That is the ID-PEER defect the rebase exists to remove — 59 of 146 GGRS
    /// checksum parts differing between equivalent hosts — arriving back through
    /// the one road built to prevent it, in the build that ships.
    ///
    /// ⭐ AND IT ASSERTS WHERE THE PANIC HAPPENED, NOT JUST THAT IT HAPPENED.
    /// A panic raised beside the rebase would leave the frame counters already
    /// reset and the GGRS clock already installed, which is a half-committed
    /// session. Every reading below is taken AFTER the unwind.
    ///
    /// ⛔⛔ **THIS ARM MUST BE READ IN BOTH PROFILES, AND THE TWO SAY DIFFERENT
    /// THINGS.** Poisoned by deleting the unconditional census, 2026-09-17:
    ///
    ///   `--release`  the session INSTALLS, silently, with a hidden carrier
    ///               present — the `debug_assert` beside the rebase is gone —
    ///               and this arm reddens at its `expect_err`. That is the
    ///               defect, and it is the profile that ships.
    ///   debug       the `debug_assert` trips first, so the arm still reddens
    ///               but on the wrong line, and the message assertion is what
    ///               catches it.
    ///
    /// ⇒ A run in debug alone would have shown this arm red under the poison and
    /// green under the fix, and concluded that `debug_assert` was sufficient.
    #[test]
    fn an_install_whose_preflight_was_invalidated_panics_before_touching_frame_zero() {
        use ambition_platformer2d_shared_tangle::sim_id::SimId;
        use bevy_ggrs::{Rollback, RollbackOrdered};
        use ambition_platformer2d_shared_tangle::construction::{
            hide_candidate_session_root, register_inactive_candidate_filter,
        };

        let settings = SyncTestSettings {
            check_distance: 0,
            max_prediction_window: 8,
            ..SyncTestSettings::for_players(1)
        };
        let session =
            build_sync_test_session(settings).expect("a one-player SyncTest session is valid");

        let mut world = World::new();
        world.init_resource::<RollbackOrdered>();
        world.spawn((Rollback, SimId::placement("alpha")));
        world.spawn((Rollback, SimId::placement("beta")));
        world.flush();
        // Frame state that is recognisably NOT frame zero, so "nothing was
        // mutated" is a reading rather than an absence.
        world.insert_resource(RollbackFrameCount(77));
        world.insert_resource(ConfirmedFrameCount(41));

        // The caller's honest preflight, taken while the world was fit.
        let eligibility = FrameZeroEligibility::check(&mut world)
            .expect("the fixture has no candidate in flight yet");
        assert_eq!(eligibility.carriers(), 2);

        // ⇒ AND THEN THE CALLER CHANGES THE WORLD, which is the whole subject.
        register_inactive_candidate_filter(&mut world);
        let hidden = world
            .query_filtered::<Entity, With<Rollback>>()
            .iter(&world)
            .next()
            .expect("the fixture built carriers");
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = bevy::prelude::Commands::new(&mut queue, &world);
        hide_candidate_session_root(&mut commands, hidden);
        queue.apply(&mut world);

        let order_before = world.resource::<RollbackOrdered>().len();
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            install_rebased_sync_test_session(
                &mut world,
                session,
                settings,
                SyncTestOwner::Caller,
                eligibility,
            );
        }));
        let payload = panicked.expect_err(
            "the install accepted a token whose premise had been invalidated and \
             declared frame zero anyway",
        );
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .unwrap_or("");
        assert!(
            message.contains("hidden construction candidate"),
            "the panic must name the reason a reader can act on; got: {message}"
        );

        assert_eq!(
            world.resource::<RollbackFrameCount>().0,
            77,
            "the frame counter was reset before the invariant was checked, so the \
             refusal left a half-declared frame zero"
        );
        assert_eq!(
            world.resource::<ConfirmedFrameCount>().0,
            41,
            "the confirmation counter was reset before the invariant was checked"
        );
        assert_eq!(
            world.resource::<RollbackOrdered>().len(),
            order_before,
            "the order table was rebuilt before the invariant was checked, which \
             drops the hidden carrier's entry and panics on the next checksum"
        );
        assert!(
            world.get_resource::<Time<GgrsTime>>().is_none(),
            "a GGRS clock was installed for a session that was refused"
        );
        assert!(
            world.get_resource::<RollbackSessionOwnership>().is_none(),
            "the session was installed after all, which is the defect this arm exists for"
        );
    }

    /// ⭐ THE CONTROL FOR THE ARM ABOVE, AND IT IS NOT OPTIONAL. Without it the
    /// arm passes against an install that panics unconditionally — a
    /// `FrameZeroEligibility` would then be worthless and every session would
    /// refuse.
    #[test]
    fn an_install_whose_preflight_still_holds_declares_frame_zero() {
        use ambition_platformer2d_shared_tangle::sim_id::SimId;
        use bevy_ggrs::{Rollback, RollbackOrdered};
        let settings = SyncTestSettings {
            check_distance: 0,
            max_prediction_window: 8,
            ..SyncTestSettings::for_players(1)
        };
        let session =
            build_sync_test_session(settings).expect("a one-player SyncTest session is valid");

        let mut world = World::new();
        world.init_resource::<RollbackOrdered>();
        world.spawn((Rollback, SimId::placement("alpha")));
        world.spawn((Rollback, SimId::placement("beta")));
        world.flush();
        world.insert_resource(RollbackFrameCount(77));
        world.insert_resource(ConfirmedFrameCount(41));

        let eligibility = FrameZeroEligibility::check(&mut world)
            .expect("the fixture has no candidate in flight");
        // ⚠ The caller mutates the world here TOO — it just does not hide a
        // carrier. A fresh carrier is the ordinary case (a room commit spawns
        // bodies), so the invariant must not be "the world is untouched".
        world.spawn((Rollback, SimId::placement("gamma")));
        world.flush();

        install_rebased_sync_test_session(
            &mut world,
            session,
            settings,
            SyncTestOwner::Caller,
            eligibility,
        );

        assert_eq!(world.resource::<RollbackFrameCount>().0, 0);
        assert_eq!(world.resource::<ConfirmedFrameCount>().0, -1);
        assert_eq!(
            world.resource::<RollbackOrdered>().len(),
            3,
            "the rebase ordered the carrier the caller added after its preflight"
        );
        assert!(matches!(
            world.resource::<AmbitionGgrsSession>(),
            AmbitionGgrsSession::SyncTest(_)
        ));
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
        world.init_resource::<RollbackRegistry>();
        world.init_resource::<RollbackDiagnosticHistory>();
        // An invalidation is a fact ABOUT A TIMELINE, so the fixture installs
        // one. A bare world has nothing to invalidate.
        start_sync_test_session(&mut world, SyncTestSettings::for_players(1))
            .expect("session starts");
        world.insert_resource(ConfirmedFrameBoundary {
            current: 7,
            confirmed: 2,
            session: 5,
        });

        invalidate_session(&mut world, "contract changed".into());

        assert!(!world.contains_resource::<ConfirmedFrameBoundary>());
        assert_eq!(
            world
                .resource::<ActiveRollbackAuthority>()
                .status()
                .invalidation
                .as_deref(),
            Some("contract changed")
        );
        assert_eq!(
            world.resource::<RollbackDiagnosticHistory>().len(),
            1,
            "the divergence is also remembered where it authorizes nothing"
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

/// ⛔⛤ **THE EDGE `Q120`'s FIRST FIX ASSERTED IN PROSE AND NEVER DECLARED.**
///
/// The original repair wrote, in a doc comment, that the rebase happened *"in
/// the same frame the edit was observed"*. It did not. The editor adapter lived
/// in the SIM schedule — under this host, `GgrsSchedule`, advanced from
/// `PreUpdate` by `RunGgrsSystems` — and the watcher lived in `Update`. ⇒ The old
/// timeline advanced, and could resimulate confirmed history, against the edited
/// value before anything looked at it.
///
/// ⭐⭐ **SO THE ORDERING IS NOW A DECLARATION, AND THIS ASKS THE SCHEDULE FOR
/// IT.** A comment cannot be poisoned; an edge can.
#[cfg(test)]
mod mechanical_edit_ordering_tests {
    use super::*;
    use ambition_platformer2d_core::MechanicalEditSet;
    use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules};

    fn set_node<S: bevy::ecs::schedule::SystemSet>(graph: &ScheduleGraph, set: S) -> NodeId {
        NodeId::Set(
            graph
                .system_sets
                .get_key(bevy::ecs::schedule::SystemSet::intern(&set))
                .unwrap_or_else(|| {
                    panic!(
                        "`{set:?}` is not in the PreUpdate graph at all, so every \
                         ordering question about it below is vacuously false"
                    )
                }),
        )
    }

    /// Reachability over the DEPENDENCY graph between two nodes the declaration
    /// itself names. ⚠ Set→MEMBER is hierarchy, not dependency, so this is only
    /// valid for set-to-set questions written as set-to-set edges — which is
    /// exactly how the chain below is declared.
    fn reaches(graph: &ScheduleGraph, from: NodeId, to: NodeId) -> bool {
        use std::collections::HashSet;
        let dependency = graph.dependency().graph();
        let mut out: std::collections::HashMap<NodeId, Vec<NodeId>> = Default::default();
        for (u, v) in dependency.all_edges() {
            out.entry(u).or_default().push(v);
        }
        let mut seen: HashSet<NodeId> = HashSet::new();
        let mut stack = vec![from];
        while let Some(node) = stack.pop() {
            if node == to && node != from {
                return true;
            }
            if !seen.insert(node) {
                continue;
            }
            if let Some(next) = out.get(&node) {
                stack.extend(next.iter().copied());
            }
        }
        false
    }

    #[test]
    fn the_mechanical_edit_chain_completes_before_the_timeline_advances() {
        let mut app = App::new();
        app.init_schedule(ReadInputs)
            .init_resource::<ambition_characters::control::SlotControlLatches>()
            .init_resource::<PendingSeatInputs>()
            .init_resource::<LocalPlayers>();
        install_session_bridge(&mut app);
        // ⚠ The graph is only populated once the schedule is initialized; asking
        // an uninitialized one returns an empty graph, which reads exactly like
        // "no edges exist" — i.e. like the finding.
        app.world_mut()
            .resource_mut::<Schedules>()
            .get_mut(PreUpdate)
            .expect("the PreUpdate schedule exists")
            .initialize(&mut World::new())
            .ok();
        let schedules = app.world().resource::<Schedules>();
        let graph = schedules
            .get(PreUpdate)
            .expect("the PreUpdate schedule exists")
            .graph();

        let propose = set_node(graph, MechanicalEditSet::Propose);
        let admit = set_node(graph, MechanicalEditSet::Admit);
        let publish = set_node(graph, MechanicalEditSet::Publish);
        let advance = set_node(graph, RunGgrsSystems);

        // ⛔ THE CONTROL, AND IT IS THE LOAD-BEARING HALF. An instrument that
        // walks too generously answers "reaches" for everything, and every
        // assertion below would then pass while proving nothing.
        assert!(
            !reaches(graph, advance, propose),
            "the instrument reports a path from the advance BACK to the proposal \
             step, which the declaration does not contain — it is answering \
             'reaches' for unrelated nodes, so nothing below is evidence"
        );

        for (what, from, to) in [
            ("a proposal is decided before it is published", propose, admit),
            ("a decision precedes the publication it authorizes", admit, publish),
            (
                "the whole chain completes before the timeline advances",
                publish,
                advance,
            ),
        ] {
            assert!(
                reaches(graph, from, to),
                "{what}: NOT ordered. A developer edit to a value every \
                 simulation system reads would reach the authority after \
                 `RunGgrsSystems` had already advanced — and RESIMULATED \
                 CONFIRMED HISTORY — with it. That is the defect `Q120` \
                 measured and the ordering its first fix only claimed."
            );
        }
    }
}

/// ⛔⛤ **ONE ANSWER TO *"MAY A MECHANICAL MUTATION HAPPEN AROUND THIS ROLLBACK
/// TIMELINE"*, AND TWO FEATURES USED TO ANSWER IT SEPARATELY — REVIEW,
/// 2026-09-13.**
///
/// `Q118` (content publication) and `Q120` (developer mechanical edits) both ask
/// the same four questions — is there a timeline, is it healthy, who owns it, may
/// this host stop it — and each had implemented its own classification:
/// `ambition_content`'s `publication_boundary` and this crate's
/// `decide_mechanical_edit_admission`. **That duplication already produced one
/// defect**: `Q118`'s transaction-lifetime lease re-checked HEALTH and not the
/// OWNERSHIP condition that had authorized publication in the first place, so a
/// generation admitted against a session this host maintained stayed admitted
/// after a peer-owned one replaced it.
///
/// ⇒ And a second discrepancy was live when the review found it: the
/// mechanical-edit decision reasoned from *session active + ownership* and never
/// consulted HEALTH, while the publication decision refused an unhealthy
/// authority outright. ⚠ The behaviour was safe for a reason neither feature
/// states — `stop_session` STANDS THE AUTHORITY DOWN rather than removing it and
/// `ActiveRollbackAuthority::installed` refuses to launder a divergence into a
/// fresh timeline, so a rebase cannot heal one — but *"safe because something
/// else happens to hold"* is the shape this repository removes, not a policy.
///
/// ⭐⭐ **SO THE ROLLBACK SUBSYSTEM ANSWERS ONCE AND THE CALLERS DIFFER ONLY IN
/// TRANSACTION SEMANTICS.** `Q120` consumes this instantaneously (*may this
/// editor proposal publish before the next GGRS advance?*); `Q118` holds the same
/// answer as a transaction-lifetime LEASE (*has the condition that authorized
/// this pending generation remained valid?*). One authority for the fact, two
/// lifetimes for the use of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MechanicalMutationBoundary {
    /// No authority, or one whose timeline is not speculating. Every
    /// composition without a rollback host lives here, permanently.
    NoTimeline,
    /// Live, healthy, and THIS host started it and may stop it.
    LocallyRebasable,
    /// Live and healthy, but owned by peers (`External`) or by a caller that did
    /// not ask for a rebase. `RollbackSessionOwnership`: *"must never be
    /// replaced unilaterally by the local host."*
    ForeignTimeline,
    /// A divergence has been RECORDED. Mutating mechanics across it is wrong
    /// under every model both rows list, because a recovery that looked clean
    /// would have laundered the desync.
    Unhealthy(String),
}

/// Classify this world's rollback timeline for a caller that wants to change
/// something the simulation reads.
///
/// ⚠ **ABSENT AUTHORITY MEANS `NoTimeline`, which is the right answer and not a
/// hole:** a composition that installs no rollback has no history to
/// contradict. A stood-down timeline is the same answer for the same reason —
/// it is not speculating.
///
/// ⛔ THE AUTHORITY'S OWN SCOPE, never a stranger's. `confirmation_for` returns
/// `Unavailable` for a scope it does not govern, and reading a stranger's answer
/// would report every world as mutable.
pub fn mechanical_mutation_boundary(world: &World) -> MechanicalMutationBoundary {
    use ambition_platformer2d_runtime::rollback::RollbackConfirmationState;

    let Some(authority) = world.get_resource::<ActiveRollbackAuthority>() else {
        return MechanicalMutationBoundary::NoTimeline;
    };
    match authority.confirmation_for(authority.owner()) {
        RollbackConfirmationState::Unavailable => MechanicalMutationBoundary::NoTimeline,
        RollbackConfirmationState::Unhealthy => MechanicalMutationBoundary::Unhealthy(
            authority
                .status()
                .invalidation
                .clone()
                .unwrap_or_else(|| format!("{:?}", authority.status().mismatch_frames)),
        ),
        // ⛔ OWNERSHIP IS FOLDED IN HERE AND NOWHERE ELSE. Asking it separately
        // is what let one caller consult half the permission for a day.
        RollbackConfirmationState::Healthy => {
            if crate::local_session::locally_rebasable_timeline(world) {
                MechanicalMutationBoundary::LocallyRebasable
            } else {
                MechanicalMutationBoundary::ForeignTimeline
            }
        }
    }
}
