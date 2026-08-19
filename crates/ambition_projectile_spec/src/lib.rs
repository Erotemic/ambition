//! **Authored projectile intent — content-free spawn data, and nothing else.**
//!
//! Carved out of `ambition_platformer2d_shared_tangle` (2026-08-02) for one
//! measured reason: `ambition_vfx` imported exactly ONE item from that 16,927-line
//! crate — [`ProjectileSpawn`] — and that single edge was the last thing keeping
//! `ambition_platformer2d_core` in its build closure. The projectile spawn-road
//! migration later removed projectile transport from `ambition_vfx` entirely;
//! this crate remains the lower authored-intent vocabulary consumed by the
//! projectile domain and re-exported by the shared projectile primitive.
//!
//! ## Why this is the first carve that actually decouples anything
//!
//! Four crates had been carved off the core crate before this one, and every one
//! of them still reached core transitively through a SECOND platformer
//! dependency (`cargo tree --edges normal -i` says so). They bought honest
//! manifests, not decoupling. At the time, `ambition_vfx` had two edges — core,
//! dropped when `ambition_geometry` appeared, and `shared_tangle`, dropped here.
//! The later spawn-road migration removed its projectile edge entirely; that is
//! the intended success condition of a carve like this, not a reason to move the
//! vocabulary back upward.
//!
//! ## The remaining physics vocabulary
//!
//! `ProjectileSpec` and `WorldHitPolicy` still live in the shared projectile
//! primitive because their `SnapshotState` implementations are owned there today.
//! When `ProjectileSpawn` was first carved out, moving either sibling would also
//! have pulled the rollback/core edge into every consumer of this crate — including
//! the old VFX projectile transport. This migration removes that VFX dependency, so
//! that historical blocker no longer determines the effect crate's closure.
//!
//! Moving the remaining physics vocabulary is therefore a separate ownership
//! decision, not unfinished spawn-road work: it should happen only with its codec
//! ownership so there is still one source of projectile physics truth. This crate
//! stays the lower authored-intent vocabulary until that boundary is deliberately
//! changed.

use bevy_math::Vec2;

/// A request to spawn one in-flight projectile: origin, direction, speed,
/// damage, lifetime, size, gravity, and presentation id. Ownership is carried by
/// the projectile domain's `ProjectileSpawnRequest`, not duplicated as a string.
///
/// Substrate-neutral data — projectile producers lower this through the
/// projectile domain's single spawn-request seam, and nothing here knows what a
/// platform is or which side fired it.
///
/// ⚠ was `EnemyProjectileSpawn`, named for a historical enemy-pool origin it
/// outgrew. The pool distinction is gone now; open-visual projectile producers
/// use the same authoritative request/materialization road as named body fire.
/// A type whose name contradicts its documentation is a standing invitation to
/// add an enemy-only assumption to it.
#[derive(Clone, Debug)]
pub struct ProjectileSpawn {
    pub origin: Vec2,
    pub dir: Vec2,
    pub speed: f32,
    pub damage: i32,
    pub max_lifetime: f32,
    pub half_extent: Vec2,
    /// Per-second downward acceleration each tick. Zero for hitscan-like
    /// volleys; positive for arcing/falling projectiles (e.g. apple rain).
    pub gravity: f32,
    /// Opaque visual id, carried for the render layer only. The physics never
    /// interprets it (exactly like `ProjectileSpec::charge_tier`): a game's
    /// content registers a named projectile look under this string (in Ambition,
    /// via `ambition_projectiles::ProjectileVisualCatalog`) and the render layer
    /// resolves it. The empty string is the unspecified / generic default.
    pub visual_id: String,
    /// How many valid support-face landings this shot may bounce off before it
    /// expires. Pairs with [`Self::bounce_on_world_contact`]; both default to the
    /// straight-and-dies-on-contact volley when a firer authors nothing.
    pub bounces: u8,
    /// Whether world contact bounces this shot (vs. expiring it).
    pub bounce_on_world_contact: bool,
}
