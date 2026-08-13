//! Per-actor locomotion state ([`ActorSpawnState`], [`ActorSurfaceState`]) and
//! the per-frame physics/AI tick (the `integration` submodule) — every actor,
//! grounded, aerial, and the adhesive crawler, integrates through the one
//! shared movement kernel (`ae::step_motion`).
//!
//! ⛔⛔ **THE ENEMY-ARCHETYPE ONTOLOGY WAS HERE AND IS DELETED** (AC6,
//! 2026-08-13): `CharacterRoster` and its fragments/registry/assembly errors,
//! the `OpenCastingDecision` waiver, `GENERIC_BODY_ROW`, the `ArchetypeSpecExt`
//! projections into this crate's runtime shapes, and the fixture rosters — about
//! 1,800 lines, plus `ArchetypeSpec` itself in `ambition_combat` and the
//! `character_archetypes.ron` it parsed.
//!
//! ⇒ **a body is what its CHARACTER says it is.** The table it replaced answered
//! by BRAIN KEY and could not fail: an identifier naming no row resolved to a
//! reserved `combatant` body, so a misspelling, a renamed creature or a deleted
//! row produced a complete and plausible wrong body. Three shipped things were
//! silently downgraded that way while every test stayed green (a boss's minions,
//! a goblin fight's heavies, an under-town skitter). Construction refuses an
//! identifier that names no character now.

use super::*;

mod integration;
pub use integration::ContactAttack;

/// The authored spawn baseline an actor reverts to on a same-room reset
/// (`reset_to_spawn`): position and body size. No entity morphs its
/// archetype in place — a composite (PirateOnShark) is spawned as two
/// SEPARATE standalone entities (`spawn_mounts`) and dismount swaps the
/// rider's brain/action-set, never its archetype — so there is nothing
/// to record here but the spatial baseline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorSpawnState {
    /// World position the actor spawned at.
    pub pos: ae::Vec2,
    /// Authored body size.
    pub size: ae::Vec2,
}

/// An actor's surface-cling state for the glued surface-walker crawl.
///
/// Ground contact (`on_ground`) and air-jump budget now live on the shared
/// movement clusters — [`crate::actor::BodyGroundState::on_ground`] and
/// [`crate::actor::BodyJumpState::air_jumps_available`] — the SAME components the
/// player carries, so there is one ground/jump authority for every body (the
/// grounded/aerial pipeline writes them directly; the surface-walker crawl writes
/// `ground.on_ground` too). This component keeps only the surface-walker's cling
/// geometry, which the shared clusters don't model.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorSurfaceState {
    /// Outward-pointing unit normal of the surface the actor is
    /// currently clinging to. Used by surface-walking archetypes
    /// (`PuppySlug`) to crawl floors, walls, and ceilings; every other
    /// archetype pins this at `(0, -1)` (floor) and ignores it. Engine
    /// y grows downward, so floor → (0, -1), right wall → (-1, 0),
    /// ceiling → (0, 1), left wall → (1, 0).
    pub surface_normal: ae::Vec2,
    /// 0.0 = ignores gravity (flying); 1.0 = full gravity.
    pub gravity_scale: f32,
}

// `RespawnPolicy` moved to the combat kit (generic death/respawn
// vocabulary); re-exported so `crate::features::RespawnPolicy`
// paths keep working.
pub use ambition_entity_catalog::placements::RespawnPolicy;

/// Flag-id suffix used by `_dead_until_rest` flags. Constant so the
/// kill hook, save sync, and `clear_dead_until_rest_flags` all
/// agree on the spelling.
pub const ENEMY_DEAD_UNTIL_REST_SUFFIX: &str = "_dead_until_rest";
