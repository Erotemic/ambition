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
    /// The item lies where the body cannot get to it: in the body's room over
    /// no surface the body reaches with its own movement, or in a room no
    /// route of rooms leads into (another live instance of the body's own
    /// room, or past a room that is live twice).
    NoRoute,
    /// The item lies nowhere the body could go for it: another body holds it,
    /// or no live room and no ledger row has it.
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
/// errand's item in another room: the zone of `body_room` it starts at, the
/// room that zone leads into, and where a body comes out there.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ErrandCrossing {
    pub zone: ae::Aabb,
    pub zone_id: String,
    pub into: CrossInto,
    pub into_id: String,
    pub arrival: ae::Vec2,
}

/// The room a crossing leads into.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CrossInto {
    /// A live room: the body goes into it.
    Live(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance),
    /// A room that is not live: the body leaves through the ledger, and the
    /// room builds it when it is live again.
    Dormant,
}

/// [`ErrandCrossing`] for `errand`. The item lies in another live room, or,
/// for a body that `may_leave` the live rooms (one with durable whereabouts),
/// in a room that is not live, where the ledger puts it.
///
/// ⛔ THE ITEM'S LIVE ROOM, NOT ITS DEFINITION. A definition can be live
/// twice, and the item lies in one of the two. The last hop enters the item's
/// own live room. A route through a definition that is live twice is
/// refused: no rule says which of the two its door leads into, and the
/// least instance is not an answer.
///
/// `Err(Gone)` when the item lies nowhere the body could go for it.
/// `Err(NoRoute)` when it lies in a live room no route of live rooms leads
/// into: another live instance of the body's own room, a room past one that
/// is not live (a body that no slot drives does not open a room, and it
/// leaves the live rooms only for its item), or past one that is live twice.
pub(crate) fn errand_crossing<'a>(
    errand: &Errand,
    body_room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    may_leave: bool,
    specs: &ambition_platformer2d_world::rooms::LiveRoomSpecs,
    ledger: Option<&ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>,
    items: impl IntoIterator<Item = (Entity, &'a SimId, &'a ambition_held_items::GroundItem, &'a ambition_held_items::ItemCustody)>,
) -> Result<ErrandCrossing, ErrandRefusal> {
    use ErrandRefusal::{Gone, NoRoute};
    let rooms = specs.rooms();
    // The live rooms of a definition.
    let live_of = |index: usize| -> Vec<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance> {
        specs
            .live_rooms()
            .filter(|(_, definition)| definition.index() == index)
            .map(|(live, _)| live)
            .collect()
    };
    let from = specs.definition_in(body_room).ok_or(Gone)?;
    let live_item = items
        .into_iter()
        .find(|(_, id, _, custody)| **id == errand.fetch && custody.in_world())
        .map(|(item, ..)| specs.live().of(item));
    // The definition the item lies in, and its live room when it is live.
    let (to, item_room) = match live_item {
        Some(item_room) => {
            let item_room = item_room.ok_or(Gone)?;
            if item_room == body_room {
                return Err(Gone);
            }
            (specs.definition_in(item_room).ok_or(Gone)?, Some(item_room))
        }
        // Not lying in a live room: where the ledger puts it, when that room
        // is not live.
        None => {
            if !may_leave {
                return Err(Gone);
            }
            let ambition_platformer2d_shared_tangle::lifecycle::OccurrenceWhereabouts::Placed { room, .. } =
                ledger.ok_or(Gone)?.whereabouts(&errand.fetch).ok_or(Gone)?
            else {
                return Err(Gone);
            };
            let to = rooms.definition_by_id(room).ok_or(Gone)?;
            if !live_of(to.index()).is_empty() {
                return Err(Gone);
            }
            (to, None)
        }
    };
    // Another live instance of the body's own room: no door leads there.
    if to == from {
        return Err(NoRoute);
    }
    let route = rooms.route(from.index(), to.index()).ok_or(NoRoute)?;
    // The live room the hop into `index` enters, or `None` for a room that
    // is not live. The last hop of a route to a live item enters the item's
    // room; any other hop enters the definition's one live room.
    let enters = |index: usize, last: bool| -> Result<Option<_>, ErrandRefusal> {
        if last && item_room.is_some() {
            return Ok(item_room);
        }
        match live_of(index).as_slice() {
            [] => Ok(None),
            [one] => Ok(Some(*one)),
            _ => Err(NoRoute),
        }
    };
    for (n, hop) in route.iter().enumerate() {
        let entered = enters(hop.to, n + 1 == route.len())?;
        if item_room.is_some() && entered.is_none() {
            return Err(NoRoute);
        }
    }
    let first = route.first().ok_or(NoRoute)?;
    let zone = rooms.spec(from).loading_zones.iter().find(|zone| zone.id == first.zone).ok_or(NoRoute)?;
    let transition = rooms.transition_through(from, &first.zone).ok_or(NoRoute)?;
    Ok(ErrandCrossing {
        zone: zone.aabb,
        zone_id: zone.id.clone(),
        into: enters(first.to, route.len() == 1)?.map_or(CrossInto::Dormant, CrossInto::Live),
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
///
/// Into a room that is not live, a body with durable whereabouts leaves
/// through the ledger: its `Placed` row names that room at the arrival, and
/// it is despawned. The room builds it there when it is live again (the
/// outlook reinstates it), and the errand ends with the body. A body that
/// holds or wears something does not leave: what it carries has no road to a
/// room that is not live.
///
/// ⛔ A gate portal that is shut for the player is shut for the errand too
/// (`GatePortalRegistry::lets_through`): the body waits in the zone until the
/// portal is `On`.
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
    portals: Option<Res<ambition_platformer2d_world::rooms::GatePortalRegistry>>,
    phases: Option<Res<ambition_platformer2d_world::rooms::GatePortalPhases>>,
) {
    let no_phases = ambition_platformer2d_world::rooms::GatePortalPhases::default();
    let phases = phases.as_deref().unwrap_or(&no_phases);
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
        let durable = ambition_platformer2d_construction::whereabouts::keeps_durable_whereabouts(&specs, config, origin);
        let Ok(crossing) =
            errand_crossing(errand, body_room, durable, &specs, occurrences.as_deref(), &items)
        else {
            continue;
        };
        // The reach of the body is its collision box, as for a take.
        if !cluster.kinematics.collision_box(cluster.sweep.as_deref()).strict_intersects(crossing.zone) {
            continue;
        }
        if portals.as_deref().is_some_and(|portals| !portals.lets_through(phases, &crossing.zone_id)) {
            continue;
        }
        let edges = edges.get_or_insert_with(|| custody.iter().map(|(entity, held)| (entity, held.custodian)).collect());
        let moving = ambition_platformer2d_shared_tangle::lifecycle::custody_closure([body], edges);
        let into = match crossing.into {
            CrossInto::Live(into) => into,
            CrossInto::Dormant => {
                let Some(occurrences) = occurrences.as_deref_mut() else {
                    continue;
                };
                if moving.len() != 1 {
                    continue;
                }
                let size = cluster.kinematics.size;
                let feet = ae::Vec2::new(crossing.arrival.x, crossing.arrival.y + size.y * 0.5);
                if occurrences.admit_crossing(sim_id.clone(), &crossing.into_id, feet) {
                    commands.entity(body).despawn();
                }
                continue;
            }
        };
        for moving in moving {
            commands
                .entity(moving)
                .insert(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(into));
        }
        let mut clusters = cluster.as_clusters_mut();
        ae::movement::transit_body(&mut model, &mut clusters, crossing.arrival, ae::movement::TransitVelocity::Zero);
        if let Some(occurrences) = occurrences.as_deref_mut() {
            if durable {
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
