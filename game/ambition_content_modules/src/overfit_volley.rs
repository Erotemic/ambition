//! Overfit volley: during the telegraph the boss memorises where its target
//! is, once at the start and then every interval of gameplay time; on the
//! first strike tick it fires one bolt at each memorised point. Migrated
//! from the native system (fast-iteration I4/I7).

use ambition_boss_special_port::{BossCaster, BossSpecialCast};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, Attachment, CodeIdentity, EntryCode, EntryDescriptor, Fault, FieldDecl,
    FieldKind, FieldRef, IdlePolicy, Invocation, Limits, ModuleDescriptor, ModuleKey, Port,
    SaveEligibility, SchemaKey, StateSchema, TriggerBinding, Value, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

/// The special-action key in `boss_profiles.ron`.
pub const KEY: &str = "overfit_volley";

const SAMPLE_INTERVAL_S: f32 = 0.30;
const SAMPLE_COUNT: u32 = 5;
const SHOT_SPEED: f32 = 360.0;
const SHOT_DAMAGE: i32 = 1;
const BOLT_HALF_EXTENT: Vec2 = Vec2::new(8.0, 8.0);
const BOLT_LIFETIME: f32 = 2.4;

pub const VOLLEY: SchemaKey = SchemaKey::new(crate::PROVIDER, "overfit_volley.volley", 1);
const SAMPLES: FieldRef = FieldRef(0);
const SAMPLE_ACCUM: FieldRef = FieldRef(1);
const FIRED_THIS_STRIKE: FieldRef = FieldRef(2);
const HAD_SEED_SAMPLE: FieldRef = FieldRef(3);

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "overfit_volley"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![StateSchema {
            key: VOLLEY,
            attachment: Attachment::Body,
            save: SaveEligibility::Transient,
            fields: vec![
                // The memorised target points, in the order taken.
                FieldDecl::new(1, "samples", FieldKind::seq(FieldKind::Vec2, SAMPLE_COUNT)),
                // Gameplay seconds since the last sample.
                FieldDecl::new(2, "sample_accum", FieldKind::F32),
                FieldDecl::new(3, "fired_this_strike", FieldKind::Bool),
                // True when this telegraph took its first sample.
                FieldDecl::new(4, "had_seed_sample", FieldKind::Bool),
            ],
        }],
        entries: vec![EntryDescriptor {
            key: "volley".into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding {
                port: BossSpecialCast::KEY,
                selector: KEY.into(),
            },
            reads: Vec::new(),
            writes: vec![VOLLEY],
            requests: vec![ProjectileSpawnPort::KEY],
            after: Vec::new(),
            limits: Limits {
                max_requests: SAMPLE_COUNT,
            },
            // An idle tick (no press, no telegraph) forgets the samples and
            // ends any strike: the same result as the call, without it.
            on_idle: IdlePolicy::ResetState,
            run: EntryCode::Native(volley),
        }],
    }
}

struct Volley {
    samples: Vec<[f32; 2]>,
    sample_accum: f32,
    fired_this_strike: bool,
    had_seed_sample: bool,
}

fn volley(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    let dt = inv.dt();
    let schema_fault = |error| Fault::Schema { schema: VOLLEY, error };
    let record = inv.state(&VOLLEY)?;
    let samples = match record.get(SAMPLES).map_err(schema_fault)? {
        Value::Seq(points) => points.iter().filter_map(Value::as_vec2).collect(),
        _ => Vec::new(),
    };
    let mut s = Volley {
        samples,
        sample_accum: record.get(SAMPLE_ACCUM).map_err(schema_fault)?.as_f32().unwrap_or(0.0),
        fired_this_strike: record.get(FIRED_THIS_STRIKE).map_err(schema_fault)?.as_bool().unwrap_or(false),
        had_seed_sample: record.get(HAD_SEED_SAMPLE).map_err(schema_fault)?.as_bool().unwrap_or(false),
    };
    let mut fire = Vec::new();
    if !caster.alive {
        s = Volley { samples: Vec::new(), sample_accum: 0.0, fired_this_strike: false, had_seed_sample: false };
    } else if caster.telegraphing {
        if !s.had_seed_sample {
            s.samples.extend(caster.target);
            s.had_seed_sample = true;
            s.sample_accum = 0.0;
        }
        s.sample_accum += dt;
        while s.sample_accum >= SAMPLE_INTERVAL_S {
            s.sample_accum -= SAMPLE_INTERVAL_S;
            if s.samples.len() < SAMPLE_COUNT as usize {
                s.samples.extend(caster.target);
            }
        }
        s.fired_this_strike = false;
    } else if caster.pressed {
        if !s.fired_this_strike {
            fire = std::mem::take(&mut s.samples);
            s.fired_this_strike = true;
            s.had_seed_sample = false;
        }
    } else {
        s = Volley { samples: Vec::new(), sample_accum: 0.0, fired_this_strike: false, had_seed_sample: false };
    }

    let record = inv.state(&VOLLEY)?;
    let points = s.samples.iter().map(|p| Value::Vec2(*p)).collect();
    record.set(SAMPLES, Value::Seq(points)).map_err(schema_fault)?;
    record.set(SAMPLE_ACCUM, Value::F32(s.sample_accum)).map_err(schema_fault)?;
    record.set(FIRED_THIS_STRIKE, Value::Bool(s.fired_this_strike)).map_err(schema_fault)?;
    record.set(HAD_SEED_SAMPLE, Value::Bool(s.had_seed_sample)).map_err(schema_fault)?;

    let origin = Vec2::from(caster.position) + Vec2::from(caster.projectile_offset);
    for point in fire {
        let dir = (Vec2::from(point) - origin).normalize_or_zero();
        if dir.length_squared() < 1e-4 {
            continue;
        }
        inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
            origin,
            dir,
            speed: SHOT_SPEED,
            damage: SHOT_DAMAGE,
            max_lifetime: BOLT_LIFETIME,
            half_extent: BOLT_HALF_EXTENT,
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
