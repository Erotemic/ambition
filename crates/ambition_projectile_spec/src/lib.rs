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

/// The projectile domain's extension request port: a module submits one
/// [`ProjectileSpawn`].
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — spawn one in-flight projectile.
/// * **Owner** — `ambition_projectiles::extension`, which lowers the value
///   through `ProjectileSpawnRequest::open`, the one spawn-request seam.
/// * **Scope and grant** — the projectile's owner is the body the invocation
///   ran for. The module cannot name another owner.
/// * **Time** — offered in `technique_execution`. The request materializes
///   before this tick's projectile step (`ProjectileStart::StepThisTick`).
/// * **Provenance** — the host attaches the invocation's move use; the damage
///   result names that use, not the move playing at impact (A12).
/// * **Result** — submitted is not applied: the projectile domain can still
///   refuse or retire the shot. No acknowledgement port exists yet.
pub struct ProjectileSpawnPort;

impl ambition_extension_sdk::Port for ProjectileSpawnPort {
    const KEY: ambition_extension_sdk::PortKey =
        ambition_extension_sdk::PortKey::new("ambition.projectiles.spawn", 1);
    const ROLE: ambition_extension_sdk::PortRole = ambition_extension_sdk::PortRole::Request;
    type Value = ProjectileSpawn;

    fn encode(v: &ProjectileSpawn, out: &mut Vec<u8>) {
        use ambition_extension_sdk::wire::*;
        put_vec2(out, v.origin.into());
        put_vec2(out, v.dir.into());
        put_f32(out, v.speed);
        put_i32(out, v.damage);
        put_f32(out, v.max_lifetime);
        put_vec2(out, v.half_extent.into());
        put_f32(out, v.gravity);
        put_str(out, &v.visual_id);
        put_u8(out, v.bounces);
        put_bool(out, v.bounce_on_world_contact);
        put_opt(out, v.boomerang_return_s, put_f32);
        put_f32(out, v.splash_half_extent);
    }

    fn decode(
        r: &mut ambition_extension_sdk::wire::Reader<'_>,
    ) -> Result<ProjectileSpawn, ambition_extension_sdk::wire::WireError> {
        use ambition_extension_sdk::wire::Reader;
        Ok(ProjectileSpawn {
            origin: r.vec2()?.into(),
            dir: r.vec2()?.into(),
            speed: r.f32()?,
            damage: r.i32()?,
            max_lifetime: r.f32()?,
            half_extent: r.vec2()?.into(),
            gravity: r.f32()?,
            visual_id: r.str()?.to_owned(),
            bounces: r.u8()?,
            bounce_on_world_contact: r.bool()?,
            boomerang_return_s: r.opt(Reader::f32)?,
            splash_half_extent: r.f32()?,
        })
    }
}
