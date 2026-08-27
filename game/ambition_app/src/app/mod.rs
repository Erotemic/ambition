//! Sandbox app-builder: domain plugins, helpers, and gameplay systems shared
//! between the visible binary (`src/bin/ambition_game_bin.rs`) and headless drivers
//! (`src/headless.rs`, `src/rl_sim/runtime.rs`).
//!
//! ## Plugin API (preferred)
//!
//! * [`AmbitionGameSimulationPlugin`] — all sim resources + systems; safe for
//!   headless and visible builds.
//! * [`AmbitionGameLdtkRuntimePlugin`] — LDtk runtime spine + `LdtkPlugin`; visible only.
//! * [`AmbitionGamePresentationPlugin`] — input, audio, VFX, HUD, debug; visible only.
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
pub mod shell_host;
mod sim_resources;
mod sim_systems;
mod startup_loading;
pub mod versus;
pub mod versus_fighters;
pub mod versus_rules;
pub mod visible_composition;
pub(crate) mod world_flow;

#[cfg(feature = "input")]
pub use ambition_platformer2d::actors::schedule::{
    apply_menu_frame_to_cutscene_request, populate_menu_control_frame_from_actions, MenuNavConsume,
};
pub use ambition_platformer2d::actors::schedule::configure_platformer2d_simulation_phases;
pub use ambition_platformer2d::sim::{
    BossSteerSlot, Platformer2dSimulationPhaseMonolith, PresentationSetupSet,
};
#[cfg(not(target_arch = "wasm32"))]
pub use cli::run_visible;
#[cfg(all(target_arch = "wasm32", feature = "web_platform"))]
pub use cli::run_web;
#[cfg(not(target_arch = "wasm32"))]
pub use cli::{
    build_visible_app, build_visible_app_with, prefetch_preparations,
    run_shared_host_acceptance_cycle, run_shared_host_headless, shared_host_startup_ticks,
    SharedHostAcceptanceReport, SharedHostHeadlessReport, VisibleRenderMode,
    SHARED_HOST_HEADLESS_TICK_HZ,
};
pub use feedback::{GameplayFeedbackWriters, ProgressionResources};
pub use hud::update_quest_panel;
pub use player_clone::{PlayerClone, SpawnPlayerCloneRequest};
// Re-exported here so existing `ambition_app::app::PlayerBodyFrameOutput` paths (tests) keep
// working.
pub use ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput;
pub use player_tick::sync_player_presentation;
pub use plugins::{
    add_ldtk_runtime_plugin, add_presentation_plugins, add_simulation_plugins,
    AmbitionGameLdtkRuntimePlugin, AmbitionGamePresentationPlugin, AmbitionGameSimulationPlugin,
};
pub use resources::{
    init_sandbox_resources, SeatsAMatchInsteadOfAHomeBody, StartRoomMustResolve, StartRoomOverride,
    StartingCharacterOverride,
};
pub use sim_systems::apply_player_reset_input_system;
pub use world_flow::RoomTransitionCoverSet;
