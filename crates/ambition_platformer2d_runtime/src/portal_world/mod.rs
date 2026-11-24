//! Apply portal geometry to a room's collision world.

use bevy::{ecs::schedule::InternedScheduleLabel, prelude::*};

use ambition_platformer2d_shared_tangle::schedule::{
    gameplay_allowed, MovingPlatformsAdvanced, WorldPrepSet,
};
use ambition_portal2d::{PlacedPortal, PortalLinkResolution, PortalSet, publish_portal_carves};

mod carves;
mod hosts;
#[cfg(test)]
mod tests;

pub(crate) fn install(app: &mut App, sim: InternedScheduleLabel) {
    app.add_systems(
        sim,
        (
            hosts::attach_portal_hosts
                .run_if(any_with_component::<PlacedPortal>)
                .run_if(gameplay_allowed)
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
