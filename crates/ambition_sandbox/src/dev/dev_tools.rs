//! Developer-facing tuning and inspection tools.
//!
//! This module is intentionally sandbox-side: it is allowed to depend on Bevy
//! reflection and inspector UI crates, while `ambition_engine` stays focused on
//! reusable Bevy-native movement/collision logic. The reflected resources here mirror
//! engine data so live tuning can happen without forcing Bevy dependencies into
//! the reusable crate.

use ambition_engine as ae;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Coarse player-body presets for feel testing.
///
/// These affect the movement collider only. The placeholder sprite is scaled
/// separately around the collider so temporary art can change without becoming
/// gameplay authority.
#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerBodyProfile {
    Compact,
    #[default]
    ReadableDefault,
    Heavy,
}

impl PlayerBodyProfile {
    pub const ALL: [Self; 3] = [Self::Compact, Self::ReadableDefault, Self::Heavy];

    pub fn label(self) -> &'static str {
        match self {
            Self::Compact => "compact 26x42",
            Self::ReadableDefault => "default 30x48",
            Self::Heavy => "heavy 32x50",
        }
    }

    pub fn size(self) -> ae::Vec2 {
        match self {
            Self::Compact => ae::Vec2::new(26.0, 42.0),
            Self::ReadableDefault => ae::Vec2::new(30.0, 48.0),
            Self::Heavy => ae::Vec2::new(32.0, 50.0),
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(1);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(1);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// High-level movement profiles for fast chassis swaps from the dev menu.
#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementProfile {
    AgileBase,
    #[default]
    SandboxFast,
    Heavy,
    Legacy,
}

impl MovementProfile {
    pub const ALL: [Self; 4] = [
        Self::AgileBase,
        Self::SandboxFast,
        Self::Heavy,
        Self::Legacy,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::AgileBase => "agile base",
            Self::SandboxFast => "sandbox fast",
            Self::Heavy => "heavy",
            Self::Legacy => "legacy",
        }
    }

    pub fn tuning(self) -> ae::MovementTuning {
        let mut tuning = ae::MovementTuning::default();
        match self {
            Self::AgileBase => {
                tuning.flight_accel = 2200.0;
                tuning.flight_drag = 1600.0;
                tuning.flight_terminal_speed = 560.0;
                tuning.flight_hover_speed = 30.0;
            }
            Self::SandboxFast => {
                // Matches the Phase 1 default constants: slower base chassis,
                // but fast explicit flight for sandbox traversal.
            }
            Self::Heavy => {
                tuning.max_run_speed = 240.0;
                tuning.run_accel = 4200.0;
                tuning.air_accel = 2400.0;
                tuning.ground_friction = 7000.0;
                tuning.air_friction = 550.0;
                tuning.gravity = 2450.0;
                tuning.jump_speed = 570.0;
                tuning.double_jump_speed = 470.0;
                tuning.max_fall_speed = 1000.0;
                tuning.dash_speed = 700.0;
                tuning.dash_time = 0.115;
                tuning.dash_cooldown = 0.220;
                tuning.flight_accel = 1900.0;
                tuning.flight_drag = 1450.0;
                tuning.flight_terminal_speed = 520.0;
                tuning.flight_hover_speed = 28.0;
            }
            Self::Legacy => {
                tuning.gravity = 2250.0;
                tuning.run_accel = 7600.0;
                tuning.air_accel = 4700.0;
                tuning.ground_friction = 9200.0;
                tuning.air_friction = 860.0;
                tuning.max_run_speed = 330.0;
                tuning.max_fall_speed = 1040.0;
                tuning.jump_speed = 690.0;
                tuning.double_jump_speed = 630.0;
                tuning.wall_jump_x = 565.0;
                tuning.wall_slide_speed = 170.0;
                tuning.wall_climb_speed = 250.0;
                tuning.dash_speed = 820.0;
                tuning.dash_time = 0.105;
                tuning.dash_cooldown = 0.060;
                tuning.dash_buffer = 0.110;
                tuning.flight_accel = 900.0;
                tuning.flight_drag = 520.0;
                tuning.flight_terminal_speed = 430.0;
                tuning.flight_hover_speed = 42.0;
                tuning.pogo_speed = 810.0;
                tuning.slash_recoil = 130.0;
            }
        }
        tuning
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(1);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(1);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Named debug visualization presets for the daily "what am I inspecting?"
/// workflow. Individual booleans still exist as the Custom backing store, but
/// these modes are the primary interface exposed in the developer menu.
#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugViewMode {
    /// Normal play view: no spatial overlays.
    Gameplay,
    /// Level-authoring view: room bounds, triggers, and camera framing.
    Authoring,
    /// Collision view: solid/query volumes with art out of the way.
    Collision,
    /// Trigger/transition view: loading zones and camera/debug volumes.
    Triggers,
    /// Combat-feel view: actor/projectile/player combat volumes.
    Combat,
    /// Everything we can draw without opening the full inspector.
    All,
    /// Hand-edited toggle state.
    #[default]
    Custom,
}

impl DebugViewMode {
    pub const ALL: [Self; 7] = [
        Self::Gameplay,
        Self::Authoring,
        Self::Collision,
        Self::Triggers,
        Self::Combat,
        Self::All,
        Self::Custom,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Gameplay => "gameplay",
            Self::Authoring => "authoring",
            Self::Collision => "collision",
            Self::Triggers => "triggers",
            Self::Combat => "combat",
            Self::All => "all",
            Self::Custom => "custom",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub const fn recommended_art_mode(self) -> DebugArtMode {
        match self {
            Self::Gameplay | Self::Authoring | Self::Triggers | Self::Combat | Self::All => {
                DebugArtMode::Normal
            }
            Self::Collision => DebugArtMode::Hidden,
            Self::Custom => DebugArtMode::Normal,
        }
    }
}

/// How normal sprite presentation should behave while inspecting debug data.
#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugArtMode {
    #[default]
    Normal,
    Placeholder,
    Hidden,
}

impl DebugArtMode {
    pub const ALL: [Self; 3] = [Self::Normal, Self::Placeholder, Self::Hidden];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Placeholder => "placeholder",
            Self::Hidden => "hidden",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

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
    /// When true, sprite/visual rendering is suppressed so only hitbox gizmos
    /// are visible. Useful for diagnosing spatial mismatches between art and
    /// collision geometry without sprite occlusion.
    pub hide_sprites: bool,
    /// When true, every textured sprite is replaced with a colored rectangle
    /// of the same size — the "placeholder art era" look. Independent from
    /// `hide_sprites`: enable placeholders to confirm that gameplay is
    /// readable with only solid rectangles, or combine with `hide_sprites`
    /// to also drop the placeholders and rely purely on debug gizmos.
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
            // Desktop keeps the traditional debug-first sandbox posture.
            // Android starts clean so the touch HUD and gameplay viewport
            // are usable on a small screen; debug/gizmo state can still be
            // toggled from settings/dev paths later.
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
                self.show_world_blocks = false;
                self.show_loading_zones = true;
                self.show_player_hitbox = false;
                self.show_player_vectors = false;
                self.show_blink_preview = false;
                self.show_combat_preview = false;
                self.show_feature_hitboxes = false;
                self.show_health_bars = false;
                self.show_moving_platform = true;
                self.show_rebound_vectors = false;
                self.show_micro_grid = false;
                self.show_camera_frame = true;
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
}

pub fn inspector_visible(tools: Res<DeveloperTools>) -> bool {
    tools.inspector_visible
}

/// Produce a compact normalized movement readout for the debug HUD.
///
/// These are design-space estimates, not an engine replay. They are meant to
/// answer questions like "how many tiles/body-heights is this jump?" while
/// swapping chassis profiles from the F3 menu.
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

/// Reflected mirror of `ambition_engine::AbilitySet` for live inspector editing.
#[derive(Resource, Reflect, Clone, Copy, Debug)]
#[reflect(Resource)]
pub struct EditableAbilitySet {
    pub move_horizontal: bool,
    pub jump: bool,
    pub variable_jump: bool,
    pub double_jump: bool,
    pub fast_fall: bool,
    pub wall_jump: bool,
    pub wall_cling: bool,
    pub wall_climb: bool,
    pub dash: bool,
    pub double_dash: bool,
    pub fly: bool,
    pub blink: bool,
    pub precision_blink: bool,
    pub blink_through_soft_walls: bool,
    pub blink_through_hard_walls: bool,
    pub attack: bool,
    pub pogo: bool,
    pub directional_primary: bool,
    pub directional_special: bool,
    pub rebound: bool,
    pub reset: bool,
    pub ledge_grab: bool,
    pub swim: bool,
    pub glide: bool,
    pub dodge: bool,
    pub shield: bool,
}

impl EditableAbilitySet {
    pub fn as_engine(self) -> ae::AbilitySet {
        ae::AbilitySet {
            move_horizontal: self.move_horizontal,
            jump: self.jump,
            variable_jump: self.variable_jump,
            double_jump: self.double_jump,
            fast_fall: self.fast_fall,
            wall_jump: self.wall_jump,
            wall_cling: self.wall_cling,
            wall_climb: self.wall_climb,
            dash: self.dash,
            double_dash: self.double_dash,
            fly: self.fly,
            blink: self.blink,
            precision_blink: self.precision_blink,
            blink_through_soft_walls: self.blink_through_soft_walls,
            blink_through_hard_walls: self.blink_through_hard_walls,
            attack: self.attack,
            pogo: self.pogo,
            directional_primary: self.directional_primary,
            directional_special: self.directional_special,
            rebound: self.rebound,
            reset: self.reset,
            ledge_grab: self.ledge_grab,
            swim: self.swim,
            glide: self.glide,
            dodge: self.dodge,
            shield: self.shield,
        }
    }
}

impl From<ae::AbilitySet> for EditableAbilitySet {
    fn from(value: ae::AbilitySet) -> Self {
        Self {
            move_horizontal: value.move_horizontal,
            jump: value.jump,
            variable_jump: value.variable_jump,
            double_jump: value.double_jump,
            fast_fall: value.fast_fall,
            wall_jump: value.wall_jump,
            wall_cling: value.wall_cling,
            wall_climb: value.wall_climb,
            dash: value.dash,
            double_dash: value.double_dash,
            fly: value.fly,
            blink: value.blink,
            precision_blink: value.precision_blink,
            blink_through_soft_walls: value.blink_through_soft_walls,
            blink_through_hard_walls: value.blink_through_hard_walls,
            attack: value.attack,
            pogo: value.pogo,
            directional_primary: value.directional_primary,
            directional_special: value.directional_special,
            rebound: value.rebound,
            reset: value.reset,
            ledge_grab: value.ledge_grab,
            swim: value.swim,
            glide: value.glide,
            dodge: value.dodge,
            shield: value.shield,
        }
    }
}

impl Default for EditableAbilitySet {
    fn default() -> Self {
        ae::AbilitySet::sandbox_all().into()
    }
}

/// Reflected mirror of `ambition_engine::MovementTuning` for live inspector editing.
#[derive(Resource, Reflect, Clone, Copy, Debug)]
#[reflect(Resource)]
pub struct EditableMovementTuning {
    pub gravity: f32,
    pub run_accel: f32,
    pub air_accel: f32,
    pub ground_friction: f32,
    pub air_friction: f32,
    pub max_run_speed: f32,
    pub max_fall_speed: f32,
    pub jump_speed: f32,
    pub double_jump_speed: f32,
    pub wall_jump_x: f32,
    pub wall_slide_speed: f32,
    pub wall_climb_speed: f32,
    pub dash_speed: f32,
    pub dash_time: f32,
    pub dash_cooldown: f32,
    pub dash_buffer: f32,
    pub blink_distance: f32,
    pub precision_blink_distance: f32,
    pub precision_blink_aim_speed: f32,
    pub blink_hold_threshold: f32,
    pub blink_cooldown: f32,
    pub blink_grace_time: f32,
    pub blink_max_downward_speed: f32,
    pub precision_blink_max_downward_speed: f32,
    pub fast_fall_accel: f32,
    pub fast_fall_speed: f32,
    pub glide_fall_speed: f32,
    pub glide_air_accel: f32,
    pub flight_accel: f32,
    pub flight_drag: f32,
    pub flight_terminal_speed: f32,
    pub flight_hover_speed: f32,
    pub flight_hover_hz: f32,
    pub coyote_time: f32,
    pub jump_buffer: f32,
    pub pogo_speed: f32,
    pub slash_recoil: f32,
    pub air_jumps: u8,
    pub dodge_roll_time: f32,
    pub dodge_roll_speed: f32,
    pub dodge_roll_cooldown: f32,
    pub parry_window_time: f32,
}

impl EditableMovementTuning {
    pub fn as_engine(self) -> ae::MovementTuning {
        ae::MovementTuning {
            gravity: self.gravity,
            run_accel: self.run_accel,
            air_accel: self.air_accel,
            ground_friction: self.ground_friction,
            air_friction: self.air_friction,
            max_run_speed: self.max_run_speed,
            max_fall_speed: self.max_fall_speed,
            jump_speed: self.jump_speed,
            double_jump_speed: self.double_jump_speed,
            wall_jump_x: self.wall_jump_x,
            wall_slide_speed: self.wall_slide_speed,
            wall_climb_speed: self.wall_climb_speed,
            dash_speed: self.dash_speed,
            dash_time: self.dash_time,
            dash_cooldown: self.dash_cooldown,
            dash_buffer: self.dash_buffer,
            blink_distance: self.blink_distance,
            precision_blink_distance: self.precision_blink_distance,
            precision_blink_aim_speed: self.precision_blink_aim_speed,
            blink_hold_threshold: self.blink_hold_threshold,
            blink_cooldown: self.blink_cooldown,
            blink_grace_time: self.blink_grace_time,
            blink_max_downward_speed: self.blink_max_downward_speed,
            precision_blink_max_downward_speed: self.precision_blink_max_downward_speed,
            fast_fall_accel: self.fast_fall_accel,
            fast_fall_speed: self.fast_fall_speed,
            glide_fall_speed: self.glide_fall_speed,
            glide_air_accel: self.glide_air_accel,
            flight_accel: self.flight_accel,
            flight_drag: self.flight_drag,
            flight_terminal_speed: self.flight_terminal_speed,
            flight_hover_speed: self.flight_hover_speed,
            flight_hover_hz: self.flight_hover_hz,
            coyote_time: self.coyote_time,
            jump_buffer: self.jump_buffer,
            pogo_speed: self.pogo_speed,
            slash_recoil: self.slash_recoil,
            air_jumps: self.air_jumps,
            dodge_roll_time: self.dodge_roll_time,
            dodge_roll_speed: self.dodge_roll_speed,
            dodge_roll_cooldown: self.dodge_roll_cooldown,
            parry_window_time: self.parry_window_time,
        }
    }
}

impl From<ae::MovementTuning> for EditableMovementTuning {
    fn from(value: ae::MovementTuning) -> Self {
        Self {
            gravity: value.gravity,
            run_accel: value.run_accel,
            air_accel: value.air_accel,
            ground_friction: value.ground_friction,
            air_friction: value.air_friction,
            max_run_speed: value.max_run_speed,
            max_fall_speed: value.max_fall_speed,
            jump_speed: value.jump_speed,
            double_jump_speed: value.double_jump_speed,
            wall_jump_x: value.wall_jump_x,
            wall_slide_speed: value.wall_slide_speed,
            wall_climb_speed: value.wall_climb_speed,
            dash_speed: value.dash_speed,
            dash_time: value.dash_time,
            dash_cooldown: value.dash_cooldown,
            dash_buffer: value.dash_buffer,
            blink_distance: value.blink_distance,
            precision_blink_distance: value.precision_blink_distance,
            precision_blink_aim_speed: value.precision_blink_aim_speed,
            blink_hold_threshold: value.blink_hold_threshold,
            blink_cooldown: value.blink_cooldown,
            blink_grace_time: value.blink_grace_time,
            blink_max_downward_speed: value.blink_max_downward_speed,
            precision_blink_max_downward_speed: value.precision_blink_max_downward_speed,
            fast_fall_accel: value.fast_fall_accel,
            fast_fall_speed: value.fast_fall_speed,
            glide_fall_speed: value.glide_fall_speed,
            glide_air_accel: value.glide_air_accel,
            flight_accel: value.flight_accel,
            flight_drag: value.flight_drag,
            flight_terminal_speed: value.flight_terminal_speed,
            flight_hover_speed: value.flight_hover_speed,
            flight_hover_hz: value.flight_hover_hz,
            coyote_time: value.coyote_time,
            jump_buffer: value.jump_buffer,
            pogo_speed: value.pogo_speed,
            slash_recoil: value.slash_recoil,
            air_jumps: value.air_jumps,
            dodge_roll_time: value.dodge_roll_time,
            dodge_roll_speed: value.dodge_roll_speed,
            dodge_roll_cooldown: value.dodge_roll_cooldown,
            parry_window_time: value.parry_window_time,
        }
    }
}

impl Default for EditableMovementTuning {
    fn default() -> Self {
        ae::MovementTuning::default().into()
    }
}

/// Keep the live player's body collider aligned with the selected development
/// profile after resets / room loads rebuild the player from engine defaults.
pub fn sync_developer_body_profile(
    developer: Res<DeveloperTools>,
    mut player_q: Query<
        &mut crate::player::PlayerMovementAuthority,
        crate::player::PrimaryPlayerOnly,
    >,
) {
    let desired = developer.player_body_profile.size();
    if let Ok(mut authority) = player_q.single_mut() {
        if (authority.player.base_size - desired).length_squared() > 0.01 {
            apply_player_body_profile(&mut authority.player, developer.player_body_profile);
        }
    }
}

/// Apply a player-body profile to the live player while keeping the feet planted.
pub fn apply_player_body_profile(player: &mut ae::Player, profile: PlayerBodyProfile) {
    let new_size = profile.size();
    let old_bottom = player.pos.y + player.size.y * 0.5;
    player.base_size = new_size;
    player.size = new_size;
    player.pos.y = old_bottom - new_size.y * 0.5;
}

/// Apply a movement profile to the reflected tuning resource and refresh live
/// movement resources that depend on the configured number of air jumps.
pub fn apply_movement_profile(
    editable_tuning: &mut EditableMovementTuning,
    profile: MovementProfile,
    authority_player: Option<&mut ambition_engine::Player>,
) {
    let tuning = profile.tuning();
    *editable_tuning = EditableMovementTuning::from(tuning);
    if let Some(player) = authority_player {
        player.refresh_movement_resources(tuning);
    }
}

/// Apply live ability-flag edits without rebuilding the player every frame.
pub fn sync_live_ability_edits(
    player: &mut ae::Player,
    desired: ae::AbilitySet,
    tuning: ae::MovementTuning,
) {
    if player.abilities == desired {
        return;
    }
    player.abilities = desired;
    if !desired.fly {
        player.fly_enabled = false;
    }
    if !desired.blink {
        player.blink_hold_active = false;
        player.blink_hold_timer = 0.0;
        player.blink_aiming = false;
    }
    player.refresh_movement_resources(tuning);
}

/// Reflected, debug-editable player gameplay stats. Surfaced through the
/// `F3` resource inspector so testers can:
///
/// - read live HP / max HP / mana / max mana (fields synced FROM runtime
///   each frame),
/// - rewrite them in-place (clicking the field commits a "set" each
///   frame the value differs from the runtime),
/// - toggle `invincible` to stop incoming damage entirely while testing
///   downstream systems (boss phase, encounter pacing, music swaps).
///
/// The damage multiplier scales the player's outgoing slash damage so
/// testers can one-shot enemies / chip a boss without recompiling.
#[derive(Resource, Reflect, Clone, Copy, Debug)]
#[reflect(Resource)]
pub struct EditablePlayerStats {
    pub health: i32,
    pub max_health: i32,
    pub mana: i32,
    pub max_mana: i32,
    pub slash_damage: i32,
    /// True → all `PlayerDamageEvent`s are ignored before they reach
    /// `handle_player_damage_events`.
    pub invincible: bool,
    /// True → fully refill HP & mana on the next frame's sync.
    pub refill_now: bool,
}

impl EditablePlayerStats {
    pub const DEFAULT_MAX_HEALTH: i32 = 5;
    pub const DEFAULT_MAX_MANA: i32 = 100;
    pub const DEFAULT_SLASH_DAMAGE: i32 = 1;
}

impl Default for EditablePlayerStats {
    fn default() -> Self {
        Self {
            health: Self::DEFAULT_MAX_HEALTH,
            max_health: Self::DEFAULT_MAX_HEALTH,
            mana: Self::DEFAULT_MAX_MANA,
            max_mana: Self::DEFAULT_MAX_MANA,
            slash_damage: Self::DEFAULT_SLASH_DAMAGE,
            invincible: false,
            refill_now: false,
        }
    }
}

/// Last-synced stats snapshot used by `sync_player_stats_with_inspector`
/// to tell user edits apart from runtime drift. Without it, any frame
/// where gameplay damaged HP would see `stats.health != live_hp` and
/// push the stale inspector value back into the runtime, undoing the
/// damage.
#[derive(Default)]
pub struct PlayerStatsSyncSnapshot {
    initialized: bool,
    health: i32,
    max_health: i32,
}

/// Bevy system: keep `EditablePlayerStats` and the live player health
/// in sync, in both directions.
///
/// - When the inspector mutates a stat field, the new value is written
///   onto the ECS player authority.
/// - When gameplay mutates the player (combat damage, pickup heal),
///   the new value is mirrored back to the inspector resource so the
///   field reads the live HP without manual refresh.
/// - `refill_now` is a one-shot button: setting it to true topples HP
///   and mana to max on the next sync, then clears the flag.
pub fn sync_player_stats_with_inspector(
    mut stats: ResMut<EditablePlayerStats>,
    mut snapshot: Local<PlayerStatsSyncSnapshot>,
    mut player_q: Query<
        &mut crate::player::PlayerMovementAuthority,
        crate::player::PrimaryPlayerOnly,
    >,
    mut health_q: Query<&mut crate::player::PlayerHealth, crate::player::PrimaryPlayerOnly>,
) {
    if !snapshot.initialized {
        snapshot.health = stats.health;
        snapshot.max_health = stats.max_health;
        snapshot.initialized = true;
    }
    if stats.refill_now {
        stats.health = stats.max_health.max(1);
        stats.mana = stats.max_mana.max(0);
        stats.refill_now = false;
    }
    let user_changed_max = stats.max_health != snapshot.max_health;
    let user_changed_hp = stats.health != snapshot.health;
    if let Ok(mut health) = health_q.single_mut() {
        if user_changed_max {
            health.health = ae::Health::new(stats.max_health.max(1));
            health.health.current = stats.health.clamp(0, stats.max_health.max(1));
        } else if user_changed_hp {
            health.health.current = stats.health.clamp(0, health.health.max.max(1));
        } else {
            stats.health = health.health.current;
            stats.max_health = health.health.max;
        }
    }
    snapshot.health = stats.health;
    snapshot.max_health = stats.max_health;
    // Mana now lives on `Player::mana` (engine `ResourceMeter`); the
    // inspector still surfaces i32 fields for player-friendly editing
    // and the conversion happens at this boundary. Future
    // mana-consuming abilities call `try_spend` directly on the meter.
    let max_mana = stats.max_mana.max(0);
    // Combat tuning + invincibility now live on `Player` (engine-side)
    // so per-player state is engine state, not sandbox state.
    if let Ok(mut authority) = player_q.single_mut() {
        authority.player.mana.max = max_mana as f32;
        authority.player.mana.current = stats.mana.clamp(0, max_mana) as f32;
        authority.player.damage_multiplier = stats.slash_damage.max(1);
        authority.player.invincible = stats.invincible;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editable_ability_set_round_trips_through_engine() {
        // Default → engine → editable should equal the original.
        let original = EditableAbilitySet::default();
        let engine = original.as_engine();
        let restored = EditableAbilitySet::from(engine);
        assert_eq!(original.move_horizontal, restored.move_horizontal);
        assert_eq!(original.glide, restored.glide);
        assert_eq!(original.swim, restored.swim);
        assert_eq!(original.ledge_grab, restored.ledge_grab);
    }

    #[test]
    fn editable_movement_tuning_round_trips_through_engine() {
        let original = EditableMovementTuning::default();
        let engine = original.as_engine();
        let restored = EditableMovementTuning::from(engine);
        // Spot-check a handful of fields including the recently-added
        // glide tuning.
        assert!((original.gravity - restored.gravity).abs() < 1e-3);
        assert!((original.jump_speed - restored.jump_speed).abs() < 1e-3);
        assert!((original.glide_fall_speed - restored.glide_fall_speed).abs() < 1e-3);
        assert!((original.glide_air_accel - restored.glide_air_accel).abs() < 1e-3);
        assert_eq!(original.air_jumps, restored.air_jumps);
    }

    #[test]
    fn editable_player_stats_default_matches_constants() {
        let s = EditablePlayerStats::default();
        assert_eq!(s.health, EditablePlayerStats::DEFAULT_MAX_HEALTH);
        assert_eq!(s.max_health, EditablePlayerStats::DEFAULT_MAX_HEALTH);
        assert_eq!(s.mana, EditablePlayerStats::DEFAULT_MAX_MANA);
        assert_eq!(s.max_mana, EditablePlayerStats::DEFAULT_MAX_MANA);
        assert_eq!(s.slash_damage, EditablePlayerStats::DEFAULT_SLASH_DAMAGE);
        assert!(!s.invincible);
        assert!(!s.refill_now);
    }

    #[test]
    fn debug_view_presets_drive_overlay_intent() {
        let mut tools = DeveloperTools::default();
        tools.apply_debug_view_mode(DebugViewMode::Collision, true);
        assert_eq!(tools.debug_view_mode, DebugViewMode::Collision);
        assert!(tools.show_world_blocks);
        assert!(tools.show_player_hitbox);
        assert!(tools.show_feature_hitboxes);
        assert!(tools.fill_debug_boxes);
        assert_eq!(tools.debug_art_mode, DebugArtMode::Hidden);

        tools.apply_debug_view_mode(DebugViewMode::Authoring, true);
        assert_eq!(tools.debug_view_mode, DebugViewMode::Authoring);
        assert!(tools.show_room_bounds);
        assert!(tools.show_loading_zones);
        assert!(!tools.show_micro_grid);
        assert!(!tools.show_world_blocks);
        assert!(!tools.fill_debug_boxes);
        assert_eq!(tools.debug_art_mode, DebugArtMode::Normal);
    }

    #[test]
    fn debug_art_mode_is_single_source_for_sprite_overrides() {
        let mut tools = DeveloperTools::default();
        tools.apply_debug_art_mode(DebugArtMode::Placeholder);
        assert!(tools.placeholder_sprites);
        assert!(!tools.hide_sprites);

        tools.apply_debug_art_mode(DebugArtMode::Hidden);
        assert!(!tools.placeholder_sprites);
        assert!(tools.hide_sprites);

        tools.apply_debug_art_mode(DebugArtMode::Normal);
        assert!(!tools.placeholder_sprites);
        assert!(!tools.hide_sprites);
    }
}
