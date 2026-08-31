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
        // ⭐ THE WORLD-SOURCE WATCHER IS THIS CRATE'S, and it now says so. Its
        // resource and its `Update` system were registered by the ACTOR KERNEL's
        // feature plugin, which is a simulation package registering a developer
        // facility — the "anti-god rule" one line up, applied to the one row that
        // had escaped it. Default = watcher disabled; the visible app pre-inserts
        // its resolved value before the engine group, and `init_resource` never
        // clobbers.
        app.init_resource::<crate::WorldSourceHotReload>();
        // ⛔⛔ `Update`, NOT THE SIMULATION. It does a BLOCKING `fs::metadata` —
        // measured at up to 3.9ms on virtiofs — so on the sim schedule a
        // dev-tooling stat sat inside the deterministic tick. It also reads
        // wall-clock `Res<Time>` and keeps its debounce in a `Local`, neither of
        // which rewinds. Every reader is a MENU system in `Update` already.
        app.add_systems(bevy::app::Update, crate::poll_world_source_changes);
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
