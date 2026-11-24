//! Run Act 3 through the shared scripted-input path for device profiles.

use ambition_platformer2d::autopilot::AutopilotOrder;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::LiveRoomSpecs;
use bevy::prelude::*;

use crate::{DARKNESS_ROOM_ID, SanicActPhase, SanicActState};

pub const ACT3_PROFILE: &str = "sanic_act3";

pub(super) fn install(app: &mut App) {
    app.add_systems(
        Update,
        play_act_three
            .in_set(ambition_platformer2d::scripted_input::AutoplayPolicy)
            .run_if(ambition_platformer2d::scripted_input::autoplay_is(
                ACT3_PROFILE,
            )),
    );
}

fn play_act_three(
    mut order: ResMut<AutopilotOrder>,
    players: Query<Entity, With<PrimaryPlayer>>,
    rooms: Option<LiveRoomSpecs>,
    acts: Query<&SanicActState>,
) {
    let Some(room) = players
        .single()
        .ok()
        .and_then(|body| rooms.as_ref().and_then(|rooms| rooms.spec_of(body)))
    else {
        order.set_if_neq(AutopilotOrder::Idle);
        return;
    };
    if room.id != DARKNESS_ROOM_ID {
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
    // The completion route holds Right through the ground road and its portals.
    order.set_if_neq(AutopilotOrder::Press(ControlFrame {
        axis_x: 1.0,
        right_pressed: true,
        ..default()
    }));
}
