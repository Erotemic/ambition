//! The movement-policy (`MotionModel`) rollback checksum codec — ADR 0024 §9.

use crate::snapshot::{put_bool, put_f32, put_u32, put_u8, put_vec2, Reader, SnapshotState};

/// The body's explicit movement policy: identity, authored parameters, and
/// policy-private runtime state — everything a deterministic continuation
/// needs. The current environmental frame is deliberately NOT here: after a
/// restore, the frame is resolved from the live restored environment.
impl SnapshotState for crate::MotionModel {
    fn encode(&self, out: &mut Vec<u8>) {
        use crate::MotionModel;
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
                match motion.route_memory {
                    None => put_bool(out, false),
                    Some(memory) => {
                        put_bool(out, true);
                        put_u32(out, memory.chain as u32);
                        put_u32(out, memory.vertex as u32);
                        put_u8(out, memory.direction as u8);
                    }
                }
                let span_count = motion.occlusions.iter().count();
                put_u8(out, span_count as u8);
                for span in motion.occlusions.iter() {
                    put_u32(out, span.chain as u32);
                    put_u32(out, span.first_segment as u32);
                    put_u32(out, span.last_segment as u32);
                }
            }
            MotionModel::AdhesiveCrawler(motion) => {
                put_u8(out, 2);
                put_f32(out, motion.params.crawl_speed);
                put_f32(out, motion.params.max_fall_speed);
                match motion.state.attachment() {
                    None => put_u8(out, 0),
                    Some(crate::CrawlAttachment::Block { normal }) => {
                        put_u8(out, 1);
                        put_vec2(out, normal);
                    }
                    Some(crate::CrawlAttachment::Chain { chain, s }) => {
                        put_u8(out, 2);
                        put_u32(out, chain);
                        put_f32(out, s);
                    }
                }
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::{
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
                let route_memory = if r.bool()? {
                    Some(crate::RouteDeparture {
                        chain: r.u32()? as usize,
                        vertex: r.u32()? as usize,
                        direction: r.u8()? as i8,
                    })
                } else {
                    None
                };
                let mut occlusions = crate::DepthOcclusions::default();
                for _ in 0..r.u8()? {
                    occlusions.push(crate::OcclusionSpan {
                        chain: r.u32()? as usize,
                        first_segment: r.u32()? as usize,
                        last_segment: r.u32()? as usize,
                    });
                }
                MotionModel::SurfaceMomentum(SurfaceMomentumMotion {
                    params,
                    state,
                    depth_lane,
                    route_memory,
                    occlusions,
                })
            }
            2 => {
                let params = CrawlerParams {
                    crawl_speed: r.f32()?,
                    max_fall_speed: r.f32()?,
                };
                let state = match r.u8()? {
                    0 => CrawlerState::DETACHED,
                    1 => CrawlerState::attached(r.vec2()?),
                    2 => CrawlerState::attached_to_chain(r.u32()?, r.f32()?),
                    _ => return None,
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
fn put_axis_maneuver_state(out: &mut Vec<u8>, state: &crate::AxisManeuverState) {
    put_f32(out, state.coyote_timer);
    put_f32(out, state.drop_through_timer);
    put_f32(out, state.rebound_cooldown);
    put_bool(out, state.wall_clinging);
    put_bool(out, state.wall_climbing);
    put_vec2(out, state.pre_wall_vel);
    put_f32(out, state.pre_wall_vel_age);
    put_f32(out, state.time_off_ledge);
    put_f32(out, state.buffer_jump);
    put_f32(out, state.jump_squat_timer);
    put_f32(out, state.buffer_burst);
    put_f32(out, state.buffer_blink);
    put_f32(out, state.dash_timer);
    put_bool(out, state.blink_hold_active);
    put_f32(out, state.blink_hold_timer);
    put_bool(out, state.blink_aiming);
    put_vec2(out, state.blink_aim_offset);
    put_f32(out, state.blink_grace_timer);
    put_f32(out, state.dodge_roll_timer);
    put_f32(out, state.evade_invuln_timer);
    put_f32(out, state.dodge_roll_push);
    put_u8(out, state.jab_locks);
    put_f32(out, state.initial_dash_timer);
    put_f32(out, state.initial_dash_dir);
    put_f32(out, state.prev_steer_dir);
    put_f32(out, state.turnaround_timer);
    put_bool(out, state.teetering);
    put_f32(out, state.air_jump_rise_owned);
    put_f32(out, state.ledge_invuln_timer);
    put_f32(out, state.ledge_vulnerable_timer);
    put_bool(out, state.spot_dodging);
    put_f32(out, state.air_dodge_timer);
    put_f32(out, state.air_dodge_endlag_timer);
    put_f32(out, state.dodge_roll_endlag_timer);
    put_f32(out, state.tumble_timer);
    put_bool(out, state.tumble_until_landing);
    put_bool(out, state.tumble_untechable);
    put_bool(out, state.tumble_unannounced);
    put_f32(out, state.tech_press_timer);
    put_f32(out, state.tech_lockout_timer);
    put_f32(out, state.knockdown_timer);
    put_f32(out, state.getup_invuln_timer);
    put_ledge_grab(out, &state.ledge_grab);
    put_wire(out, &state.wire);
    put_bool(out, state.gliding);
    put_bool(out, state.fast_falling);
    put_bool(out, state.running);
    put_f32(out, state.flight_phase);
    put_bool(out, state.phased_jump.active);
    put_u8(out, state.phased_jump.launch_band);
    put_bool(out, state.phased_jump.hold_cancelled);
}

fn axis_maneuver_state(r: &mut Reader<'_>) -> Option<crate::AxisManeuverState> {
    Some(crate::AxisManeuverState {
        coyote_timer: r.f32()?,
        drop_through_timer: r.f32()?,
        rebound_cooldown: r.f32()?,
        wall_clinging: r.bool()?,
        wall_climbing: r.bool()?,
        pre_wall_vel: r.vec2()?,
        pre_wall_vel_age: r.f32()?,
        time_off_ledge: r.f32()?,
        buffer_jump: r.f32()?,
        jump_squat_timer: r.f32()?,
        buffer_burst: r.f32()?,
        buffer_blink: r.f32()?,
        dash_timer: r.f32()?,
        blink_hold_active: r.bool()?,
        blink_hold_timer: r.f32()?,
        blink_aiming: r.bool()?,
        blink_aim_offset: r.vec2()?,
        blink_grace_timer: r.f32()?,
        dodge_roll_timer: r.f32()?,
        evade_invuln_timer: r.f32()?,
        dodge_roll_push: r.f32()?,
        jab_locks: r.u8()?,
        initial_dash_timer: r.f32()?,
        initial_dash_dir: r.f32()?,
        prev_steer_dir: r.f32()?,
        turnaround_timer: r.f32()?,
        teetering: r.bool()?,
        air_jump_rise_owned: r.f32()?,
        ledge_invuln_timer: r.f32()?,
        ledge_vulnerable_timer: r.f32()?,
        spot_dodging: r.bool()?,
        air_dodge_timer: r.f32()?,
        air_dodge_endlag_timer: r.f32()?,
        dodge_roll_endlag_timer: r.f32()?,
        tumble_timer: r.f32()?,
        tumble_until_landing: r.bool()?,
        tumble_untechable: r.bool()?,
        tumble_unannounced: r.bool()?,
        tech_press_timer: r.f32()?,
        tech_lockout_timer: r.f32()?,
        knockdown_timer: r.f32()?,
        getup_invuln_timer: r.f32()?,
        ledge_grab: ledge_grab(r)?,
        wire: wire(r)?,
        gliding: r.bool()?,
        fast_falling: r.bool()?,
        running: r.bool()?,
        flight_phase: r.f32()?,
        phased_jump: crate::PhasedJumpState {
            active: r.bool()?,
            launch_band: r.u8()?,
            hold_cancelled: r.bool()?,
        },
    })
}

/// The hang state machine: a rollback into a hang must land on the same
/// anchor, with the same carried momentum, or the getup goes somewhere else.
/// THE WIRE, and every knob it was authored with.
///
/// ⛔⛔ THE PARAMS RIDE IN THE SNAPSHOT BESIDE THE STATE, which is not what a
/// codec usually does. The kernel cannot see the MOVE that put her on the rope,
/// so a restore that carried the angle but not `winch_speed` would resume the
/// same lift on a different wire — a desync that reads as a tuning difference.
/// The anchor is the same argument the ledge grab's is: a rollback into a hang
/// must land on the same anchor or the getup goes somewhere else.
fn put_wire(out: &mut Vec<u8>, wire: &Option<crate::WireState>) {
    match wire {
        None => put_bool(out, false),
        Some(w) => {
            put_bool(out, true);
            put_vec2(out, w.anchor);
            put_f32(out, w.length);
            put_f32(out, w.angle);
            put_f32(out, w.ang_vel);
            put_f32(out, w.winch_speed);
            put_f32(out, w.lift_remaining_s);
            put_f32(out, w.lift_total_s);
            put_f32(out, w.max_angle);
            put_f32(out, w.swing_accel);
            put_f32(out, w.release_rise);
        }
    }
}

fn wire(r: &mut Reader<'_>) -> Option<Option<crate::WireState>> {
    Some(if r.bool()? {
        Some(crate::WireState {
            anchor: r.vec2()?,
            length: r.f32()?,
            angle: r.f32()?,
            ang_vel: r.f32()?,
            winch_speed: r.f32()?,
            lift_remaining_s: r.f32()?,
            lift_total_s: r.f32()?,
            max_angle: r.f32()?,
            swing_accel: r.f32()?,
            release_rise: r.f32()?,
        })
    } else {
        None
    })
}

fn put_ledge_grab(out: &mut Vec<u8>, grab: &Option<crate::LedgeGrabState>) {
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

fn ledge_grab(r: &mut Reader<'_>) -> Option<Option<crate::LedgeGrabState>> {
    use crate::ledge_grab::{LedgeContact, LedgeGetupKind, LedgeGrabQuality, LedgeGrabState};
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

fn put_surface_motion(out: &mut Vec<u8>, state: crate::SurfaceMotion) {
    use crate::{SurfaceMotion, SurfaceRef};
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

fn surface_motion(r: &mut Reader<'_>) -> Option<crate::SurfaceMotion> {
    use crate::{SurfaceMotion, SurfaceRef};
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

fn put_momentum_params(out: &mut Vec<u8>, p: &crate::MomentumParams) {
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

fn momentum_params(r: &mut Reader<'_>) -> Option<crate::MomentumParams> {
    Some(crate::MomentumParams {
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

fn put_axis_swept_params(out: &mut Vec<u8>, p: &crate::AxisSweptParams) {
    let l = &p.locomotion;
    put_axis_horizontal_law(out, l.horizontal_law);
    put_axis_jump_law(out, l.jump_law);
    put_f32(out, l.run_accel);
    put_f32(out, l.air_accel);
    put_f32(out, l.ground_friction);
    put_f32(out, l.air_friction);
    put_f32(out, l.air_stop_assist);
    put_f32(out, l.carried_decay);
    put_f32(out, l.max_run_speed);
    put_f32(out, l.max_air_speed);
    put_f32(out, l.run_commit_frac);
    put_f32(out, l.crouch_speed_frac);
    put_f32(out, l.initial_dash_time);
    put_f32(out, l.initial_dash_speed);
    put_f32(out, l.turnaround_time);
    put_f32(out, l.teeter_margin);
    put_f32(out, l.max_fall_speed);
    put_f32(out, l.jump_speed);
    put_f32(out, l.double_jump_speed);
    put_f32(out, l.wall_jump_x);
    put_f32(out, l.wall_slide_speed);
    put_f32(out, l.wall_climb_speed);
    put_f32(out, l.coyote_time);
    put_f32(out, l.jump_buffer);
    put_f32(out, l.jump_squat_time);
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
    put_f32(out, a.air_dodge_time);
    put_f32(out, a.air_dodge_speed);
    put_f32(out, a.air_dodge_endlag);
    put_f32(out, a.dodge_roll_endlag);
    put_f32(out, a.dodge_stale_step);
    put_f32(out, a.dodge_stale_floor);
    put_f32(out, a.dodge_stale_recovery);
    put_f32(out, a.untechable_launch_speed);
    put_f32(out, a.evade_cancel_tail);
    put_f32(out, a.tumble_speed);
    put_f32(out, a.spot_dodge_time);
    put_f32(out, a.sdi_step);
    put_f32(out, a.asdi_step);
    put_f32(out, a.jab_lock_speed);
    put_u8(out, a.jab_lock_limit);
    put_f32(out, a.parry_window_time);
    // a DISCRIMINANT, and the checker's `snapshot_unit_enum!` fold does not
    // see a hand-written `put_u8` of one — so the version log carries the claim.
    put_u8(
        out,
        match a.parry_timing {
            crate::ParryTiming::OnRaise => 0,
            crate::ParryTiming::OnRelease => 1,
        },
    );
    put_f32(out, a.shield.max_health);
    put_f32(out, a.shield.drain_per_second);
    put_f32(out, a.shield.regen_per_second);
    put_f32(out, a.shield.damage_scale);
    put_f32(out, a.shield.break_stun_time);
    put_f32(out, a.shield.stun_per_damage);
    put_f32(out, a.shield.pushback_per_damage);
    put_f32(out, a.shield.min_coverage);
    put_f32(out, a.shield.tilt_range);
    put_f32(out, a.shield.drop_lag);
    // The out-of-shield rule is a five-bit AUTHORED table with a "no rule at
    // all" case, encoded as one byte: `0` is `None`, and a policy sets bit 0
    // so it can never collide with it.
    put_u8(out, encode_out_of_shield(a.shield.out_of_shield));
    // Whether a guard may be raised airborne — a ruleset RULE, so it travels
    // with the rest of the shield policy rather than being re-derived.
    put_u8(out, u8::from(a.shield.air_guard));
    put_u8(out, u8::from(a.shield.platform_drop));
    put_f32(out, a.footstool.rise_speed);
    put_f32(out, a.footstool.press_speed);
    put_f32(out, a.footstool.flinch_time);
    put_f32(out, a.footstool.air_tumble_time);
    put_f32(out, a.footstool.stomper_invuln);
    put_f32(out, a.footstool.band);
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
    match f.invariant_speed {
        Some(speed) => {
            put_bool(out, true);
            put_f32(out, speed);
        }
        None => put_bool(out, false),
    }
}

fn axis_swept_params(r: &mut Reader<'_>) -> Option<crate::AxisSweptParams> {
    use crate::{
        AxisLocomotion, AxisSweptParams, FlightTuning, LedgeMomentumTuning, TraversalAbilityTuning,
    };
    Some(AxisSweptParams {
        locomotion: AxisLocomotion {
            horizontal_law: axis_horizontal_law(r)?,
            jump_law: axis_jump_law(r)?,
            run_accel: r.f32()?,
            air_accel: r.f32()?,
            ground_friction: r.f32()?,
            air_friction: r.f32()?,
            air_stop_assist: r.f32()?,
            carried_decay: r.f32()?,
            max_run_speed: r.f32()?,
            max_air_speed: r.f32()?,
            run_commit_frac: r.f32()?,
            crouch_speed_frac: r.f32()?,
            initial_dash_time: r.f32()?,
            initial_dash_speed: r.f32()?,
            turnaround_time: r.f32()?,
            teeter_margin: r.f32()?,
            max_fall_speed: r.f32()?,
            jump_speed: r.f32()?,
            double_jump_speed: r.f32()?,
            wall_jump_x: r.f32()?,
            wall_slide_speed: r.f32()?,
            wall_climb_speed: r.f32()?,
            coyote_time: r.f32()?,
            jump_buffer: r.f32()?,
            jump_squat_time: r.f32()?,
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
            air_dodge_time: r.f32()?,
            air_dodge_speed: r.f32()?,
            air_dodge_endlag: r.f32()?,
            dodge_roll_endlag: r.f32()?,
            dodge_stale_step: r.f32()?,
            dodge_stale_floor: r.f32()?,
            dodge_stale_recovery: r.f32()?,
            untechable_launch_speed: r.f32()?,
            evade_cancel_tail: r.f32()?,
            tumble_speed: r.f32()?,
            spot_dodge_time: r.f32()?,
            sdi_step: r.f32()?,
            asdi_step: r.f32()?,
            jab_lock_speed: r.f32()?,
            jab_lock_limit: r.u8()?,
            parry_window_time: r.f32()?,
            parry_timing: match r.u8()? {
                0 => crate::ParryTiming::OnRaise,
                1 => crate::ParryTiming::OnRelease,
                _ => return None,
            },
            shield: crate::ShieldTuning {
                max_health: r.f32()?,
                drain_per_second: r.f32()?,
                regen_per_second: r.f32()?,
                damage_scale: r.f32()?,
                break_stun_time: r.f32()?,
                stun_per_damage: r.f32()?,
                pushback_per_damage: r.f32()?,
                min_coverage: r.f32()?,
                tilt_range: r.f32()?,
                drop_lag: r.f32()?,
                out_of_shield: decode_out_of_shield(r.u8()?),
                air_guard: r.u8()? != 0,
                platform_drop: r.u8()? != 0,
            },
            footstool: crate::FootstoolTuning {
                rise_speed: r.f32()?,
                press_speed: r.f32()?,
                flinch_time: r.f32()?,
                air_tumble_time: r.f32()?,
                stomper_invuln: r.f32()?,
                band: r.f32()?,
            },
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
            invariant_speed: if r.bool()? { Some(r.f32()?) } else { None },
        },
    })
}

fn put_axis_horizontal_law(out: &mut Vec<u8>, law: crate::AxisHorizontalLaw) {
    use crate::AxisHorizontalLaw;
    match law {
        AxisHorizontalLaw::Responsive => put_u8(out, 0),
        AxisHorizontalLaw::Momentum(params) => {
            put_u8(out, 1);
            put_f32(out, params.ground_reverse_accel);
            put_f32(out, params.ground_coast_decel);
            put_f32(out, params.air_reverse_accel);
            put_f32(out, params.air_coast_decel);
        }
    }
}

fn axis_horizontal_law(r: &mut Reader<'_>) -> Option<crate::AxisHorizontalLaw> {
    use crate::{AxisHorizontalLaw, MomentumHorizontalTuning};
    Some(match r.u8()? {
        0 => AxisHorizontalLaw::Responsive,
        1 => AxisHorizontalLaw::Momentum(MomentumHorizontalTuning {
            ground_reverse_accel: r.f32()?,
            ground_coast_decel: r.f32()?,
            air_reverse_accel: r.f32()?,
            air_coast_decel: r.f32()?,
        }),
        _ => return None,
    })
}

fn put_axis_jump_law(out: &mut Vec<u8>, law: crate::AxisJumpLaw) {
    use crate::AxisJumpLaw;
    match law {
        AxisJumpLaw::VelocityCut => put_u8(out, 0),
        AxisJumpLaw::PhasedGravity(params) => {
            put_u8(out, 1);
            for threshold in params.speed_thresholds {
                put_f32(out, threshold);
            }
            for offset in params.launch_offsets {
                put_f32(out, offset);
            }
            put_f32(out, params.held_rise_gravity_scale);
            put_f32(out, params.released_rise_gravity_scale);
            put_f32(out, params.fall_gravity_scale);
            put_f32(out, params.held_phase_min_upward_speed);
        }
    }
}

fn axis_jump_law(r: &mut Reader<'_>) -> Option<crate::AxisJumpLaw> {
    use crate::{AxisJumpLaw, PhasedGravityJumpTuning};
    Some(match r.u8()? {
        0 => AxisJumpLaw::VelocityCut,
        1 => AxisJumpLaw::PhasedGravity(PhasedGravityJumpTuning {
            speed_thresholds: [r.f32()?, r.f32()?, r.f32()?],
            launch_offsets: [r.f32()?, r.f32()?, r.f32()?, r.f32()?],
            held_rise_gravity_scale: r.f32()?,
            released_rise_gravity_scale: r.f32()?,
            fall_gravity_scale: r.f32()?,
            held_phase_min_upward_speed: r.f32()?,
        }),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use crate::snapshot::{decode_state, encode_state};

    #[test]
    fn phased_axis_profile_and_active_arc_round_trip() {
        use crate::{
            AxisHorizontalLaw, AxisJumpLaw, MomentumHorizontalTuning, MotionModel,
            PhasedGravityJumpTuning,
        };

        let mut params = crate::AxisSweptParams::default();
        params.locomotion.horizontal_law = AxisHorizontalLaw::Momentum(MomentumHorizontalTuning {
            ground_reverse_accel: 900.0,
            ground_coast_decel: 393.75,
            air_reverse_accel: 900.0,
            air_coast_decel: 0.0,
        });
        params.locomotion.jump_law = AxisJumpLaw::PhasedGravity(PhasedGravityJumpTuning {
            speed_thresholds: [120.0, 187.5, 210.0],
            launch_offsets: [-30.0, -15.0, 0.0, 30.0],
            held_rise_gravity_scale: 0.2,
            released_rise_gravity_scale: 1.0,
            fall_gravity_scale: 1.0,
            held_phase_min_upward_speed: 240.0,
        });

        let mut model = MotionModel::axis_swept(params);
        let MotionModel::AxisSwept(axis) = &mut model else {
            unreachable!();
        };
        axis.state.phased_jump.begin(2);
        axis.state.phased_jump.cancel_hold();
        axis.state.buffer_jump = 0.03125;

        let bytes = encode_state(&model);
        let restored =
            decode_state::<MotionModel>(&bytes).expect("codec must accept its own bytes");
        let MotionModel::AxisSwept(axis) = restored else {
            panic!("axis policy identity did not survive rollback");
        };
        assert_eq!(axis.params, params);
        assert_eq!(axis.state.phased_jump.launch_band, 2);
        assert!(axis.state.phased_jump.active);
        assert!(axis.state.phased_jump.hold_cancelled);
        assert_eq!(axis.state.buffer_jump, 0.03125);
    }
}

/// One byte for the out-of-shield table. Bit 0 is PRESENCE, so `0` decodes as
/// "this game has no out-of-shield rule" and can never be confused with a
/// policy that happens to permit nothing.
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
