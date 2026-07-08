//! Backend-agnostic authored world IR.
//!
//! This crate owns the room graph, authored placement records, room metadata,
//! and moving-platform math. Backend adapters such as LDtk convert into these
//! types; simulation crates interpret them through explicit lowering seams.

pub mod debug_label;
pub mod placements;
pub mod platforms;
pub mod ron_room;
pub mod rooms;

pub use debug_label::{DebugLabel, DebugLabelKind};

#[cfg(test)]
mod dependency_tests {
    use std::collections::BTreeSet;

    #[test]
    fn ambition_world_dependency_allowlist_ratchets_world_ir_purity() {
        let manifest = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        )
        .expect("reads ambition_world Cargo.toml");

        let mut in_deps = false;
        let mut ambition_deps = BTreeSet::new();
        for line in manifest.lines() {
            let trimmed = line.trim();
            if trimmed == "[dependencies]" {
                in_deps = true;
                continue;
            }
            if in_deps && trimmed.starts_with('[') {
                break;
            }
            if !in_deps || !trimmed.starts_with("ambition_") {
                continue;
            }
            let Some((name, _)) = trimmed.split_once('=') else {
                continue;
            };
            ambition_deps.insert(name.trim().to_string());
        }

        let allowed = BTreeSet::from([
            "ambition_engine_core".to_string(),
            "ambition_entity_catalog".to_string(),
            "ambition_time".to_string(),
        ]);
        assert_eq!(
            ambition_deps, allowed,
            "ambition_world must only name explicit world-IR dependencies; \
             remove legacy entries from this allow-list as each placement branch dissolves"
        );
    }
}
