//! Session teardown: reset the process-global resources that hold live-session
//! state when the active session scope retires.
//!
//! The generic [`despawn_retired_session_entities`] sweep already despawns every
//! `SessionScopedEntity` for the retiring scope, so all of a session's ECS
//! entities and relationships die with it. What the sweep does NOT touch is the
//! handful of process-global `Resource`s that MIRROR that live state — an index
//! from an id to a now-dead entity, the possession pair, the room's advancing
//! moving platforms, transient room bookkeeping. Left alone, they retain dangling
//! `Entity` handles and stale gameplay state across a teardown, and the next
//! activation's populate/setup systems (gated on `specs_loaded`-style flags) will
//! not re-arm because they believe the state is already loaded.
//!
//! This is the resource half of the "activate A, tear it down, activate B, and
//! prove nothing refers to the old scope" gate. It deliberately resets ONLY
//! session-scoped mutable mirrors that a fresh activation rebuilds — never the
//! App-global authored catalogs, provider registrations, or lowering registries,
//! which are process-global authority by design.
//!
//! Symmetry note: the same registries are reset by
//! [`super::reset::process_sandbox_reset_request`] on a same-session sandbox
//! reset; teardown additionally clears [`PossessionState`] because a retirement
//! (unlike a reset) despawns the player, leaving its possession handles dangling.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_platformer_primitives::lifecycle::{SessionScopeRetired, SessionScopeSet};
use ambition_platformer_primitives::markers::ControlledSubject;

use crate::abilities::traversal::possession::PossessionState;
use crate::boss_encounter::BossEncounterRegistry;
use crate::control::SlotInteractionState;
use crate::encounter::{EncounterRegistry, EncounterView, SwitchActivationQueue};
use crate::SandboxSimState;
use ambition_persistence::quest::QuestRegistry;
use ambition_world::collision::MovingPlatformSet;

/// The process-global resources that mirror one live session's state and must be
/// cleared when that session retires. Grouped so the teardown system stays within
/// Bevy's system-parameter budget and so the ownership set is stated in one place.
#[derive(SystemParam)]
pub struct SessionScopedResources<'w> {
    /// The active room's advancing platform kinematics; a fresh activation
    /// rebuilds it from the new room (and it is snapshot-registered state).
    moving_platforms: ResMut<'w, MovingPlatformSet>,
    /// Possession pair (`possessed`/`home` entity handles + restore brain). The
    /// player is despawned on retirement, so these would dangle.
    possession: ResMut<'w, PossessionState>,
    /// The driven-body handle. It self-heals each tick from the `Brain::Player`
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
    sim_state: ResMut<'w, SandboxSimState>,
    /// Slot-level buffered gestures belong to the retired control session. The
    /// simulation sleeps at the launcher, so they cannot rely on a later tick
    /// to decay before the next activation.
    slot_interactions: ResMut<'w, SlotInteractionState>,
    /// Switch activations intentionally cross one simulation-frame boundary.
    /// Retirement between production and consumption must not deliver a
    /// session-A activation into session B.
    switch_activations: ResMut<'w, SwitchActivationQueue>,
}

/// Reset the session-scoped resource mirrors when any session scope retires.
///
/// Reads [`SessionScopeRetired`] (emitted by the game-shell bridge on route
/// deactivation, alongside the entity-sweep signal). The mirrors are
/// process-global rather than per-scope, so a single reset on retirement is
/// correct — there is at most one live session at a time.
pub fn reset_session_scoped_resources_on_retire(
    mut retired: MessageReader<SessionScopeRetired>,
    mut resources: SessionScopedResources,
) {
    // Drain the channel; act if any scope retired this frame.
    if retired.read().count() == 0 {
        return;
    }
    *resources.moving_platforms = MovingPlatformSet::default();
    *resources.possession = PossessionState::default();
    *resources.controlled_subject = ControlledSubject::default();
    *resources.encounter_registry = EncounterRegistry::default();
    *resources.encounter_view = EncounterView::default();
    *resources.boss_registry = BossEncounterRegistry::default();
    *resources.quest_registry = QuestRegistry::default();
    *resources.sim_state = SandboxSimState::default();
    *resources.slot_interactions = SlotInteractionState::default();
    *resources.switch_activations = SwitchActivationQueue::default();
}

/// Installs [`reset_session_scoped_resources_on_retire`] into the exact-scope
/// cleanup seam, beside the generic entity sweep.
///
/// Added by the platformer provider lifecycle (the session activation authority),
/// which is only composed by shell hosts — the exact contexts that install
/// `SessionScopePlugin` and therefore emit [`SessionScopeRetired`]. Direct-entry
/// and headless apps never retire a session, so they do not install this.
pub struct SessionTeardownPlugin;

impl Plugin for SessionTeardownPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            reset_session_scoped_resources_on_retire.in_set(SessionScopeSet::Cleanup),
        );
    }
}

#[cfg(test)]
mod tests;
