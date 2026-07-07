//! Live-editable tuning resources (abilities, movement, player stats) + the
//! systems that mirror their edits into the running player clusters.

use super::*;
use ambition_engine_core as ae;
use bevy::prelude::*;

/// Reflected mirror of `ambition_engine_core::AbilitySet` for live inspector editing.
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

/// Reflected mirror of `ambition_engine_core::MovementTuning` for live inspector editing.
#[derive(Resource, Reflect, Clone, Copy, Debug)]
#[reflect(Resource)]
pub struct EditableMovementTuning {
    pub gravity: f32,
    pub run_accel: f32,
    pub air_accel: f32,
    pub ground_friction: f32,
    pub air_friction: f32,
    pub air_stop_assist: f32,
    pub carried_decay: f32,
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
    // Ledge momentum-carry boost. Seconds-after-grab during which a
    // getup option can claim incoming momentum; gains scale incoming
    // velocity into the boost; caps clamp the post-gain magnitude.
    // Set `ledge_boost_window` to 0.0 to disable the mechanic.
    pub ledge_boost_window: f32,
    pub ledge_boost_x_gain: f32,
    pub ledge_boost_y_gain: f32,
    pub ledge_boost_x_cap: f32,
    pub ledge_boost_y_cap: f32,
    /// Shortens the climb / roll / attack transition when momentum
    /// was carried. 1.0 = full momentum roughly halves the duration.
    /// 0.0 disables the speedup.
    pub ledge_boost_getup_speedup_gain: f32,
}

impl EditableMovementTuning {
    pub fn as_engine(self) -> ae::MovementTuning {
        ae::MovementTuning {
            gravity: self.gravity,
            // Runtime-overridden each frame from the world GravityField; default
            // upright here.
            gravity_sign: 1.0,
            gravity_dir: ae::Vec2::new(0.0, 1.0),
            // Default; the live control preference is applied per-frame alongside
            // `gravity_dir` (see player_tick / sim_systems `apply_gravity_dir`).
            movement_frame_mode: ae::InputFrameMode::DEFAULT_MOVEMENT,
            run_accel: self.run_accel,
            air_accel: self.air_accel,
            ground_friction: self.ground_friction,
            air_friction: self.air_friction,
            air_stop_assist: self.air_stop_assist,
            carried_decay: self.carried_decay,
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
            // The editable dev tuning drives the PLAYER body (smoothed flight);
            // direct-velocity is a per-body opt-in the boss sets in its own tuning.
            flight_direct_velocity: false,
            coyote_time: self.coyote_time,
            jump_buffer: self.jump_buffer,
            pogo_speed: self.pogo_speed,
            slash_recoil: self.slash_recoil,
            air_jumps: self.air_jumps,
            dodge_roll_time: self.dodge_roll_time,
            dodge_roll_speed: self.dodge_roll_speed,
            dodge_roll_cooldown: self.dodge_roll_cooldown,
            parry_window_time: self.parry_window_time,
            ledge_momentum: ae::LedgeMomentumTuning {
                window: self.ledge_boost_window,
                x_gain: self.ledge_boost_x_gain,
                y_gain: self.ledge_boost_y_gain,
                x_cap: self.ledge_boost_x_cap,
                y_cap: self.ledge_boost_y_cap,
                getup_speedup_gain: self.ledge_boost_getup_speedup_gain,
            },
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
            air_stop_assist: value.air_stop_assist,
            carried_decay: value.carried_decay,
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
            ledge_boost_window: value.ledge_momentum.window,
            ledge_boost_x_gain: value.ledge_momentum.x_gain,
            ledge_boost_y_gain: value.ledge_momentum.y_gain,
            ledge_boost_x_cap: value.ledge_momentum.x_cap,
            ledge_boost_y_cap: value.ledge_momentum.y_cap,
            ledge_boost_getup_speedup_gain: value.ledge_momentum.getup_speedup_gain,
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
        (
            &mut ambition_engine_core::BodyKinematics,
            &mut ambition_engine_core::BodyBaseSize,
        ),
        ambition_platformer_primitives::markers::PrimaryPlayerOnly,
    >,
) {
    let desired = developer.player_body_profile.size();
    if let Ok((mut kinematics, mut base_size)) = player_q.single_mut() {
        if (base_size.base_size - desired).length_squared() > 0.01 {
            // Inline the body-profile resize directly on the cluster
            // components (formerly `apply_player_body_profile(&mut Player, ...)`).
            let new_size = developer.player_body_profile.size();
            let old_bottom = kinematics.pos.y + kinematics.size.y * 0.5;
            base_size.base_size = new_size;
            kinematics.size = new_size;
            kinematics.pos.y = old_bottom - new_size.y * 0.5;
        }
    }
}

/// Apply a player-body profile to the live player while keeping the feet planted.
/// Callers pass the player's `BodyKinematics` directly.
///
/// This updates the live collider `size` + `pos`; the player's authored
/// standing baseline (`BodyBaseSize`) is reconciled to the selected profile by
/// [`sync_developer_body_profile`], which runs every frame, so the menu caller
/// does not need to hold a `&mut BodyBaseSize`.
pub fn apply_player_body_profile(
    kinematics: &mut ambition_engine_core::BodyKinematics,
    profile: PlayerBodyProfile,
) {
    let new_size = profile.size();
    let old_bottom = kinematics.pos.y + kinematics.size.y * 0.5;
    kinematics.size = new_size;
    kinematics.pos.y = old_bottom - new_size.y * 0.5;
}

/// Apply a movement profile to the reflected tuning resource and refresh live
/// movement resources that depend on the configured number of air jumps.
///
/// `live_movement_refs` is `Some((abilities, dash, jump))` when there is a
/// live player to refresh; `None` (e.g. unit tests, no player yet) skips the
/// refresh and only updates `editable_tuning`.
pub fn apply_movement_profile(
    editable_tuning: &mut EditableMovementTuning,
    profile: MovementProfile,
    live_movement_refs: Option<(
        &ambition_engine_core::BodyAbilities,
        &mut ambition_engine_core::BodyDashState,
        &mut ambition_engine_core::BodyJumpState,
    )>,
) {
    let tuning = profile.tuning();
    *editable_tuning = EditableMovementTuning::from(tuning);
    if let Some((abilities, dash, jump)) = live_movement_refs {
        ae::refresh_movement_resources_clusters(abilities, dash, jump, tuning);
    }
}

/// Apply live ability-flag edits without rebuilding the player every frame.
///
/// Mutates `BodyAbilities` + side-effects on `BodyFlightState`,
/// `BodyBlinkState`, `BodyDashState`, and `BodyJumpState` directly.
pub fn sync_live_ability_edits_clusters(
    abilities: &mut ambition_engine_core::BodyAbilities,
    flight: &mut ambition_engine_core::BodyFlightState,
    blink: &mut ambition_engine_core::BodyBlinkState,
    dash: &mut ambition_engine_core::BodyDashState,
    jump: &mut ambition_engine_core::BodyJumpState,
    desired: ae::AbilitySet,
    tuning: ae::MovementTuning,
) {
    if abilities.abilities == desired {
        return;
    }
    abilities.abilities = desired;
    if !desired.fly {
        flight.fly_enabled = false;
    }
    if !desired.blink {
        blink.hold_active = false;
        blink.hold_timer = 0.0;
        blink.aiming = false;
    }
    // Inline `refresh_movement_resources(tuning)` for the cluster path.
    dash.charges_available = desired.dash_charge_count();
    jump.air_jumps_available = desired.air_jump_count(tuning.air_jumps);
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
    /// True → all `HitEvent`s are ignored before they reach
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
        (
            &mut ambition_engine_core::BodyMana,
            &mut ambition_engine_core::BodyOffense,
        ),
        ambition_platformer_primitives::markers::PrimaryPlayerOnly,
    >,
    mut health_q: Query<
        &mut ambition_characters::actor::BodyHealth,
        ambition_platformer_primitives::markers::PrimaryPlayerOnly,
    >,
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
            health.health = ambition_characters::actor::Health::new(stats.max_health.max(1));
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
    if let Ok((mut mana, mut offense)) = player_q.single_mut() {
        mana.meter.max = max_mana as f32;
        mana.meter.current = stats.mana.clamp(0, max_mana) as f32;
        offense.damage_multiplier = stats.slash_damage.max(1);
        offense.invincible = stats.invincible;
    }
}
