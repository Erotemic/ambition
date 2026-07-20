//! `DevToolsSimPlugin` — the dev-tools DOMAIN plugin for the simulation App.
//!
//! Track 6 (decisions-2026-07-16 #9): domain crates install their own local
//! resources and systems and expose public SETS; the sim assembly orders sets
//! instead of naming leaf systems. This plugin owns the dev-editable sim
//! resources (formerly initialized inside the runtime's core-resources bundle)
//! and registers the two live-edit sync systems into the public sets below —
//! `ambition_runtime` now only positions [`DevEditApplySet`] /
//! [`DevInspectorMirrorSet`] in its phase chains.

use ambition_platformer_primitives::schedule::SimScheduleExt;
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
        app.init_resource::<crate::SandboxDevState>();
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
