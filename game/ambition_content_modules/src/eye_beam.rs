//! The Smirking Behemoth's eye beam: during the telegraph the boss locks where
//! its target is; on the first strike tick it fires a short line of fast
//! bubble-laser boxes from its eye toward that point. One readable beam, not
//! a barrage. Migrated from the native system (fast-iteration I4/I7).

use ambition_boss_special_port::{BossCaster, BossSpecialCast};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

use crate::strike;

/// The special-action key in `boss_profiles.ron`.
pub const KEY: &str = "eye_beam";

const SHOT_SPEED: f32 = 780.0;
const DAMAGE: i32 = 1;
const BOX_COUNT: u8 = 5;
const BOX_SPACING: f32 = 26.0;
const HALF_EXTENT: Vec2 = Vec2::new(15.0, 8.0);
const LIFETIME_S: f32 = 0.58;

pub const STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "eye_beam.strike", 1);

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "eye_beam"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![strike::locked_schema(STRIKE)],
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
                max_requests: u32::from(BOX_COUNT),
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
    let Some(target) = strike::locked(inv, &STRIKE, &caster)? else {
        return Ok(());
    };
    // The eye is mirrored with the body.
    let offset = Vec2::new(
        caster.projectile_offset[0] * caster.facing,
        caster.projectile_offset[1],
    );
    let origin = Vec2::from(caster.position) + offset;
    let delta = Vec2::from(target) - origin;
    let dir = if delta.length_squared() < 1e-4 {
        Vec2::new(caster.facing, 0.0)
    } else {
        delta.normalize()
    };
    for i in 0..BOX_COUNT.max(1) {
        inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
            origin: origin + dir * BOX_SPACING.max(1.0) * f32::from(i),
            dir,
            speed: SHOT_SPEED.max(1.0),
            damage: DAMAGE,
            max_lifetime: LIFETIME_S.max(0.05),
            half_extent: Vec2::new(HALF_EXTENT.x.max(1.0), HALF_EXTENT.y.max(1.0)),
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
