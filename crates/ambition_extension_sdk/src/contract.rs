//! Compare module declarations across code backends.

use std::fmt::Debug;

use crate::{IdlePolicy, ModuleDescriptor, Value};

/// One difference in a module declaration. `expected` is the reference module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclarationDifference {
    pub path: String,
    pub expected: String,
    pub actual: String,
}

impl std::fmt::Display for DeclarationDifference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: expected {}; got {}", self.path, self.expected, self.actual)
    }
}

impl ModuleDescriptor {
    /// The declaration without code identity or diagnostic field names.
    /// Schema and permission lists have key order. Entry and field order stay
    /// intact: they control entry scheduling and record layout.
    ///
    /// These bytes describe compatibility, not executable identity. Admission
    /// must still check both modules against the installed domain ports.
    pub fn semantic_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        normalized(self).put(&mut bytes);
        bytes
    }

    /// Compare the semantic declarations. Code identity is checked separately.
    /// A match does not prove that the two algorithms have the same behavior.
    pub fn semantic_differences(&self, candidate: &Self) -> Vec<DeclarationDifference> {
        let expected = normalized(self);
        let actual = normalized(candidate);
        let mut differences = Vec::new();
        compare(&mut differences, "module", &expected.key, &actual.key);
        compare(&mut differences, "api", &expected.api, &actual.api);
        compare(&mut differences, "schemas.len", &expected.schemas.len(), &actual.schemas.len());
        for (index, (a, b)) in expected.schemas.iter().zip(&actual.schemas).enumerate() {
            let path = format!("schemas[{index}]({})", a.key);
            compare(&mut differences, &format!("{path}.key"), &a.key, &b.key);
            compare(&mut differences, &format!("{path}.attachment"), &a.attachment, &b.attachment);
            compare(&mut differences, &format!("{path}.save"), &a.save, &b.save);
            compare(&mut differences, &format!("{path}.fields.len"), &a.fields.len(), &b.fields.len());
            for (index, (a, b)) in a.fields.iter().zip(&b.fields).enumerate() {
                let path = format!("{path}.fields[{index}](tag={})", a.tag);
                compare(&mut differences, &format!("{path}.tag"), &a.tag, &b.tag);
                compare(&mut differences, &format!("{path}.kind"), &a.kind, &b.kind);
                compare(&mut differences, &format!("{path}.initial"), &a.initial, &b.initial);
            }
        }
        compare(&mut differences, "entries.len", &expected.entries.len(), &actual.entries.len());
        for (index, (a, b)) in expected.entries.iter().zip(&actual.entries).enumerate() {
            let path = format!("entries[{index}]({})", a.key);
            compare(&mut differences, &format!("{path}.key"), &a.key, &b.key);
            compare(&mut differences, &format!("{path}.phase"), &a.phase, &b.phase);
            compare(&mut differences, &format!("{path}.trigger"), &a.trigger, &b.trigger);
            compare(&mut differences, &format!("{path}.reads"), &a.reads, &b.reads);
            compare(&mut differences, &format!("{path}.writes"), &a.writes, &b.writes);
            compare(&mut differences, &format!("{path}.requests"), &a.requests, &b.requests);
            compare(&mut differences, &format!("{path}.after"), &a.after, &b.after);
            compare(&mut differences, &format!("{path}.replies"), &a.replies, &b.replies);
            compare(&mut differences, &format!("{path}.limits"), &a.limits, &b.limits);
            compare(&mut differences, &format!("{path}.idle"), &a.on_idle, &b.on_idle);
        }
        differences
    }
}

fn normalized(module: &ModuleDescriptor) -> ModuleDescriptor {
    let mut module = module.clone();
    module.schemas.sort_by(|a, b| a.key.cmp(&b.key));
    for schema in &mut module.schemas {
        for field in &mut schema.fields {
            normalize_field(field);
        }
    }
    for entry in &mut module.entries {
        entry.reads.sort();
        entry.requests.sort();
        entry.writes.sort();
        entry.after.sort();
        entry.replies.sort();
        if let IdlePolicy::ResetStateExcept(keep) = &mut entry.on_idle {
            keep.sort();
        }
    }
    module
}

fn normalize_field(field: &mut crate::FieldDecl) {
    field.name = "".into();
    normalize_kind(&mut field.kind);
    normalize_value(&mut field.initial);
}

fn normalize_kind(kind: &mut crate::FieldKind) {
    match kind {
        crate::FieldKind::Record(fields) => fields.iter_mut().for_each(normalize_field),
        crate::FieldKind::Opt(inner) | crate::FieldKind::Seq { elem: inner, .. } => normalize_kind(inner),
        _ => {}
    }
}

fn normalize_value(value: &mut Value) {
    match value {
        Value::F32(v) if *v == 0.0 => *v = 0.0,
        Value::Vec2(v) => {
            for v in v {
                if *v == 0.0 {
                    *v = 0.0;
                }
            }
        }
        Value::Opt(Some(v)) => normalize_value(v),
        Value::Seq(v) => v.iter_mut().for_each(normalize_value),
        Value::Record(record) => {
            for i in 0..record.values().len() {
                normalize_value(record.get_mut(crate::FieldRef(i)).unwrap());
            }
        }
        _ => {}
    }
}

fn compare<T: PartialEq + Debug>(differences: &mut Vec<DeclarationDifference>, path: &str, a: &T, b: &T) {
    if a != b {
        differences.push(DeclarationDifference {
            path: path.to_owned(),
            expected: format!("{a:?}"),
            actual: format!("{b:?}"),
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    fn module() -> ModuleDescriptor {
        let schema = StateSchema {
            key: SchemaKey::new("test", "cadence", 1),
            attachment: Attachment::Body,
            save: SaveEligibility::Transient,
            fields: vec![
                FieldDecl { tag: 1, name: "ticks".into(), kind: FieldKind::U32, initial: Value::U32(0) },
                FieldDecl { tag: 2, name: "elapsed".into(), kind: FieldKind::F32, initial: Value::F32(0.0) },
            ],
        };
        ModuleDescriptor {
            key: ModuleKey::new("test", "clock"), api: API_VERSION,
            code: CodeIdentity::StaticNative { build: Some([1; 32]), crate_name: "clock".into(), version: "1".into() },
            schemas: vec![schema.clone()],
            entries: vec![EntryDescriptor {
                replies: Vec::new(),
                key: "tick".into(), phase: phases::ACTOR_THINK,
                trigger: TriggerBinding { port: PortKey::new("think", 1), selector: "clock".into() },
                reads: vec![PortKey::new("view", 1), PortKey::new("target", 2)],
                writes: vec![schema.key], requests: vec![PortKey::new("steer", 1)], after: vec![],
                limits: Limits { max_requests: 1 }, on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(|_| Ok(())),
            }],
        }
    }

    #[test]
    fn different_code_and_diagnostic_names_keep_the_contract() {
        let reference = module();
        let mut candidate = reference.clone();
        if let CodeIdentity::StaticNative { build, .. } = &mut candidate.code {
            *build = Some([123; 32]);
        }
        assert_ne!(reference.identity_digest(), candidate.identity_digest());
        assert_eq!(reference.semantic_bytes(), candidate.semantic_bytes());
        candidate.code = CodeIdentity::Loaded { abi: "rhai".into(), digest: 123 };
        candidate.entries[0].run = EntryCode::Loaded { index: 9 };
        candidate.entries[0].reads.reverse();
        candidate.schemas[0].fields[0].name = "counter".into();
        candidate.schemas[0].fields[1].initial = Value::F32(-0.0);
        assert_ne!(reference.identity_digest(), candidate.identity_digest());
        assert_eq!(reference.semantic_bytes(), candidate.semantic_bytes());
        assert!(reference.semantic_differences(&candidate).is_empty());
    }

    #[test]
    fn state_and_port_drift_name_the_contract_fields() {
        let reference = module();
        let mut candidate = reference.clone();
        candidate.schemas[0].attachment = Attachment::Session;
        candidate.schemas[0].save = SaveEligibility::Durable;
        candidate.schemas[0].fields[0].kind = FieldKind::I32;
        candidate.schemas[0].fields[0].initial = Value::I32(0);
        candidate.entries[0].requests.clear();
        let differences = reference.semantic_differences(&candidate);
        for suffix in [".attachment", ".save", ".kind", ".initial", ".requests"] {
            assert!(differences.iter().any(|d| d.path.ends_with(suffix)), "{differences:?}");
        }
        assert_ne!(reference.semantic_bytes(), candidate.semantic_bytes());
    }

    #[test]
    fn field_and_entry_order_remain_part_of_the_contract() {
        let reference = module();
        let mut candidate = reference.clone();
        candidate.schemas[0].fields.reverse();
        assert!(!reference.semantic_differences(&candidate).is_empty());
        assert_ne!(reference.semantic_bytes(), candidate.semantic_bytes());
        let mut reference = reference;
        let mut second = reference.entries[0].clone();
        second.key = "second".into();
        reference.entries.push(second);
        candidate = reference.clone();
        candidate.entries.reverse();
        assert!(!reference.semantic_differences(&candidate).is_empty());
        assert_ne!(reference.semantic_bytes(), candidate.semantic_bytes());
    }
}
