//! An errand: a goal given to a body from outside (NAVIGATION, slice 1).
//!
//! A body that carries an [`Errand`] goes to fetch one item, named by its
//! stable identity. The navigation advisor looks at the item for the body
//! (`ErrandSight`, in the body's `NavAdvice`), a navigating brain goes to the
//! place, and [`settle_errands`] ends the errand: the body takes the item when
//! it touches it, by the one take of a ground item, or the errand is refused
//! with its reason.
//!
//! The take is not an Attack press. A brain that attacks near an item does not
//! take it; a body takes an item only because its errand asks for that item.
//!
//! Rollback state: what the body is doing, and how its errand ended, are facts
//! that a rewind restores.

use ambition_platformer2d_core as ae;
use ae::navigation::ErrandSight;
use ae::AabbExt;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::{Commands, Component, Entity, Query, Res};

use super::navigation::NavigationAdvice;

/// Fetch the item `fetch`. Given from outside: by a test, a script or a
/// policy above the brain.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Errand {
    pub fetch: SimId,
    pub outcome: ErrandOutcome,
}

impl Errand {
    pub fn fetch(item: SimId) -> Self {
        Self { fetch: item, outcome: ErrandOutcome::Pending }
    }

    /// One number for the errand, for the rollback probe: a restore that
    /// brought back another errand, or another outcome, is a mismatch.
    pub fn probe(&self) -> u64 {
        let named = self
            .fetch
            .as_str()
            .bytes()
            .fold(0xCBF2_9CE4_8422_2325_u64, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3));
        let outcome = match self.outcome {
            ErrandOutcome::Pending => 0,
            ErrandOutcome::Done => 1,
            ErrandOutcome::Refused(ErrandRefusal::NoRoute) => 2,
            ErrandOutcome::Refused(ErrandRefusal::Gone) => 3,
            ErrandOutcome::Refused(ErrandRefusal::HandFull) => 4,
        };
        ae::navigation::mix(named ^ outcome)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrandOutcome {
    Pending,
    /// The body holds the item.
    Done,
    Refused(ErrandRefusal),
}

/// Why an errand ended without the item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrandRefusal {
    /// The item lies in the body's room, over no surface the body can get to
    /// with its own movement.
    NoRoute,
    /// The item does not lie in the body's room: another body holds it, or it
    /// is in another room.
    Gone,
    /// The body holds something already. A body holds one thing.
    HandFull,
}

/// Where the errand item of a body lies, as the advisor sees it: in the body's
/// live room, in the world, and named by the errand. `None`: not there.
pub(crate) fn errand_item_at<'a>(
    errand: &Errand,
    body_room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    rooms: &ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    items: impl IntoIterator<Item = (Entity, &'a SimId, &'a ambition_held_items::GroundItem, &'a ambition_held_items::ItemCustody)>,
) -> Option<ae::Vec2> {
    items
        .into_iter()
        .find(|(item, id, _, custody)| **id == errand.fetch && custody.in_world() && rooms.of(*item) == body_room)
        .map(|(_, _, ground, _)| ground.pos)
}

/// End each pending errand that can end this tick. Runs after the press
/// pickup, so a body that took an item by a press this tick has a full hand.
pub fn settle_errands(
    mut commands: Commands,
    advice: Res<NavigationAdvice>,
    mut bodies: Query<(
        Entity,
        &mut Errand,
        &ae::BodyKinematics,
        Option<&ae::SweepSample>,
        ambition_combat::hand::RepertoireQuery,
    )>,
    mut grounds: Query<(
        Entity,
        &SimId,
        &mut ambition_held_items::GroundItem,
        &mut ambition_held_items::ItemCustody,
    )>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    for (body, mut errand, kinematics, last_step, mut repertoire) in &mut bodies {
        if errand.outcome != ErrandOutcome::Pending {
            continue;
        }
        match advice.of(body).errand {
            ErrandSight::NoRoute => {
                errand.outcome = ErrandOutcome::Refused(ErrandRefusal::NoRoute);
                continue;
            }
            ErrandSight::Gone => {
                errand.outcome = ErrandOutcome::Refused(ErrandRefusal::Gone);
                continue;
            }
            // Not looked at this tick (the body is in the air, or has not
            // stepped yet): the errand waits.
            ErrandSight::None => continue,
            ErrandSight::At(_) => {}
        }
        if repertoire.held.is_some() {
            errand.outcome = ErrandOutcome::Refused(ErrandRefusal::HandFull);
            continue;
        }
        // The reach of the body is its collision box, as for a press.
        let reach = kinematics.collision_box(last_step);
        for (item, id, mut ground, mut custody) in &mut grounds {
            if *id != errand.fetch || !custody.in_world() || rooms.of(item) != rooms.of(body) {
                continue;
            }
            if reach.strict_intersects(ae::Aabb::new(ground.pos, ground.half_extent)) {
                ambition_held_items::take_ground_item(
                    &mut commands,
                    body,
                    &mut repertoire,
                    item,
                    &mut ground,
                    &mut custody,
                );
                errand.outcome = ErrandOutcome::Done;
            }
            break;
        }
    }
}
