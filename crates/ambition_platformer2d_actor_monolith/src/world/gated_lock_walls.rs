//! Authored flag-gated lock walls.
//!
//! Each `LockWall` may author `gated_by`; this system evaluates
//! `world.flag_set(<gated_by>)` through the shared [`ConditionCatalog`] rather than
//! reading save data directly. The current authored field intentionally names only
//! a flag even though the condition mechanism is extensible. Evaluation is exclusive
//! because condition callbacks receive `&World`; if this grows to many independent
//! rules, evaluate conditions once and publish outcomes for parallel readers.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::authored_logic::{
    ConditionCatalog, ConditionId, PreparedCondition,
};

/// The block-name prefix a gated wall contributes under.
pub const GATED_LOCK_BLOCK_PREFIX: &str = "gated_lock:";

/// One authored wall that is waiting on something.
#[derive(Clone, Debug, PartialEq)]
pub struct GatedLockWall {
    pub id: String,
    pub gated_by: String,
    pub min: ambition_platformer2d_core::Vec2,
    pub size: ambition_platformer2d_core::Vec2,
}

/// Every `LockWall` in `room` that authors a `gated_by`.
///
/// pure, and takes the ROOM rather than the world, so the selection policy
/// stays testable without an ECS. That separation is inherited from the content
/// system this replaces and was the good part of it.
pub fn authored_gated_lock_walls(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
) -> Vec<GatedLockWall> {
    room.lock_walls
        .iter()
        .filter_map(|wall| {
            // a `LockWall` with no `gated_by` is not this system's business —
            // it is either an encounter's (whose phase drives it) or inert.
            // Skipping silently is correct: the field is optional precisely so
            // the other consumers keep working.
            let gated_by = wall.gated_by.as_ref()?;
            let id = wall.id.trim();
            if id.is_empty() {
                return None;
            }
            Some(GatedLockWall {
                id: id.to_string(),
                gated_by: gated_by.clone(),
                min: ambition_platformer2d_core::Vec2::new(wall.min.x, wall.min.y),
                size: ambition_platformer2d_core::Vec2::new(wall.size.x, wall.size.y),
            })
        })
        .collect()
}

/// One cached wall and the question it asks.
///
/// ⭐⭐ THE QUESTION IS PREPARED ONCE, WITH THE WALL. `PreparedCondition` has no
/// public constructor, so holding one is a structural claim that
/// `world.flag_set` exists and takes exactly this argument — made when the room
/// is cached rather than re-spelled and re-minted on every frame this wall is
/// on screen.
///
/// ⛔ `None` MEANS THE QUESTION COULD NOT BE PREPARED, and it is retried. A
/// provider can register after the first room is cached, so a permanent `None`
/// would be a wall that stands forever because of startup ORDER. The retry costs
/// one preparation per frame per unpreparable wall, which is the population that
/// is supposed to be empty.
#[derive(Clone, Debug)]
struct CachedWall {
    wall: GatedLockWall,
    question: Option<PreparedCondition>,
}

/// Per-frame cache — see the module header on why its three inputs are three.
#[derive(Resource, Default)]
pub struct GatedLockWallCache {
    room: Option<String>,
    walls: Vec<CachedWall>,
}

/// Contribute a solid for every authored gated wall whose condition is not yet
/// satisfied.
///
/// `is_satisfied()` and not "not unsatisfied": an unanswerable condition
/// leaves the wall STANDING, which is the safe direction. A gate that opened
/// because nobody could answer its question would open in exactly the situations
/// where the world is least well understood.
pub fn sync_authored_gated_lock_walls(world: &mut World) {
    // the room set is a COMPONENT on the session root, not a resource — the `SessionWorldRef` a
    // normal system takes is a `Single<Ref<T>, With<SessionRoot>>`. An exclusive system has to
    // ask for it the long way. This function already held the room set to find the active room;
    // it just also asked LDtk what was in it.
    let (active_room_id, walls, rooms_changed) = {
        let mut rooms = world.query_filtered::<
            bevy::prelude::Ref<ambition_platformer2d_world::rooms::RoomSet>,
            bevy::prelude::With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
        >();
        let Some(set) = rooms.iter(world).next() else {
            return;
        };
        let spec = set.active_spec();
        (
            spec.id.clone(),
            authored_gated_lock_walls(spec),
            set.is_changed(),
        )
    };
    if world.get_resource::<ConditionCatalog>().is_none() {
        return;
    }

    // ── refresh the cache if any of its inputs moved ─────────────────────────
    //
    // The walls come off the room set now, so THAT is what has to be watched: a hot reload that
    // rebuilds rooms under an unchanged room ID would otherwise leave a stale cache, which is
    // the exact case the old signal existed for. This caches the walls that EXIST, and whether
    // each stands is asked fresh every frame through its condition. So the cached value is a
    // pure function of (project, room) — which is also why it can be declared derived to
    // rollback rather than registered: neither input can change inside a rollback window, since
    // a room transition commits only on a confirmed frame.
    let stale = {
        let cache = world.get_resource::<GatedLockWallCache>();
        cache.is_none_or(|cache| cache.room.as_deref() != Some(active_room_id.as_str()))
    };
    let flag_set = ConditionId::new("world", "flag_set");
    if rooms_changed || stale {
        let catalog = world.resource::<ConditionCatalog>().clone();
        let prepared: Vec<CachedWall> = walls
            .into_iter()
            .map(|wall| CachedWall {
                question: prepare_question(&catalog, &flag_set, &wall),
                wall,
            })
            .collect();
        let mut cache = world.get_resource_or_insert_with(GatedLockWallCache::default);
        cache.walls = prepared;
        cache.room = Some(active_room_id.clone());
    }

    // ── ask, then contribute ─────────────────────────────────────────────────
    //
    // The catalog and the cache are cloned out because evaluating needs `&World`
    // and pushing needs `&mut` on the overlay; a borrow that spanned both would
    // not compile, and cloning a handful of rows once a frame is not the cost
    // worth contorting the code to avoid.
    let catalog = world.resource::<ConditionCatalog>().clone();
    let walls = world
        .get_resource::<GatedLockWallCache>()
        .map(|cache| cache.walls.clone())
        .unwrap_or_default();
    let standing: Vec<&GatedLockWall> = walls
        .iter()
        .filter(|cached| {
            // ⛔ AN UNPREPARABLE QUESTION LEAVES THE WALL STANDING, the same
            // direction an unanswerable one does, and for the same reason: a gate
            // that opened because nobody could ask its question would open in
            // exactly the situations where the world is least well understood.
            // Retried here so a provider registering after the first room is
            // cached is not a wall that stands forever.
            let Some(question) = cached
                .question
                .clone()
                .or_else(|| prepare_question(&catalog, &flag_set, &cached.wall))
            else {
                return true;
            };
            !catalog.ask(world, &question).is_satisfied()
        })
        .map(|cached| &cached.wall)
        .collect();
    if standing.is_empty() {
        return;
    }
    let Some(mut overlay) = world.get_resource_mut::<
        ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay,
    >() else {
        return;
    };
    for wall in standing {
        overlay
            .gate_solids
            .push(ambition_platformer2d_core::Block::solid(
                format!("{GATED_LOCK_BLOCK_PREFIX}{}", wall.id),
                wall.min,
                wall.size,
            ));
    }
}

/// Prepare one wall's `world.flag_set(<gated_by>)`, or `None` when the catalog
/// cannot yet answer for it.
///
/// ⚠ SILENT ON FAILURE, because the caller's answer to `None` is already the safe
/// one and a per-frame retry would make a warning here a per-frame warning. The
/// visible symptom of a permanently unpreparable wall is a wall that never opens,
/// which is the same symptom the unanswerable path has always had.
fn prepare_question(
    catalog: &ConditionCatalog,
    flag_set: &ConditionId,
    wall: &GatedLockWall,
) -> Option<PreparedCondition> {
    catalog
        .prepare(flag_set.clone(), &[wall.gated_by.as_str()])
        .ok()
}

#[cfg(test)]
mod tests;
