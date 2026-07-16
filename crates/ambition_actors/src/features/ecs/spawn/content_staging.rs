//! **`RoomContentStagingRegistry` — the open seam for content-staged room
//! occupants** (N3.2b closeout).
//!
//! Some room occupants are not authored `RoomSpec` placements: the spectator
//! duel's fighters, a demo level's walkers. Content used to stage them from
//! systems consuming the `RoomLoaded` notification — which had two structural
//! faults:
//!
//! 1. **The occupants were invisible to room construction.** A snapshot restore
//!    that stages a room (netcode.md N3.2b) rebuilds exactly what construction
//!    builds; an occupant created only by a future-frame notification consumer
//!    came back as a bare identity with no authored components.
//! 2. **The staging tick was not a sim fact.** The consumers ran on the
//!    presentation schedule (`Update`), so *when* the occupants appeared,
//!    relative to sim ticks, was a function of frame rate.
//!
//! This registry closes both: providers/content register a **pure** stager —
//! `RoomSpec` in, [`SpawnActorRequest`]s out — and room construction
//! ([`spawn_room_feature_entities_with_registry`](super::spawn_room_feature_entities_with_registry))
//! drains every registered stager for the room being staged, on the sim side,
//! in both the normal load path and the restore staging path. `RoomLoaded`
//! remains a pure downstream notification (resource re-arms, presentation
//! beats); it no longer creates snapshot-authoritative entities.
//!
//! Purity is what makes the seam preflightable: a stager must be a function of
//! the `RoomSpec` alone, so a mutation-free caller (`RoomStaging::prepare`, a
//! roster preflight) can ask "what WOULD this room stage?" without staging it.

use std::sync::Arc;

use bevy::ecs::resource::Resource;

use super::super::spawn_actors::SpawnActorRequest;
use crate::rooms::RoomSpec;

/// A registered content stager: a pure function from the authored room to the
/// actors content stages into it.
type Stager = Arc<dyn Fn(&RoomSpec) -> Vec<SpawnActorRequest> + Send + Sync>;

/// App-installed registry of per-room content stagers. Clone-cheap (the
/// stagers are `Arc`s), like the placement-lowering registry it mirrors.
///
/// Registration order is preserved and is the drain order — a function of
/// plugin build order, hence of the binary, hence identical across two sims of
/// the same build (the same rule `SnapshotRegistry` follows).
#[derive(Resource, Clone, Default)]
pub struct RoomContentStagingRegistry {
    stagers: Vec<(String, Stager)>,
}

impl RoomContentStagingRegistry {
    /// Register a pure content stager for `room_id`. The stager runs every
    /// time that room's contents are staged: activation, transition, reset,
    /// hot-reload, and snapshot-restore staging alike.
    pub fn register(
        &mut self,
        room_id: impl Into<String>,
        stager: impl Fn(&RoomSpec) -> Vec<SpawnActorRequest> + Send + Sync + 'static,
    ) {
        self.stagers.push((room_id.into(), Arc::new(stager)));
    }

    /// Every request content stages into `room`, in registration order.
    pub fn requests_for(&self, room: &RoomSpec) -> Vec<SpawnActorRequest> {
        self.stagers
            .iter()
            .filter(|(room_id, _)| *room_id == room.id)
            .flat_map(|(_, stager)| stager(room))
            .collect()
    }

    /// The feature ids `requests_for` would stage — the mutation-free roster
    /// prediction a restore preflight needs.
    pub fn staged_ids_for(&self, room: &RoomSpec) -> Vec<String> {
        self.requests_for(room)
            .into_iter()
            .map(|request| request.id)
            .collect()
    }
}
