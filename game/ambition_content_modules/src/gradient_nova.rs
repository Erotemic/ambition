//! The gradient nova: on the first tick of a strike, sixteen shots burst out
//! of the boss in a full circle, at three speed tiers so the ring tears into
//! layers. Migrated from the native system (fast-iteration I4/I7).

use ambition_boss_special_port::{BossCaster, BossSpecialCast};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy,
    Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

/// The special-action key in `boss_profiles.ron`.
pub const KEY: &str = "gradient_nova";

const COUNT: u32 = 16;
const BASE_SPEED: f32 = 260.0;
const DAMAGE: i32 = 1;
const HALF_EXTENT: Vec2 = Vec2::new(9.0, 9.0);
const LIFETIME: f32 = 1.6;
const SPAWN_RADIUS: f32 = 28.0;

pub const STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "gradient_nova.strike", 1);

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "gradient_nova"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![crate::strike::once_schema(STRIKE)],
        entries: vec![EntryDescriptor {
            key: "burst".into(),
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
                max_requests: COUNT,
            },
            // An idle tick ends any strike: the same result as the call.
            on_idle: IdlePolicy::ResetState,
            run: EntryCode::Native(burst),
        }],
    }
}

/// `count` (direction, speed) pairs round a full circle; every third shot
/// is half again faster than the one before it.
pub fn nova(count: u32, base_speed: f32) -> Vec<(Vec2, f32)> {
    let n = count.max(1);
    (0..n)
        .map(|i| {
            let theta = std::f32::consts::TAU * (i as f32) / (n as f32);
            let dir = Vec2::new(theta.cos(), theta.sin());
            let speed = base_speed * (1.0 + 0.5 * (i % 3) as f32);
            (dir, speed)
        })
        .collect()
}

fn burst(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    if !crate::strike::once(inv, &STRIKE, &caster)? {
        return Ok(());
    }
    let origin = Vec2::from(caster.position) + Vec2::from(caster.projectile_offset);
    for (dir, speed) in nova(COUNT, BASE_SPEED) {
        inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
            origin: origin + dir * SPAWN_RADIUS,
            dir,
            speed,
            damage: DAMAGE,
            max_lifetime: LIFETIME,
            half_extent: HALF_EXTENT,
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
    fn the_nova_spreads_a_full_circle_in_three_speed_tiers() {
        let shots = nova(16, 260.0);
        assert_eq!(shots.len(), 16);
        for (dir, _) in &shots {
            assert!((dir.length() - 1.0).abs() < 1e-3);
        }
        let speeds: std::collections::BTreeSet<u32> =
            shots.iter().map(|(_, s)| (*s * 10.0) as u32).collect();
        assert_eq!(speeds.len(), 3, "three tiers: {speeds:?}");
        let sum: Vec2 = shots.iter().map(|(d, _)| *d).sum();
        assert!(sum.length() < 1e-3, "a full circle cancels");
    }
}
