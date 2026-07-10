//! `dependency-allowlist` / `dependency-denylist`: crate-boundary ratchets over
//! a PARSED manifest (not a line grep). The allowlist pins a crate's complete
//! `ambition*` dependency set; the denylist forbids specific edges.

use crate::model::{Policy, Report};
use crate::workspace::Workspace;

fn manifest_of(policy: &Policy) -> &str {
    policy.manifest.as_deref().unwrap_or_else(|| {
        panic!(
            "policy `{}` (dependency rule) has no `manifest` path",
            policy.id
        )
    })
}

/// The `ambition*` deps of `manifest` must be a subset of `allow`. A dep not in
/// the list fails; the list may name crates that are not currently deps (it is a
/// ceiling, not an exact set — the extraction ratchets tighten it over time).
pub fn allowlist(ws: &Workspace, policy: &Policy, report: &mut Report) {
    let manifest = manifest_of(policy);
    assert!(
        !policy.allow.is_empty(),
        "policy `{}` (dependency-allowlist) has an empty `allow` — that forbids ALL ambition deps; \
         if that is intended, say so explicitly rather than leaving it blank",
        policy.id
    );
    let deps = ws.ambition_deps(manifest);
    for dep in &deps {
        if !policy.allow.iter().any(|a| a == dep) {
            report.push(policy.diag(
                format!("{manifest} → {dep}"),
                format!("dependency not in the allowlist {:?}", policy.allow),
            ));
        }
    }
    // Bidirectional ratchet: a stale allow entry — one that names no current dep —
    // means the edge dissolved. Fail so the allowlist shrinks with the code (this
    // is the property the ambition_world world-IR purity ratchet relies on).
    if policy.exact {
        for allowed in &policy.allow {
            if !deps.contains(allowed) {
                report.push(policy.diag(
                    format!("{manifest} → {allowed}"),
                    "stale allowlist entry — no longer a dependency; remove it from `allow`",
                ));
            }
        }
    }
}

/// `manifest` must not depend on any crate in `deny`. `deny` entries are matched
/// against the manifest's `ambition*` deps AND, for non-`ambition` names (e.g.
/// `bevy_ecs_ldtk`), against a parsed lookup of all dependency tables.
pub fn denylist(ws: &Workspace, policy: &Policy, report: &mut Report) {
    let manifest = manifest_of(policy);
    assert!(
        !policy.deny.is_empty(),
        "policy `{}` (dependency-denylist) has an empty `deny` — vacuous",
        policy.id
    );
    let ambition = ws.ambition_deps(manifest);
    let all = all_deps(ws, manifest);
    for denied in &policy.deny {
        if ambition.contains(denied) || all.contains(denied) {
            report.push(policy.diag(
                format!("{manifest} → {denied}"),
                "forbidden dependency present",
            ));
        }
    }
}

/// Every dependency name (any crate, not just `ambition*`) across the standard
/// dependency tables, for denylist entries like `bevy_ecs_ldtk`.
fn all_deps(ws: &Workspace, manifest_rel: &str) -> std::collections::BTreeSet<String> {
    let text = std::fs::read_to_string(ws.abs(manifest_rel))
        .unwrap_or_else(|e| panic!("read manifest `{manifest_rel}`: {e}"));
    let mut out = std::collections::BTreeSet::new();
    let Ok(table) = text.parse::<toml::Table>() else {
        return out;
    };
    let mut collect = |t: Option<&toml::Value>| {
        if let Some(deps) = t.and_then(|v| v.as_table()) {
            out.extend(deps.keys().cloned());
        }
    };
    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        collect(table.get(key));
    }
    if let Some(targets) = table.get("target").and_then(|v| v.as_table()) {
        for cfg in targets.values() {
            if let Some(cfg_t) = cfg.as_table() {
                for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
                    collect(cfg_t.get(key));
                }
            }
        }
    }
    out
}
