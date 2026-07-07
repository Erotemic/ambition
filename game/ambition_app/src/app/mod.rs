//! Sandbox app-builder: domain plugins, helpers, and gameplay systems shared
//! between the visible binary (`src/bin/ambition_game_bin.rs`) and headless drivers
//! (`src/headless.rs`, `src/rl_sim/runtime.rs`).
//!
//! ## Plugin API (preferred)
//!
//! * [`SandboxSimulationPlugin`] — all sim resources + systems; safe for
//!   headless and visible builds.
//! * [`SandboxLdtkPlugin`] — LDtk runtime spine + `LdtkPlugin`; visible only.
//! * [`SandboxPresentationPlugin`] — input, audio, VFX, HUD, debug; visible only.
//!
//! ## Function API (lower-level)
//!
//! * [`init_sandbox_resources`] — parse + validate LDtk world, insert resources.
//! * [`add_simulation_plugins`] — register sim plugins and update schedule.
//! * [`add_ldtk_runtime_plugin`] — register LDtk runtime.
//! * [`add_presentation_plugins`] — register presentation systems.
//!
//! Use the function API when you need to inject resources between steps
//! (e.g. `StartRoomOverride`); use the plugin API otherwise.

mod cli;
mod dev_runtime;
mod feedback;
mod hud;
mod phases;
mod player_clone;
mod player_tick;
mod plugins;
mod resources;
mod scene_setup;
mod setup_systems;
mod sim_resources;
mod sim_systems;
pub(crate) mod world_flow;

#[cfg(feature = "input")]
pub use ambition_actors::schedule::{
    apply_menu_frame_to_cutscene_request, populate_control_frame_from_actions,
    populate_menu_control_frame_from_actions, MenuNavConsume,
};
pub use ambition_actors::schedule::{
    configure_sandbox_sets, BossSteerSlot, PresentationSetupSet, SandboxSet,
};
#[cfg(not(target_arch = "wasm32"))]
pub use cli::run_visible;
#[cfg(all(target_arch = "wasm32", feature = "web_platform"))]
pub use cli::run_web;
pub use feedback::{ProgressionResources, SandboxEventWriters, SandboxQueues};
pub use hud::update_quest_panel;
pub use player_clone::{PlayerClone, SpawnPlayerCloneRequest};
// `PlayerBodyFrameOutput` moved to `ambition_actors::player` (the movement
// phase that writes it is now the unified `integrate_sim_bodies`). Re-exported here
// so existing `ambition_app::app::PlayerBodyFrameOutput` paths (tests) keep working.
pub use ambition_actors::player::PlayerBodyFrameOutput;
pub use player_tick::sync_player_presentation;
pub use plugins::{
    add_ldtk_runtime_plugin, add_presentation_plugins, add_simulation_plugins, SandboxLdtkPlugin,
    SandboxPresentationPlugin, SandboxSimulationPlugin,
};
pub use resources::{init_sandbox_resources, StartRoomOverride};
pub use sim_systems::{apply_player_reset_input_system, apply_room_replay_request_system};
