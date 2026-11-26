//! The movement-policy (`MotionModel`) rollback checksum codec — ADR 0024 §9.

use crate::snapshot::{put_bool, put_f32, put_u32, put_u8, put_vec2, Reader, SnapshotState};

mod axis;

use axis::{axis_maneuver_state, axis_swept_params, put_axis_maneuver_state, put_axis_swept_params};
pub(crate) use axis::same_axis_motion;

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
