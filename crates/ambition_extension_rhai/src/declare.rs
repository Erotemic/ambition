//! A module a script declares itself (scripting audit P3, step 3): a module
//! the game does not link, with its whole declaration in the script.
//!
//! The script defines `fn declare()` (`module` is a word Rhai reserves),
//! which returns the declaration as a
//! table. It is called once, when the file loads, under the same limits as
//! an entry; its result is the module's descriptor, and admission checks it
//! as it checks any module's.
//!
//! ```text
//! fn declare() {
//!     #{
//!         provider: "drill",
//!         key: "turret_tally",
//!         schemas: [
//!             #{ key: "turret_tally.count", attachment: "body",
//!                fields: [ #{ tag: 1, name: "ticks", kind: "u32" } ] },
//!         ],
//!         entries: [
//!             #{ key: "tally", phase: "module_entity_tick",
//!                trigger: #{ port: "module_entity_tick", selector: "sentry" },
//!                writes: ["turret_tally.count"], requests: [], limit: 1 },
//!         ],
//!     }
//! }
//!
//! fn tally() { ... }
//! ```
//!
//! A field's `kind` is a scalar (`bool`, `u32`, `i32`, `u64`, `f32`,
//! `vec2`), a reference to a body or an item by its identity (`body`,
//! `item`), `opt <kind>`, or `seq <max length> <kind>` (`seq 16 opt item`).
//!
//! Ports are named as scripts name them (`projectile`, `sound`,
//! `module_entity_tick`: docs/sdk/script-ports.md); a schema is named by its
//! key, and its provider and the module's are one. Defaults: a schema is
//! body-attached, transient, revision 1; an entry's limit is 8 requests and
//! it is called on an idle trigger (`idle: "reset"` resets its records
//! instead). Every entry the declaration names must be a function of the
//! script: a declared module has no native code to fall back on.

use ambition_extension_sdk::script::ScriptPorts;
use ambition_extension_sdk::{
    Attachment, CodeIdentity, EntryCode, EntryDescriptor, FieldDecl, FieldKind, IdlePolicy, Limits, ModuleDescriptor,
    ModuleKey, Name, Phase, PortKey, SaveEligibility, SchemaKey, StateSchema, TriggerBinding, Value, API_VERSION,
};
use rhai::{Dynamic, Map};

/// The phases a declaration may name.
const PHASES: &[Phase] = &[
    ambition_extension_sdk::phases::CAMERA_POLICY,
    ambition_extension_sdk::phases::TECHNIQUE_EXECUTION,
    ambition_extension_sdk::phases::WIELDED_USE,
    ambition_extension_sdk::phases::MODULE_ENTITY_TICK,
    ambition_extension_sdk::phases::BOSS_CONDUCT,
    ambition_extension_sdk::phases::ACTOR_THINK,
    ambition_extension_sdk::phases::BODY_CONTACT,
    ambition_extension_sdk::phases::HIT_REACTION,
];

/// The descriptor a `declare()` table declares. Each entry's code is the
/// script's (`EntryCode::Loaded`).
pub fn descriptor(table: &Dynamic, ports: &ScriptPorts) -> Result<ModuleDescriptor, String> {
    let table = map(table, "declare()")?;
    let provider = text(&table, "provider", "declare()")?;
    let key = text(&table, "key", "declare()")?;
    let mut schemas = Vec::new();
    for (n, schema) in list(&table, "schemas")?.iter().enumerate() {
        schemas.push(schema_of(schema, &provider, &format!("schemas[{n}]"))?);
    }
    let schema_key = |name: &str, at: &str| -> Result<SchemaKey, String> {
        schemas
            .iter()
            .find(|s: &&StateSchema| s.key.key == name)
            .map(|s| s.key.clone())
            .ok_or_else(|| format!("{at}: \"{name}\" is not one of this module's schemas"))
    };
    let port = |alias: &str, at: &str| -> Result<PortKey, String> {
        ports.by_alias(alias).map(|p| p.key.clone()).ok_or_else(|| {
            let known: Vec<&str> = ports.entries().iter().map(|e| e.alias.as_ref()).collect();
            format!("{at}: no port is named \"{alias}\" for scripts (they are {known:?})")
        })
    };
    let mut entries = Vec::new();
    for (index, entry) in list(&table, "entries")?.iter().enumerate() {
        let at = format!("entries[{index}]");
        let entry = map(entry, &at)?;
        let phase_name = text(&entry, "phase", &at)?;
        let phase = PHASES.iter().find(|p| p.0 == phase_name).cloned().ok_or_else(|| {
            let known: Vec<&str> = PHASES.iter().map(|p| p.0.as_ref()).collect();
            format!("{at}: phase \"{phase_name}\" is not one of {known:?}")
        })?;
        let trigger = map(entry.get("trigger").ok_or_else(|| format!("{at}: no trigger"))?, &format!("{at}.trigger"))?;
        let trigger = TriggerBinding {
            port: port(&text(&trigger, "port", &format!("{at}.trigger"))?, &format!("{at}.trigger"))?,
            selector: Name::Owned(text(&trigger, "selector", &format!("{at}.trigger"))?),
        };
        let names = |field: &str| -> Result<Vec<String>, String> {
            list(&entry, field)?
                .iter()
                .map(|v| v.clone().into_string().map_err(|t| format!("{at}.{field}: expected names, got {t}")))
                .collect()
        };
        let writes = names("writes")?.iter().map(|n| schema_key(n, &format!("{at}.writes"))).collect::<Result<Vec<_>, _>>()?;
        let reads = names("reads")?.iter().map(|n| port(n, &format!("{at}.reads"))).collect::<Result<Vec<_>, _>>()?;
        let requests =
            names("requests")?.iter().map(|n| port(n, &format!("{at}.requests"))).collect::<Result<Vec<_>, _>>()?;
        let after = names("after")?.into_iter().map(Name::Owned).collect();
        let limit = match entry.get("limit") {
            None => 8,
            Some(v) => v.as_int().map_err(|t| format!("{at}.limit: expected a number, got {t}"))?,
        };
        let on_idle = match entry.get("idle").map(|v| v.clone().into_string()) {
            None => IdlePolicy::Invoke,
            Some(Ok(s)) if s == "invoke" => IdlePolicy::Invoke,
            Some(Ok(s)) if s == "reset" => IdlePolicy::ResetState,
            Some(other) => return Err(format!("{at}.idle: expected \"invoke\" or \"reset\", got {other:?}")),
        };
        entries.push(EntryDescriptor {
            key: Name::Owned(text(&entry, "key", &at)?),
            phase,
            trigger,
            reads,
            writes,
            requests,
            after,
            replies: names("replies")?.into_iter().map(Name::Owned).collect(),
            limits: Limits { max_requests: u32::try_from(limit).map_err(|_| format!("{at}.limit: {limit} is out of range"))? },
            on_idle,
            run: EntryCode::Loaded { index: index as u32 },
        });
    }
    if entries.is_empty() {
        return Err("declare(): declares no entries".into());
    }
    Ok(ModuleDescriptor {
        key: ModuleKey { provider: Name::Owned(provider), key: Name::Owned(key) },
        api: API_VERSION,
        code: CodeIdentity::Loaded { abi: crate::ABI.into(), digest: 0 },
        schemas,
        entries,
    })
}

fn schema_of(value: &Dynamic, provider: &str, at: &str) -> Result<StateSchema, String> {
    let table = map(value, at)?;
    let attachment = match optional_text(&table, "attachment", at)?.as_deref() {
        None | Some("body") => Attachment::Body,
        Some("session") => Attachment::Session,
        Some("view") => Attachment::View,
        Some(other) => return Err(format!("{at}.attachment: expected \"body\", \"session\", or \"view\", got \"{other}\"")),
    };
    let save = match optional_text(&table, "save", at)?.as_deref() {
        None | Some("transient") => SaveEligibility::Transient,
        Some("checkpoint") => SaveEligibility::Checkpoint,
        Some("durable") => SaveEligibility::Durable,
        Some(other) => {
            return Err(format!("{at}.save: expected \"transient\", \"checkpoint\" or \"durable\", got \"{other}\""))
        }
    };
    let revision = match table.get("revision") {
        None => 1,
        Some(v) => v.as_int().map_err(|t| format!("{at}.revision: expected a number, got {t}"))?,
    };
    let fields = fields_of(&list(&table, "fields")?, at, 0)?;
    let schema = StateSchema {
        key: SchemaKey {
            provider: Name::Owned(provider.to_owned()),
            key: Name::Owned(text(&table, "key", at)?),
            revision: u32::try_from(revision).map_err(|_| format!("{at}.revision: {revision} is out of range"))?,
        },
        attachment,
        save,
        fields,
    };
    schema.validate().map_err(|e| format!("{at}: {e:?}"))?;
    Ok(schema)
}

fn fields_of(values: &[Dynamic], at: &str, depth: usize) -> Result<Vec<FieldDecl>, String> {
    values.iter().enumerate().map(|(n, value)| {
        let at = format!("{at}.fields[{n}]");
        let field = map(value, &at)?;
        let kind = kind_dynamic(field.get("kind").ok_or_else(|| format!("{at}: no kind"))?, &at, depth)?;
        let tag = field.get("tag").ok_or_else(|| format!("{at}: no tag"))?
            .as_int().map_err(|t| format!("{at}.tag: expected a number, got {t}"))?;
        let initial = match field.get("initial") {
            None => kind.initial(),
            Some(v) => initial_of(v, &kind).map_err(|e| format!("{at}.initial: {e}"))?,
        };
        Ok(FieldDecl {
            tag: u16::try_from(tag).map_err(|_| format!("{at}.tag: {tag} is out of range"))?,
            name: Name::Owned(text(&field, "name", &at)?), kind, initial,
        })
    }).collect()
}

fn kind_dynamic(value: &Dynamic, at: &str, depth: usize) -> Result<FieldKind, String> {
    if depth >= ambition_extension_sdk::MAX_SCHEMA_DEPTH {
        return Err(format!("{at}.kind: too much nesting"));
    }
    if let Ok(text) = value.clone().into_string() {
        return kind_text(&text, depth).map_err(|e| format!("{at}.kind: {e}"));
    }
    let table = map(value, &format!("{at}.kind"))?;
    if table.len() == 1 {
        if let Some(fields) = table.get("record") {
            let fields = fields.clone().into_array().map_err(|t| format!("{at}.kind.record: expected fields, got {t}"))?;
            return Ok(FieldKind::Record(fields_of(&fields, &format!("{at}.kind.record"), depth + 1)?));
        }
        if let Some(inner) = table.get("opt") {
            return Ok(FieldKind::opt(kind_dynamic(inner, &format!("{at}.kind.opt"), depth + 1)?));
        }
    }
    if table.len() == 2 {
        if let (Some(bound), Some(inner)) = (table.get("seq"), table.get("of")) {
            let n = bound.as_int().map_err(|t| format!("{at}.kind.seq: expected a bound, got {t}"))?;
            let max_len = u32::try_from(n).map_err(|_| format!("{at}.kind.seq: {n} is out of range"))?;
            return Ok(FieldKind::seq(kind_dynamic(inner, &format!("{at}.kind.of"), depth + 1)?, max_len));
        }
    }
    Err(format!("{at}.kind: expected text, #{{record: fields}}, #{{opt: kind}}, or #{{seq: bound, of: kind}}"))
}

/// Read a scalar, reference, bounded text, option or bounded sequence kind.
pub fn kind_of(text: &str) -> Result<FieldKind, String> {
    kind_text(text, 0)
}

fn kind_text(text: &str, depth: usize) -> Result<FieldKind, String> {
    fn parse<'a>(words: &mut impl Iterator<Item = &'a str>, depth: usize) -> Result<FieldKind, String> {
        if depth >= ambition_extension_sdk::MAX_SCHEMA_DEPTH { return Err("too much nesting".into()); }
        let word = words.next().ok_or("a kind is missing")?;
        Ok(match word {
            "object" => FieldKind::Object,
            "bool" => FieldKind::Bool,
            "u32" => FieldKind::U32,
            "i32" => FieldKind::I32,
            "u64" => FieldKind::U64,
            "f32" => FieldKind::F32,
            "vec2" => FieldKind::Vec2,
            "body" => FieldKind::Ref(ambition_extension_sdk::RefKind::Body),
            "item" => FieldKind::Ref(ambition_extension_sdk::RefKind::Item),
            "opt" => FieldKind::opt(parse(words, depth + 1)?),
            "text" | "seq" => {
                let len = words.next().ok_or_else(|| format!("`{word}` needs a bound"))?;
                let n = len.parse::<u32>().map_err(|_| format!("`{word} {len}`: the bound is not a number"))?;
                if word == "text" { FieldKind::Text { max_bytes: n } }
                else { FieldKind::seq(parse(words, depth + 1)?, n) }
            }
            other => return Err(format!(
                "\"{other}\" is not one of bool, u32, i32, u64, f32, vec2, body, item, text <bytes>, opt <kind>, seq <n> <kind>")),
        })
    }
    let mut words = text.split_whitespace();
    let kind = parse(&mut words, depth)?;
    match words.next() {
        None => Ok(kind),
        Some(extra) => Err(format!("\"{text}\": \"{extra}\" follows a whole kind")),
    }
}

fn initial_of(value: &Dynamic, kind: &FieldKind) -> Result<Value, String> {
    let wrong = || format!("expected {kind:?}, got {}", value.type_name());
    Ok(match kind {
        FieldKind::Object => ambition_extension_sdk::script::value_from_script(kind, &crate::from_dynamic(value.clone())?)?,
        FieldKind::Bool => Value::Bool(value.as_bool().map_err(|_| wrong())?),
        FieldKind::U32 => Value::U32(u32::try_from(value.as_int().map_err(|_| wrong())?).map_err(|_| wrong())?),
        FieldKind::I32 => Value::I32(i32::try_from(value.as_int().map_err(|_| wrong())?).map_err(|_| wrong())?),
        FieldKind::U64 => Value::U64(value.clone().into_string().map_err(|_| wrong())?.parse().map_err(|_| wrong())?),
        FieldKind::F32 => Value::F32(value.as_float().map_err(|_| wrong())?),
        FieldKind::Vec2 => {
            let items = value.clone().into_array().map_err(|_| wrong())?;
            match items.as_slice() {
                [x, y] => Value::Vec2([x.as_float().map_err(|_| wrong())?, y.as_float().map_err(|_| wrong())?]),
                _ => return Err(wrong()),
            }
        }
        FieldKind::Ref(_) => Value::Ref(value.clone().into_string().map_err(|_| wrong())?),
        FieldKind::Text { .. } => Value::Text(value.clone().into_string().map_err(|_| wrong())?),
        FieldKind::Opt(_) if value.is_unit() => Value::Opt(None),
        FieldKind::Opt(inner) => Value::some(initial_of(value, inner)?),
        FieldKind::Seq { elem, max_len } => {
            let items = value.clone().into_array().map_err(|_| wrong())?;
            if items.len() > *max_len as usize { return Err("sequence exceeds its bound".into()); }
            Value::Seq(items.iter().enumerate().map(|(i, v)| initial_of(v, elem)
                .map_err(|e| format!("[{i}]: {e}"))).collect::<Result<_, _>>()?)
        }
        FieldKind::Record(fields) => {
            let table = map(value, "record")?;
            if let Some(name) = table.keys().find(|name| !fields.iter().any(|f| f.name == name.as_str())) {
                return Err(format!("{name}: no such field"));
            }
            Value::Record(ambition_extension_sdk::Record::from_values(fields.iter().map(|f| {
                match table.get(f.name.as_ref()) {
                    Some(v) => initial_of(v, &f.kind).map_err(|e| format!("{}: {e}", f.name)),
                    None => Ok(f.initial.clone()),
                }
            }).collect::<Result<_, _>>()?))
        }
    })
}

fn map(value: &Dynamic, at: &str) -> Result<Map, String> {
    value.clone().try_cast::<Map>().ok_or_else(|| format!("{at}: expected a table, got {}", value.type_name()))
}

fn text(table: &Map, field: &str, at: &str) -> Result<String, String> {
    optional_text(table, field, at)?.ok_or_else(|| format!("{at}: no {field}"))
}

fn optional_text(table: &Map, field: &str, at: &str) -> Result<Option<String>, String> {
    table
        .get(field)
        .map(|v| v.clone().into_string().map_err(|t| format!("{at}.{field}: expected text, got {t}")))
        .transpose()
}

fn list(table: &Map, field: &str) -> Result<Vec<Dynamic>, String> {
    match table.get(field) {
        None => Ok(Vec::new()),
        Some(v) => v.clone().into_array().map_err(|t| format!("{field}: expected a list, got {t}")),
    }
}


#[cfg(test)]
mod tests {
    use super::kind_of;
    use ambition_extension_sdk::{FieldKind, RefKind};

    #[test]
    fn a_declared_kind_reads_as_its_words_say() {
        assert_eq!(kind_of("u32"), Ok(FieldKind::U32));
        assert_eq!(kind_of("seq 16 opt item"), Ok(FieldKind::seq(FieldKind::opt(FieldKind::Ref(RefKind::Item)), 16)));
        assert_eq!(kind_of("opt body"), Ok(FieldKind::opt(FieldKind::Ref(RefKind::Body))));
        assert!(kind_of("seq body").is_err(), "a sequence states its length");
        assert!(kind_of("u32 u32").is_err(), "nothing follows a whole kind");
        assert!(kind_of("string").is_err());
    }
}
