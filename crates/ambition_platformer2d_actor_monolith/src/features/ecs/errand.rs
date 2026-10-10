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

/// The first crossing of the route from the live room `body_room` to the
/// errand's item in another live room: the zone of `body_room` it starts at,
/// the live room that zone leads into, and where a body comes out there.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ErrandCrossing {
    pub zone: ae::Aabb,
    pub into: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    pub into_id: String,
    pub arrival: ae::Vec2,
}

/// [`ErrandCrossing`] for `errand`. `None` when the item lies in no live
/// room, lies in `body_room`, or when no route of live rooms gets to it: a
/// body that no slot drives does not open a room.
pub(crate) fn errand_crossing<'a>(
    errand: &Errand,
    body_room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    specs: &ambition_platformer2d_world::rooms::LiveRoomSpecs,
    items: impl IntoIterator<Item = (Entity, &'a SimId, &'a ambition_held_items::GroundItem, &'a ambition_held_items::ItemCustody)>,
) -> Option<ErrandCrossing> {
    let item_room = items
        .into_iter()
        .find(|(_, id, _, custody)| **id == errand.fetch && custody.in_world())
        .and_then(|(item, ..)| specs.live().of(item))?;
    if item_room == body_room {
        return None;
    }
    let rooms = specs.rooms();
    let from = specs.definition_in(body_room)?;
    let to = specs.definition_in(item_room)?;
    let route = rooms.route(from.index(), to.index())?;
    // The live room of each room on the route. When one definition is live
    // twice, the least instance, so each peer picks the same.
    let live_of = |index: usize| {
        specs
            .live_rooms()
            .filter(|(_, definition)| definition.index() == index)
            .map(|(live, _)| live)
            .min()
    };
    if route.iter().any(|hop| live_of(hop.to).is_none()) {
        return None;
    }
    let first = route.first()?;
    let zone = rooms.spec(from).loading_zones.iter().find(|zone| zone.id == first.zone)?;
    let transition = rooms.transition_through(from, &first.zone)?;
    Some(ErrandCrossing {
        zone: zone.aabb,
        into: live_of(first.to)?,
        into_id: rooms.rooms[first.to].id.clone(),
        arrival: transition.arrival,
    })
}

/// A body on an errand that stands in the zone where its route to the item
/// starts goes into the next live room: what it carries goes with it
/// (`custody_closure`), its room stamp names that room, and it comes out at
/// the zone's arrival. The road of a second seat's return
/// (`bring_a_fallen_seat_back_beside_the_primary`), not a player's crossing:
/// both rooms are live, so no room is built or retired, and a body no slot
/// drives has no participant to keep a room for.
///
/// A body that keeps durable whereabouts gets its `Placed` row in the room it
/// went into, so the room that authored it does not author it again while it
/// lives there. The advisor sees the item in the new room on the next tick.
#[allow(clippy::type_complexity)]
pub fn cross_on_errands(
    mut commands: Commands,
    advice: Res<NavigationAdvice>,
    specs: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    items: Query<(
        Entity,
        &SimId,
        &ambition_held_items::GroundItem,
        &ambition_held_items::ItemCustody,
    )>,
    custody: Query<(Entity, &ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf)>,
    mut bodies: Query<(
        Entity,
        &Errand,
        &SimId,
        &ambition_combat::actor_tuning::ActorConfig,
        Option<&ambition_platformer2d_shared_tangle::construction::SpawnOrigin>,
        ae::BodyClusterQueryData,
        &mut ae::movement::MotionModel,
    )>,
    mut occurrences: Option<bevy::prelude::ResMut<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>>,
) {
    let Some(specs) = specs else {
        return;
    };
    let mut edges: Option<Vec<(Entity, Entity)>> = None;
    for (body, errand, sim_id, config, origin, mut cluster, mut model) in &mut bodies {
        if errand.outcome != ErrandOutcome::Pending || !matches!(advice.of(body).errand, ErrandSight::Door(_)) {
            continue;
        }
        let Some(body_room) = specs.live().of(body) else {
            continue;
        };
        let Some(crossing) = errand_crossing(errand, body_room, &specs, &items) else {
            continue;
        };
        // The reach of the body is its collision box, as for a take.
        if !cluster.kinematics.collision_box(cluster.sweep.as_deref()).strict_intersects(crossing.zone) {
            continue;
        }
        let edges = edges.get_or_insert_with(|| custody.iter().map(|(entity, held)| (entity, held.custodian)).collect());
        for moving in ambition_platformer2d_shared_tangle::lifecycle::custody_closure([body], edges) {
            commands
                .entity(moving)
                .insert(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(crossing.into));
        }
        let mut clusters = cluster.as_clusters_mut();
        ae::movement::transit_body(&mut model, &mut clusters, crossing.arrival, ae::movement::TransitVelocity::Zero);
        if let Some(occurrences) = occurrences.as_deref_mut() {
            if crate::body_whereabouts::keeps_durable_whereabouts(&specs, config, origin) {
                let feet = ae::Vec2::new(crossing.arrival.x, crossing.arrival.y + clusters.kinematics.size.y * 0.5);
                let admitted = occurrences.admit_crossing(sim_id.clone(), &crossing.into_id, feet);
                // A body that walks is in no custody, and a body with a
                // terminal row is not alive to walk.
                debug_assert!(admitted, "the ledger refused the crossing of {sim_id:?}");
            }
        }
    }
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
            // On the way to the zone that leads to the item.
            ErrandSight::Door(_) => continue,
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
