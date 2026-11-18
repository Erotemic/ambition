//! Authored projectile intent — content-free spawn data, and nothing else.
//!
//! Lower authored-intent vocabulary consumed by the projectile domain and
//! re-exported by the shared projectile primitive. It intentionally has no
//! platformer/runtime dependency.
//!
//! `ProjectileSpec` and `WorldHitPolicy` remain with the shared projectile
//! primitive and its snapshot codecs. This crate owns only authored spawn intent.

use bevy_math::Vec2;
mod hit_properties;

/// A request to spawn one in-flight projectile: origin, direction, speed,
/// damage, lifetime, size, gravity, and presentation id. Ownership is carried by
/// the projectile domain's `ProjectileSpawnRequest`, not duplicated as a string.
///
/// Substrate-neutral data — projectile producers lower this through the
/// projectile domain's single spawn-request seam, and nothing here knows what a
/// platform is or which side fired it.
#[derive(Clone, Debug)]
pub struct ProjectileSpawn {
    pub properties: ambition_hit_spec::HitProperties,
    pub guard: ambition_hit_spec::HitGuard,
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
/// * **Owner** — `ambition_combat::projectile_spawn::extension`, which lowers the value
///   through `ProjectileSpawnRequest::open`, the one spawn-request seam.
/// * **Scope and grant** — the projectile's owner is the body the invocation
///   ran for. The module cannot name another owner.
/// * **Time** — offered in `technique_execution`, `wielded_use`,
///   `module_entity_tick`, `boss_conduct`, and `hit_reaction`. Early requests
///   materialize before the projectile step. Late reactions wait in rollback
///   state for the next early materializer.
/// * **Provenance** — the host attaches the invocation's move use; the damage
///   result names that use, not the move playing at impact (A12).
/// * **Result** — submitted is not applied: the projectile domain can still
///   refuse or retire the shot. No acknowledgement port exists yet.
pub struct ProjectileSpawnPort;

impl ambition_extension_sdk::Port for ProjectileSpawnPort {
    const KEY: ambition_extension_sdk::PortKey =
        ambition_extension_sdk::PortKey::new("ambition.projectiles.spawn", 2);
    const ROLE: ambition_extension_sdk::PortRole = ambition_extension_sdk::PortRole::Request;
    type Value = ProjectileSpawn;

    fn encode(v: &ProjectileSpawn, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(
        r: &mut ambition_extension_sdk::wire::WireReader<'_>,
    ) -> Result<ProjectileSpawn, ambition_extension_sdk::wire::WireError> {
        let spawn: ProjectileSpawn = ambition_extension_sdk::structure::read(r)?;
        if !spawn.properties.is_valid() { return Err(ambition_extension_sdk::wire::WireError::BadTag(255)); }
        Ok(spawn)
    }
}

/// The codec of a [`Vec2`]: `[x, y]`, as the SDK's.
pub struct Vec2Codec;

impl ambition_extension_sdk::structure::Field<Vec2> for Vec2Codec {
    fn shape() -> ambition_extension_sdk::structure::Shape {
        <[f32; 2] as ambition_extension_sdk::structure::Field<[f32; 2]>>::shape()
    }
    fn put(v: &Vec2, out: &mut Vec<u8>) {
        <[f32; 2] as ambition_extension_sdk::structure::Field<[f32; 2]>>::put(&v.to_array(), out)
    }
    fn read(r: &mut ambition_extension_sdk::wire::WireReader<'_>) -> Result<Vec2, ambition_extension_sdk::wire::WireError> {
        <[f32; 2] as ambition_extension_sdk::structure::Field<[f32; 2]>>::read(r).map(Vec2::from)
    }
    fn to_script(v: &Vec2) -> ambition_extension_sdk::script::ScriptValue {
        <[f32; 2] as ambition_extension_sdk::structure::Field<[f32; 2]>>::to_script(&v.to_array())
    }
    fn from_script(v: &ambition_extension_sdk::script::ScriptValue) -> Result<Vec2, String> {
        <[f32; 2] as ambition_extension_sdk::structure::Field<[f32; 2]>>::from_script(v).map(Vec2::from)
    }
}

ambition_extension_sdk::port_record! {
    ProjectileSpawn {
        /// Contact policy.
        properties: ambition_hit_spec::HitProperties as hit_properties::HitPropertiesCodec = ambition_hit_spec::HitProperties::default(),
        /// Shield damage bonus, or unit for an unblockable shot.
        guard: ambition_hit_spec::HitGuard as hit_properties::HitGuardCodec = ambition_hit_spec::HitGuard::default(),
        /// Where the shot starts (world units, +Y down).
        origin: Vec2 as Vec2Codec,
        /// Its direction; the domain normalizes it.
        dir: Vec2 as Vec2Codec,
        /// World units per second.
        speed: f32,
        /// Damage to the body it hits.
        damage: i32,
        /// Seconds before it expires.
        max_lifetime: f32,
        /// Half its width and height.
        half_extent: Vec2 as Vec2Codec,
        /// Downward acceleration per second; zero for a straight shot.
        gravity: f32 = 0.0,
        /// A named projectile look, for the render layer only; empty for
        /// the generic one.
        visual_id: String = String::new(),
        /// How many support-face landings it bounces off before it expires.
        bounces: u8 = 0,
        /// Whether world contact bounces it rather than expiring it.
        bounce_on_world_contact: bool = false,
        /// Seconds until it turns and comes back, or `()` for never.
        boomerang_return_s: Option<f32> = None,
        /// Half the size of the burst it deals where it lands; zero for none.
        splash_half_extent: f32 = 0.0,
    }
}

ambition_extension_sdk::script_port!(ProjectileSpawnPort);

#[cfg(test)]
mod hit_policy_tests {
    use super::*;
    use ambition_extension_sdk::{Port, structure::Field};
    use ambition_hit_spec::*;

    #[test]
    fn script_and_wire_keep_projectile_contact_policy() {
        let shot = ProjectileSpawn {
            origin: Vec2::ZERO, dir: Vec2::X, speed: 300.0, damage: 7,
            max_lifetime: 2.0, half_extent: Vec2::splat(5.0), gravity: 0.0,
            visual_id: String::new(), bounces: 0, bounce_on_world_contact: false,
            boomerang_return_s: None, splash_half_extent: 0.0,
            guard: HitGuard::Unblockable,
            properties: HitProperties {
                clash: ClashPolicy::Transcendent, hitlag_scale: 0.5, sdi_scale: 0.0,
                tech: TechPolicy::Deny, weight: LaunchWeight::Fixed(1.0),
                armor: ArmorInteraction::Bypass,
                targets: HitTargets { grounded: false, airborne: true, prone: false },
            },
        };
        let script = ProjectileSpawn::to_script(&shot);
        let scripted = ProjectileSpawn::from_script(&script).unwrap();
        let mut bytes = Vec::new();
        ProjectileSpawnPort::encode(&scripted, &mut bytes);
        let mut reader = ambition_extension_sdk::wire::WireReader::new(&bytes);
        let decoded = ProjectileSpawnPort::decode(&mut reader).unwrap();
        assert_eq!(decoded.properties, shot.properties);
        assert_eq!(decoded.guard, shot.guard);
        let mut invalid = shot;
        invalid.properties.hitlag_scale = -1.0;
        bytes.clear();
        ProjectileSpawnPort::encode(&invalid, &mut bytes);
        assert!(ProjectileSpawnPort::decode(&mut ambition_extension_sdk::wire::WireReader::new(&bytes)).is_err());
    }
}
