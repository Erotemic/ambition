//! Lint-style guardrails that grep `crates/ambition_gameplay_core/src/` for
//! patterns that should not re-appear (legacy `asset_exists`, raw
//! `BEVY_ASSET_ROOT` env probes outside the catalog, `setup.rs`
//! disk-path helper, deprecated `should_attempt_optional_load`).

// ─────────────────────────────────────────────────────────────────
// Guardrail tests — these fail loud when the catalog migration
// regresses (legacy `asset_exists` re-appears, embedded source
// breaks the WebStatic flip, etc.). Add to this section, don't
// delete.
// ─────────────────────────────────────────────────────────────────

/// No `fn asset_exists` / `fn desktop_asset_exists` *definitions*
/// live anywhere under `crates/ambition_gameplay_core/src/`. The only
/// host-filesystem probe is `desktop_candidate_roots` in this
/// file. Catching a regression here means someone re-added a
/// per-target existence walker; collapse it back through
/// [`super::super::SandboxAssetCatalog::resolve_local_file_path`].
///
/// Matches at line start (`^[ \t]*`) so the test's own doc-comment
/// mentioning the function names doesn't trip the guard.
#[test]
fn no_legacy_asset_exists_copies_in_sandbox_src() {
    use std::process::Command;
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("src");
    let output = Command::new("grep")
        .args([
            "-rln",
            "-E",
            "^[[:space:]]*(pub(\\([^)]*\\))?[[:space:]]+)?fn[[:space:]]+(asset_exists|desktop_asset_exists)\\b",
        ])
        .arg(&src)
        .output();
    let stdout = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return, // `grep` missing → skip the guard rather than spuriously failing.
    };
    let offenders: Vec<&str> = stdout.lines().filter(|line| !line.is_empty()).collect();
    assert!(
        offenders.is_empty(),
        "legacy asset_exists / desktop_asset_exists copies re-appeared:\n  {}\n\
         Collapse the candidate-roots walk back through \
         SandboxAssetCatalog::resolve_local_file_path.",
        offenders.join("\n  "),
    );
}

/// No raw `env::var_os("BEVY_ASSET_ROOT"` / `env::var("BEVY_ASSET_ROOT"`
/// outside `sandbox_assets/`. The catalog owns the only probe.
/// Catches regressions where a new loader re-implements the
/// candidate-roots dance instead of calling
/// `SandboxAssetCatalog::resolve_local_file_path`. Doc-comments
/// mentioning the env var by name are allowed.
#[test]
fn no_unauthorized_bevy_asset_root_probes() {
    use std::process::Command;
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("src");
    let output = Command::new("grep")
        .args([
            "-rln",
            "-E",
            "env::(var|var_os)\\([[:space:]]*\"BEVY_ASSET_ROOT\"",
        ])
        .arg(&src)
        .output();
    let stdout = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return,
    };
    let allowed = [
        "sandbox_assets/mod.rs",
        "sandbox_assets/tests.rs",
        "sandbox_assets/tests/profiles.rs",
        "sandbox_assets/tests/static_probes.rs",
    ];
    let offenders: Vec<String> = stdout
        .lines()
        .filter(|line| {
            !line.is_empty()
                && !allowed
                    .iter()
                    .any(|a| line.ends_with(a) || line.contains(&format!("/{a}:")))
        })
        .map(String::from)
        .collect();
    assert!(
        offenders.is_empty(),
        "unauthorized BEVY_ASSET_ROOT probe(s) re-appeared:\n  {}\n\
         Approved sites are `sandbox_assets/mod.rs` and the catalog \
         test files only. Route new host-filesystem reads through \
         SandboxAssetCatalog::resolve_local_file_path.",
        offenders.join("\n  "),
    );
}

/// SFX bank byte resolution goes through the catalog. No ad-hoc
/// candidate walker should exist in `setup.rs`. The catalog's
/// `AMBITION_SFX_BANK_PATH` env override is an authored
/// `LooseFilesystem` `LocationCandidate` — visible policy, not a
/// side path.
#[test]
fn no_setup_resolve_to_disk_path_helper() {
    use std::process::Command;
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let setup = manifest_dir.join("src/setup.rs");
    let output = Command::new("grep")
        .args(["-n", "fn resolve_to_disk_path"])
        .arg(&setup)
        .output();
    if let Ok(o) = output {
        let stdout = String::from_utf8_lossy(&o.stdout).to_string();
        assert!(
            stdout.is_empty(),
            "setup.rs::resolve_to_disk_path re-appeared:\n  {stdout}\n\
             Route SFX bank disk reads through \
             SandboxAssetCatalog::resolve_local_file_path instead.",
        );
    }
}

/// Guardrail: `SandboxAssetCatalog::should_attempt_optional_load(path: &str)`
/// has been removed. Catches a regression where someone adds the
/// gate back to satisfy a new dynamic-path consumer instead of
/// authoring a catalog id.
#[test]
fn no_should_attempt_optional_load_method_definition() {
    use std::process::Command;
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("src");
    let output = Command::new("grep")
        .args([
            "-rln",
            "-E",
            "fn[[:space:]]+should_attempt_optional_load[[:space:]]*\\(",
        ])
        .arg(&src)
        .output();
    let stdout = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return,
    };
    let offenders: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert!(
        offenders.is_empty(),
        "should_attempt_optional_load(...) reappeared:\n  {}\n\
         Author a catalog id + `with_embedded_core_candidate` if needed; \
         loaders should use `try_path_for_load` only.",
        offenders.join("\n  "),
    );
}
