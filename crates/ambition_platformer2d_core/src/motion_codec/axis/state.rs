use crate::snapshot::{put_u8, put_vec2, Reader, SnapshotState};

use super::AxisField;

pub(in crate::motion_codec) fn put_axis_maneuver_state(
    out: &mut Vec<u8>,
    state: &crate::AxisManeuverState,
) {
    state.write_axis(out);
}

pub(in crate::motion_codec) fn axis_maneuver_state(
    r: &mut Reader<'_>,
) -> Option<crate::AxisManeuverState> {
    crate::AxisManeuverState::read_axis(r)
}

impl AxisField for crate::Vec2 {
    fn write_axis(&self, out: &mut Vec<u8>) {
        put_vec2(out, *self);
    }

    fn read_axis(r: &mut Reader<'_>) -> Option<Self> {
        r.vec2()
    }

    #[inline(always)]
    fn same_bits(&self, other: &Self) -> bool {
        crate::snapshot::ExactState::same_bits(self, other)
    }
}

impl AxisField for Option<crate::movement::knockdown::LaunchDirection> {
    fn write_axis(&self, out: &mut Vec<u8>) {
        put_u8(
            out,
            self.map_or(0, crate::movement::knockdown::LaunchDirection::code),
        );
    }

    fn read_axis(r: &mut Reader<'_>) -> Option<Self> {
        Some(match r.u8()? {
            0 => None,
            code => Some(crate::movement::knockdown::LaunchDirection::from_code(
                code,
            )?),
        })
    }

    #[inline]
    fn same_bits(&self, other: &Self) -> bool {
        self == other
    }
}

impl AxisField for crate::movement::knockdown::GetupKind {
    fn write_axis(&self, out: &mut Vec<u8>) {
        put_u8(out, self.code());
    }

    fn read_axis(r: &mut Reader<'_>) -> Option<Self> {
        Self::from_code(r.u8()?)
    }

    #[inline]
    fn same_bits(&self, other: &Self) -> bool {
        self == other
    }
}

macro_rules! axis_use_enum_codec {
    ($ty:ty) => {
        impl AxisField for $ty {
            fn write_axis(&self, out: &mut Vec<u8>) {
                self.encode(out);
            }

            fn read_axis(r: &mut Reader<'_>) -> Option<Self> {
                Self::decode(r)
            }

            #[inline]
            fn same_bits(&self, other: &Self) -> bool {
                self == other
            }
        }
    };
}

axis_use_enum_codec!(crate::ledge_grab::LedgeGetupKind);
axis_use_enum_codec!(crate::ledge_grab::LedgeGrabQuality);

axis_fields!(crate::AxisManeuverState {
    coyote_timer,
    drop_through_timer,
    rebound_cooldown,
    wall_clinging,
    wall_climbing,
    pre_wall_vel,
    pre_wall_vel_age,
    time_off_ledge,
    ledge_grabs,
    buffer_jump,
    jump_squat_timer,
    buffer_burst,
    buffer_blink,
    dash_timer,
    blink_hold_active,
    blink_hold_timer,
    blink_aiming,
    blink_aim_offset,
    blink_grace_timer,
    held_velocity_timer,
    dodge_roll_timer,
    evade_invuln_timer,
    dodge_roll_push,
    jab_locks,
    initial_dash_timer,
    initial_dash_dir,
    prev_steer_dir,
    turnaround_timer,
    teetering,
    air_jump_rise_owned,
    ledge_invuln_timer,
    ledge_vulnerable_timer,
    spot_dodging,
    air_dodge_timer,
    air_dodge_push,
    air_dodge_endlag_timer,
    air_dodge_directional,
    air_dodge_up,
    dodge_roll_endlag_timer,
    tumble_timer,
    tumble_until_landing,
    tumble_untechable,
    tumble_unannounced,
    tech_press_timer,
    tech_lockout_timer,
    knockdown_timer,
    getup_invuln_timer,
    launch_direction,
    prone_face_down,
    getup_kind,
    ledge_grab,
    wire,
    gliding,
    fast_falling,
    running,
    flight_phase,
    phased_jump,
});

axis_fields!(crate::PhasedJumpState {
    active,
    launch_band,
    hold_cancelled,
});

// The wire retains the authored settings of the move that started it.
axis_fields!(crate::WireState {
    anchor,
    length,
    angle,
    ang_vel,
    winch_speed,
    lift_remaining_s,
    lift_total_s,
    max_angle,
    swing_accel,
    release_rise,
});

axis_fields!(crate::LedgeContact {
    wall_normal_x,
    anchor,
    climb_target,
});

axis_fields!(crate::LedgeGrabState {
    contact,
    elapsed,
    climbing,
    getup_kind,
    climb_elapsed,
    momentum_at_grab,
    grab_quality,
});
