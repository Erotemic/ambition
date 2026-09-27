//! Ambition's portal gameplay setting.
//!
//! This module carries the `portal_reverses_facing` gameplay setting into the
//! editable portal tuning. What a transit means for a body (carried momentum, a
//! shot's carried acceleration) and for the seat that drives it is the
//! runtime's (`portal_body`, `portal_seat`).

use bevy::prelude::*;

/// Carry the `portal_reverses_facing` gameplay setting into the editable
/// portal tuning, and propose it.
///
/// It writes `EditablePortalTuning`, not `PortalTuning`. Writing the
/// authority from inside `GgrsSchedule` would do this:
///
/// ```text
/// developer edits reorient_facing in the portal inspector
///   -> proposed, admitted, published into PortalTuning
///   -> GgrsSchedule runs
///   -> THIS overwrites it from the persisted gameplay setting
/// ```
///
/// Then the last writer wins. It would also read `UserSettings` inside the
/// rollback window, so a replay of frame N would see the current setting.
///
/// `EditablePortalTuning` is where the field is authored, and
/// `publish_editable_portal_tuning` is the only writer of the authority.
///
/// For `reorient_facing`, the gameplay setting is the author and the panel is
/// not. This system is change-guarded on `editable.reorient_facing != want`,
/// so it does nothing while they agree and republishes the persisted value
/// when they differ. An inspector edit of this field is therefore reverted on
/// the next pass, with no race. The panel's row states this.
///
/// The other `EditablePortalTuning` fields are panel-authored: this system
/// touches one field only.
///
/// The gameplay setting defaults off, so by default the player keeps the same
/// facing through a same-wall portal turn-around; the portal crate's own
/// default stays on for standalone use. Change-guarded, so an untouched
/// setting proposes nothing and never stops a rollback baseline.
pub fn sync_portal_reorient_from_settings(
    // Optional: headless / unit-test apps may run portal transit without the
    // settings resource. Absent → leave the portal crate's default (ON).
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut editable: ResMut<ambition_platformer2d::portal::EditablePortalTuning>,
    mut pending: ResMut<ambition_platformer2d_core::PendingMechanicalEdits>,
) {
    let Some(settings) = settings else {
        return;
    };
    let want = settings.gameplay.portal_reverses_facing;
    if editable.reorient_facing != want {
        editable.reorient_facing = want;
        pending.propose(ambition_platformer2d::portal::portal_tuning_domain());
    }
}
