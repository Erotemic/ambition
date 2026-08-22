//! Per-actor locomotion state ([`ActorSpawnState`], [`ActorSurfaceState`]) and
//! the per-frame physics/AI tick (the `integration` submodule) — every actor,
//! grounded, aerial, and the adhesive crawler, integrates through the one
//! shared movement kernel (`ae::step_motion`).
//!
//! **THE ENEMY-ARCHETYPE ONTOLOGY WAS HERE AND IS DELETED** (AC6,
//! ): `CharacterRoster` and its fragments/registry/assembly errors,
//! the `OpenCastingDecision` waiver, `GENERIC_BODY_ROW`, the `ArchetypeSpecExt`
//! projections into this crate's runtime shapes, and the fixture rosters — about
//! 1,800 lines, plus `ArchetypeSpec` itself in `ambition_combat` and the
//! `character_archetypes.ron` it parsed.
//!
//! Three shipped things were silently downgraded that way while every test stayed green (a
//! boss's minions, a goblin fight's heavies, an under-town skitter). Construction refuses an
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

// `ActorSurfaceState` moved DOWN to the floor crate beside the body clusters
// its own doc already pointed at (`BodyGroundState`, `BodyJumpState`);
// re-exported so `crate::features::ActorSurfaceState` paths keep working, the
// same shape as `RespawnPolicy` immediately below.
//
// this is what unblocks moving the capture SYSTEMS to `ambition_combat`,
// which owns the capture vocabulary: it was the one type in
// `features/ecs/capture.rs` that a lower crate could not see.
pub use ambition_platformer2d_core::ActorSurfaceState;

// `RespawnPolicy` moved to the combat kit (generic death/respawn
// vocabulary); re-exported so `crate::features::RespawnPolicy`
// paths keep working.
pub use ambition_entity_catalog::placements::RespawnPolicy;

/// Flag-id suffix used by `_dead_until_rest` flags. Constant so the
/// kill hook, save sync, and `clear_dead_until_rest_flags` all
/// agree on the spelling.
pub const ENEMY_DEAD_UNTIL_REST_SUFFIX: &str = "_dead_until_rest";
