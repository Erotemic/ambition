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
use ambition_entity_catalog::placements::{PlacementKind, PlacementSchema};
use ambition_platformer_primitives::lifecycle::SessionSpawnScope;
use bevy_app::App;
use bevy_ecs::prelude::{Commands, Resource};
use std::collections::HashMap;

/// One authored placement: WHERE (footprint) + WHAT (schema), durably named.
///
/// `id` is REQUIRED ([W-d]): the LDtk `iid` for authored maps, or the
/// bake-synthesized `"{room}:{index}"` for generated/`ron-room` content.
/// `WorldDelta::RemovePlacement`, SimView identity, replay, and netcode
/// SimIds all join on it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlacementRecord {
    pub id: ae::PlacementId,
    /// Authored display label (editor-facing / entity naming / nameplates —
    /// the `PropSpec.name` precedent). Defaults to the id when a bake has no
    /// name. RULING (fable final audit F7): record-level metadata like `id`,
    /// NOT schema data — lowering must not fall back to iids for labels.
    #[serde(default)]
    pub name: String,
    /// The closed Tier-0 authored schema (§4b.3).
    pub schema: PlacementSchema,
    /// Authored footprint (pos + size).
    pub aabb: ae::Aabb,
}

impl PlacementRecord {
    pub fn new(id: impl Into<String>, schema: PlacementSchema, aabb: ae::Aabb) -> Self {
        let id = ae::PlacementId::new(id);
        Self {
            name: id.as_str().to_string(),
            id,
            schema,
            aabb,
        }
    }

    pub fn kind(&self) -> PlacementKind {
        self.schema.kind()
    }
}

/// Room-load context handed to placement interpreters. It wraps exactly the
/// facts a lowering function needs today and can grow by explicit need.
pub struct LoweringCtx<'w, 's, 'a> {
    pub commands: &'a mut Commands<'w, 's>,
    pub room_id: &'a str,
    pub paths: &'a [(String, ae::KinematicPath)],
    /// Gameplay-session ownership captured when room staging was requested.
    pub session_scope: SessionSpawnScope,
}

pub type LoweringFn = for<'w, 's, 'a> fn(&PlacementRecord, &mut LoweringCtx<'w, 's, 'a>);

/// Registry from authored placement kind to the simulation/content interpreter
/// that lowers the record into live room-scoped entities.
#[derive(Resource, Clone, Default)]
pub struct PlacementLoweringRegistry {
    interpreters: HashMap<PlacementKind, LoweringFn>,
}

impl PlacementLoweringRegistry {
    pub fn register(&mut self, kind: PlacementKind, f: LoweringFn) {
        if self.interpreters.insert(kind, f).is_some() {
            panic!("duplicate placement lowering interpreter registered for {kind:?}");
        }
    }

    pub fn registered_kinds(&self) -> Vec<PlacementKind> {
        // AMBITION_REVIEW(determinism): the hash-ordered keys are sorted on the very
        // next line, before anything can observe them, and this runs once at room
        // load rather than in the tick. `PlacementKind` has no `Ord`, so a
        // `BTreeMap` would need one invented for a debug-name sort we already do.
        let mut kinds: Vec<_> = self.interpreters.keys().copied().collect();
        kinds.sort_by_key(|kind| format!("{kind:?}"));
        kinds
    }

    fn interpreter_for(&self, record: &PlacementRecord, room_id: &str) -> LoweringFn {
        let kind = record.kind();
        let Some(lower) = self.interpreters.get(&kind) else {
            panic!(
                "unknown placement kind {kind:?} for placement '{}' in room '{}'; registered kinds: {:?}",
                record.id.as_str(),
                room_id,
                self.registered_kinds(),
            );
        };
        *lower
    }

    pub fn lower<'w, 's, 'a>(&self, record: &PlacementRecord, ctx: &mut LoweringCtx<'w, 's, 'a>) {
        let lower = self.interpreter_for(record, ctx.room_id);
        lower(record, ctx);
    }
}

pub trait PlacementLoweringAppExt {
    fn register_placement_interpreter(&mut self, kind: PlacementKind, f: LoweringFn) -> &mut Self;
}

impl PlacementLoweringAppExt for App {
    fn register_placement_interpreter(&mut self, kind: PlacementKind, f: LoweringFn) -> &mut Self {
        if !self
            .world()
            .contains_resource::<PlacementLoweringRegistry>()
        {
            self.init_resource::<PlacementLoweringRegistry>();
        }
        self.world_mut()
            .resource_mut::<PlacementLoweringRegistry>()
            .register(kind, f);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::placements::{DamageKind, DamageTeam, HazardRespawn, HazardSpec};

    fn sample_record(id: &str) -> PlacementRecord {
        PlacementRecord::new(
            id,
            PlacementSchema::Hazard(HazardSpec {
                damage: 1,
                knockback: [0.0, 0.0],
                kind: DamageKind::Hazard,
                team: DamageTeam::Environment,
                hitstop_seconds: 0.0,
                respawn: HazardRespawn::Never,
                path_id: None,
            }),
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(4.0)),
        )
    }

    fn noop_lowering(_record: &PlacementRecord, _ctx: &mut LoweringCtx<'_, '_, '_>) {}

    #[test]
    fn placement_schema_reports_kind() {
        assert_eq!(sample_record("haz").kind(), PlacementKind::Hazard);
    }

    #[test]
    #[should_panic(expected = "duplicate placement lowering interpreter")]
    fn duplicate_interpreter_registration_panics() {
        let mut registry = PlacementLoweringRegistry::default();
        registry.register(PlacementKind::Hazard, noop_lowering);
        registry.register(PlacementKind::Hazard, noop_lowering);
    }

    #[test]
    #[should_panic(expected = "unknown placement kind Hazard")]
    fn missing_interpreter_panics_with_kind() {
        PlacementLoweringRegistry::default().interpreter_for(&sample_record("haz_1"), "test_room");
    }
}
