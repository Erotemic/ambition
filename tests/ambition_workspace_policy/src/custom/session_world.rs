//! Canonical session-world authority ratchet.
//!
//! The live room/world state is a component bundle on the exact `SessionRoot`.
//! These checks prevent the deleted process-resource projection from returning
//! and prevent any canonical world part from regaining a `Resource` derive.

use std::path::Path;

use crate::model::{CustomMeta, Diagnostic, Report, Scope, Severity};
use crate::workspace::{self, Workspace};

const POLICY_ID: &str = "engine.canonical-session-world-only";
const TARGETS: &[&str] = &[
    "RoomSet",
    "RoomGeometry",
    "ActiveRoomMetadata",
    "StartingCharacter",
    "LdtkRuntimeIndex",
    "RoomMusicRequest",
    "EncounterMusicRequest",
];
const DELETED_BRIDGE_NAMES: &[&str] = &[
    "SessionWorldProjectionAuthority",
    "PlatformerSessionWorldProjectionPlugin",
    "ActivePlatformerSessionWorld",
    "ActivePlatformerSessionWorldMut",
    "sync_session_world_projection",
];

fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

fn exact_public_struct_position(compact: &str, type_name: &str) -> Option<usize> {
    let needle = format!("pubstruct{type_name}");
    let mut rest = compact;
    let mut offset = 0usize;
    while let Some(relative) = rest.find(&needle) {
        let pos = offset + relative;
        let after = compact[pos + needle.len()..].chars().next();
        if after.is_none_or(|ch| !ch.is_alphanumeric() && ch != '_') {
            return Some(pos);
        }
        let advance = relative + needle.len();
        offset += advance;
        rest = &rest[advance..];
    }
    None
}

fn declaration_derives(text: &str, type_name: &str, derive_name: &str) -> bool {
    let compact = compact(text);
    let Some(pos) = exact_public_struct_position(&compact, type_name) else {
        return false;
    };
    let prefix = &compact[..pos];
    let Some(derive) = prefix.rfind("#[derive(") else {
        return false;
    };
    let Some(end) = prefix[derive..].find(")]") else {
        return false;
    };
    prefix[derive..derive + end]
        .split(|c| c == '(' || c == ',' || c == ')')
        .any(|item| item.rsplit("::").next() == Some(derive_name))
}

fn declaration_derives_resource(text: &str, type_name: &str) -> bool {
    declaration_derives(text, type_name, "Resource")
}

fn generic_authority_mentions(compact: &str, prefix: &str, type_name: &str) -> bool {
    let mut rest = compact;
    while let Some(start) = rest.find(prefix) {
        let candidate = &rest[start + prefix.len()..];
        let Some(end) = candidate.find('>') else {
            return false;
        };
        let first_generic = candidate[..end].split(',').next().unwrap_or_default();
        if first_generic.ends_with(type_name) {
            return true;
        }
        rest = &candidate[end + 1..];
    }
    false
}

fn inspect_source(path: &Path, text: &str) -> Vec<String> {
    let mut violations = Vec::new();
    for deleted in DELETED_BRIDGE_NAMES {
        if text.contains(deleted) {
            violations.push(format!("deleted projection symbol `{deleted}` is present"));
        }
    }
    let compact_source = compact(text);
    for target in TARGETS {
        if declaration_derives_resource(text, target)
            || compact_source.contains(&format!("implResourcefor{target}"))
        {
            violations.push(format!(
                "canonical session-world type `{target}` implements Resource; it must remain component-only"
            ));
        }
        for prefix in [
            "Res<",
            "ResMut<",
            "resource::<",
            "resource_mut::<",
            "get_resource::<",
            "get_resource_mut::<",
            "init_resource::<",
            "insert_resource::<",
            "remove_resource::<",
            "resource_scope::<",
        ] {
            if generic_authority_mentions(&compact_source, prefix, target) {
                violations.push(format!(
                    "canonical session-world type `{target}` is accessed through process-resource API `{prefix}`"
                ));
            }
        }
    }
    if path.ends_with("crates/ambition_runtime/src/session_world.rs") {
        let compact = compact(text);
        for required in ["#[derive(Bundle", "pubstructPlatformerSessionWorld"] {
            if !compact.contains(required) {
                violations.push(format!(
                    "canonical bundle source is missing required shape `{required}`"
                ));
            }
        }
        if declaration_derives_resource(text, "PlatformerSessionWorld")
            || declaration_derives(text, "PlatformerSessionWorld", "Component")
        {
            violations.push(
                "PlatformerSessionWorld must be a preparation/activation Bundle, not a resident Component or Resource"
                    .to_string(),
            );
        }
    }
    violations
}

pub fn metas() -> Vec<CustomMeta> {
    vec![CustomMeta {
        id: POLICY_ID.to_string(),
        scope: Scope::Engine,
        owners: vec!["ambition_platformer_primitives".to_string()],
        watch_paths: vec!["crates".to_string(), "game".to_string()],
        source_doc: "docs/planning/engine/architecture.md".to_string(),
        severity: Severity::Error,
    }]
}

pub fn run(ws: &Workspace, report: &mut Report) {
    let mut scanned = 0usize;
    for root in ["crates", "game"] {
        for file in workspace::rust_sources_under(&ws.abs(root)) {
            scanned += 1;
            let text = std::fs::read_to_string(&file).expect("read Rust source");
            for detail in inspect_source(&file, &text) {
                report.push(Diagnostic {
                    policy_id: POLICY_ID.to_string(),
                    owners: vec!["ambition_platformer_primitives".to_string()],
                    source_doc: "docs/planning/engine/architecture.md"
                        .to_string(),
                    rationale: "live platformer world state exists only as components on the exact SessionRoot; process resources and synchronization bridges are forbidden"
                        .to_string(),
                    location: file
                        .strip_prefix(ws.root())
                        .unwrap_or(&file)
                        .display()
                        .to_string(),
                    detail,
                });
            }
        }
    }
    assert!(scanned > 0, "session-world ratchet scanned no Rust sources");
}

pub fn poison_self_tests() {
    assert!(declaration_derives_resource(
        "#[derive(Component, Resource)] pub struct RoomSet;",
        "RoomSet"
    ));
    assert!(!declaration_derives_resource(
        "#[derive(Component)] pub struct RoomSet;",
        "RoomSet"
    ));
    assert!(!declaration_derives_resource(
        "#[derive(Resource)] pub struct StartingCharacterOverride;",
        "StartingCharacter"
    ));
    assert!(inspect_source(
        Path::new("x.rs"),
        "fn x() { let _ = SessionWorldProjectionAuthority; }"
    )
    .iter()
    .any(|v| v.contains("deleted projection")));
    assert!(inspect_source(
        Path::new("x.rs"),
        "fn x(room: Res<crate::rooms::RoomSet>) {}"
    )
    .iter()
    .any(|v| v.contains("process-resource API")));
    assert!(
        inspect_source(Path::new("x.rs"), "impl Resource for RoomGeometry {}")
            .iter()
            .any(|v| v.contains("implements Resource"))
    );
    assert!(inspect_source(
        Path::new("x.rs"),
        "fn x(world: &mut World) { world.insert_resource::<RoomSet>(todo!()); }"
    )
    .iter()
    .any(|v| v.contains("process-resource API")));
    assert!(inspect_source(
        Path::new("x.rs"),
        "fn x(world: &mut World) { world.remove_resource::<RoomSet>(); }"
    )
    .iter()
    .any(|v| v.contains("process-resource API")));
    assert!(inspect_source(
        Path::new("x.rs"),
        "fn x(world: &mut World) { world.resource_scope::<RoomGeometry, _>(|_, _| {}); }"
    )
    .iter()
    .any(|v| v.contains("process-resource API")));
}
