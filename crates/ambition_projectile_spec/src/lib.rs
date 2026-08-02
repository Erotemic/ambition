//! **Authored projectile intent — content-free spawn data, and nothing else.**
//!
//! Carved out of `ambition_platformer2d_shared_tangle` (2026-08-02) for one
//! measured reason: `ambition_vfx` imported exactly ONE item from that 16,927-line
//! crate — [`ProjectileSpawn`] — and that single edge was the last thing keeping
//! `ambition_platformer2d_core` in its build closure.
//!
//! ## Why this is the first carve that actually decouples anything
//!
//! Four crates had been carved off the core crate before this one, and every one
//! of them still reached core transitively through a SECOND platformer
//! dependency (`cargo tree --edges normal -i` says so). They bought honest
//! manifests, not decoupling. `ambition_vfx` had two edges — core, dropped when
//! `ambition_geometry` appeared, and `shared_tangle`, dropped here — and its
//! other two dependencies (`ambition_geometry`, `ambition_sfx`) are already
//! core-free. So it is the first crate to genuinely leave.
//!
//! ## ⛔ Why its two siblings could NOT come with it
//!
//! `ProjectileSpec` and `WorldHitPolicy` are equally content-free and belong
//! here on every other measure. They are blocked, and not by the rollback
//! FINGERPRINT that blocks the snapshot codec — by the **orphan rule**:
//!
//! ```text
//! snapshot_impls.rs:  snapshot_unit_enum!(crate::projectile::WorldHitPolicy { … })
//! ```
//!
//! That macro implements core's `SnapshotState` for `WorldHitPolicy` from inside
//! `shared_tangle`. Move the type here and that impl becomes a foreign trait on a
//! foreign type — it stops compiling. Moving the impl too would make THIS crate
//! depend on core for `SnapshotState`, which puts core straight back into
//! `ambition_vfx`'s closure and defeats the carve entirely. `ProjectileSpec`
//! follows it, because it carries a `world_hit: WorldHitPolicy` field.
//!
//! ⭐ **so the snapshot codec's home blocks carve-outs TWICE over**, by two
//! independent mechanisms: it moves the rollback fingerprint (queue S30), and it
//! anchors trait impls to whichever crate owns the trait. Only the first was
//! written down. Both are arguments for the same decision.
//!
//! When S30 lands and the codec leaves core, `ProjectileSpec` and
//! `WorldHitPolicy` should join this crate — that is what it is shaped for, and
//! why it is named for the vocabulary rather than for the one type it holds today.

use bevy_math::Vec2;

/// A request to spawn one in-flight projectile: origin, direction, speed,
/// damage, lifetime, size, owner id, gravity.
///
/// Substrate-neutral data — the effect vocabulary and both projectile pools
/// build bodies from it, and nothing here knows what a platform is.
///
/// ⚠ was `EnemyProjectileSpawn`, named for a historical enemy-pool origin it
/// outgrew: the player pool builds these too, and its own doc already said "it
/// is pool-agnostic". A type whose name contradicts its documentation is a
/// standing invitation to add an enemy-only assumption to it.
#[derive(Clone, Debug)]
pub struct ProjectileSpawn {
    pub origin: Vec2,
    pub dir: Vec2,
    pub speed: f32,
    pub damage: i32,
    pub max_lifetime: f32,
    /// Id of the spawning actor — self-friendly-fire ignore lists, sprite
    /// routing in the visuals layer, debug traces.
    pub owner_id: String,
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
