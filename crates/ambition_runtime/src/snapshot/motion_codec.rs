//! The movement-policy (`MotionModel`) snapshot codec — ADR 0024 §9.
//!
//! Split from `codecs.rs` for the D-B module-size gate; shares the module core
//! via `use super::*`. Explicit field order, fixed-width LE, every field
//! present — the same discipline as every other codec.
use super::*;

/// The body's explicit movement policy: identity, authored parameters, and
/// policy-private runtime state — everything a deterministic continuation
/// needs. The current environmental frame is deliberately NOT here: after a
/// restore, the frame is resolved from the live restored environment.
impl SnapshotState for ambition_engine_core::MotionModel {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_engine_core::MotionModel;
        match self {
            MotionModel::AxisSwept(motion) => {
                put_u8(out, 0);
                put_axis_swept_params(out, &motion.params);
                put_axis_maneuver_state(out, &motion.state);
            }
            MotionModel::SurfaceMomentum(motion) => {
                put_u8(out, 1);
                put_momentum_params(out, &motion.params);
                put_surface_motion(out, motion.state);
                put_u8(out, motion.depth_lane as u8);
            }
            MotionModel::AdhesiveCrawler(motion) => {
                put_u8(out, 2);
                put_f32(out, motion.params.crawl_speed);
                put_f32(out, motion.params.max_fall_speed);
                match motion.state.attachment() {
                    Some(normal) => {
                        put_bool(out, true);
                        put_vec2(out, normal);
                    }
                    None => put_bool(out, false),
                }
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_engine_core::{
            AdhesiveCrawlerMotion, AxisSweptMotion, CrawlerParams, CrawlerState, MotionModel,
            SurfaceMomentumMotion,
        };
        Some(match r.u8()? {
            0 => MotionModel::AxisSwept(AxisSweptMotion {
                params: axis_swept_params(r)?,
                state: axis_maneuver_state(r)?,
            }),
            1 => {
                let params = momentum_params(r)?;
                let state = surface_motion(r)?;
                let depth_lane = r.u8()? as i8;
                MotionModel::SurfaceMomentum(SurfaceMomentumMotion {
                    params,
                    state,
                    depth_lane,
                })
            }
            2 => {
                let params = CrawlerParams {
                    crawl_speed: r.f32()?,
                    max_fall_speed: r.f32()?,
                };
                let state = if r.bool()? {
                    CrawlerState::attached(r.vec2()?)
                } else {
                    CrawlerState::DETACHED
                };
                MotionModel::AdhesiveCrawler(AdhesiveCrawlerMotion { params, state })
            }
            _ => return None,
        })
    }
}

/// The axis policy's PRIVATE maneuver state (ADR 0024 O4) — every field, in
/// declaration order, so a rollback into a dash / blink hold / ledge hang
/// resumes exactly where it left off.
fn put_axis_maneuver_state(out: &mut Vec<u8>, state: &ambition_engine_core::AxisManeuverState) {
    put_f32(out, state.coyote_timer);
    put_f32(out, state.drop_through_timer);
    put_f32(out, state.rebound_cooldown);
    put_bool(out, state.wall_clinging);
    put_bool(out, state.wall_climbing);
    put_vec2(out, state.pre_wall_vel);
    put_f32(out, state.pre_wall_vel_age);
    put_f32(out, state.buffer_jump);
    put_f32(out, state.buffer_dash);
    put_f32(out, state.buffer_blink);
    put_f32(out, state.dash_timer);
    put_bool(out, state.blink_hold_active);
    put_f32(out, state.blink_hold_timer);
    put_bool(out, state.blink_aiming);
    put_vec2(out, state.blink_aim_offset);
    put_f32(out, state.blink_grace_timer);
    put_f32(out, state.dodge_roll_timer);
    put_ledge_grab(out, &state.ledge_grab);
    put_bool(out, state.gliding);
    put_bool(out, state.fast_falling);
    put_f32(out, state.flight_phase);
}

fn axis_maneuver_state(r: &mut Reader<'_>) -> Option<ambition_engine_core::AxisManeuverState> {
    Some(ambition_engine_core::AxisManeuverState {
        coyote_timer: r.f32()?,
        drop_through_timer: r.f32()?,
        rebound_cooldown: r.f32()?,
        wall_clinging: r.bool()?,
        wall_climbing: r.bool()?,
        pre_wall_vel: r.vec2()?,
        pre_wall_vel_age: r.f32()?,
        buffer_jump: r.f32()?,
        buffer_dash: r.f32()?,
        buffer_blink: r.f32()?,
        dash_timer: r.f32()?,
        blink_hold_active: r.bool()?,
        blink_hold_timer: r.f32()?,
        blink_aiming: r.bool()?,
        blink_aim_offset: r.vec2()?,
        blink_grace_timer: r.f32()?,
        dodge_roll_timer: r.f32()?,
        ledge_grab: ledge_grab(r)?,
        gliding: r.bool()?,
        fast_falling: r.bool()?,
        flight_phase: r.f32()?,
    })
}

/// The hang state machine: a rollback into a hang must land on the same
/// anchor, with the same carried momentum, or the getup goes somewhere else.
fn put_ledge_grab(out: &mut Vec<u8>, grab: &Option<ambition_engine_core::LedgeGrabState>) {
    match grab {
        None => put_bool(out, false),
        Some(g) => {
            put_bool(out, true);
            put_f32(out, g.contact.wall_normal_x);
            put_vec2(out, g.contact.anchor);
            put_vec2(out, g.contact.climb_target);
            put_f32(out, g.elapsed);
            put_bool(out, g.climbing);
            g.getup_kind.encode(out);
            put_f32(out, g.climb_elapsed);
            put_vec2(out, g.momentum_at_grab);
            g.grab_quality.encode(out);
        }
    }
}

fn ledge_grab(r: &mut Reader<'_>) -> Option<Option<ambition_engine_core::LedgeGrabState>> {
    use ambition_engine_core::ledge_grab::{
        LedgeContact, LedgeGetupKind, LedgeGrabQuality, LedgeGrabState,
    };
    Some(if r.bool()? {
        Some(LedgeGrabState {
            contact: LedgeContact {
                wall_normal_x: r.f32()?,
                anchor: r.vec2()?,
                climb_target: r.vec2()?,
            },
            elapsed: r.f32()?,
            climbing: r.bool()?,
            getup_kind: LedgeGetupKind::decode(r)?,
            climb_elapsed: r.f32()?,
            momentum_at_grab: r.vec2()?,
            grab_quality: LedgeGrabQuality::decode(r)?,
        })
    } else {
        None
    })
}

fn put_surface_motion(out: &mut Vec<u8>, state: ambition_engine_core::SurfaceMotion) {
    use ambition_engine_core::{SurfaceMotion, SurfaceRef};
    match state {
        SurfaceMotion::Airborne => put_u8(out, 0),
        SurfaceMotion::Riding { on, s, v_t } => {
            put_u8(out, 1);
            match on {
                SurfaceRef::Chain(i) => {
                    put_u8(out, 0);
                    put_u32(out, i as u32);
                }
                SurfaceRef::Block(i) => {
                    put_u8(out, 1);
                    put_u32(out, i as u32);
                }
            }
            put_f32(out, s);
            put_f32(out, v_t);
        }
    }
}

fn surface_motion(r: &mut Reader<'_>) -> Option<ambition_engine_core::SurfaceMotion> {
    use ambition_engine_core::{SurfaceMotion, SurfaceRef};
    Some(match r.u8()? {
        0 => SurfaceMotion::Airborne,
        1 => {
            let on = match r.u8()? {
                0 => SurfaceRef::Chain(r.u32()? as usize),
                1 => SurfaceRef::Block(r.u32()? as usize),
                _ => return None,
            };
            SurfaceMotion::Riding {
                on,
                s: r.f32()?,
                v_t: r.f32()?,
            }
        }
        _ => return None,
    })
}

fn put_momentum_params(out: &mut Vec<u8>, p: &ambition_engine_core::MomentumParams) {
    put_f32(out, p.ground_accel);
    put_f32(out, p.brake);
    put_f32(out, p.friction);
    put_f32(out, p.slope_factor);
    put_f32(out, p.top_speed);
    put_f32(out, p.air_accel);
    put_f32(out, p.jump_speed);
    put_f32(out, p.stick_factor);
    put_f32(out, p.min_stick_speed);
}

fn momentum_params(r: &mut Reader<'_>) -> Option<ambition_engine_core::MomentumParams> {
    Some(ambition_engine_core::MomentumParams {
        ground_accel: r.f32()?,
        brake: r.f32()?,
        friction: r.f32()?,
        slope_factor: r.f32()?,
        top_speed: r.f32()?,
        air_accel: r.f32()?,
        jump_speed: r.f32()?,
        stick_factor: r.f32()?,
        min_stick_speed: r.f32()?,
    })
}

fn put_axis_swept_params(out: &mut Vec<u8>, p: &ambition_engine_core::AxisSweptParams) {
    let l = &p.locomotion;
    put_f32(out, l.run_accel);
    put_f32(out, l.air_accel);
    put_f32(out, l.ground_friction);
    put_f32(out, l.air_friction);
    put_f32(out, l.air_stop_assist);
    put_f32(out, l.carried_decay);
    put_f32(out, l.max_run_speed);
    put_f32(out, l.max_fall_speed);
    put_f32(out, l.jump_speed);
    put_f32(out, l.double_jump_speed);
    put_f32(out, l.wall_jump_x);
    put_f32(out, l.wall_slide_speed);
    put_f32(out, l.wall_climb_speed);
    put_f32(out, l.coyote_time);
    put_f32(out, l.jump_buffer);
    put_u8(out, l.air_jumps);
    put_f32(out, l.fast_fall_accel);
    put_f32(out, l.fast_fall_speed);
    put_f32(out, l.glide_fall_speed);
    put_f32(out, l.glide_air_accel);
    let a = &p.abilities;
    put_f32(out, a.dash_speed);
    put_f32(out, a.dash_time);
    put_f32(out, a.dash_cooldown);
    put_f32(out, a.dash_buffer);
    put_f32(out, a.blink_distance);
    put_f32(out, a.precision_blink_distance);
    put_f32(out, a.precision_blink_aim_speed);
    put_f32(out, a.blink_hold_threshold);
    put_f32(out, a.blink_cooldown);
    put_f32(out, a.blink_grace_time);
    put_f32(out, a.blink_max_downward_speed);
    put_f32(out, a.precision_blink_max_downward_speed);
    put_f32(out, a.pogo_speed);
    put_f32(out, a.slash_recoil);
    put_f32(out, a.dodge_roll_time);
    put_f32(out, a.dodge_roll_speed);
    put_f32(out, a.dodge_roll_cooldown);
    put_f32(out, a.parry_window_time);
    put_f32(out, a.ledge_momentum.window);
    put_f32(out, a.ledge_momentum.x_gain);
    put_f32(out, a.ledge_momentum.y_gain);
    put_f32(out, a.ledge_momentum.x_cap);
    put_f32(out, a.ledge_momentum.y_cap);
    put_f32(out, a.ledge_momentum.getup_speedup_gain);
    let f = &p.flight;
    put_f32(out, f.accel);
    put_f32(out, f.drag);
    put_f32(out, f.terminal_speed);
    put_f32(out, f.hover_speed);
    put_f32(out, f.hover_hz);
    put_bool(out, f.direct_velocity);
}

fn axis_swept_params(r: &mut Reader<'_>) -> Option<ambition_engine_core::AxisSweptParams> {
    use ambition_engine_core::{
        AxisLocomotion, AxisSweptParams, FlightTuning, LedgeMomentumTuning, TraversalAbilityTuning,
    };
    Some(AxisSweptParams {
        locomotion: AxisLocomotion {
            run_accel: r.f32()?,
            air_accel: r.f32()?,
            ground_friction: r.f32()?,
            air_friction: r.f32()?,
            air_stop_assist: r.f32()?,
            carried_decay: r.f32()?,
            max_run_speed: r.f32()?,
            max_fall_speed: r.f32()?,
            jump_speed: r.f32()?,
            double_jump_speed: r.f32()?,
            wall_jump_x: r.f32()?,
            wall_slide_speed: r.f32()?,
            wall_climb_speed: r.f32()?,
            coyote_time: r.f32()?,
            jump_buffer: r.f32()?,
            air_jumps: r.u8()?,
            fast_fall_accel: r.f32()?,
            fast_fall_speed: r.f32()?,
            glide_fall_speed: r.f32()?,
            glide_air_accel: r.f32()?,
        },
        abilities: TraversalAbilityTuning {
            dash_speed: r.f32()?,
            dash_time: r.f32()?,
            dash_cooldown: r.f32()?,
            dash_buffer: r.f32()?,
            blink_distance: r.f32()?,
            precision_blink_distance: r.f32()?,
            precision_blink_aim_speed: r.f32()?,
            blink_hold_threshold: r.f32()?,
            blink_cooldown: r.f32()?,
            blink_grace_time: r.f32()?,
            blink_max_downward_speed: r.f32()?,
            precision_blink_max_downward_speed: r.f32()?,
            pogo_speed: r.f32()?,
            slash_recoil: r.f32()?,
            dodge_roll_time: r.f32()?,
            dodge_roll_speed: r.f32()?,
            dodge_roll_cooldown: r.f32()?,
            parry_window_time: r.f32()?,
            ledge_momentum: LedgeMomentumTuning {
                window: r.f32()?,
                x_gain: r.f32()?,
                y_gain: r.f32()?,
                x_cap: r.f32()?,
                y_cap: r.f32()?,
                getup_speedup_gain: r.f32()?,
            },
        },
        flight: FlightTuning {
            accel: r.f32()?,
            drag: r.f32()?,
            terminal_speed: r.f32()?,
            hover_speed: r.f32()?,
            hover_hz: r.f32()?,
            direct_velocity: r.bool()?,
        },
    })
}
