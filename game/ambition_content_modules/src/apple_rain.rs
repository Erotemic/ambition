//! Apple rain: while the boss presses the key, an apple falls every interval
//! of gameplay time, its lane spread across the boss's room by a golden-ratio
//! sequence and moved out from under the boss. Migrated from the native
//! system (fast-iteration I4/I7).

use ambition_boss_special_port::{BossCaster, BossSpecialCast};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, Attachment, CodeIdentity, EntryCode, EntryDescriptor, Fault, FieldDecl,
    FieldKind, FieldRef, IdlePolicy, Invocation, Limits, ModuleDescriptor, ModuleKey, Port,
    SaveEligibility, SchemaKey, StateSchema, TriggerBinding, Value, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

/// The special-action key in `boss_profiles.ron`.
pub const KEY: &str = "apple_rain";

const INTERVAL_S: f32 = 0.35;
const SPAWN_SPEED: f32 = 35.0;
const DAMAGE: i32 = 1;
const HALF_EXTENT: Vec2 = Vec2::new(14.0, 16.0);
const GRAVITY: f32 = 540.0;
const LIFETIME: f32 = 6.0;
const SPAWN_HEIGHT_ABOVE_BOSS: f32 = 320.0;
const PHI_FRAC: f32 = 0.618_033_99;
/// The most apples one call can drop: a tick of up to 1.4 s.
const MAX_PER_CALL: u32 = 4;

pub const RAIN: SchemaKey = SchemaKey::new(crate::PROVIDER, "apple_rain.rain", 1);
const SPAWN_ACCUM: FieldRef = FieldRef(0);
const SPAWN_INDEX: FieldRef = FieldRef(1);

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "apple_rain"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![StateSchema {
            key: RAIN,
            attachment: Attachment::Body,
            save: SaveEligibility::Transient,
            fields: vec![
                // Gameplay seconds since the last apple of this strike.
                FieldDecl::new(1, "spawn_accum", FieldKind::F32),
                // The golden-ratio sequence index of the next apple. It
                // continues across strikes.
                FieldDecl::new(2, "spawn_index", FieldKind::U32),
            ],
        }],
        entries: vec![EntryDescriptor {
            key: "rain".into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding {
                port: BossSpecialCast::KEY,
                selector: KEY.into(),
            },
            reads: Vec::new(),
            writes: vec![RAIN],
            requests: vec![ProjectileSpawnPort::KEY],
            after: Vec::new(),
            limits: Limits {
                max_requests: MAX_PER_CALL,
            },
            // NOT `ResetState`: an idle tick resets the interval, but the
            // lane sequence continues into the next strike.
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(rain),
        }],
    }
}

/// The world x of the `spawn_index`-th apple: a golden-ratio spread across
/// the room width, moved to the nearer side of the boss when it would fall
/// on the boss's body.
pub fn spawn_x(spawn_index: u32, world_width: f32, body_min_x: f32, body_max_x: f32) -> f32 {
    let margin = HALF_EXTENT.x + 8.0;
    let max_x = (world_width - margin).max(margin);
    let spawnable_width = (max_x - margin).max(0.0);
    let frac = ((spawn_index as f32) * PHI_FRAC).fract();
    let mut x = margin + frac * spawnable_width;
    let self_left = body_min_x - HALF_EXTENT.x;
    let self_right = body_max_x + HALF_EXTENT.x;
    if x > self_left && x < self_right {
        x = if x - self_left < self_right - x { self_left } else { self_right };
        x = x.clamp(margin, max_x);
    }
    x
}

fn rain(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    let dt = inv.dt();
    let schema_fault = |error| Fault::Schema { schema: RAIN, error };
    let record = inv.state(&RAIN)?;
    if !caster.pressed {
        // The strike is over: the next one starts on a clean beat.
        return record.set(SPAWN_ACCUM, Value::F32(0.0)).map_err(schema_fault);
    }
    // A dead boss, or one whose room cannot be told, keeps its interval.
    let Some(room) = caster.room_size.filter(|_| caster.alive) else {
        return Ok(());
    };
    let mut accum = record.get(SPAWN_ACCUM).map_err(schema_fault)?.as_f32().unwrap_or(0.0) + dt;
    let mut index = record.get(SPAWN_INDEX).map_err(schema_fault)?.as_u32().unwrap_or(0);
    let body_min_x = caster.body_center[0] - caster.body_half_size[0];
    let body_max_x = caster.body_center[0] + caster.body_half_size[0];
    let spawn_y = (caster.position[1] - SPAWN_HEIGHT_ABOVE_BOSS).max(HALF_EXTENT.y + 8.0);
    let mut drops = Vec::new();
    while accum >= INTERVAL_S {
        accum -= INTERVAL_S;
        drops.push(spawn_x(index, room[0], body_min_x, body_max_x));
        index = index.wrapping_add(1);
    }
    record.set(SPAWN_ACCUM, Value::F32(accum)).map_err(schema_fault)?;
    record.set(SPAWN_INDEX, Value::U32(index)).map_err(schema_fault)?;
    for x in drops {
        inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
            origin: Vec2::new(x, spawn_y),
            // Down at once, so the apple keeps its lane before gravity acts.
            dir: Vec2::new(0.0, 1.0),
            speed: SPAWN_SPEED,
            damage: DAMAGE,
            max_lifetime: LIFETIME,
            half_extent: HALF_EXTENT,
            gravity: GRAVITY,
            visual_id: "apple".into(),
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
    fn apples_stay_in_the_room_spread_and_miss_the_boss() {
        let world_width = 1792.0;
        let margin = HALF_EXTENT.x + 8.0;
        let (min_x, max_x) = (896.0 - 110.0, 896.0 + 110.0);
        let (left, right) = (min_x - HALF_EXTENT.x, max_x + HALF_EXTENT.x);
        let xs: Vec<f32> = (0..64).map(|i| spawn_x(i, world_width, min_x, max_x)).collect();
        for x in &xs {
            assert!(*x >= margin - 1e-3 && *x <= world_width - margin + 1e-3);
            assert!(*x <= left + 1e-3 || *x >= right - 1e-3);
        }
        let mid = world_width / 2.0;
        assert!(xs.iter().any(|&x| x < mid) && xs.iter().any(|&x| x > mid));
    }
}
