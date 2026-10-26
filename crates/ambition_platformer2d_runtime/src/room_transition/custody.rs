//! The away occurrences whose custody a pending crossing ends at its commit
//! (Q38): the transition's reading of the body-whereabouts ledger.
//!
//! The ledger and its writer stay with the bodies
//! (`ambition_platformer2d_construction::whereabouts`); what a crossing
//! retires is the transition's own rule, so it is read here, beside the loader
//! that prepares the destination.

use bevy::prelude::{Entity, Query, Res, With};

use ambition_platformer2d_session::rooms;

/// The away occurrences whose custody a pending crossing ends at its commit
/// (Q38).
///
/// A crossing's destination is prepared before its commit, from the ledger as
/// it stood before the crossing was recorded. The room the crossing leaves is
/// retired at the commit unless another player stays in it
/// ([`rooms::another_player_stays`], the commit's own rule), and an
/// away population occurrence there dies with it. So the destination, when it
/// is the occurrence's home, must author its replacement: the ledger it is
/// prepared from has these custody rows released.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CustodyEndingAtCommit<'w, 's> {
    away: Option<Res<'w, ambition_platformer2d_shared_tangle::lifecycle::AwayFromAuthoredRoom>>,
    bodies: Query<
        'w,
        's,
        (
            Entity,
            &'static ambition_platformer2d_shared_tangle::sim_id::SimId,
            Option<&'static ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        ),
    >,
    roots: Query<
        'w,
        's,
        &'static ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
    drivers: Query<'w, 's, (Entity, &'static ambition_characters::control::DrivingParticipant)>,
    live_bodies: ambition_platformer2d_shared_tangle::lifecycle::LiveBodies<'w, 's>,
}

impl CustodyEndingAtCommit<'_, '_> {
    /// The away occurrences in the live room `intent` retires. Empty when the
    /// intent is not a crossing, when another player stays in the room it
    /// leaves, or when nothing lives away from home.
    pub fn released_by(
        &self,
        intent: &ambition_platformer2d_session::lifecycle_commit::LifecycleIntent,
    ) -> std::collections::BTreeSet<ambition_platformer2d_shared_tangle::sim_id::SimId> {
        let mut released = std::collections::BTreeSet::new();
        let Some(away) = self.away.as_deref().filter(|away| !away.0.is_empty()) else {
            return released;
        };
        let ambition_platformer2d_session::lifecycle_commit::LifecycleIntent::Transition(intent) = intent else {
            return released;
        };
        let stamp = |entity| {
            self.bodies
                .get(entity)
                .ok()
                .and_then(|(_, _, stamp)| stamp)
                .map(|stamp| stamp.0)
        };
        let subject = self.live_bodies.entity_of(&intent.subject);
        // The commit's own departing room: the subject's, else the sole one.
        let Some(departing) = subject.and_then(stamp).or_else(|| self.roots.single().ok().copied()) else {
            return released;
        };
        if rooms::another_player_stays(
            subject,
            intent.participant,
            departing,
            self.drivers.iter().map(|(entity, driver)| (entity, driver.0, stamp(entity))),
        ) {
            return released;
        }
        for (_, sim_id, room) in &self.bodies {
            if away.0.contains(sim_id) && room.map(|room| room.0) == Some(departing) {
                released.insert(sim_id.clone());
            }
        }
        released
    }

    /// The away occurrences that live on after `intent` commits: every one
    /// that [`Self::released_by`] does not release. A construction that
    /// rebuilds a room from an older ledger (a checkpoint reset) holds these
    /// as carried, because the rooms they live in are not rewound.
    pub fn surviving(
        &self,
        intent: &ambition_platformer2d_session::lifecycle_commit::LifecycleIntent,
    ) -> std::collections::BTreeSet<ambition_platformer2d_shared_tangle::sim_id::SimId> {
        let Some(away) = self.away.as_deref().filter(|away| !away.0.is_empty()) else {
            return std::collections::BTreeSet::new();
        };
        let released = self.released_by(intent);
        away.0.difference(&released).cloned().collect()
    }
}
