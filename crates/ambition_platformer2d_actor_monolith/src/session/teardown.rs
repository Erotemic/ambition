//! Reset process-global mirrors of live-session state when a session scope retires.
//! Entity cleanup handles `SessionScopedEntity`; this module clears resources that retain entity
//! handles or per-session latches. App-global authored catalogs and registries remain intact.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{SessionScopeRetired, SessionScopeSet};
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;

use crate::abilities::traversal::possession::PossessionState;
use ambition_characters::control::SlotInteractionState;
use crate::encounter::SwitchActivationQueue;
use ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown;
use ambition_boss_encounter::BossEncounterRegistry;
use ambition_encounter::{EncounterRegistry, EncounterView};
use ambition_persistence::quest::QuestRegistry;
use ambition_platformer2d_world::collision::MovingPlatformSet;

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
}

/// Reset process-global session mirrors when any live session scope retires.
pub fn reset_session_scoped_resources_on_retire(
    mut retired: MessageReader<SessionScopeRetired>,
    mut resources: SessionScopedResources,
) {
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
    *resources.sim_state = RoomTransitionCooldown::default();
    *resources.slot_interactions = SlotInteractionState::default();
    *resources.switch_activations = SwitchActivationQueue::default();
    *resources.save_restored = crate::session::durable_horizon::SaveRestored::default();
}

/// Installs session-resource reset in the exact-scope cleanup seam.
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
