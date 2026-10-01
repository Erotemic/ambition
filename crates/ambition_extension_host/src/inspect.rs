//! Text views of the extension host, for a developer or an agent that has no
//! access to the `World` (fast-iteration I7 item 5).
//!
//! [`describe_composition`] says which ports are installed, which entries run
//! in which order with which code, what each one reads, writes and requests,
//! which modules were replaced, and the generation text. [`describe_records`]
//! says what each body's records hold now, by field name. Both are
//! deterministic: the same world gives the same text.

use std::fmt::Write as _;

use bevy::prelude::*;

use crate::{AdmittedExtensions, BodyRecords, EntryRunner, ExtensionGeneration, RecordSet, SessionRecords};

/// The admitted composition as text. Before admission, says so.
pub fn describe_composition(world: &World) -> String {
    let mut out = String::new();
    let Some(admitted) = world.get_resource::<AdmittedExtensions>() else {
        let _ = writeln!(out, "extension host: not admitted yet");
        if let Some(generation) = world.get_resource::<ExtensionGeneration>() {
            let _ = write!(out, "declared:\n{}", generation.0);
        }
        return out;
    };
    let admitted = &admitted.0;
    let _ = writeln!(out, "admitted digest {:016x}", admitted.digest);

    let mut offers: Vec<_> = admitted.offers.iter().collect();
    offers.sort_by(|a, b| (&a.phase, &a.key).cmp(&(&b.phase, &b.key)));
    let _ = writeln!(out, "ports ({}):", offers.len());
    for offer in offers {
        let _ = writeln!(
            out,
            "  {} {:?} in {} (owner {})",
            offer.key, offer.role, offer.phase.0, offer.owner
        );
    }

    let _ = writeln!(out, "entries in serial order ({}):", admitted.entries.len());
    for entry in &admitted.entries {
        let d = &entry.descriptor;
        let runner = match &entry.runner {
            EntryRunner::Native(_) => "native".to_owned(),
            EntryRunner::Loaded { module, entry, .. } => format!("loaded (module {module}, entry {entry})"),
        };
        let _ = writeln!(out, "  {} [{runner}]", entry.path);
        let _ = writeln!(
            out,
            "    phase {}; trigger {} {:?}; idle {:?}; max {} requests",
            d.phase.0, d.trigger.port, d.trigger.selector.as_ref(), d.on_idle, d.limits.max_requests
        );
        let list = |items: Vec<String>| if items.is_empty() { "-".to_owned() } else { items.join(", ") };
        let _ = writeln!(out, "    reads {}", list(d.reads.iter().map(ToString::to_string).collect()));
        let _ = writeln!(out, "    writes {}", list(d.writes.iter().map(ToString::to_string).collect()));
        let _ = writeln!(out, "    requests {}", list(d.requests.iter().map(ToString::to_string).collect()));
    }
    if !admitted.replaced.is_empty() {
        let _ = writeln!(out, "replaced: {}", admitted.replaced.join(", "));
    }
    if let Some(generation) = world.get_resource::<ExtensionGeneration>() {
        let _ = write!(out, "generation:\n{}", generation.0);
    }
    out
}

/// Every body's records, then the session's, by schema and field name,
/// ordered by entity.
pub fn describe_records(world: &mut World) -> String {
    let schemas = world
        .get_resource::<AdmittedExtensions>()
        .map(|a| a.0.schemas.clone())
        .unwrap_or_default();
    let mut bodies: Vec<(Entity, RecordSet)> = world
        .query::<(Entity, &BodyRecords)>()
        .iter(world)
        .map(|(e, r)| (e, r.0.clone()))
        .collect();
    bodies.sort_by_key(|(e, _)| e.to_bits());
    let sessions: Vec<(Entity, RecordSet)> = world
        .query::<(Entity, &SessionRecords)>()
        .iter(world)
        .map(|(e, r)| (e, r.0.clone()))
        .collect();
    let mut out = String::new();
    let _ = writeln!(out, "bodies with records: {}", bodies.len());
    for (entity, records) in bodies {
        write_records(world, &schemas, &mut out, entity, &records);
    }
    let _ = writeln!(out, "session records: {}", sessions.len());
    for (entity, records) in sessions {
        write_records(world, &schemas, &mut out, entity, &records);
    }
    out
}

fn write_records(
    world: &World,
    schemas: &std::collections::BTreeMap<ambition_extension_sdk::SchemaKey, crate::admission::AdmittedSchema>,
    out: &mut String,
    entity: Entity,
    records: &RecordSet,
) {
    let name = world.get::<Name>(entity).map(|n| format!(" {n}")).unwrap_or_default();
    let _ = writeln!(out, "  {entity}{name}");
    for stored in records.records() {
        let _ = write!(out, "    {}:", stored.key);
        match schemas.get(&stored.key) {
            Some(schema) => {
                for (i, field) in schema.schema.fields.iter().enumerate() {
                    let value = stored
                        .record
                        .get(ambition_extension_sdk::FieldRef(i))
                        .map(|v| format!("{v:?}"))
                        .unwrap_or_else(|e| format!("<{e:?}>"));
                    let _ = write!(out, " {}={value}", field.name);
                }
            }
            None => {
                let _ = write!(out, " <schema not admitted>");
            }
        }
        let _ = writeln!(out);
    }
}
