//! Run Sanic through the shared scripted-input path for device profiles.

use ambition_platformer2d::autopilot::AutopilotOrder;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::scripted_input::{Autoplay, AutoplayPolicy, autoplay_is};
use ambition_platformer2d::sim::GameMode;
use ambition_platformer2d::world::rooms::LiveRoomSpecs;
use bevy::prelude::*;

use crate::{DARKNESS_ROOM_ID, HIGHWAY_ROOM_ID, SPEEDWAY_ROOM_ID, SanicActPhase, SanicActState};

pub const ACT3_PROFILE: &str = "sanic_act3";
pub const ALL_ACTS_PROFILE: &str = "sanic_all";

pub(super) fn install(app: &mut App) {
    app.add_systems(
        Update,
        play_sanic.in_set(AutoplayPolicy)
            .run_if(autoplay_is(ACT3_PROFILE).or_else(autoplay_is(ALL_ACTS_PROFILE))),
    );
    app.add_systems(Last, report_act_frames.run_if(autoplay_is(ALL_ACTS_PROFILE)));
}

fn act_number(room: &str) -> Option<u8> {
    match room {
        SPEEDWAY_ROOM_ID => Some(1),
        HIGHWAY_ROOM_ID => Some(2),
        DARKNESS_ROOM_ID => Some(3),
        _ => None,
    }
}

fn play_sanic(
    mut order: ResMut<AutopilotOrder>,
    profile: Res<Autoplay>,
    players: Query<(Entity, &BodyKinematics), With<PrimaryPlayer>>,
    rooms: Option<LiveRoomSpecs>,
    acts: Query<&SanicActState>,
) {
    let Some((room, body)) = players
        .single()
        .ok()
        .and_then(|(entity, body)| Some((rooms.as_ref()?.spec_of(entity)?, body)))
    else {
        order.set_if_neq(AutopilotOrder::Idle);
        return;
    };
    if act_number(&room.id).is_none() || (profile.0 == ACT3_PROFILE && room.id != DARKNESS_ROOM_ID) {
        order.set_if_neq(AutopilotOrder::Idle);
        return;
    }
    if acts
        .iter()
        .any(|act| matches!(act.phase, SanicActPhase::Cleared { .. }))
    {
        order.set_if_neq(AutopilotOrder::Idle);
        return;
    }
    // The spike bed sets the jump interval in Act 1.
    let jump = room.id == SPEEDWAY_ROOM_ID && room.placements.iter()
        .find(|record| record.name == "pit_spikes")
        .is_some_and(|pit| body.pos.x > pit.aabb.min.x - 220.0 && body.pos.x < pit.aabb.max.x);
    order.set_if_neq(AutopilotOrder::Press(ControlFrame {
        axis_x: 1.0,
        right_pressed: true,
        jump_pressed: jump,
        jump_held: jump,
        ..default()
    }));
}

#[derive(Default)]
struct ActFrames {
    act: Option<u8>,
    running: bool,
    frame_ms: Vec<f64>,
}

fn report_act_frames(
    mut run: Local<ActFrames>,
    time: Res<Time<Real>>,
    mode: Res<State<GameMode>>,
    players: Query<Entity, With<PrimaryPlayer>>,
    rooms: Option<LiveRoomSpecs>,
    acts: Query<&SanicActState>,
) {
    let room = players.single().ok().and_then(|entity| rooms.as_ref()?.spec_of(entity));
    let Some((act, state)) = room.and_then(|room| Some((act_number(&room.id)?, acts.single().ok()?))) else {
        run.running = false;
        return;
    };
    let at = time.elapsed_secs_f64();
    if matches!(state.phase, SanicActPhase::Cleared { .. }) {
        if run.act == Some(act) && !run.frame_ms.is_empty() {
            let frames = run.frame_ms.len();
            let seconds = run.frame_ms.iter().sum::<f64>() / 1000.0;
            run.frame_ms.sort_unstable_by(f64::total_cmp);
            let percentile = |q: f64| run.frame_ms[((frames - 1) as f64 * q).round() as usize];
            eprintln!(
                "[sanic-autoplay] CLEAR act={act} t={at:.3}s frames={frames} seconds={seconds:.3} fps={:.2} p50={:.2}ms p95={:.2}ms",
                frames as f64 / seconds, percentile(0.50), percentile(0.95),
            );
            run.frame_ms.clear();
        }
        run.running = false;
        return;
    }
    if *mode.get() != GameMode::Playing {
        run.running = false;
        return;
    }
    if run.act != Some(act) {
        run.act = Some(act);
        run.running = false;
        run.frame_ms.clear();
        eprintln!("[sanic-autoplay] BEGIN act={act} t={at:.3}s");
    }
    let delta = time.delta_secs_f64();
    if run.running && delta > 0.0 {
        run.frame_ms.push(delta * 1000.0);
    }
    run.running = true;
}
