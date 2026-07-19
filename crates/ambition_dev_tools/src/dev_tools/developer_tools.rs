//! The `DeveloperTools` resource (debug toggles + inspector state) + helpers.

use super::*;
use ambition_engine_core as ae;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Top-level switches for debug UI and gizmo layers.
#[derive(Resource, Reflect, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[reflect(Resource)]
#[serde(default)]
pub struct DeveloperTools {
    /// Show the reflected resource inspector windows.
    pub inspector_visible: bool,
    /// Show the heavier full-world entity/resource inspector.
    pub world_inspector_visible: bool,
    /// Master switch for Bevy gizmo overlays. `F1` still controls the old HUD/debug mode.
    pub gizmos_enabled: bool,
    pub show_hud: bool,
    /// Keep the HUD compact now that the inspector exposes detailed live state.
    pub compact_hud: bool,
    pub debug_view_mode: DebugViewMode,
    pub debug_art_mode: DebugArtMode,
    pub show_room_bounds: bool,
    pub show_world_blocks: bool,
    pub show_loading_zones: bool,
    pub show_player_hitbox: bool,
    pub show_player_vectors: bool,
    pub show_blink_preview: bool,
    pub show_combat_preview: bool,
    pub show_feature_hitboxes: bool,
    pub show_health_bars: bool,
    pub show_moving_platform: bool,
    /// Draw an 8px subdivision grid over the authored 16px tile grid.
    /// Useful for checking whether a level needs half-tile/freeform collision
    /// affordances without changing LDtk art scale yet.
    pub show_micro_grid: bool,
    /// Draw the current requested/actual camera frame rectangles. Kept separate
    /// from player vectors because the camera rectangles are intentionally huge
    /// and can be visually mistaken for a player-local hitbox.
    pub show_camera_frame: bool,
    pub show_rebound_vectors: bool,
    /// Toggle a zoomed-out camera for inspecting large or stitched active areas.
    pub overview_camera: bool,
    /// Orthographic scale used while overview camera is enabled.
    pub overview_camera_scale: f32,

    // Manual override for camera zoom tweaking
    pub camera_view_override_enabled: bool,
    pub camera_view_w: f32,
    pub camera_view_h: f32,

    /// When true, sprite/visual rendering is suppressed so only hitbox gizmos
    /// are visible. Useful for diagnosing spatial mismatches between art and
    /// collision geometry without sprite occlusion.
    pub hide_sprites: bool,
    /// When true, every textured sprite is replaced with a colored rectangle
    /// sized to the gameplay/debug volume. Owned by `debug_art_mode` together
    /// with `hide_sprites`; if stale state ever leaves both true, placeholder
    /// mode wins so the rectangles remain visible.
    pub placeholder_sprites: bool,
    /// When true, gizmo AABBs are drawn with a translucent fill in addition
    /// to their outline. Makes overlapping volumes and empty regions easier
    /// to read at a glance.
    pub fill_debug_boxes: bool,
    /// High-level movement collider size preset for sandbox feel testing.
    pub player_body_profile: PlayerBodyProfile,
    /// High-level movement tuning preset for sandbox feel testing.
    pub movement_profile: MovementProfile,
}

impl Default for DeveloperTools {
    fn default() -> Self {
        let phone_demo = cfg!(target_os = "android");
        let debug_view_mode = if phone_demo {
            DebugViewMode::Gameplay
        } else {
            DebugViewMode::Authoring
        };
        let debug_art_mode = if phone_demo {
            DebugArtMode::Normal
        } else {
            debug_view_mode.recommended_art_mode()
        };
        let mut tools = Self {
            inspector_visible: false,
            world_inspector_visible: false,
            // Desktop keeps the detailed layers ready behind the shared
            // debug-overlay gate. Android also disables the heavier layers so
            // the touch HUD and gameplay viewport remain usable on a small screen.
            gizmos_enabled: !phone_demo,
            show_hud: !phone_demo,
            compact_hud: true,
            debug_view_mode,
            debug_art_mode,
            show_room_bounds: false,
            show_world_blocks: false,
            show_loading_zones: false,
            show_player_hitbox: false,
            show_player_vectors: false,
            show_blink_preview: false,
            show_combat_preview: false,
            show_feature_hitboxes: false,
            show_health_bars: false,
            show_moving_platform: false,
            show_micro_grid: false,
            show_camera_frame: false,
            show_rebound_vectors: false,
            overview_camera: false,
            overview_camera_scale: 2.35,
            camera_view_override_enabled: false,
            camera_view_w: 800.0,
            camera_view_h: 450.0,
            hide_sprites: false,
            placeholder_sprites: false,
            fill_debug_boxes: false,
            player_body_profile: PlayerBodyProfile::default(),
            movement_profile: MovementProfile::default(),
        };
        tools.apply_debug_view_mode(debug_view_mode, !phone_demo);
        tools.apply_debug_art_mode(debug_art_mode);
        tools
    }
}

impl DeveloperTools {
    pub fn apply_debug_view_mode(&mut self, mode: DebugViewMode, apply_art_recommendation: bool) {
        self.debug_view_mode = mode;
        match mode {
            DebugViewMode::Gameplay => {
                self.show_room_bounds = false;
                self.show_world_blocks = false;
                self.show_loading_zones = false;
                self.show_player_hitbox = false;
                self.show_player_vectors = false;
                self.show_blink_preview = false;
                self.show_combat_preview = false;
                self.show_feature_hitboxes = false;
                self.show_health_bars = false;
                self.show_moving_platform = false;
                self.show_rebound_vectors = false;
                self.show_micro_grid = false;
                self.show_camera_frame = false;
                self.fill_debug_boxes = false;
            }
            DebugViewMode::Authoring => {
                self.show_room_bounds = true;
                self.show_world_blocks = true;
                self.show_loading_zones = true;
                self.show_player_hitbox = true;
                self.show_player_vectors = true;
                self.show_blink_preview = true;
                self.show_combat_preview = true;
                self.show_feature_hitboxes = true;
                self.show_health_bars = true;
                self.show_moving_platform = true;
                self.show_rebound_vectors = true;
                self.show_micro_grid = false;
                self.show_camera_frame = false;
                self.fill_debug_boxes = false;
            }
            DebugViewMode::Collision => {
                self.show_room_bounds = true;
                self.show_world_blocks = true;
                self.show_loading_zones = false;
                self.show_player_hitbox = true;
                self.show_player_vectors = false;
                self.show_blink_preview = true;
                self.show_combat_preview = false;
                self.show_feature_hitboxes = true;
                self.show_health_bars = false;
                self.show_moving_platform = true;
                self.show_rebound_vectors = true;
                self.show_micro_grid = false;
                self.show_camera_frame = false;
                self.fill_debug_boxes = true;
            }
            DebugViewMode::Triggers => {
                self.show_room_bounds = true;
                self.show_world_blocks = false;
                self.show_loading_zones = true;
                self.show_player_hitbox = true;
                self.show_player_vectors = false;
                self.show_blink_preview = false;
                self.show_combat_preview = false;
                self.show_feature_hitboxes = true;
                self.show_health_bars = false;
                self.show_moving_platform = false;
                self.show_rebound_vectors = false;
                self.show_micro_grid = false;
                self.show_camera_frame = true;
                self.fill_debug_boxes = true;
            }
            DebugViewMode::Combat => {
                self.show_room_bounds = false;
                self.show_world_blocks = false;
                self.show_loading_zones = false;
                self.show_player_hitbox = true;
                self.show_player_vectors = true;
                self.show_blink_preview = false;
                self.show_combat_preview = true;
                self.show_feature_hitboxes = true;
                self.show_health_bars = true;
                self.show_moving_platform = false;
                self.show_rebound_vectors = false;
                self.show_micro_grid = false;
                self.show_camera_frame = false;
                self.fill_debug_boxes = true;
            }
            DebugViewMode::All => {
                self.show_room_bounds = true;
                self.show_world_blocks = true;
                self.show_loading_zones = true;
                self.show_player_hitbox = true;
                self.show_player_vectors = true;
                self.show_blink_preview = true;
                self.show_combat_preview = true;
                self.show_feature_hitboxes = true;
                self.show_health_bars = true;
                self.show_moving_platform = true;
                self.show_rebound_vectors = true;
                self.show_micro_grid = true;
                self.show_camera_frame = true;
                self.fill_debug_boxes = true;
            }
            DebugViewMode::Custom => {}
        }
        if apply_art_recommendation {
            self.apply_debug_art_mode(mode.recommended_art_mode());
        }
    }

    pub fn mark_debug_view_custom(&mut self) {
        self.debug_view_mode = DebugViewMode::Custom;
    }

    pub fn apply_debug_art_mode(&mut self, mode: DebugArtMode) {
        self.debug_art_mode = mode;
        self.hide_sprites = matches!(mode, DebugArtMode::Hidden);
        self.placeholder_sprites = matches!(mode, DebugArtMode::Placeholder);
    }

    /// Repair stale persisted states from the older independent
    /// hide/placeholder toggles so `DebugArtMode` remains the sole owner.
    pub fn normalize_debug_modes(&mut self) {
        let mode = match (
            self.debug_art_mode,
            self.placeholder_sprites,
            self.hide_sprites,
        ) {
            (_, true, _) => DebugArtMode::Placeholder,
            (_, false, true) => DebugArtMode::Hidden,
            (mode, false, false) => mode,
        };
        self.apply_debug_art_mode(mode);
    }
}

pub fn inspector_visible(tools: Res<DeveloperTools>) -> bool {
    tools.inspector_visible
}

/// Produce a compact normalized movement readout for the debug HUD.
///
/// These are design-space estimates, not an engine replay. They are meant to
/// answer questions like "how many tiles/body-heights is this jump?" while
/// swapping chassis profiles from the F3 menu.
#[allow(
    dead_code,
    reason = "Design-space movement readout exposed for the F3 debug HUD chassis-profile swap; pre-existing dev surface that's wired into a HUD subscreen that hasn't landed yet."
)]
pub fn feel_metrics_summary(base_size: ae::Vec2, tuning: ae::MovementTuning) -> String {
    const TILE: f32 = 16.0;
    let body_w = base_size.x.max(1.0);
    let body_h = base_size.y.max(1.0);
    let run_tiles = tuning.max_run_speed / TILE;
    let run_body = tuning.max_run_speed / body_w;
    let jump_height = (tuning.jump_speed * tuning.jump_speed) / (2.0 * tuning.gravity.max(1.0));
    let jump_tiles = jump_height / TILE;
    let jump_body = jump_height / body_h;
    let apex = tuning.jump_speed / tuning.gravity.max(1.0);
    let dash_distance = tuning.dash_speed * tuning.dash_time;
    let dash_tiles = dash_distance / TILE;
    let dash_body = dash_distance / body_w;
    let double_ratio = if tuning.jump_speed.abs() > 1.0 {
        (tuning.double_jump_speed / tuning.jump_speed).powi(2) * 100.0
    } else {
        0.0
    };
    let view_note = format!(
        "run {:.1} tiles/s {:.1} body/s | jump {:.1} tiles {:.2} body apex {:.2}s | dash {:.1} tiles {:.1} body | dj {:.0}%",
        run_tiles,
        run_body,
        jump_tiles,
        jump_body,
        apex,
        dash_tiles,
        dash_body,
        double_ratio,
    );
    view_note
}

pub fn world_inspector_visible(tools: Res<DeveloperTools>) -> bool {
    tools.world_inspector_visible
}
