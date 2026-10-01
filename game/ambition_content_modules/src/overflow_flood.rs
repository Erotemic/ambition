//! Overflow's boundary flood: during the telegraph the boss locks where its
//! target is (the safe lane); on the first strike tick shots fall from above
//! in every column of the boss's room except that lane. Migrated from the
//! native system (fast-iteration I4/I7).

use ambition_boss_special_port::{BossCaster, BossSpecialCast};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

use crate::strike;

/// The special-action key in `boss_profiles.ron`.
pub const KEY: &str = "overflow_flood";

const SPACING: f32 = 60.0;
const GAP_HALF: f32 = 78.0;
const MARGIN: f32 = 40.0;
const SPEED: f32 = 60.0;
const GRAVITY: f32 = 520.0;
const DAMAGE: i32 = 1;
const HALF_EXTENT: Vec2 = Vec2::new(12.0, 14.0);
const LIFETIME: f32 = 6.0;
const SPAWN_HEIGHT_ABOVE_BOSS: f32 = 300.0;
/// The most columns a room can have: a room 8192 units wide.
const MAX_COLUMNS: u32 = 8192 / SPACING as u32 + 1;

pub const STRIKE: SchemaKey = SchemaKey::new(crate::PROVIDER, "overflow_flood.strike", 1);

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "overflow_flood"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![strike::locked_schema(STRIKE)],
        entries: vec![EntryDescriptor {
            key: "flood".into(),
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
                max_requests: MAX_COLUMNS,
            },
            // An idle tick (no press, no telegraph) ends any strike: the
            // same result as the call, without it.
            on_idle: IdlePolicy::ResetState,
            run: EntryCode::Native(flood),
        }],
    }
}

/// The world-x columns of the flood: evenly spaced across the room width at
/// `spacing`, without the columns within `gap_half` of `gap_x` (the one lane
/// the player must hold).
pub fn columns(world_width: f32, spacing: f32, gap_x: f32, gap_half: f32) -> Vec<f32> {
    let spacing = spacing.max(8.0);
    let min_x = MARGIN;
    let max_x = (world_width - MARGIN).max(min_x);
    let mut out = Vec::new();
    let mut x = min_x;
    while x <= max_x {
        if (x - gap_x).abs() > gap_half {
            out.push(x);
        }
        x += spacing;
    }
    out
}

fn flood(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    // Without the boss's room there is no width to flood: the strike stays
    // armed until the room can be told.
    let Some(gap) = strike::locked_when(inv, &STRIKE, &caster, caster.room_size.is_some())? else {
        return Ok(());
    };
    let Some(room) = caster.room_size else {
        return Ok(());
    };
    let spawn_y = (caster.position[1] - SPAWN_HEIGHT_ABOVE_BOSS).max(HALF_EXTENT.y + 8.0);
    for x in columns(room[0], SPACING, gap[0], GAP_HALF) {
        inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
            origin: Vec2::new(x, spawn_y),
            dir: Vec2::new(0.0, 1.0),
            speed: SPEED,
            damage: DAMAGE,
            max_lifetime: LIFETIME,
            half_extent: HALF_EXTENT,
            gravity: GRAVITY,
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
    fn the_flood_leaves_exactly_one_safe_lane() {
        let world_width = 1792.0;
        let (gap_x, gap_half) = (900.0, 78.0);
        let cols = columns(world_width, 60.0, gap_x, gap_half);
        assert!(!cols.is_empty(), "the flood has columns");
        assert!(cols.iter().all(|&x| (x - gap_x).abs() > gap_half));
        assert!(cols.iter().any(|&x| x < gap_x - gap_half));
        assert!(cols.iter().any(|&x| x > gap_x + gap_half));
        assert!(cols.iter().all(|&x| x >= MARGIN - 1e-3));
        assert!(cols.iter().all(|&x| x <= world_width - MARGIN + 1e-3));
        assert!(columns(8192.0, SPACING, -1000.0, GAP_HALF).len() as u32 <= MAX_COLUMNS);
    }
}
