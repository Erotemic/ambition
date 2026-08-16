//! **An authored `LockWall` decides for itself what opens it.**
//!
//! # What this replaces, and why the replacement is the point
//!
//! This was `sync_intro_flag_gated_lock_walls` in `ambition_content`, and the
//! fact that gates it read from a Rust const table:
//!
//! ```ignore
//! pub const INTRO_FLAG_GATED_LOCK_WALLS: &[(&str, &str)] = &[
//!     ("alice_private_return_lock", "bob_field_survey_received"),
//!     ("gate_alice_private_lock", "bob_field_survey_received"),
//! ];
//! ```
//!
//! ⭐⭐ **the wall was in the level and the reason it opened was in the
//! compiler.** An author adding a gated wall had to edit Rust, in another crate,
//! in a table whose two halves are matched by string; an agent reading the level
//! could see a `LockWall` and no way to find out what it was waiting for. The
//! table is gone and the answer lives on the entity, as a `gated_by` field.
//!
//! ⭐ **and the capability generalised on the way out.** This was Ambition intro
//! content; it is now an engine system, so Mary-O, Sanic and anything else built
//! on this engine author a flag-gated wall the same way, with no Rust at all.
//!
//! # ⚠ The condition is asked, not read
//!
//! The obvious shortcut is to read the save flag here. It would be shorter and
//! it would put the engine's collision layer in the business of knowing what a
//! save flag is. Instead the wall's gate is a **condition** —
//! `world.flag_set(<gated_by>)` — asked through
//! [`ConditionCatalog`](ambition_platformer2d_shared_tangle::authored_logic::ConditionCatalog),
//! so the day a wall wants to be gated on something else (an item held, a
//! mechanism powered, an encounter cleared) the answer is a different condition
//! id and not a different system.
//!
//! ⚠ **`gated_by` names a FLAG rather than a whole condition, and that is a
//! deliberate narrowing of the authored surface.** The mechanism is general; the
//! spelling is not, because no customer yet needs a wall gated on anything else
//! and an authored surface is much harder to take back than to widen.
//!
//! # ⛔ Why this system is EXCLUSIVE, and what to do when that stops being fine
//!
//! Evaluating a condition needs `&World` — a domain answers by looking at
//! whatever state it owns, and the catalog cannot know in advance which. A
//! system that took `&World` could not also take `ResMut` on the overlay it
//! writes, so this takes `&mut World` and does both.
//!
//! ⇒ **that is one schedule sync point in `WorldPrep`, and it is the price of
//! one rule.** ⭐ the shape to reach for when there are MANY rules is different
//! and is worth writing down now: **evaluate every live condition once in a
//! single exclusive pass, publish the outcomes into a resource, and let ordinary
//! parallel systems read them.** ⛔ do not instead give each rule its own
//! exclusive system — that is the version of this that gets slow without anybody
//! noticing which change did it.
//!
//! # ⚠ The cache is inherited, not invented
//!
//! Walking every level's entities per frame was measured as the single hottest
//! our-code symbol in the headless boss profile (~1.8%). The cache and its three
//! invalidation inputs — the save, the active room, and **the project itself** —
//! come across unchanged. ⛔ the project input is the one that was missing
//! originally: a hot reload that swapped the LDtk project under an unchanged room
//! id kept serving walls computed from the replaced project.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::authored_logic::{
    ConditionArg, ConditionCatalog, ConditionId,
};

/// **The block-name prefix a gated wall contributes under.**
///
/// ⭐ renamed from `intro_lock:` when this capability left Ambition's intro
/// content and became an engine one. The render layer matches on this prefix to
/// draw a `LockWallVisual`, so the name is a small cross-layer contract — and
/// `intro_` was a lie about a system every game can now use.
pub const GATED_LOCK_BLOCK_PREFIX: &str = "gated_lock:";

/// One authored wall that is waiting on something.
#[derive(Clone, Debug, PartialEq)]
pub struct GatedLockWall {
    pub id: String,
    pub gated_by: String,
    pub min: ambition_platformer2d_core::Vec2,
    pub size: ambition_platformer2d_core::Vec2,
}

/// **Every `LockWall` in `active_room_id` that authors a `gated_by`.**
///
/// ⭐ **pure, and takes the project rather than the world**, so the LDtk-walking
/// policy stays testable without an ECS. That separation is inherited from the
/// content system this replaces and was the good part of it.
///
/// ⚠ a `LockWall` with no `gated_by` is not this system's business — it is
/// either an encounter's (whose phase drives it) or inert. Skipping silently is
/// correct: the field is optional precisely so the other consumers keep working.
pub fn authored_gated_lock_walls(
    project: &ambition_platformer2d_ldtk::LdtkProject,
    active_room_id: &str,
) -> Vec<GatedLockWall> {
    let mut out = Vec::new();
    for level in &project.levels {
        if level.active_area() != active_room_id {
            continue;
        }
        for entity in level.all_entity_instances() {
            if entity.identifier != "LockWall" {
                continue;
            }
            let Some(id) = ambition_platformer2d_ldtk::field_string(entity, "id") else {
                continue;
            };
            let Some(gated_by) = ambition_platformer2d_ldtk::field_string(entity, "gated_by")
            else {
                continue;
            };
            let gated_by = gated_by.trim();
            if gated_by.is_empty() {
                continue;
            }
            out.push(GatedLockWall {
                id: id.trim().to_string(),
                gated_by: gated_by.to_string(),
                min: ambition_platformer2d_core::Vec2::new(
                    entity.px[0] as f32,
                    entity.px[1] as f32,
                ),
                size: ambition_platformer2d_core::Vec2::new(
                    entity.width as f32,
                    entity.height as f32,
                ),
            });
        }
    }
    out
}

/// Per-frame cache — see the module header on why its three inputs are three.
#[derive(Resource, Default)]
pub struct GatedLockWallCache {
    room: Option<String>,
    walls: Vec<GatedLockWall>,
}

/// Contribute a solid for every authored gated wall whose condition is not yet
/// satisfied.
///
/// ⚠ **`is_satisfied()` and not "not unsatisfied"**: an unanswerable condition
/// leaves the wall STANDING, which is the safe direction. A gate that opened
/// because nobody could answer its question would open in exactly the situations
/// where the world is least well understood.
pub fn sync_authored_gated_lock_walls(world: &mut World) {
    // ⚠ the room set is a COMPONENT on the session root, not a resource — the
    // `SessionWorldRef` a normal system takes is a `Single<Ref<T>, With<SessionRoot>>`.
    // An exclusive system has to ask for it the long way.
    let active_room_id = {
        let mut rooms = world.query_filtered::<
            &crate::rooms::RoomSet,
            bevy::prelude::With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
        >();
        let Some(set) = rooms.iter(world).next() else {
            return;
        };
        set.active_spec().id.clone()
    };
    if world.get_resource::<ConditionCatalog>().is_none() {
        return;
    }

    // ── refresh the cache if any of its three inputs moved ───────────────────
    let project_changed = world
        .get_resource_ref::<ambition_platformer2d_ldtk::ActiveLdtkProject>()
        .is_some_and(|project| project.is_changed());
    // ⭐⭐ **THE SAVE IS NO LONGER AN INPUT, AND DROPPING IT IS WHAT THE
    // MIGRATION BOUGHT.** The old content system cached the walls that should be
    // SOLID, so a flag flip changed the cached set and the cache had to watch the
    // save. This caches the walls that EXIST, and whether each stands is asked
    // fresh every frame through its condition. So the cached value is a pure
    // function of (project, room) — which is also why it can be declared derived
    // to rollback rather than registered: neither input can change inside a
    // rollback window, since a room transition commits only on a confirmed frame.
    let stale = {
        let cache = world.get_resource::<GatedLockWallCache>();
        cache.is_none_or(|cache| cache.room.as_deref() != Some(active_room_id.as_str()))
    };
    if project_changed || stale {
        let walls = world
            .get_resource::<ambition_platformer2d_ldtk::ActiveLdtkProject>()
            .map(|project| authored_gated_lock_walls(&project.0, &active_room_id))
            .unwrap_or_default();
        let mut cache = world.get_resource_or_insert_with(GatedLockWallCache::default);
        cache.walls = walls;
        cache.room = Some(active_room_id.clone());
    }

    // ── ask, then contribute ─────────────────────────────────────────────────
    //
    // The catalog and the cache are cloned out because evaluating needs `&World`
    // and pushing needs `&mut` on the overlay; a borrow that spanned both would
    // not compile, and cloning a handful of rows once a frame is not the cost
    // worth contorting the code to avoid.
    let flag_set = ConditionId::new("world", "flag_set");
    let catalog = world.resource::<ConditionCatalog>().clone();
    let walls = world
        .get_resource::<GatedLockWallCache>()
        .map(|cache| cache.walls.clone())
        .unwrap_or_default();
    let standing: Vec<&GatedLockWall> = walls
        .iter()
        .filter(|wall| {
            !catalog
                .evaluate(
                    world,
                    &flag_set,
                    &[ConditionArg::Name(wall.gated_by.clone())],
                )
                .is_satisfied()
        })
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

#[cfg(test)]
mod tests;
