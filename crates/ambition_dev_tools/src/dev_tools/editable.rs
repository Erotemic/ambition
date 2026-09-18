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
    /// ⭐ THE DESTRUCTURE IS THE GUARD. This mirror exists so the live inspector can
    /// edit `AbilitySet` through `Reflect`, and nothing else forces the two to agree:
    /// reading `value.field` one field at a time still COMPILES when `AbilitySet` grows
    /// a 30th ability, so the new ability would simply be missing from the inspector
    /// and no test would notice -- the mirror is a hand-kept copy of somebody else's
    /// struct.
    ///
    /// ⇒ Binding every field by name with no `..` makes that an E0027 here instead.
    /// Measured 2026-09-06: both structs carry the same 29 fields today, so this is
    /// prevention, not a repair.
    fn from(value: ae::AbilitySet) -> Self {
        let ae::AbilitySet {
            move_horizontal,
            jump,
            variable_jump,
            double_jump,
            fast_fall,
            wall_jump,
            wall_cling,
            wall_climb,
            dash,
            double_dash,
            fly,
            fly_toggle,
            blink,
            precision_blink,
            blink_through_soft_walls,
            blink_through_hard_walls,
            attack,
            pogo,
            directional_primary,
            directional_special,
            rebound,
            reset,
            ledge_grab,
            swim,
            glide,
            dodge,
            shield,
            grab,
            interact,
        } = value;
        Self {
            move_horizontal,
            jump,
            variable_jump,
            double_jump,
            fast_fall,
            wall_jump,
            wall_cling,
            wall_climb,
            dash,
            double_dash,
            fly,
            fly_toggle,
            blink,
            precision_blink,
            blink_through_soft_walls,
            blink_through_hard_walls,
            attack,
            pogo,
            directional_primary,
            directional_special,
            rebound,
            reset,
            ledge_grab,
            swim,
            glide,
            dodge,
            shield,
            grab,
            interact,
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
            // ⛔⛔ CARRIED MEANS CARRIED. Reading `DEFAULT_TUNING` here REPLACES
            // whatever the stage declared with the engine's answer, so a game
            // with its own dash phases loses them to an unrelated slider. The
            // ONLY thing "not edited" adds is that no inspector row offers it,
            // which `reflect(ignore)` is for.
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
/// This domain's key in [`ae::PendingMechanicalEdits`].
/// The marker type that OWNS this domain.
pub struct DeveloperBodyProfileDomain;

pub fn body_profile_domain() -> ae::MechanicalDomain {
    ae::MechanicalDomain::of::<DeveloperBodyProfileDomain>("developer_body_profile")
}

/// Raise a changed developer body profile as a PROPOSAL.
///
/// ⛔⛤ **THE `Local` MOVED OUT OF THE ROLLBACK WINDOW WITH THE SYSTEM, AND THAT
/// IS HALF THE FIX — `Q120`, 2026-09-13.** This selection test used to live
/// inside `sync_developer_body_profile`, which was registered into
/// `app.sim_schedule()` — under the rollback host, `GgrsSchedule`. A `Local` there
/// runs once per ADVANCE, resimulations included, so after a rewind it still
/// remembered that the new profile had been applied and made its decision from
/// PRESENT-FRAME HOST HISTORY rather than from the frame being simulated.
///
/// ⇒ In `PreUpdate` it runs once per rendered frame, which is the lifetime a
/// "last applied" memory actually has. The proposal it raises is what carries
/// across a refusal.
pub fn propose_developer_body_profile(
    developer: Res<DeveloperTools>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
    mut last_proposed: Local<Option<PlayerBodyProfile>>,
) {
    // Propose only when the developer CHANGES the selected profile. Re-proposing
    // every frame would make the dev tool authoritative over legitimate
    // gameplay-driven body-size changes — and would stop the rollback baseline
    // every frame for an edit nobody made.
    if *last_proposed == Some(developer.player_body_profile) {
        return;
    }
    *last_proposed = Some(developer.player_body_profile);
    pending.propose(body_profile_domain());
}

/// The body profile this session has ADMITTED, independent of whether a body
/// exists to wear it.
///
/// ⛔⛤ **ADMITTING A VALUE AND PROJECTING IT ONTO A TARGET ARE DIFFERENT JOBS,
/// AND COLLAPSING THEM LOST EDITS — REVIEW, 2026-09-13, ABOUT CODE SHIPPED THE
/// SAME DAY.** The first version of the migration drained the proposal and THEN
/// looked for a player:
///
/// ```text
/// profile edited -> proposal admitted -> no player exists
///   -> proposal removed -> nothing receives the profile
///   -> the proposer's `Local` has already advanced, so it never proposes again
/// ```
///
/// ⇒ The edit was gone with nothing to say so. And the same collapse contradicted
/// the system's own stated purpose: it exists to keep the player's body aligned
/// after a reset or a room load rebuilds it from engine defaults, and a
/// reconstructed player does not cause the EDITOR to change — so after the
/// migration there was no persistent authority a reconstruction could project.
///
/// ⭐⭐ **SO EACH MECHANICAL-EDITOR DOMAIN HAS THREE STAGES, NOT TWO:** editable
/// desired value → pending proposal → **admitted domain authority** → projection
/// onto whatever entities exist. Movement tuning and portal tuning already had
/// the third stage (`ActiveMovementTuning`, `PortalTuning`); this domain did not,
/// which is why it was the one that broke.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ActivePlayerBodyProfile(pub Option<PlayerBodyProfile>);

/// The ADMITTED developer ability mask — the third stage for the ability domain.
///
/// ⛔⛤ **THE ABILITY DOMAIN WAS THE LAST ONE USING ITS EDITOR RESOURCE AS THE
/// ADMITTED AUTHORITY**, and the review of 2026-09-14 named it. The rule
/// `ActivePlayerBodyProfile` above states applies here word for word: editable
/// desired value → pending proposal → **admitted domain authority** → projection
/// onto whatever entities exist.
///
/// ⚠ **WHAT THE COLLAPSE COST IS SUBTLER HERE THAN IT WAS FOR THE BODY PROFILE,
/// WHICH IS WHY IT SURVIVED.** `project_editable_abilities` reads
/// `EditableAbilitySet` directly and treats it as the last admitted value
/// *"whenever nothing is pending"* — sound, but it means ADMISSION and
/// PROJECTION are both gated on a primary player EXISTING. With a live locally
/// maintained timeline and a momentarily absent player, a still-pending proposal
/// re-enters the admission/rebase decision on every frame until a body appears.
///
/// ⭐ Splitting them makes a temporary absence of a player irrelevant to whether
/// the edit was admitted, and makes reset/reconstruction semantics explicit: a
/// body built later projects the admitted mask rather than whatever the editor
/// happens to hold then.
///
/// ⚠ `None` means no admission has happened YET — the first publish seeds it from
/// the editable, so the continuous `base ∩ mask` reconciliation this domain also
/// performs keeps working unchanged. That reconciliation is NOT a mechanical edit
/// and breaking it is a real consequence; see the system's own comment.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ActiveEditableAbilityMask(pub Option<ae::AbilitySet>);

pub fn sync_developer_body_profile(
    developer: Res<DeveloperTools>,
    admission: Option<Res<ae::MechanicalEditAdmission>>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
    mut active: ResMut<ActivePlayerBodyProfile>,
) {
    // ⛔ ONLY THIS DOMAIN'S PROPOSAL, and only when something with a view of the
    // rollback timeline has admitted it. See `PendingMechanicalEdits`.
    if !pending.is_pending(body_profile_domain()) {
        return;
    }
    if matches!(
        admission.as_deref(),
        Some(ae::MechanicalEditAdmission::Refuse)
    ) {
        return;
    }
    // ⛔ THE AUTHORITY MOVES WHETHER OR NOT A BODY EXISTS TO WEAR IT. That is the
    // whole repair: draining the proposal is now safe because the admitted value
    // is kept, and a player constructed later reads it from here.
    active.0 = Some(developer.player_body_profile);
    pending.take(body_profile_domain());
}

/// Project the ADMITTED body profile onto whatever primary player exists.
///
/// ⛔ **A PROJECTION, NOT A PUBLICATION.** It re-applies on its own whenever the
/// live body disagrees with the admitted value, so a reset or a room load that
/// rebuilds the player from engine defaults gets the developer's profile back —
/// which is what this domain always claimed to do and stopped doing for the few
/// hours between the migration and this repair.
///
/// ⚠ It writes nothing until something has been ADMITTED: `None` means the
/// developer has not chosen, and stamping a default over an authored body would
/// make the dev tool authoritative over content that never asked.
pub fn project_developer_body_profile(
    active: Res<ActivePlayerBodyProfile>,
    mut player_q: Query<
        (
            &mut ambition_platformer2d_core::BodyKinematics,
            &mut ambition_platformer2d_core::BodyBaseSize,
        ),
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
) {
    let Some(profile) = active.0 else {
        return;
    };
    let desired = profile.size();
    if let Ok((mut kinematics, mut base_size)) = player_q.single_mut() {
        if (base_size.base_size - desired).length_squared() > 0.01 {
            // Resize the body while preserving the planted-foot position.
            let new_size = desired;
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

/// Last-synced stats snapshot, used to tell USER EDITS apart from runtime drift.
///
/// Without it, any frame where gameplay damaged HP would see
/// `stats.health != live_hp` and push the stale inspector value back into the
/// runtime, undoing the damage.
///
/// ⛔⛤ **A RESOURCE, NOT A `Local`, SINCE `Q120` SPLIT THIS DOMAIN — 2026-09-13.**
/// Three systems share it now (the proposer, the publisher and the body→inspector
/// mirror) and a `Local` belongs to exactly one. ⚠ **AND IT WAS A `Local` INSIDE
/// THE SIM SCHEDULE, WHICH IS THE DEEPER HALF:** under the rollback host that is
/// `GgrsSchedule`, so it advanced once per ADVANCE — resimulations included — and
/// after a rewind still remembered a value from a frame that had been undone.
/// It is host-side now and moves once per rendered frame.
#[derive(bevy::prelude::Resource, Default)]
pub struct PlayerStatsSyncSnapshot {
    initialized: bool,
    health: i32,
    max_health: i32,
    // ⛔⛤ **THE MANA AND OFFENSE FIELDS ARE NEW, AND THEIR ABSENCE WAS A DEFECT
    // NOBODY HAD NAMED.** The combined system wrote `BodyMana.meter` and
    // `BodyOffense.damage_multiplier` from the inspector **UNCONDITIONALLY**, on
    // every run, with no change test at all — so inside `GgrsSchedule` that was a
    // per-ADVANCE write of canonical state from a live developer resource,
    // strictly worse than the health half. Snapshotting them is what lets the
    // publisher ask the same question about them that it always asked about HP.
    mana: i32,
    max_mana: i32,
    slash_damage: i32,
}

/// This domain's key in [`ae::PendingMechanicalEdits`].
/// The marker type that OWNS this domain.
pub struct EditablePlayerStatsDomain;

pub fn player_stats_domain() -> ae::MechanicalDomain {
    ae::MechanicalDomain::of::<EditablePlayerStatsDomain>("editable_player_stats")
}

/// Raise a developer stat edit as a PROPOSAL.
///
/// ⛔⛤ **`stats.is_changed()` IS THE WRONG TEST HERE AND THAT IS WHY THIS DOMAIN
/// TOOK LONGER THAN THE OTHER THREE.** The body→inspector MIRROR writes
/// `EditablePlayerStats` every time gameplay moves the player's HP, so change
/// detection would raise a proposal — and therefore stop the rollback baseline —
/// on every point of damage the player takes. The snapshot is what separates
/// *"the developer typed a number"* from *"the game changed one"*, and it is the
/// same discrimination the combined system always made; it is just asked here now.
pub fn propose_player_stats_edits(
    stats: Res<EditablePlayerStats>,
    snapshot: Res<PlayerStatsSyncSnapshot>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
) {
    if !snapshot.initialized {
        // The first frame establishes the baseline; the publisher does that, and
        // proposing before one exists would treat the defaults as an edit.
        return;
    }
    // ⚠ `refill_now` IS AN EDIT even though it changes no value yet: it is a
    // one-shot button whose whole effect is a write, and a refill staged behind
    // a refusal must fire when the refusal lifts rather than being forgotten.
    if stats.refill_now
        || stats.health != snapshot.health
        || stats.max_health != snapshot.max_health
        || stats.mana != snapshot.mana
        || stats.max_mana != snapshot.max_mana
        || stats.slash_damage != snapshot.slash_damage
    {
        pending.propose(player_stats_domain());
    }
}

/// Publish an ADMITTED developer stat edit onto the live player.
///
/// ⛔⛤ **THIS USED TO BE ONE BIDIRECTIONAL SYSTEM IN THE SIM SCHEDULE — `Q120`,
/// 2026-09-13.** `sync_player_stats_with_inspector` did three jobs at once:
/// inspector→body (health/max_health when the user moved them), body→inspector
/// (the `else` branch, *"so the F3 panel shows truth"*), and an UNCONDITIONAL
/// inspector→body write of `BodyMana.meter` and `BodyOffense.damage_multiplier`.
/// Registered into `app.sim_schedule()` — `GgrsSchedule` under the rollback host
/// — every one of those writes landed inside the rollback window, and the last
/// one landed on every single ADVANCE.
///
/// ⭐ **THE THREE JOBS ARE THREE SYSTEMS NOW.** This one WRITES, and only when
/// the timeline's owner has admitted the edit;
/// [`mirror_player_stats_into_the_inspector`] reads the body back into the panel
/// and stays where it was; [`propose_player_stats_edits`] decides that the
/// developer — rather than gameplay — moved something.
///
/// ⚠ **`refill_now` IS CONSUMED HERE, NOT AT THE PROPOSAL.** A refill staged
/// behind a foreign rollback timeline must still fire when the refusal lifts;
/// clearing the flag when it was merely noticed would drop the button press.
pub fn publish_player_stats_edits(
    mut stats: ResMut<EditablePlayerStats>,
    admission: Option<Res<ae::MechanicalEditAdmission>>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
    mut snapshot: ResMut<PlayerStatsSyncSnapshot>,
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
    // ⛔ THE BASELINE IS ESTABLISHED HERE AND NOWHERE ELSE, so the proposer has
    // exactly one definition of "what the developer last saw" to compare against.
    if !snapshot.initialized {
        snapshot.health = stats.health;
        snapshot.max_health = stats.max_health;
        snapshot.mana = stats.mana;
        snapshot.max_mana = stats.max_mana;
        snapshot.slash_damage = stats.slash_damage;
        snapshot.initialized = true;
        return;
    }
    if !pending.is_pending(player_stats_domain()) {
        return;
    }
    if matches!(
        admission.as_deref(),
        Some(ae::MechanicalEditAdmission::Refuse)
    ) {
        return;
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
        }
    }
    // Mana lives on `Player::mana` (engine `ResourceMeter`); the inspector
    // surfaces i32 fields for player-friendly editing and the conversion happens
    // at this boundary. Combat tuning and invincibility live on `Player`
    // (engine-side) so per-player state is engine state, not sandbox state.
    //
    // ⛔⛤ **FIELD-CONDITIONAL, AND IT WAS UNCONDITIONAL UNTIL 2026-09-14.** These
    // three writes fired on EVERY admitted proposal of this domain, whatever the
    // developer had actually moved. `mirror_player_stats_into_the_inspector`
    // refreshes only health and max_health from the live body, so the panel's
    // mana goes stale the moment gameplay spends any:
    //
    // ```text
    //   gameplay spends mana      live 40, inspector still 100
    //   developer edits max_health only
    //     → the stats domain becomes pending
    //     → health is applied  (correctly, conditionally)
    //     → AND mana is written back to the stale 100
    //   ⇒ a HEALTH edit refilled mana.
    // ```
    //
    // The same shape applied to `damage_multiplier` against anything else that
    // legitimately moves it. Found by the GPT architecture review 2026-09-14.
    //
    // ⭐ THE SNAPSHOT ALREADY MEANS "what the developer last saw", which is what
    // the health branch above has always compared against — so the repair is to
    // give the other two fields the same test rather than to invent a per-field
    // dirty structure. A field that did not move is not an edit.
    let user_changed_mana = stats.mana != snapshot.mana || stats.max_mana != snapshot.max_mana;
    let user_changed_offense = stats.slash_damage != snapshot.slash_damage;
    let max_mana = stats.max_mana.max(0);
    if let Ok((mut mana, mut offense)) = player_q.single_mut() {
        if user_changed_mana {
            mana.meter.max = max_mana as f32;
            mana.meter.current = stats.mana.clamp(0, max_mana) as f32;
        }
        if user_changed_offense {
            offense.damage_multiplier = stats.slash_damage.max(1);
        }
    }

    snapshot.health = stats.health;
    snapshot.max_health = stats.max_health;
    snapshot.mana = stats.mana;
    snapshot.max_mana = stats.max_mana;
    snapshot.slash_damage = stats.slash_damage;
    pending.take(player_stats_domain());
}

/// Mirror the live player's health back into the inspector, so the F3 panel
/// shows truth.
///
/// ⭐ **THIS HALF IS PRESENTATION AND STAYS WHERE IT WAS.** It reads the body and
/// writes the developer resource; it changes nothing the simulation reads, so it
/// is not a mechanical edit and does not belong behind an admission boundary.
///
/// ⛔ **IT DOES NOT RUN WHILE THIS DOMAIN HAS A PROPOSAL PENDING.** A staged edit
/// the developer typed must not be overwritten by the body's current value while
/// it waits for a refusal to lift — that would make a refusal indistinguishable
/// from a silent discard, which is the failure staging exists to prevent.
pub fn mirror_player_stats_into_the_inspector(
    mut stats: ResMut<EditablePlayerStats>,
    mut snapshot: ResMut<PlayerStatsSyncSnapshot>,
    pending: Res<ae::PendingMechanicalEdits>,
    health_q: Query<
        &ambition_characters::actor::BodyHealth,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    live_q: Query<
        (
            &ambition_platformer2d_core::BodyMana,
            &ambition_platformer2d_core::BodyOffense,
        ),
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
) {
    if !snapshot.initialized || pending.is_pending(player_stats_domain()) {
        return;
    }
    let Ok(health) = health_q.single() else {
        return;
    };
    stats.health = health.health.current;
    stats.max_health = health.health.max;
    snapshot.health = stats.health;
    snapshot.max_health = stats.max_health;
    // ⭐⭐ **AND MANA AND OFFENSE, BECAUSE A HALF-MIRROR IS THE DANGEROUS STATE.**
    // Mirroring only health left the panel showing a mana the body had long since
    // spent, and that stale number was what the publisher wrote back on the next
    // unrelated edit. Mirroring the live values keeps "what the developer last
    // saw" true for every field this domain publishes — and updating `stats` and
    // `snapshot` TOGETHER is what stops ordinary gameplay mana consumption from
    // looking like a proposal.
    if let Ok((mana, offense)) = live_q.single() {
        stats.mana = mana.meter.current.round() as i32;
        stats.max_mana = mana.meter.max.round() as i32;
        stats.slash_damage = offense.damage_multiplier;
        snapshot.mana = stats.mana;
        snapshot.max_mana = stats.max_mana;
        snapshot.slash_damage = stats.slash_damage;
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
/// ⛔⛤ **SPLIT IN TWO ON 2026-09-13, BECAUSE ONE HALF OF IT IS A DECISION THE
/// ROLLBACK TIMELINE OWNS.** See [`propose_editable_movement_tuning`] for the
/// first half. This one only WRITES, and only when something with a view of the
/// timeline has said it may.
///
/// ⚠ It is deliberately NOT change-guarded any more. The guard now lives on the
/// PROPOSAL: an untouched inspector raises nothing, so this never runs its
/// write. Re-adding `is_changed` here would drop every edit that had to be
/// staged for a frame — which is the entire point of staging it.
pub fn publish_editable_movement_tuning(
    editable: Res<EditableMovementTuning>,
    admission: Option<Res<ae::MechanicalEditAdmission>>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
    mut active: ResMut<ae::ActiveMovementTuning>,
) {
    // ⛔⛤ **ASKED ABOUT THIS DOMAIN, NOT ABOUT THE BATCH.** The first version
    // read a single global `bool`: with two domains proposing in one frame, the
    // first publisher cleared the bit and the second silently dropped its edit.
    // See `PendingMechanicalEdits`.
    if !pending.is_pending(movement_tuning_domain()) {
        return;
    }
    // ⛔ ABSENT ⇒ PUBLISH, matching the resource's own default: a composition
    // with no rollback host has no history an edit could contradict, and a
    // missing answer that read as `Refuse` would silently kill live editing in
    // every non-rollback build. The host that CAN refuse always installs it.
    if matches!(
        admission.as_deref(),
        Some(ae::MechanicalEditAdmission::Refuse)
    ) {
        return;
    }
    active.0 = editable.as_engine();
    // ⛔ DRAIN ONLY OURS. There is no method on `PendingMechanicalEdits` that
    // can drain another domain's proposal, which is the point of the type.
    pending.take(movement_tuning_domain());
}

/// Raise the developer's movement-tuning edit as a PROPOSAL.
///
/// ⛔⛤ **THE EDIT DOES NOT REACH THE SIMULATION HERE, AND THAT IS THE FIX.**
/// This used to be one system, `apply_editable_movement_tuning`, registered into
/// the SIM schedule — which under the rollback host is `GgrsSchedule`. So the
/// authoritative value moved inside the rollback window, where a resimulation of
/// already-confirmed frames would read it: `Q120` measured that desyncing the
/// GGRS sync-test canary. The first attempt at a fix added a WATCHER in `Update`
/// to stop the baseline afterwards; `Update` runs after `RunGgrsSystems`, so the
/// old timeline consumed the edit first and the watcher's doc comment claimed an
/// ordering the schedule never established.
///
/// ⇒ Now: propose in [`MechanicalEditSet::Propose`], the timeline's owner
/// answers in `Admit`, and [`publish_editable_movement_tuning`] writes in
/// `Publish` — the whole chain in `PreUpdate`, before the advance.
///
/// ⚠ **`is_added` IS EXCLUDED** deliberately: Bevy counts INSERTION as a change,
/// and the mirror is installed with its own defaults before content finishes
/// seeding. Proposing that first "change" would let the inspector's defaults
/// overwrite authored tuning on frame one — and would stop the session the
/// composition had just started, every time.
pub fn propose_editable_movement_tuning(
    editable: Res<EditableMovementTuning>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
) {
    if !editable.is_changed() || editable.is_added() {
        return;
    }
    pending.propose(movement_tuning_domain());
}

/// This domain's key in [`ae::PendingMechanicalEdits`].
///
/// ⭐ **DECLARED BESIDE THE VALUE IT OWNS, not in a central enum.** `Q120` names
/// five more mutable mechanical values owned by five different crates; a central
/// enum would make `ambition_platformer2d_core` name every one of them, which is
/// the dependency edge this protocol exists to avoid.
/// The marker type that OWNS this domain.
pub struct MovementTuningDomain;

pub fn movement_tuning_domain() -> ae::MechanicalDomain {
    ae::MechanicalDomain::of::<MovementTuningDomain>("movement_tuning")
}

#[cfg(test)]
mod adapter_tests {
    use super::*;

    fn app_with_adapter() -> App {
        let mut app = App::new();
        app.init_resource::<EditableMovementTuning>();
        app.init_resource::<ae::ActiveMovementTuning>();
        app.init_resource::<ae::PendingMechanicalEdits>();
        app.init_resource::<ae::MechanicalEditAdmission>();
        app.add_systems(
            Update,
            (
                propose_editable_movement_tuning,
                publish_editable_movement_tuning,
            )
                .chain(),
        );
        app
    }

    /// ⛔⛤ **A REFUSED EDIT DOES NOT MOVE THE VALUE THE SIMULATION READS, AND IS
    /// NOT THROWN AWAY.**
    ///
    /// This is `Q120`'s model 1 as its row states it — *"refuse mechanical live
    /// edits while a rollback timeline is active"* — and the first version of
    /// this fix implemented the opposite: the edit landed and a watcher tried to
    /// clean up after it. An external or caller-owned timeline cannot be stopped
    /// by this host, so the only coherent answer is to hold the proposal.
    ///
    /// ⭐ THE SECOND HALF IS THE ONE WORTH GUARDING. A refusal that dropped the
    /// edit would leave the inspector showing a value that will never be
    /// published — the developer drags a slider, nothing happens, and nothing
    /// ever will.
    #[test]
    fn a_refused_edit_is_staged_and_publishes_when_the_refusal_lifts() {
        let mut app = app_with_adapter();
        app.update();
        app.insert_resource(ae::MechanicalEditAdmission::Refuse);

        app.world_mut()
            .resource_mut::<EditableMovementTuning>()
            .jump_speed = -777.0;
        app.update();

        assert_eq!(
            app.world()
                .resource::<ae::ActiveMovementTuning>()
                .jump_speed,
            ae::MovementTuning::default().jump_speed,
            "a refused edit still reached the authority every simulation system \
             reads, so a timeline nobody could rebase resimulates against \
             mechanics it never ran with"
        );
        assert!(
            app.world()
                .resource::<ae::PendingMechanicalEdits>()
                .is_pending(movement_tuning_domain()),
            "the refused edit was discarded instead of staged"
        );

        // ⚠ SEVERAL FRAMES while refused: the proposal is sticky, and
        // `is_changed` has long since gone quiet.
        for _ in 0..3 {
            app.update();
        }
        app.insert_resource(ae::MechanicalEditAdmission::Publish);
        app.update();

        assert_eq!(
            app.world()
                .resource::<ae::ActiveMovementTuning>()
                .jump_speed,
            -777.0,
            "the staged edit never published after the refusal lifted, so the \
             developer's change was silently lost"
        );
        assert!(
            !app.world()
                .resource::<ae::PendingMechanicalEdits>()
                .is_pending(movement_tuning_domain()),
            "a published proposal stayed pending, so every later frame republishes it"
        );
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

#[cfg(test)]
mod player_stats_domain_tests {
    use super::*;

    /// The three systems in the order the composition runs them: propose, then
    /// publish, then the body→inspector mirror.
    ///
    /// ⚠ The MIRROR is registered last on purpose — in the shipped app it is in
    /// the sim schedule, which is after `PreUpdate` — so a fixture that ran it
    /// first would be testing an order the game does not have.
    fn app_with_the_stats_domain() -> App {
        let mut app = App::new();
        app.init_resource::<EditablePlayerStats>();
        app.init_resource::<PlayerStatsSyncSnapshot>();
        app.init_resource::<ae::PendingMechanicalEdits>();
        app.init_resource::<ae::MechanicalEditAdmission>();
        // ⛔⛤ **AN OBSERVER BETWEEN PROPOSE AND PUBLISH, AND ITS ABSENCE MADE THE
        // DAMAGE ARM UNFALSIFIABLE.** MEASURED: poisoning the proposer to propose
        // UNCONDITIONALLY left that arm GREEN, because the publisher runs in the
        // same frame and DRAINS the proposal — so "not pending after the update"
        // is satisfied by a propose-then-publish cycle and cannot see
        // over-proposing at all. Over-proposing is the whole hazard: it would
        // stop and rebase the rollback baseline on every point of damage.
        app.init_resource::<ProposalsSeen>();
        app.add_systems(
            Update,
            (
                propose_player_stats_edits,
                (|pending: Res<ae::PendingMechanicalEdits>, mut seen: ResMut<ProposalsSeen>| {
                    if pending.is_pending(player_stats_domain()) {
                        seen.0 += 1;
                    }
                }),
                publish_player_stats_edits,
                mirror_player_stats_into_the_inspector,
            )
                .chain(),
        );
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
            ambition_characters::actor::BodyHealth::new(
                ambition_characters::actor::Health::new(5),
            ),
            ambition_platformer2d_core::BodyMana::default(),
            ambition_platformer2d_core::BodyOffense::default(),
        ));
        // The first update establishes the baseline and must change nothing.
        app.update();
        app
    }

    /// How many frames the domain was PENDING when the publisher was about to
    /// run. The quantity the damage arm is really about.
    #[derive(Resource, Default)]
    struct ProposalsSeen(u32);

    fn live_health(app: &mut App) -> (i32, i32) {
        let world = app.world_mut();
        let mut query = world.query_filtered::<
            &ambition_characters::actor::BodyHealth,
            ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
        >();
        let health = query.single(world).expect("the fixture has a player body");
        (health.health.current, health.health.max)
    }

    fn live_mana_and_offense(app: &mut App) -> (f32, i32) {
        let world = app.world_mut();
        let mut query = world.query_filtered::<(
            &ambition_platformer2d_core::BodyMana,
            &ambition_platformer2d_core::BodyOffense,
        ), ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>();
        let (mana, offense) = query.single(world).expect("the fixture has a player body");
        (mana.meter.current, offense.damage_multiplier)
    }

    /// ⛔⛤ **EDITING ONE FIELD MUST NOT REPUBLISH THE OTHERS — FOUND BY THE GPT
    /// ARCHITECTURE REVIEW, 2026-09-14.**
    ///
    /// The publisher applied HP conditionally and then wrote mana and
    /// `damage_multiplier` on EVERY admitted proposal of this domain, whatever the
    /// developer had moved. The mirror refreshed only health, so the panel's mana
    /// went stale as soon as gameplay spent any — and that stale number is what
    /// the next unrelated edit wrote back:
    ///
    /// ```text
    ///   gameplay spends mana       live 31, inspector still says 100
    ///   developer edits max_health ONLY
    ///     → health applied, correctly
    ///     → AND mana written back to 100
    ///   ⇒ a HEALTH edit refilled mana.
    /// ```
    ///
    /// ⭐ The premise is asserted first: the live mana must actually differ from
    /// the inspector's when the edit is made, or the arm passes by agreeing with
    /// itself and says nothing about field granularity.
    #[test]
    fn editing_one_stat_leaves_the_others_where_gameplay_put_them() {
        let mut app = app_with_the_stats_domain();

        // GAMEPLAY spends mana and buffs offense — neither is a developer edit.
        {
            let world = app.world_mut();
            let mut query = world.query_filtered::<(
                &mut ambition_platformer2d_core::BodyMana,
                &mut ambition_platformer2d_core::BodyOffense,
            ), ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>();
            let (mut mana, mut offense) = query.single_mut(world).expect("player body");
            mana.meter.max = 100.0;
            mana.meter.current = 31.0;
            offense.damage_multiplier = 7;
        }
        // Let the mirror carry that into the panel, so the inspector is telling
        // the truth before the edit rather than after it.
        app.update();
        let (mana_before, offense_before) = live_mana_and_offense(&mut app);
        assert_eq!(
            (mana_before, offense_before),
            (31.0, 7),
            "gameplay's own writes did not survive a frame, so this fixture is \
             not exercising the case at all",
        );

        // ⛔ THE PREMISE, AND IT IS WHAT MAKES THE ARM DISCRIMINATING. Under the
        // half-mirror the panel never learned these values, so it still held the
        // editor's DEFAULTS — different numbers from the body's. If the defaults
        // happened to equal what gameplay wrote, a stale write-back would be
        // invisible and this arm would pass on the defect.
        let defaults = EditablePlayerStats::default();
        assert!(
            defaults.mana as f32 != mana_before && defaults.slash_damage != offense_before,
            "the editor's defaults ({}, {}) already equal what gameplay wrote \
             ({mana_before}, {offense_before}), so writing the panel back over \
             the body would change nothing and this arm cannot see the defect",
            defaults.mana,
            defaults.slash_damage,
        );

        // Edit ONLY max health. Nothing touches the panel's mana or offense.
        let seen_before = app.world().resource::<ProposalsSeen>().0;
        app.world_mut()
            .resource_mut::<EditablePlayerStats>()
            .max_health = 9;
        app.update();

        assert!(
            app.world().resource::<ProposalsSeen>().0 > seen_before,
            "the HP edit raised no proposal at all, so the publisher never ran \
             and the field-granularity claims below are vacuous",
        );
        let (_, max_health) = live_health(&mut app);
        assert_eq!(max_health, 9, "the developer's max-health edit was not applied");
        let (mana_after, offense_after) = live_mana_and_offense(&mut app);
        assert_eq!(
            mana_after, 31.0,
            "a HEALTH edit refilled mana: the publisher writes this domain's \
             every field on any admitted proposal, and the panel's value was \
             stale because the mirror never read mana back",
        );
        assert_eq!(
            offense_after, 7,
            "a HEALTH edit overwrote the damage multiplier for the same reason",
        );
    }

    /// ⛔⛤ **AND THIS IS THE ARM THAT ISOLATES THE PUBLISHER'S HALF — THE OTHER
    /// ONE CANNOT.** MEASURED 2026-09-14: poisoning the field-conditional publish
    /// ALONE leaves `editing_one_stat_leaves_the_others_where_gameplay_put_them`
    /// GREEN, and so does poisoning the half-mirror alone; only both together
    /// fire it. The two repairs are redundant with respect to that arm, so it
    /// guards the PAIR and nothing guards either one.
    ///
    /// ⭐ **THE CASE THAT SEPARATES THEM IS THE ONE THIS PROTOCOL EXISTS FOR: AN
    /// EDIT STAGED BEHIND A REFUSAL.** The mirror deliberately does not run while
    /// the domain is pending — a staged edit must not be overwritten by the body
    /// — so while a refusal holds, the panel legitimately goes stale against a
    /// body gameplay keeps moving. When the refusal lifts, an unconditional
    /// publisher writes that stale mana back over the live value.
    ///
    /// ```text
    ///   developer edits max_health      domain pending
    ///   timeline says Refuse            publisher declines, mirror stays out
    ///   gameplay spends mana            live 31, panel frozen at its old value
    ///   timeline says Publish           → conditional: only max_health lands
    ///                                   → unconditional: mana snaps back
    /// ```
    #[test]
    fn an_edit_staged_behind_a_refusal_publishes_only_the_field_it_staged() {
        let mut app = app_with_the_stats_domain();
        let staged_mana = app.world().resource::<EditablePlayerStats>().mana;

        // The timeline refuses, and the developer edits max health.
        *app.world_mut().resource_mut::<ae::MechanicalEditAdmission>() =
            ae::MechanicalEditAdmission::Refuse;
        app.world_mut()
            .resource_mut::<EditablePlayerStats>()
            .max_health = 11;
        app.update();

        // Gameplay moves mana while the edit waits. The mirror is out because the
        // domain is pending, so the panel keeps the value it had.
        {
            let world = app.world_mut();
            let mut query = world.query_filtered::<
                &mut ambition_platformer2d_core::BodyMana,
                ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
            >();
            let mut mana = query.single_mut(world).expect("player body");
            mana.meter.max = 100.0;
            mana.meter.current = 31.0;
        }

        // ⛔ THE PREMISE: the edit really is still staged, and the panel really
        // does disagree with the body about mana.
        assert!(
            app.world()
                .resource::<ae::PendingMechanicalEdits>()
                .is_pending(player_stats_domain()),
            "the refusal did not stage anything, so there is no staged edit to \
             publish and the assertion below is vacuous",
        );
        assert_ne!(
            staged_mana as f32, 31.0,
            "the panel already agreed with the body about mana, so an \
             unconditional write-back would change nothing",
        );

        // The refusal lifts.
        *app.world_mut().resource_mut::<ae::MechanicalEditAdmission>() =
            ae::MechanicalEditAdmission::Publish;
        app.update();

        let (_, max_health) = live_health(&mut app);
        assert_eq!(
            max_health, 11,
            "the staged max-health edit never landed once the refusal lifted, \
             which is the whole point of staging it",
        );
        let (mana_after, _) = live_mana_and_offense(&mut app);
        assert_eq!(
            mana_after, 31.0,
            "publishing a staged HEALTH edit wrote the panel's frozen mana back \
             over the value gameplay had reached: the publisher writes every \
             field of this domain on any admitted proposal",
        );
    }

    /// AND THE FIELDS THE DEVELOPER DOES MOVE STILL LAND. The falsifier for a
    /// repair that simply stopped publishing mana and offense at all.
    #[test]
    fn editing_mana_or_offense_alone_still_publishes_that_field() {
        let mut app = app_with_the_stats_domain();
        {
            let mut stats = app.world_mut().resource_mut::<EditablePlayerStats>();
            stats.max_mana = 80;
            stats.mana = 55;
        }
        app.update();
        let (mana, _) = live_mana_and_offense(&mut app);
        assert_eq!(mana, 55.0, "an explicit mana edit did not reach the body");

        {
            let mut stats = app.world_mut().resource_mut::<EditablePlayerStats>();
            stats.slash_damage = 4;
        }
        app.update();
        let (_, offense) = live_mana_and_offense(&mut app);
        assert_eq!(offense, 4, "an explicit offense edit did not reach the body");
    }

    /// ⛔⛤ **GAMEPLAY DAMAGE MUST NOT LOOK LIKE A DEVELOPER EDIT — `Q120`,
    /// 2026-09-13, AND IT IS WHY THIS DOMAIN COULD NOT USE `is_changed()`.**
    ///
    /// The body→inspector mirror writes `EditablePlayerStats` every time gameplay
    /// moves the player's HP. A proposer built on change detection would raise a
    /// proposal — and therefore STOP THE ROLLBACK BASELINE — on every point of
    /// damage the player takes. The snapshot is what separates *"the developer
    /// typed a number"* from *"the game changed one"*.
    #[test]
    fn taking_damage_does_not_propose_a_mechanical_edit() {
        let mut app = app_with_the_stats_domain();

        // Gameplay damages the player, the way combat does.
        {
            let world = app.world_mut();
            let mut query = world.query_filtered::<
                &mut ambition_characters::actor::BodyHealth,
                ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
            >();
            query.single_mut(world).expect("a player body").health.current = 2;
        }
        app.update();

        assert_eq!(
            app.world().resource::<ProposalsSeen>().0,
            0,
            "a point of combat damage raised a MECHANICAL EDIT PROPOSAL, which \
             would stop and rebase the rollback baseline every time the player \
             is hit. ⚠ This counts proposals BETWEEN the proposer and the \
             publisher: asking `is_pending` after the update cannot see this, \
             because the publisher drains the proposal in the same frame."
        );
        assert_eq!(
            app.world().resource::<EditablePlayerStats>().health,
            2,
            "the panel did not follow the live HP, so the mirror is broken — \
             which is the half that must keep working"
        );
        assert_eq!(live_health(&mut app).0, 2, "the damage was undone");
    }

    /// ⛔ **A DEVELOPER EDIT REACHES THE BODY**, which is the control: without it
    /// the arm above is satisfied by a domain that proposes nothing, ever.
    #[test]
    fn a_developer_edit_proposes_and_reaches_the_body() {
        let mut app = app_with_the_stats_domain();
        app.world_mut().resource_mut::<EditablePlayerStats>().max_health = 9;
        app.update();

        assert_eq!(
            app.world().resource::<ProposalsSeen>().0,
            1,
            "the developer's edit raised no proposal, so nothing would tell the \
             rollback host a mechanical value is about to move"
        );
        assert_eq!(
            live_health(&mut app).1,
            9,
            "the developer's max-health edit never reached the player body"
        );
        assert!(
            !app.world()
                .resource::<ae::PendingMechanicalEdits>()
                .is_pending(player_stats_domain()),
            "a published proposal stayed pending, so every later frame \
             republishes it and the mirror never runs again"
        );
    }

    /// ⛔⛤ **A REFUSED EDIT IS STAGED, AND THE MIRROR MUST NOT OVERWRITE IT.**
    ///
    /// This is the arm the split exists for. While an edit waits for a foreign
    /// rollback timeline to end, the body→inspector mirror would happily copy the
    /// body's CURRENT value over the number the developer typed — making a
    /// refusal indistinguishable from a silent discard, which is the failure
    /// staging exists to prevent.
    #[test]
    fn a_refused_stat_edit_survives_the_inspector_mirror_and_publishes_later() {
        let mut app = app_with_the_stats_domain();
        app.insert_resource(ae::MechanicalEditAdmission::Refuse);
        app.world_mut().resource_mut::<EditablePlayerStats>().max_health = 9;

        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            live_health(&mut app).1,
            5,
            "a REFUSED stat edit reached the player body anyway"
        );
        assert_eq!(
            app.world().resource::<EditablePlayerStats>().max_health,
            9,
            "the mirror overwrote the developer's staged edit with the body's \
             current value, so the refusal is indistinguishable from a discard"
        );
        assert!(
            app.world()
                .resource::<ae::PendingMechanicalEdits>()
                .is_pending(player_stats_domain()),
            "the refused edit was dropped rather than staged"
        );

        app.insert_resource(ae::MechanicalEditAdmission::Publish);
        app.update();
        assert_eq!(
            live_health(&mut app).1,
            9,
            "the staged edit never published after the refusal lifted"
        );
    }
}

#[cfg(test)]
mod body_profile_domain_tests {
    use super::*;

    fn app_with_the_body_profile_domain() -> App {
        let mut app = App::new();
        app.init_resource::<DeveloperTools>();
        app.init_resource::<ActivePlayerBodyProfile>();
        app.init_resource::<ae::PendingMechanicalEdits>();
        app.init_resource::<ae::MechanicalEditAdmission>();
        app.add_systems(
            Update,
            (
                propose_developer_body_profile,
                sync_developer_body_profile,
                project_developer_body_profile,
            )
                .chain(),
        );
        app
    }

    fn spawn_a_player(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::markers::PlayerEntity,
                ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
                ambition_platformer2d_core::BodyKinematics::default(),
                ambition_platformer2d_core::BodyBaseSize::default(),
            ))
            .id()
    }

    fn base_size(app: &App, entity: Entity) -> ambition_platformer2d_core::Vec2 {
        app.world()
            .entity(entity)
            .get::<ambition_platformer2d_core::BodyBaseSize>()
            .expect("the fixture's body has a base size")
            .base_size
    }

    /// A profile whose size differs from the default, so "it arrived" is a real
    /// difference rather than two defaults agreeing.
    fn a_distinct_profile() -> PlayerBodyProfile {
        let default_size = PlayerBodyProfile::default().size();
        // ⭐ DERIVED FROM THE TYPE'S OWN LIST, so a new profile cannot leave this
        // fixture silently picking the default and asserting nothing.
        PlayerBodyProfile::ALL
            .into_iter()
        .find(|profile| (profile.size() - default_size).length_squared() > 0.01)
        .expect("some shipped profile differs from the default")
    }

    /// ⛔⛤ **AN EDIT MADE WHILE NO PLAYER EXISTS REACHES THE PLAYER THAT APPEARS
    /// LATER — AND THE FIRST VERSION OF THIS MIGRATION LOST IT.**
    ///
    /// That version drained the proposal and THEN looked for a body: with no
    /// player the proposal was consumed, nothing received the profile, and the
    /// proposer's `Local` had already advanced so it never proposed again. The
    /// edit was gone with nothing to say so.
    #[test]
    fn a_profile_admitted_with_no_player_reaches_the_player_that_appears_later() {
        let mut app = app_with_the_body_profile_domain();
        let profile = a_distinct_profile();

        // ⚠ THE PREMISE: there really is no player, or the arm is the easy case.
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>()
                .iter(app.world())
                .count(),
            0,
            "the fixture already has a player, so this arm is not about the \
             no-target case"
        );

        app.world_mut().resource_mut::<DeveloperTools>().player_body_profile = profile;
        app.update();

        assert_eq!(
            app.world().resource::<ActivePlayerBodyProfile>().0,
            Some(profile),
            "the admitted authority did not move, so an edit made with no body \
             present is lost the moment its proposal is drained"
        );

        // The player appears — a session activation, a room load, a reset.
        let player = spawn_a_player(&mut app);
        app.update();

        assert_eq!(
            base_size(&app, player),
            profile.size(),
            "a player constructed AFTER the edit was admitted did not receive \
             the developer's profile. There is no persistent admitted authority \
             for a reconstruction to project, which is the lifecycle purpose \
             this domain claims in its own doc comment."
        );
    }

    /// ⛔ **AND A REBUILT BODY GETS IT BACK**, which is the purpose the migration
    /// broke: a reset or a room load reconstructs the player from engine
    /// defaults, and the developer's selection did not change — so nothing
    /// proposes, and only a PROJECTION can restore it.
    #[test]
    fn a_player_rebuilt_from_defaults_is_restored_to_the_admitted_profile() {
        let mut app = app_with_the_body_profile_domain();
        let profile = a_distinct_profile();
        let first = spawn_a_player(&mut app);
        app.world_mut().resource_mut::<DeveloperTools>().player_body_profile = profile;
        app.update();
        assert_eq!(base_size(&app, first), profile.size(), "the premise: it applied once");

        // The world rebuilds the player from authored defaults.
        app.world_mut().entity_mut(first).despawn();
        let rebuilt = spawn_a_player(&mut app);
        assert_ne!(
            base_size(&app, rebuilt),
            profile.size(),
            "the premise: the rebuilt body starts at the engine default"
        );
        app.update();

        assert_eq!(
            base_size(&app, rebuilt),
            profile.size(),
            "the rebuilt player kept engine defaults. The editor value did not \
             change, so nothing proposes — only a projection off the admitted \
             authority can restore it, which is what this domain exists for"
        );
    }

    /// ⛔ **A REFUSED EDIT DOES NOT MOVE THE ADMITTED AUTHORITY.** Model 1 means
    /// the authoritative value does not change, and here the authority is the
    /// admitted profile rather than the body.
    #[test]
    fn a_refused_profile_edit_leaves_the_admitted_authority_alone_and_stays_staged() {
        let mut app = app_with_the_body_profile_domain();
        app.insert_resource(ae::MechanicalEditAdmission::Refuse);
        let profile = a_distinct_profile();
        app.world_mut().resource_mut::<DeveloperTools>().player_body_profile = profile;
        app.update();

        assert_eq!(
            app.world().resource::<ActivePlayerBodyProfile>().0,
            None,
            "a REFUSED profile edit moved the admitted authority anyway"
        );
        assert!(
            app.world()
                .resource::<ae::PendingMechanicalEdits>()
                .is_pending(body_profile_domain()),
            "the refused edit was discarded rather than staged"
        );

        app.insert_resource(ae::MechanicalEditAdmission::Publish);
        app.update();
        assert_eq!(
            app.world().resource::<ActivePlayerBodyProfile>().0,
            Some(profile),
            "the staged edit never published after the refusal lifted"
        );
    }
}
