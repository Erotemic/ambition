//! Re-establish the process-global mirrors of live-session state at BOTH edges
//! of a gameplay session.
//!
//! Entity cleanup handles `SessionScopedEntity`; this module owns the resources
//! that retain entity handles or per-session latches. App-global authored
//! catalogs and registries remain intact.
//!
//! # Which edge is correctness
//!
//! ```text
//! SessionScopeActivated  ->  CORRECTNESS. The session about to read these
//!                            values writes them first, so nothing a previous
//!                            session left can reach it.
//! SessionScopeRetired    ->  HYGIENE. Frees dangling entity handles while the
//!                            title screen is up. Skipping it leaks memory and
//!                            stale diagnostics; it cannot change behaviour.
//! ```
//!
//! ⛔⛔ IT WAS ONLY THE SECOND ONE, AND "must happen" IS NOT AN INVARIANT. Nine
//! of the eleven resources below are live-session authority with no owner — a
//! dangling possessed-body handle, a `specs_loaded` latch that suppresses the
//! next session's repopulation, a room-transition cooldown that refuses doors, a
//! buffered interact nobody pressed. Every one of them reaches the next game if
//! retirement is delayed a frame, misordered, or skipped by an abnormal exit.
//! The rollback timeline's version of exactly this bug is what prompted the
//! audit; see `ambition_platformer2d_runtime::rollback::authority`.
//!
//! ⭐ Resetting at activation is the cheap form of ownership for a resource with
//! dozens of readers: it needs no accessor check at any of them, because the
//! value a session reads is one its own activation wrote.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{
    SessionScopeActivated, SessionScopeRetired, SessionScopeSet,
};
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;

use crate::control::possession::PossessionState;
use ambition_boss_encounter::BossEncounterRegistry;
use ambition_characters::control::SlotInteractionState;
use ambition_encounter::switches::SwitchActivationQueue;
use ambition_encounter::EncounterView;
use ambition_persistence::quest::QuestRegistry;
use ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown;

/// The process-global resources that mirror ONE live session's state.
///
/// Grouped so each system stays within Bevy's system-parameter budget, and so
/// the ownership set is stated in one place rather than rediscovered. Adding a
/// resource here is the declaration that it belongs to a gameplay session and
/// not to the process — see the module doc for which edge makes that safe.
#[derive(SystemParam)]
pub struct SessionScopedResources<'w> {
    /// Possession pair (`possessed`/`home` entity handles + restore brain). The
    /// player is despawned on retirement, so these would dangle.
    possession: ResMut<'w, PossessionState>,
    /// The driven-body handle. It self-heals each tick from the `DrivingParticipant`
    /// query while a session is live, but the sim sleeps at the launcher, so
    /// without an explicit reset it would hold the retired session's dead body
    /// across the whole frontend visit.
    controlled_subject: ResMut<'w, ControlledSubject>,
    /// The encounter read model — cleared so no published view describes the dead
    /// session between retirement and the next activation's first rebuild.
    encounter_view: ResMut<'w, EncounterView>,
    /// Boss profiles; `specs_loaded` re-arms the populate pass on next activation.
    /// `Option`: named bosses are removable (`Capability::BossEncounters`).
    boss_registry: Option<ResMut<'w, BossEncounterRegistry>>,
    /// The boss defeats since the last checkpoint. The next session's file is
    /// its baseline, so a replay there must retract none of this session's.
    /// `Option` because a composition without the boss capability has none.
    boss_defeats_since_checkpoint:
        Option<ResMut<'w, ambition_boss_encounter::BossDefeatsSinceCheckpoint>>,
    /// The one-time pickups consumed since the last checkpoint, with their
    /// owners. The next session's file is its baseline.
    consumed_since_checkpoint:
        Option<ResMut<'w, crate::features::ecs::pickups::ConsumedSinceCheckpoint>>,
    /// The grants of the mints collected since the last checkpoint. The next
    /// session's file is its baseline, so a retraction there must take back
    /// none of this session's.
    reward_grants: Option<ResMut<'w, crate::items::pickup::RewardGrantsSinceCheckpoint>>,
    /// The bag spends since the last checkpoint, for the same reason.
    bag_spends: Option<ResMut<'w, ambition_held_items::BagSpendsSinceCheckpoint>>,
    /// Quest progress; the next activation reloads it from the session save.
    quest_registry: ResMut<'w, QuestRegistry>,
    /// Transient per-room bookkeeping (room-transition cooldown, etc.).
    sim_state: ResMut<'w, RoomTransitionCooldown>,
    /// Slot-level buffered gestures belong to the retired control session.
    slot_interactions: ResMut<'w, SlotInteractionState>,
    /// Switch activations intentionally cross one simulation-frame boundary.
    /// Retirement between production and consumption must not deliver a
    /// session-A activation into session B.
    switch_activations: ResMut<'w, SwitchActivationQueue>,
    /// Whether the loaded save has been applied to the current world.
    /// Retirement resets the latch so the next session restores into its fresh world.
    save_restored: ResMut<'w, crate::session::durable_horizon::SaveRestored>,
    /// ⛔⛔ WHERE THE OCCURRENCES **THIS WORLD** HAS MINTED ACTUALLY ARE. The
    /// ledger is a statement about ONE live world, and it was app-level: session
    /// B's first room construction read session A's rows. A row saying an object
    /// is lying in room X SUPPRESSES that object when X is built, so an
    /// inherited ledger deletes things from the next session's world.
    ///
    /// ⭐ The activation edge is what makes this safe, and the order matters:
    /// `CandidateDurableHorizon::install` — the incoming candidate's own file,
    /// read into a value before it was validated — runs AFTER this reset, so a
    /// LOAD re-seeds the cleared ledger on the same edge. MEASURED uniformly
    /// over 639 shipped activations: reset, then install, then the world is
    /// promoted.
    occurrences: ResMut<'w, ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>,
    /// The checkpoint copies of the same three facts. They describe the same one
    /// world, so they carry the same defect and get the same answer — a
    /// checkpoint baseline from the previous session is a baseline for a world
    /// that no longer exists.
    ///
    /// ⭐ THE FOURTH CHECKPOINT BASELINE, `OwnedItemsBaseline`, IS A MEMBER SINCE
    /// 2026-10-04 ([`owned_items_baseline`](Self::owned_items_baseline)). It
    /// was left out on purpose on 2026-09-18, and the argument was real but its
    /// premise was wrong, so the argument stays here with the measurement.
    ///
    /// The argument: these three describe WORLD PLACEMENT, which a new session
    /// invalidates. Stored quantities are the player's and travel with the bag
    /// (`OwnedItems` is not in this file), so "resetting the baseline without
    /// resetting the bag would make a death in session B restore to an empty
    /// entitlement while the bag still held items". And the peer divergence it
    /// could carry "is the two peers' SAVE FILES differing, which resetting at
    /// the session edge does not cure".
    ///
    /// The measurement (shell host, rollback, two hosts with EQUAL saves): at
    /// tick 0 of a session that followed another one the baseline held the old
    /// session's bag, and on a fresh host it held zeros. The two agreed from
    /// tick 1, when the restore writes it. So the row differed with equal
    /// saves, because a fresh process has captured no baseline. And the hazard
    /// (a zero baseline, a full bag) is the state each first session has at
    /// tick 0. The reset adds no state that a first session does not have.
    occurrence_baseline:
        ResMut<'w, ambition_platformer2d_shared_tangle::lifecycle::OccurrenceBaseline>,
    custody_baseline: ResMut<'w, ambition_platformer2d_shared_tangle::lifecycle::CustodyBaseline>,
    minted_baseline: ResMut<'w, crate::items::pickup::minted_horizon::MintedItemBaseline>,
    // ⭐ THE TWO ROOM-ENTRY EDGE MEMORIES (`LastQuestRoom`, `LastCutsceneRoom`) ARE
    // NOT MEMBERS HERE (C03, 2026-10-07). Each remembers "the room I last
    // announced", so the next tick's `RoomEntered` push / cutscene trigger fires
    // only on a CHANGE; inherited across sessions, a new game that starts in the
    // room the previous session ended in would skip its first room's quest events
    // and cutscene trigger. They are components of the session root now, so a new
    // session's root is born with no memory and nothing resets them.
    /// ⭐ THE DETERMINISTIC PROJECTILE ID SOURCE, and it is CANONICAL rollback
    /// state (`rollback_resource_canonical::<ProjectileSeqCounter>`), so its
    /// value is inside the state checksum. A process-global monotonic counter
    /// that is never reset means session B's first projectile id depends on how
    /// many session A fired — so two hosts with different local histories can
    /// disagree at frame 0 in a CHECKSUMMED value, while every entity in the
    /// world matches. Safe to scope: projectile ids are transient and reach no
    /// save family, unlike a `SimId`, which is why the per-spawner `SimIdCounter`
    /// (a COMPONENT, not a resource) is deliberately not here.
    projectile_seq: ResMut<'w, ambition_projectiles::ProjectileSeqCounter>,
    /// ⭐ THE ONE-SLOT, EARLIEST-STICKY LIFECYCLE INTENT. `take()` clears it when
    /// the host commits and `retract_transition_for_subject` clears a matching
    /// transition, but a session that RETIRES with an intent still pending
    /// clears nothing — and the slot is rollback-registered, so its stale
    /// `frame` is a frame index from the DEAD session. Because `record` keeps
    /// the EARLIEST intent, that residue does not merely fire late: it REFUSES
    /// the next session's first lifecycle intent.
    pending_lifecycle: ResMut<'w, crate::session::lifecycle_commit::PendingLifecycleCommit>,
    /// ⛔⛤ **THE ROOM'S AMBIENT GRAVITY, ADDED 2026-09-13 AFTER A REVIEW FOUND IT
    /// CROSSING SESSIONS.** `BaseGravity` is canonical ROLLBACK state and a
    /// MECHANIC — gravity-flip switches write it — and its only reset roads were
    /// a room replay and a room TRANSITION's commit. Neither is a session edge,
    /// so *"flip gravity, quit, start a new game"* began the next session upside
    /// down until some later transition happened to correct it.
    ///
    /// ⚠ Its NEW-GAME edge is not here: a New Game is admitted as a replay, and
    /// the gravity domain answers that itself, in
    /// `gravity::lifecycle::reset_gravity_on_room_reset`.
    base_gravity: ResMut<'w, ambition_platformer2d_shared_tangle::gravity::BaseGravity>,
    // ⛔⛤ **THE CHECKPOINT COORDINATOR IS NOT A MEMBER HERE, AND ITS OWNER IS THE
    // SESSION ROOT (C03, 2026-10-07).** `OutstandingCheckpointRequest` was a member
    // until 2026-09-13, when a review found the rest of the coordinator still
    // process-global and gave all six their own bundle and their own reset. The six
    // are components of the root now (`SessionCheckpointState`), so neither list
    // resets them: a new root carries new components.
    /// ⛔⛤ **CUTSCENE PLAYBACK, WHICH USED TO OUTLIVE THE SESSION THAT STARTED
    /// IT.** `LastCutsceneRoom` above is the MEMORY of where one played; this is
    /// the runtime that is playing. While `is_playing()` holds,
    /// `declare_in_session_input_contexts` declares the high-priority cutscene
    /// context, so a session B beginning after A quit mid-cutscene could open
    /// with its input captured. ⛔ And it is durable as well as mechanical:
    /// `tick_active_cutscene` calls `end_cutscene`, which writes the running
    /// script's `seen_flag` into the CURRENT `AmbitionGameSave` — so A's stale
    /// cutscene finishing after B installed its save writes A's narrative flag
    /// into B's file.
    // The cutscene and conversation members are `Option`: they belong to
    // capabilities a composition may leave out (`Capability::Cutscenes`,
    // `Capability::Dialogue`), and a session that never had them has nothing to reset.
    active_cutscene: Option<ResMut<'w, ambition_cutscene::ActiveCutscene>>,
    /// The other half: a trigger raised just before retirement and consumed just
    /// after the next session begins plays A's cutscene in B.
    cutscene_triggers: Option<ResMut<'w, ambition_cutscene::CutsceneTriggerQueue>>,
    /// ⛔⛤ **THE CONVERSATION THE SIMULATION IS HAVING — same class as the
    /// cutscene above, and the 2026-09-13 review rated it a RISK rather than a
    /// confirmed bug for one reason: `break_dialogue_on_hit_or_separation` closes
    /// a conversation whose participants no longer exist, and a retired session's
    /// entities are despawned, so the stale state probably heals.** *"Probably"*
    /// and *"eventually"* are ordering claims: there is an interval after B
    /// begins in which the rollback snapshot still carries A's conversation and
    /// input declaration can still see its owner.
    ///
    /// ⭐ **SO IT IS MADE IMPOSSIBLE RATHER THAN TIMED.** One line of membership
    /// here costs nothing and removes the interval; establishing exactly how long
    /// the interval is would cost a poison test and leave the interval there.
    active_conversation: Option<ResMut<'w, ambition_conversation::ActiveConversation>>,
    /// The cutscene's partial skip hold. Simulation state since the skip
    /// reads the seat's `ControlFrame::cancel_held` inside the timeline; reset
    /// here so a half-held skip from the retired session cannot finish on the
    /// next session's opening cutscene. (A completed dismiss or skip no longer
    /// waits in a resource between frames: it is part of the tick's input.)
    cutscene_skip_hold: Option<ResMut<'w, ambition_cutscene::CutsceneSkipHold>>,
    /// ⛔⛤ **THE MATCH-IDENTITY MIRRORS, AND THEY ARE HERE FOR A PEER
    /// CHECKSUM RATHER THAN FOR HYGIENE.** Each of the three below is stamped
    /// with a whole [`MatchInstance`] and decides whether it belongs to the live
    /// match by comparing it — but the PEER projection of that stamp is the
    /// session-relative match ordinal alone, which restarts at zero every
    /// session. So `session A / match 0` and `session B / match 0` project
    /// identically, and a stale session-A stamp sitting beside session B's first
    /// match makes two peers answer `settled(active)` differently while their
    /// checksums agree. A false-negative checksum is the one failure ID-PEER
    /// exists to prevent.
    ///
    /// ⇒ The ordinal does not need a session term; the STALE STAMP needs to be
    /// impossible. Resetting at [`SessionScopeSet::Activate`] — the correctness
    /// edge, before any provider builds the world — is what makes it impossible,
    /// and it is the road this crate already had. Named by the GPT architecture
    /// review of 2026-09-16.
    ///
    /// ⛔⛤ **AND FOR A DAY THAT RESET THE MIRRORS AND LEFT THE THING THEY
    /// MIRROR.** [`ambition_match::ActiveMatch`] is the match authority these
    /// three compare themselves against, it is rollback-registered peer state in
    /// its own right, and its peer projection is `(seat count, ordinal)` — both
    /// of which a PREVIOUS session wrote. Two hosts entering the same new session
    /// after different histories (`match 0 / 2 seats` and `match 3 / 4 seats`)
    /// therefore began that session with DIFFERENT checksummed state, which is
    /// the exact failure this group exists to prevent. Worse than checksum
    /// hygiene: `count_the_live_match_ticks` treats any `ActiveMatch` as a live
    /// match, `spawn_match_items` reads it for match identity and random context,
    /// and settlement reads it — so the stale receipt is mechanically observable
    /// before the new session's first match activates.
    ///
    /// ⚠ The shell-experience route does register it for removal
    /// (`releasing_witnessed` in Versus/Smash), and that is a DIFFERENT boundary:
    /// a gameplay `SessionScopeId` can change without leaving the shell route.
    /// Route cleanup is still useful; it does not stand in for session cleanup.
    ///
    /// ⇒ It is removed rather than defaulted, in the system body, because it has
    /// no meaningful default — see the note there. Named by the GPT architecture
    /// review of 2026-09-16.
    settled: ResMut<'w, ambition_match::StocksMatchSettled>,
    sudden_death: ResMut<'w, ambition_match::SuddenDeathEntered>,
    live_match_ticks: ResMut<'w, crate::character_runtime::live_match_clock::LiveMatchTicks>,
    /// ⭐ AND THE MINT ITSELF, WHICH USED TO RESET LAZILY INSIDE `take`. A mint
    /// that notices the session changed at the moment it is next ASKED still
    /// holds the previous session's count until then, so two peers with different
    /// prior match counts disagreed for exactly the window between joining a
    /// session and activating its first match. The eager edge closes it: a new
    /// session's mint is new, because a new session's state is new.
    match_ordinal: ResMut<'w, ambition_match::seating::SessionMatchOrdinal>,
    /// The canonical timeline (`Q128`). A session starts at tick `0` on every
    /// peer, whatever the App ran before it. `Option` because a composition
    /// without the sim clock has none.
    sim_tick: Option<ResMut<'w, ambition_time::SimTick>>,
    /// The impact freeze holds an absolute expiry on [`ambition_time::SimTick`].
    /// Kept across the reset of the tick, a freeze from the previous session
    /// would hold the new one until that session's tick came round again.
    impact_hitstop: Option<ResMut<'w, ambition_combat::impact_hitstop::ImpactHitstop>>,
    /// The pace the simulation clock is asked for, and the pace it runs at
    /// ([`clock_state`](Self::clock_state)). The two are peer-compared, and a
    /// session that ends in a hitstop leaves them below neutral. Measured
    /// 2026-10-04 on the shipped host: the session that replaced it began with
    /// a pace of 0.42 and ran its first two ticks at 0.65 and 0.88, on that
    /// host only. A session starts at the neutral pace on every peer.
    /// `Option` as for `sim_tick`.
    requested_clock_scale: Option<ResMut<'w, ambition_time::time_control::RequestedClockScale>>,
    clock_state: Option<ResMut<'w, ambition_time::ClockState>>,
    /// The step of the last tick. It is peer-compared, and the first tick of a
    /// session writes it, so before that tick it was the last step of the
    /// session that ended (measured 2026-10-04: 0.0167, or 0.0097 after a
    /// hitstop, against 0.0 on a fresh host). `Option` as for `sim_tick`.
    world_time: Option<ResMut<'w, ambition_time::WorldTime>>,
    /// ⛔ THE PHASE OF EACH GATE PORTAL, WHICH DECIDES IF A BODY CAN GO THROUGH.
    /// The tick integrates it from the switch in the save, and nothing put it
    /// back when a session ended. Measured 2026-10-04 on the shell host, two
    /// hosts with equal saves and the switch on: the portal of a session that
    /// followed another one was `On` from tick 0, and the portal of a fresh
    /// host was `Opening` until tick 40. For 39 ticks one peer could take the
    /// gate and the other could not. Each session now starts with no phase, so
    /// its portals open from `Off` on every peer. `Option` because a
    /// composition without the room domain has none.
    gate_portal_phases: Option<ResMut<'w, ambition_platformer2d_world::rooms::GatePortalPhases>>,
    /// The bag at the last checkpoint. See the note above `occurrence_baseline`
    /// for why it was not a member until 2026-10-04. `Option` because a
    /// composition without the pickup domain has none.
    owned_items_baseline:
        Option<ResMut<'w, crate::items::pickup::minted_horizon::OwnedItemsBaseline>>,
}

/// Re-establish the session mirrors for a scope that is about to be built.
///
/// ⭐ THE CORRECTNESS EDGE. Runs in [`SessionScopeSet::Activate`], before any
/// provider constructs the world these values describe.
///
/// ⛔⛤ **TWENTY-THREE OF THESE RESOURCES ARE ROLLBACK-REGISTERED AND THIS IS AN
/// ORDINARY `Update` SYSTEM, WHICH THE ROLLBACK-MUTATOR GUARD ASKS ABOUT.**
/// (MEASURED 2026-09-16 across every `.rs` in `crates/` and `game/`, over all ten
/// `rollback_resource_*` methods the registrar declares. This line read SIXTEEN,
/// then TWENTY-TWO; ⚠ the count is load-bearing for the argument below, so it is
/// stated with the method that produced it. Five more were added 2026-10-04:
/// `RequestedClockScale`, `ClockState` and `WorldTime`
/// (`rollback_resource_canonical`), and `GatePortalPhases` and
/// `OwnedItemsBaseline` (`rollback_resource_clone_checksum`);
/// the total was not measured again. The 22 -> 23 step is re-derived
/// rather than decremented by hand: `AuthoredOccurrences` moved from
/// `declare_rollback_derived_resource` to `rollback_resource_clone_checksum` in
/// schema v195, which `rollback_schema_baseline.txt` records as exactly one row
/// added and one removed. Two remain derived — `ControlledSubject` and
/// `EncounterView` — which is a recorded decision that they are recomputed, and
/// four carry no rollback decision at all; see `docs/planning/queue.md`'s
/// CUTSCENE-ROLLBACK-DECISION.) Its
/// question is whether a write is replayed with the value it mutates, and two
/// things can answer it: the write is inside the rewind window, or the write is
/// at a point no rewind crosses. This is the second, and the ordering that
/// establishes it spans three crates rather than living here:
///
/// - `ambition_game_shell`'s session plugin chains
///   `(GameplaySessionSet::Bridge, SessionScopeSet::Activate,
///   GameplaySessionSet::Providers)` in `Update`;
/// - `adopt_candidate_platformer_session`, which makes a prepared root the LIVE
///   one, is in `GameplaySessionSet::Providers`;
/// - `maintain_local_session` starts GGRS only when
///   `session_world_entity(world).is_some()`.
///
/// ⇒ A scope's rollback timeline cannot exist until that scope's world root is
/// live, and the root is not live until after this runs. These writes land
/// before there is a frame zero to rewind to.
///
/// ⚠ **IT RESTS ON THE A10 CANDIDATE STAYING HIDDEN.** A10.5 prepares the
/// session world while the route is still PENDING, so a root for the incoming
/// scope exists before this system runs. It carries `InactiveCandidate`, a Bevy
/// disabling component, so the default query behind `session_world_entity`
/// cannot see it. ⭐ That is HELD, not assumed —
/// `a_hidden_candidate_session_is_invisible_to_the_live_world_and_visible_to_its_transaction`
/// (`shared_tangle/src/lifecycle/session/tests.rs`) pins all three halves,
/// including that ownership does not inherit the disabling. The session root's
/// canonical `SimId` rests on the same arm.
///
/// ⛔ **NOT A REBASE ARGUMENT.** `LifecycleIntent` has two variants,
/// `Transition` and `ReconstituteRoom`, and a session activation records
/// neither — so the confirmed-commit GGRS rebase does not fire here and an
/// argument built on one would be false.
pub fn reset_session_scoped_resources_on_activation(
    mut activated: MessageReader<SessionScopeActivated>,
    resources: SessionScopedResources,
    mut commands: bevy::prelude::Commands,
) {
    if activated.read().count() == 0 {
        return;
    }
    reset(resources);
    // ⛔⛤ **REMOVED, NOT DEFAULTED — THERE IS NO SUCH THING AS A DEFAULT LIVE
    // MATCH.** Every other member of this group answers "nothing has happened
    // yet" with a default value; `ActiveMatch` is a RECEIPT, and the honest state
    // between sessions is that no match has been activated. `Default` would have
    // to invent a seat count, and a zero-seat receipt is still a receipt — it
    // reads as a live match to `count_the_live_match_ticks` and to `MatchScoped`.
    //
    // ⭐ ABSENCE IS ALREADY A STATE THE ROLLBACK LAYER CARRIES: the registration
    // is `rollback_resource_optional_canonical_checksum` and
    // `ResourceSnapshotPlugin::load` maps `(Some(_), None)` to `remove_resource`,
    // so a rewind across activation restores the same absence this writes.
    //
    // ⚠ The removal lands at the sync point the `Activate -> Providers` chain
    // inserts, not at this statement — a set edge orders when a system RUNS.
    // Nothing between here and that flush reads `ActiveMatch`, and
    // `activate_the_prepared_match` (which would replace it anyway) runs after.
    commands.remove_resource::<ambition_match::ActiveMatch>();
}

/// Release the session mirrors of a scope that has ended.
///
/// ⚠ HYGIENE, NOT CORRECTNESS. Its job is to stop dead entity handles and a
/// retired session's latches sitting in memory for the whole frontend visit.
/// [`reset_session_scoped_resources_on_activation`] is what makes the next
/// session safe, and it does not depend on this having run.
///
/// ⚠ **SO ITS ANSWER TO THE ROLLBACK-MUTATOR GUARD IS WEAKER THAN ITS SIBLING'S,
/// AND DELIBERATELY STATED AS SUCH.** The sibling's writes precede the timeline;
/// these land on one that is being DISCARDED. The set chain `RetireAuthority ->
/// Cleanup` looks like it stops the session first and does not:
/// `retire_rollback_authority_with_its_scope` does its work through
/// `commands.queue`, so the stand-down applies at a later flush and the live
/// `AmbitionGgrsSession` may still be installed while this runs. A set edge
/// orders when a system RUNS, not when its effect LANDS. Nothing reads the
/// result, because the next session's correctness comes from the activation edge
/// above.
pub fn reset_session_scoped_resources_on_retire(
    mut retired: MessageReader<SessionScopeRetired>,
    active: bevy::prelude::Res<
        ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope,
    >,
    resources: SessionScopedResources,
    mut commands: bevy::prelude::Commands,
) {
    // ⛔⛤ **WHICH SCOPE RETIRED — THIS USED TO WIPE ON ANY RETIREMENT AT ALL, AND
    // A 2026-09-13 review found what that costs.** The shared lifecycle already
    // states the rule beside `ActiveSessionScope::clear_if_current`: *"Retiring A
    // after B activated must not clear B's spawn context."* Entity cleanup obeys
    // it (it checks each entity's owner) and the rollback session cleanup obeys
    // it. This system did not ask at all.
    //
    // ⇒ A delayed `SessionScopeRetired(A)` delivered after B became current wiped
    // **B's** mechanics, occurrence state, baselines, encounters, possession,
    // projectile counter and lifecycle intent — twenty authorities belonging to a
    // session that was still being played, while `ActiveSessionScope` correctly
    // went on naming B.
    //
    // ⭐ **AND THE TEST IS "IS THE RETIRING SCOPE STILL THE CURRENT ONE", NOT
    // "ARE THEY EQUAL TO SOMETHING".** A quit-to-title retires A and leaves NO
    // scope current, and that case must still free A's handles — it is the whole
    // reason this hygiene system exists. Only a retirement arriving while a
    // DIFFERENT scope is live is refused.
    let retiring: Vec<_> = retired.read().map(|event| event.0).collect();
    if retiring.is_empty() {
        return;
    }
    let live = active.current();
    if retiring
        .iter()
        .all(|scope| live.is_some_and(|current| current != *scope))
    {
        // Every retirement in this batch belongs to a scope that is not the live
        // one. Nothing here is ours to clear.
        return;
    }
    reset(resources);
    // ⛔⛤ **REMOVED, NOT DEFAULTED, AND THE DIFFERENCE IS THE WHOLE CONTRACT.**
    // `SessionMechanics` is the generation's OWN registries, and
    // `GenerationMechanics` treats its PRESENCE as "a generation was activated,
    // so its values outrank the App's". Sending it through `reset()` above would
    // leave a DEFAULT one installed — an empty cast that still wins — and a
    // composition with no activated generation would build its rooms out of
    // nothing instead of out of the App registries it has always used. Absent is
    // the honest state, so it is made absent.
    //
    // ⚠ This is hygiene like the rest of this system, not correctness: the next
    // activation overwrites the resource before any road reads it.
    commands.remove_resource::<crate::session::mechanics::SessionMechanics>();
    // ⛔ AND THE CAST FROZEN WITH THEM, and for this one absence is NOT only
    // hygiene: `SessionCast` reads its presence as "a session runs", so a
    // retired session's cast left installed would be what a menu or a select
    // screen shows instead of the published cast.
    commands.remove_resource::<ambition_characters::prepared::ActiveSessionCast>();
}

fn reset(resources: SessionScopedResources) {
    // ⛔⛔ AN EXHAUSTIVE DESTRUCTURE, AND DELIBERATELY NO `..`. This was
    // seventeen `*resources.field = Default::default()` lines, which is a
    // hand-kept list wearing the shape of code: adding a resource to the
    // `SystemParam` above and forgetting it here COMPILED, and the omission was
    // silent in exactly the way this whole set exists to prevent — session B
    // inheriting session A's state. Binding every field by name makes the next
    // addition a compile error at the only moment its author is still looking.
    //
    // ⚠ It is also what makes `retirement_clears_every_session_scoped_mirror`'s
    // name true. That test asserts a SUBSET by hand; "every" is guaranteed here,
    // by the compiler, and not there.
    let SessionScopedResources {
        mut possession,
        mut controlled_subject,
        mut encounter_view,
        mut boss_registry,
        boss_defeats_since_checkpoint,
        consumed_since_checkpoint,
        reward_grants,
        bag_spends,
        mut quest_registry,
        mut sim_state,
        mut slot_interactions,
        mut switch_activations,
        mut save_restored,
        mut occurrences,
        mut occurrence_baseline,
        mut custody_baseline,
        mut minted_baseline,
        mut projectile_seq,
        mut pending_lifecycle,
        mut base_gravity,
        mut active_cutscene,
        mut cutscene_triggers,
        mut active_conversation,
        mut cutscene_skip_hold,
        mut settled,
        mut sudden_death,
        mut live_match_ticks,
        mut match_ordinal,
        sim_tick,
        impact_hitstop,
        requested_clock_scale,
        clock_state,
        world_time,
        gate_portal_phases,
        owned_items_baseline,
    } = resources;
    *possession = PossessionState::default();
    *controlled_subject = ControlledSubject::default();
    *encounter_view = EncounterView::default();
    if let Some(boss_registry) = boss_registry.as_deref_mut() {
        *boss_registry = BossEncounterRegistry::default();
    }
    if let Some(mut since) = boss_defeats_since_checkpoint {
        since.forget_all();
    }
    if let Some(mut since) = consumed_since_checkpoint {
        since.forget_all();
    }
    if let Some(mut grants) = reward_grants {
        grants.forget_all();
    }
    if let Some(mut spends) = bag_spends {
        spends.forget_all();
    }
    *quest_registry = QuestRegistry::default();
    *sim_state = RoomTransitionCooldown::default();
    *slot_interactions = SlotInteractionState::default();
    *switch_activations = SwitchActivationQueue::default();
    *save_restored = crate::session::durable_horizon::SaveRestored::default();
    *occurrences = ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences::default();
    *occurrence_baseline =
        ambition_platformer2d_shared_tangle::lifecycle::OccurrenceBaseline::default();
    *custody_baseline = ambition_platformer2d_shared_tangle::lifecycle::CustodyBaseline::default();
    *minted_baseline = crate::items::pickup::minted_horizon::MintedItemBaseline::default();
    *projectile_seq = ambition_projectiles::ProjectileSeqCounter::default();
    *pending_lifecycle = crate::session::lifecycle_commit::PendingLifecycleCommit::default();
    *base_gravity = ambition_platformer2d_shared_tangle::gravity::BaseGravity::default();
    if let Some(active_cutscene) = active_cutscene.as_deref_mut() {
        *active_cutscene = ambition_cutscene::ActiveCutscene::default();
    }
    if let Some(cutscene_triggers) = cutscene_triggers.as_deref_mut() {
        *cutscene_triggers = ambition_cutscene::CutsceneTriggerQueue::default();
    }
    if let Some(active_conversation) = active_conversation.as_deref_mut() {
        *active_conversation = ambition_conversation::ActiveConversation::default();
    }
    if let Some(cutscene_skip_hold) = cutscene_skip_hold.as_deref_mut() {
        *cutscene_skip_hold = ambition_cutscene::CutsceneSkipHold::default();
    }
    *settled = ambition_match::StocksMatchSettled::default();
    *sudden_death = ambition_match::SuddenDeathEntered::default();
    *live_match_ticks =
        crate::character_runtime::live_match_clock::LiveMatchTicks::default();
    *match_ordinal = ambition_match::seating::SessionMatchOrdinal::default();
    if let Some(mut tick) = sim_tick {
        *tick = ambition_time::SimTick::default();
    }
    if let Some(mut hitstop) = impact_hitstop {
        *hitstop = ambition_combat::impact_hitstop::ImpactHitstop::default();
    }
    if let Some(mut requested) = requested_clock_scale {
        *requested = ambition_time::time_control::RequestedClockScale::default();
    }
    if let Some(mut clock) = clock_state {
        *clock = ambition_time::ClockState::default();
    }
    if let Some(mut time) = world_time {
        *time = ambition_time::WorldTime::default();
    }
    if let Some(mut phases) = gate_portal_phases {
        *phases = ambition_platformer2d_world::rooms::GatePortalPhases::default();
    }
    if let Some(mut baseline) = owned_items_baseline {
        *baseline = crate::items::pickup::minted_horizon::OwnedItemsBaseline::default();
    }
}

/// Installs session-resource re-establishment at both edges of a session.
pub struct SessionTeardownPlugin;

impl Plugin for SessionTeardownPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                reset_session_scoped_resources_on_activation.in_set(SessionScopeSet::Activate),
                reset_session_scoped_resources_on_retire.in_set(SessionScopeSet::Cleanup),
            ),
        );
    }
}

#[cfg(test)]
mod tests;
