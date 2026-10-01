//! The seismic stomp: on the first tick of a strike, a line of damage boxes
//! stands on the floor under the boss's feet, one under the boss and five
//! each side. Migrated from the native system (fast-iteration I4/I7).

use ambition_boss_special_port::{BossCaster, BossSpecialCast};
use ambition_combat_port::{DamageBox, DamageBoxPort};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

/// The special-action key in `boss_profiles.ron`.
pub const KEY: &str = "seismic_stomp";

const SEGMENTS_PER_SIDE: i32 = 5;
const SPACING: f32 = 84.0;
const HALF_EXTENT: [f32; 2] = [40.0, 26.0];
const DAMAGE: i32 = 2;
const KNOCKBACK: f32 = 1.8;
const LIFETIME: f32 = 0.55;

pub const STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "seismic_stomp.strike", 1);

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "seismic_stomp"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![crate::strike::once_schema(STRIKE)],
        entries: vec![EntryDescriptor {
            key: "stomp".into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding {
                port: BossSpecialCast::KEY,
                selector: KEY.into(),
            },
            reads: Vec::new(),
            writes: vec![STRIKE],
            requests: vec![DamageBoxPort::KEY],
            after: Vec::new(),
            limits: Limits {
                max_requests: (2 * SEGMENTS_PER_SIDE + 1) as u32,
            },
            // An idle tick (no press, no telegraph) ends any strike: the
            // same result as the call, without it.
            on_idle: IdlePolicy::ResetState,
            run: EntryCode::Native(stomp),
        }],
    }
}

/// Offsets from the boss: the boss's own tile, then `per_side` each way.
pub fn offsets(per_side: i32, spacing: f32) -> Vec<f32> {
    (-per_side.max(0)..=per_side.max(0)).map(|i| i as f32 * spacing).collect()
}

fn stomp(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    if !crate::strike::once(inv, &STRIKE, &caster)? {
        return Ok(());
    }
    // The line stands on the bottom face of the combat box (+Y is down).
    let foot_y = caster.body_center[1] + caster.body_half_size[1] - HALF_EXTENT[1];
    for dx in offsets(SEGMENTS_PER_SIDE, SPACING) {
        inv.submit::<DamageBoxPort>(DamageBox {
            center: [caster.position[0] + dx, foot_y],
            half_extent: HALF_EXTENT,
            damage: DAMAGE,
            knockback: KNOCKBACK,
            lifetime_s: LIFETIME,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offsets_are_symmetric_and_include_the_boss_tile() {
        let o = offsets(5, 84.0);
        assert_eq!(o.len(), 11);
        assert!(o.contains(&0.0));
        for (a, b) in o.iter().zip(o.iter().rev()) {
            assert!((a + b).abs() < 1e-3);
        }
        assert_eq!(offsets(0, 84.0), vec![0.0]);
    }
}
