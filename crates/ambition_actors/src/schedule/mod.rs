//! Schedule + input-frame vocabulary shared by the machinery lib, the
//! content crate, and the app crate.
//!
//! The Bevy app ASSEMBLY (plugins, resources, sim systems, HUD, CLI)
//! moved to the `ambition_app` crate (Stage 20 / A3). What stays here
//! is the vocabulary other layers order against: the `SandboxSet`
//! schedule labels (+ the content/machinery slot sets) and the
//! device -> ControlFrame populate systems the menu/host layers anchor
//! to.

mod input_systems;
mod schedule;

pub use ambition_platformer_primitives::schedule::{
    BossSteerSlot, CombatSet, PresentationSetupSet, SandboxSet, SimulationSetupSet,
};
pub use input_systems::declare_gameplay_input_context;
#[cfg(feature = "input")]
pub use input_systems::{
    apply_menu_frame_to_cutscene_request, populate_control_frame_from_actions,
    populate_menu_control_frame_from_actions, spawn_primary_input_participant,
    toggle_player_trail_emission_from_actions, MenuNavConsume,
};
pub use schedule::configure_sandbox_sets;
