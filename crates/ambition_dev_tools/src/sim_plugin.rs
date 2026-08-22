//! `DevToolsSimPlugin` — the dev-tools DOMAIN plugin for the simulation App.
//!
//! Owns the dev-editable simulation resources and registers their live-edit
//! systems into public sets. The runtime positions those sets in its phase
//! chains without naming the leaf systems.

use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, SystemSet};

/// PlayerInput-phase seam: apply the developer's live tuning edits
/// (movement/abilities/stats mirrors) onto the controlled body BEFORE the
/// input→brain chain consumes them this frame. The sim assembly positions this
/// set at the tail of its time-control chain; anything that must observe the
/// post-edit state orders `.after(DevEditApplySet)`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DevEditApplySet;

/// Progression-phase seam: mirror the player's live stats back into the
/// inspector-editable resource so the F3 panel shows truth.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DevInspectorMirrorSet;

pub struct DevToolsSimPlugin;

impl Plugin for DevToolsSimPlugin {
    fn build(&self, app: &mut App) {
        // The dev-editable sim resources this crate owns (anti-god rule: the
        // plugin that owns the systems initializes their resources).
        app.init_resource::<crate::profiling::StartupProfiler>();
        app.init_resource::<crate::profiling::FrameCensus>();
        app.init_resource::<crate::DeveloperRuntimeState>();
        app.init_resource::<crate::dev_tools::DeveloperTools>();
        app.init_resource::<crate::dev_tools::EditablePlayerStats>();
        app.init_resource::<crate::dev_tools::EditableMovementTuning>();
        app.init_resource::<crate::dev_tools::EditableAbilitySet>();
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            (
                // Editor → neutral authority, before the body-side edit apply
                // reads it. Sim systems never see the inspector mirror.
                crate::dev_tools::apply_editable_movement_tuning,
                crate::sync_live_player_dev_edits_system,
                // This mutates rollback state, so it must run in the simulation
                // schedule with the other developer edits.
                crate::dev_tools::sync_developer_body_profile,
            )
                .chain()
                .in_set(DevEditApplySet),
        );
        app.add_systems(
            sim,
            crate::dev_tools::sync_player_stats_with_inspector.in_set(DevInspectorMirrorSet),
        );
    }
}
