//! The cohort of a direct crossing and the native relation effects it
//! settles (REL3).
//!
//! Direction and errand move a body to another live room without a
//! publication. The bodies that go are found from the crossing's actual root:
//! what is in its custody, and what is attached to it (a mount it actively
//! rides, the limbs of a limbed body; `ambition_mount::attachment_edges`).
//! `InCustodyOf` alone grows only beneath a root that travels already, so a
//! room-scoped NPC rider's mount carries no custody edge.

use std::collections::BTreeSet;

use ambition_characters::actor::limb::Limb;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf;
use bevy::prelude::{Commands, Entity, Query, World};

/// The reads that find a crossing's cohort.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CrossingCohort<'w, 's> {
    custody: Query<'w, 's, (Entity, &'static InCustodyOf)>,
    rides: ambition_mount::ActiveRides<'w, 's>,
    limbs: Query<'w, 's, (Entity, &'static Limb)>,
}

impl CrossingCohort<'_, '_> {
    /// `(member, anchor)` edges: custody, active rides and limbs.
    pub fn edges(&self) -> Vec<(Entity, Entity)> {
        let mut edges: Vec<_> = self.custody.iter().map(|(entity, held)| (entity, held.custodian)).collect();
        edges.extend(ambition_mount::attachment_edges_of(
            self.rides.pairs(),
            self.limbs.iter().map(|(limb, of)| (limb, of.of)),
        ));
        edges
    }
}

/// Queue the native effects of one accepted crossing of `moving`, rooted at
/// `root`, in the order they apply after the membership writes: the
/// attachments stand at the arrival when the root moved (`moved_by`), a ride
/// or hold the crossing splits ends by its own owner's road. A crossing that
/// despawns its cohort passes no `moved_by` and settles the splits before
/// the despawn it queues next.
pub fn settle_crossing(commands: &mut Commands, root: Entity, moved_by: Option<ae::Vec2>, moving: &BTreeSet<Entity>) {
    let moving = moving.clone();
    commands.queue(move |world: &mut World| {
        if let Some(moved_by) = moved_by {
            ambition_mount::arrive_attachments(world, root, moved_by, &moving);
        }
        ambition_mount::end_separated_rides(world, &moving);
        ambition_combat::capture::systems::release_separated_captures(world, &moving);
    });
}
