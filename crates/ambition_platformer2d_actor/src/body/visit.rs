//! A visit: a body asked from outside to go to a place in its room.
//!
//! The sibling of an [`Errand`](super::errand::Errand), which fetches an item.
//! A planner's "go to the bench" is a visit: the navigation advisor answers it
//! with the same sight it gives an errand (`ErrandSight::At`, the feet point
//! under the place on a surface the body can get to), a navigating brain goes
//! there with the body's own movement, and [`settle_visits`] ends the visit
//! when the body stands at the place, or refuses it when no route gets there.
//!
//! In-room only: a place in another room is a later slice of the planner work
//! (`docs/planning/engine/radiant-npc-planning.md`).
//!
//! Rollback state: where a body was sent, and how that ended, are facts that
//! a rewind restores.

use ambition_platformer2d_core as ae;
use ae::navigation::{ErrandSight, NavNext};
use bevy::prelude::{Component, Entity, Query, Res};

use super::navigation::NavigationAdvice;

/// Go to `place` (a world point in the body's live room).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Visit {
    pub place: ae::Vec2,
    pub outcome: VisitOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisitOutcome {
    Pending,
    /// The body stands at the place.
    Arrived,
    /// The place is over no surface the body gets to with its own movement.
    NoRoute,
}

impl Visit {
    pub fn to(place: ae::Vec2) -> Self {
        Self { place, outcome: VisitOutcome::Pending }
    }

    /// One number for the visit, for the rollback probe: a restore that
    /// brought back another place, or another outcome, is a mismatch.
    pub fn probe(&self) -> u64 {
        let outcome = match self.outcome {
            VisitOutcome::Pending => 0,
            VisitOutcome::Arrived => 1,
            VisitOutcome::NoRoute => 2,
        };
        let place = (u64::from(self.place.x.to_bits()) << 32) | u64::from(self.place.y.to_bits());
        ae::navigation::mix(place ^ outcome)
    }
}

/// End each pending visit the advisor answered this tick: arrived when the
/// advice for the body's goal is that it stands there, refused when no route
/// gets to the place.
pub fn settle_visits(advice: Res<NavigationAdvice>, mut bodies: Query<(Entity, &mut Visit)>) {
    for (body, mut visit) in &mut bodies {
        if visit.outcome != VisitOutcome::Pending {
            continue;
        }
        let advice = advice.of(body);
        match advice.errand {
            ErrandSight::NoRoute => visit.outcome = VisitOutcome::NoRoute,
            // The brain's goal is the place the advisor gave; its arrival is
            // the advisor's word for that goal.
            ErrandSight::At(place) if advice.goal == Some(place) && advice.next == NavNext::Arrived => {
                visit.outcome = VisitOutcome::Arrived;
            }
            // Not looked at (in the air, not stepped yet), or on the way.
            _ => {}
        }
    }
}
