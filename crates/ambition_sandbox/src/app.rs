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

/// Bundled `MessageWriter`s for the sim → presentation event channel.
///
/// `sandbox_update` outgrew Bevy's 16-system-param limit when individual
/// writers were passed; bundling them in a single `SystemParam` keeps the
/// sim system signature within budget while preserving the Vec-collector →
/// drain pattern documented in `docs/events_refactor_plan.md`. Adding new
/// channels to the sim → presentation seam happens here, not on the
/// `sandbox_update` signature.
#[derive(SystemParam)]
pub struct SandboxEventWriters<'w> {
    sfx: MessageWriter<'w, SfxMessage>,
    vfx: MessageWriter<'w, VfxMessage>,
    debris: MessageWriter<'w, DebrisBurstMessage>,
    died: MessageWriter<'w, PlayerDiedMessage>,
}

/// Mutable producer queues `sandbox_update` writes into during the
/// gameplay tick.
///
/// The encounter / feature pipeline is wide — switch presses, feature
/// events (door open, NPC death, breakable destroyed, …), and the
/// reset-feature messages all need `ResMut` access alongside everything
/// else `sandbox_update` already takes. Bundling them in a single
/// `SystemParam` keeps the system signature within Bevy's
/// 16-`SystemParam` budget (each `Res`/`ResMut` counts as one).
///
/// Producers (here):
/// - `switch_queue`: one switch activation enqueued per frame the
///   player toggles a Switch entity. Drained by
///   `crate::encounter::sync_encounter_controller_states`.
/// - `feature_bus`: aggregate of damage / heal / interaction events
///   produced by `crate::features::run_feature_logic`.
///
/// Add new sim → sim queues (NOT sim → presentation, which is
/// `SandboxEventWriters`) here when they grow naturally; resist the
/// urge to thread them through the system signature directly.
#[derive(SystemParam)]
pub struct SandboxQueues<'w> {
    pub switch_queue: ResMut<'w, crate::encounter::SwitchActivationQueue>,
    pub feature_bus: ResMut<'w, crate::features::FeatureEventBus>,
}

/// Read-only progression-state bundle for the HUD and pause menu.
///
/// Same `SystemParam`-packing trick as `SandboxQueues` — the HUD reads
/// from many independent registries (quests, cutscene state, bosses,
/// encounters, world map) and would otherwise blow the 16-param budget
/// when combined with windowing / camera / font handles. Grouping them
/// behind a single param both keeps the budget headroom and documents
/// the intentional read-only contract: HUD systems must not mutate
/// progression state. Mutators live in the producer side
/// (`sandbox_update`, `crate::quest`, `crate::boss_encounter`, etc.).
#[derive(SystemParam)]
pub struct ProgressionResources<'w> {
    pub quests: Res<'w, crate::quest::QuestRegistry>,
    pub cutscene: Res<'w, crate::cutscene::ActiveCutscene>,
    pub cutscene_request: Res<'w, crate::cutscene::CutsceneAdvanceRequest>,
    pub bosses: Res<'w, crate::boss_encounter::BossEncounterRegistry>,
    pub encounters: Res<'w, crate::encounter::EncounterRegistry>,
    pub map: Res<'w, crate::map_menu::MapMenuState>,
}

/// Per-frame Vec collectors for the sim → presentation event channels.
///
/// `sandbox_update` is the only producer; phase helpers append messages as
/// the gameplay loop runs and `flush_feedback` drains them into the
/// `MessageWriter`s at every return point. Keeping the collectors on a
/// single struct lets phase helpers take one parameter instead of three.
struct FrameFeedback {
    sfx: Vec<SfxMessage>,
    vfx: Vec<VfxMessage>,
    debris: Vec<DebrisBurstMessage>,
    died: Vec<PlayerDiedMessage>,
}

impl FrameFeedback {
    fn new() -> Self {
        Self {
            sfx: Vec::new(),
            vfx: Vec::new(),
            debris: Vec::new(),
            died: Vec::new(),
        }
    }
}

/// Local control-flow signal for `sandbox_update` phase helpers. `Return`
/// means the phase wants `sandbox_update` to flush feedback and stop the
/// frame here; `Continue` means proceed to the next phase.
#[must_use]
enum PhaseOutcome {
    Continue,
    Return,
}

/// Drain the per-frame `FrameFeedback` into the bundled `MessageWriter`s.
/// Call at every `sandbox_update` return point so audio/fx/debris
/// subscribers see the messages this frame.
fn flush_feedback(feedback: &mut FrameFeedback, writers: &mut SandboxEventWriters) {
    writers.sfx.write_batch(feedback.sfx.drain(..));
    writers.vfx.write_batch(feedback.vfx.drain(..));
    writers.debris.write_batch(feedback.debris.drain(..));
    writers.died.write_batch(feedback.died.drain(..));
}

/// True when no display server is reachable for `bevy_winit` to attach to.
/// Linux only — other platforms always return `false` and rely on Bevy's
/// own diagnostics. The check is conservative: any of `DISPLAY`,
/// `WAYLAND_DISPLAY`, or `WAYLAND_SOCKET` being set means we attempt the
/// visible path. If `--headless` was passed on the CLI, the caller has
/// already chosen the headless path and this check doesn't run.
fn no_display_server_available() -> bool {
    if cfg!(not(target_os = "linux")) {
        return false;
    }
    std::env::var_os("DISPLAY").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none()
        && std::env::var_os("WAYLAND_SOCKET").is_none()
}

fn cli_force_headless() -> bool {
    std::env::args().any(|arg| arg == "--headless")
}

fn cli_headless_ticks() -> u32 {
    let args: Vec<String> = std::env::args().collect();
    parse_headless_ticks(&args).unwrap_or(120)
}

fn parse_headless_ticks(args: &[String]) -> Option<u32> {
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--headless-ticks" => return args.get(i + 1).and_then(|raw| raw.parse().ok()),
            arg if arg.starts_with("--headless-ticks=") => {
                return arg
                    .trim_start_matches("--headless-ticks=")
                    .parse()
                    .ok();
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod headless_arg_tests {
    use super::parse_headless_ticks;

    fn args(slice: &[&str]) -> Vec<String> {
        slice.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_flag_returns_none() {
        assert_eq!(parse_headless_ticks(&args(&[])), None);
        assert_eq!(parse_headless_ticks(&args(&["--headless"])), None);
    }

    #[test]
    fn space_form() {
        assert_eq!(
            parse_headless_ticks(&args(&["--headless-ticks", "300"])),
            Some(300)
        );
    }

    #[test]
    fn equals_form() {
        assert_eq!(
            parse_headless_ticks(&args(&["--headless-ticks=42"])),
            Some(42)
        );
    }

    #[test]
    fn invalid_value_returns_none() {
        assert_eq!(
            parse_headless_ticks(&args(&["--headless-ticks", "abc"])),
            None
        );
    }
}

/// Build + run the visible Bevy app. The thin `fn main()` shim in
/// `src/main.rs` calls this.
///
/// Falls back to the headless simulation runner when no display server is
/// reachable (no `DISPLAY` / `WAYLAND_DISPLAY` on Linux), or when the
/// caller passes `--headless` on the CLI. The fallback path prints a
/// short diagnostic so users on a headless VM get a working
/// `cargo run` instead of a `bevy_winit` event-loop panic. Override the
/// number of ticks with `--headless-ticks N` (default 120).
pub fn run_visible() {
    if cli_force_headless() || no_display_server_available() {
        let max_ticks = cli_headless_ticks();
        let reason = if cli_force_headless() {
            "--headless flag"
        } else {
            "no DISPLAY / WAYLAND_DISPLAY env var"
        };
        eprintln!(
            "ambition_sandbox: running headless ({reason}); use `--bin headless` for the dedicated runner"
        );
        match crate::headless::run_headless(max_ticks) {
            Ok(report) => {
                println!("{report}");
                return;
            }
            Err(error) => {
                eprintln!("headless fallback failed: {error}");
                std::process::exit(1);
            }
        }
    }
    let asset_config = GameAssetConfig::from_args();
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Ambition - Tangent Space Sandbox (Bevy)".into(),
            resolution: WindowResolution::new(WINDOW_W, WINDOW_H),
            resizable: true,
            resize_constraints: WindowResizeConstraints {
                min_width: 640.0,
                min_height: 360.0,
                ..default()
            },
            ..default()
        }),
        ..default()
    }));
    // DefaultPlugins installs StatesPlugin, so initialize GameMode after it.
    app.init_state::<GameMode>();
    app.insert_resource(asset_config);
    init_sandbox_resources(&mut app);
    add_simulation_plugins(&mut app);
    add_ldtk_runtime_plugin(&mut app);
    add_presentation_plugins(&mut app);
    app.run();
}

/// Parse + validate the embedded LDtk world, build the `RoomSet`, and insert
/// the sim-required resources both visible and headless binaries need.
///
/// Both binaries call this after registering Bevy's plugin foundation
/// (DefaultPlugins or MinimalPlugins + AssetPlugin + StatesPlugin +
/// `init_state::<GameMode>`) and before the App-builder helpers.
///
/// Exits with status 2 on LDtk validation errors — invalid sandbox content
/// is a hard error per the LDtk authoring rules (see ADR 0009 + LDtk
/// authoring memory).
fn cli_start_room_arg() -> Option<String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    parse_start_room_arg(&args)
}

fn parse_start_room_arg(args: &[String]) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--start-room" | "--room" => {
                return args.get(i + 1).cloned();
            }
            arg if arg.starts_with("--start-room=") => {
                return Some(arg.trim_start_matches("--start-room=").to_string());
            }
            arg if arg.starts_with("--room=") => {
                return Some(arg.trim_start_matches("--room=").to_string());
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod cli_arg_tests {
    use super::parse_start_room_arg;

    fn args(slice: &[&str]) -> Vec<String> {
        slice.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_start_room_flag_returns_none() {
        assert_eq!(parse_start_room_arg(&args(&[])), None);
        assert_eq!(parse_start_room_arg(&args(&["--no-assets"])), None);
    }

    #[test]
    fn start_room_space_form() {
        assert_eq!(
            parse_start_room_arg(&args(&["--start-room", "mob_lab"])),
            Some("mob_lab".to_string())
        );
        assert_eq!(
            parse_start_room_arg(&args(&["--room", "central_hub_main"])),
            Some("central_hub_main".to_string())
        );
    }

    #[test]
    fn start_room_equals_form() {
        assert_eq!(
            parse_start_room_arg(&args(&["--start-room=water_world"])),
            Some("water_world".to_string())
        );
        assert_eq!(
            parse_start_room_arg(&args(&["--room=basement_boss"])),
            Some("basement_boss".to_string())
        );
    }

    #[test]
    fn start_room_first_match_wins() {
        // If both --start-room and --room are provided, the first one
        // in arg order wins. Bevy's own arg parsing leaves both alone.
        assert_eq!(
            parse_start_room_arg(&args(&["--room", "a", "--start-room", "b"])),
            Some("a".to_string())
        );
    }

    #[test]
    fn start_room_without_value_returns_none() {
        // Trailing flag with no value: don't crash, just return None.
        assert_eq!(parse_start_room_arg(&args(&["--start-room"])), None);
    }
}

/// Programmatic start-room override. SandboxSim and other library
/// callers insert this resource before `init_sandbox_resources` runs;
/// the function consumes it (taking precedence over the
/// `--start-room` CLI flag) so callers don't need to manipulate
/// `std::env::args` to pin a starting room.
#[derive(Resource, Clone, Debug)]
pub struct StartRoomOverride(pub String);

pub fn init_sandbox_resources(app: &mut App) {
    let sandbox_data = data::SandboxDataSpec::load_embedded();
    let ldtk_project = ldtk_world::LdtkProject::load_embedded();
    let ldtk_report = ldtk_project.validate();
    ldtk_report.print_to_stderr();
    let valid_track_ids = sandbox_data
        .audio
        .music_tracks
        .iter()
        .map(|t| t.id.as_str());
    for warning in ldtk_project.music_track_warnings(valid_track_ids) {
        eprintln!("LDtk validation warning: {warning}");
    }
    let editable_abilities = EditableAbilitySet::from(sandbox_data.abilities);
    let editable_tuning = EditableMovementTuning::from(sandbox_data.tuning);
    let mut room_set = match ldtk_project.to_room_set() {
        Ok(room_set) => room_set,
        Err(errors) => {
            eprintln!("embedded LDtk world failed validation; fix crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk before running:");
            for error in &errors {
                eprintln!("  - {error}");
            }
            std::process::exit(2);
        }
    };
    // Programmatic override (SandboxSim / library callers) takes
    // precedence over the CLI flag. Either one resolving by id wins;
    // the other is silently ignored. If neither matches, the LDtk
    // project's authored start room stays active.
    let resource_override = app
        .world_mut()
        .remove_resource::<StartRoomOverride>()
        .map(|r| r.0);
    if let Some(start_room) = resource_override.or_else(cli_start_room_arg) {
        if room_set.set_start_by_id(&start_room) {
            eprintln!("[ambition] start room: {start_room}");
        } else {
            eprintln!(
                "[ambition] warning: start-room '{start_room}' did not match any room id/name"
            );
        }
    }
    let ldtk_index = ldtk_world::LdtkRuntimeIndex::from_project(
        &ldtk_project,
        room_set.active_spec().id.clone(),
    );
    let active_world = room_set.active_world().clone();

    app.insert_resource(ldtk_world::SandboxLdtkProject(ldtk_project.clone()))
        .insert_resource(GameWorld(active_world))
        .insert_resource(rooms::ActiveRoomMetadata::default())
        .insert_resource(room_set)
        .insert_resource(ldtk_index)
        .insert_resource(ldtk_world::LdtkHotReloadState::from_current_file())
        .insert_resource(ldtk_world::LdtkRuntimeSpineStats::default())
        .insert_resource(ldtk_world::LdtkRuntimeSpineIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeSolidIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeOneWayIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeDamageIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeSpineParity::default())
        // PhysicsSandboxSettings is read by setup_simulation_system; on the
        // visible binary AmbitionPhysicsPlugin re-inserts the default value
        // (harmless — same default), but headless does not load that plugin
        // (it depends on ScenePlugin), so the resource must be available
        // up front.
        .insert_resource(physics::PhysicsSandboxSettings::default())
        .insert_resource(LdtkSettings {
            // Ambition still renders runtime rooms for now; let bevy_ecs_ldtk
            // own level/entity lifecycle without also drawing LDtk background
            // rectangles behind every level.
            level_background: LevelBackground::Nonexistent,
            // bevy_ecs_ldtk's default `IntGridRendering::Colorful` spawns a
            // colored tile sprite per non-zero IntGrid cell when no tileset
            // is configured (1004 sprites for central_hub_main alone). Those
            // tiles render in raw LDtk world-pixel coordinates from
            // `LdtkWorldBundle`'s default transform, while our compose path
            // (`int_grid_value_to_block` → `spawn_block`) renders in
            // Ambition's centered Bevy frame via `world_to_bevy`. The two
            // frames disagree by ~half-room-width on x, so the plugin's
            // IntGrid output appeared as a duplicated, horizontally-shifted
            // copy of our render. Force the plugin to emit no visual at all
            // for IntGrid cells; the runtime-spine `LdtkSolid` component
            // (our typed authority) is unaffected by this setting.
            int_grid_rendering: IntGridRendering::Invisible,
            ..default()
        })
        .insert_resource(sandbox_data)
        .insert_resource(DeveloperTools::default())
        .insert_resource(EditablePlayerStats::default())
        .insert_resource(SandboxFeelTuning::default())
        .insert_resource(editable_abilities)
        .insert_resource(editable_tuning)
        // Sim/presentation seam for input (ADR 0012): the sim reads
        // `Res<ControlFrame>`. Visible builds populate it from leafwing in
        // `populate_control_frame_from_actions`; headless tests can write
        // directly. Default = no buttons pressed = idle player.
        .init_resource::<ControlFrame>()
        // Aggregate user settings (video/audio/controls/gameplay).
        // Mutated by the pause menu; read by audio/video/gameplay
        // systems and the input deadzone/hysteresis filter.
        .insert_resource(crate::settings::UserSettings::default());
}

/// Register core simulation plugins, message types, and the gameplay
/// schedule. Headless and visible both call this. Bevy `LdtkPlugin` is
/// **not** included here (see `add_ldtk_runtime_plugin`) because its
/// tile-rendering pipeline requires the `RenderApp` sub-app, which only
/// exists when DefaultPlugins (or RenderPlugin) is installed. Without
/// LdtkPlugin the runtime-spine systems still run as no-ops; sandbox
/// gameplay drives the JSON-derived `GameWorld` collision world that
/// `init_sandbox_resources` already populated.
///
/// Caller is responsible for installing the appropriate Bevy plugin
/// foundation first (DefaultPlugins for visible, MinimalPlugins +
/// AssetPlugin + ImagePlugin + TransformPlugin + StatesPlugin for
/// headless) and calling `init_sandbox_resources` to populate the
/// LDtk-derived resources.
pub fn add_simulation_plugins(app: &mut App) {
    // AmbitionPhysicsPlugin (Avian2D) is intentionally NOT here. Per
    // ADR 0007 Avian is secondary physics for debris/ragdoll visuals;
    // the player controller is custom via parry2d in ambition_engine.
    // Avian's collider backend needs `SceneSpawner` (from ScenePlugin in
    // DefaultPlugins), which headless doesn't have. Until Avian's debris
    // role is migrated to presentation events end-to-end (or Avian gains
    // a headless-friendly init path), it lives in
    // `add_presentation_plugins`.
    app.add_message::<SfxMessage>()
        .add_message::<VfxMessage>()
        .add_message::<DebrisBurstMessage>()
        .add_message::<PlayerDiedMessage>()
        .register_type::<GameMode>()
        // StartupProfiler captures wall-clock at each marked phase so a
        // PostStartup report prints "where did the first frame's
        // worth of init time go" without needing an external profiler
        // attached. See `crate::profiling` for the helper API and
        // `docs/profiling.md` for Tracy / per-system profiling.
        .insert_resource(crate::profiling::StartupProfiler::default())
        .insert_resource(crate::trace::GameplayTraceBuffer::default())
        .insert_resource(crate::mechanics::MechanicsRegistry::default())
        .add_plugins(RonAssetPlugin::<data::SandboxDataSpec>::new(&["ron"]))
        .add_plugins(ae::AmbitionStateMachinePlugin)
        .add_systems(
            Startup,
            (
                crate::profiling::phase_mark("startup_begin"),
                data::load_data_asset_handle,
                crate::profiling::phase_mark("after_load_data_handle"),
                setup_simulation_system,
                crate::profiling::phase_mark("after_setup_simulation"),
            )
                .chain(),
        )
        // Final report. Runs once on the first PostStartup tick. The
        // pre-report mark captures the time between the last Startup
        // mark and PostStartup, so any heavy Startup systems we
        // didn't explicitly mark show up as a delta on the
        // "post_startup_begin" line.
        .add_systems(
            PostStartup,
            (
                crate::profiling::phase_mark("post_startup_begin"),
                crate::profiling::report_startup_phases,
            )
                .chain(),
        )
        .insert_resource(crate::projectile::PlayerProjectileState::default())
        // Encounter system. The legacy single-encounter `EncounterState`
        // resource stays for backwards-compat tests; the live
        // multi-encounter store is `EncounterRegistry`, populated
        // from LDtk by `populate_encounter_registry`.
        .insert_resource(crate::encounter::EncounterState::default())
        .insert_resource(crate::encounter::EncounterRegistry::default())
        .insert_resource(crate::encounter::SwitchActivationQueue::default())
        .insert_resource(crate::encounter::EncounterMusicRequest::default())
        .insert_resource(crate::rooms::RoomMusicRequest::default())
        // Sandbox save game (encounter defeat + switch state).
        // Loaded from disk by `load_save_at_startup` in the
        // presentation half so headless / RL drivers don't touch
        // disk; mutated by encounter + switch systems; written by
        // `autosave_sandbox_save` when change-detection fires.
        .insert_resource(crate::save::SandboxSave::default())
        // Quest + cutscene systems. Both are sim-side state machines
        // that read/write the save resource and surface HUD lines via
        // the encounter overlay.
        .insert_resource(crate::quest::QuestRegistry::default())
        .insert_resource(crate::cutscene::default_cutscene_library())
        .insert_resource(crate::cutscene::ActiveCutscene::default())
        .insert_resource(crate::cutscene::CutsceneTriggerQueue::default())
        .insert_resource(crate::cutscene::CutsceneAdvanceRequest::default())
        .insert_resource(crate::cutscene::RoomCutsceneBindings::defaults())
        .insert_resource(crate::boss_encounter::BossEncounterRegistry::default())
        .insert_resource(crate::features::FeatureEventBus::default())
        .insert_resource(crate::map_menu::MapMenuState::default())
        .insert_resource(crate::CameraEaseState::default())
        .insert_resource(crate::CameraEaseTuning::default())
        .insert_resource(crate::reset::SandboxResetRequested::default())
        .add_systems(
            Update,
            (
                ldtk_world::poll_ldtk_file_changes,
                sandbox_update,
                ldtk_world::sync_plugin_spawned_ambition_entities,
                ldtk_world::rebuild_ldtk_runtime_spine_index,
                ldtk_world::rebuild_ldtk_runtime_solid_index,
                ldtk_world::rebuild_ldtk_runtime_one_way_index,
                ldtk_world::rebuild_ldtk_runtime_damage_index,
                ldtk_world::check_ldtk_runtime_spine_parity,
                platforms::sync_moving_platform,
                crate::projectile::update_projectiles,
                crate::encounter::update_encounters_from_world,
                crate::encounter::sync_encounter_controller_states,
            )
                .chain(),
        )
        // Progression chain: cutscenes, boss encounters, quest events,
        // and the F3 stats editor sync. Split out from the main update
        // tuple so each chain stays under the macro tuple-arity limit.
        .add_systems(
            Update,
            (
                crate::cutscene::auto_trigger_room_cutscenes,
                crate::cutscene::drain_cutscene_triggers,
                crate::cutscene::tick_active_cutscene,
                crate::features::drain_feature_event_bus,
                crate::boss_encounter::update_boss_encounters,
                crate::features::sync_features_with_save,
                crate::quest::push_room_entered_quest_events,
                crate::quest::apply_quest_advance_events,
                crate::ledge_grab::update_ledge_grab,
                crate::body_mode::update_body_mode,
                crate::rooms::sync_active_room_metadata,
                crate::rooms::sync_room_music_request,
                crate::map_menu::track_room_visits,
                crate::map_menu::sync_map_from_save,
                dev_tools::sync_player_stats_with_inspector,
            )
                .chain()
                .after(crate::encounter::sync_encounter_controller_states),
        )
        // Populate the encounter / quest / boss registries from the LDtk
        // project + save. These run on Update (not Startup) with their
        // existing `specs_loaded` / `initialized` short-circuits so:
        //   1. The first Update tick populates them (Startup is done by
        //      the time any Update fires, so SandboxLdtkProject + save
        //      are ready).
        //   2. The "reset sandbox" flow (`process_sandbox_reset_request`)
        //      can flip those flags back to false and the next tick
        //      repopulates from the freshly-cleared save — without us
        //      having to inline the populate logic in two places.
        // The cost when already loaded is one ResMut acquisition + one
        // bool check per registry per frame: negligible.
        .add_systems(
            Update,
            (
                crate::quest::populate_quest_registry,
                crate::boss_encounter::populate_boss_encounter_registry,
                crate::encounter::populate_encounter_registry,
            ),
        )
        // Sandbox reset processor: consumes pending reset requests
        // (set by the pause-menu "Reset Sandbox" item or any other
        // caller). Runs after `sandbox_update` so it can't race with
        // in-flight gameplay mutations, and before the populate
        // systems on the next frame so they see the cleared
        // registries when re-running.
        .add_systems(
            Update,
            crate::reset::process_sandbox_reset_request.after(sandbox_update),
        )
        // Trace recorder lives at the simulation seam: `record_frame_system`
        // captures one frame per Update tick after `sandbox_update` has
        // resolved player state; `flush_pending_dump` writes any pending
        // dump to disk on the same tick. Both run on the simulation half
        // so headless and visible builds share trace output.
        .add_systems(
            Update,
            (
                crate::trace::record_frame_system.after(sandbox_update),
                crate::trace::flush_pending_dump.after(crate::trace::record_frame_system),
            ),
        );
}

/// Register Bevy's `LdtkPlugin` plus the supporting Ambition glue
/// (entity registrations, asset collection, LdtkWorldBundle spawn,
/// level-set sync, asset handle preload). Visible binary only —
/// `LdtkPlugin` panics in headless because its tile pipeline expects a
/// `RenderApp` sub-app, and `asset_server.load::<LdtkProject>` requires
/// the LDtk asset type to be registered.
///
/// Once the LDtk runtime-spine roadmap finishes promoting LDtk entity
/// categories to direct Ambition components (see
/// `project_ldtk_roadmap` memory), this dependency goes away and
/// headless can spawn the same entity set without bevy_ecs_ldtk's
/// rendering machinery.
pub fn add_ldtk_runtime_plugin(app: &mut App) {
    // `SandboxAssetCollection` includes a typed LDtk handle, so the LDtk
    // asset type and loader must be initialized before bevy_asset_loader
    // allocates collection handles. Keep this before `init_collection`.
    app.add_plugins(LdtkPlugin)
        .init_collection::<loading::SandboxAssetCollection>()
        .add_plugins(ldtk_world::AmbitionLdtkRegistrationPlugin)
        .add_systems(
            Startup,
            (
                ldtk_world::load_ldtk_asset_handle,
                spawn_ldtk_world_root.after(setup_simulation_system),
            ),
        )
        .add_systems(Update, ldtk_world::sync_ldtk_level_set);
}

/// Spawn the `LdtkWorldBundle` entity. Runs in `add_ldtk_runtime_plugin`
/// (visible binary only) after `setup_simulation_system` so the
/// `LdtkRuntimeIndex` resource is available.
fn spawn_ldtk_world_root(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ldtk_index: Res<ldtk_world::LdtkRuntimeIndex>,
    room_set: Res<rooms::RoomSet>,
    ldtk_asset: Option<Res<ldtk_world::SandboxLdtkAsset>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
) {
    let ldtk_handle = ldtk_asset
        .as_ref()
        .map(|asset| asset.0.clone())
        .or_else(|| {
            sandbox_asset_collection
                .as_ref()
                .map(|collection| collection.ldtk_project.clone())
        })
        .unwrap_or_else(|| asset_server.load(ldtk_world::SANDBOX_LDTK_ASSET));
    commands.spawn((
        bevy_ecs_ldtk::prelude::LdtkWorldBundle {
            ldtk_handle: ldtk_handle.into(),
            level_set: ldtk_index.level_set_for(&room_set.active_spec().id),
            // AMBITION_REVIEW(spatial): migrate each registered marker from
            // adapter-driven semantics to direct Ambition components.
            ..default()
        },
        ldtk_world::SandboxLdtkWorldRoot,
        Name::new("LDtk Runtime Spine Root"),
    ));
}

/// Register presentation-side plugins (input, dialogue, inspector, audio
/// and VFX subscribers, HUD, debug overlays). Visible binary only.
pub fn add_presentation_plugins(app: &mut App) {
    app.insert_resource(ClearColor(Color::srgb(0.020, 0.024, 0.035)))
        .insert_resource(windowing::DisplayModeState::default())
        .register_type::<DeveloperTools>()
        .register_type::<EditableAbilitySet>()
        .register_type::<EditableMovementTuning>()
        .register_type::<EditablePlayerStats>()
        .register_type::<SandboxFeelTuning>();

    add_dev_tools_plugins(app);
    add_physics_debris_plugins(app);
    add_ui_plugins(app);
    add_input_plugins(app);
    add_audio_plugins(app);
    add_mobile_touch_plugin(app);

    app.add_systems(Startup, ui_fonts::load_ui_fonts);

    // Settings + sandbox-save persistence. Both load on startup and
    // autosave when the relevant resource changes (`Res::is_changed`
    // throttle). Headless drivers do not register these systems, so
    // a `cargo run --bin headless` never reads or writes user files.
    app.add_systems(
        Startup,
        (
            crate::settings::persistence::load_settings_at_startup,
            crate::save::load_save_at_startup,
        ),
    )
    .add_systems(
        Update,
        (
            crate::settings::persistence::save_settings_on_change,
            crate::save::autosave_sandbox_save,
        ),
    );

    app.insert_resource(pause_menu::PauseMenuState::default())
        .insert_resource(inventory::InventoryUiState::default())
        .insert_resource(inventory::PlayerInventory::starter())
        .add_systems(
            Startup,
            (
                pause_menu::spawn_pause_menu,
                inventory::spawn_inventory_panel,
            )
                .after(setup_simulation_system),
        )
        .add_systems(
            Update,
            (
                pause_menu::sync_pause_menu,
                inventory::sync_inventory_panel,
                crate::map_menu::sync_map_menu,
            )
                .after(sandbox_update),
        )
        .add_systems(
            Startup,
            (
                crate::profiling::phase_mark("before_setup_presentation"),
                setup_presentation_system,
                crate::profiling::phase_mark("after_setup_presentation"),
                crate::map_menu::populate_map_rooms,
                crate::map_menu::spawn_map_menu,
                crate::profiling::phase_mark("after_map_menu_spawn"),
            )
                .chain()
                .after(setup_simulation_system)
                .after(ui_fonts::load_ui_fonts),
        )
        .add_systems(
            Update,
            (
                dialog::dialog_input,
                handle_ldtk_hot_reload,
                handle_debug_hotkeys,
                crate::trace::handle_trace_hotkey,
                crate::map_menu::handle_map_menu_hotkeys,
                // Spawn visual entities for encounter-spawned enemies
                // BEFORE sync_visuals reads positions for them.
                crate::rendering::spawn_dynamic_feature_visuals,
                sync_visuals,
                upgrade_enemy_sprites,
                upgrade_boss_sprites,
                animate_player,
                animate_enemies,
                animate_bosses,
                camera_follow,
                debug_overlay::draw_debug_overlay,
                fx::update_particles,
                fx::update_impacts,
                fx::update_slash_previews,
                windowing::window_mode_hotkeys,
                update_hud,
                dialog::sync_dialog_ui,
            )
                .chain()
                .after(sandbox_update),
        )
        .add_systems(
            Update,
            crate::rendering::sync_health_overlays.after(sync_visuals),
        )
        // Mouse / touch dismissal for the map menu — separate from the
        // big presentation tuple to keep that under Bevy's 16-system
        // tuple budget.
        .add_systems(Update, crate::map_menu::map_menu_pointer_dismiss)
        // Quest panel runs alongside the verbose HUD; placed in its
        // own `add_systems` so the main presentation tuple doesn't
        // overflow Bevy's 16-system tuple budget.
        .add_systems(Update, update_quest_panel.after(update_hud))
        // Procedural morph-ball visual: build the texture once at
        // startup, spawn the sibling sprite as soon as the player
        // entity exists, and toggle visibility / position each frame
        // based on `Player::body_mode`. After `sync_visuals` so the
        // player's transform is already mirrored when we read it.
        .add_systems(Startup, crate::body_mode::build_morph_ball_sprite)
        .add_systems(
            Update,
            (
                crate::body_mode::spawn_morph_ball_visual,
                crate::body_mode::sync_morph_ball_visual,
            )
                .chain()
                .after(sync_visuals),
        )
        // Player projectile visuals: rebuild the sprite ring each tick
        // from `PlayerProjectileState::bodies`. Lives in its own
        // `add_systems` because the main visible-only chain is at the
        // tuple-arity ceiling. Must run after `update_projectiles` so
        // the body list reflects this frame's spawn / tick / collision
        // before the visuals are rebuilt — otherwise newly-fired
        // projectiles would only become visible one frame late.
        .add_systems(
            Update,
            crate::projectile::sync_projectile_visuals.after(crate::projectile::update_projectiles),
        )
        // VFX + debris subscribe on the visible binary only. Audio's
        // subscriber lives in `add_audio_plugins` so the entire kira
        // chain stays behind the `audio` feature. Headless builds omit
        // these so the message queues drain without entity spawns or
        // audio playback.
        .add_systems(Update, vfx_spawn_messages.after(sandbox_update));
    // Live blink-destination preview ring. Reads leafwing action state to
    // know when the blink button is held, so it lives behind the `input`
    // feature alongside the other gameplay-input-driven presentation.
    #[cfg(feature = "input")]
    app.add_systems(Update, fx::update_blink_preview.after(sandbox_update));
}

/// Install the egui inspector plugins. Gated by the `dev_tools` feature so
/// shipping/headless builds don't pay for `bevy-inspector-egui` /
/// `bevy_egui` in the dep graph. The inspector quick plugins require
/// EguiPlugin first; that's why both live behind the same gate.
#[cfg(feature = "dev_tools")]
fn add_dev_tools_plugins(app: &mut App) {
    app.add_plugins(EguiPlugin::default())
        .add_plugins(
            ResourceInspectorPlugin::<DeveloperTools>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditableAbilitySet>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditableMovementTuning>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditablePlayerStats>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<SandboxFeelTuning>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(WorldInspectorPlugin::new().run_if(dev_tools::world_inspector_visible));
}

#[cfg(not(feature = "dev_tools"))]
fn add_dev_tools_plugins(_app: &mut App) {}

/// Install the Avian2D secondary-physics plugin and its presentation-side
/// debris subscriber. Gated by `physics_debris` so headless / minimal
/// builds drop `avian2d` from the dep graph entirely. Per ADR 0007, this
/// is secondary physics for debris/ragdoll visuals only — the player
/// controller stays kinematic.
#[cfg(feature = "physics_debris")]
fn add_physics_debris_plugins(app: &mut App) {
    app.add_plugins(physics::AmbitionPhysicsPlugin)
        .add_systems(Update, physics_spawn_debris_messages.after(sandbox_update));
}

#[cfg(not(feature = "physics_debris"))]
fn add_physics_debris_plugins(_app: &mut App) {}

/// Install UI-shell plugins: Yarn Spinner runtime and bevy_material_ui's
/// styling layer. The dialogue overlay (`dialog::sync_dialog_ui`) draws
/// with Bevy's core UI primitives and stays installed unconditionally;
/// only the optional plugins live behind `ui`.
#[cfg(feature = "ui")]
fn add_ui_plugins(app: &mut App) {
    app.add_plugins(dialog::yarn_spinner_plugin())
        .add_plugins(MaterialUiPlugin);
}

#[cfg(not(feature = "ui"))]
fn add_ui_plugins(_app: &mut App) {}

/// Install the leafwing-input-manager plugin, the player-input attach
/// startup system, and the bridge that keeps `Res<ControlFrame>` in sync
/// with leafwing's `ActionState`. Gated behind `input` so headless /
/// minimal builds can drop `leafwing-input-manager` from the dep graph;
/// the sim itself reads `Res<ControlFrame>` (always-available) and is
/// agnostic to where the frame came from.
#[cfg(feature = "input")]
fn add_input_plugins(app: &mut App) {
    app.init_resource::<MenuInputState>()
        .init_resource::<PlayerDashTriggerState>()
        .add_plugins(InputManagerPlugin::<SandboxAction>::default())
        .add_systems(
            Startup,
            attach_player_input_components.after(setup_simulation_system),
        )
        // Collect input into ControlFrame FIRST, then run menu / pause /
        // inventory toggles that consume it. Touch fold (mobile_input
        // plugin) runs `.after(populate_control_frame_from_actions)`
        // and `.before(pause_menu_toggle)`, so by the time pause /
        // inventory / navigate read ControlFrame, both keyboard and
        // touch have been merged.
        //
        // Per Jon 2026-05-07: "We need an elegant structure and
        // abstraction layer so different control methods are not
        // finicky." This reorder makes ControlFrame the canonical
        // input seam for menus/pause too -- previously they read
        // ActionState directly and missed touch input entirely.
        .add_systems(
            Update,
            (
                populate_control_frame_from_actions,
                pause_menu::pause_menu_toggle,
                inventory::inventory_input,
                pause_menu::pause_menu_pointer_input,
                inventory::inventory_pointer_input,
                pause_menu::pause_menu_navigate,
            )
                .chain()
                .before(sandbox_update),
        )
        .add_systems(Update, sync_preset_input_map.before(sandbox_update));
}

#[cfg(not(feature = "input"))]
fn add_input_plugins(_app: &mut App) {}

/// Register the mobile-touch input plugin (`virtual_joystick` sticks
/// + on-screen action buttons that fold into ControlFrame). Gated
/// behind the `mobile_touch` feature; on desktop builds without the
/// feature this is a no-op.
///
/// The mobile plugin runs ALONGSIDE the desktop input pipeline --
/// both write into the same `ControlFrame` resource, with the
/// mobile-side write happening after the desktop one in this
/// session's chain. On a phone, the desktop pipeline produces
/// neutral output (no keyboard / gamepad); on desktop, the mobile
/// stick UI is invisible without touch input, so neither path
/// stomps the other in practice. A future polish pass can detect
/// the active input source (touch vs keyboard) and skip the
/// inactive folder.
#[cfg(feature = "mobile_touch")]
fn add_mobile_touch_plugin(app: &mut App) {
    app.add_plugins(crate::mobile_input::bevy_plugin::MobileTouchPlugin);
}

#[cfg(not(feature = "mobile_touch"))]
fn add_mobile_touch_plugin(_app: &mut App) {}

/// Install the kira audio backend, channel resources, default music
/// startup, and the SFX subscriber. Gated by `audio` so headless / RL
/// builds drop `bevy_kira_audio` and `fundsp` from the dep graph
/// entirely. The sim still emits `SfxMessage`s; without this plugin the
/// message queue just drains harmlessly per the ADR 0012 seam.
#[cfg(feature = "audio")]
fn add_audio_plugins(app: &mut App) {
    app.add_plugins(KiraAudioPlugin)
        .add_audio_channel::<MusicChannel>()
        .add_audio_channel::<SfxChannel>()
        .add_audio_channel::<crate::music::MusicLayer0AChannel>()
        .add_audio_channel::<crate::music::MusicLayer1AChannel>()
        .add_audio_channel::<crate::music::MusicLayer2AChannel>()
        .add_audio_channel::<crate::music::MusicLayer3AChannel>()
        .add_audio_channel::<crate::music::MusicLayer4AChannel>()
        .add_audio_channel::<crate::music::MusicLayer5AChannel>()
        .add_audio_channel::<crate::music::MusicLayer0BChannel>()
        .add_audio_channel::<crate::music::MusicLayer1BChannel>()
        .add_audio_channel::<crate::music::MusicLayer2BChannel>()
        .add_audio_channel::<crate::music::MusicLayer3BChannel>()
        .add_audio_channel::<crate::music::MusicLayer4BChannel>()
        .add_audio_channel::<crate::music::MusicLayer5BChannel>()
        .add_systems(
            Startup,
            (
                crate::profiling::phase_mark("before_audio_init"),
                start_default_music,
                crate::music::load_music_cues,
                crate::profiling::phase_mark("after_audio_init"),
            )
                .chain()
                .after(setup_presentation_system),
        )
        .add_systems(Update, audio_play_sfx_messages.after(sandbox_update))
        // Push UserSettings.audio (master/music/sfx/mute) into the
        // Kira channels whenever the user changes the menu sliders.
        // Cheap; the system early-returns when settings are unchanged.
        .add_systems(Update, apply_audio_settings.after(sandbox_update))
        // Unified director: resolves room/encounter simple tracks and
        // adaptive cue states behind one music intent layer.
        .add_systems(
            Update,
            crate::music::drive_music_director.after(sandbox_update),
        );
}

#[cfg(not(feature = "audio"))]
fn add_audio_plugins(_app: &mut App) {}

/// Presentation-side companion to `setup_simulation_system`: attach
/// leafwing's `ActionState` and the active preset's `InputMap` to the
/// player entity. Sim-only setup spawns the player without these so the
/// sim path stays leafwing-free per the ADR 0012 input seam.
#[cfg(feature = "input")]
fn attach_player_input_components(
    mut commands: Commands,
    runtime: Res<SandboxRuntime>,
    scene: Res<crate::rendering::SceneEntities>,
) {
    let input_map = runtime.preset().input_map();
    commands
        .entity(scene.player)
        .insert((ActionState::<SandboxAction>::default(), input_map));
}

/// Bridge leafwing's `ActionState` into the sim-side `ControlFrame` resource.
///
/// This is the visible-binary half of the ADR 0012 input seam. The sim
/// reads `Res<ControlFrame>` only — it never queries `ActionState` —
/// which means headless / RL drivers can populate the resource directly
/// without an `InputManagerPlugin` in scope.
///
/// Dialogue mode also resets leafwing's pressed/just-pressed edges so
/// action edges from the moment dialogue opened don't leak into the
/// next gameplay frame.
#[cfg(feature = "input")]
pub fn populate_control_frame_from_actions(
    mode: Res<State<GameMode>>,
    mut player_input: Query<&mut ActionState<SandboxAction>, With<PlayerVisual>>,
    mut frame: ResMut<ControlFrame>,
    user_settings: Res<crate::settings::UserSettings>,
    mut dash_state: ResMut<PlayerDashTriggerState>,
    cutscene: Res<crate::cutscene::ActiveCutscene>,
    mut cutscene_request: ResMut<crate::cutscene::CutsceneAdvanceRequest>,
    time: Res<Time>,
) {
    if matches!(mode.get(), GameMode::Dialogue) {
        if let Ok(mut action_state) = player_input.single_mut() {
            action_state.reset_all();
        }
        *frame = ControlFrame::default();
        return;
    }
    // Cutscene takes precedence over gameplay input. We snapshot
    // interact_pressed into the dismiss request and zero out the
    // gameplay frame so movement / attack can't fire while a beat
    // plays. Holding `Reset` (Backspace/Delete/pad-Select) for
    // `SKIP_HOLD_THRESHOLD_SECS` requests a full cutscene skip so a
    // mistap can't burn through scripted content. Reset is chosen
    // (not Start) so the pause toggle still works during cutscenes
    // and a held button doesn't fight the existing
    // press-to-advance-dialogue mapping on Interact / Jump.
    if cutscene.is_playing() {
        if let Ok(action_state) = player_input.single() {
            let interact = action_state.pressed(&SandboxAction::Interact)
                || action_state.pressed(&SandboxAction::Jump);
            if interact {
                cutscene_request.dismiss_dialogue = true;
            }
            if action_state.pressed(&SandboxAction::Reset) {
                cutscene_request.skip_hold_seconds += time.delta_secs();
                if cutscene_request.skip_hold_seconds >= crate::cutscene::SKIP_HOLD_THRESHOLD_SECS {
                    cutscene_request.skip_cutscene = true;
                    cutscene_request.skip_hold_seconds = 0.0;
                }
            } else {
                cutscene_request.skip_hold_seconds = 0.0;
            }
        }
        *frame = ControlFrame::default();
        return;
    }
    // Outside cutscenes, decay the skip-hold counter so a stale
    // mid-cutscene press can't carry over.
    cutscene_request.skip_hold_seconds = 0.0;
    *frame = match player_input.single() {
        Ok(action_state) => {
            if mode.get().allows_gameplay() {
                let (next_frame, next_state) = ControlFrame::read_gameplay_with_settings(
                    action_state,
                    &user_settings.controls,
                    dash_state.edge,
                );
                dash_state.edge = next_state;
                next_frame
            } else {
                // While paused, suppress gameplay input AND reset the
                // dash trigger state so the post-pause re-press starts
                // from a clean Released edge.
                dash_state.edge = crate::settings::TriggerEdgeState::default();
                ControlFrame::read_menu(action_state)
            }
        }
        Err(_) => ControlFrame::default(),
    };
}

// `GameWorld`, `SandboxRuntime`, and the time-scale ramp helper `move_toward`
// have moved to `crate::lib` (`ambition_sandbox`) so both binaries can share
// them. They are re-imported above through `use ambition_sandbox::*;`.

/// Sim-only startup. Calls `crate::setup::simulation_world` to spawn the
/// LdtkWorldBundle, build the SandboxRuntime resource, and spawn the player
/// entity (with gameplay-essential components but no Sprite). Inserts
/// SceneEntities with `hud: Entity::PLACEHOLDER`; the presentation startup
/// system later overwrites that with the real HUD entity.
fn setup_simulation_system(
    mut commands: Commands,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data_asset: Option<Res<data::SandboxDataAsset>>,
    ldtk_asset: Option<Res<ldtk_world::SandboxLdtkAsset>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
    asset_server: Res<AssetServer>,
    ldtk_index: Res<ldtk_world::LdtkRuntimeIndex>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    sandbox_data: Res<data::SandboxDataSpec>,
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
) {
    let _player = setup::simulation_world(
        &mut commands,
        setup::SimulationSetup {
            world: &world,
            room_set: &room_set,
            ldtk_index: &ldtk_index,
            sandbox_data: &sandbox_data,
            editable_abilities: &editable_abilities,
            editable_tuning: &editable_tuning,
            physics_settings: *physics_settings,
            sandbox_data_asset: sandbox_data_asset.as_deref(),
            ldtk_asset: ldtk_asset.as_deref(),
            sandbox_asset_collection: sandbox_asset_collection.as_deref(),
            asset_server: &asset_server,
        },
    );
}

/// Presentation startup. Runs after `setup_simulation_system` so the
/// SceneEntities resource (with player Entity) is visible. Adds the
/// player's Sprite, spawns Camera2d, room visuals, HUD text, generated
/// Kira audio library, and overwrites SceneEntities to fill in the HUD
/// entity.
#[cfg(feature = "audio")]
fn setup_presentation_system(
    mut commands: Commands,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data: Res<data::SandboxDataSpec>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    scene_entities: Res<SceneEntities>,
    ui_fonts: Option<Res<ui_fonts::UiFonts>>,
    mut profiler: ResMut<crate::profiling::StartupProfiler>,
) {
    let t0 = std::time::Instant::now();
    let game_assets =
        game_assets::load_game_assets(&asset_config, &asset_server, &mut atlas_layouts);
    let t_assets = t0.elapsed().as_secs_f32() * 1000.0;
    profiler
        .marks
        .push(("setup_presentation::load_game_assets", std::time::Instant::now()));
    let t1 = std::time::Instant::now();
    setup::presentation_world(
        &mut commands,
        &mut audio_sources,
        &asset_server,
        setup::PresentationSetup {
            world: &world,
            room_set: &room_set,
            sandbox_data: &sandbox_data,
            physics_settings: *physics_settings,
            game_assets: &game_assets,
            ui_fonts: ui_fonts.as_deref(),
        },
        scene_entities.player,
    );
    let t_present = t1.elapsed().as_secs_f32() * 1000.0;
    eprintln!(
        "[startup]   setup_presentation breakdown: load_game_assets={t_assets:.1}ms presentation_world={t_present:.1}ms"
    );
    profiler.marks.push((
        "setup_presentation::presentation_world",
        std::time::Instant::now(),
    ));
    commands.insert_resource(game_assets);
}

#[cfg(not(feature = "audio"))]
fn setup_presentation_system(
    mut commands: Commands,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data: Res<data::SandboxDataSpec>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    scene_entities: Res<SceneEntities>,
) {
    let game_assets =
        game_assets::load_game_assets(&asset_config, &asset_server, &mut atlas_layouts);
    setup::presentation_world(
        &mut commands,
        setup::PresentationSetup {
            world: &world,
            room_set: &room_set,
            sandbox_data: &sandbox_data,
            physics_settings: *physics_settings,
            game_assets: &game_assets,
        },
        scene_entities.player,
    );
    commands.insert_resource(game_assets);
}

/// Bevy gameplay system that drives the sandbox simulation.
///
/// This is intentionally a thin orchestrator around named `*_phase`
/// helpers — the function body should make the gameplay frame order
/// readable in one screen so future agents can find the right phase by
/// grep without reading the whole loop.
///
/// The next likely refactor is promoting these phase helpers into
/// individually ordered Bevy systems / `SimSet`s once their behavior is
/// covered by tests. Until then, keep them as plain functions on a
/// shared `&mut SandboxRuntime` + `&mut FrameFeedback` so the borrow
/// graph stays linear.
///
/// Phase order (each phase comments its scope and what it should not own):
/// 1. `mode_gate_phase` — dialogue / pause / non-gameplay early returns.
/// 2. `input_timer_phase` — gameplay timer decay + double-tap detection.
/// 3. `reset_phase` — explicit reset input.
/// 4. `player_control_phase` — control-clock player update + pogo routing.
/// 5. `player_simulation_phase` — sim-clock player update + landing dust.
/// 6. `interaction_input_phase` — interact / double-tap-up + buffering.
/// 7. `feature_runtime_phase` — `runtime.features.update` + feedback.
/// 8. `damage_heal_dialogue_phase` — heals/damage/dialogue/feature reset.
/// 9. `room_transition_phase` — loading-zone transition + `load_room`.
/// 10. `attack_phase` — slash/pogo attack triggering.
/// 11. `cleanup_timers_phase` — flash/preset/slash animation timer decay.
/// 12. `flush_feedback` — drains `SfxMessage` / `VfxMessage` /
///     `DebrisBurstMessage` queues into the bundled writers.
pub fn sandbox_update(
    mut commands: Commands,
    time: Res<Time>,
    mut world: ResMut<GameWorld>,
    mut room_set: ResMut<rooms::RoomSet>,
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    feel_tuning: Res<SandboxFeelTuning>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut runtime: ResMut<SandboxRuntime>,
    mut event_writers: SandboxEventWriters,
    control_frame: Res<ControlFrame>,
    user_settings: Res<crate::settings::UserSettings>,
    mut queues: SandboxQueues,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    game_assets: Option<Res<crate::game_assets::GameAssets>>,
) {
    let switch_queue = &mut queues.switch_queue;
    let feature_bus = &mut queues.feature_bus;
    let mut feedback = FrameFeedback::new();
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;
    let physics_settings = runtime.physics_settings;
    // Compose difficulty + assist + the fine-grained menu multiplier
    // into one scalar that `handle_player_damage_events` consults.
    // Assist mode halves incoming damage on top of difficulty so a
    // user who needs the extra help can stack the two.
    let assist_factor = match user_settings.gameplay.assist {
        crate::settings::AssistMode::Off => 1.0,
        crate::settings::AssistMode::On => 0.5,
    };
    let difficulty_multiplier = user_settings.gameplay.difficulty.damage_taken_multiplier()
        * user_settings.gameplay.player_damage_multiplier
        * assist_factor;
    dev_tools::sync_live_ability_edits(&mut runtime, editable_abilities.as_engine(), tuning);

    // sandbox_update no longer queries leafwing directly. Input arrives
    // through `Res<ControlFrame>` — visible builds derive it from
    // ActionState in `populate_control_frame_from_actions` (runs
    // `.before(sandbox_update)`); headless / RL drivers can write the
    // resource directly. Debug hotkeys live in their own presentation-side
    // system, also `.before(sandbox_update)`. Local mutable copy because
    // `interaction_input_phase` rewrites `controls.interact_pressed` via
    // the input buffer (runtime state, not raw input).
    let mut controls = *control_frame;
    let frame_dt = time.delta_secs();

    if matches!(
        mode_gate_phase(mode.get(), &mut runtime, frame_dt),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    // Pause/resume toggling has moved to `pause_menu::pause_menu_toggle`,
    // which runs `.before(sandbox_update)`. The `start_pressed` flag is
    // still read here for compile-completeness; the pause logic itself
    // lives in the pause menu so it can drive a real overlay.
    let _ = controls.start_pressed;

    let door_double_tap_up = input_timer_phase(&mut controls, &mut runtime, feel, frame_dt);

    if matches!(
        reset_phase(
            &controls,
            &world.0,
            &mut runtime,
            &mut feedback,
            tuning,
            feel
        ),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    if matches!(
        player_control_phase(
            controls,
            &world.0,
            &mut runtime,
            &mut feedback,
            tuning,
            feel,
            frame_dt,
        ),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    if matches!(
        player_simulation_phase(
            controls,
            &world.0,
            &mut runtime,
            &mut feedback,
            tuning,
            feel,
            frame_dt,
        ),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    interaction_input_phase(
        &mut controls,
        &mut runtime,
        feel,
        door_double_tap_up,
        frame_dt,
    );

    let feature_events = feature_runtime_phase(
        &controls,
        &world.0,
        &mut runtime,
        &mut feedback,
        feel,
        frame_dt,
    );

    // Drain switch activations into the encounter system's queue.
    // The encounter `update_encounters_from_world` system reads it
    // after `sandbox_update` fires.
    for payload in &feature_events.switch_activations {
        if let Some(activation) = crate::encounter::SwitchActivation::parse_custom(payload) {
            switch_queue.0.push(activation);
        }
    }
    // Forward boss-damage / quest / flag events to downstream
    // systems via the bus. Drained next frame by
    // `drain_feature_event_bus`.
    feature_bus.ingest(&feature_events);

    if matches!(
        damage_heal_dialogue_phase(
            &world.0,
            &mut runtime,
            &mut feedback,
            &feature_events,
            &mut next_mode,
            tuning,
            feel,
            difficulty_multiplier,
        ),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    if matches!(
        room_transition_phase(
            &mut commands,
            &controls,
            &mut world,
            &mut room_set,
            &mut runtime,
            &mut feedback,
            &room_visuals,
            tuning,
            feel,
            physics_settings,
            game_assets.as_deref(),
        ),
        PhaseOutcome::Return
    ) {
        flush_feedback(&mut feedback, &mut event_writers);
        return;
    }

    attack_phase(&controls, &mut runtime, &mut feedback, tuning, feel);

    cleanup_timers_phase(&mut runtime, frame_dt);

    flush_feedback(&mut feedback, &mut event_writers);
}

/// Phase 1 — dialogue / pause / non-gameplay early returns.
///
/// Owns: zeroing `time_scale`, decaying `flash_timer` + `preset_flash` in
/// modes that intentionally suspend gameplay.
///
/// Should not own: gameplay input edits, movement, combat, or room
/// transitions. New "in dialogue / paused / cutscene" timer decay
/// belongs here; new gameplay logic does not.
fn mode_gate_phase(mode: &GameMode, runtime: &mut SandboxRuntime, frame_dt: f32) -> PhaseOutcome {
    if matches!(mode, GameMode::Dialogue) {
        runtime.time_scale = 0.0;
        runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
        runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
        return PhaseOutcome::Return;
    }
    if !mode.allows_gameplay() {
        // Pause, dialogue, and transition modes intentionally do not consume
        // gameplay inputs or advance simulation timers. Developer hotkeys
        // and HUD sync remain responsive because those systems are outside
        // this early return.
        runtime.time_scale = 0.0;
        runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
        runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
        return PhaseOutcome::Return;
    }
    PhaseOutcome::Continue
}

/// Phase 2 — gameplay timer decay + semantic input tweaks.
///
/// Owns: per-frame decay of `room_transition_cooldown`,
/// `damage_invuln_timer`, `hitstun_timer`, `hitstop_timer`; rewriting
/// `controls.fast_fall_pressed` from a down double-tap; producing the
/// `door_double_tap_up` signal returned to the caller.
///
/// Should not own: movement, combat, feature runtime updates. New
/// gameplay-only timers and new input-edge gestures belong here. Returns
/// the door / NPC double-tap-up signal so `interaction_input_phase` can
/// fold it in alongside the explicit `Interact` action.
fn input_timer_phase(
    controls: &mut ControlFrame,
    runtime: &mut SandboxRuntime,
    feel: SandboxFeelTuning,
    frame_dt: f32,
) -> bool {
    runtime.room_transition_cooldown = (runtime.room_transition_cooldown - frame_dt).max(0.0);
    runtime.damage_invuln_timer = (runtime.damage_invuln_timer - frame_dt).max(0.0);
    runtime.hitstun_timer = (runtime.hitstun_timer - frame_dt).max(0.0);
    let double_tap_down =
        runtime.register_down_tap(controls.down_pressed, frame_dt, feel.down_double_tap_window);
    controls.fast_fall_pressed = double_tap_down;
    // Re-route the double-tap-down edge through SandboxRuntime so the
    // body-mode driver in the progression chain (after sandbox_update)
    // can read it. The local `controls` mutation here doesn't reach
    // post-update systems because sandbox_update consumes a copy of the
    // resource; engine-side fast-fall is consumed inline so the local
    // copy is fine for that, but morph-ball entry needs the edge to
    // survive past `sandbox_update`'s scope.
    if double_tap_down {
        runtime.double_tap_down_pending = true;
    }
    let door_double_tap_up =
        runtime.register_up_tap(controls.up_pressed, frame_dt, feel.up_double_tap_window);
    runtime.hitstop_timer = (runtime.hitstop_timer - frame_dt).max(0.0);
    door_double_tap_up
}

/// Phase 3 — explicit reset input.
///
/// Owns: routing the `reset_pressed` button through `reset_sandbox`. New
/// "the player asked for a reset / restart" branches belong here; engine
/// or feature-driven resets stay in `player_control_phase`,
/// `player_simulation_phase`, or `damage_heal_dialogue_phase`.
fn reset_phase(
    controls: &ControlFrame,
    world: &ae::World,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) -> PhaseOutcome {
    if controls.reset_pressed {
        reset_sandbox(
            world,
            &mut feedback.sfx,
            &mut feedback.vfx,
            runtime,
            tuning,
            feel,
        );
        return PhaseOutcome::Return;
    }
    PhaseOutcome::Continue
}

/// Phase 4 — control-clock half of the two-clock player update.
///
/// Owns: hitstun-filtered control snapshot, real-time `frame_dt`
/// `update_player_control_with_tuning` call, pogo-bounce → feature-event
/// routing, `handle_player_events` for the control-clock pass.
///
/// Should not own: gravity/platform/AI ticks (those run on `sim_dt` in
/// `player_simulation_phase`). New responsive-input mechanics that need
/// real time (jump buffers, blink aim, dash chains) belong here. Returns
/// `Return` if the engine asked for a sandbox reset.
fn player_control_phase(
    controls: ControlFrame,
    world: &ae::World,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
) -> PhaseOutcome {
    // Two-clock update:
    // - control_dt is real time for responsive inputs and precision-blink aim;
    // - sim_dt is scaled game time for gravity, platforms, enemies, particles.
    let filtered = controls_for_hitstun(controls, feel, runtime.hitstun_timer);
    let input = filtered.engine_input(frame_dt);
    let control_world =
        features::world_with_sandbox_solids(world, &runtime.moving_platform, &runtime.features);
    let control_events = ae::update_player_control_with_tuning(
        &control_world,
        &mut runtime.player,
        input,
        frame_dt,
        tuning,
    );
    if control_events.reset {
        reset_sandbox(
            world,
            &mut feedback.sfx,
            &mut feedback.vfx,
            runtime,
            tuning,
            feel,
        );
        return PhaseOutcome::Return;
    }
    // Damage breakable pogo orbs the player just bounced off. The
    // engine reports orb AABBs; the sandbox matches them against
    // breakables flagged `pogo_refresh` and routes hit/break events
    // through the standard feature pipeline.
    for &orb_aabb in &control_events.pogo_hits {
        let feature_events = runtime.features.on_pogo_bounce(orb_aabb, 1);
        handle_feature_events(
            &mut feedback.sfx,
            &mut feedback.vfx,
            &mut feedback.debris,
            &feature_events,
            runtime.player.pos,
        );
    }
    handle_player_events(
        &mut feedback.sfx,
        &mut feedback.vfx,
        runtime,
        control_events,
        None,
    );
    PhaseOutcome::Continue
}

/// Phase 5 — sim-clock half of the two-clock player update.
///
/// Owns: `update_time_scale` (hitstop / bullet-time / slowmo ramp),
/// scaled `sim_dt`, moving-platform tick + ride-along, sandbox-side
/// solid rebuild, `update_player_simulation_with_tuning`, landing-dust
/// feedback through `handle_player_events`.
///
/// Should not own: feature-runtime ticks or interact-buffering. New
/// game-time-affected motion (gravity tweaks, platform AI, knockback
/// resolution) belongs here. Returns `Return` if simulation asked for a
/// sandbox reset.
fn player_simulation_phase(
    controls: ControlFrame,
    world: &ae::World,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
) -> PhaseOutcome {
    let filtered = controls_for_hitstun(controls, feel, runtime.hitstun_timer);
    let input = filtered.engine_input(frame_dt);

    runtime.update_time_scale(frame_dt, feel);
    let sim_dt = sandbox_dt(runtime, frame_dt);

    let platform_delta = runtime.moving_platform.update(sim_dt);
    let riding_now = runtime.moving_platform.is_riding(&runtime.player);
    let was_riding_platform = runtime.player.was_riding_platform;
    if riding_now != was_riding_platform {
        // Diagnostic: log riding-state transitions. Useful for
        // chasing the "intermittent glitchy platform behavior" repro
        // (TODO S) — once a player reports a glitch, the platform
        // contact transitions in the log immediately before are the
        // first place to look.
        debug!(
            target: "ambition::platform",
            riding = riding_now,
            player_pos = ?runtime.player.pos,
            player_vel = ?runtime.player.vel,
            on_ground = runtime.player.on_ground,
            platform_pos = ?runtime.moving_platform.pos,
            platform_dir = runtime.moving_platform.direction(),
            "moving-platform riding transition"
        );
    }
    runtime.player.was_riding_platform = riding_now;
    if riding_now {
        runtime.player.pos += platform_delta;
    }
    let collision_world =
        features::world_with_sandbox_solids(world, &runtime.moving_platform, &runtime.features);

    let was_grounded = runtime.player.on_ground;
    let sim_events = ae::update_player_simulation_with_tuning(
        &collision_world,
        &mut runtime.player,
        input,
        sim_dt,
        tuning,
    );
    if sim_events.reset {
        reset_sandbox(
            world,
            &mut feedback.sfx,
            &mut feedback.vfx,
            runtime,
            tuning,
            feel,
        );
        return PhaseOutcome::Return;
    }
    handle_player_events(
        &mut feedback.sfx,
        &mut feedback.vfx,
        runtime,
        sim_events,
        Some(was_grounded),
    );
    PhaseOutcome::Continue
}

/// Phase 6 — interact / double-tap-up + buffering.
///
/// Owns: hitstun gating of interaction, folding the explicit `Interact`
/// action together with the `door_double_tap_up` signal from
/// `input_timer_phase`, writing the buffered result back into
/// `controls.interact_pressed` via `runtime.buffered_interact`.
///
/// Should not own: actually triggering doors, NPCs, chests, or pickups —
/// `feature_runtime_phase` and `room_transition_phase` consume the
/// buffered signal. Up is too valuable for platforming/flight/aiming to
/// double as a one-tap door or NPC trigger, so doors/NPCs/chests accept
/// either the dedicated `Interact` action or a deliberate double-tap-up
/// gesture.
fn interaction_input_phase(
    controls: &mut ControlFrame,
    runtime: &mut SandboxRuntime,
    feel: SandboxFeelTuning,
    door_double_tap_up: bool,
    frame_dt: f32,
) {
    let raw_interact_pressed = if runtime.hitstun_timer > 0.0 {
        false
    } else {
        controls.interact_pressed || door_double_tap_up
    };
    controls.interact_pressed =
        runtime.buffered_interact(raw_interact_pressed, frame_dt, feel.interaction_buffer_time);
}

/// Phase 7 — feature runtime tick.
///
/// Owns: per-frame `runtime.features.update` call for hazards, enemies,
/// bosses, breakables, pickups, chests, and NPCs; routing the resulting
/// audio/vfx/debris cues through `handle_feature_events`.
///
/// Should not own: applying the resulting damage / heals / dialogue /
/// reset flags — those are intentionally split into
/// `damage_heal_dialogue_phase` so the side-effect surface is grep-able
/// in one place. Returns the raw `FeatureEvents` so the next phase can
/// consume them.
fn feature_runtime_phase(
    controls: &ControlFrame,
    world: &ae::World,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    feel: SandboxFeelTuning,
    frame_dt: f32,
) -> features::FeatureEvents {
    let feature_dt = sandbox_dt(runtime, frame_dt);
    let feature_world =
        features::world_with_sandbox_solids(world, &runtime.moving_platform, &runtime.features);
    let feature_player = runtime.player.clone();
    // Invincibility short-circuits at the emit site too: otherwise
    // standing in a hazard while the F3 toggle is on would re-emit a
    // damage event (and its impact / message side effects) every frame
    // — the handler drops the event, but the impacts still spawn
    // particles and SFX.
    let player_vulnerable = !runtime.player.invincible && runtime.damage_invuln_timer <= 0.0;
    let feature_events = runtime.features.update(
        &feature_world,
        &feature_player,
        controls.interact_pressed,
        player_vulnerable,
        feel.feature_combat_tuning(),
        feature_dt,
    );
    handle_feature_events(
        &mut feedback.sfx,
        &mut feedback.vfx,
        &mut feedback.debris,
        &feature_events,
        runtime.player.pos,
    );
    feature_events
}

/// Phase 8 — apply heals/damage, dialogue start, feature-driven reset.
///
/// Owns: `handle_player_heal_events`, `handle_player_damage_events`,
/// `remember_safe_player_position` when the player wasn't damaged this
/// frame, clearing the interact buffer when a feature consumed it,
/// starting `GameMode::Dialogue` on a feature-issued dialogue request,
/// routing feature-driven reset through `reset_sandbox`.
///
/// Should not own: the feature tick itself (that's
/// `feature_runtime_phase`) or attack / room-transition routing. Returns
/// `Return` if dialogue started or the feature requested a sandbox
/// reset.
fn damage_heal_dialogue_phase(
    world: &ae::World,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    feature_events: &features::FeatureEvents,
    next_mode: &mut NextState<GameMode>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    difficulty_multiplier: f32,
) -> PhaseOutcome {
    let feature_damaged_player = !feature_events.player_damage.is_empty();
    let feature_interaction_consumed = feature_events.consumed_interaction;
    handle_player_heal_events(runtime, feature_events);
    handle_player_damage_events(
        world,
        &mut feedback.sfx,
        &mut feedback.vfx,
        &mut feedback.died,
        runtime,
        feature_events,
        tuning,
        feel,
        difficulty_multiplier,
    );
    {
        let safe_world =
            features::world_with_sandbox_solids(world, &runtime.moving_platform, &runtime.features);
        let ctx = crate::SafePositionContext {
            damaged_this_frame: feature_damaged_player,
            in_hitstun: runtime.hitstun_timer > 0.0,
            feature_requested_reset: feature_events.reset_player,
            blink_grace_active: runtime.player.blink_grace_timer > 0.0,
            room_transitioning: runtime.room_transition_cooldown > 0.0,
        };
        runtime.remember_safe_player_position(&safe_world, ctx);
    }
    if feature_interaction_consumed {
        runtime.clear_interact_buffer();
    }
    if let Some(request) = &feature_events.dialogue_request {
        runtime
            .dialogue
            .start(&request.dialogue_id, &request.npc_name);
        runtime.clear_interact_buffer();
        runtime.hitstop_timer = 0.0;
        next_mode.set(GameMode::Dialogue);
        return PhaseOutcome::Return;
    }
    if feature_events.reset_player {
        reset_sandbox(
            world,
            &mut feedback.sfx,
            &mut feedback.vfx,
            runtime,
            tuning,
            feel,
        );
        return PhaseOutcome::Return;
    }
    PhaseOutcome::Continue
}

/// Phase 9 — loading-zone transition + `load_room`.
///
/// Owns: cooldown gate, `room_set.transition_for_player` query against
/// the buffered interact signal, clearing the interact buffer on a
/// matched transition, calling `load_room` for the actual swap.
///
/// Should not own: which buttons trigger a transition (that's
/// `interaction_input_phase`) or per-zone content rebuild (that's
/// `load_room`). Returns `Return` if a transition fired this frame.
fn room_transition_phase(
    commands: &mut Commands,
    controls: &ControlFrame,
    world: &mut GameWorld,
    room_set: &mut rooms::RoomSet,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    game_assets: Option<&crate::game_assets::GameAssets>,
) -> PhaseOutcome {
    if runtime.room_transition_cooldown > 0.0 {
        return PhaseOutcome::Continue;
    }
    let Some(zone) = room_set.transition_for_player(&runtime.player, controls.interact_pressed)
    else {
        return PhaseOutcome::Continue;
    };
    runtime.clear_interact_buffer();
    load_room(
        commands,
        &mut feedback.sfx,
        &mut feedback.vfx,
        runtime,
        world,
        room_set,
        room_visuals,
        zone,
        tuning,
        feel,
        physics_settings,
        game_assets,
    );
    PhaseOutcome::Return
}

/// Phase 10 — slash / pogo attack triggering.
///
/// Owns: hitstun gate, attack/pogo button check, dispatching to
/// `process_attack` (which itself emits sfx/vfx/debris and runs the
/// feature-side hit application).
///
/// Should not own: damage application semantics — those live in
/// `process_attack` and the engine. New attack archetypes should add
/// branches here only when the trigger condition differs.
fn attack_phase(
    controls: &ControlFrame,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    if runtime.hitstun_timer <= 0.0 && (controls.attack_pressed || controls.pogo_pressed) {
        process_attack(
            &mut feedback.sfx,
            &mut feedback.vfx,
            &mut feedback.debris,
            runtime,
            *controls,
            tuning,
            feel,
        );
    }
}

/// Phase 11 — flash / preset / slash animation timer decay.
///
/// Owns: real-time decay of `flash_timer`, `preset_flash`,
/// `slash_anim_timer`. New presentation-flash timers belong here;
/// gameplay timers belong in `input_timer_phase`.
fn cleanup_timers_phase(runtime: &mut SandboxRuntime, frame_dt: f32) {
    runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
    runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
    runtime.slash_anim_timer = (runtime.slash_anim_timer - frame_dt).max(0.0);
}

/// Presentation-side debug hotkey reader.
///
/// Slice 5 of the events refactor moved this out of `sandbox_update` so the
/// gameplay loop no longer reads `Res<ButtonInput<KeyCode>>`. That lets
/// `sandbox_update` run on the headless App-builder track (where
/// `InputPlugin` is absent and `ButtonInput<KeyCode>` therefore does not
/// exist as a resource).
///
/// Runs `.before(sandbox_update)` so preset / debug-flag mutations land
/// before the gameplay loop reads them this frame. Updates the player's
/// `InputMap` directly when the preset cycles, which is the same effect
/// the old in-`sandbox_update` `preset_changed` branch had.
fn handle_debug_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut runtime: ResMut<SandboxRuntime>,
    mut tools: ResMut<DeveloperTools>,
) {
    if keys.just_pressed(KeyCode::F1) {
        runtime.debug = !runtime.debug;
    }
    if keys.just_pressed(KeyCode::F9) {
        runtime.preset_index =
            (runtime.preset_index + runtime.presets.len() - 1) % runtime.presets.len();
        runtime.preset_flash = 1.2;
    }
    if keys.just_pressed(KeyCode::F10) {
        runtime.preset_index = (runtime.preset_index + 1) % runtime.presets.len();
        runtime.preset_flash = 1.2;
    }
    if keys.just_pressed(KeyCode::F2) {
        runtime.slowmo = !runtime.slowmo;
    }
    if keys.just_pressed(KeyCode::F3) {
        tools.inspector_visible = !tools.inspector_visible;
    }
    if keys.just_pressed(KeyCode::F4) {
        tools.world_inspector_visible = !tools.world_inspector_visible;
    }
    if keys.just_pressed(KeyCode::F5) {
        tools.overview_camera = !tools.overview_camera;
    }
}

/// When the player cycles input presets via F9/F10, sync leafwing's
/// `InputMap` on the player entity so the next-frame inputs reflect the
/// new preset. Detected by polling `runtime.preset_index`. Gated behind
/// `input` because it owns leafwing components.
#[cfg(feature = "input")]
fn sync_preset_input_map(
    runtime: Res<SandboxRuntime>,
    mut last_preset: Local<Option<usize>>,
    entities: Res<SceneEntities>,
    mut player_input: Query<
        (
            &mut ActionState<SandboxAction>,
            &mut InputMap<SandboxAction>,
        ),
        With<PlayerVisual>,
    >,
) {
    let current = runtime.preset_index;
    if *last_preset == Some(current) {
        return;
    }
    if let Ok((mut action_state, mut input_map)) = player_input.get_mut(entities.player) {
        *input_map = runtime.preset().input_map();
        action_state.reset_all();
    }
    *last_preset = Some(current);
}

fn handle_ldtk_hot_reload(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut world: ResMut<GameWorld>,
    mut room_set: ResMut<rooms::RoomSet>,
    mut runtime: ResMut<SandboxRuntime>,
    mut ldtk_index: ResMut<ldtk_world::LdtkRuntimeIndex>,
    mut ldtk_reload: ResMut<ldtk_world::LdtkHotReloadState>,
    editable_tuning: Res<EditableMovementTuning>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    game_assets: Option<Res<crate::game_assets::GameAssets>>,
) {
    if keys.just_pressed(KeyCode::F12) {
        ldtk_reload.auto_apply = !ldtk_reload.auto_apply;
        ldtk_reload.last_status = format!(
            "LDtk auto-apply {}",
            if ldtk_reload.auto_apply {
                "enabled"
            } else {
                "disabled"
            }
        );
    }

    let requested = keys.just_pressed(KeyCode::F11);
    let should_apply = requested || (ldtk_reload.pending && ldtk_reload.auto_apply);
    if !should_apply {
        return;
    }

    match reload_ldtk_world_from_disk(
        &mut commands,
        &mut world,
        &mut room_set,
        &mut runtime,
        &mut ldtk_index,
        editable_tuning.as_engine(),
        &room_visuals,
        game_assets.as_deref(),
    ) {
        Ok(active_room) => {
            ldtk_reload.mark_applied(&active_room);
            eprintln!("LDtk hot reload applied to active room '{active_room}'");
        }
        Err(errors) => {
            for error in &errors {
                eprintln!("LDtk hot reload rejected: {error}");
            }
            ldtk_reload.mark_failed(errors);
        }
    }
}

struct LdtkReloadTransaction {
    project: ldtk_world::LdtkProject,
    next_room_set: rooms::RoomSet,
    next_spec: rooms::RoomSpec,
    safe_player_pos: ae::Vec2,
}

fn prepare_ldtk_reload_transaction(
    current_room_id: &str,
    preserved_pos: ae::Vec2,
    player_size: ae::Vec2,
) -> Result<LdtkReloadTransaction, Vec<String>> {
    let project = ldtk_world::LdtkProject::load_from_disk().map_err(|error| vec![error])?;
    let report = project.validate();
    report.print_to_stderr();
    if !report.is_ok() {
        return Err(report.errors);
    }

    let mut next_room_set = project.to_room_set()?;
    let Some(next_active) = next_room_set
        .rooms
        .iter()
        .position(|room| room.id == current_room_id)
    else {
        return Err(vec![format!(
            "LDtk reload would delete current active area '{current_room_id}'. Move the player elsewhere or restore that activeArea before applying."
        )]);
    };
    next_room_set.active = next_active;
    let next_spec = next_room_set.active_spec().clone();

    let mut hard_errors = Vec::new();
    for warning in next_room_set.layout_warnings() {
        if warning.contains("references missing") {
            hard_errors.push(format!("LDtk reload graph error: {warning}"));
        } else {
            eprintln!("LDtk reload layout warning: {warning}");
        }
    }
    if !hard_errors.is_empty() {
        return Err(hard_errors);
    }

    let safe_player_pos = rooms::validated_spawn(&next_spec.world, preserved_pos, player_size);
    Ok(LdtkReloadTransaction {
        project,
        next_room_set,
        next_spec,
        safe_player_pos,
    })
}

fn reload_ldtk_world_from_disk(
    commands: &mut Commands,
    world: &mut GameWorld,
    room_set: &mut rooms::RoomSet,
    runtime: &mut SandboxRuntime,
    ldtk_index: &mut ldtk_world::LdtkRuntimeIndex,
    tuning: ae::MovementTuning,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    assets: Option<&crate::game_assets::GameAssets>,
) -> Result<String, Vec<String>> {
    let current_room_id = room_set.active_spec().id.clone();
    let preserved_pos = runtime.player.pos;
    let transaction =
        prepare_ldtk_reload_transaction(&current_room_id, preserved_pos, runtime.player.size)?;

    // Everything above this line is non-mutating: invalid edits, deleted active
    // areas, bad graph links, and unsafe player positions are rejected before
    // touching the live world. Only commit after the complete replacement room
    // graph and repaired player position have been built.
    for (entity, physics_entity) in room_visuals.iter() {
        if physics_entity.is_some() {
            physics::retire_physics_entity(commands, entity);
        } else {
            commands.entity(entity).despawn();
        }
    }

    let active_room = transaction.next_spec.id.clone();
    *room_set = transaction.next_room_set;
    world.0 = transaction.next_spec.world.clone();

    runtime.player.pos = transaction.safe_player_pos;
    runtime.player.refresh_movement_resources(tuning);
    runtime.last_safe_player_pos = transaction.safe_player_pos;
    runtime.moving_platform = transaction
        .next_spec
        .moving_platform
        .unwrap_or_else(|| platforms::MovingPlatformState::time_reference(&world.0));
    runtime.features = features::FeatureRuntime::from_world(&world.0);
    runtime.dialogue.close();
    runtime.hitstop_timer = 0.0;
    runtime.hitstun_timer = 0.0;
    runtime.room_transition_cooldown = 0.10;
    runtime.preset_flash = 1.0;

    ldtk_index.replace_from_project(&transaction.project, active_room.clone());

    spawn_room_visuals(
        commands,
        &world.0,
        &room_set.active_spec().loading_zones,
        runtime.physics_settings,
        assets,
    );
    platforms::spawn_moving_platform(commands, &world.0, runtime.moving_platform);

    Ok(active_room)
}

fn sandbox_dt(runtime: &SandboxRuntime, frame_dt: f32) -> f32 {
    if runtime.hitstop_timer > 0.0 {
        0.0
    } else {
        frame_dt * runtime.time_scale
    }
}

// `move_toward` has moved to `crate::lib` (`ambition_sandbox`) so the
// `SandboxRuntime` impl can use it; it is re-imported via the wildcard above.

fn reset_sandbox(
    world: &ae::World,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    let reset_from = runtime.player.pos;
    runtime.reset(world, tuning);
    runtime.flash_timer = feel.reset_flash_time;
    let reset_to = runtime.player.pos;
    sfx.push(SfxMessage::Reset { pos: reset_to });
    vfx.push(VfxMessage::ResetEffects {
        from: reset_from,
        to: reset_to,
    });
}

fn load_room(
    commands: &mut Commands,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    world: &mut GameWorld,
    room_set: &mut rooms::RoomSet,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    transition: rooms::RoomTransition,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    assets: Option<&crate::game_assets::GameAssets>,
) {
    let old_velocity = runtime.player.vel;
    let abilities = runtime.player.abilities;
    let fly_enabled = runtime.player.fly_enabled;
    let edge_exit = matches!(
        transition.zone.activation,
        rooms::LoadingZoneActivation::EdgeExit
    );

    for (entity, physics_entity) in room_visuals.iter() {
        if physics_entity.is_some() {
            physics::retire_physics_entity(commands, entity);
        } else {
            commands.entity(entity).despawn();
        }
    }
    let spec = room_set.set_active(transition.target_room).clone();
    world.0 = spec.world.clone();

    // Room transitions are not player deaths/resets. Rebuild transient room
    // state, but preserve ability progression and, for edge exits, preserve
    // velocity so side-to-side room changes feel continuous. Door transitions
    // intentionally zero velocity because they are discrete interactions.
    let arrival = rooms::validated_spawn(&world.0, transition.arrival, runtime.player.size);
    runtime.player = ae::Player::new_with_abilities(arrival, abilities);
    runtime.player.refresh_movement_resources(tuning);
    runtime.player.fly_enabled = fly_enabled && runtime.player.abilities.fly;
    if edge_exit {
        runtime.player.vel = old_velocity;
    }
    runtime.flash_timer = if edge_exit {
        feel.edge_transition_flash
    } else {
        feel.door_transition_flash
    };
    runtime.hitstop_timer = 0.0;
    runtime.damage_invuln_timer = 0.0;
    runtime.hitstun_timer = 0.0;
    runtime.last_safe_player_pos = runtime.player.pos;
    runtime.time_scale = 1.0;
    runtime.down_tap_timer = 0.0;
    runtime.moving_platform = spec
        .moving_platform
        .unwrap_or_else(|| platforms::MovingPlatformState::time_reference(&world.0));
    runtime.features = features::FeatureRuntime::from_world(&world.0);
    runtime.dialogue.close();
    // This guard prevents immediate backtracking when arriving inside/near a
    // paired zone. It should not feel like frozen input, so keep it short and
    // rely on validated arrivals to do most of the safety work.
    runtime.room_transition_cooldown = if edge_exit {
        feel.edge_transition_cooldown
    } else {
        feel.door_transition_cooldown
    };
    runtime.preset_flash = 1.0;

    spawn_room_visuals(
        commands,
        &world.0,
        &spec.loading_zones,
        physics_settings,
        assets,
    );
    platforms::spawn_moving_platform(commands, &world.0, runtime.moving_platform);
    sfx.push(SfxMessage::Reset {
        pos: runtime.player.pos,
    });
    if edge_exit {
        // Edge exits should feel like contiguous room scrolling, not a death-like
        // teleport. Only show an arrival puff in the new room because `from` was
        // expressed in the previous room's coordinate space.
        vfx.push(VfxMessage::Burst {
            pos: runtime.player.pos,
            count: 18,
            speed: 260.0,
            color: [0.35, 0.95, 1.0, 0.75],
            kind: ParticleKind::Dust,
        });
    } else {
        // Door transitions are discrete interactions, so a teleport-like effect
        // is acceptable; use the destination for both endpoints to avoid mixing
        // coordinate systems from two rooms.
        vfx.push(VfxMessage::ResetEffects {
            from: runtime.player.pos,
            to: runtime.player.pos,
        });
    }
}

fn handle_player_events(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    events: ae::FrameEvents,
    was_grounded: Option<bool>,
) {
    let pos = runtime.player.pos;
    for op in &events.operations {
        match op {
            ae::MovementOp::Jump | ae::MovementOp::WallJump => {
                sfx.push(SfxMessage::Jump { pos });
                vfx.push(VfxMessage::Dust {
                    pos: runtime.player.pos,
                    facing: runtime.player.facing,
                });
            }
            ae::MovementOp::DoubleJump => {
                sfx.push(SfxMessage::DoubleJump { pos });
                vfx.push(VfxMessage::Burst {
                    pos: runtime.player.pos,
                    count: 14,
                    speed: 210.0,
                    color: [0.70, 1.0, 0.86, 0.82],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Dash | ae::MovementOp::DoubleDash => {
                sfx.push(SfxMessage::Dash { pos });
                vfx.push(VfxMessage::Burst {
                    pos: runtime.player.pos,
                    count: 10,
                    speed: 330.0,
                    color: [1.0, 0.86, 0.38, 0.90],
                    kind: ParticleKind::Spark,
                });
            }
            ae::MovementOp::Blink | ae::MovementOp::PrecisionBlink => {
                // Blink visuals use the explicit `events.blinks` endpoint data below.
            }
            ae::MovementOp::FlyToggle => {
                vfx.push(VfxMessage::Burst {
                    pos: runtime.player.pos,
                    count: 12,
                    speed: 180.0,
                    color: [0.45, 0.82, 1.0, 0.72],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Pogo | ae::MovementOp::Rebound => {
                sfx.push(SfxMessage::Pogo { pos });
            }
            ae::MovementOp::WallCling | ae::MovementOp::WallClimb | ae::MovementOp::Slash => {}
            ae::MovementOp::Reset => {
                sfx.push(SfxMessage::Reset { pos });
            }
        }
    }
    for blink in &events.blinks {
        sfx.push(SfxMessage::Blink {
            pos: blink.from,
            precision: blink.precision,
        });
        vfx.push(VfxMessage::BlinkEffects {
            from: blink.from,
            to: blink.to,
            precision: blink.precision,
        });
    }
    if events.hazard || !events.operations.is_empty() {
        runtime.flash_timer = 0.12;
    }
    if let Some(was_grounded) = was_grounded {
        if !was_grounded && runtime.player.on_ground {
            vfx.push(VfxMessage::Dust {
                pos: runtime.player.pos + ae::Vec2::new(0.0, runtime.player.size.y * 0.5),
                facing: runtime.player.facing,
            });
        }
    }
}

fn handle_feature_events(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    debris: &mut Vec<DebrisBurstMessage>,
    events: &features::FeatureEvents,
    player_pos: ae::Vec2,
) {
    if events.reset_player {
        sfx.push(SfxMessage::Reset { pos: player_pos });
    }
    for physics_burst in &events.physics_bursts {
        let cue = match physics_burst.cue {
            features::FeaturePhysicsCue::Breakable => physics::PhysicsDebrisCue::Breakable,
            features::FeaturePhysicsCue::EnemyRagdoll => physics::PhysicsDebrisCue::EnemyRagdoll,
            features::FeaturePhysicsCue::BossRagdoll => physics::PhysicsDebrisCue::BossRagdoll,
        };
        debris.push(DebrisBurstMessage {
            pos: physics_burst.pos,
            cue,
        });
    }
    for &pos in &events.impacts {
        vfx.push(VfxMessage::Impact { pos });
        vfx.push(VfxMessage::Burst {
            pos,
            count: 14,
            speed: 300.0,
            color: [1.0, 0.34, 0.28, 0.88],
            kind: ParticleKind::Shard,
        });
        debris.push(DebrisBurstMessage {
            pos,
            cue: physics::PhysicsDebrisCue::Impact,
        });
    }
    for &pos in &events.bursts {
        vfx.push(VfxMessage::Burst {
            pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
    }
}

fn handle_player_heal_events(runtime: &mut SandboxRuntime, events: &features::FeatureEvents) {
    if events.player_heal > 0 {
        runtime.player_health.heal(events.player_heal);
    }
}

fn death_respawn_player(
    world: &ae::World,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    died: &mut Vec<PlayerDiedMessage>,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
) {
    let to = world.spawn;
    runtime.reset(world, tuning);
    runtime.player_health.reset();
    runtime.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    runtime.flash_timer = feel.reset_flash_time.max(0.35);
    runtime.features.banner = "PLAYER DOWN: respawned at room start with full HP".to_string();
    runtime.features.banner_timer = 2.4;
    sfx.push(SfxMessage::Death { pos: from });
    vfx.push(VfxMessage::ResetEffects { from, to });
    died.push(PlayerDiedMessage { pos: from });
}

fn handle_player_damage_events(
    world: &ae::World,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    died: &mut Vec<PlayerDiedMessage>,
    runtime: &mut SandboxRuntime,
    events: &features::FeatureEvents,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    difficulty_multiplier: f32,
) {
    let Some(mut damage) = events.player_damage.first().copied() else {
        return;
    };
    // Invincibility (debug toggle): drop the damage event entirely
    // before any state mutates so testing systems that consume HP
    // (boss phases, encounter pacing, music) can run uninterrupted.
    if runtime.player.invincible {
        return;
    }
    // Difficulty / assist scaling. Easy halves incoming damage, hard
    // doubles it; the menu setting also exposes a fine-grained
    // gameplay damage multiplier. The minimum is one HP so a damage
    // event always lands somewhere.
    let scaled = ((damage.amount as f32) * difficulty_multiplier).round() as i32;
    damage.amount = scaled.max(1);
    if runtime.player_health.damage(damage.amount) {
        death_respawn_player(
            world,
            sfx,
            vfx,
            died,
            runtime,
            tuning,
            feel,
            damage.impact_pos,
        );
        return;
    }
    match damage.mode {
        features::PlayerDamageMode::SafeRespawn => {
            safe_respawn_player(sfx, vfx, runtime, tuning, feel, damage.impact_pos);
        }
        features::PlayerDamageMode::Knockback => {
            apply_player_knockback(sfx, vfx, runtime, tuning, feel, damage);
        }
    }
}

fn safe_respawn_player(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
) {
    let to = runtime.last_safe_player_pos;
    runtime.player.reset_to(to);
    runtime.player.refresh_movement_resources(tuning);
    runtime.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    runtime.hitstun_timer = 0.0;
    runtime.hitstop_timer = 0.0;
    runtime.flash_timer = feel.reset_flash_time;
    runtime.time_scale = 1.0;
    sfx.push(SfxMessage::Reset { pos: to });
    vfx.push(VfxMessage::ResetEffects { from, to });
}

fn apply_player_knockback(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    damage: features::PlayerDamageEvent,
) {
    let _source_pos_for_future_directional_rules = damage.source_pos;
    let boss_hit = matches!(
        damage.source,
        features::PlayerDamageSource::BossBody | features::PlayerDamageSource::BossAttack
    );
    let dir = if damage.knockback_dir.abs() <= 0.001 {
        -runtime.player.facing
    } else {
        damage.knockback_dir.signum()
    };
    let strength = damage.strength.max(0.0);
    let knock_x = if boss_hit {
        feel.boss_knockback_x
    } else {
        feel.enemy_knockback_x
    };
    let knock_y = if boss_hit {
        feel.boss_knockback_y
    } else {
        feel.enemy_knockback_y
    };
    runtime.player.vel.x = dir * knock_x * strength;
    runtime.player.vel.y = -knock_y * strength;
    runtime.player.refresh_movement_resources(tuning);
    runtime.hitstun_timer = if boss_hit {
        feel.boss_hitstun_time
    } else {
        feel.enemy_hitstun_time
    } * strength.max(0.35);
    runtime.damage_invuln_timer = feel.knockback_invulnerability_time;
    runtime.hitstop_timer = feel.player_damage_hitstop_time;
    runtime.flash_timer = 0.20;
    sfx.push(SfxMessage::Hit {
        pos: damage.impact_pos,
    });
    vfx.push(VfxMessage::Impact {
        pos: damage.impact_pos,
    });
}

fn controls_for_hitstun(
    mut controls: ControlFrame,
    feel: SandboxFeelTuning,
    hitstun_timer: f32,
) -> ControlFrame {
    if hitstun_timer <= 0.0 {
        return controls;
    }
    let scale = feel.hitstun_control_scale.clamp(0.0, 1.0);
    controls.axis_x *= scale;
    controls.axis_y *= scale;
    controls.jump_pressed = false;
    controls.dash_pressed = false;
    controls.fast_fall_pressed = false;
    controls.blink_pressed = false;
    controls.blink_held = false;
    controls.blink_released = false;
    controls.attack_pressed = false;
    controls.pogo_pressed = false;
    controls.fly_toggle_pressed = false;
    controls.interact_pressed = false;
    controls
}

fn process_attack(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    debris: &mut Vec<DebrisBurstMessage>,
    runtime: &mut SandboxRuntime,
    controls: ControlFrame,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    if !runtime.player.abilities.attack {
        return;
    }
    let player_pos = runtime.player.pos;
    sfx.push(SfxMessage::Slash { pos: player_pos });
    // Roughly the slash sheet's eight 75ms frames; the animation system
    // freezes on the last frame once `clip_held` is set, so this only
    // needs to cover the typical clip duration.
    runtime.slash_anim_timer = 0.60;
    let attack = ae::slash_hitbox(&runtime.player, controls.axis_y, controls.pogo_pressed);
    vfx.push(VfxMessage::SlashPreview { hitbox: attack });
    let mut landed = false;
    let mut killed = false;
    let player_facing = runtime.player.facing;
    let slash_damage = runtime.player.damage_multiplier.max(1);
    let feature_events =
        runtime
            .features
            .apply_player_attack(attack, slash_damage, player_facing * 300.0);
    landed |= !feature_events.impacts.is_empty();
    killed |= feature_events
        .messages
        .iter()
        .any(|message| message.contains("defeated"));
    handle_feature_events(sfx, vfx, debris, &feature_events, player_pos);

    if landed {
        sfx.push(SfxMessage::Hit { pos: player_pos });
        runtime.hitstop_timer = feel.attack_hitstop_time;
        runtime.flash_timer = 0.16;
    }
    if killed {
        sfx.push(SfxMessage::Death { pos: player_pos });
    }
    if landed && runtime.player.abilities.pogo && (controls.pogo_pressed || controls.axis_y > 0.25)
    {
        runtime.player.vel.y = -tuning.pogo_speed;
        runtime.player.refresh_movement_resources(tuning);
        sfx.push(SfxMessage::Pogo { pos: player_pos });
    }
}

fn update_hud(
    runtime: Res<SandboxRuntime>,
    mode: Res<State<GameMode>>,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    display_mode: Res<windowing::DisplayModeState>,
    developer_tools: Res<DeveloperTools>,
    ldtk_reload: Res<ldtk_world::LdtkHotReloadState>,
    ldtk_spine: Res<ldtk_world::LdtkRuntimeSpineStats>,
    ldtk_spine_index: Res<ldtk_world::LdtkRuntimeSpineIndex>,
    trace: Res<crate::trace::GameplayTraceBuffer>,
    mechanics: Res<crate::mechanics::MechanicsRegistry>,
    progression: ProgressionResources,
    windows: Query<&Window, With<PrimaryWindow>>,
    entities: Res<SceneEntities>,
    mut query: Query<&mut Text, With<HudText>>,
) {
    let quest_registry = &progression.quests;
    let cutscene = &progression.cutscene;
    let boss_registry = &progression.bosses;
    let encounter_registry = &progression.encounters;
    let map_state = &progression.map;
    let Ok(mut text) = query.get_mut(entities.hud) else {
        return;
    };
    if !developer_tools.show_hud {
        **text = String::new();
        return;
    }
    if !runtime.debug {
        **text = "F1 debug | F3 inspector".to_string();
        return;
    }
    let preset = runtime.preset();
    let enemy_health = runtime
        .features
        .enemies
        .iter()
        .map(|e| {
            format!(
                "{} hp {}/{} alive {}",
                e.name,
                e.health.current.max(0),
                e.health.max,
                e.alive
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let mut gamepad = String::new();
    for (physical, semantic) in GAMEPAD_MAP.iter().take(6) {
        gamepad.push_str(&format!("{} = {}  ", physical, semantic));
    }
    let window_line = windows
        .single()
        .map(|w| {
            format!(
                "window: {:.0}x{:.0} {}",
                w.width(),
                w.height(),
                display_mode.label()
            )
        })
        .unwrap_or_else(|_| format!("window: unknown {}", display_mode.label()));
    let zone_hint = {
        let hints = room_set.nearby_zone_hints(&runtime.player, runtime.player.fly_enabled);
        if hints.is_empty() {
            "zones: none".to_string()
        } else {
            format!("zones: {}", hints.join(" | "))
        }
    };
    let feature_banner = if runtime.features.banner_timer > 0.0 {
        format!("\nFEATURE: {}", runtime.features.banner)
    } else {
        String::new()
    };
    // Quest content now lives in its own UI surface
    // (`update_quest_panel` writes to `QuestPanelText`); the debug HUD
    // no longer carries a `\nQUESTS: ...` trailer. The compact HUD
    // branch keeps emitting the line for the single-screen dump
    // (testers want everything-at-once); the verbose branch omits it.
    let quest_lines = quest_registry.quest_log_lines();
    let quest_line = if quest_lines.is_empty() {
        String::new()
    } else {
        format!("\nQUESTS: {}", quest_lines.join("  ::  "))
    };
    let cutscene_line = if let Some(rt) = cutscene.runtime.as_ref() {
        let beat = match cutscene.current_dialogue.as_ref() {
            Some((speaker, text)) => format!("[{speaker}]  {text}  (E to continue)"),
            None => match cutscene.current_banner.as_ref() {
                Some((banner, _)) => format!("// {banner}"),
                None => format!("cutscene: beat {}", rt.beat_index),
            },
        };
        // Skip-hold progress: only render the bar while a hold is in
        // progress, so an idle cutscene doesn't show a clutter prompt.
        let skip_progress = progression.cutscene_request.skip_progress();
        let skip_hint = if skip_progress > 0.01 {
            let filled = (skip_progress * 12.0).round().clamp(0.0, 12.0) as usize;
            let empty = 12usize.saturating_sub(filled);
            format!(
                "  hold Backspace/Select to skip [{}{}] {:.0}%",
                "=".repeat(filled),
                "-".repeat(empty),
                skip_progress * 100.0,
            )
        } else {
            "  (Backspace/Select held = skip)".to_string()
        };
        format!("\nCUTSCENE: {beat}{skip_hint}")
    } else {
        String::new()
    };
    let boss_line = if let Some((id, phase)) = boss_registry.active_phase() {
        if let Some(state) = boss_registry.get(id) {
            // Health bar: 16-tick string that shrinks as boss HP drops
            // so the player gets a glanceable progress signal even
            // before a real HUD lands.
            let frac = state.hp_fraction();
            let filled = (frac * 16.0).round().clamp(0.0, 16.0) as usize;
            let empty = 16usize.saturating_sub(filled);
            let bar = format!("[{}{}]", "=".repeat(filled), "-".repeat(empty));
            format!(
                "\nBOSS [{}] {} hp {}/{} {} {:.0}%",
                id,
                phase.label(),
                state.hp,
                state.spec.max_hp,
                bar,
                frac * 100.0,
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let encounter_line = {
        let mut bits = Vec::new();
        for (_id, state) in encounter_registry.encounters.iter() {
            if matches!(
                state.phase,
                crate::encounter::EncounterPhase::Starting { .. }
                    | crate::encounter::EncounterPhase::Active { .. }
            ) {
                bits.push(state.hud_summary());
            }
        }
        if bits.is_empty() {
            String::new()
        } else {
            format!("\nENCOUNTER {}", bits.join("  ::  "))
        }
    };
    let map_lines = map_state.summary_lines(&room_set.active_spec().id);
    let map_line = if map_lines.is_empty() {
        String::new()
    } else {
        format!("\nMAP\n{}", map_lines.join("\n"))
    };
    let locomotion = ae::LocomotionState::from_player(&runtime.player).label();
    let body_mode = ae::BodyMode::from_player(&runtime.player).label();
    let trace_status = match (&trace.last_dump_status, &trace.last_dump_path) {
        (Some(status), _) => status.clone(),
        (None, _) => format!(
            "{} frames / {} events buffered (F8 dump)",
            trace.frame_count(),
            trace.event_count()
        ),
    };
    let mechanics_summary = format!(
        "stable={} backend={} planned={}",
        mechanics.count_by_maturity(crate::mechanics::MechanicMaturity::Stable),
        mechanics.count_by_maturity(crate::mechanics::MechanicMaturity::Backend),
        mechanics.count_by_maturity(crate::mechanics::MechanicMaturity::Planned),
    );
    let metadata = room_set.active_metadata();
    let metadata_summary = if metadata.is_empty() {
        "—".to_string()
    } else {
        let mut bits: Vec<String> = Vec::new();
        if let Some(b) = &metadata.biome {
            bits.push(format!("biome={b}"));
        }
        if let Some(t) = &metadata.music_track {
            bits.push(format!("music={t}"));
        }
        if let Some(a) = &metadata.ambient_profile {
            bits.push(format!("ambient={a}"));
        }
        if let Some(v) = &metadata.visual_theme {
            bits.push(format!("theme={v}"));
        }
        bits.join(" ")
    };
    let mechanics_line = format!(
        "\nLOCO: {locomotion}  BODY: {body_mode}  MECH: {mechanics_summary}  ROOM: {metadata_summary}  TRACE: {trace_status}"
    );
    if developer_tools.compact_hud {
        **text = format!(
            "{} | {} | room {}/{} | hp {}/{} | vel ({:+.0},{:+.0}) | grounded {} | dash {} | jumps {}\ncombo: {} | hint: {}\n{} | ldtk: {} auto={} pending={} spine={} rev={} promoted={} last={} | hitstun {:.2} invuln {:.2} hitstop {:.2} | preset {} | F1 debug F3 inspector F4 world F5 overview={} F11 reload F12 auto\n{}{}{}{}{}{}{}{}\n",
            world.0.name,
            mode.get().label(),
            room_set.active + 1,
            room_set.rooms.len(),
            runtime.player_health.current.max(0),
            runtime.player_health.max,
            runtime.player.vel.x,
            runtime.player.vel.y,
            runtime.player.on_ground,
            runtime.player.dash_charges_available,
            runtime.player.air_jumps_available,
            runtime.player.combo_symbols(),
            runtime.player.current_combo_hint(),
            zone_hint,
            ldtk_reload.last_status,
            ldtk_reload.auto_apply,
            ldtk_reload.pending,
            ldtk_spine.spawned_entities,
            ldtk_spine_index.revision,
            ldtk_spine_index.promoted_summary(),
            if ldtk_spine.last_entity.is_empty() { "none" } else { &ldtk_spine.last_entity },
            runtime.hitstun_timer,
            runtime.damage_invuln_timer,
            runtime.hitstop_timer,
            preset.name,
            developer_tools.overview_camera,
            runtime.features.feature_summary(),
            feature_banner,
            quest_line,
            cutscene_line,
            boss_line,
            encounter_line,
            map_line,
            mechanics_line,
        );
        return;
    }
    let flash_line = if runtime.preset_flash > 0.0 {
        format!("\nPRESET: {}", preset.name)
    } else {
        String::new()
    };
    // Verbose HUD: high-level gameplay readout. Low-level player physics
    // (velocities, timers, blink/fly flags, hitstop/hitstun/invuln,
    // time_scale, inspector visibility) live in `bevy-inspector-egui`
    // (F3) — surfacing them again here just clutters the screen during
    // play. The compact HUD branch (above) keeps a single-screen
    // diagnostic dump for when you want everything at once.
    **text = format!(
        "{}  mode: {}  room {}/{}  size {:.0}x{:.0}\n\
         {}\n\
         hp {}/{}  dash {}  air_jumps {}  charges {}  combo: {}\n\
         hint: {}\n\
         preset: {}\n\
         F1 debug  F2 slowmo  F3 inspector  F4 world-inspector  F5 overview={}  F8 trace dump  F11 LDtk reload  F12 LDtk auto={}  Esc mode={}  Delete reset\n\
         LDtk: {} (spine {} entities, promoted {})\n\
         {}\n\
         enemies: {}\n\
         {}\n\
         gamepad: {}{}{}{}\n",
        world.0.name,
        mode.get().label(),
        room_set.active + 1,
        room_set.rooms.len(),
        world.0.size.x,
        world.0.size.y,
        zone_hint,
        runtime.player_health.current.max(0),
        runtime.player_health.max,
        runtime.player.dash_charges_available,
        runtime.player.air_jumps_available,
        runtime.player.mana.current as i32,
        runtime.player.combo_symbols(),
        runtime.player.current_combo_hint(),
        preset.name,
        developer_tools.overview_camera,
        ldtk_reload.auto_apply,
        mode.get().label(),
        ldtk_reload.last_status,
        ldtk_spine.spawned_entities,
        ldtk_spine_index.promoted_summary(),
        window_line,
        enemy_health,
        runtime.features.feature_summary(),
        gamepad,
        flash_line,
        feature_banner,
        mechanics_line,
    );
    // Cutscene / boss / encounter / map lines stay in the verbose HUD
    // because they're tightly coupled to the live combat / traversal
    // status the rest of the HUD shows. Quests live in their own
    // panel (`update_quest_panel`).
    if !cutscene_line.is_empty()
        || !boss_line.is_empty()
        || !encounter_line.is_empty()
        || !map_line.is_empty()
    {
        text.push_str(&cutscene_line);
        text.push_str(&boss_line);
        text.push_str(&encounter_line);
        text.push_str(&map_line);
    }
}

/// Update the dedicated quest-panel text widget.
///
/// Lives separately from `update_hud` so the quest log doesn't trail
/// the giant debug stats dump and can be styled / positioned
/// independently. Writes empty string when there are no active
/// quests, which collapses the panel visually.
pub fn update_quest_panel(
    quests: Res<crate::quest::QuestRegistry>,
    entities: Res<SceneEntities>,
    mut query: Query<&mut Text, With<crate::rendering::QuestPanelText>>,
) {
    if entities.quest_panel == Entity::PLACEHOLDER {
        return;
    }
    let Ok(mut text) = query.get_mut(entities.quest_panel) else {
        return;
    };
    let lines = quests.quest_log_lines();
    if lines.is_empty() {
        **text = String::new();
    } else {
        **text = format!("QUESTS\n  {}", lines.join("\n  "));
    }
}
