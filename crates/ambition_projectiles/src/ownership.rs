//! Current projectile credit, separate from creation provenance and live links.

use ambition_platformer2d_shared_tangle::{
    lifecycle::{LiveRoomInstance, LiveRooms, SessionScopedEntity},
    sim_id::SimId,
};
use bevy::prelude::*;

/// The current credited body, qualified by the projectile's session.
/// A native live link updates the reference room when its body travels.
/// This semantic reference can restore a missing link in the same scope.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectileCredit {
    pub body: Option<SimId>,
    pub room: Option<LiveRoomInstance>,
}
impl ProjectileCredit {
    pub fn capture(id: Option<&SimId>, room: Option<LiveRoomInstance>) -> Self {
        Self {
            body: id.cloned(),
            room,
        }
    }
}
/// Capture credit for native shots that did not use the request materializer.
/// Run before a shot can be intercepted or its original owner can retire.
pub fn capture_missing_projectile_credit(
    mut commands: Commands,
    shots: Query<
        (
            Entity,
            &crate::ProjectileOwner,
            Option<&SessionScopedEntity>,
        ),
        (With<crate::LiveProjectile>, Without<ProjectileCredit>),
    >,
    identities: Query<(&SimId, Option<&SessionScopedEntity>)>,
    rooms: LiveRooms,
) {
    for (shot, owner, session) in &shots {
        let credit = identities
            .get(owner.0)
            .ok()
            .filter(|(_, scope)| scope.map(|scope| scope.0) == session.map(|scope| scope.0))
            .map_or_else(ProjectileCredit::default, |(id, _)| {
                ProjectileCredit::capture(Some(id), rooms.of(owner.0))
            });
        commands.entity(shot).insert(credit);
    }
}
