//! Mode collapse: during the telegraph the boss locks where its target is; on
//! the first strike tick a ring of shots appears around that point and
//! converges on it. Migrated from the native system (fast-iteration I4/I7).

use ambition_boss_special_port::{BossCaster, BossSpecialCast};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

use crate::strike;

/// The special-action key in `boss_profiles.ron`.
pub const KEY: &str = "mode_collapse_converge";

const RING_COUNT: u32 = 12;
const RING_RADIUS: f32 = 190.0;
const RING_SPEED: f32 = 320.0;
const RING_DAMAGE: i32 = 1;
const RING_HALF_EXTENT: Vec2 = Vec2::new(10.0, 10.0);
const RING_LIFETIME: f32 = 1.5;

pub const STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "mode_collapse.strike", 1);

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "mode_collapse"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![strike::locked_schema(STRIKE)],
        entries: vec![EntryDescriptor {
            key: "converge".into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding {
                port: BossSpecialCast::KEY,
                selector: KEY.into(),
            },
            reads: Vec::new(),
            writes: vec![STRIKE],
            requests: vec![ProjectileSpawnPort::KEY],
            after: Vec::new(),
            limits: Limits {
                max_requests: RING_COUNT,
            },
            // An idle tick (no press, no telegraph) ends any strike: the
            // same result as the call, without it.
            on_idle: IdlePolicy::ResetState,
            run: EntryCode::Native(converge),
        }],
    }
}

/// `count` (origin, inward direction) pairs evenly on a circle round `center`.
pub fn converge_ring(center: Vec2, count: u32, radius: f32) -> Vec<(Vec2, Vec2)> {
    let n = count.max(1);
    (0..n)
        .map(|i| {
            let theta = std::f32::consts::TAU * (i as f32) / (n as f32);
            let offset = Vec2::new(theta.cos(), theta.sin()) * radius;
            (center + offset, (-offset).normalize_or_zero())
        })
        .collect()
}

fn converge(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    let Some(center) = strike::locked(inv, &STRIKE, &caster)? else {
        return Ok(());
    };
    for (origin, dir) in converge_ring(Vec2::from(center), RING_COUNT, RING_RADIUS) {
        if dir.length_squared() < 1e-4 {
            continue;
        }
        inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
            origin,
            dir,
            speed: RING_SPEED,
            damage: RING_DAMAGE,
            max_lifetime: RING_LIFETIME,
            half_extent: RING_HALF_EXTENT,
            gravity: 0.0,
            visual_id: String::new(),
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 0.0,
            boomerang_return_s: None,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_sits_on_its_radius_and_aims_inward() {
        let center = Vec2::new(640.0, 400.0);
        let ring = converge_ring(center, 12, 190.0);
        assert_eq!(ring.len(), 12);
        for (origin, dir) in &ring {
            let out = *origin - center;
            assert!((out.length() - 190.0).abs() < 1e-2);
            assert!(dir.dot(out) < 0.0);
            assert!((dir.length() - 1.0).abs() < 1e-3);
        }
        assert_eq!(converge_ring(center, 0, 190.0).len(), 1);
    }
}
