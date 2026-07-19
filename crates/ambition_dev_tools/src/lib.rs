//! Reusable developer-tooling state + logic (E1d carve out of
//! `ambition_actors`).
//!
//! Owns the content-free half of the old `dev/` module:
//!
//! - [`dev_tools`] — the [`DeveloperTools`](dev_tools::DeveloperTools) debug/
//!   gizmo toggle resource, the reflected editable player-tuning / ability /
//!   stats resources + their engine conversions, the movement/debug profile
//!   enums, and the inspector-visibility run conditions. Plus the live-edit
//!   sync systems that push inspector edits onto the authoritative player body
//!   (they name only the foundational `Body*` clusters + `PrimaryPlayerOnly`).
//! - [`profiling`] — the startup profiler marks (read by audio + setup).
//! - [`persistence`] — `DeveloperTools` disk persistence (developer.ron).
//! - [`sync_live_player_dev_edits_system`] — the host-scheduled system that
//!   applies live ability/tuning edits to the player each frame.
//!
//! ## What stays elsewhere
//!
//! The egui overlay UI (`DevToolsPlugin`, the F3 inspector, fps + debug
//! overlays, portal inspector) is app-level presentation and stays in
//! `ambition_app`. The gameplay `trace` recorder samples live sim state
//! (`player`/`features`/`rooms`/`portal`/`game_mode`) and stays sim-side in
//! `ambition_actors::dev::trace`.

pub mod dev_tools;
pub mod persistence;
pub mod profiling;
pub mod sim_plugin;

pub use persistence::DeveloperPersistenceSchedulePlugin;
pub use sim_plugin::{DevEditApplySet, DevInspectorMirrorSet, DevToolsSimPlugin};

use bevy::prelude::*;

use ambition_engine_core::{
    AbilityBase, AuthoredMovementTuning, BodyAbilities, BodyDashState, BodyFlightState,
    BodyJumpState, MotionModel,
};
use ambition_platformer_primitives::markers::PrimaryPlayerOnly;
use dev_tools::{EditableAbilitySet, EditableMovementTuning};

/// Push live dev-tools ability/tuning edits onto the authoritative player.
///
/// Registered by the host to run even while gameplay is suspended so the F3
/// inspector stays responsive; the logic is body-state mutation and lives here
/// beside the dev STATE it reads.
///
/// The editable ability set is a **session MASK**, not a wholesale replacement:
/// the effective set is the body's intrinsic [`AbilityBase`] intersected with
/// the editable set. A mask can only ever gate a verb OFF, never conjure one the
/// character was not authored to have — so a restricted character (a demo
/// protagonist authored with a run-and-jump kit) keeps its identity instead of
/// being clobbered up to the inspector's `sandbox_all` default every frame. For
/// the sandbox protagonist (base `sandbox_all`) the intersection equals the
/// editable set, so the F3 experiment workflow is unchanged.
pub fn sync_live_player_dev_edits_system(
    editable_tuning: Res<EditableMovementTuning>,
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
    let effective_tuning = authored_tuning
        .map(|t| t.0)
        .unwrap_or_else(|| editable_tuning.as_engine());
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

/// Developer/debug state: debug flags and the HUD flash timer.
///
/// The keyboard preset index deliberately does NOT live here. It once did, as
/// a second authority beside `UserSettings.controls.keyboard_preset_index` —
/// with no writer, so the settings-menu selector was a silent no-op for
/// keyboard input and HUD glyphs while touch read the real setting. The
/// persisted setting is the ONE authority; input-owning callers map it through
/// `ambition_input::KeyboardPreset::by_index`.
#[derive(Resource)]
pub struct SandboxDevState {
    pub debug: bool,
    pub slowmo: bool,
    pub preset_flash: f32,
}

impl Default for SandboxDevState {
    fn default() -> Self {
        Self {
            debug: false,
            slowmo: false,
            preset_flash: 1.2,
        }
    }
}

impl SandboxDevState {
    pub fn debug_enabled(&self) -> bool {
        self.debug
    }
}

#[cfg(test)]
mod sandbox_dev_state_tests {
    use super::*;

    #[test]
    fn debug_overlay_defaults_off_for_every_game() {
        assert!(!SandboxDevState::default().debug);
    }
}
