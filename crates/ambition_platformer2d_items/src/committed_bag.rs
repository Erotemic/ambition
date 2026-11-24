//! The shared bag's committed road for rooms that rewind alone (CW2).
//!
//! Under one rollback authority (local play, GGRS) a cell grant or use
//! changes the bag on its own tick: everything that could have read the bag
//! rewinds with it. When a room rewinds alone, another room may read the bag
//! between the write and the rewind. An undo of the write cannot undo what
//! that reader did with it, so the bag takes no write from such a room until
//! the writing tick is confirmed.
//!
//! - **Authority selects the road; admission only accepts or refuses on it.**
//!   [`admit_grant`] / [`admit_use`] classify the request: a body under one
//!   rollback authority applies now ([`BagAdmission::Immediate`]); a body in
//!   a room that rewinds alone is held ([`BagAdmission::Held`]); a request
//!   whose authority, identity or tick cannot be told under independent
//!   rooms is refused. A refusal never falls through to an immediate write.
//! - **Each room spends its own proposal credits.** A room holds at most
//!   [`PROPOSAL_CREDITS_PER_ROOM`] proposals in flight. A proposal takes a
//!   credit when it is made and gives it back at its decision's cut. The
//!   balance at a tick is counted from that room's own proposals and the
//!   decisions on them (kept for the rewind horizon), so a replay of the room
//!   meets the admissions it met before, whatever another room has queued
//!   since. A request past the room's credits is refused.
//! - **The bag decides on confirmation.** On the live tick that confirms a
//!   proposal's tick the bag decides it once, by (tick, room, order within
//!   the room's tick): a grant enters the bag; a use takes one cell (an
//!   allocation) or is refused (`bag_empty`, or `settlement_full` when no
//!   allocation row can be reserved, with no debit).
//! - **An allocation owns its cell until it is settled.** The allocation is
//!   disposed of in the body's room at its cut: used (the heal or the Mana)
//!   or unused (the body is gone, or the cell does nothing for it). The
//!   disposition belongs to the room's history: a replay of the cut
//!   recomputes it, without another debit. When the cut is confirmed the
//!   recorded disposition settles once: an unused cell goes back to the bag,
//!   a used one is consumed. Nothing is refunded before that, and an open
//!   allocation never expires.
//!
//! The owner page is `docs/planning/engine/causal-world-commit.md` (CW2 and
//! its follow-up).

use bevy::prelude::*;

use ambition_items::{Item, OwnedItems};
use ambition_platformer2d_core::ConfirmedFrameBoundary;
use ambition_platformer2d_shared_tangle::lifecycle::{
    simulation_island, InRoomInstance, LiveRoomInstance, RoomReplay, RoomWrite, SimulationIsland, SimulationIslands,
    ROOM_REWIND_HORIZON_TICKS,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// How many proposals one room may have in flight.
pub const PROPOSAL_CREDITS_PER_ROOM: usize = 16;
/// How many allocations may be open or kept as receipts at once. A use that
/// cannot reserve its row is refused before the bag is debited.
pub const MAX_ALLOCATIONS: usize = 64;

/// What a room asked of the shared bag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BagOperation {
    /// A conversation gave `count` of `item`.
    Grant { item: Item, count: u32 },
    /// The body `body` uses one `item` (a cell).
    Use { item: Item, body: SimId },
}

/// How the bag took one request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BagAdmission {
    /// One rollback authority: the caller applies it on this tick.
    Immediate,
    /// Held for the bag's decision once its tick is confirmed.
    Held,
    /// Refused, with nothing written and no effect: `no_authority`,
    /// `no_identity`, `no_tick` or `credits_spent`.
    Refused(&'static str),
}

/// One proposal's identity: the room and tick that made it, and its order
/// among that room's proposals on that tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProposalId {
    pub written: RoomWrite,
    pub order: u32,
}

/// The bag's decision on one confirmed proposal, or the settlement of one
/// allocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BagDecision {
    /// The live tick it was decided on: the proposal's credit returns to its
    /// room here.
    pub tick: u64,
    pub id: ProposalId,
    pub operation: BagOperation,
    /// A decision: `granted`, `allocated`, or the refusals `bag_empty` and
    /// `settlement_full`. A settlement: `consumed` or `returned`.
    pub outcome: &'static str,
}

impl BagDecision {
    fn settles(&self) -> bool {
        matches!(self.outcome, "consumed" | "returned")
    }
}

#[derive(Clone, Debug)]
struct Proposed {
    id: ProposalId,
    operation: BagOperation,
}

/// What the body's room did with an allocated cell at its cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Disposition {
    Used,
    Unused,
}

/// One cell the bag gave a body, disposed of in its room at `cut`.
#[derive(Clone, Debug)]
struct Allocated {
    id: ProposalId,
    cut: u64,
    body: SimId,
    item: Item,
    disposition: Disposition,
    settled: bool,
}

/// Proposals from rooms that rewind alone, waiting for their tick to be
/// confirmed; the allocations the bag made and their dispositions; and the
/// bag's decisions. Settled allocations and decisions are kept for the
/// rewind horizon ([`ROOM_REWIND_HORIZON_TICKS`]); an open allocation is
/// kept until it settles.
#[derive(Resource, Clone, Debug, Default)]
pub struct HeldBagOperations {
    proposed: Vec<Proposed>,
    allocated: Vec<Allocated>,
    decided: Vec<BagDecision>,
}

impl HeldBagOperations {
    /// Proposals waiting for their tick to be confirmed.
    pub fn len(&self) -> usize {
        self.proposed.len()
    }
    pub fn is_empty(&self) -> bool {
        self.proposed.is_empty()
    }
    /// The bag's decisions and settlements within the rewind horizon, in
    /// the order made.
    pub fn decisions(&self) -> &[BagDecision] {
        &self.decided
    }
    /// Allocations not yet settled.
    pub fn open_allocations(&self) -> usize {
        self.allocated.iter().filter(|row| !row.settled).count()
    }
    /// Whether [`settle_bag_operations`] has anything to do.
    pub fn busy(&self) -> bool {
        !self.proposed.is_empty() || !self.allocated.is_empty()
    }
    /// The credits `room` has left at `tick`: its proposals made by then
    /// that were not yet decided by then. Counted from the room's own
    /// history, so a replay of the room meets the balance it met before.
    pub fn credits(&self, room: LiveRoomInstance, tick: u64) -> usize {
        let made_by = |id: &ProposalId| id.written.room == room && id.written.tick <= tick;
        let waiting = self.proposed.iter().filter(|row| made_by(&row.id)).count();
        let deciding = self.decided.iter().filter(|row| !row.settles() && made_by(&row.id) && row.tick > tick).count();
        PROPOSAL_CREDITS_PER_ROOM.saturating_sub(waiting + deciding)
    }
}

/// The room `body` proposes from, when it is in a room that rewinds alone.
pub fn proposing_room(world: &World, body: Entity) -> Option<LiveRoomInstance> {
    match simulation_island(world, body)? {
        SimulationIsland::Room(_, room) => Some(room),
        SimulationIsland::Session(_) => None,
    }
}

/// The rollback authority a request from `body` belongs to: `Ok(None)` for
/// one authority, `Ok(Some(room))` for a room that rewinds alone, and a
/// refusal when independent rooms are on and the request names no body
/// whose room can be told.
fn authority(world: &World, body: Option<Entity>) -> Result<Option<LiveRoomInstance>, &'static str> {
    let independent = world.get_resource::<SimulationIslands>().is_some_and(|policy| (policy.0)(world));
    match body.and_then(|body| simulation_island(world, body)) {
        Some(SimulationIsland::Room(_, room)) => Ok(Some(room)),
        Some(SimulationIsland::Session(_)) => Ok(None),
        None if independent => Err("no_authority"),
        None => Ok(None),
    }
}

/// Admit a conversation's grant from `body`'s seat. An immediate grant is
/// the caller's to apply.
pub fn admit_grant(world: &mut World, body: Option<Entity>, item: Item, count: u32) -> BagAdmission {
    match authority(world, body) {
        Err(why) => BagAdmission::Refused(why),
        Ok(None) => BagAdmission::Immediate,
        Ok(Some(room)) => hold(world, room, BagOperation::Grant { item, count }),
    }
}

/// Admit `body`'s use of one `item`. An immediate use is the caller's to
/// apply.
pub fn admit_use(world: &mut World, body: Entity, item: Item) -> BagAdmission {
    match authority(world, Some(body)) {
        Err(why) => BagAdmission::Refused(why),
        Ok(None) => BagAdmission::Immediate,
        Ok(Some(room)) => match world.get::<SimId>(body).cloned() {
            Some(id) => hold(world, room, BagOperation::Use { item, body: id }),
            None => BagAdmission::Refused("no_identity"),
        },
    }
}

/// Hold `operation` from `room` on this tick, spending one of its credits.
fn hold(world: &mut World, room: LiveRoomInstance, operation: BagOperation) -> BagAdmission {
    let Some(tick) = world.get_resource::<ambition_time::SimTick>().map(|tick| tick.0) else {
        return BagAdmission::Refused("no_tick");
    };
    let mut store = world.get_resource_or_init::<HeldBagOperations>();
    if store.credits(room, tick) == 0 {
        warn!(target: "crate::committed_bag", "bag proposal refused, {room:?} has no credits left: {operation:?}");
        return BagAdmission::Refused("credits_spent");
    }
    let written = RoomWrite { room, tick };
    let order = store.proposed.iter().filter(|row| row.id.written == written).count() as u32;
    store.proposed.push(Proposed { id: ProposalId { written, order }, operation });
    BagAdmission::Held
}

/// Whether one `item` does anything for `body`: a Health Cell for a body with
/// health, a Mana Cell for a body that holds Mana.
pub fn cell_takes_effect(world: &World, body: Entity, item: Item) -> bool {
    match item {
        Item::HealthCell => world.get::<ambition_characters::actor::BodyHealth>(body).is_some(),
        Item::ManaCell => world
            .get::<ambition_platformer2d_core::resources::ActorResources>(body)
            .is_some_and(|bank| bank.level_of(&ambition_abilities::mana::MANA).is_some()),
        _ => false,
    }
}

/// Apply one `item`'s effect to `body`: the heal is a request the heal
/// applier reads later this tick; the Mana is refilled now.
pub fn apply_cell_effect(world: &mut World, body: Entity, item: Item) {
    match item {
        Item::HealthCell => {
            world.write_message(ambition_damage::healing::BodyHealRequested::new(crate::narrative::HEALTH_CELL_HEAL, body));
        }
        Item::ManaCell => {
            if let Some(mut bank) = world.get_mut::<ambition_platformer2d_core::resources::ActorResources>(body) {
                if let Some(mana) = bank.level_of_mut(&ambition_abilities::mana::MANA) {
                    mana.refill(crate::narrative::MANA_CELL_RESTORE);
                }
            }
        }
        _ => {}
    }
}

/// On a live tick, decide every proposal whose tick is confirmed, then
/// settle every allocation whose cut is confirmed. While a room replays,
/// dispose again of the allocations it was given on the tick now running.
/// (sim)
pub fn settle_bag_operations(world: &mut World) {
    if world.contains_resource::<RoomReplay>() {
        dispose_again(world);
    } else {
        decide_confirmed(world);
        settle_confirmed(world);
    }
}

fn confirmed(world: &World, tick: u64) -> bool {
    world.get_resource::<ConfirmedFrameBoundary>().is_none_or(|boundary| boundary.is_confirmed(tick as i32))
}

fn decide_confirmed(world: &mut World) {
    let now = world.resource::<ambition_time::SimTick>().0;
    let boundary = world.get_resource::<ConfirmedFrameBoundary>().copied();
    let mut store = world.resource_mut::<HeldBagOperations>();
    let kept = |tick: u64| tick.saturating_add(ROOM_REWIND_HORIZON_TICKS) >= now;
    store.allocated.retain(|row| !row.settled || kept(row.cut));
    store.decided.retain(|row| kept(row.tick));
    let (mut ready, waiting): (Vec<Proposed>, Vec<Proposed>) = std::mem::take(&mut store.proposed)
        .into_iter()
        .partition(|row| boundary.is_none_or(|boundary| boundary.is_confirmed(row.id.written.tick as i32)));
    store.proposed = waiting;
    ready.sort_by_key(|row| (row.id.written.tick, row.id.written.room.ordinal(), row.id.order));
    for row in ready {
        let outcome = decide(world, now, &row);
        info!(target: "crate::committed_bag", "{:?} from {:?} -> {outcome}", row.operation, row.id);
        world.resource_mut::<HeldBagOperations>().decided.push(BagDecision {
            tick: now,
            id: row.id,
            operation: row.operation,
            outcome,
        });
    }
}

fn decide(world: &mut World, now: u64, row: &Proposed) -> &'static str {
    match &row.operation {
        BagOperation::Grant { item, count } => {
            let before = world.resource::<OwnedItems>().count(*item);
            world.resource_scope(|world, mut bag: Mut<OwnedItems>| {
                bag.grant(ambition_items::item_catalog(world), *item, *count);
            });
            let gained = world.resource::<OwnedItems>().count(*item) - before;
            // Committed: no room rewind takes it back, so it names no room.
            if let Some(mut since) = world.get_resource_mut::<crate::pickup::BagChangesSinceCheckpoint>() {
                since.record(crate::pickup::BagChange {
                    item: Some(*item),
                    gained,
                    gave: 0,
                    payer: None,
                    coins: 0,
                    written: None,
                    retention: crate::pickup::minted_horizon::BagChangeRetention::NarrativeFacts,
                });
            }
            "granted"
        }
        BagOperation::Use { item, body } => {
            // Reserve the row that settles the allocation before debiting.
            if world.resource::<HeldBagOperations>().allocated.len() >= MAX_ALLOCATIONS {
                return "settlement_full";
            }
            if world.resource_mut::<OwnedItems>().take(*item, 1) == 0 {
                return "bag_empty";
            }
            let mut allocated =
                Allocated { id: row.id, cut: now, body: body.clone(), item: *item, disposition: Disposition::Unused, settled: false };
            allocated.disposition = dispose(world, &allocated);
            world.resource_mut::<HeldBagOperations>().allocated.push(allocated);
            "allocated"
        }
    }
}

/// The body's room disposes of one allocation at its cut: the effect, for a
/// body the cell does something for; otherwise the cell stays unused. The
/// body is the one carrying the identity in the proposing room.
fn dispose(world: &mut World, allocated: &Allocated) -> Disposition {
    let room = allocated.id.written.room;
    let body = world
        .query::<(Entity, &SimId, &InRoomInstance)>()
        .iter(world)
        .find(|(_, id, at)| **id == allocated.body && at.0 == room)
        .map(|(body, ..)| body);
    match body {
        Some(body) if cell_takes_effect(world, body, allocated.item) => {
            apply_cell_effect(world, body, allocated.item);
            Disposition::Used
        }
        _ => Disposition::Unused,
    }
}

/// Settle each open allocation whose cut is confirmed, by the disposition
/// its room recorded: an unused cell goes back to the bag.
fn settle_confirmed(world: &mut World) {
    let now = world.resource::<ambition_time::SimTick>().0;
    let due: Vec<usize> = {
        let store = world.resource::<HeldBagOperations>();
        (0..store.allocated.len()).filter(|&at| !store.allocated[at].settled && confirmed(world, store.allocated[at].cut)).collect()
    };
    for at in due {
        let row = {
            let mut store = world.resource_mut::<HeldBagOperations>();
            store.allocated[at].settled = true;
            store.allocated[at].clone()
        };
        let outcome = match row.disposition {
            Disposition::Used => "consumed",
            Disposition::Unused => {
                world.resource_mut::<OwnedItems>().put_back(row.item, 1);
                "returned"
            }
        };
        info!(target: "crate::committed_bag", "allocation {:?} -> {outcome}", row.id);
        world.resource_mut::<HeldBagOperations>().decided.push(BagDecision {
            tick: now,
            id: row.id,
            operation: BagOperation::Use { item: row.item, body: row.body },
            outcome,
        });
    }
}

fn dispose_again(world: &mut World) {
    let replay = *world.resource::<RoomReplay>();
    let now = world.resource::<ambition_time::SimTick>().0;
    let due: Vec<usize> = {
        let store = world.resource::<HeldBagOperations>();
        (0..store.allocated.len())
            .filter(|&at| {
                let row = &store.allocated[at];
                !row.settled && row.cut == now && replay.allows(Some(row.id.written.room))
            })
            .collect()
    };
    for at in due {
        let row = world.resource::<HeldBagOperations>().allocated[at].clone();
        let disposition = dispose(world, &row);
        world.resource_mut::<HeldBagOperations>().allocated[at].disposition = disposition;
    }
}

/// A rewind of `room` to `saved_at` drops the proposals it made after it.
/// Their credits come back with them; the decided ones were confirmed, so
/// the rewind cannot reach them.
pub fn retract_room(world: &mut World, room: LiveRoomInstance, saved_at: u64) {
    if let Some(mut store) = world.get_resource_mut::<HeldBagOperations>() {
        store.proposed.retain(|row| !row.id.written.retracted_by(room, saved_at));
    }
}

/// A new session starts with nothing held.
pub fn forget(mut store: ResMut<HeldBagOperations>) {
    *store = HeldBagOperations::default();
}

#[cfg(test)]
mod retirement_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::{SessionScopeId, SessionScopedEntity};

    const LAB: LiveRoomInstance = LiveRoomInstance::ACTIVATION.next();
    const HUB: LiveRoomInstance = LAB.next();
    /// A later live instance of the lab, minted after the first retired.
    const LAB_AGAIN: LiveRoomInstance = HUB.next();

    /// Two rooms that rewind alone, one Health Cell in the bag, and nothing
    /// confirmed after tick 9.
    fn world() -> World {
        let mut world = World::new();
        world.insert_resource(SimulationIslands(|_| true));
        world.insert_resource(ambition_time::SimTick(10));
        world.insert_resource(ConfirmedFrameBoundary { current: 10, confirmed: 9, session: 0 });
        world.init_resource::<OwnedItems>();
        world.resource_mut::<OwnedItems>().put_back(Item::HealthCell, 1);
        world.init_resource::<HeldBagOperations>();
        world.init_resource::<bevy::ecs::message::Messages<ambition_damage::healing::BodyHealRequested>>();
        world
    }

    fn body(world: &mut World, id: &str, room: LiveRoomInstance) -> Entity {
        world
            .spawn((
                SimId::placement(id),
                InRoomInstance(room),
                SessionScopedEntity(SessionScopeId(1)),
                ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(3)),
            ))
            .id()
    }

    /// Confirm every tick up to `tick` and let the bag decide and settle.
    fn confirm(world: &mut World, tick: u64) {
        world.resource_mut::<ambition_time::SimTick>().0 = tick;
        *world.resource_mut::<ConfirmedFrameBoundary>() = ConfirmedFrameBoundary { current: tick as i32, confirmed: tick as i32, session: 0 };
        settle_bag_operations(world);
    }

    fn outcomes(world: &World) -> Vec<&'static str> {
        world.resource::<HeldBagOperations>().decisions().iter().map(|row| row.outcome).collect()
    }

    fn heals(world: &World) -> usize {
        world.resource::<bevy::ecs::message::Messages<ambition_damage::healing::BodyHealRequested>>().len()
    }

    /// CW2 retirement: a use whose recipient's room retires before the
    /// decision gives that body nothing, and the cell comes back once. A
    /// later instance of the same room, with a body of the same placement,
    /// is a different address: the old allocation does not reach it. The
    /// control keeps the recipient in its room, and the cell is used.
    #[test]
    fn a_recipient_whose_room_retires_before_the_cut_gets_nothing_and_the_cell_returns_once() {
        let mut stays = world();
        let bob = body(&mut stays, "bob", LAB);
        assert_eq!(admit_use(&mut stays, bob, Item::HealthCell), BagAdmission::Held);
        confirm(&mut stays, 12);
        assert_eq!((outcomes(&stays), heals(&stays)), (vec!["allocated", "consumed"], 1), "control: a recipient in its room is healed");

        let mut leaves = world();
        let bob = body(&mut leaves, "bob", LAB);
        assert_eq!(admit_use(&mut leaves, bob, Item::HealthCell), BagAdmission::Held);
        // The lab retires: Bob is in the hub, and a new lab holds a body of
        // the same placement.
        leaves.entity_mut(bob).insert(InRoomInstance(HUB));
        body(&mut leaves, "bob", LAB_AGAIN);
        confirm(&mut leaves, 12);
        assert_eq!(outcomes(&leaves), vec!["allocated", "returned"]);
        assert_eq!(heals(&leaves), 0, "the cell reached a body at another address");
        assert_eq!(leaves.resource::<OwnedItems>().count(Item::HealthCell), 1, "the cell did not come back");
        confirm(&mut leaves, 13);
        assert_eq!(leaves.resource::<OwnedItems>().count(Item::HealthCell), 1, "the cell came back twice");
        assert_eq!(leaves.resource::<HeldBagOperations>().open_allocations(), 0);
    }

    /// CW2 retirement: a proposal whose source room retires before its tick
    /// is confirmed is still decided at its cut. The retired room can rewind
    /// no more, so what it proposed is final; the record is not dropped, and
    /// nothing is decided before the cut.
    #[test]
    fn a_proposal_whose_source_room_retires_is_decided_at_its_cut() {
        let mut world = world();
        let alice = body(&mut world, "alice", HUB);
        assert_eq!(admit_grant(&mut world, Some(alice), Item::HealthCell, 1), BagAdmission::Held);
        world.despawn(alice);
        settle_bag_operations(&mut world);
        assert!(outcomes(&world).is_empty(), "a proposal was decided before its tick was confirmed");
        assert_eq!(world.resource::<HeldBagOperations>().len(), 1, "the retired room's proposal was dropped");
        confirm(&mut world, 12);
        assert_eq!(outcomes(&world), vec!["granted"]);
    }
}
