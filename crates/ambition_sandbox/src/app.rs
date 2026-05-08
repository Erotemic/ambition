//! Visible-binary App-builder helpers and gameplay systems shared between
//! `src/main.rs` (visible) and `src/headless.rs` (run_headless).
//!
//! Slice 5 of ADR 0012's events refactor moved this code out of `main.rs`
//! into the library so the headless binary can drive the same gameplay loop
//! (`sandbox_update` and friends) without InputPlugin / RenderPlugin /
//! Kira audio. The visible binary's `fn main()` is now a thin shim that
//! calls `run_visible`, which composes:
//!
//! * `init_sandbox_resources`: parse + validate the embedded LDtk world,
//!   build the `RoomSet`, and insert sim resources both halves need.
//! * `add_simulation_plugins`: register sim plugins, messages, and the
//!   gameplay schedule. Headless calls this; visible calls this.
//! * `add_presentation_plugins`: register DefaultPlugins-derived rendering,
//!   inspector overlays, audio/VFX/debris subscribers, HUD, debug overlays,
//!   and input-driven systems. Visible calls this; headless does not.

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
use crate::data;
use crate::debug_overlay;
use crate::dev_tools::{
    self, DeveloperTools, EditableAbilitySet, EditableMovementTuning, EditablePlayerStats,
};
use crate::dialog;
use crate::features;
use crate::feel::SandboxFeelTuning;
use crate::fx::{self, vfx_spawn_messages, ParticleKind, VfxMessage};
use crate::game_assets::{self, GameAssetConfig};
use crate::game_mode::GameMode;
#[cfg(feature = "input")]
use crate::input::SandboxAction;
use crate::input::{ControlFrame, GAMEPAD_MAP};
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
    animate_bosses, animate_enemies, animate_player, camera_follow, spawn_room_visuals,
    sync_visuals, upgrade_boss_sprites, upgrade_enemy_sprites, HudText, PlayerVisual, RoomVisual,
    SceneEntities,
};
use crate::rooms;
use crate::setup;
use crate::ui_fonts;
use crate::windowing;
use crate::{GameWorld, PlayerDiedMessage, SandboxRuntime};

mod cli;
mod dev_runtime;
mod feedback;
mod hud;
mod input_systems;
mod phases;
mod plugins;
mod resources;
mod setup_systems;
mod update;
mod world_flow;

pub use cli::run_visible;
pub use feedback::{ProgressionResources, SandboxEventWriters, SandboxQueues};
pub use hud::update_quest_panel;
#[cfg(feature = "input")]
pub use input_systems::populate_control_frame_from_actions;
pub use plugins::{add_ldtk_runtime_plugin, add_presentation_plugins, add_simulation_plugins};
pub use resources::{init_sandbox_resources, StartRoomOverride};
pub use update::sandbox_update;
