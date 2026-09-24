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
    pub crouch: bool,
    pub climb: bool,
    pub morph: bool,
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
            crouch: self.crouch,
            climb: self.climb,
            morph: self.morph,
        }
    }
}

impl From<ae::AbilitySet> for EditableAbilitySet {
    /// The destructure is the guard. Nothing else keeps this mirror in step with
    /// `AbilitySet`: reading `value.field` one by one still compiles when
    /// `AbilitySet` gets a new field. Binding every field by name with no `..`
    /// makes a missing field an E0027 here.
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
            crouch,
            climb,
            morph,
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
            crouch,
            climb,
            morph,
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
    /// Carried, not edited (see the `parry_timing` line in `as_engine`). A
    /// parry-timing swap is a rules declaration about which game a stage
    /// reproduces, not a slider. `reflect(ignore)` hides it from the inspector,
    /// which builds its sliders from reflection.
    #[reflect(ignore)]
    pub parry_timing: ae::ParryTiming,
    /// The match's evade-staling and tech rules. Carried and hidden for the same
    /// reason. A fighter's tuning sets them beside `parry_timing`; the editor must
    /// return them unchanged, not zeroed.
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
    /// The ground-movement phases and the crouch cost: rules, carried and hidden
    /// like the fields above.
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
    /// The out-of-shield rule: carried, not edited. Like `parry_timing`, it is a
    /// rules declaration, not a slider. Rebuilding tuning from sliders alone would
    /// delete the stage's rule on the first unrelated edit. `reflect(ignore)`
    /// hides it; the foundation crate has no reflection.
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
            // The inspector edits the responsive profile. Authored character profiles
            // can select other laws without passing through this control surface.
            horizontal_law: ae::AxisHorizontalLaw::Responsive,
            jump_law: ae::AxisJumpLaw::VelocityCut,
            run_accel: self.run_accel,
            air_accel: self.air_accel,
            ground_friction: self.ground_friction,
            air_friction: self.air_friction,
            air_stop_assist: self.air_stop_assist,
            carried_decay: self.carried_decay,
            max_run_speed: self.max_run_speed,
            max_air_speed: self.max_air_speed,
            run_commit_frac: self.run_commit_frac,
            // Not an F3 slider, but carried, not defaulted. What a crouch costs is a
            // match rule (`MatchBody::crouch_speed_frac`) that composes over this value.
            crouch_speed_frac: self.crouch_speed_frac,
            // Carried: reading `DEFAULT_TUNING` here would replace the stage's dash
            // phases on any unrelated edit. "Not edited" only means no inspector row,
            // which `reflect(ignore)` does.
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
            // The dev tuning drives the player body (smoothed flight). Direct velocity is
            // a per-body opt-in that the boss sets in its own tuning.
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
            // Match rules, carried like `parry_timing`. A fighter's tuning sets them in one
            // preset (see `abilities.rs`), so zeroing them here would delete the match's
            // rules on an unrelated edit.
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
            // Not editable, but carried. A parry-timing swap is a rules declaration, not
            // a slider; rebuilding it from `default()` on any edit would delete it.
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
                // Carried, not editable, like `parry_timing`: whether a guard exists in the
                // air is a rules declaration.
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

/// Marker type that owns the developer body-profile domain in
/// [`ae::PendingMechanicalEdits`].
pub struct DeveloperBodyProfileDomain;

pub fn body_profile_domain() -> ae::MechanicalDomain {
    ae::MechanicalDomain::of::<DeveloperBodyProfileDomain>("developer_body_profile")
}

/// Raise a changed developer body profile as a proposal.
///
/// This runs in `PreUpdate`, once per rendered frame, not in the sim schedule.
/// In the sim schedule (`GgrsSchedule` under the rollback host) a `Local` runs
/// once per advance, including resimulations, so after a rewind it would
/// decide from present-frame history. The proposal carries the edit across a
/// refusal.
pub fn propose_developer_body_profile(
    developer: Res<DeveloperTools>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
    mut last_proposed: Local<Option<PlayerBodyProfile>>,
) {
    // Propose only when the developer changes the selected profile. Proposing
    // every frame would override gameplay-driven size changes and stop the
    // rollback baseline every frame.
    //
    // Before the first proposal the baseline is the default (unchosen) profile.
    // Treating the default as an edit would stamp the default box over every
    // prepared character body on the first tick.
    let unchosen = PlayerBodyProfile::default();
    if last_proposed.unwrap_or(unchosen) == developer.player_body_profile {
        return;
    }
    *last_proposed = Some(developer.player_body_profile);
    pending.propose(body_profile_domain());
}

/// The body profile this session has admitted, independent of whether a body
/// exists to wear it.
///
/// Admitting a value and projecting it onto a target are separate jobs. If the
/// proposal is drained only when a player exists, this happens:
///
/// ```text
/// profile edited -> proposal admitted -> no player exists
///   -> proposal removed -> nothing receives the profile
///   -> the proposer's `Local` has already advanced, so it never proposes again
/// ```
///
/// A reset or room load also rebuilds the player without an editor change, so
/// it needs a persistent admitted value to project.
///
/// So each mechanical-editor domain has three stages: editable desired value,
/// pending proposal, admitted domain authority, then projection onto the
/// entities that exist. Movement and portal tuning use `ActiveMovementTuning`
/// and `PortalTuning` for the third stage.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ActivePlayerBodyProfile(pub Option<PlayerBodyProfile>);

/// The admitted developer ability mask — the third stage for the ability domain.
///
/// Same three stages as [`ActivePlayerBodyProfile`]. Without this resource,
/// both admission and projection depend on a primary player existing, so a
/// pending proposal re-enters the admission decision every frame while the
/// player is absent. With it, a body built later projects the admitted mask,
/// not whatever the editor holds at that time.
///
/// `None` means nothing is admitted yet. The first publish seeds it from the
/// editable, so the primary player has the mask from the first tick.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ActiveEditableAbilityMask(pub Option<ae::AbilitySet>);

pub fn sync_developer_body_profile(
    developer: Res<DeveloperTools>,
    admission: Option<Res<ae::MechanicalEditAdmission>>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
    mut active: ResMut<ActivePlayerBodyProfile>,
) {
    // Only this domain's proposal, and only when admitted. See
    // `PendingMechanicalEdits`.
    if !pending.is_pending(body_profile_domain()) {
        return;
    }
    if matches!(
        admission.as_deref(),
        Some(ae::MechanicalEditAdmission::Refuse)
    ) {
        return;
    }
    // Store the admitted value whether or not a body exists. A player built later
    // reads it from here, so draining the proposal is safe.
    active.0 = Some(developer.player_body_profile);
    pending.take(body_profile_domain());
}

/// Project the admitted body profile onto whatever primary player exists.
///
/// This is a projection, not a publication. It re-applies whenever the live
/// body differs from the admitted value, so a reset or room load that rebuilds
/// the player from engine defaults gets the developer's profile back.
///
/// It writes nothing until a value is admitted. `None` means the developer has
/// not chosen, and a default must not overwrite an authored body.
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

/// Reflected, debug-editable player gameplay stats. Surfaced through the
/// `F3` resource inspector so testers can:
///
/// - read live HP / max HP / mana / max mana (fields synced from runtime
///   each frame),
/// - rewrite them in-place (clicking the field commits a "set" each
///   frame the value differs from the runtime),
/// - toggle `invincible` to stop incoming damage entirely while testing
///   downstream systems (boss phase, encounter pacing, music swaps).
#[derive(Resource, Reflect, Clone, Copy, Debug)]
#[reflect(Resource)]
pub struct EditablePlayerStats {
    pub health: i32,
    pub max_health: i32,
    pub mana: i32,
    pub max_mana: i32,
    /// True → all `HitEvent`s are ignored before they reach
    /// `handle_player_damage_events`.
    pub invincible: bool,
    /// True → fully refill HP & mana on the next frame's sync.
    pub refill_now: bool,
}

impl EditablePlayerStats {
    pub const DEFAULT_MAX_HEALTH: i32 = 5;
    pub const DEFAULT_MAX_MANA: i32 = 100;
}

impl Default for EditablePlayerStats {
    fn default() -> Self {
        Self {
            health: Self::DEFAULT_MAX_HEALTH,
            max_health: Self::DEFAULT_MAX_HEALTH,
            mana: Self::DEFAULT_MAX_MANA,
            max_mana: Self::DEFAULT_MAX_MANA,
            invincible: false,
            refill_now: false,
        }
    }
}

/// Last-synced stats snapshot, used to tell user edits apart from runtime drift.
///
/// Without it, any frame where gameplay damaged HP would see
/// `stats.health != live_hp` and push the stale inspector value back into the
/// runtime, undoing the damage.
///
/// A resource, not a `Local`: three systems share it (the proposer, the
/// publisher and the body-to-inspector mirror). They run host-side, once per
/// rendered frame, not once per rollback advance.
#[derive(bevy::prelude::Resource, Default)]
pub struct PlayerStatsSyncSnapshot {
    initialized: bool,
    health: i32,
    max_health: i32,
    // Snapshot mana too, so the publisher writes mana only when the developer
    // changed it, as it does for health.
    mana: i32,
    max_mana: i32,
}

/// Marker type that owns the player-stats domain in
/// [`ae::PendingMechanicalEdits`].
pub struct EditablePlayerStatsDomain;

pub fn player_stats_domain() -> ae::MechanicalDomain {
    ae::MechanicalDomain::of::<EditablePlayerStatsDomain>("editable_player_stats")
}

/// Raise a developer stat edit as a proposal.
///
/// `stats.is_changed()` is the wrong test here. The body-to-inspector mirror
/// writes `EditablePlayerStats` whenever gameplay changes HP, so change
/// detection would raise a proposal (and stop the rollback baseline) on every
/// hit. The snapshot separates "the developer typed a number" from "the game
/// changed one".
pub fn propose_player_stats_edits(
    stats: Res<EditablePlayerStats>,
    snapshot: Res<PlayerStatsSyncSnapshot>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
) {
    if !snapshot.initialized {
        // The publisher sets the baseline on the first frame. Proposing before that
        // would treat the defaults as an edit.
        return;
    }
    // `refill_now` is an edit even though no value changed yet. A refill staged
    // behind a refusal must fire when the refusal lifts.
    if stats.refill_now
        || stats.health != snapshot.health
        || stats.max_health != snapshot.max_health
        || stats.mana != snapshot.mana
        || stats.max_mana != snapshot.max_mana
    {
        pending.propose(player_stats_domain());
    }
}

/// Publish an admitted developer stat edit onto the live player.
///
/// The stats sync is three systems: this one writes to the body, and only after
/// admission; [`mirror_player_stats_into_the_inspector`] reads the body back
/// into the panel; [`propose_player_stats_edits`] detects that the developer,
/// not gameplay, changed a value. None of them writes inside the rollback
/// window.
///
/// `refill_now` is consumed here, not at the proposal, so a refill staged
/// behind a refusal still fires when the refusal lifts.
pub fn publish_player_stats_edits(
    mut stats: ResMut<EditablePlayerStats>,
    admission: Option<Res<ae::MechanicalEditAdmission>>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
    mut snapshot: ResMut<PlayerStatsSyncSnapshot>,
    mut player_q: Query<
        Option<&mut ambition_platformer2d_core::resources::ActorResources>,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    mut health_q: Query<
        &mut ambition_characters::actor::BodyHealth,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
) {
    // The baseline is set only here, so the proposer has one definition of "what
    // the developer last saw".
    if !snapshot.initialized {
        snapshot.health = stats.health;
        snapshot.max_health = stats.max_health;
        snapshot.mana = stats.mana;
        snapshot.max_mana = stats.max_mana;
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
    // Mana is the body's banked `abilities::mana::MANA` level; the inspector
    // surfaces i32 fields for player-friendly editing and the conversion happens
    // at this boundary. A body that holds no Mana is not given any by an edit —
    // the pool is the experience's declaration, not the inspector's.
    //
    // Write mana only if the developer changed a mana field. Otherwise a health
    // edit would write a stale panel mana back to the body:
    //
    // ```text
    //   gameplay spends mana      live 40, inspector still 100
    //   developer edits max_health only
    //     → the stats domain becomes pending
    //     → health is applied  (correctly, conditionally)
    //     → AND mana is written back to the stale 100
    //   ⇒ a HEALTH edit refilled mana.
    // ```
    let user_changed_mana = stats.mana != snapshot.mana || stats.max_mana != snapshot.max_mana;
    let max_mana = stats.max_mana.max(0);
    if let Ok(mut resources) = player_q.single_mut() {
        if let Some(mana) = resources
            .as_deref_mut()
            .and_then(|bank| bank.level_of_mut(&ambition_entity_catalog::mana::MANA))
            .filter(|_| user_changed_mana)
        {
            mana.max = max_mana as f32;
            mana.current = stats.mana.clamp(0, max_mana) as f32;
        }
    }

    snapshot.health = stats.health;
    snapshot.max_health = stats.max_health;
    snapshot.mana = stats.mana;
    snapshot.max_mana = stats.max_mana;
    pending.take(player_stats_domain());
}

/// Mirror the live player's health back into the inspector, so the F3 panel
/// shows truth.
///
/// This half is presentation. It reads the body and writes the developer
/// resource, and changes nothing the simulation reads, so it is not behind the
/// admission boundary.
///
/// It does not run while this domain has a pending proposal. Otherwise the
/// body's value would overwrite a staged edit, and a refusal would look like a
/// discard.
pub fn mirror_player_stats_into_the_inspector(
    mut stats: ResMut<EditablePlayerStats>,
    mut snapshot: ResMut<PlayerStatsSyncSnapshot>,
    pending: Res<ae::PendingMechanicalEdits>,
    health_q: Query<
        &ambition_characters::actor::BodyHealth,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    live_q: Query<
        Option<&ambition_platformer2d_core::resources::ActorResources>,
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
    // Mirror mana too. With health only, the panel kept a stale mana that the
    // publisher could write back. Update `stats` and `snapshot` together so that
    // gameplay mana use does not look like a proposal.
    if let Ok(resources) = live_q.single() {
        // A body without Mana reads 0/0 — the panel's i32 fields have no absent.
        let mana = resources.and_then(|bank| bank.level_of(&ambition_entity_catalog::mana::MANA));
        stats.mana = mana.map_or(0, |mana| mana.current.round() as i32);
        stats.max_mana = mana.map_or(0, |mana| mana.max.round() as i32);
        snapshot.mana = stats.mana;
        snapshot.max_mana = stats.max_mana;
    }
}

/// The editor adapter: push the inspector's live movement edits into the
/// simulation's neutral authority.
///
/// [`EditableMovementTuning`] is a reflected mirror, so bevy-inspector-egui
/// can edit tuning without `MovementTuning` deriving `Reflect`. The simulation
/// must not read it. Sim systems read `ae::ActiveMovementTuning`, so a build
/// without developer tools does not install this system and keeps the
/// authored tuning.
///
/// This is the publish half; see [`propose_editable_movement_tuning`] for the
/// proposal. It writes only when admitted.
///
/// It is not change-guarded; the proposal is. An untouched inspector raises no
/// proposal, so this does not write. Adding `is_changed` here would drop every
/// edit that was staged for a frame.
pub fn publish_editable_movement_tuning(
    editable: Res<EditableMovementTuning>,
    admission: Option<Res<ae::MechanicalEditAdmission>>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
    mut active: ResMut<ae::ActiveMovementTuning>,
) {
    // Ask about this domain, not the batch: a single global flag let one
    // publisher clear another domain's edit. See `PendingMechanicalEdits`.
    if !pending.is_pending(movement_tuning_domain()) {
        return;
    }
    // Absent means publish, as the resource's default does. A composition with no
    // rollback host has no history to contradict. The host that can refuse always
    // installs the resource.
    if matches!(
        admission.as_deref(),
        Some(ae::MechanicalEditAdmission::Refuse)
    ) {
        return;
    }
    active.0 = editable.as_engine();
    // Drain only this domain. `PendingMechanicalEdits` cannot drain another
    // domain's proposal.
    pending.take(movement_tuning_domain());
}

/// Raise the developer's movement-tuning edit as a proposal.
///
/// The edit does not reach the simulation here. In the sim schedule
/// (`GgrsSchedule` under the rollback host) the value would change inside the
/// rollback window, where a resimulation of confirmed frames reads it and
/// desyncs. A watcher in `Update` cannot fix this, because `Update` runs after
/// `RunGgrsSystems`.
///
/// The chain: propose in [`MechanicalEditSet::Propose`], the timeline owner
/// answers in `Admit`, and [`publish_editable_movement_tuning`] writes in
/// `Publish`. All of it is in `PreUpdate`, before the advance.
///
/// `is_added` is excluded: Bevy counts insertion as a change, and the mirror is
/// inserted with defaults before content seeds tuning. Proposing that would
/// overwrite authored tuning on frame one and stop the new session.
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
/// Declared beside the value it owns, not in a central enum. A central enum
/// would make `ambition_platformer2d_core` name every owning crate.
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

    /// A refused edit does not move the value the simulation reads, and it is not
    /// discarded. This host cannot stop an external or caller-owned timeline, so
    /// it holds the proposal. A dropped edit would leave the inspector showing a
    /// value that never publishes.
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

        // Several frames while refused: the proposal persists after `is_changed`
        // goes quiet.
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

    /// Live editing still works: an F3 panel edit reaches the value the simulation
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

    /// An edit must not delete the rules it was not about (D186). A fighter
    /// carries `parry_timing` and the five evade-staling / tech rules in one
    /// authored preset. The loss happens on the way back out, so the test edits
    /// one unrelated knob and reads the others.
    #[test]
    fn editing_one_knob_does_not_wipe_the_rules_it_was_not_about() {
        let declared = ae::MovementTuning {
            // `OnRelease`, not `OnRaise`: `OnRaise` is the default, so it would not
            // detect a reset to `default()`.
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

    /// The crouch cost and the four ground-movement phase timings are carried too.
    /// A game with its own dash phases must keep them after an unrelated edit.
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
    /// The mirror is last on purpose: in the shipped app it is in the sim
    /// schedule, which runs after `PreUpdate`.
    fn app_with_the_stats_domain() -> App {
        let mut app = App::new();
        app.init_resource::<EditablePlayerStats>();
        app.init_resource::<PlayerStatsSyncSnapshot>();
        app.init_resource::<ae::PendingMechanicalEdits>();
        app.init_resource::<ae::MechanicalEditAdmission>();
        // An observer between propose and publish. The publisher drains the proposal
        // in the same frame, so "not pending after the update" cannot detect
        // over-proposing. Over-proposing would stop and rebase the rollback baseline
        // on every hit.
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
            ambition_platformer2d_core::resources::ActorResources::declared(&[
                ambition_entity_catalog::mana::POOL,
            ])
            .expect("valid")
            .expect("declared"),
        ));
        // The first update establishes the baseline and must change nothing.
        app.update();
        app
    }

    /// How many frames the domain was pending just before the publisher ran.
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

    fn live_mana(app: &mut App) -> f32 {
        let world = app.world_mut();
        let mut query = world.query_filtered::<
            &ambition_platformer2d_core::resources::ActorResources,
            ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
        >();
        let bank = query.single(world).expect("the fixture has a player body");
        bank.level_of(&ambition_entity_catalog::mana::MANA).expect("the fixture holds Mana").current
    }

    /// Editing one field must not republish the others. If the publisher writes
    /// every field and the mirror reads back only health, the panel's mana goes
    /// stale, and the next unrelated edit writes it back:
    ///
    /// ```text
    ///   gameplay spends mana       live 31, inspector still says 100
    ///   developer edits max_health ONLY
    ///     → health applied, correctly
    ///     → AND mana written back to 100
    ///   ⇒ a HEALTH edit refilled mana.
    /// ```
    ///
    /// The premise is asserted first: live mana must differ from the inspector's
    /// default, or the test cannot detect the defect.
    #[test]
    fn editing_one_stat_leaves_the_others_where_gameplay_put_them() {
        let mut app = app_with_the_stats_domain();

        // Gameplay spends mana; this is not a developer edit.
        {
            let world = app.world_mut();
            let mut query = world.query_filtered::<
                &mut ambition_platformer2d_core::resources::ActorResources,
                ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
            >();
            let mut bank = query.single_mut(world).expect("player body");
            let mana = bank
                .level_of_mut(&ambition_entity_catalog::mana::MANA)
                .expect("the fixture holds Mana");
            mana.max = 100.0;
            mana.current = 31.0;
        }
        // Let the mirror copy that into the panel before the edit.
        app.update();
        let mana_before = live_mana(&mut app);
        assert_eq!(
            mana_before, 31.0,
            "gameplay's own write did not survive a frame, so this fixture is \
             not exercising the case at all",
        );

        // Premise: the panel's default mana must differ from what gameplay wrote, or
        // a stale write-back would be invisible.
        let defaults = EditablePlayerStats::default();
        assert!(
            defaults.mana as f32 != mana_before,
            "the editor's default mana ({}) already equals what gameplay wrote \
             ({mana_before}), so writing the panel back over the body would \
             change nothing and this arm cannot see the defect",
            defaults.mana,
        );

        // Edit only max health.
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
        let mana_after = live_mana(&mut app);
        assert_eq!(
            mana_after, 31.0,
            "a HEALTH edit refilled mana: the publisher writes this domain's \
             every field on any admitted proposal, and the panel's value was \
             stale because the mirror never read mana back",
        );
    }

    /// This test isolates the publisher's half. The test above fails only when
    /// both the field-conditional publish and the full mirror are removed, so it
    /// guards the pair, not each one.
    ///
    /// The case that separates them is an edit staged behind a refusal. The mirror
    /// does not run while the domain is pending, so the panel goes stale while
    /// gameplay moves the body. When the refusal lifts, an unconditional publisher
    /// writes the stale mana back.
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
                &mut ambition_platformer2d_core::resources::ActorResources,
                ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
            >();
            let mut bank = query.single_mut(world).expect("player body");
            let mana = bank
                .level_of_mut(&ambition_entity_catalog::mana::MANA)
                .expect("the fixture holds Mana");
            mana.max = 100.0;
            mana.current = 31.0;
        }

        // Premise: the edit is still staged, and the panel disagrees with the body
        // about mana.
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
        let mana_after = live_mana(&mut app);
        assert_eq!(
            mana_after, 31.0,
            "publishing a staged HEALTH edit wrote the panel's frozen mana back \
             over the value gameplay had reached: the publisher writes every \
             field of this domain on any admitted proposal",
        );
    }

    /// The field the developer does change still lands. This fails if the fix
    /// simply stopped publishing mana.
    #[test]
    fn editing_mana_alone_still_publishes_it() {
        let mut app = app_with_the_stats_domain();
        {
            let mut stats = app.world_mut().resource_mut::<EditablePlayerStats>();
            stats.max_mana = 80;
            stats.mana = 55;
        }
        app.update();
        let mana = live_mana(&mut app);
        assert_eq!(mana, 55.0, "an explicit mana edit did not reach the body");
    }

    /// Gameplay damage must not look like a developer edit. The mirror writes
    /// `EditablePlayerStats` whenever gameplay changes HP, so a proposer based on
    /// `is_changed()` would stop the rollback baseline on every hit.
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

    /// Control for the test above: a developer edit proposes and reaches the body.
    /// Without it, a domain that never proposes would pass.
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

    /// A refused edit is staged, and the mirror must not overwrite it with the
    /// body's current value. Otherwise a refusal looks like a discard.
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
        // Derived from the type's own list, so a new profile cannot make this pick
        // the default.
        PlayerBodyProfile::ALL
            .into_iter()
        .find(|profile| (profile.size() - default_size).length_squared() > 0.01)
        .expect("some shipped profile differs from the default")
    }

    /// An edit made while no player exists reaches the player that appears later.
    /// If the proposal were drained without storing the admitted value, the edit
    /// would be lost, and the proposer's `Local` would not propose it again.
    #[test]
    fn a_profile_admitted_with_no_player_reaches_the_player_that_appears_later() {
        let mut app = app_with_the_body_profile_domain();
        let profile = a_distinct_profile();

        // Premise: no player exists yet.
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

    /// A rebuilt body gets the profile back. A reset or room load rebuilds the
    /// player from engine defaults without an editor change, so nothing proposes;
    /// only the projection can restore it.
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

    /// A refused edit does not move the admitted authority. Here the authority is
    /// the admitted profile, not the body.
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
