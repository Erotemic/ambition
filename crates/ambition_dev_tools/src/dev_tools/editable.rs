//! Live-editable tuning resources (abilities, movement, player stats) + the
//! systems that mirror their edits into the running player clusters.

use super::*;
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

/// Reflected mirror of `ambition_platformer2d_core::AbilitySet` for live inspector editing.
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
    pub fly_toggle: bool,
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
    pub grab: bool,
    /// World interaction (talk / open). A dev toggle over the same ability the
    /// character's grant list decides — see `AbilitySet::interact`.
    pub interact: bool,
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
            fly_toggle: self.fly_toggle,
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
            grab: self.grab,
            interact: self.interact,
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
            fly_toggle: value.fly_toggle,
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
            grab: value.grab,
            interact: value.interact,
        }
    }
}

impl Default for EditableAbilitySet {
    fn default() -> Self {
        ae::AbilitySet::sandbox_all().into()
    }
}

/// Reflected mirror of `ambition_platformer2d_core::MovementTuning` for live inspector editing.
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
    /// `0.0` inherits [`Self::max_run_speed`]; see
    /// [`ae::MovementTuning::max_air_speed`].
    pub max_air_speed: f32,
    /// [`ae::MovementTuning::run_commit_frac`] — where a walk becomes a run.
    pub run_commit_frac: f32,
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
    /// `0.0` is an instant leap; see [`ae::MovementTuning::jump_squat_time`].
    pub jump_squat_time: f32,
    pub pogo_speed: f32,
    pub slash_recoil: f32,
    pub air_jumps: u8,
    pub dodge_roll_time: f32,
    pub dodge_roll_speed: f32,
    pub dodge_roll_cooldown: f32,
    /// Recovery after a ground roll — the roll comes to rest and owes this
    /// before the body can act. See `AbilityTuning::dodge_roll_endlag`.
    pub dodge_roll_endlag: f32,
    /// The aerial evade, exposed here for the same reason the roll is: it is a
    /// feel knob, and a feel knob the inspector cannot reach is one nobody tunes.
    pub air_dodge_time: f32,
    pub air_dodge_speed: f32,
    pub air_dodge_endlag: f32,
    /// See [`ae::TraversalAbilityTuning::tumble_speed`] — 0.0 means this body
    /// has no floor game.
    pub tumble_speed: f32,
    /// See [`ae::MovementTuning::sdi_step`] — 0.0 means this body cannot
    /// influence its way out of a combo, which is every body but a fighter.
    pub sdi_step: f32,
    /// One displacement per hit, paid when hitlag ends. A feel slider.
    pub asdi_step: f32,
    /// The jab lock, carried through an edit rather than edited: how weak a hit
    /// pins a downed body, and how many pins before it resets.
    pub jab_lock_speed: f32,
    pub jab_lock_limit: u8,
    /// See [`ae::MovementTuning::spot_dodge_time`] — 0.0 means the grounded
    /// evade is always the roll.
    pub spot_dodge_time: f32,
    /// CARRIED, NOT EDITED — see the `parry_timing` line in the round trip
    /// below. A parry-timing swap is a rules DECLARATION about which game a
    /// stage reproduces, and rebuilding it from `default()` on an unrelated
    /// edit silently replaced whatever the stage had declared.
    ///
    /// ⭐ `reflect(ignore)` IS THE POINT, not a workaround: this struct is
    /// reflected to BUILD THE SLIDERS, so a field the inspector must not offer
    /// is a field reflection must not see. Carried and invisible is exactly the
    /// contract the comment above asks for.
    #[reflect(ignore)]
    pub parry_timing: ae::ParryTiming,
    /// The MATCH's evade-staling and tech rules, carried for the same reason
    /// and hidden for the same reason. A fighter's authored tuning sets these
    /// in one preset beside `parry_timing`; the editor must return them
    /// unchanged rather than zero.
    #[reflect(ignore)]
    pub dodge_stale_step: f32,
    #[reflect(ignore)]
    pub dodge_stale_floor: f32,
    #[reflect(ignore)]
    pub dodge_stale_recovery: f32,
    #[reflect(ignore)]
    pub untechable_launch_speed: f32,
    #[reflect(ignore)]
    pub evade_cancel_tail: f32,
    /// The ground-movement PHASES and the crouch cost — a rules declaration
    /// like the six above, carried for the same reason and hidden for the same
    /// reason.
    #[reflect(ignore)]
    pub crouch_speed_frac: f32,
    #[reflect(ignore)]
    pub initial_dash_time: f32,
    #[reflect(ignore)]
    pub initial_dash_speed: f32,
    #[reflect(ignore)]
    pub turnaround_time: f32,
    #[reflect(ignore)]
    pub teeter_margin: f32,
    pub parry_window_time: f32,
    /// Shield integrity, its drain/regen rates, its cost per blocked point, and
    /// the dizzy a break costs. `shield_max_health = 0.0` = unlimited guard.
    pub shield_max_health: f32,
    pub shield_drain_per_second: f32,
    pub shield_regen_per_second: f32,
    pub shield_damage_scale: f32,
    pub shield_break_stun_time: f32,
    pub shield_stun_per_damage: f32,
    pub shield_pushback_per_damage: f32,
    pub shield_min_coverage: f32,
    /// How far a held stick shifts the guard, as a fraction of half-height. A
    /// feel slider: how much a tilt is worth against how much it gives up.
    pub shield_tilt_range: f32,
    /// Shield-drop lag: what letting a guard down costs, in seconds. A feel
    /// slider like the rates above it.
    pub shield_drop_lag: f32,
    /// The OUT-OF-SHIELD rule, carried through an edit rather than edited.
    ///
    /// Which actions a raised guard permits is a rules DECLARATION about which
    /// game a stage reproduces — the same class of fact as `parry_timing` above
    /// — so it is not a slider. ⭐ but it is CARRIED, not defaulted: rebuilding
    /// the tuning from sliders alone would silently delete the stage's rule the
    /// first time anybody dragged an unrelated one.
    /// ⛔ `reflect(ignore)` so the editor cannot draw it as a draggable value:
    /// the foundation crate carries no reflection and a rule is not a slider.
    #[reflect(ignore)]
    pub shield_out_of_shield: Option<ae::OutOfShield>,
    /// Carried, not edited — see the note at the construction site.
    pub shield_air_guard: bool,
    /// Carried through an edit, not edited: see `ShieldTuning::platform_drop`.
    pub shield_platform_drop: bool,
    /// Footstool: the hop, the shove, the stun, and the head band. 0.0 rise = off.
    pub footstool_rise_speed: f32,
    pub footstool_press_speed: f32,
    pub footstool_flinch_time: f32,
    pub footstool_air_tumble_time: f32,
    pub footstool_stomper_invuln: f32,
    pub footstool_band: f32,
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
            // The current inspector edits the historical responsive profile.
            // Authored character profiles may select the newer composable laws
            // without being flattened through this legacy control surface.
            horizontal_law: ae::AxisHorizontalLaw::Responsive,
            jump_law: ae::AxisJumpLaw::VelocityCut,
            // Runtime-overridden each frame from the world GravityField; default
            // upright here.
            // Default; the live control preference is applied per-frame alongside
            // `gravity_dir` (see player_tick / sim_systems `apply_gravity_dir`).
            run_accel: self.run_accel,
            air_accel: self.air_accel,
            ground_friction: self.ground_friction,
            air_friction: self.air_friction,
            air_stop_assist: self.air_stop_assist,
            carried_decay: self.carried_decay,
            max_run_speed: self.max_run_speed,
            max_air_speed: self.max_air_speed,
            run_commit_frac: self.run_commit_frac,
            // ⛔ NOT AN F3 SLIDER — and carried rather than defaulted, for the
            // reason the dash phases below are. What a crouch costs is a MATCH
            // rule the stage declares (`MatchBody::crouch_speed_frac`); the
            // ruleset composing its own over the default only works when there
            // IS a default to compose over, and it is not this projection's job
            // to decide there was none.
            crouch_speed_frac: self.crouch_speed_frac,
            // ⛔⛔ "CARRIED, NOT EDITED" — AND IT WAS NOT CARRIED. This said the
            // right thing and did the opposite: reading `DEFAULT_TUNING`
            // REPLACES whatever the stage declared with the engine's answer, so
            // a game with its own dash phases lost them to an unrelated slider.
            // Carried means carried; the ONLY thing "not edited" adds is that no
            // inspector row offers it, which `reflect(ignore)` is for.
            initial_dash_time: self.initial_dash_time,
            initial_dash_speed: self.initial_dash_speed,
            turnaround_time: self.turnaround_time,
            teeter_margin: self.teeter_margin,
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
            flight_invariant_speed: None,
            coyote_time: self.coyote_time,
            jump_buffer: self.jump_buffer,
            jump_squat_time: self.jump_squat_time,
            pogo_speed: self.pogo_speed,
            slash_recoil: self.slash_recoil,
            air_jumps: self.air_jumps,
            dodge_roll_time: self.dodge_roll_time,
            dodge_roll_speed: self.dodge_roll_speed,
            dodge_roll_cooldown: self.dodge_roll_cooldown,
            dodge_roll_endlag: self.dodge_roll_endlag,
            // ⛔⛔ THE SAME HOLE `parry_timing` HAD, five fields wide, and the
            // audit this row asked for. These are MATCH rules rather than
            // per-body dev knobs — the editor tunes one body, and staling is
            // about the option — but a fighter's tuning genuinely CARRIES them
            // (`abilities.rs` authors `dodge_stale_step: 0.25`,
            // `untechable_launch_speed: 1400.0` and the rest beside
            // `parry_timing: OnRaise`, in one preset). Zeroing them on the way
            // back out deleted the match's rules because somebody dragged an
            // unrelated slider.
            dodge_stale_step: self.dodge_stale_step,
            dodge_stale_floor: self.dodge_stale_floor,
            dodge_stale_recovery: self.dodge_stale_recovery,
            untechable_launch_speed: self.untechable_launch_speed,
            evade_cancel_tail: self.evade_cancel_tail,
            air_dodge_time: self.air_dodge_time,
            air_dodge_speed: self.air_dodge_speed,
            air_dodge_endlag: self.air_dodge_endlag,
            tumble_speed: self.tumble_speed,
            sdi_step: self.sdi_step,
            asdi_step: self.asdi_step,
            jab_lock_speed: self.jab_lock_speed,
            jab_lock_limit: self.jab_lock_limit,
            spot_dodge_time: self.spot_dodge_time,
            // ⛔⛔ NOT EDITABLE IS NOT THE SAME AS NOT CARRIED, and this line
            // used to do the second while claiming the first. A parry-timing
            // swap is a rules DECLARATION about which game a stage reproduces —
            // so it is not a slider, and rebuilding it from `default()` on ANY
            // edit DELETED the declaration: touch an unrelated knob and the
            // stage's ruleset was gone, silently.
            //
            // ⭐ `air_guard` two dozen lines down carries the same rule and got
            // it right — *"NOT editable, for the same reason `parry_timing` is
            // not"* — while doing the opposite thing. Two fields, one stated
            // rule, opposite behaviour; this is now the one `air_guard` says it
            // is.
            parry_timing: self.parry_timing,
            parry_window_time: self.parry_window_time,
            shield: ae::ShieldTuning {
                max_health: self.shield_max_health,
                drain_per_second: self.shield_drain_per_second,
                regen_per_second: self.shield_regen_per_second,
                damage_scale: self.shield_damage_scale,
                break_stun_time: self.shield_break_stun_time,
                stun_per_damage: self.shield_stun_per_damage,
                pushback_per_damage: self.shield_pushback_per_damage,
                min_coverage: self.shield_min_coverage,
                tilt_range: self.shield_tilt_range,
                drop_lag: self.shield_drop_lag,
                out_of_shield: self.shield_out_of_shield,
                // NOT editable, for the same reason `parry_timing` is not:
                // whether a guard exists in the air is a rules DECLARATION about
                // which game a stage reproduces, not a slider.
                air_guard: self.shield_air_guard,
                platform_drop: self.shield_platform_drop,
            },
            footstool: ae::FootstoolTuning {
                rise_speed: self.footstool_rise_speed,
                press_speed: self.footstool_press_speed,
                flinch_time: self.footstool_flinch_time,
                air_tumble_time: self.footstool_air_tumble_time,
                stomper_invuln: self.footstool_stomper_invuln,
                band: self.footstool_band,
            },
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
            max_air_speed: value.max_air_speed,
            run_commit_frac: value.run_commit_frac,
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
            jump_squat_time: value.jump_squat_time,
            pogo_speed: value.pogo_speed,
            slash_recoil: value.slash_recoil,
            air_jumps: value.air_jumps,
            dodge_roll_time: value.dodge_roll_time,
            dodge_roll_speed: value.dodge_roll_speed,
            dodge_roll_cooldown: value.dodge_roll_cooldown,
            dodge_roll_endlag: value.dodge_roll_endlag,
            air_dodge_time: value.air_dodge_time,
            air_dodge_speed: value.air_dodge_speed,
            air_dodge_endlag: value.air_dodge_endlag,
            tumble_speed: value.tumble_speed,
            sdi_step: value.sdi_step,
            asdi_step: value.asdi_step,
            jab_lock_speed: value.jab_lock_speed,
            jab_lock_limit: value.jab_lock_limit,
            spot_dodge_time: value.spot_dodge_time,
            parry_timing: value.parry_timing,
            dodge_stale_step: value.dodge_stale_step,
            dodge_stale_floor: value.dodge_stale_floor,
            dodge_stale_recovery: value.dodge_stale_recovery,
            untechable_launch_speed: value.untechable_launch_speed,
            evade_cancel_tail: value.evade_cancel_tail,
            crouch_speed_frac: value.crouch_speed_frac,
            initial_dash_time: value.initial_dash_time,
            initial_dash_speed: value.initial_dash_speed,
            turnaround_time: value.turnaround_time,
            teeter_margin: value.teeter_margin,
            parry_window_time: value.parry_window_time,
            shield_max_health: value.shield.max_health,
            shield_drain_per_second: value.shield.drain_per_second,
            shield_regen_per_second: value.shield.regen_per_second,
            shield_damage_scale: value.shield.damage_scale,
            shield_break_stun_time: value.shield.break_stun_time,
            shield_stun_per_damage: value.shield.stun_per_damage,
            shield_pushback_per_damage: value.shield.pushback_per_damage,
            shield_min_coverage: value.shield.min_coverage,
            shield_tilt_range: value.shield.tilt_range,
            shield_drop_lag: value.shield.drop_lag,
            shield_out_of_shield: value.shield.out_of_shield,
            shield_air_guard: value.shield.air_guard,
            shield_platform_drop: value.shield.platform_drop,
            footstool_rise_speed: value.footstool.rise_speed,
            footstool_press_speed: value.footstool.press_speed,
            footstool_flinch_time: value.footstool.flinch_time,
            footstool_air_tumble_time: value.footstool.air_tumble_time,
            footstool_stomper_invuln: value.footstool.stomper_invuln,
            footstool_band: value.footstool.band,
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
            &mut ambition_platformer2d_core::BodyKinematics,
            &mut ambition_platformer2d_core::BodyBaseSize,
        ),
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    mut last_applied: Local<Option<PlayerBodyProfile>>,
) {
    // Apply only when the developer changes the selected profile. Reapplying
    // every frame would make the dev tool authoritative over legitimate
    // gameplay-driven body-size changes.
    let desired = developer.player_body_profile.size();
    let selection_changed = *last_applied != Some(developer.player_body_profile);
    if !selection_changed {
        return;
    }
    *last_applied = Some(developer.player_body_profile);
    if let Ok((mut kinematics, mut base_size)) = player_q.single_mut() {
        if (base_size.base_size - desired).length_squared() > 0.01 {
            // Resize the body while preserving the planted-foot position.
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
    kinematics: &mut ambition_platformer2d_core::BodyKinematics,
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
/// `live_movement_refs` is `Some((abilities, dash, jump, dodge))` when there is a
/// live player to refresh; `None` (e.g. unit tests, no player yet) skips the
/// refresh and only updates `editable_tuning`.
pub fn apply_movement_profile(
    editable_tuning: &mut EditableMovementTuning,
    profile: MovementProfile,
    live_movement_refs: Option<(
        &ambition_platformer2d_core::BodyAbilities,
        &mut ambition_platformer2d_core::BodyDashState,
        &mut ambition_platformer2d_core::BodyJumpState,
        &mut ambition_platformer2d_core::BodyDodgeState,
    )>,
) {
    let tuning = profile.tuning();
    *editable_tuning = EditableMovementTuning::from(tuning);
    if let Some((abilities, dash, jump, dodge)) = live_movement_refs {
        ae::refresh_movement_resources_clusters(
            abilities,
            dash,
            jump,
            dodge,
            tuning.air_jumps,
            // A dev-tools profile swap rebuilds the body's resources outright;
            // there is no move in flight for it to be interrupting.
            ae::RecoveryRefresh::Answered,
        );
    }
}

/// Apply live ability-flag edits without rebuilding the player every frame.
///
/// Mutates `BodyAbilities` + side-effects on `BodyFlightState`,
/// `MotionModel`, `BodyDashState`, and `BodyJumpState` directly.
pub fn sync_live_ability_edits_clusters(
    abilities: &mut ambition_platformer2d_core::BodyAbilities,
    flight: &mut ambition_platformer2d_core::BodyFlightState,
    model: &mut ambition_platformer2d_core::MotionModel,
    dash: &mut ambition_platformer2d_core::BodyDashState,
    jump: &mut ambition_platformer2d_core::BodyJumpState,
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
        // Cancel any in-flight blink telegraph: hold/aim state is the axis
        // policy's private maneuver state (ADR 0024), so this deliberate
        // dev-tools poke goes through the model variant.
        if let ambition_platformer2d_core::MotionModel::AxisSwept(axis) = model {
            axis.state.blink_hold_active = false;
            axis.state.blink_hold_timer = 0.0;
            axis.state.blink_aiming = false;
        }
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
            &mut ambition_platformer2d_core::BodyMana,
            &mut ambition_platformer2d_core::BodyOffense,
        ),
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    mut health_q: Query<
        &mut ambition_characters::actor::BodyHealth,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
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
    }
}

/// The editor adapter: push the inspector's live movement edits into the
/// simulation's neutral authority.
///
/// This is the ONLY direction tuning flows in a developer build.
/// [`EditableMovementTuning`] is a reflected mirror that exists so
/// bevy-inspector-egui can edit tuning without `MovementTuning` deriving
/// `Reflect` on a hot path; it is not, and must not become, what the simulation
/// reads. Sim systems read `ae::ActiveMovementTuning`, so a build without
/// developer tools simply never installs this system and keeps whatever content
/// authored.
///
/// Change-guarded, so an untouched inspector never trips Bevy change detection
/// on a resource the whole simulation reads.
///
/// `is_added` is excluded deliberately: Bevy counts INSERTION as a change, and
/// the mirror is installed with its own defaults before content finishes
/// seeding. Propagating that first "change" would let the inspector's defaults
/// overwrite authored tuning on frame one — which is the exact ownership
/// inversion this slice exists to remove.
pub fn apply_editable_movement_tuning(
    editable: Res<EditableMovementTuning>,
    mut active: ResMut<ae::ActiveMovementTuning>,
) {
    if !editable.is_changed() || editable.is_added() {
        return;
    }
    active.0 = editable.as_engine();
}

#[cfg(test)]
mod adapter_tests {
    use super::*;

    fn app_with_adapter() -> App {
        let mut app = App::new();
        app.init_resource::<EditableMovementTuning>();
        app.init_resource::<ae::ActiveMovementTuning>();
        app.add_systems(Update, apply_editable_movement_tuning);
        app
    }

    /// Live editing still works: the F3 panel's edit reaches the value the simulation actually
    /// reads.
    #[test]
    fn an_inspector_edit_reaches_the_simulation_authority() {
        let mut app = app_with_adapter();
        app.update();

        app.world_mut()
            .resource_mut::<EditableMovementTuning>()
            .jump_speed = -777.0;
        app.update();

        assert_eq!(
            app.world()
                .resource::<ae::ActiveMovementTuning>()
                .jump_speed,
            -777.0,
            "the editor writes through to the authority the sim reads"
        );
    }

    /// The authority is content's until an edit happens: an untouched inspector
    /// must not stamp its own defaults over what the game authored.
    #[test]
    fn an_untouched_inspector_does_not_overwrite_authored_tuning() {
        let mut app = app_with_adapter();
        let authored = ae::MovementTuning {
            jump_speed: -123.0,
            ..Default::default()
        };
        app.insert_resource(ae::ActiveMovementTuning(authored));
        // Several frames: change detection must stay quiet the whole time.
        for _ in 0..3 {
            app.update();
        }

        assert_eq!(
            app.world()
                .resource::<ae::ActiveMovementTuning>()
                .jump_speed,
            -123.0,
            "authored content survives an inspector nobody touched"
        );
    }

    /// ⛔⛔ AN EDIT MUST NOT DELETE THE RULES IT WAS NOT ABOUT (D186).
    ///
    /// The round trip rebuilt six fields from constants: `parry_timing` from
    /// `default()` and the five evade-staling / tech rules from zero. A fighter
    /// carries all six in one authored preset — `abilities.rs` sets
    /// `dodge_stale_step: 0.25` and `untechable_launch_speed: 1400.0` beside
    /// `parry_timing: OnRaise` — so dragging ANY unrelated slider silently
    /// replaced the stage's ruleset with the engine's defaults and said nothing.
    ///
    /// ⭐ THE ARM IS AN EDIT TO SOMETHING ELSE. Asserting the fields survive a
    /// round trip with no edit would agree with the bug: the wipe happens on the
    /// way BACK OUT, so the test has to touch one knob and read the others.
    #[test]
    fn editing_one_knob_does_not_wipe_the_rules_it_was_not_about() {
        let declared = ae::MovementTuning {
            // ⛔ `OnRelease`, NOT `OnRaise`. `OnRaise` IS the default, so a test
            // that declared it agreed with the wipe — measured: poisoning the
            // projection back to `default()` left this arm green.
            parry_timing: ae::ParryTiming::OnRelease,
            dodge_stale_step: 0.25,
            dodge_stale_floor: 0.34,
            dodge_stale_recovery: 1.2,
            untechable_launch_speed: 1400.0,
            evade_cancel_tail: 4.0 / 60.0,
            jump_speed: -500.0,
            ..Default::default()
        };
        let mut editable = EditableMovementTuning::from(declared);
        // The unrelated slider somebody actually dragged.
        editable.jump_speed = -600.0;
        let out = editable.as_engine();

        assert_eq!(out.jump_speed, -600.0, "the edit itself must land");
        assert_eq!(
            out.parry_timing,
            ae::ParryTiming::OnRelease,
            "the stage's parry ruling was replaced by the engine default"
        );
        assert_eq!(out.dodge_stale_step, 0.25);
        assert_eq!(out.dodge_stale_floor, 0.34);
        assert_eq!(out.dodge_stale_recovery, 1.2);
        assert_eq!(out.untechable_launch_speed, 1400.0);
        assert_eq!(out.evade_cancel_tail, 4.0 / 60.0);
    }

    /// ⛔⛔ AND FIVE MORE FIELDS SAID "CARRIED" WHILE NOT CARRYING. The
    /// projection read `DEFAULT_TUNING` for the crouch cost and the four
    /// ground-movement phase timings under a comment reading *"Carried, not
    /// edited: which ground-movement PHASES a game has is a rules
    /// declaration"* — which is the right rule and the opposite behaviour. A
    /// game with its own dash phases lost them to an unrelated slider.
    #[test]
    fn a_games_own_movement_phases_survive_an_unrelated_edit() {
        let declared = ae::MovementTuning {
            crouch_speed_frac: 0.4,
            initial_dash_time: 0.11,
            initial_dash_speed: 900.0,
            turnaround_time: 0.07,
            teeter_margin: 6.0,
            jump_speed: -500.0,
            ..Default::default()
        };
        let mut editable = EditableMovementTuning::from(declared);
        editable.jump_speed = -600.0;
        let out = editable.as_engine();

        assert_eq!(out.jump_speed, -600.0, "the edit itself must land");
        assert_eq!(out.crouch_speed_frac, 0.4);
        assert_eq!(out.initial_dash_time, 0.11);
        assert_eq!(out.initial_dash_speed, 900.0);
        assert_eq!(out.turnaround_time, 0.07);
        assert_eq!(out.teeter_margin, 6.0);
    }
}
