//! The scripting-language bake-off: the sentry module's turret entry in Rhai
//! and in Luau, each behind the extension host's backend contract
//! (`ModuleBackend::invoke`: invocation bytes in, checked output bytes out),
//! compared with the native sentry on the same invocations.
//!
//! The binding is shared, so a difference is the language's: the script sees
//! one neutral value (JSON-shaped) for its invocation,
//!
//! ```text
//! inv = { tick, dt, trigger = {...}, state = { <schema>: {...} }, requests = [] }
//! ```
//!
//! changes `inv.state` and appends `{ port, value }` to `inv.requests`, and the
//! host converts both back. Records convert generically, by their schema's
//! field names and kinds; ports convert by a hand-written converter each (the
//! audit's P2 would generate those from the port's own description). The
//! invocation itself is decoded and encoded by the SDK
//! (`abi::invoke_with`), exactly as a compiled guest's is, so a fault leaves
//! nothing staged.

use ambition_combat_port::{BodySound, BodySoundPort, ModuleEntityTick, ModuleEntityTickPort, Place};
use ambition_extension_sdk::schema::{FieldKind, Record, StateSchema, Value};
use ambition_extension_sdk::{Fault, Invocation, ModuleDescriptor, Port};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;
use serde_json::{json, Value as J};

#[cfg(feature = "rhai")]
pub mod rhai_backend;
#[cfg(feature = "luau")]
pub mod luau_backend;

/// The extension host's backend contract (`ambition_extension_host::ModuleBackend`),
/// mirrored so the fixture does not link the Bevy host. A backend here is a
/// backend there: the signature is the same.
pub trait Backend {
    fn invoke(&self, module: u32, entry: u32, input: &[u8]) -> Vec<u8>;
}

/// What a language adapter provides: run entry `entry` on the neutral `inv`
/// and give back the changed `inv`, or the script's error.
pub trait ScriptCall {
    fn call(&self, entry: &str, inv: J) -> Result<J, String>;
}

/// The schema's short name in a script: the last segment of its key
/// (`sentry.cadence` is `inv.state.cadence`).
pub fn short_name(schema: &StateSchema) -> &str {
    schema.key.key.rsplit('.').next().unwrap_or(&schema.key.key)
}

fn value_to_json(value: &Value) -> J {
    match value {
        Value::Bool(b) => json!(b),
        Value::U32(v) => json!(v),
        Value::I32(v) => json!(v),
        Value::U64(v) => json!(v),
        Value::F32(v) => json!(v),
        Value::Vec2([x, y]) => json!([x, y]),
        Value::Opt(None) => J::Null,
        Value::Opt(Some(v)) => value_to_json(v),
        Value::Seq(items) => J::Array(items.iter().map(value_to_json).collect()),
        Value::Ref(id) | Value::Text(id) => json!(id),
        // The bake-off's scripts hold scalars, lists and options only.
        Value::Record(_) | Value::Object(_) => panic!("the bake-off passes no nested record or object to a script"),
    }
}

fn number(j: &J, what: &str) -> Result<f64, String> {
    j.as_f64().ok_or_else(|| format!("{what}: expected a number, got {j}"))
}

fn json_to_value(kind: &FieldKind, j: &J, what: &str) -> Result<Value, String> {
    Ok(match kind {
        FieldKind::Bool => Value::Bool(j.as_bool().ok_or_else(|| format!("{what}: expected a bool, got {j}"))?),
        FieldKind::U32 => Value::U32(number(j, what)? as u32),
        FieldKind::I32 => Value::I32(number(j, what)? as i32),
        FieldKind::U64 => Value::U64(number(j, what)? as u64),
        FieldKind::F32 => Value::F32(number(j, what)? as f32),
        FieldKind::Vec2 => Value::Vec2(vec2(j, what)?),
        FieldKind::Opt(inner) => Value::Opt(match j {
            J::Null => None,
            j => Some(Box::new(json_to_value(inner, j, what)?)),
        }),
        FieldKind::Seq { elem, .. } => Value::Seq(
            j.as_array()
                .ok_or_else(|| format!("{what}: expected a list, got {j}"))?
                .iter()
                .map(|j| json_to_value(elem, j, what))
                .collect::<Result<_, _>>()?,
        ),
        FieldKind::Ref(_) => Value::Ref(j.as_str().ok_or_else(|| format!("{what}: expected an identity, got {j}"))?.to_string()),
        FieldKind::Text { .. } => Value::Text(j.as_str().ok_or_else(|| format!("{what}: expected text, got {j}"))?.to_string()),
        other => return Err(format!("{what}: the bake-off takes no {other:?} field from a script")),
    })
}

fn vec2(j: &J, what: &str) -> Result<[f32; 2], String> {
    match j.as_array().map(Vec::as_slice) {
        Some([x, y]) => Ok([number(x, what)? as f32, number(y, what)? as f32]),
        _ => Err(format!("{what}: expected [x, y], got {j}")),
    }
}

pub fn record_to_json(schema: &StateSchema, record: &Record) -> J {
    let mut map = serde_json::Map::new();
    for (field, value) in schema.fields.iter().zip(record.values()) {
        map.insert(field.name.to_string(), value_to_json(value));
    }
    J::Object(map)
}

/// A record from a script's table. A field the schema does not declare, or a
/// declared field the table lacks, is an error naming it: a misspelt field
/// must not pass silently.
pub fn json_to_record(schema: &StateSchema, j: &J) -> Result<Record, String> {
    let name = short_name(schema);
    let map = j.as_object().ok_or_else(|| format!("state.{name}: expected a table, got {j}"))?;
    if let Some(stray) = map.keys().find(|key| !schema.fields.iter().any(|field| field.name == key.as_str())) {
        return Err(format!("state.{name}.{stray}: the schema {} declares no such field", schema.key.key));
    }
    let mut record = schema.initial_record();
    for (index, field) in schema.fields.iter().enumerate() {
        let what = format!("state.{name}.{}", field.name);
        let j = map.get(field.name.as_ref()).ok_or_else(|| format!("{what}: missing"))?;
        record
            .set(ambition_extension_sdk::schema::FieldRef(index), json_to_value(&field.kind, j, &what)?)
            .map_err(|e| format!("{what}: {e:?}"))?;
    }
    Ok(record)
}

fn trigger_json(inv: &Invocation<'_>, module: &ModuleDescriptor, entry: u32) -> Result<J, Fault> {
    let port = &module.entries[entry as usize].trigger.port;
    if *port == ModuleEntityTickPort::KEY {
        let tick: &ModuleEntityTick = inv.trigger::<ModuleEntityTickPort>()?;
        return Ok(json!({
            "position": tick.position,
            "remaining_s": tick.remaining_s,
            "nearest_enemy": tick.nearest_enemy,
        }));
    }
    Err(Fault::Module(format!("the bake-off binds no trigger {port}").into()))
}

fn submit(inv: &mut Invocation<'_>, request: &J) -> Result<(), String> {
    let port = request.get("port").and_then(J::as_str).ok_or("a request needs a port name")?;
    let v = request.get("value").ok_or("a request needs a value")?;
    let field = |name: &str| v.get(name).ok_or_else(|| format!("request {port}: no field {name}"));
    let fault = |e: Fault| e.to_string();
    match port {
        "projectile" => inv
            .submit::<ProjectileSpawnPort>(ProjectileSpawn {
                reply: None,
                properties: Default::default(),
                guard: Default::default(),
                origin: Vec2::from(vec2(field("origin")?, "origin")?),
                dir: Vec2::from(vec2(field("dir")?, "dir")?),
                speed: number(field("speed")?, "speed")? as f32,
                damage: number(field("damage")?, "damage")? as i32,
                max_lifetime: number(field("max_lifetime")?, "max_lifetime")? as f32,
                half_extent: Vec2::from(vec2(field("half_extent")?, "half_extent")?),
                gravity: 0.0,
                visual_id: String::new(),
                bounces: 0,
                bounce_on_world_contact: false,
                splash_half_extent: 0.0,
                boomerang_return_s: None,
            })
            .map_err(fault),
        "sound" => inv
            .submit::<BodySoundPort>(BodySound {
                cue: field("cue")?.as_str().ok_or("sound: cue is not text")?.to_owned(),
                at: Place::World(vec2(field("at")?, "at")?),
            })
            .map_err(fault),
        other => Err(format!("no request port named {other:?}")),
    }
}

/// Run one entry of `module` from a script: the SDK decodes the invocation,
/// the script runs on the neutral value, the host converts its state and
/// requests back, and the SDK encodes them. Any error is a fault output.
pub fn invoke_script(module: &ModuleDescriptor, entry: u32, input: &[u8], script: &dyn ScriptCall) -> Vec<u8> {
    ambition_extension_sdk::abi::invoke_with(module, entry, input, |inv| {
        let descriptor = &module.entries[entry as usize];
        let mut state = serde_json::Map::new();
        let writes: Vec<&StateSchema> =
            module.schemas.iter().filter(|schema| descriptor.writes.contains(&schema.key)).collect();
        for schema in &writes {
            let record = inv.state(&schema.key)?.clone();
            state.insert(short_name(schema).to_owned(), record_to_json(schema, &record));
        }
        let input = json!({
            "tick": inv.tick().expect("a simulation entry"),
            "dt": inv.dt(),
            "trigger": trigger_json(inv, module, entry)?,
            "state": state,
            "requests": [],
        });
        let module_fault = |message: String| Fault::Module(format!("{}.{}: {message}", module.key, descriptor.key).into());
        let out = script.call(&descriptor.key, input).map_err(module_fault)?;
        for schema in &writes {
            let j = out.get("state").and_then(|state| state.get(short_name(schema))).cloned().unwrap_or(J::Null);
            let record = json_to_record(schema, &j).map_err(module_fault)?;
            *inv.state(&schema.key)? = record;
        }
        for request in out.get("requests").and_then(J::as_array).into_iter().flatten() {
            submit(inv, request).map_err(module_fault)?;
        }
        Ok(())
    })
}
