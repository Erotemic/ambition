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

#![allow(unused_imports)]

use ambition_gameplay_core::engine_core as ae;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResizeConstraints, WindowResolution};
use bevy_asset_loader::asset_collection::AssetCollectionApp;
use bevy_common_assets::ron::RonAssetPlugin;
use bevy_ecs_ldtk::prelude::{IntGridRendering, LdtkPlugin, LdtkSettings, LevelBackground};
#[cfg(feature = "dev_tools")]
use bevy_inspector_egui::{
    bevy_egui::EguiPlugin,
    quick::{ResourceInspectorPlugin, WorldInspectorPlugin},
};
#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::{
    AudioApp, AudioPlugin as KiraAudioPlugin, AudioSource as KiraAudioSource,
};
#[cfg(feature = "ui")]
use bevy_material_ui::MaterialUiPlugin;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::{ActionState, InputManagerPlugin, InputMap};

use crate::dev::debug_overlay;
use crate::host::windowing;
use ambition_content::content_validation;
use ambition_gameplay_core::assets::game_assets::{self, GameAssetConfig};
use ambition_gameplay_core::assets::loading;
use ambition_gameplay_core::audio::SfxMessage;
#[cfg(feature = "audio")]
use ambition_gameplay_core::audio::{
    apply_audio_environment, audio_play_sfx_messages, detect_audio_environment,
    smooth_audio_environment, start_default_music_when_ready, AudioEnvironment,
    DefaultMusicStarted, MusicChannel, SfxChannel,
};
use ambition_gameplay_core::config::{WINDOW_H, WINDOW_W};
use ambition_gameplay_core::dev::dev_tools::{
    self, DeveloperTools, EditableAbilitySet, EditableMovementTuning, EditablePlayerStats,
    MovementProfile, PlayerBodyProfile,
};
use ambition_gameplay_core::dialog;
use ambition_gameplay_core::features;
use ambition_gameplay_core::game_mode::{gameplay_allowed, gameplay_suspended, GameMode};
#[cfg(feature = "input")]
use ambition_gameplay_core::input::SandboxAction;
use ambition_gameplay_core::input::{ControlFrame, MenuControlFrame, GAMEPAD_MAP};
#[cfg(feature = "input")]
use ambition_gameplay_core::input::{MenuInputState, PlayerDashTriggerState};
use ambition_gameplay_core::inventory_ui;
use ambition_gameplay_core::ldtk_world;
use ambition_gameplay_core::platformer_runtime::lifecycle::RoomScopedEntity;
use ambition_gameplay_core::rooms;
use ambition_gameplay_core::session::data;
use ambition_gameplay_core::session::setup;
use ambition_gameplay_core::time::feel::SandboxFeelTuning;
#[cfg(feature = "physics_debris")]
use ambition_gameplay_core::world::physics::physics_spawn_debris_messages;
use ambition_gameplay_core::world::physics::{self, DebrisBurstMessage};
use ambition_gameplay_core::world::platforms;
use ambition_gameplay_core::{GameWorld, PlayerDiedMessage, SandboxDevState};
use ambition_render::fx::{self, vfx_spawn_messages, ParticleKind, VfxMessage};
use ambition_render::rendering::{
    animate_bosses, animate_characters, animate_player, camera_follow, spawn_room_visuals,
    sync_visuals, upgrade_boss_sprites, upgrade_enemy_sprites, upgrade_npc_sprites, HudText,
    PlayerVisual, SceneEntities,
};
use ambition_render::ui_fonts;

mod cli;
mod combat_schedule;
mod dev_runtime;
mod feedback;
mod hud;
mod phases;
mod player_clone;
mod player_tick;
mod plugins;
mod progression_schedule;
mod resources;
mod scene_setup;
mod setup_systems;
mod sim_resources;
mod sim_systems;
mod world_flow;

#[cfg(feature = "input")]
pub use ambition_gameplay_core::schedule::{
    apply_menu_frame_to_cutscene_request, populate_control_frame_from_actions,
    populate_menu_control_frame_from_actions, MenuNavConsume,
};
pub use ambition_gameplay_core::schedule::{
    configure_sandbox_sets, BossSteerSlot, PresentationSetupSet, SandboxSet,
};
#[cfg(not(target_arch = "wasm32"))]
pub use cli::run_visible;
#[cfg(all(target_arch = "wasm32", feature = "web_platform"))]
pub use cli::run_web;
pub use feedback::{ProgressionResources, SandboxEventWriters, SandboxQueues};
pub use hud::update_quest_panel;
pub use player_clone::{PlayerClone, SpawnPlayerCloneRequest};
pub use player_tick::{
    clear_sandbox_reset_this_frame, player_control_system, player_simulation_system,
};
pub use plugins::{
    add_ldtk_runtime_plugin, add_presentation_plugins, add_simulation_plugins, SandboxLdtkPlugin,
    SandboxPresentationPlugin, SandboxSimulationPlugin,
};
pub use resources::{init_sandbox_resources, SandboxResetThisFrame, StartRoomOverride};
pub(crate) use setup_systems::setup_presentation_system;
pub use sim_systems::{
    apply_player_hit_events, apply_player_reset_input_system, apply_suspended_time_scale_system,
    attack_advance_system, cleanup_timers_system, detect_room_transition_system,
    input_timer_system, interaction_input_system, sync_live_player_dev_edits_system,
};
