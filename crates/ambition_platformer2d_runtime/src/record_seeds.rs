//! Script-authored initial records for a body that a native constructor creates.

use std::collections::BTreeSet;
use ambition_extension_host::Admitted;
use ambition_extension_sdk::{Attachment, Record, SchemaKey};
use ambition_world_port::behavior::StateSeed;

/// One checked initial record: its schema, the admitted shape and the value.
pub(crate) type PreparedSeed = (SchemaKey, u64, Record);

/// Resolve, decode and check `seeds` against the schemas that the initializer may write.
///
/// A seed names its schema by the qualified key (`provider::key@revision`) or by the local key.
/// A local key that more than one allowed schema shares is refused as ambiguous, so the result
/// never depends on declaration order. Each schema takes at most one seed and must be
/// body-attached. `bytes` adds the encoded size across calls, so the caller sets the scope of
/// `max_bytes`. Each diagnostic starts with `subject`.
pub(crate) fn prepare_record_seeds<'a, I>(admitted: &Admitted, allowed: I, seeds: &[StateSeed],
    bytes: &mut usize, max_bytes: u32, subject: &str) -> Result<Vec<PreparedSeed>, String>
where I: IntoIterator<Item = &'a SchemaKey> + Clone {
    let mut seeded = BTreeSet::new();
    seeds.iter().map(|seed| {
        let name = seed.schema.as_str();
        let mut matches = allowed.clone().into_iter()
            .filter(|key| key.key == name || key.to_string() == name);
        let key = matches.next().ok_or_else(|| format!("{subject} cannot initialize schema {name}."))?;
        if matches.any(|other| other != key) {
            return Err(format!("{subject}: state schema {name} is ambiguous; use its qualified name."));
        }
        if !seeded.insert(key) { return Err(format!("{subject}: state schema {name} is repeated.")); }
        let declared = &admitted.schemas[key];
        if declared.schema.attachment != Attachment::Body {
            return Err(format!("{subject}: state schema {name} is not body-attached."));
        }
        let record = ambition_extension_sdk::script::record_from_script(&declared.schema, &seed.value)
            .map_err(|error| format!("{subject} state {name}: {error}"))?;
        declared.schema.check(&record).map_err(|error| format!("{subject} state {name}: {error}"))?;
        let mut encoded = Vec::new();
        record.put_wire(&mut encoded);
        *bytes = bytes.saturating_add(encoded.len());
        if *bytes > max_bytes as usize { return Err(format!("{subject}: initial state exceeds the byte bound.")); }
        Ok((key.clone(), declared.shape, record))
    }).collect()
}
