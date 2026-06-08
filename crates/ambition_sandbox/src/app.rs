//! Sandbox app-builder: domain plugins, helpers, and gameplay systems shared
//! between the visible binary (`src/main.rs`) and headless drivers
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

use crate::engine_core as ae;
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

use crate::assets::game_assets::{self, GameAssetConfig};
use crate::assets::loading;
use crate::audio::SfxMessage;
#[cfg(feature = "audio")]
use crate::audio::{
    apply_audio_environment, audio_play_sfx_messages, detect_audio_environment,
    smooth_audio_environment, start_default_music_when_ready, AudioEnvironment,
    DefaultMusicStarted, MusicChannel, SfxChannel,
};
use crate::config::{WINDOW_H, WINDOW_W};
use crate::content::content_validation;
use crate::content::data;
use crate::dev::debug_overlay;
use crate::dev::dev_tools::{
    self, DeveloperTools, EditableAbilitySet, EditableMovementTuning, EditablePlayerStats,
    MovementProfile, PlayerBodyProfile,
};
use crate::dialog;
use crate::features;
use crate::game_mode::{gameplay_allowed, gameplay_suspended, GameMode};
use crate::host::windowing;
#[cfg(feature = "input")]
use crate::input::SandboxAction;
use crate::input::{ControlFrame, MenuControlFrame, GAMEPAD_MAP};
#[cfg(feature = "input")]
use crate::input::{MenuInputState, PlayerDashTriggerState};
use crate::inventory;
use crate::ldtk_world;
use crate::platformer_runtime::lifecycle::RoomScopedEntity;
use crate::presentation::fx::{self, vfx_spawn_messages, ParticleKind, VfxMessage};
use crate::presentation::rendering::{
    animate_bosses, animate_characters, animate_player, camera_follow, spawn_room_visuals,
    sync_visuals, upgrade_boss_sprites, upgrade_enemy_sprites, upgrade_npc_sprites, HudText,
    PlayerVisual, SceneEntities,
};
use crate::presentation::ui_fonts;
use crate::rooms;
use crate::runtime::setup;
use crate::time::feel::SandboxFeelTuning;
#[cfg(feature = "physics_debris")]
use crate::world::physics::physics_spawn_debris_messages;
use crate::world::physics::{self, DebrisBurstMessage};
use crate::world::platforms;
use crate::{GameWorld, PlayerDiedMessage, SandboxDevState};

mod cli;
mod combat_schedule;
mod dev_runtime;
mod feedback;
mod hud;
mod input_systems;
mod phases;
mod player_tick;
mod plugins;
mod progression_schedule;
mod resources;
mod schedule;
mod setup_systems;
mod sim_resources;
mod sim_systems;
mod world_flow;

#[cfg(not(target_arch = "wasm32"))]
pub use cli::run_visible;
#[cfg(all(target_arch = "wasm32", feature = "web_platform"))]
pub use cli::run_web;
pub use feedback::{ProgressionResources, SandboxEventWriters, SandboxQueues};
pub use hud::update_quest_panel;
#[cfg(feature = "input")]
pub use input_systems::{
    apply_menu_frame_to_cutscene_request, populate_control_frame_from_actions,
    populate_menu_control_frame_from_actions, MenuNavConsume,
};
pub use player_tick::{
    clear_sandbox_reset_this_frame, player_control_system, player_simulation_system,
};
pub use plugins::{
    add_ldtk_runtime_plugin, add_presentation_plugins, add_simulation_plugins, SandboxLdtkPlugin,
    SandboxPresentationPlugin, SandboxSimulationPlugin,
};
pub use resources::{init_sandbox_resources, SandboxResetThisFrame, StartRoomOverride};
pub use schedule::{configure_sandbox_sets, SandboxSet};
pub(crate) use setup_systems::setup_presentation_system;
pub use sim_systems::{
    apply_player_hit_events, apply_player_reset_input_system, apply_suspended_time_scale_system,
    attack_advance_system, cleanup_timers_system, detect_room_transition_system,
    input_timer_system, interaction_input_system, sync_live_player_dev_edits_system,
};
pub use world_flow::apply_room_transition_system;
