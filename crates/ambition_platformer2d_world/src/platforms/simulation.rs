//! Advance live room platforms and retire their old collision compositions.

use crate::collision::{ComposedSolids, MovingPlatformSet, retire_stale_composed_solids};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{
    LiveRoomInstance, RoomInstanceRoot,
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

/// Each live room advances on the simulation clock, at its own pace (ISL-1b:
/// a held room's platforms hold with the bodies on them). A lift responds to
/// a grounded body in its room and collision plane, regardless of its
/// controller.
///
/// Each advance is a proposal (crushing C3): a platform that may not push a
/// body past its clearance keeps the part of its move that does not
/// ([`super::obstruction`]); an unstoppable one, the default, keeps its
/// schedule.
#[allow(clippy::type_complexity)]
fn advance_moving_platforms(
    time: Res<ambition_time::WorldTime>,
    mut rooms: Query<(&LiveRoomInstance, &mut MovingPlatformSet, Option<&ae::RoomGeometry>), With<RoomInstanceRoot>>,
    riders: Query<(
        Entity,
        &ae::BodyKinematics,
        &ae::BodyGroundState,
        Option<&ae::SweepSample>,
        Option<&ae::DepthPlane>,
    )>,
    bodies: Query<(Entity, &ae::BodyKinematics, Option<&ae::DepthPlane>), Without<ae::BodyInStorage>>,
    policy: Option<Res<ae::crush::CrushPolicy>>,
    carried: Option<Res<super::obstruction::CarriedObstructions>>,
    freeze: ambition_platformer2d_shared_tangle::lifecycle::RoomFreeze,
) {
    let live = freeze.live();
    let dt = time.sim_dt();
    let default_policy = ae::crush::CrushPolicy::default();
    let policy = policy.as_deref().unwrap_or(&default_policy);
    for (instance, mut platforms, geometry) in &mut rooms {
        // Empty sets must not receive a change tick: composed walls use it.
        if platforms.0.is_empty() {
            continue;
        }
        let room = Some(*instance);
        // Only a platform that may stall reads the others; a room of
        // unstoppable ones (the default) advances as before, building nothing.
        let any_may_stall = platforms.0.iter()
            .any(|platform| policy.platform_authority(&platform.name, platform.authority, platform.is_lift()).allowance_px().is_some());
        let others: Vec<(ae::GeoId, ae::Aabb, bool)> = if !any_may_stall { Vec::new() } else {
            platforms.0.iter()
                .map(|platform| { let block = platform.as_collision_block(); (block.id, block.aabb, platform.one_way) })
                .collect()
        };
        for platform in &mut platforms.0 {
            if platform.is_lift() {
                let ridden = riders.iter().any(|(body, kin, ground, sample, depth)| {
                    depth.is_none_or(|depth| depth.is_playable())
                        && live.of(body) == room
                        && platform.is_supporting_body(
                            kin.aabb(),
                            ground.on_ground,
                            sample.map_or(ae::DEFAULT_GRAVITY_DIR, |sample| sample.down),
                        )
                });
                platform.set_ridden(ridden);
            }
            let step = dt * freeze.scale_in(room);
            let authority = policy.platform_authority(&platform.name, platform.authority, platform.is_lift());
            let Some(allowance) = authority.allowance_px() else {
                platform.update(step);
                continue;
            };
            let before = platform.clone();
            let delta = platform.update(step);
            if delta == ae::Vec2::ZERO { continue; }
            let own = before.as_collision_block().id;
            let carried_here = carried.as_deref().map_or_else(Vec::new, |carried| carried.of(&own));
            let obstacles: Vec<super::obstruction::Obstacle> = bodies.iter()
                .filter(|(body, _, depth)| depth.is_none_or(|depth| depth.is_playable()) && live.of(*body) == room
                    && !carried_here.iter().any(|through| through.body == *body))
                .map(|(_, kin, _)| super::obstruction::Obstacle { aabb: kin.aabb() })
                .collect();
            let along_gravity = delta.normalize_or_zero().dot(ae::DEFAULT_GRAVITY_DIR) > 0.5;
            let mut solids = geometry.map_or_else(Vec::new,
                |geometry| super::obstruction::pushing_solids(&geometry.0, delta, ae::DEFAULT_GRAVITY_DIR, &own));
            solids.extend(others.iter().filter(|(id, _, one_way)| *id != own && (!*one_way || along_gravity))
                .map(|(_, aabb, _)| *aabb));
            let mut accepted = super::obstruction::accepted_fraction(before.aabb(), delta, allowance, &obstacles, &solids);
            accepted = accepted.min(super::obstruction::accepted_through(delta, allowance, &carried_here));
            if accepted < 1.0 {
                *platform = before;
                platform.update(step * accepted);
            }
        }
    }
}

#[cfg(test)]
mod tests;
