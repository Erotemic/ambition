//! Gradient cascade: on the first strike tick, minions drop in from the top
//! of the arena, spread evenly about the boss. Migrated from the native
//! system (fast-iteration I4/I7).

use ambition_boss_special_port::{BossCaster, BossSpecialCast, BossSummon, BossSummonPort};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

use crate::strike;

/// The special-action key in `boss_profiles.ron`.
pub const KEY: &str = "gradient_cascade";

const MINION_COUNT: u32 = 2;
/// The summon id namespace (kept from the native cascade).
const MINION_LABEL: &str = "gradient_sentinel_cascade";
const MINION_CHARACTER: &str = "npc_ai_slop";
const MINION_HALF_SIZE: [f32; 2] = [15.0, 20.0];
/// The world y the minions appear at: the top of the arena.
const SPAWN_Y: f32 = 80.0;
/// The outermost minions are this far from the boss in x.
const X_SPREAD: f32 = 220.0;

pub const STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "gradient_cascade.strike", 1);

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "gradient_cascade"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![strike::once_numbered_schema(STRIKE)],
        entries: vec![EntryDescriptor {
            key: "cascade".into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding {
                port: BossSpecialCast::KEY,
                selector: KEY.into(),
            },
            reads: Vec::new(),
            writes: vec![STRIKE],
            requests: vec![BossSummonPort::KEY],
            after: Vec::new(),
            limits: Limits {
                max_requests: MINION_COUNT,
            },
            // The strike number continues across strikes.
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(cascade),
        }],
    }
}

/// The x offset from the boss of minion `i` of `count`: evenly from
/// `-X_SPREAD` to `X_SPREAD`; a lone minion on the boss.
pub fn minion_x_offset(i: u32, count: u32) -> f32 {
    let t = if count <= 1 { 0.5 } else { i as f32 / (count - 1) as f32 };
    (t - 0.5) * 2.0 * X_SPREAD
}

fn cascade(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    let Some(number) = strike::once_numbered(inv, &STRIKE, &caster)? else {
        return Ok(());
    };
    let count = MINION_COUNT.max(1);
    for i in 0..count {
        inv.submit::<BossSummonPort>(BossSummon {
            label: MINION_LABEL.into(),
            serial: vec![number, i],
            position: [caster.position[0] + minion_x_offset(i, count), SPAWN_Y],
            half_size: MINION_HALF_SIZE,
            character_id: MINION_CHARACTER.into(),
            health: None,
            keeps_contact_damage: true,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_minions_spread_symmetrically_about_the_boss() {
        assert_eq!(minion_x_offset(0, 1), 0.0);
        assert_eq!(minion_x_offset(0, 2), -X_SPREAD);
        assert_eq!(minion_x_offset(1, 2), X_SPREAD);
        assert_eq!(minion_x_offset(2, 5), 0.0);
        let xs: Vec<f32> = (0..5).map(|i| minion_x_offset(i, 5)).collect();
        assert!((xs[0] + xs[4]).abs() < 1e-3);
        assert!(xs.windows(2).all(|w| w[1] > w[0]));
        assert!(xs.iter().all(|x| x.abs() <= X_SPREAD + 1e-3));
    }
}
