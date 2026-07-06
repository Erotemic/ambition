//! Authored placement RECORDS on the room IR — the [W-b] shape
//! (decomposition.md, W-track ruling; architecture.md §4b).
//!
//! A record joins the spatial footprint (record-level: `id` + `aabb`, owned
//! by the space IR) to the CLOSED Tier-0 authored schema
//! ([`ambition_entity_catalog::placements::PlacementSchema`] — what the
//! author SAID). Backend converters (LDtk today) parse entities into
//! records; a lowering registry (W-queue step 3) maps each record → live
//! entities at room load. This module is the space IR's half only — it
//! lives in `gameplay_core::world` today and moves whole to
//! `ambition_world` at W3.

use ambition_engine_core as ae;
use ambition_entity_catalog::placements::PlacementSchema;

/// One authored placement: WHERE (footprint) + WHAT (schema), durably named.
///
/// `id` is REQUIRED ([W-d]): the LDtk `iid` for authored maps, or the
/// bake-synthesized `"{room}:{index}"` for generated/`ron-room` content.
/// `WorldDelta::RemovePlacement`, SimView identity, replay, and netcode
/// SimIds all join on it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlacementRecord {
    pub id: ae::PlacementId,
    /// The closed Tier-0 authored schema (§4b.3).
    pub schema: PlacementSchema,
    /// Authored footprint (pos + size).
    pub aabb: ae::Aabb,
}

impl PlacementRecord {
    pub fn new(id: impl Into<String>, schema: PlacementSchema, aabb: ae::Aabb) -> Self {
        Self {
            id: ae::PlacementId::new(id),
            schema,
            aabb,
        }
    }
}
