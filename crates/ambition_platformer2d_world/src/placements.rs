//! Authored placement records on the room IR.
//!
//! A record joins its spatial footprint (`id` + `aabb`) to the closed authored
//! [`ambition_entity_catalog::placements::PlacementSchema`]. Backend converters
//! parse authored entities into records; lowering maps each record to live
//! entities at room load.

use ambition_platformer2d_core as ae;
use ambition_entity_catalog::placements::{PlacementKind, PlacementSchema};
use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;
use bevy_app::App;
use bevy_ecs::prelude::{Commands, Resource};
use std::collections::HashMap;
use std::fmt;

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
pub struct LoweringCtx<'w, 's, 'a, C: ?Sized = ()> {
    pub commands: &'a mut Commands<'w, 's>,
    pub room_id: &'a str,
    pub paths: &'a [(String, ae::KinematicPath)],
    /// Gameplay-session ownership captured when room staging was requested.
    pub session_scope: SessionSpawnScope,
    /// The entity this placement POPULATES. Allocated by the caller — the construction executor
    /// for planned rows — so identity, provenance, and transaction ownership are stamped on the
    /// same body the interpreter builds.
    pub root: bevy_ecs::entity::Entity,
    /// Runtime context supplied by the simulation layer. The world IR remains
    /// generic and content-free; callers choose the context type needed by
    /// their lowering interpreters.
    pub context: &'a C,
}

pub type LoweringFn<C = ()> = for<'w, 's, 'a> fn(&PlacementRecord, &mut LoweringCtx<'w, 's, 'a, C>);

/// A placement interpreter resolved during mutation-free room preparation.
///
/// Construction stores the exact function pointer beside an owned copy of the
/// authored record, so commit does not repeat registry lookup and cannot discover
/// a missing interpreter after the outgoing room has begun to retire.
#[derive(Clone)]
struct PlannedPlacement<C: Send + Sync + 'static> {
    record: PlacementRecord,
    lower: LoweringFn<C>,
}

/// Mutation-free, deterministic lowering plan for one room's authored placements.
///
/// This is deliberately narrower than a prefab graph: it freezes the existing
/// single lowering authority into an inspectable artifact that normal activation,
/// transitions, reset, hot reload, and restore can execute identically.
#[derive(Clone)]
pub struct PlacementLoweringPlan<C: Send + Sync + 'static = ()> {
    room_id: String,
    paths: Vec<(String, ae::KinematicPath)>,
    placements: Vec<PlannedPlacement<C>>,
}

impl<C: Send + Sync + 'static> PlacementLoweringPlan<C> {
    pub fn room_id(&self) -> &str {
        &self.room_id
    }

    pub fn len(&self) -> usize {
        self.placements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.placements.is_empty()
    }

    /// Execute only the decisions frozen by [`PlacementLoweringRegistry::plan_room`].
    pub fn lower_all<'w, 's>(
        &self,
        commands: &mut Commands<'w, 's>,
        session_scope: SessionSpawnScope,
        context: &C,
    ) {
        for planned in &self.placements {
            let root = commands.spawn_empty().id();
            let mut ctx = LoweringCtx {
                commands,
                room_id: &self.room_id,
                paths: &self.paths,
                session_scope,
                root,
                context,
            };
            (planned.lower)(&planned.record, &mut ctx);
        }
    }

    /// The frozen (record, interpreter) decisions this plan holds, in authored
    /// order — the construction planner reads these to turn each placement
    /// into a plan row carrying its already-resolved interpreter.
    pub fn planned(&self) -> impl Iterator<Item = (&PlacementRecord, LoweringFn<C>)> {
        self.placements
            .iter()
            .map(|planned| (&planned.record, planned.lower))
    }
}

/// Mutation-free placement-lowering preflight failure.
///
/// Room-transition preparation uses this error before touching the live room so an incomplete
/// target never tears down the source room first.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacementLoweringError {
    pub room_id: String,
    pub placement_id: String,
    pub kind: PlacementKind,
    pub registered_kinds: Vec<PlacementKind>,
}

impl fmt::Display for PlacementLoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown placement kind {:?} for placement '{}' in room '{}'; registered kinds: {:?}",
            self.kind, self.placement_id, self.room_id, self.registered_kinds,
        )
    }
}

impl std::error::Error for PlacementLoweringError {}

/// Registry from authored placement kind to the simulation/content interpreter
/// that lowers the record into live room-scoped entities.
#[derive(Resource, Clone)]
pub struct PlacementLoweringRegistry<C: Send + Sync + 'static = ()> {
    interpreters: HashMap<PlacementKind, PlacementLoweringEntry<C>>,
}

#[derive(Clone)]
struct PlacementLoweringEntry<C: Send + Sync + 'static> {
    meta: ambition_registry_core::RegistrationMeta,
    lower: LoweringFn<C>,
}

/// ⛔⛔ HAND-WRITTEN, AND THE FUNCTION ADDRESS IS PART OF IT. `derive(PartialEq)`
/// would compare the fn pointer too — but writing it out is the point: this is
/// the ONE registry in the workspace whose identity includes a function, and
/// `ambition_registry_core::classify` decides New/Idempotent/Conflict from
/// exactly this comparison. Re-registering one kind with the same
/// owner/source/schema but a DIFFERENT lowering function is a CONFLICT, and it
/// has to stay one: two interpreters under one `PlacementKind` is the ambiguity
/// this registry exists to refuse.
impl<C: Send + Sync + 'static> PartialEq for PlacementLoweringEntry<C> {
    fn eq(&self, other: &Self) -> bool {
        self.meta == other.meta && std::ptr::fn_addr_eq(self.lower, other.lower)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementLoweringRegistrationError {
    EmptyIdentity {
        field: &'static str,
    },
    Conflict {
        kind: PlacementKind,
        existing_owner: String,
        existing_source: String,
        existing_schema: String,
        candidate_owner: String,
        candidate_source: String,
        candidate_schema: String,
    },
}

impl std::fmt::Display for PlacementLoweringRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyIdentity { field } => write!(f, "placement lowering {field} must not be empty"),
            Self::Conflict { kind, existing_owner, existing_source, existing_schema, candidate_owner, candidate_source, candidate_schema } => write!(f,
                "conflicting placement lowering for '{}': existing {existing_owner}/{existing_source} schema '{existing_schema}', candidate {candidate_owner}/{candidate_source} schema '{candidate_schema}'",
                kind.stable_id()),
        }
    }
}
impl std::error::Error for PlacementLoweringRegistrationError {}

impl<C: Send + Sync + 'static> Default for PlacementLoweringRegistry<C> {
    fn default() -> Self {
        Self {
            interpreters: HashMap::new(),
        }
    }
}

impl<C: Send + Sync + 'static> PlacementLoweringRegistry<C> {
    pub fn try_register(
        &mut self,
        kind: PlacementKind,
        owner: impl Into<String>,
        source: impl Into<String>,
        schema_id: impl Into<String>,
        f: LoweringFn<C>,
    ) -> Result<(), PlacementLoweringRegistrationError> {
        // Identity validation is the core's: same three fields, same order, so
        // the first blank field a content author hears about is unchanged.
        let meta = ambition_registry_core::RegistrationMeta::new(owner, source, schema_id)
            .map_err(|empty| PlacementLoweringRegistrationError::EmptyIdentity {
                field: empty.field,
            })?;
        let incoming = PlacementLoweringEntry { meta, lower: f };
        match ambition_registry_core::classify(self.interpreters.get(&kind), &incoming) {
            ambition_registry_core::Classification::Idempotent => Ok(()),
            ambition_registry_core::Classification::Conflict { existing } => {
                Err(PlacementLoweringRegistrationError::Conflict {
                    kind,
                    existing_owner: existing.meta.owner.clone(),
                    existing_source: existing.meta.source.clone(),
                    existing_schema: existing.meta.schema_id.clone(),
                    candidate_owner: incoming.meta.owner,
                    candidate_source: incoming.meta.source,
                    candidate_schema: incoming.meta.schema_id,
                })
            }
            ambition_registry_core::Classification::New => {
                self.interpreters.insert(kind, incoming);
                Ok(())
            }
        }
    }

    pub fn registered_kinds(&self) -> Vec<PlacementKind> {
        // AMBITION_REVIEW(determinism): the hash-ordered keys are sorted on the very
        // next line, before anything can observe them, and this runs once at room
        // load rather than in the tick. `PlacementKind` has no `Ord`, so a
        // `BTreeMap` would need one invented for a debug-name sort we already do.
        let mut kinds: Vec<_> = self.interpreters.keys().copied().collect();
        kinds.sort_by_key(|kind| kind.stable_id());
        kinds
    }

    /// Stable provider/source/schema rows used by prepared-content assembly.
    pub fn schema_descriptors(&self) -> Vec<(String, String, String, String)> {
        let mut rows = self
            .interpreters
            .iter()
            .map(|(kind, entry)| {
                (
                    kind.stable_id().to_owned(),
                    entry.meta.owner.clone(),
                    entry.meta.source.clone(),
                    entry.meta.schema_id.clone(),
                )
            })
            .collect::<Vec<_>>();
        rows.sort();
        rows
    }

    pub fn deterministic_dump(&self) -> String {
        self.schema_descriptors()
            .into_iter()
            // Byte-identical to the `format!` this replaced — `canonical_row` is
            // `join("\t")` plus a newline — and it adds the assertion that no
            // field smuggles a tab or newline into the grammar.
            .map(|(kind, owner, source, schema)| {
                ambition_registry_core::canonical_row(&[&kind, &owner, &source, &schema])
            })
            .collect()
    }

    /// Resolve every authored placement interpreter and clone the exact spatial
    /// inputs required at commit time, without mutating the ECS world.
    pub fn plan_room(
        &self,
        room_id: &str,
        paths: &[(String, ae::KinematicPath)],
        records: &[PlacementRecord],
    ) -> Result<PlacementLoweringPlan<C>, PlacementLoweringError> {
        let placements = records
            .iter()
            .map(|record| {
                self.try_interpreter_for(record, room_id)
                    .map(|lower| PlannedPlacement {
                        record: record.clone(),
                        lower,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PlacementLoweringPlan {
            room_id: room_id.to_string(),
            paths: paths.to_vec(),
            placements,
        })
    }

    /// Validate that every authored placement in `room_id` has an installed
    /// lowering interpreter, without mutating the ECS world.
    pub fn validate_room(
        &self,
        room_id: &str,
        records: &[PlacementRecord],
    ) -> Result<(), PlacementLoweringError> {
        self.plan_room(room_id, &[], records).map(|_| ())
    }

    fn try_interpreter_for(
        &self,
        record: &PlacementRecord,
        room_id: &str,
    ) -> Result<LoweringFn<C>, PlacementLoweringError> {
        let kind = record.kind();
        self.interpreters
            .get(&kind)
            .map(|entry| entry.lower)
            .ok_or_else(|| PlacementLoweringError {
                room_id: room_id.to_string(),
                placement_id: record.id.as_str().to_string(),
                kind,
                registered_kinds: self.registered_kinds(),
            })
    }

    fn interpreter_for(&self, record: &PlacementRecord, room_id: &str) -> LoweringFn<C> {
        self.try_interpreter_for(record, room_id)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    pub fn lower<'w, 's, 'a>(
        &self,
        record: &PlacementRecord,
        ctx: &mut LoweringCtx<'w, 's, 'a, C>,
    ) {
        let lower = self.interpreter_for(record, ctx.room_id);
        lower(record, ctx);
    }
}

pub trait PlacementLoweringAppExt<C: Send + Sync + 'static> {
    fn register_placement_interpreter(
        &mut self,
        kind: PlacementKind,
        owner: &'static str,
        source: &'static str,
        schema_id: &'static str,
        f: LoweringFn<C>,
    ) -> &mut Self;
}

impl<C: Send + Sync + 'static> PlacementLoweringAppExt<C> for App {
    fn register_placement_interpreter(
        &mut self,
        kind: PlacementKind,
        owner: &'static str,
        source: &'static str,
        schema_id: &'static str,
        f: LoweringFn<C>,
    ) -> &mut Self {
        if !self
            .world()
            .contains_resource::<PlacementLoweringRegistry<C>>()
        {
            self.init_resource::<PlacementLoweringRegistry<C>>();
        }
        self.world_mut()
            .resource_mut::<PlacementLoweringRegistry<C>>()
            .try_register(kind, owner, source, schema_id, f)
            .unwrap_or_else(|error| panic!("{error}"));
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

    fn noop_lowering(_record: &PlacementRecord, _ctx: &mut LoweringCtx<'_, '_, '_, ()>) {}

    #[test]
    fn placement_schema_reports_kind() {
        assert_eq!(sample_record("haz").kind(), PlacementKind::Hazard);
    }

    #[test]
    fn room_validation_reports_missing_interpreter_without_panicking() {
        let err = PlacementLoweringRegistry::<()>::default()
            .validate_room("test_room", &[sample_record("haz_1")])
            .expect_err("missing interpreter should be a preflight error");
        assert_eq!(err.room_id, "test_room");
        assert_eq!(err.placement_id, "haz_1");
        assert_eq!(err.kind, PlacementKind::Hazard);
    }

    #[test]
    fn conflicting_interpreter_registration_is_structured_and_transactional() {
        let mut registry = PlacementLoweringRegistry::<()>::default();
        registry
            .try_register(
                PlacementKind::Hazard,
                "test",
                "fixture",
                "hazard.v1",
                noop_lowering,
            )
            .unwrap();
        let before = registry.deterministic_dump();
        let error = registry
            .try_register(
                PlacementKind::Hazard,
                "other",
                "fixture",
                "hazard.v2",
                noop_lowering,
            )
            .expect_err("conflict must be structured");
        assert!(matches!(
            error,
            PlacementLoweringRegistrationError::Conflict { .. }
        ));
        assert_eq!(registry.deterministic_dump(), before);
    }

    #[test]
    fn identical_interpreter_registration_is_idempotent() {
        let mut registry = PlacementLoweringRegistry::<()>::default();
        registry
            .try_register(
                PlacementKind::Hazard,
                "test",
                "fixture",
                "hazard.v1",
                noop_lowering,
            )
            .unwrap();
        let before = registry.deterministic_dump();
        registry
            .try_register(
                PlacementKind::Hazard,
                "test",
                "fixture",
                "hazard.v1",
                noop_lowering,
            )
            .unwrap();
        assert_eq!(registry.deterministic_dump(), before);
    }

    #[test]
    #[should_panic(expected = "unknown placement kind Hazard")]
    fn missing_interpreter_panics_with_kind() {
        PlacementLoweringRegistry::<()>::default()
            .interpreter_for(&sample_record("haz_1"), "test_room");
    }
}
