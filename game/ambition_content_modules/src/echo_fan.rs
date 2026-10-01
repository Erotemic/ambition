//! The Mockingbird's echo fan: one strike copies a shot across a cone aimed
//! at the boss's target.
//!
//! Migrated from the native `spawn_echo_fan_from_special_messages` system
//! (fast-iteration I4). The beat `Special("echo_fan")` in `boss_profiles.ron`
//! triggers it.
//!
//! A STRIKE is a run of ticks on which the boss presses the key. The fan
//! fires once per strike, on the first tick the boss is alive. A tick with no
//! press ends the strike.

use ambition_boss_special_port::{BossCaster, BossSpecialCast};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, StateSchema, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

/// The special-action key in `boss_profiles.ron`.
pub const KEY: &str = "echo_fan";

const COUNT: u32 = 7;
const SPREAD_RAD: f32 = 0.9; // total cone width (~52°)
const SPEED: f32 = 300.0;
const DAMAGE: i32 = 1;
const HALF_EXTENT: Vec2 = Vec2::new(9.0, 9.0);
const LIFETIME: f32 = 2.0;

/// The strike record of one boss.
pub const STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "echo_fan.strike", 1);

pub fn schema() -> StateSchema {
    crate::strike::once_schema(STRIKE)
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "echo_fan"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![schema()],
        entries: vec![EntryDescriptor {
            key: "fire".into(),
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
            // An idle tick (no press, no telegraph) ends any strike: the
            // same result as the call, without it.
            on_idle: IdlePolicy::ResetState,
            run: EntryCode::Native(fire),
        }],
    }
}

fn fire(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    if !crate::strike::once(inv, &STRIKE, &caster)? {
        return Ok(());
    }

    let origin = Vec2::from(caster.position) + Vec2::from(caster.projectile_offset);
    // Aim at the target; with no target, straight ahead by facing.
    let aim = caster
        .target
        .map(|t| Vec2::from(t) - origin)
        .filter(|d| d.length_squared() > 1e-4)
        .unwrap_or_else(|| Vec2::new(caster.facing, 0.0));
    for dir in fan(aim, COUNT, SPREAD_RAD) {
        inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
            origin,
            dir,
            speed: SPEED,
            damage: DAMAGE,
            max_lifetime: LIFETIME,
            half_extent: HALF_EXTENT,
            gravity: 0.0,
            visual_id: String::new(),
            // A straight shot: this ability authors no bounce.
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 0.0,
            boomerang_return_s: None,
        })?;
    }
    Ok(())
}

/// `count` unit directions, evenly across a `spread` cone centred on `aim`.
/// One shot flies straight along `aim`.
pub fn fan(aim: Vec2, count: u32, spread: f32) -> Vec<Vec2> {
    let n = count.max(1);
    let base = if aim.length_squared() < 1e-6 {
        0.0
    } else {
        aim.y.atan2(aim.x)
    };
    (0..n)
        .map(|i| {
            let t = if n == 1 {
                0.0
            } else {
                (i as f32) / ((n - 1) as f32) - 0.5
            };
            let theta = base + t * spread;
            Vec2::new(theta.cos(), theta.sin())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fan_spreads_evenly_around_the_aim() {
        let fan7 = fan(Vec2::X, 7, 0.9);
        assert_eq!(fan7.len(), 7);
        for d in &fan7 {
            assert!((d.length() - 1.0).abs() < 1e-3, "unit dirs");
        }
        assert!(fan7[3].y.abs() < 1e-3 && fan7[3].x > 0.0, "centre shot is the aim");
        assert!((fan7[0].y + fan7[6].y).abs() < 1e-3, "symmetric about the aim");
        assert!(fan7[0].y * fan7[6].y < 0.0, "ends straddle the aim");
        let one = fan(Vec2::X, 1, 0.9);
        assert_eq!(one.len(), 1);
        assert!(one[0].y.abs() < 1e-3 && one[0].x > 0.0);
    }

    #[test]
    fn the_schema_is_valid() {
        assert_eq!(schema().validate(), Ok(()));
    }
}
