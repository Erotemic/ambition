//! The publish/install step: copy the runtime-classified files out of a
//! staging directory into a runtime root, skip the diagnostics, and return a
//! [`PublishManifest`] recording exactly what happened.
//!
//! This is deliberately the *small* publisher the plan's first slice calls for.
//! It does not repack atlases or rewrite loaders; it makes the install boundary
//! real: staging in, selected runtime artifacts out, diagnostics recorded but
//! not shipped.

use std::fs;
use std::io;
use std::path::Path;

use super::classify::{classify, ArtifactClass};
use super::manifest::{DiagnosticEntry, InstalledEntry, PublishManifest, Quality};
use super::walk::walk_files;

/// Options for a single publish run.
pub struct PublishOptions<'a> {
    pub profile: &'a str,
    /// Opaque timestamp/hash stamped into the manifest (kept clock-free).
    pub generated_at: &'a str,
    pub quality: Quality,
    /// The runtime root path as it should be recorded in the manifest's
    /// `runtime_roots` and used to prefix each `destination`.
    pub runtime_root_label: &'a str,
}

/// Install the runtime artifacts staged under `staging` into `runtime_root`.
///
/// Runtime-classified files are copied (creating parent dirs as needed) and
/// recorded under `installed`. Diagnostic-classified files are recorded under
/// `diagnostics` with `installed: false` and are never copied. Author-time
/// intermediates are neither copied nor recorded.
pub fn install(
    staging: &Path,
    runtime_root: &Path,
    opts: &PublishOptions<'_>,
) -> io::Result<PublishManifest> {
    let mut installed = Vec::new();
    let mut diagnostics = Vec::new();

    for rel in walk_files(staging)? {
        let class = classify(&rel);
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        if class == ArtifactClass::Diagnostic {
            diagnostics.push(DiagnosticEntry {
                kind: diagnostic_kind(&rel_str),
                path: rel_str,
                installed: false,
            });
            continue;
        }
        if !class.is_runtime() {
            continue; // author-time intermediate: not shipped, not a diagnostic
        }

        let dest_abs = runtime_root.join(&rel);
        if let Some(parent) = dest_abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(staging.join(&rel), &dest_abs)?;

        let stem = logical_stem(&rel_str);
        installed.push(InstalledEntry {
            logical_id: format!(
                "sprite.{stem}.{}.{}",
                quality_slug(opts.quality),
                class.manifest_kind()
            ),
            kind: class.manifest_kind().to_string(),
            quality: opts.quality,
            source: rel_str.clone(),
            destination: format!(
                "{}/{}",
                opts.runtime_root_label.trim_end_matches('/'),
                rel_str
            ),
        });
    }

    // Deterministic order so the manifest is diff-stable across runs.
    installed.sort_by(|a, b| a.destination.cmp(&b.destination));
    diagnostics.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(PublishManifest {
        schema_version: 1,
        profile: opts.profile.to_string(),
        generated_at: opts.generated_at.to_string(),
        runtime_roots: vec![opts.runtime_root_label.to_string()],
        installed,
        diagnostics,
    })
}

fn quality_slug(q: Quality) -> &'static str {
    match q {
        Quality::High => "high",
        Quality::Medium => "medium",
        Quality::Low => "low",
        Quality::Potato => "potato",
    }
}

/// A short label for a diagnostic, used only for human readability in the
/// manifest.
fn diagnostic_kind(rel_str: &str) -> String {
    let name = rel_str.rsplit('/').next().unwrap_or(rel_str);
    if name.ends_with("_preview_labeled.png") {
        "preview_sheet".into()
    } else if name.contains("_canonical") {
        "canonical_pose".into()
    } else if name.ends_with("_debug.png") {
        "debug_overlay".into()
    } else {
        "diagnostic".into()
    }
}

/// Reduce a file's relative path to a stable logical stem for id building:
/// drop the directory and the recognized runtime suffix/extension.
fn logical_stem(rel_str: &str) -> String {
    let name = rel_str.rsplit('/').next().unwrap_or(rel_str);
    for suffix in [
        "_spritesheet.ron",
        "_spritesheet.png",
        "_actor.ron",
        "_entity.ron",
    ] {
        if let Some(stem) = name.strip_suffix(suffix) {
            return stem.to_string();
        }
    }
    // Fall back to the filename without its final extension.
    name.rsplit_once('.')
        .map(|(head, _)| head)
        .unwrap_or(name)
        .to_string()
}
