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

use crate::abilities::traversal::possession::PossessionState;
use ambition_boss_encounter::BossEncounterRegistry;
use ambition_characters::control::SlotInteractionState;
use ambition_encounter::switches::SwitchActivationQueue;
use ambition_encounter::{EncounterRegistry, EncounterView};
use ambition_persistence::quest::QuestRegistry;
use ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown;
use ambition_platformer2d_world::collision::MovingPlatformSet;

/// The process-global resources that mirror ONE live session's state.
///
/// Grouped so each system stays within Bevy's system-parameter budget, and so
/// the ownership set is stated in one place rather than rediscovered. Adding a
/// resource here is the declaration that it belongs to a gameplay session and
/// not to the process — see the module doc for which edge makes that safe.
#[derive(SystemParam)]
pub struct SessionScopedResources<'w> {
    /// The active room's advancing platform kinematics; a fresh activation
    /// rebuilds it from the new room (and it is snapshot-registered state).
    moving_platforms: ResMut<'w, MovingPlatformSet>,
    /// Possession pair (`possessed`/`home` entity handles + restore brain). The
    /// player is despawned on retirement, so these would dangle.
    possession: ResMut<'w, PossessionState>,
    /// The driven-body handle. It self-heals each tick from the `DrivingParticipant`
    /// query while a session is live, but the sim sleeps at the launcher, so
    /// without an explicit reset it would hold the retired session's dead body
    /// across the whole frontend visit.
    controlled_subject: ResMut<'w, ControlledSubject>,
    /// Encounter id → live encounter entity index. Re-armed from the empty save
    /// on the next activation once cleared (its `specs_loaded` flag flips false).
    encounter_registry: ResMut<'w, EncounterRegistry>,
    /// The encounter read model — cleared so no published view describes the dead
    /// session between retirement and the next activation's first rebuild.
    encounter_view: ResMut<'w, EncounterView>,
    /// Boss profiles; `specs_loaded` re-arms the populate pass on next activation.
    boss_registry: ResMut<'w, BossEncounterRegistry>,
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
    /// `adopt_the_occurrence_ledger_at_activation` runs AFTER this reset, so a
    /// LOAD re-seeds the cleared ledger from its own file on the same edge.
    occurrences: ResMut<'w, ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>,
    /// The checkpoint copies of the same three facts. They describe the same one
    /// world, so they carry the same defect and get the same answer — a
    /// checkpoint baseline from the previous session is a baseline for a world
    /// that no longer exists.
    occurrence_baseline:
        ResMut<'w, ambition_platformer2d_shared_tangle::lifecycle::OccurrenceBaseline>,
    custody_baseline: ResMut<'w, ambition_platformer2d_shared_tangle::lifecycle::CustodyBaseline>,
    minted_baseline: ResMut<'w, crate::items::pickup::minted_horizon::MintedItemBaseline>,
    /// ⛔⛔ THE TWO ROOM-ENTRY EDGE MEMORIES (S2 moved them out of `Local`s; S6
    /// makes them session-scoped). Each remembers "the room I last announced",
    /// so the next tick's `RoomEntered` push / cutscene trigger fires only on a
    /// CHANGE. Inherited across sessions, a new game that starts in the room
    /// the previous session ended in — quitting at the start and starting
    /// over is exactly that — would skip its first room's quest events and
    /// cutscene trigger, because the memory already said "you are there".
    quest_last_room: ResMut<'w, ambition_persistence::quest::LastQuestRoom>,
    cutscene_last_room: ResMut<'w, ambition_cutscene::LastCutsceneRoom>,
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
}

/// Re-establish the session mirrors for a scope that is about to be built.
///
/// ⭐ THE CORRECTNESS EDGE. Runs in [`SessionScopeSet::Activate`], before any
/// provider constructs the world these values describe.
pub fn reset_session_scoped_resources_on_activation(
    mut activated: MessageReader<SessionScopeActivated>,
    resources: SessionScopedResources,
) {
    if activated.read().count() == 0 {
        return;
    }
    reset(resources);
}

/// Release the session mirrors of a scope that has ended.
///
/// ⚠ HYGIENE, NOT CORRECTNESS. Its job is to stop dead entity handles and a
/// retired session's latches sitting in memory for the whole frontend visit.
/// [`reset_session_scoped_resources_on_activation`] is what makes the next
/// session safe, and it does not depend on this having run.
pub fn reset_session_scoped_resources_on_retire(
    mut retired: MessageReader<SessionScopeRetired>,
    resources: SessionScopedResources,
) {
    if retired.read().count() == 0 {
        return;
    }
    reset(resources);
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
        mut moving_platforms,
        mut possession,
        mut controlled_subject,
        mut encounter_registry,
        mut encounter_view,
        mut boss_registry,
        mut quest_registry,
        mut sim_state,
        mut slot_interactions,
        mut switch_activations,
        mut save_restored,
        mut occurrences,
        mut occurrence_baseline,
        mut custody_baseline,
        mut minted_baseline,
        mut quest_last_room,
        mut cutscene_last_room,
        mut projectile_seq,
    } = resources;
    *moving_platforms = MovingPlatformSet::default();
    *possession = PossessionState::default();
    *controlled_subject = ControlledSubject::default();
    *encounter_registry = EncounterRegistry::default();
    *encounter_view = EncounterView::default();
    *boss_registry = BossEncounterRegistry::default();
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
    *quest_last_room = ambition_persistence::quest::LastQuestRoom::default();
    *cutscene_last_room = ambition_cutscene::LastCutsceneRoom::default();
    *projectile_seq = ambition_projectiles::ProjectileSeqCounter::default();
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
