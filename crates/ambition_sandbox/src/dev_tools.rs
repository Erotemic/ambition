//! Developer-facing tuning and inspection tools.
//!
//! This module is intentionally sandbox-side: it is allowed to depend on Bevy
//! reflection and inspector UI crates, while `ambition_engine` stays focused on
//! backend-neutral movement/collision logic. The reflected resources here mirror
//! engine data so live tuning can happen without forcing Bevy dependencies into
//! the reusable crate.

use ambition_engine as ae;
use bevy::prelude::*;

use crate::{
    BLINK_HOLD_SLOW_SCALE, BULLET_TIME_SCALE, DEBUG_SLOWMO_SCALE, DOWN_DOUBLE_TAP_WINDOW,
    TIME_RAMP_DOWN_RATE, TIME_RAMP_UP_RATE, UP_DOUBLE_TAP_WINDOW,
};

/// Top-level switches for debug UI and gizmo layers.
#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct DeveloperTools {
    /// Show the reflected resource inspector windows.
    pub inspector_visible: bool,
    /// Show the heavier full-world entity/resource inspector.
    pub world_inspector_visible: bool,
    /// Master switch for Bevy gizmo overlays. `F1` still controls the old HUD/debug mode.
    pub gizmos_enabled: bool,
    pub show_hud: bool,
    pub show_room_bounds: bool,
    pub show_world_blocks: bool,
    pub show_loading_zones: bool,
    pub show_player_hitbox: bool,
    pub show_player_vectors: bool,
    pub show_blink_preview: bool,
    pub show_combat_preview: bool,
    pub show_feature_hitboxes: bool,
    pub show_moving_platform: bool,
    pub show_dummies: bool,
    pub show_rebound_vectors: bool,
}

impl Default for DeveloperTools {
    fn default() -> Self {
        Self {
            inspector_visible: true,
            world_inspector_visible: false,
            gizmos_enabled: true,
            show_hud: true,
            show_room_bounds: true,
            show_world_blocks: false,
            show_loading_zones: true,
            show_player_hitbox: true,
            show_player_vectors: true,
            show_blink_preview: true,
            show_combat_preview: true,
            show_feature_hitboxes: true,
            show_moving_platform: true,
            show_dummies: true,
            show_rebound_vectors: true,
        }
    }
}

pub fn inspector_visible(tools: Res<DeveloperTools>) -> bool {
    tools.inspector_visible
}

pub fn world_inspector_visible(tools: Res<DeveloperTools>) -> bool {
    tools.world_inspector_visible
}

/// Sandbox-only time/input feel constants that used to be compile-time values.
#[derive(Resource, Reflect, Clone, Copy, Debug)]
#[reflect(Resource)]
pub struct SandboxFeelTuning {
    pub bullet_time_scale: f32,
    pub blink_hold_slow_scale: f32,
    pub debug_slowmo_scale: f32,
    pub time_ramp_down_rate: f32,
    pub time_ramp_up_rate: f32,
    pub down_double_tap_window: f32,
    pub up_double_tap_window: f32,
    pub interaction_buffer_time: f32,
    pub attack_hitstop_time: f32,
    pub reset_flash_time: f32,
    pub edge_transition_cooldown: f32,
    pub door_transition_cooldown: f32,
    pub edge_transition_flash: f32,
    pub door_transition_flash: f32,
}

impl Default for SandboxFeelTuning {
    fn default() -> Self {
        Self {
            bullet_time_scale: BULLET_TIME_SCALE,
            blink_hold_slow_scale: BLINK_HOLD_SLOW_SCALE,
            debug_slowmo_scale: DEBUG_SLOWMO_SCALE,
            time_ramp_down_rate: TIME_RAMP_DOWN_RATE,
            time_ramp_up_rate: TIME_RAMP_UP_RATE,
            down_double_tap_window: DOWN_DOUBLE_TAP_WINDOW,
            up_double_tap_window: UP_DOUBLE_TAP_WINDOW,
            interaction_buffer_time: 0.120,
            attack_hitstop_time: 0.055,
            reset_flash_time: 0.18,
            edge_transition_cooldown: 0.14,
            door_transition_cooldown: 0.16,
            edge_transition_flash: 0.24,
            door_transition_flash: 0.24,
        }
    }
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
        }
    }
}

impl Default for EditableMovementTuning {
    fn default() -> Self {
        ae::MovementTuning::default().into()
    }
}

/// Apply live ability-flag edits without rebuilding the player every frame.
pub fn sync_live_ability_edits(
    runtime: &mut crate::SandboxRuntime,
    desired: ae::AbilitySet,
    tuning: ae::MovementTuning,
) {
    if runtime.player.abilities == desired {
        return;
    }
    runtime.player.abilities = desired;
    if !desired.fly {
        runtime.player.fly_enabled = false;
    }
    if !desired.blink {
        runtime.player.blink_hold_active = false;
        runtime.player.blink_hold_timer = 0.0;
        runtime.player.blink_aiming = false;
    }
    runtime.player.refresh_movement_resources(tuning);
}
