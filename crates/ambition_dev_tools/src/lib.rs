//! Reusable developer-tooling state and simulation-side logic.
//!
//! Owns:
//!
//! - [`dev_tools`] — the [`DeveloperTools`](dev_tools::DeveloperTools) debug/
//!   gizmo toggle resource, the reflected editable player-tuning / ability /
//!   stats resources + their engine conversions, the movement/debug profile
//!   enums, and the inspector-visibility run conditions. Plus the live-edit
//!   sync systems that push inspector edits onto the authoritative player body
//!   (they name only the foundational `Body*` clusters + `PrimaryPlayerOnly`).
//! - [`profiling`] — the startup profiler marks (read by audio + setup).
//! - [`runtime_census`] — the profiling-only workload censuses (off unless
//!   `AMBITION_PROFILE_CENSUS` is set) and the clock every census samples on.
//! - [`persistence`] — `DeveloperTools` disk persistence (developer.ron).
//! - [`sync_live_player_dev_edits_system`] — the host-scheduled system that
//!   applies live ability/tuning edits to the player each frame.
//!
//! Presentation UI remains in `ambition_app`; gameplay tracing remains with the
//! simulation state it samples.

pub mod dev_tools;
/// The authored world file's mtime watch + reload status — the state half of
/// the Developer page's auto-apply row.
pub mod hot_reload;
pub mod persistence;
pub mod profiling;
/// The profiling-only workload censuses and the shared census clock they sample on.
pub mod runtime_census;
pub mod sim_plugin;

pub use hot_reload::{poll_world_source_changes, WorldSourceHotReload};
pub use persistence::DeveloperPersistenceSchedulePlugin;
pub use sim_plugin::{DevEditApplySet, DevInspectorMirrorSet, DevToolsSimPlugin};

use bevy::prelude::*;

use ambition_platformer2d_core::{
    AbilityBase, ActiveMovementTuning, AuthoredMovementTuning, BodyAbilities, BodyDashState,
    BodyFlightState, BodyJumpState, MotionModel,
};
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly;
use dev_tools::EditableAbilitySet;

/// Push live dev-tools ability/tuning edits onto the authoritative player.
///
/// Registered by the host to run even while gameplay is suspended so the F3
/// inspector stays responsive; the logic is body-state mutation and lives here
/// beside the dev STATE it reads.
///
/// The editable ability set is a session MASK, not a wholesale replacement:
/// the effective set is the body's intrinsic [`AbilityBase`] intersected with
/// the editable set. A mask can only ever gate a verb OFF, never conjure one the
/// character was not authored to have — so a restricted character (a demo
/// protagonist authored with a run-and-jump kit) keeps its identity instead of
/// being clobbered up to the inspector's `sandbox_all` default every frame. For
/// the sandbox protagonist (base `sandbox_all`) the intersection equals the
/// editable set, so the F3 experiment workflow is unchanged.
pub fn sync_live_player_dev_edits_system(
    // The neutral authority, NOT the inspector mirror: `apply_editable_movement_tuning`
    // is chained immediately before this in `DevEditApplySet`, so an F3 edit is
    // already here — and a body whose tuning came from content rather than the
    // inspector now resolves correctly too.
    active_tuning: Res<ActiveMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    mut player_q: Query<
        (
            &mut BodyAbilities,
            &AbilityBase,
            &mut BodyFlightState,
            &mut MotionModel,
            &mut BodyDashState,
            &mut BodyJumpState,
            // Presence means the body's feel is authored (a demo protagonist),
            // so the resource-refresh below uses THAT tuning's air-jump count,
            // never the shared editable's — the same rule the live integrator
            // applies. Absent for the sandbox protagonist, which tracks F3.
            Option<&AuthoredMovementTuning>,
        ),
        PrimaryPlayerOnly,
    >,
) {
    let Ok((mut abilities, base, mut flight, mut model, mut dash, mut jump, authored_tuning)) =
        player_q.single_mut()
    else {
        return;
    };
    let desired_abilities = base.abilities.intersect(editable_abilities.as_engine());
    let effective_tuning = authored_tuning.map(|t| t.0).unwrap_or(active_tuning.0);
    // Reading through `Mut<T>` is change-neutral; coercing it to `&mut T` is
    // not. Keep the equality guard here, before the helper call, so an
    // unchanged inspector resource does not mark `BodyAbilities` changed every
    // frame and spuriously refresh every downstream derived persona system.
    if abilities.abilities == desired_abilities {
        return;
    }
    dev_tools::sync_live_ability_edits_clusters(
        &mut abilities,
        &mut flight,
        &mut model,
        &mut dash,
        &mut jump,
        desired_abilities,
        effective_tuning,
    );
}

/// Developer/debug state: debug flags and the HUD flash timer. Keyboard preset
/// selection is owned by persisted user settings, not duplicated here.
#[derive(Resource)]
pub struct DeveloperRuntimeState {
    pub debug: bool,
    pub slowmo: bool,
    pub preset_flash: f32,
}

impl Default for DeveloperRuntimeState {
    fn default() -> Self {
        Self {
            debug: false,
            slowmo: false,
            preset_flash: 1.2,
        }
    }
}

impl DeveloperRuntimeState {
    pub fn debug_enabled(&self) -> bool {
        self.debug
    }
}

/// Turn the COMBAT overlay on, everywhere it is gated.
///
/// ⛔⛔ THE GIZMO PASS IS GATED ON THREE SEPARATE THINGS — the debug flag, the
/// gizmo toggle, and the per-view fields — and all three are off in a plain
/// build. Missing any one of them produces a photograph of a swing with no
/// volume on it, which reads as "the move has no hitbox" rather than "the
/// overlay is off". Every tool that wants combat geometry in a picture asks for
/// it here, so the count of gates lives in one place.
///
/// Idempotent: safe to call every frame, which is what a capture tool must do —
/// settings load and the developer-tools default both write this state, so a
/// startup-only write is a race against whichever of them runs last.
pub fn force_combat_overlay(
    state: &mut DeveloperRuntimeState,
    tools: &mut dev_tools::DeveloperTools,
) {
    if !state.debug {
        state.debug = true;
    }
    if !tools.gizmos_enabled {
        tools.gizmos_enabled = true;
    }
    if tools.debug_view_mode != dev_tools::DebugViewMode::Combat {
        tools.apply_debug_view_mode(dev_tools::DebugViewMode::Combat, false);
    }
}

#[cfg(test)]
mod developer_runtime_state_tests {
    use super::*;

    #[test]
    fn debug_overlay_defaults_off_for_every_game() {
        assert!(!DeveloperRuntimeState::default().debug);
    }
}
