//! Apply portal geometry to a room's collision world.

use bevy::{ecs::schedule::InternedScheduleLabel, prelude::*};

use ambition_platformer2d_shared_tangle::schedule::{
    GameplayGated, MovingPlatformsAdvanced, WorldPrepSet,
};
use ambition_portal2d::{PlacedPortal, PortalLinkResolution, PortalSet, publish_portal_carves};

mod carried;
mod carves;
mod hosts;
#[cfg(test)]
mod tests;

pub(crate) fn install(app: &mut App, sim: InternedScheduleLabel) {
    // Crushing C5: bodies a moving host pushes through its portal, read by
    // the platforms' advance (`advance_moving_platforms`).
    app.init_resource::<ambition_platformer2d_world::platforms::obstruction::CarriedObstructions>();
    app.add_systems(
        sim,
        carried::publish_carried_obstructions
            .in_set(GameplayGated)
            .in_set(WorldPrepSet::BeforeIntegrate)
            .before(ambition_platformer2d_world::platforms::MovingPlatformStep),
    );
    app.add_systems(
        sim,
        (
            hosts::attach_portal_hosts
                .run_if(any_with_component::<PlacedPortal>)
                .in_set(GameplayGated)
                .in_set(WorldPrepSet::BeforeIntegrate)
                .before(MovingPlatformsAdvanced),
            hosts::refresh_hosted_portal_frames
                .run_if(any_with_component::<PlacedPortal>)
                .in_set(PortalSet::Pose)
                .before(PortalLinkResolution),
        ),
    );
    app.add_systems(
        sim,
        (
            carves::sync_portal_host_depths
                .run_if(carves::portals_or_depths_stand)
                .before(publish_portal_carves),
            carves::bridge_portal_carves
                .run_if(carves::carves_or_holes_stand)
                .after(publish_portal_carves),
        )
            .in_set(PortalSet::Carves),
    );
}
