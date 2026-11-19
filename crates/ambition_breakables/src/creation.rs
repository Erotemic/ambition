//! Checked runtime batches use the same placement recipe as authored destructibles.

use ambition_entity_catalog::placements::{BreakableSpec, HazardRespawn, PlacementSchema};
use ambition_platformer2d_construction::{
    runtime::{RuntimeConstruction, RuntimeInitialization}, ActorConstructionParams, ActorConstructionRequest,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::{lifecycle::*, sim_id::SimId};
use ambition_platformer2d_world::placements::PlacementRecord;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub use ambition_platformer2d_world::creation::BreakableCreationLimits;

pub struct RuntimeBreakable<Initialization = ()> {
    pub key: String,
    pub label: String,
    pub aabb: ae::Aabb,
    pub spec: BreakableSpec,
    pub initialization: Initialization,
}
pub struct CreatedBreakable {
    pub key: String,
    pub id: SimId,
    pub entity: Entity,
}

/// Build the complete batch or refuse without births or counter changes.
/// These fresh objects belong to the caller's room, independent of its creator.
pub fn create_breakables<Initialization: RuntimeInitialization>(
    world: &mut World,
    caller: Entity,
    objects: &[RuntimeBreakable<Initialization>],
    limits: BreakableCreationLimits,
) -> Result<Vec<CreatedBreakable>, String> {
    use ae::AabbExt as _;
    if objects.is_empty() || objects.len() > limits.max_breakables as usize {
        return Err("The breakable count exceeds the room's batch bound.".into());
    }
    let context = RuntimeConstruction::prepare(world, caller, objects.len())?;
    let mut keys = BTreeSet::new();
    for object in objects {
        let spec = &object.spec;
        if object.key.is_empty() || object.key.len() > 128 || !keys.insert(&object.key) {
            return Err("Breakable keys must be nonempty, unique and at most 128 bytes.".into());
        }
        if object.label.len() > limits.max_label_bytes as usize
            || spec
                .look
                .as_ref()
                .is_some_and(|id| id.is_empty() || id.len() > 128)
        {
            return Err("A breakable label or look exceeds its bounds.".into());
        }
        let center = object.aabb.center();
        let half = object.aabb.half_size();
        if !center.x.is_finite()
            || !center.y.is_finite()
            || !half.x.is_finite()
            || !half.y.is_finite()
            || half.x <= 0.0
            || half.y <= 0.0
        {
            return Err("Breakable boxes must be finite and have positive size.".into());
        }
        if spec.health_max < 1
            || spec.health_current != spec.health_max
            || spec.state != ambition_entity_catalog::placements::BreakableStateSpec::Intact
        {
            return Err("A fresh breakable must be intact at full positive health.".into());
        }
        match spec.respawn {
            HazardRespawn::Never => {}
            HazardRespawn::AfterSeconds(seconds) if seconds.is_finite() && seconds > 0.0 => {}
            _ => return Err(
                "Runtime breakables use room lifetime, with an optional positive respawn delay."
                    .into(),
            ),
        }
    }
    let mut population = world.query::<(
        &SessionScopedEntity,
        &InRoomInstance,
        &crate::BreakableFeature,
    )>();
    let count = population
        .iter(world)
        .filter(|(owner, at, _)| owner.0 == context.session() && at.0 == context.room())
        .count();
    if count.saturating_add(objects.len()) > limits.max_room_breakables as usize {
        return Err("The room's breakable population bound is exhausted.".into());
    }
    let mut rows = Vec::with_capacity(objects.len());
    let lowering = world
        .get_resource::<ambition_platformer2d_construction::placements::PlacementLoweringRegistry>()
        .ok_or("The breakable placement capability is not installed.")?;
    for (index, object) in objects.iter().enumerate() {
        let id = context.id(index);
        let mut record = PlacementRecord::new(
            id.as_str(),
            PlacementSchema::Breakable(object.spec.clone()),
            object.aabb,
        );
        record.name = object.label.clone();
        let lower = lowering
            .try_interpreter_for(&record, "runtime")
            .map_err(|error| error.to_string())?;
        rows.push(ActorConstructionRequest {
            sim_id: id,
            origin: context.origin(index),
            parameters: ActorConstructionParams::Placement {
                record,
                paths: Vec::new(),
                lower,
            },
            relations: Vec::new(),
        });
    }
    let ids: Vec<_> = rows.iter().map(|row| row.sim_id.clone()).collect();
    let states: BTreeMap<_, _> = ids.iter().cloned().zip(objects.iter().map(|object| &object.initialization)).collect();
    let receipt = context.commit(world, rows, &states, |_, _| {})?;
    Ok(objects
        .iter()
        .zip(ids)
        .map(|(object, id)| {
            let entity = receipt
                .entity(&id)
                .expect("the complete batch committed every root");
            CreatedBreakable {
                key: object.key.clone(),
                id,
                entity,
            }
        })
        .collect())
}
