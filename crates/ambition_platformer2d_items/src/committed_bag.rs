//! The shared bag's committed road for rooms that rewind alone (CW2).
//!
//! Under one rollback authority (local play, GGRS) a cell grant or use
//! changes the bag on its own tick: everything that could have read the bag
//! rewinds with it. When a room rewinds alone, another room may read the bag
//! between the write and the rewind. An undo of the write cannot undo what
//! that reader did with it, so the bag takes no write from such a room until
//! the writing tick is confirmed:
//!
//! - A room's grant or cell use is a proposal, private to its room. A rewind
//!   of the room drops the proposals it made after its save, and its replay
//!   proposes again what it does again.
//! - On the live tick that confirms a proposal's tick, the bag decides it
//!   once, in the order proposals were made: a grant enters the bag; a use
//!   takes one cell from the committed bag (an allocation to the body that
//!   asked) or is refused when the bag has none.
//! - An allocation is spent in the body's room on that tick (the cut): the
//!   heal or the Mana together with the cell. A body the cell would do
//!   nothing for (gone from the room, no health, no Mana) leaves the cell
//!   unused, and the room proposes its return, which commits like any other
//!   proposal. A replay of the room that runs the cut spends the allocation
//!   again by the same rule, without taking another cell from the bag.
//!
//! The owner page is `docs/planning/engine/causal-world-commit.md` (CW2).

use bevy::prelude::*;

use ambition_items::{Item, OwnedItems};
use ambition_platformer2d_core::ConfirmedFrameBoundary;
use ambition_platformer2d_shared_tangle::lifecycle::{
    simulation_island, InRoomInstance, LiveRoomInstance, RoomReplay, RoomWrite, SimulationIsland, ROOM_REWIND_HORIZON_TICKS,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// The most proposals held for confirmation at once.
pub const MAX_HELD_BAG_OPERATIONS: usize = 64;

/// What a room asked of the shared bag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BagOperation {
    /// A conversation gave `count` of `item`.
    Grant { item: Item, count: u32 },
    /// The body `body` uses one `item` (a cell).
    Use { item: Item, body: SimId },
    /// An allocated cell its body did not use goes back.
    Return { item: Item },
}

/// The bag's decision on one confirmed proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BagDecision {
    /// The live tick it was decided on.
    pub tick: u64,
    /// The proposal, with the room and tick that made it.
    pub written: RoomWrite,
    pub operation: BagOperation,
    /// `granted`, `allocated`, `returned`, or the refusal `bag_empty`.
    pub outcome: &'static str,
}

#[derive(Clone, Debug)]
struct Proposed {
    written: RoomWrite,
    operation: BagOperation,
}

/// One cell the bag gave a body, spent in its room at `cut`.
#[derive(Clone, Debug)]
struct Allocated {
    cut: u64,
    room: LiveRoomInstance,
    body: SimId,
    item: Item,
}

/// Proposals from rooms that rewind alone, waiting for their tick to be
/// confirmed; the allocations a replay of their room spends again; and the
/// bag's decisions. Allocations and decisions are kept for the rewind
/// horizon ([`ROOM_REWIND_HORIZON_TICKS`]).
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
    /// The bag's decisions within the rewind horizon, in the order made.
    pub fn decisions(&self) -> &[BagDecision] {
        &self.decided
    }
    /// Whether [`settle_bag_operations`] has anything to do.
    pub fn busy(&self) -> bool {
        !self.proposed.is_empty() || !self.allocated.is_empty()
    }
}

/// The room `body` proposes from, when it is in a room that rewinds alone.
pub fn proposing_room(world: &World, body: Entity) -> Option<LiveRoomInstance> {
    match simulation_island(world, body)? {
        SimulationIsland::Room(_, room) => Some(room),
        SimulationIsland::Session(_) => None,
    }
}

/// Hold `operation` from the room and tick in `written` until that tick is
/// confirmed. False, with a warning, when the store is full.
pub fn propose(world: &mut World, written: RoomWrite, operation: BagOperation) -> bool {
    let mut store = world.get_resource_or_init::<HeldBagOperations>();
    if store.proposed.len() >= MAX_HELD_BAG_OPERATIONS {
        warn!(target: "crate::committed_bag", "bag proposal refused, the queue is full: {operation:?}");
        return false;
    }
    store.proposed.push(Proposed { written, operation });
    true
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

/// On a live tick, decide every proposal whose tick is confirmed. While a
/// room replays, spend again the allocations it was given on the tick now
/// running. (sim)
pub fn settle_bag_operations(world: &mut World) {
    if world.contains_resource::<RoomReplay>() {
        spend_allocations_again(world);
    } else {
        decide_confirmed(world);
    }
}

fn decide_confirmed(world: &mut World) {
    let boundary = world.get_resource::<ConfirmedFrameBoundary>().copied();
    let now = world.resource::<ambition_time::SimTick>().0;
    let mut store = world.resource_mut::<HeldBagOperations>();
    let kept = |tick: u64| tick.saturating_add(ROOM_REWIND_HORIZON_TICKS) >= now;
    store.allocated.retain(|row| kept(row.cut));
    store.decided.retain(|row| kept(row.tick));
    let (mut ready, waiting): (Vec<Proposed>, Vec<Proposed>) = std::mem::take(&mut store.proposed)
        .into_iter()
        .partition(|row| boundary.is_none_or(|boundary| boundary.is_confirmed(row.written.tick as i32)));
    store.proposed = waiting;
    // In the order they were made: by tick, then by room. A room's own
    // proposals on one tick keep the order its replay makes them in.
    ready.sort_by_key(|row| (row.written.tick, row.written.room.ordinal()));
    for row in ready {
        let outcome = decide(world, now, &row);
        info!(target: "crate::committed_bag", "{:?} from {:?} -> {outcome}", row.operation, row.written);
        world.resource_mut::<HeldBagOperations>().decided.push(BagDecision {
            tick: now,
            written: row.written,
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
        BagOperation::Return { item } => {
            world.resource_mut::<OwnedItems>().put_back(*item, 1);
            "returned"
        }
        BagOperation::Use { item, body } => {
            if world.resource_mut::<OwnedItems>().take(*item, 1) == 0 {
                return "bag_empty";
            }
            let allocated = Allocated { cut: now, room: row.written.room, body: body.clone(), item: *item };
            spend(world, &allocated);
            world.resource_mut::<HeldBagOperations>().allocated.push(allocated);
            "allocated"
        }
    }
}

/// The body's room spends one allocation at its cut: the effect, or, for a
/// body the cell does nothing for, the cell's return.
fn spend(world: &mut World, allocated: &Allocated) {
    let body = world
        .query::<(Entity, &SimId, &InRoomInstance)>()
        .iter(world)
        .find(|(_, id, room)| **id == allocated.body && room.0 == allocated.room)
        .map(|(body, ..)| body);
    match body {
        Some(body) if cell_takes_effect(world, body, allocated.item) => apply_cell_effect(world, body, allocated.item),
        _ => {
            let written = RoomWrite { room: allocated.room, tick: allocated.cut };
            let rewinds_alone = body.is_none_or(|body| proposing_room(world, body).is_some());
            if !rewinds_alone || !propose(world, written, BagOperation::Return { item: allocated.item }) {
                world.resource_mut::<OwnedItems>().put_back(allocated.item, 1);
            }
        }
    }
}

fn spend_allocations_again(world: &mut World) {
    let replay = *world.resource::<RoomReplay>();
    let now = world.resource::<ambition_time::SimTick>().0;
    let rows: Vec<Allocated> = world
        .resource::<HeldBagOperations>()
        .allocated
        .iter()
        .filter(|row| row.cut == now && replay.allows(Some(row.room)))
        .cloned()
        .collect();
    for row in rows {
        spend(world, &row);
    }
}

/// A rewind of `room` to `saved_at` drops the proposals it made after it.
pub fn retract_room(world: &mut World, room: LiveRoomInstance, saved_at: u64) {
    if let Some(mut store) = world.get_resource_mut::<HeldBagOperations>() {
        store.proposed.retain(|row| !row.written.retracted_by(room, saved_at));
    }
}

/// A new session starts with nothing held.
pub fn forget(mut store: ResMut<HeldBagOperations>) {
    *store = HeldBagOperations::default();
}
