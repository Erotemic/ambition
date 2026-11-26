use crate::snapshot::{put_bool, put_f32, put_u8, Reader};

// Keep the snapshot field order and the settled-motion comparison in one declaration.
trait AxisField: Sized {
    fn write_axis(&self, out: &mut Vec<u8>);
    fn read_axis(r: &mut Reader<'_>) -> Option<Self>;
    fn same_bits(&self, other: &Self) -> bool;
}

macro_rules! axis_fields {
    ($ty:path { $($field:ident $(=> $codec:ident)?),+ $(,)? }) => {
        impl AxisField for $ty {
            fn write_axis(&self, out: &mut Vec<u8>) {
                $(axis_fields!(@write &self.$field, out $(, $codec)?);)+
            }

            fn read_axis(r: &mut Reader<'_>) -> Option<Self> {
                Some(Self { $($field: axis_fields!(@read r $(, $codec)?)?),+ })
            }

            #[inline]
            fn same_bits(&self, other: &Self) -> bool {
                $(axis_fields!(@compare &self.$field, &other.$field $(, $codec)?))&&+
            }
        }
    };
    (@write $field:expr, $out:ident) => { AxisField::write_axis($field, $out) };
    (@read $r:ident) => { AxisField::read_axis($r) };
    (@compare $a:expr, $b:expr) => { AxisField::same_bits($a, $b) };
    // These shield flags accept any nonzero byte on decode.
    (@write $field:expr, $out:ident, nonzero_bool) => { put_bool($out, *$field) };
    (@read $r:ident, nonzero_bool) => { $r.u8().map(|value| value != 0) };
    (@compare $a:expr, $b:expr, nonzero_bool) => { $a == $b };
    (@write $field:expr, $out:ident, out_of_shield) => { put_u8($out, encode_out_of_shield(*$field)) };
    (@read $r:ident, out_of_shield) => { $r.u8().map(decode_out_of_shield) };
    (@compare $a:expr, $b:expr, out_of_shield) => { $a == $b };
}

macro_rules! axis_scalar {
    ($ty:ty, $put:ident, $get:ident, $key:expr) => {
        impl AxisField for $ty {
            fn write_axis(&self, out: &mut Vec<u8>) {
                $put(out, *self);
            }

            fn read_axis(r: &mut Reader<'_>) -> Option<Self> {
                r.$get()
            }

            #[inline(always)]
            fn same_bits(&self, other: &Self) -> bool {
                $key(*self) == $key(*other)
            }
        }
    };
}

axis_scalar!(f32, put_f32, f32, f32::to_bits);
axis_scalar!(u8, put_u8, u8, std::convert::identity);
axis_scalar!(bool, put_bool, bool, std::convert::identity);

impl<T: AxisField> AxisField for Option<T> {
    fn write_axis(&self, out: &mut Vec<u8>) {
        put_bool(out, self.is_some());
        if let Some(value) = self {
            value.write_axis(out);
        }
    }

    fn read_axis(r: &mut Reader<'_>) -> Option<Self> {
        Some(if r.bool()? {
            Some(T::read_axis(r)?)
        } else {
            None
        })
    }

    #[inline]
    fn same_bits(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => a.same_bits(b),
            (None, None) => true,
            _ => false,
        }
    }
}

impl<const N: usize> AxisField for [f32; N] {
    fn write_axis(&self, out: &mut Vec<u8>) {
        for value in self {
            value.write_axis(out);
        }
    }

    fn read_axis(r: &mut Reader<'_>) -> Option<Self> {
        let mut values = [0.0; N];
        for value in &mut values {
            *value = r.f32()?;
        }
        Some(values)
    }

    #[inline]
    fn same_bits(&self, other: &Self) -> bool {
        self.iter().zip(other).all(|(a, b)| a.same_bits(b))
    }
}

impl AxisField for crate::ParryTiming {
    fn write_axis(&self, out: &mut Vec<u8>) {
        put_u8(
            out,
            match self {
                Self::OnRaise => 0,
                Self::OnRelease => 1,
            },
        );
    }

    fn read_axis(r: &mut Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(Self::OnRaise),
            1 => Some(Self::OnRelease),
            _ => None,
        }
    }

    #[inline]
    fn same_bits(&self, other: &Self) -> bool {
        self == other
    }
}

axis_fields!(crate::MomentumHorizontalTuning {
    ground_reverse_accel,
    ground_coast_decel,
    air_reverse_accel,
    air_coast_decel,
});

axis_fields!(crate::PhasedGravityJumpTuning {
    speed_thresholds,
    launch_offsets,
    held_rise_gravity_scale,
    released_rise_gravity_scale,
    fall_gravity_scale,
    held_phase_min_upward_speed,
});

axis_fields!(crate::SquatDecidedJumpTuning {
    short_hop_speed_frac
});

impl AxisField for crate::AxisHorizontalLaw {
    fn write_axis(&self, out: &mut Vec<u8>) {
        match self {
            Self::Responsive => put_u8(out, 0),
            Self::Momentum(value) => {
                put_u8(out, 1);
                value.write_axis(out);
            }
        }
    }

    fn read_axis(r: &mut Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(Self::Responsive),
            1 => Some(Self::Momentum(AxisField::read_axis(r)?)),
            _ => None,
        }
    }

    #[inline]
    fn same_bits(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Responsive, Self::Responsive) => true,
            (Self::Momentum(a), Self::Momentum(b)) => a.same_bits(b),
            _ => false,
        }
    }
}

impl AxisField for crate::AxisJumpLaw {
    fn write_axis(&self, out: &mut Vec<u8>) {
        match self {
            Self::VelocityCut => put_u8(out, 0),
            Self::PhasedGravity(value) => {
                put_u8(out, 1);
                value.write_axis(out);
            }
            Self::SquatDecided(value) => {
                put_u8(out, 2);
                value.write_axis(out);
            }
        }
    }

    fn read_axis(r: &mut Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(Self::VelocityCut),
            1 => Some(Self::PhasedGravity(AxisField::read_axis(r)?)),
            2 => Some(Self::SquatDecided(AxisField::read_axis(r)?)),
            _ => None,
        }
    }

    #[inline]
    fn same_bits(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::VelocityCut, Self::VelocityCut) => true,
            (Self::PhasedGravity(a), Self::PhasedGravity(b)) => a.same_bits(b),
            (Self::SquatDecided(a), Self::SquatDecided(b)) => a.same_bits(b),
            _ => false,
        }
    }
}

pub(crate) fn same_axis_law(a: &crate::AxisSweptParams, b: &crate::AxisSweptParams) -> bool {
    a.same_bits(b)
}

pub(super) fn put_axis_swept_params(out: &mut Vec<u8>, params: &crate::AxisSweptParams) {
    params.write_axis(out);
}

pub(super) fn axis_swept_params(r: &mut Reader<'_>) -> Option<crate::AxisSweptParams> {
    crate::AxisSweptParams::read_axis(r)
}

axis_fields!(crate::AxisSweptParams {
    locomotion,
    abilities,
    flight,
});

axis_fields!(crate::AxisLocomotion {
    horizontal_law,
    jump_law,
    run_accel,
    air_accel,
    ground_friction,
    air_friction,
    air_stop_assist,
    carried_decay,
    launch_decay,
    max_run_speed,
    max_air_speed,
    run_commit_frac,
    crouch_speed_frac,
    initial_dash_time,
    initial_dash_speed,
    dash_deflection,
    walk_speed,
    turnaround_time,
    holds_facing_in_air,
    teeter_margin,
    balance,
    max_fall_speed,
    jump_speed,
    double_jump_speed,
    wall_jump_x,
    wall_slide_speed,
    wall_climb_speed,
    coyote_time,
    jump_buffer,
    jump_squat_time,
    air_jumps,
    fast_fall_accel,
    fast_fall_speed,
    glide_fall_speed,
    glide_air_accel,
});

axis_fields!(crate::TraversalAbilityTuning {
    dash_speed,
    dash_time,
    dash_cooldown,
    dash_buffer,
    blink_distance,
    precision_blink_distance,
    precision_blink_aim_speed,
    blink_hold_threshold,
    blink_cooldown,
    blink_grace_time,
    blink_max_downward_speed,
    precision_blink_max_downward_speed,
    pogo_speed,
    slash_recoil,
    dodge_roll_time,
    dodge_roll_speed,
    dodge_roll_cooldown,
    air_dodge_time,
    air_dodge_speed,
    air_dodge_endlag,
    directional_air_dodge_time,
    directional_air_dodge_endlag,
    air_dodge_landing_lag,
    directional_air_dodge_landing_lag,
    dodge_roll_endlag,
    dodge_stale_step,
    dodge_stale_floor,
    dodge_stale_recovery,
    untechable_launch_speed,
    evade_cancel_tail,
    tumble_speed,
    spot_dodge_time,
    spot_dodge_endlag,
    sdi_step,
    asdi_step,
    jab_lock_speed,
    jab_lock_limit,
    parry_window_time,
    parry_timing,
    shield,
    footstool,
    ledge_momentum,
});

axis_fields!(crate::ShieldTuning {
    max_health,
    drain_per_second,
    regen_per_second,
    damage_scale,
    break_stun_time,
    stun_per_damage,
    pushback_per_damage,
    min_coverage,
    tilt_range,
    drop_lag,
    // Bit zero marks the presence of an out-of-shield policy.
    out_of_shield => out_of_shield,
    air_guard => nonzero_bool,
    platform_drop => nonzero_bool,
});

axis_fields!(crate::FootstoolTuning {
    rise_speed,
    press_speed,
    flinch_time,
    air_tumble_time,
    stomper_invuln,
    band,
});

axis_fields!(crate::LedgeMomentumTuning {
    window,
    x_gain,
    y_gain,
    x_cap,
    y_cap,
    getup_speedup_gain,
});

axis_fields!(crate::FlightTuning {
    accel,
    drag,
    terminal_speed,
    hover_speed,
    hover_hz,
    direct_velocity,
    invariant_speed,
});

axis_fields!(crate::Balance {
    teeter_band,
    tip_speed,
});

fn encode_out_of_shield(policy: Option<crate::OutOfShield>) -> u8 {
    match policy {
        None => 0,
        Some(p) => {
            1 | ((p.jump as u8) << 1)
                | ((p.burst as u8) << 2)
                | ((p.grab as u8) << 3)
                | ((p.up_attack as u8) << 4)
                | ((p.up_special as u8) << 5)
        }
    }
}

fn decode_out_of_shield(bits: u8) -> Option<crate::OutOfShield> {
    (bits & 1 != 0).then(|| crate::OutOfShield {
        jump: bits & (1 << 1) != 0,
        burst: bits & (1 << 2) != 0,
        grab: bits & (1 << 3) != 0,
        up_attack: bits & (1 << 4) != 0,
        up_special: bits & (1 << 5) != 0,
    })
}
