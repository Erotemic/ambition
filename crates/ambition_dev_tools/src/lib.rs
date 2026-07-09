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

pub use persistence::DeveloperPersistenceSchedulePlugin;

use bevy::prelude::*;

use ambition_engine_core::{
    BodyAbilities, BodyBlinkState, BodyDashState, BodyFlightState, BodyJumpState,
};
use ambition_platformer_primitives::markers::PrimaryPlayerOnly;
use dev_tools::{EditableAbilitySet, EditableMovementTuning};

/// Push live dev-tools ability/tuning edits onto the authoritative player.
///
/// Registered by the host to run even while gameplay is suspended so the F3
/// inspector stays responsive; the logic is body-state mutation and lives here
/// beside the dev STATE it reads.
pub fn sync_live_player_dev_edits_system(
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    mut player_q: Query<
        (
            &mut BodyAbilities,
            &mut BodyFlightState,
            &mut BodyBlinkState,
            &mut BodyDashState,
            &mut BodyJumpState,
        ),
        PrimaryPlayerOnly,
    >,
) {
    let Ok((mut abilities, mut flight, mut blink, mut dash, mut jump)) = player_q.single_mut()
    else {
        return;
    };
    dev_tools::sync_live_ability_edits_clusters(
        &mut abilities,
        &mut flight,
        &mut blink,
        &mut dash,
        &mut jump,
        editable_abilities.as_engine(),
        editable_tuning.as_engine(),
    );
}

/// Developer/debug state: keyboard preset selection and debug flags.
///
/// This crate stores only the preset index, not the input crate's
/// `KeyboardPreset` table. Input-owning callers map the index through
/// `ambition_input::KeyboardPreset::by_index` when they need an input map or
/// HUD glyphs. Keeping this state as an index preserves `ambition_dev_tools` as
/// foundational dev-tool state instead of pulling the input layer underneath it.
#[derive(Resource)]
pub struct SandboxDevState {
    pub debug: bool,
    pub slowmo: bool,
    pub preset_index: usize,
    pub preset_flash: f32,
}

impl Default for SandboxDevState {
    fn default() -> Self {
        Self {
            debug: !cfg!(target_os = "android"),
            slowmo: false,
            preset_index: 0,
            preset_flash: 1.2,
        }
    }
}

impl SandboxDevState {
    pub fn debug_enabled(&self) -> bool {
        self.debug
    }
}
