//! Authored projectile intent — content-free spawn data, and nothing else.
//!
//! Lower authored-intent vocabulary consumed by the projectile domain and
//! re-exported by the shared projectile primitive. It intentionally has no
//! platformer/runtime dependency.
//!
//! `ProjectileSpec` and `WorldHitPolicy` remain with the shared projectile
//! primitive and its snapshot codecs. This crate owns only authored spawn intent.

use bevy_math::Vec2;

/// A request to spawn one in-flight projectile: origin, direction, speed,
/// damage, lifetime, size, gravity, and presentation id. Ownership is carried by
/// the projectile domain's `ProjectileSpawnRequest`, not duplicated as a string.
///
/// Substrate-neutral data — projectile producers lower this through the
/// projectile domain's single spawn-request seam, and nothing here knows what a
/// platform is or which side fired it.
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
    /// Seconds until this shot turns around and comes back, or `None` for one
    /// that never does. See `ProjectileGameplay::accel`.
    pub boomerang_return_s: Option<f32>,
    /// Half-extent of the burst this shot deals when it lands (on a body, a
    /// feature, or the world), or `0.0` for a shot that hits only what it
    /// touched. A fireball's splash; a bolt's nothing. Absorbed from the former
    /// held-shot simulation so one projectile road carries every projectile.
    pub splash_half_extent: f32,
}
