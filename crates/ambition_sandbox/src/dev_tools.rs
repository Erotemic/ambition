//! Developer-facing tuning and inspection tools.
//!
//! This module is intentionally sandbox-side: it is allowed to depend on Bevy
//! reflection and inspector UI crates, while `ambition_engine` stays focused on
//! backend-neutral movement/collision logic. The reflected resources here mirror
//! engine data so live tuning can happen without forcing Bevy dependencies into
//! the reusable crate.

use ambition_engine as ae;
use bevy::prelude::*;

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
    /// Keep the HUD compact now that the inspector exposes detailed live state.
    pub compact_hud: bool,
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
    pub show_rebound_vectors: bool,
    /// Toggle a zoomed-out camera for inspecting large or stitched active areas.
    pub overview_camera: bool,
    /// Orthographic scale used while overview camera is enabled.
    pub overview_camera_scale: f32,
}

impl Default for DeveloperTools {
    fn default() -> Self {
        Self {
            inspector_visible: false,
            world_inspector_visible: false,
            gizmos_enabled: true,
            show_hud: true,
            compact_hud: true,
            show_room_bounds: true,
            // Default ON so collision rects are visible from the start —
            // sprite art often has large transparent regions that cover
            // less of the actual collision box than the eye reads.
            show_world_blocks: true,
            show_loading_zones: true,
            show_player_hitbox: true,
            show_player_vectors: true,
            show_blink_preview: true,
            show_combat_preview: true,
            show_feature_hitboxes: true,
            show_health_bars: true,
            show_moving_platform: true,
            show_rebound_vectors: true,
            overview_camera: false,
            overview_camera_scale: 2.35,
        }
    }
}

pub fn inspector_visible(tools: Res<DeveloperTools>) -> bool {
    tools.inspector_visible
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

/// Bevy system: keep `EditablePlayerStats` and `SandboxRuntime`
/// player health in sync, in both directions.
///
/// - When the inspector mutates a stat field, the new value is written
///   onto the runtime.
/// - When gameplay mutates the runtime (combat damage, pickup heal),
///   the new value is mirrored back to the inspector resource so the
///   field reads the live HP without manual refresh.
/// - `refill_now` is a one-shot button: setting it to true topples HP
///   and mana to max on the next sync, then clears the flag.
///
/// Mana isn't yet a real engine resource (the player sim doesn't
/// consume it). It is intentionally on the inspector now so future
/// abilities (precision blink cost, special attack) can read from
/// `SandboxRuntime` without adding a new editor.
pub fn sync_player_stats_with_inspector(
    mut stats: ResMut<EditablePlayerStats>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut snapshot: Local<PlayerStatsSyncSnapshot>,
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
    let live_hp = runtime.player_health.current;
    let live_max = runtime.player_health.max;
    let user_changed_max = stats.max_health != snapshot.max_health;
    let user_changed_hp = stats.health != snapshot.health;
    if user_changed_max {
        runtime.player_health = ae::Health::new(stats.max_health.max(1));
        runtime.player_health.current = stats.health.clamp(0, stats.max_health.max(1));
    } else if user_changed_hp {
        runtime.player_health.current = stats.health.clamp(0, live_max.max(1));
    } else {
        // Mirror runtime back into the inspector field so HP edits
        // happening from gameplay show up live.
        stats.health = live_hp;
        stats.max_health = live_max;
    }
    snapshot.health = stats.health;
    snapshot.max_health = stats.max_health;
    // Mana now lives on `Player::mana` (engine `ResourceMeter`); the
    // inspector still surfaces i32 fields for player-friendly editing
    // and the conversion happens at this boundary. Future
    // mana-consuming abilities call `try_spend` directly on the meter.
    let max_mana = stats.max_mana.max(0);
    runtime.player.mana.max = max_mana as f32;
    runtime.player.mana.current = stats.mana.clamp(0, max_mana) as f32;
    // Combat tuning + invincibility now live on `Player` (engine-side)
    // so per-player state is engine state, not sandbox state.
    runtime.player.damage_multiplier = stats.slash_damage.max(1);
    runtime.player.invincible = stats.invincible;
}
