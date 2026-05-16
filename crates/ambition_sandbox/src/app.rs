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

use ambition_engine as ae;
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

use crate::audio::SfxMessage;
#[cfg(feature = "audio")]
use crate::audio::{
    apply_audio_settings, audio_play_sfx_messages, start_default_music, MusicChannel, SfxChannel,
};
use crate::config::{WINDOW_H, WINDOW_W};
use crate::content_validation;
use crate::data;
use crate::debug_overlay;
use crate::dev_tools::{
    self, DeveloperTools, EditableAbilitySet, EditableMovementTuning, EditablePlayerStats,
    MovementProfile, PlayerBodyProfile,
};
use crate::dialog;
use crate::features;
use crate::feel::SandboxFeelTuning;
use crate::fx::{self, vfx_spawn_messages, ParticleKind, VfxMessage};
use crate::game_assets::{self, GameAssetConfig};
use crate::game_mode::{gameplay_allowed, gameplay_suspended, GameMode};
#[cfg(feature = "input")]
use crate::input::SandboxAction;
use crate::input::{ControlFrame, MenuControlFrame, GAMEPAD_MAP};
#[cfg(feature = "input")]
use crate::input::{MenuInputState, PlayerDashTriggerState};
use crate::inventory;
use crate::ldtk_world;
use crate::loading;
use crate::pause_menu;
#[cfg(feature = "physics_debris")]
use crate::physics::physics_spawn_debris_messages;
use crate::physics::{self, DebrisBurstMessage};
use crate::platforms;
use crate::rendering::{
    animate_bosses, animate_characters, animate_player, camera_follow, spawn_room_visuals,
    sync_visuals, upgrade_boss_sprites, upgrade_enemy_sprites, upgrade_npc_sprites, HudText,
    PlayerVisual, RoomVisual, SceneEntities,
};
use crate::rooms;
use crate::setup;
use crate::ui_fonts;
use crate::windowing;
use crate::{GameWorld, PlayerDiedMessage, SandboxDevState};

mod cli;
mod dev_runtime;
mod feedback;
mod hud;
mod input_systems;
mod phases;
mod plugins;
mod resources;
mod schedule;
mod setup_systems;
mod sim_systems;
mod update;
mod world_flow;

pub use cli::run_visible;
pub use feedback::{ProgressionResources, SandboxEventWriters, SandboxQueues};
pub use hud::update_quest_panel;
#[cfg(feature = "input")]
pub use input_systems::{
    apply_menu_frame_to_cutscene_request, populate_control_frame_from_actions,
    populate_menu_control_frame_from_actions,
};
pub use plugins::{
    add_ldtk_runtime_plugin, add_presentation_plugins, add_simulation_plugins,
    SandboxLdtkPlugin, SandboxPresentationPlugin, SandboxSimulationPlugin,
};
pub use resources::{init_sandbox_resources, StartRoomOverride};
pub use schedule::{configure_sandbox_sets, SandboxSet};
pub use sim_systems::{
    apply_suspended_time_scale_system, cleanup_timers_system, detect_room_transition_system,
    input_timer_system, interaction_input_system, sync_live_player_dev_edits_system,
};
pub use update::sandbox_update;
pub use world_flow::apply_room_transition_system;
