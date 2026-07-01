//! Tests for the publish boundary.
//!
//! Three groups, matching the plan's first-pass test list:
//! - **manifest** shape validation (covered inline in `manifest.rs`);
//! - **publisher fixture** — a tiny staging tree installs correctly and records
//!   the diagnostic as not-installed;
//! - **runtime-root hygiene** — the real shipped sprite roots contain no
//!   leaked diagnostics.

use std::fs;
use std::path::{Path, PathBuf};

use super::classify::ArtifactClass;
use super::hygiene::scan_runtime_root;
use super::manifest::Quality;
use super::publish::{install, PublishOptions};
use super::RUNTIME_SPRITE_ROOTS;

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

/// The plan's canonical publisher fixture: a staging dir with one runtime
/// record, one runtime page, and one diagnostic preview. After install the two
/// runtime files land in the runtime root and the diagnostic does not; the
/// manifest records all three appropriately.
#[test]
fn publisher_installs_runtime_files_and_skips_diagnostics() {
    let tmp = tempfile::tempdir().unwrap();
    let staging = tmp.path().join("staging");
    let runtime_root = tmp.path().join("runtime/sprites");

    write(&staging.join("goblin_spritesheet.ron"), "([])");
    write(&staging.join("goblin_spritesheet.png"), "PNGDATA");
    write(&staging.join("goblin_preview_labeled.png"), "PREVIEW");

    let manifest = install(
        &staging,
        &runtime_root,
        &PublishOptions {
            profile: "dev",
            generated_at: "fixture",
            quality: Quality::High,
            runtime_root_label: "assets/sprites",
        },
    )
    .unwrap();

    // Runtime files installed on disk.
    assert!(runtime_root.join("goblin_spritesheet.ron").exists());
    assert!(runtime_root.join("goblin_spritesheet.png").exists());
    // Diagnostic NOT installed on disk.
    assert!(!runtime_root.join("goblin_preview_labeled.png").exists());

    // Manifest records both runtime files and the un-installed diagnostic.
    assert_eq!(manifest.installed.len(), 2);
    let kinds: Vec<&str> = manifest.installed.iter().map(|e| e.kind.as_str()).collect();
    assert!(kinds.contains(&"sheet_record"));
    assert!(kinds.contains(&"image_page"));
    assert_eq!(manifest.diagnostics.len(), 1);
    assert!(!manifest.diagnostics[0].installed);

    // The produced manifest is internally consistent and round-trips.
    assert!(manifest.validate_shape().is_empty());
    assert!(manifest.validate_sources(&staging).is_empty());
    let reparsed = super::PublishManifest::parse(&manifest.to_ron().unwrap()).unwrap();
    assert_eq!(manifest, reparsed);
}

/// A hygiene scan of a staging-style tree flags the diagnostic as a hard error
/// and the throwaway YAML as a warning, while passing the runtime files.
#[test]
fn hygiene_scan_separates_errors_from_warnings() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(&root.join("goblin_spritesheet.ron"), "([])");
    write(&root.join("goblin_spritesheet.yaml"), "target: goblin");
    write(&root.join("goblin_canonical.png"), "CANON");
    write(&root.join("canonicals/alice_canonical.png"), "CANON");

    let report = scan_runtime_root(root).unwrap();
    assert_eq!(
        report.errors.len(),
        2,
        "both canonical pngs are hard errors"
    );
    assert!(report
        .errors
        .iter()
        .all(|f| f.class == ArtifactClass::Diagnostic));
    assert_eq!(report.warnings.len(), 1, "the yaml is a warning");
    assert!(!report.is_clean());
}

/// The real, shipped runtime sprite roots must contain no leaked diagnostics.
/// This is the boundary's teeth: if a generator ever dumps a `*_canonical.png`
/// or `*_preview_labeled.png` back under a runtime root, this fails.
///
/// Warnings (throwaway YAML intermediates) are tolerated and only printed, per
/// the plan's "legacy unmanaged files are warnings, not hard errors" rule.
#[test]
fn shipped_runtime_roots_have_no_leaked_diagnostics() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut all_errors = Vec::new();
    for rel in RUNTIME_SPRITE_ROOTS {
        let root = crate_dir.join(rel);
        let report = scan_runtime_root(&root).unwrap();
        if !report.warnings.is_empty() {
            println!(
                "[hygiene] {rel}: {} intermediate warning(s) (tolerated)",
                report.warnings.len()
            );
        }
        for finding in report.errors {
            all_errors.push(format!("{rel}/{}", finding.path.display()));
        }
    }
    assert!(
        all_errors.is_empty(),
        "diagnostics leaked into runtime sprite roots:\n  - {}",
        all_errors.join("\n  - ")
    );
}
