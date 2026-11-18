//! What a script sees: one neutral value for an invocation, and the
//! conversions that make it.
//!
//! A script backend (a language adapter, such as Rhai's) runs an entry on one
//! [`ScriptValue`]:
//!
//! ```text
//! inv = { tick, dt, trigger, observe = { <alias>: .. }, state = { <schema>: .. }, requests = [] }
//! ```
//!
//! The entry changes `inv.state` and appends `{ port: <alias>, value: .. }` to
//! `inv.requests`; [`invoke_script`] converts both back and stages them on
//! the [`Invocation`], so every check a compiled module meets (declared
//! ports, request limits, record schemas) applies to a script unchanged.
//!
//! Each domain owns how its values look to a script ([`ScriptPort`], beside
//! its [`Port`]); records convert generically, by their schema's field names
//! and kinds. A language adapter converts [`ScriptValue`] to its own values
//! and names no domain type. Numbers are `f32`, the engine's: an adapter that
//! computes in another width does not give the module's results.

use crate::schema::{FieldDecl, FieldKind, Record, StateSchema, Value};
use crate::{Fault, Invocation, ModuleDescriptor, Name, Port, PortKey};

/// A value as a script sees it.
#[derive(Clone, Debug, PartialEq)]
pub enum ScriptValue {
    Unit,
    Bool(bool),
    Int(i64),
    Num(f32),
    Text(String),
    List(Vec<ScriptValue>),
    /// Named fields, in a fixed order: the order a value was built in.
    Map(Vec<(String, ScriptValue)>),
}

impl ScriptValue {
    /// The value as a script would write it (`()`, `0.0`, `"text"`,
    /// `[1.0, 2.0]`, `#{ a: 1 }`): how a reference shows a default.
    pub fn literal(&self) -> String {
        match self {
            ScriptValue::Unit => "()".into(),
            ScriptValue::Bool(v) => v.to_string(),
            ScriptValue::Int(v) => v.to_string(),
            ScriptValue::Num(v) => format!("{v:?}"),
            ScriptValue::Text(v) => crate::structure::json_str(v),
            ScriptValue::List(items) => {
                format!("[{}]", items.iter().map(ScriptValue::literal).collect::<Vec<_>>().join(", "))
            }
            ScriptValue::Map(fields) => format!(
                "#{{ {} }}",
                fields.iter().map(|(k, v)| format!("{k}: {}", v.literal())).collect::<Vec<_>>().join(", ")
            ),
        }
    }

    pub fn map<const N: usize>(fields: [(&str, ScriptValue); N]) -> Self {
        Self::Map(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }

    pub fn vec2(v: [f32; 2]) -> Self {
        Self::List(vec![Self::Num(v[0]), Self::Num(v[1])])
    }

    pub fn opt(v: Option<Self>) -> Self {
        v.unwrap_or(Self::Unit)
    }

    /// Field `name` of a map; absent, or not a map, is an error naming it.
    pub fn field(&self, name: &str) -> Result<&ScriptValue, String> {
        self.field_opt(name)?.ok_or_else(|| format!("no field `{name}`"))
    }

    /// Field `name` of a map, or `None` when it is absent or unit.
    pub fn field_opt(&self, name: &str) -> Result<Option<&ScriptValue>, String> {
        match self {
            Self::Map(fields) => Ok(fields
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v)
                .filter(|v| **v != Self::Unit)),
            other => Err(format!("expected a table with `{name}`, got {}", other.kind())),
        }
    }

    pub fn as_num(&self) -> Result<f32, String> {
        match self {
            Self::Num(v) => Ok(*v),
            Self::Int(v) => Ok(*v as f32),
            other => Err(format!("expected a number, got {}", other.kind())),
        }
    }

    pub fn as_int(&self) -> Result<i64, String> {
        match self {
            Self::Int(v) => Ok(*v),
            Self::Num(v) if v.fract() == 0.0 && v.is_finite() => Ok(*v as i64),
            other => Err(format!("expected a whole number, got {}", other.kind())),
        }
    }

    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Self::Bool(v) => Ok(*v),
            other => Err(format!("expected true or false, got {}", other.kind())),
        }
    }

    pub fn as_text(&self) -> Result<&str, String> {
        match self {
            Self::Text(v) => Ok(v),
            other => Err(format!("expected text, got {}", other.kind())),
        }
    }

    pub fn as_list(&self) -> Result<&[ScriptValue], String> {
        match self {
            Self::List(v) => Ok(v),
            other => Err(format!("expected a list, got {}", other.kind())),
        }
    }

    pub fn as_vec2(&self) -> Result<[f32; 2], String> {
        match self.as_list()? {
            [x, y] => Ok([x.as_num()?, y.as_num()?]),
            other => Err(format!("expected [x, y], got a list of {}", other.len())),
        }
    }

    /// Field `name` as a number (`inv.trigger.field("speed")` and the like).
    pub fn num(&self, name: &str) -> Result<f32, String> {
        self.field(name)?.as_num().map_err(|e| format!("`{name}`: {e}"))
    }

    pub fn num_or(&self, name: &str, default: f32) -> Result<f32, String> {
        self.field_opt(name)?.map_or(Ok(default), |v| v.as_num().map_err(|e| format!("`{name}`: {e}")))
    }

    pub fn point(&self, name: &str) -> Result<[f32; 2], String> {
        self.field(name)?.as_vec2().map_err(|e| format!("`{name}`: {e}"))
    }

    pub fn text(&self, name: &str) -> Result<&str, String> {
        self.field(name)?.as_text().map_err(|e| format!("`{name}`: {e}"))
    }

    /// The kind's name, for diagnostics.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Unit => "nothing",
            Self::Bool(_) => "true/false",
            Self::Int(_) => "a whole number",
            Self::Num(_) => "a number",
            Self::Text(_) => "text",
            Self::List(_) => "a list",
            Self::Map(_) => "a table",
        }
    }
}

/// How a port's value looks to a script. The domain that owns the port
/// implements it beside the port.
pub trait ScriptPort: Port {
    /// The value as a script reads it (a trigger, an observation).
    fn to_script(value: &Self::Value) -> ScriptValue;

    /// The value from what a script wrote (a request). A trigger or an
    /// observation is the host's to give, not a script's to write.
    fn from_script(_value: &ScriptValue) -> Result<Self::Value, String> {
        Err(format!("port {} is not written by a script", Self::KEY))
    }

    /// The value's structural description, when the port has one
    /// ([`crate::structure`]; [`crate::script_port!`] gives it).
    fn shape() -> Option<crate::structure::Shape> {
        None
    }
}

/// One port as a script backend reaches it.
#[derive(Clone)]
pub struct ScriptPortEntry {
    pub key: PortKey,
    /// The name scripts use (`projectile`, `sound`).
    pub alias: Name,
    pub trigger: fn(&Invocation<'_>) -> Result<ScriptValue, Fault>,
    pub observe: fn(&Invocation<'_>) -> Result<ScriptValue, Fault>,
    pub submit: fn(&mut Invocation<'_>, &ScriptValue) -> Result<(), String>,
    /// The value a script wrote, checked and in its full form (defaults
    /// filled), or the error naming the field: what a backend calls where the
    /// script makes the request, so the error carries the script's position.
    pub check: fn(&ScriptValue) -> Result<ScriptValue, String>,
    pub role: crate::PortRole,
    pub shape: Option<crate::structure::Shape>,
}

fn trigger_of<P: ScriptPort>(inv: &Invocation<'_>) -> Result<ScriptValue, Fault> {
    Ok(P::to_script(inv.trigger::<P>()?))
}

fn observe_of<P: ScriptPort>(inv: &Invocation<'_>) -> Result<ScriptValue, Fault> {
    Ok(P::to_script(inv.observe::<P>()?))
}

fn submit_of<P: ScriptPort>(inv: &mut Invocation<'_>, value: &ScriptValue) -> Result<(), String> {
    let value = P::from_script(value)?;
    inv.submit::<P>(value).map_err(|e| e.to_string())
}

fn check_of<P: ScriptPort>(value: &ScriptValue) -> Result<ScriptValue, String> {
    P::from_script(value).map(|v| P::to_script(&v))
}

/// The ports a composition makes reachable from scripts.
#[derive(Clone, Default)]
pub struct ScriptPorts {
    entries: Vec<ScriptPortEntry>,
}

impl ScriptPorts {
    /// Make port `P` reachable as `alias`. A second alias or key is refused.
    pub fn add<P: ScriptPort>(&mut self, alias: &'static str) -> &mut Self {
        assert!(
            !self.entries.iter().any(|e| e.key == P::KEY || e.alias == alias),
            "script port {} or alias {alias} is already added",
            P::KEY
        );
        self.entries.push(ScriptPortEntry {
            key: P::KEY,
            alias: alias.into(),
            trigger: trigger_of::<P>,
            observe: observe_of::<P>,
            submit: submit_of::<P>,
            check: check_of::<P>,
            role: P::ROLE,
            shape: P::shape(),
        });
        self
    }

    pub fn by_key(&self, key: &PortKey) -> Option<&ScriptPortEntry> {
        self.entries.iter().find(|e| e.key == *key)
    }

    pub fn by_alias(&self, alias: &str) -> Option<&ScriptPortEntry> {
        self.entries.iter().find(|e| e.alias == alias)
    }

    pub fn entries(&self) -> &[ScriptPortEntry] {
        &self.entries
    }

    /// The author's reference to these ports, as Markdown: each alias, its
    /// port, its role, and its value's fields with their kinds, defaults and
    /// documentation. Generated from the ports' own descriptions.
    pub fn reference_markdown(&self) -> String {
        let mut out = String::new();
        for entry in &self.entries {
            let role = match entry.role {
                crate::PortRole::Trigger => "trigger: read as `this.trigger`",
                crate::PortRole::Observation => "observation: read as `this.observe.<alias>`",
                crate::PortRole::Request => "request: `this.requests.push(request(\"<alias>\", #{ .. }))`",
            };
            out.push_str(&format!("## `{}`\n\nPort `{}`, {role}.\n\n", entry.alias, entry.key));
            match &entry.shape {
                Some(crate::structure::Shape::Record(fields)) if fields.is_empty() => {
                    out.push_str(&format!("No fields: `request(\"{}\", #{{}})`.\n", entry.alias));
                }
                Some(crate::structure::Shape::Record(fields)) => {
                    // Only a script writes a request, so only a request has
                    // fields it may leave out.
                    let written = entry.role == crate::PortRole::Request;
                    out.push_str(if written {
                        "| field | value | when absent | meaning |\n|---|---|---|---|\n"
                    } else {
                        "| field | value | meaning |\n|---|---|---|\n"
                    });
                    for f in fields {
                        let doc = f.doc.split_whitespace().collect::<Vec<_>>().join(" ").replace('|', "\\|");
                        let shape = f.shape.summary().replace('|', "\\|");
                        if written {
                            let absent = f.default.as_ref().map_or("required".to_owned(), |d| format!("`{d}`"));
                            out.push_str(&format!("| `{}` | {shape} | {absent} | {doc} |\n", f.name));
                        } else {
                            out.push_str(&format!("| `{}` | {shape} | {doc} |\n", f.name));
                        }
                    }
                }
                Some(shape) => out.push_str(&format!("Value: {}\n", shape.summary())),
                None => out.push_str("No structural description yet.\n"),
            }
            out.push('\n');
        }
        out
    }

    /// The same reference as JSON: `[{ alias, port, version, role, shape }]`.
    pub fn reference_json(&self) -> String {
        let rows: Vec<String> = self
            .entries
            .iter()
            .map(|e| {
                format!(
                    "{{\"alias\":{},\"port\":{},\"version\":{},\"role\":{},\"shape\":{}}}",
                    crate::structure::json_str(&e.alias),
                    crate::structure::json_str(&e.key.name),
                    e.key.version,
                    crate::structure::json_str(&format!("{:?}", e.role)),
                    e.shape.as_ref().map_or("null".to_owned(), |s| s.to_json())
                )
            })
            .collect();
        format!("[{}]", rows.join(","))
    }
}

/// The schema's name in a script: the last segment of its key
/// (`sentry.cadence` is `inv.state.cadence`).
pub fn state_name(schema: &StateSchema) -> &str {
    schema.key.key.rsplit('.').next().unwrap_or(&schema.key.key)
}

fn value_to_script(kind: &FieldKind, value: &Value) -> Result<ScriptValue, String> {
    Ok(match (kind, value) {
        (FieldKind::Bool, Value::Bool(b)) => ScriptValue::Bool(*b),
        (FieldKind::U32, Value::U32(v)) => ScriptValue::Int(i64::from(*v)),
        (FieldKind::I32, Value::I32(v)) => ScriptValue::Int(i64::from(*v)),
        (FieldKind::U64, Value::U64(v)) => <u64 as crate::structure::Field<u64>>::to_script(v),
        (FieldKind::F32, Value::F32(v)) => ScriptValue::Num(*v),
        (FieldKind::Vec2, Value::Vec2(v)) => ScriptValue::vec2(*v),
        (FieldKind::Opt(_), Value::Opt(None)) => ScriptValue::Unit,
        (FieldKind::Opt(inner), Value::Opt(Some(v))) => value_to_script(inner, v)?,
        (FieldKind::Seq { elem, .. }, Value::Seq(items)) => ScriptValue::List(
            items.iter().map(|v| value_to_script(elem, v)).collect::<Result<_, _>>()?),
        (FieldKind::Ref(_), Value::Ref(id)) | (FieldKind::Text { .. }, Value::Text(id)) => ScriptValue::Text(id.clone()),
        (FieldKind::Record(fields), Value::Record(record)) => fields_to_script(fields, record)?,
        _ => return Err(format!("value does not match {kind:?}")),
    })
}

/// Read a typed state value. Record fields must match the declaration exactly.
pub fn value_from_script(kind: &FieldKind, v: &ScriptValue) -> Result<Value, String> {
    let int = |v: &ScriptValue| v.as_int();
    Ok(match kind {
        FieldKind::Bool => Value::Bool(v.as_bool()?),
        FieldKind::U32 => Value::U32(u32::try_from(int(v)?).map_err(|_| "out of range for u32".to_owned())?),
        FieldKind::I32 => Value::I32(i32::try_from(int(v)?).map_err(|_| "out of range for i32".to_owned())?),
        FieldKind::U64 => Value::U64(<u64 as crate::structure::Field<u64>>::from_script(v)?),
        FieldKind::F32 => Value::F32(v.as_num()?),
        FieldKind::Vec2 => Value::Vec2(v.as_vec2()?),
        FieldKind::Opt(inner) => Value::Opt(match v {
            ScriptValue::Unit => None,
            v => Some(Box::new(value_from_script(inner, v)?)),
        }),
        FieldKind::Seq { elem, .. } => {
            Value::Seq(v.as_list()?.iter().enumerate().map(|(i, v)|
                value_from_script(elem, v).map_err(|e| format!("[{i}]: {e}"))).collect::<Result<_, _>>()?)
        }
        FieldKind::Ref(_) => Value::Ref(v.as_text()?.to_owned()),
        FieldKind::Text { .. } => Value::Text(v.as_text()?.to_owned()),
        FieldKind::Record(fields) => Value::Record(fields_from_script(fields, v)?),
    })
}

pub fn record_to_script(schema: &StateSchema, record: &Record) -> Result<ScriptValue, String> {
    schema.check(record).map_err(|e| e.to_string())?;
    fields_to_script(&schema.fields, record)
}

fn fields_to_script(fields: &[FieldDecl], record: &Record) -> Result<ScriptValue, String> {
    Ok(ScriptValue::Map(fields.iter().zip(record.values())
        .map(|(f, v)| Ok((f.name.to_string(), value_to_script(&f.kind, v)?)))
        .collect::<Result<_, String>>()?))
}

fn fields_from_script(declared: &[FieldDecl], v: &ScriptValue) -> Result<Record, String> {
    let ScriptValue::Map(fields) = v else {
        return Err(format!("expected a table, got {}", v.kind()));
    };
    for (i, (name, _)) in fields.iter().enumerate() {
        if !declared.iter().any(|f| f.name == name.as_str()) {
            return Err(format!("{name}: the schema declares no such field"));
        }
        if fields[..i].iter().any(|(other, _)| other == name) {
            return Err(format!("{name}: duplicate field"));
        }
    }
    Ok(Record::from_values(declared.iter().map(|field| {
        let v = fields.iter().find(|(k, _)| k.as_str() == field.name.as_ref())
            .map(|(_, v)| v).ok_or_else(|| format!("{}: missing", field.name))?;
        value_from_script(&field.kind, v).map_err(|e| format!("{}: {e}", field.name))
    }).collect::<Result<_, _>>()?))
}

/// A record from a script's table. A field the schema does not declare, or a
/// declared field the table lacks, is an error naming it: a misspelt field
/// must not pass silently. The schema's own check (finite floats, sequence
/// bounds) still runs when the host takes the record back.
pub fn record_from_script(schema: &StateSchema, v: &ScriptValue) -> Result<Record, String> {
    fields_from_script(&schema.fields, v).map_err(|e| format!("state.{}.{}", state_name(schema), e))
}

/// Run entry `entry` of `module` from a script: the SDK decodes the
/// invocation (as for a compiled guest), `call` runs the script's entry on
/// the neutral `inv`, and the changed state and the requests are staged. Any
/// error (a script fault, a misspelt field, an undeclared port) is a fault
/// output, prefixed with the module and entry, and nothing is staged.
pub fn invoke_script(
    module: &ModuleDescriptor,
    entry: u32,
    input: &[u8],
    ports: &ScriptPorts,
    call: impl FnOnce(&str, ScriptValue) -> Result<ScriptValue, String>,
) -> Vec<u8> {
    crate::abi::invoke_with(module, entry, input, |inv| {
        let descriptor = &module.entries[entry as usize];
        let fault = |message: String| Fault::Module(format!("{}.{}: {message}", module.key, descriptor.key).into());
        let trigger_port = &descriptor.trigger.port;
        let trigger = ports
            .by_key(trigger_port)
            .ok_or_else(|| fault(format!("the trigger port {trigger_port} is not reachable from scripts")))?;
        let trigger = (trigger.trigger)(inv)?;
        let mut observe = Vec::new();
        for port in &descriptor.reads {
            let entry = ports
                .by_key(port)
                .ok_or_else(|| fault(format!("the observation port {port} is not reachable from scripts")))?;
            observe.push((entry.alias.to_string(), (entry.observe)(inv)?));
        }
        let writes: Vec<&StateSchema> =
            module.schemas.iter().filter(|schema| descriptor.writes.contains(&schema.key)).collect();
        let mut state = Vec::new();
        for schema in &writes {
            let record = inv.state(&schema.key)?.clone();
            state.push((state_name(schema).to_owned(), record_to_script(schema, &record).map_err(fault)?));
        }
        let inv_value = ScriptValue::Map(vec![
            ("tick".into(), ScriptValue::Int(inv.tick() as i64)),
            ("dt".into(), ScriptValue::Num(inv.dt())),
            ("trigger".into(), trigger),
            ("observe".into(), ScriptValue::Map(observe)),
            ("state".into(), ScriptValue::Map(state)),
            ("requests".into(), ScriptValue::List(Vec::new())),
        ]);
        let out = call(&descriptor.key, inv_value).map_err(fault)?;
        let out_state = out.field("state").map_err(fault)?;
        for schema in &writes {
            let v = out_state.field(state_name(schema)).map_err(|e| fault(format!("state: {e}")))?;
            let record = record_from_script(schema, v).map_err(fault)?;
            *inv.state(&schema.key)? = record;
        }
        let requests = match out.field_opt("requests").map_err(fault)? {
            None => &[][..],
            Some(list) => list.as_list().map_err(|e| fault(format!("requests: {e}")))?,
        };
        for (n, request) in requests.iter().enumerate() {
            let alias = request.text("port").map_err(|e| fault(format!("request {n}: {e}")))?;
            let port = ports
                .by_alias(alias)
                .ok_or_else(|| fault(format!("request {n}: no port named `{alias}` is reachable from scripts")))?;
            let value = request.field("value").map_err(|e| fault(format!("request {n}: {e}")))?;
            (port.submit)(inv, value).map_err(|e| fault(format!("request {n} ({alias}): {e}")))?;
        }
        Ok(())
    })
}
