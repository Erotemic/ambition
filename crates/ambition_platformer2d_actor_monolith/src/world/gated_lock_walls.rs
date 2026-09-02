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
/// The room-set query, built once and reused.
///
/// ⭐ `World::query_filtered` BUILDS A FRESH `QueryState` on every call, which
/// re-matches every archetype in the world. In an exclusive system that runs
/// each frame, that is a per-frame archetype scan to read a single component
/// off one entity. `Local` is an `ExclusiveSystemParam`, so the state can just
/// live across frames; `QueryState::iter` updates archetypes itself, so a
/// cached state still sees entities that appeared since the last run.
type RoomSetQuery = bevy::ecs::query::QueryState<
    bevy::prelude::Ref<'static, ambition_platformer2d_world::rooms::RoomSet>,
    bevy::prelude::With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
>;

pub fn sync_authored_gated_lock_walls(
    world: &mut World,
    mut rooms: bevy::prelude::Local<Option<RoomSetQuery>>,
) {
    // the room set is a COMPONENT on the session root, not a resource — the `SessionWorldRef` a
    // normal system takes is a `Single<Ref<T>, With<SessionRoot>>`. An exclusive system has to
    // ask for it the long way.
    let rooms = rooms.get_or_insert_with(|| RoomSetQuery::new(world));

    // ⭐ THE ROOM'S WALLS ARE READ ONLY WHEN THE CACHE IS ACTUALLY BEING
    // REFRESHED. `authored_gated_lock_walls` allocates a `Vec<GatedLockWall>`
    // with a `String` and a condition id cloned per wall; computing it every
    // frame and then discarding it unless `rooms_changed || stale` defeated the
    // cache it feeds. Room identity and the change tick are cheap, so decide
    // first and pay for the walls second.
    let (active_room_id, rooms_changed) = {
        let Some(set) = rooms.iter(world).next() else {
            return;
        };
        (set.active_spec().id.clone(), set.is_changed())
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
    // ⛔ THE CATALOG IS AN INPUT TO THE CACHE, so a catalog that moved makes it
    // stale exactly as a changed room does. This is what the per-frame retry
    // below used to stand in for: a provider that publishes its condition AFTER
    // the first room was cached left every wall keyed on it unpreparable
    // forever, and re-preparing on every tick was the workaround. Rebuild once
    // on the edge instead — the question is asked at the same moment either way,
    // and the per-tick preparation road goes.
    let catalog_moved = world.is_resource_changed::<ConditionCatalog>();
    let stale = {
        let cache = world.get_resource::<GatedLockWallCache>();
        cache.is_none_or(|cache| cache.room.as_deref() != Some(active_room_id.as_str()))
    };
    let flag_set = ConditionId::new("world", "flag_set");
    if rooms_changed || stale || catalog_moved {
        let walls = {
            let Some(set) = rooms.iter(world).next() else {
                return;
            };
            authored_gated_lock_walls(set.active_spec())
        };
        let catalog = world.resource::<ConditionCatalog>().clone();
        let prepared: Vec<CachedWall> = walls
            .into_iter()
            .map(|wall| CachedWall {
                question: prepare_question(&active_room_id, &catalog, &flag_set, &wall),
                wall,
            })
            .collect();
        let mut cache = world.get_resource_or_insert_with(GatedLockWallCache::default);
        cache.walls = prepared;
        cache.room = Some(active_room_id.clone());
    }

    // ⛔ A ROOM WITH NO GATED WALLS IS THE COMMON CASE, and everything below it
    // — two catalog clones, a cache clone, an overlay lookup — is work whose
    // only possible result is an empty `standing`. Leave before paying for it.
    if world
        .get_resource::<GatedLockWallCache>()
        .is_none_or(|cache| cache.walls.is_empty())
    {
        return;
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
            //
            // ⭐ NO RE-PREPARATION HERE. The question was prepared when the cache
            // was built, and the cache rebuilds when the catalog moves, so the
            // late-provider case the old per-frame retry existed for is handled
            // on that edge instead of by parsing on every tick for every wall.
            let Some(question) = cached.question.clone() else {
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
/// Prepare one wall's question, REPORTING the authored fault when it cannot be.
///
/// ⛔⛔ THE `.ok()` THIS REPLACES WAS A SILENT SOFT-LOCK. `prepare` returns a
/// `PreparationError` carrying the authored source and the reason — its own doc
/// says it keeps the source because *"a diagnostic an author cannot act on"* is
/// useless — and this threw all of it away. A misspelt `gated_by` therefore
/// produced a wall that stands FOREVER, in a room the player cannot finish, with
/// nothing written anywhere. That is the worst shape a failure can take here:
/// the level is wrong, the engine knows exactly why, and nobody is told.
///
/// The wall still stands (an unanswerable gate must not open — see the caller),
/// so this changes no behaviour. It changes whether anyone can find out.
fn prepare_question(
    room: &str,
    catalog: &ConditionCatalog,
    flag_set: &ConditionId,
    wall: &GatedLockWall,
) -> Option<PreparedCondition> {
    match catalog.prepare(flag_set.clone(), &[wall.gated_by.as_str()]) {
        Ok(prepared) => Some(prepared),
        Err(error) => {
            // Room, wall and authored text: the three facts an author needs to
            // find the row in the level. `reason` is the substrate's own words
            // ("takes 2 arguments, got 1", an unknown id) and is quoted rather
            // than re-worded.
            bevy::log::error!(
                target: "ambition_platformer2d::gated_lock_walls",
                "room `{room}` wall `{}` is gated by `{}`, which cannot be prepared: {} \
                 (authored source `{}`). The wall STANDS until this is fixed — a gate \
                 whose question cannot be asked must not open.",
                wall.id,
                wall.gated_by,
                error.reason(),
                error.source(),
            );
            None
        }
    }
}

#[cfg(test)]
mod tests;
