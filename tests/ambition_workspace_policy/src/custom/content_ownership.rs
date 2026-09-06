//! The archetype-free enemy-config gate (custom scanner). The DURABLE enemy
//! structs — `ActorConfig` (persisted) and `ActorMut` (per-frame view) — must
//! carry projected generic kit data (tuning / brain_spec / caps), never the named
//! roster enum `CharacterArchetype`. That is what lets the roster leave the
//! machinery lib for `ambition_content`. The spawn-time `ActorClusterSeed` may
//! carry the enum (consumed before the entity exists), so this guards only the
//! two durable structs by parsing their bodies.

use crate::model::{CustomMeta, Diagnostic, Report, Scope, Severity};
use crate::workspace::{self, Workspace};

const POLICY_ID: &str = "engine.enemy-config-archetype-free";

/// The two durable structs and where each one is DECLARED.
///
/// ⛔⛔ ONE PATH FOR BOTH WAS A LATENT PANIC. `ActorConfig` left the monolith for
/// `ambition_combat::actor_tuning` (aa70a41e3, 2026-08-26) and this scanner
/// still read only `actor_clusters.rs`, so `engine_policies` aborted with
/// "`pub struct ActorConfig {` not found" rather than reporting anything. A
/// checker that locates its subject by a hard-coded path owes that path a
/// per-subject entry, or the next move silently turns the gate into a crash.
const STRUCTS: &[(&str, &str)] = &[
    (
        "pub struct ActorConfig {",
        "crates/ambition_combat/src/actor_tuning.rs",
    ),
    (
        "pub struct ActorMut<'a> {",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/actor_clusters.rs",
    ),
];

/// Fields of a struct body that name the roster enum. Doc/comment lines are
/// skipped (a field's prose may say "projected from the archetype" while the
/// field itself is generic).
fn archetype_fields(text: &str, struct_name: &str) -> Option<Vec<String>> {
    let start = text.find(struct_name)?;
    let body = &text[start..];
    let end = body.find("\n}")?;
    Some(
        body[..end]
            .lines()
            .map(str::trim)
            .filter(|line| !workspace::is_comment_line(line))
            .filter(|line| line.contains("CharacterArchetype") || line.contains("archetype:"))
            .map(str::to_string)
            .collect(),
    )
}

pub fn metas() -> Vec<CustomMeta> {
    vec![CustomMeta {
        id: POLICY_ID.to_string(),
        scope: Scope::Engine,
        owners: vec!["ambition_platformer2d_actor_monolith".to_string()],
        watch_paths: STRUCTS
            .iter()
            .map(|(_, file)| (*file).to_string())
            .collect(),
        source_doc: "docs/architecture/architecture-boundaries.md".to_string(),
        severity: Severity::Error,
    }]
}

pub fn run(ws: &Workspace, report: &mut Report) {
    for (struct_name, file) in STRUCTS {
        let text = std::fs::read_to_string(ws.abs(file))
            .unwrap_or_else(|error| panic!("read {file}: {error}"));
        let fields = archetype_fields(&text, struct_name)
            .unwrap_or_else(|| panic!("{struct_name} not found in {file}"));
        for field in fields {
            report.push(Diagnostic {
                policy_id: POLICY_ID.to_string(),
                owners: vec!["ambition_platformer2d_actor_monolith".to_string()],
                source_doc: "docs/architecture/architecture-boundaries.md".to_string(),
                rationale: "durable enemy structs must stay archetype-free — project generic kit data (tuning/brain_spec/caps) at spawn instead of storing the roster enum".to_string(),
                location: format!("{file} :: {struct_name}"),
                detail: format!("names the roster enum in a field: {field}"),
            });
        }
    }
}

pub fn poison_self_tests() {
    let poison = "pub struct ActorConfig {\n    pub archetype: CharacterArchetype,\n}\n";
    assert_eq!(
        archetype_fields(poison, "pub struct ActorConfig {")
            .unwrap()
            .len(),
        1,
        "an archetype field must be detected"
    );
    let clean = "pub struct ActorConfig {\n    // projected from the archetype at spawn\n    pub tuning: EnemyTuning,\n}\n";
    assert!(
        archetype_fields(clean, "pub struct ActorConfig {")
            .unwrap()
            .is_empty(),
        "a generic field with archetype PROSE in a comment must not trip"
    );
}
