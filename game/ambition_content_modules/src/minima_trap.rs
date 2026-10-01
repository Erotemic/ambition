//! Minima trap: on the first strike tick a pit of damage opens where the
//! boss's target is, and a crawler appears beside it, on the boss's side.
//! Migrated from the native system (fast-iteration I4/I7).

use ambition_boss_special_port::{BossCaster, BossSpecialCast, BossSummon, BossSummonPort};
use ambition_combat_port::{DamageBox, DamageBoxPort};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

use crate::strike;

/// The special-action key in `boss_profiles.ron`.
pub const KEY: &str = "minima_trap";

const HAZARD_DURATION_S: f32 = 5.0;
const DAMAGE: i32 = 2;
const HALF_EXTENT: [f32; 2] = [56.0, 24.0];
const KNOCKBACK: f32 = 1.4;
/// The summon id namespace (kept from the native trap, so saves and the
/// inspector see the same ids).
const MINION_LABEL: &str = "gradient_sentinel_minima_minion";
/// The pacifist crawler. A character id that names nothing makes a generic
/// body, so this must name a real character.
const MINION_CHARACTER: &str = "npc_puppy_slug";
const MINION_HALF_SIZE: [f32; 2] = [24.0, 11.0];
/// The crawler appears this far from the pit centre, toward the boss: outside
/// the pit's half width and the player's body, so it never appears on the
/// player.
const MINION_SPAWN_OFFSET: f32 = 90.0;

pub const STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "minima_trap.strike", 1);

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "minima_trap"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![strike::once_numbered_schema(STRIKE)],
        entries: vec![EntryDescriptor {
            key: "trap".into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding {
                port: BossSpecialCast::KEY,
                selector: KEY.into(),
            },
            reads: Vec::new(),
            writes: vec![STRIKE],
            requests: vec![DamageBoxPort::KEY, BossSummonPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 2 },
            // The strike number continues across strikes.
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(trap),
        }],
    }
}

fn trap(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    let Some(number) = strike::once_numbered(inv, &STRIKE, &caster)? else {
        return Ok(());
    };
    let pit = caster.target.unwrap_or(caster.position);
    inv.submit::<DamageBoxPort>(DamageBox {
        center: pit,
        half_extent: HALF_EXTENT,
        damage: DAMAGE,
        knockback: KNOCKBACK,
        lifetime_s: HAZARD_DURATION_S.max(0.05),
    })?;
    let to_boss_x = caster.position[0] - pit[0];
    // Boss straight above or below the pit: the left side, so the crawler is
    // never at the pit centre.
    let side = if to_boss_x.abs() < f32::EPSILON { -1.0 } else { to_boss_x.signum() };
    inv.submit::<BossSummonPort>(BossSummon {
        label: MINION_LABEL.into(),
        serial: vec![number],
        position: [pit[0] + side * MINION_SPAWN_OFFSET, pit[1]],
        half_size: MINION_HALF_SIZE,
        character_id: MINION_CHARACTER.into(),
        health: None,
        keeps_contact_damage: true,
    })?;
    Ok(())
}
