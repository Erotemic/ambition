//! Advance live room platforms and retire their old collision compositions.

use crate::collision::{ComposedSolids, MovingPlatformSet, retire_stale_composed_solids};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{
    LiveRoomInstance, LiveRooms, RoomInstanceRoot,
};
use ambition_platformer2d_shared_tangle::schedule::{MovingPlatformsAdvanced, WorldPrepSet};
use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;

/// Platform motion and collision-cache retirement form one world step.
/// Bodies create rescue platforms before this set and sample contacts after it.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct MovingPlatformStep;

/// Install the world step without a body, controller, or view dependency.
pub fn install_moving_platform_simulation(app: &mut App, schedule: impl ScheduleLabel + Clone) {
    app.init_resource::<ComposedSolids>();
    app.add_systems(
        schedule,
        (
            advance_moving_platforms
                .run_if(any_moving_platforms)
                .in_set(MovingPlatformsAdvanced),
            retire_stale_composed_solids,
        )
            .chain()
            .in_set(MovingPlatformStep)
            .in_set(WorldPrepSet::BeforeIntegrate),
    );
}

fn any_moving_platforms(rooms: Query<&MovingPlatformSet, With<RoomInstanceRoot>>) -> bool {
    rooms.iter().any(|platforms| !platforms.0.is_empty())
}

/// Each live room advances on the simulation clock. A lift responds to a
/// grounded body in its room and collision plane, regardless of its controller.
#[allow(clippy::type_complexity)]
fn advance_moving_platforms(
    time: Res<ambition_time::WorldTime>,
    mut rooms: Query<(&LiveRoomInstance, &mut MovingPlatformSet), With<RoomInstanceRoot>>,
    riders: Query<(
        Entity,
        &ae::BodyKinematics,
        &ae::BodyGroundState,
        Option<&ae::SweepSample>,
        Option<&ae::DepthPlane>,
    )>,
    live: LiveRooms,
) {
    let dt = time.sim_dt();
    for (instance, mut platforms) in &mut rooms {
        // Empty sets must not receive a change tick: composed walls use it.
        if platforms.0.is_empty() {
            continue;
        }
        let room = Some(*instance);
        for platform in &mut platforms.0 {
            if platform.is_lift() {
                let ridden = riders.iter().any(|(body, kin, ground, sample, depth)| {
                    depth.is_none_or(|depth| depth.is_playable())
                        && live.of(body) == room
                        && platform.is_supporting_body(
                            kin.collision_box(sample),
                            ground.on_ground,
                            sample.map_or(ae::DEFAULT_GRAVITY_DIR, |sample| sample.down),
                        )
                });
                platform.set_ridden(ridden);
            }
            platform.update(dt);
        }
    }
}

#[cfg(test)]
mod tests;
