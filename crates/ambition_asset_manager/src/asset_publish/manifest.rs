//! The [`PublishManifest`]: the record of exactly what a publish/install step
//! placed into the runtime asset roots, at what quality, and which diagnostics
//! it generated but deliberately did *not* install.
//!
//! The manifest is the first-class artifact of the publish boundary. Its
//! validation answers, with no filesystem access required for the shape checks:
//!
//! - what did the publisher install, and where?
//! - is every destination inside an allowed runtime root?
//! - did anything marked diagnostic get installed anyway?

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Runtime quality tier. Mirrors the on-disk root suffixes
/// (`sprites`, `sprites_0_5x`, `sprites_0_25x`, `sprites_potato`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    High,
    Medium,
    Low,
    Potato,
}

/// One runtime file the publisher installed from staging into a runtime root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledEntry {
    /// Stable logical id (e.g. `sprite.goblin.high.sheet_record`). Groups the
    /// record + page + sidecars that make up one visual across qualities.
    pub logical_id: String,
    /// Manifest `kind` string (see [`crate::asset_publish::ArtifactClass::manifest_kind`]).
    pub kind: String,
    /// Quality tier this file belongs to.
    pub quality: Quality,
    /// Path the file was copied *from* (staging), relative to the publish run.
    pub source: String,
    /// Path the file was installed *to*, relative to the repo root.
    pub destination: String,
}

/// A diagnostic the publish run produced. `installed` is expected to be `false`;
/// a `true` here is a boundary violation the validator rejects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticEntry {
    pub kind: String,
    pub path: String,
    #[serde(default)]
    pub installed: bool,
}

/// The install record for one publish run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishManifest {
    pub schema_version: u32,
    /// Publish profile label (e.g. `"dev"`).
    pub profile: String,
    /// Free-form timestamp/hash the caller stamps in. Kept opaque so the type
    /// stays dependency- and clock-free.
    pub generated_at: String,
    /// The runtime roots this run is allowed to write into. Every installed
    /// `destination` must be under one of these.
    pub runtime_roots: Vec<String>,
    pub installed: Vec<InstalledEntry>,
    #[serde(default)]
    pub diagnostics: Vec<DiagnosticEntry>,
}

/// A shape/hygiene problem found while validating a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// An installed destination is not under any declared runtime root.
    DestinationOutsideRuntimeRoots {
        logical_id: String,
        destination: String,
    },
    /// A diagnostic entry claims it was installed.
    DiagnosticMarkedInstalled { path: String },
    /// A referenced installed source file does not exist under the staging base.
    MissingInstalledSource { logical_id: String, source: String },
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::DestinationOutsideRuntimeRoots {
                logical_id,
                destination,
            } => write!(
                f,
                "installed `{logical_id}` destination `{destination}` is outside all runtime roots"
            ),
            ManifestError::DiagnosticMarkedInstalled { path } => {
                write!(f, "diagnostic `{path}` is marked installed")
            }
            ManifestError::MissingInstalledSource { logical_id, source } => {
                write!(f, "installed `{logical_id}` source `{source}` is missing")
            }
        }
    }
}

impl PublishManifest {
    /// Parse a manifest from RON text.
    pub fn parse(ron_text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(ron_text)
    }

    /// Serialize to pretty RON.
    pub fn to_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }

    /// Filesystem-free shape checks. Every failure here is a hard boundary
    /// violation regardless of what is on disk:
    ///
    /// - no installed destination may escape the declared runtime roots;
    /// - no diagnostic may be marked installed.
    pub fn validate_shape(&self) -> Vec<ManifestError> {
        let mut errors = Vec::new();
        let roots: Vec<&str> = self.runtime_roots.iter().map(String::as_str).collect();

        for entry in &self.installed {
            let dest = Path::new(&entry.destination);
            let under_root = roots.iter().any(|root| dest.starts_with(root));
            if !under_root {
                errors.push(ManifestError::DestinationOutsideRuntimeRoots {
                    logical_id: entry.logical_id.clone(),
                    destination: entry.destination.clone(),
                });
            }
        }

        for diag in &self.diagnostics {
            if diag.installed {
                errors.push(ManifestError::DiagnosticMarkedInstalled {
                    path: diag.path.clone(),
                });
            }
        }

        errors
    }

    /// Filesystem check: every installed `source` must exist under `base`.
    /// Use with a staged fixture directory to catch a manifest that references
    /// files the publish run never produced.
    pub fn validate_sources(&self, base: &Path) -> Vec<ManifestError> {
        self.installed
            .iter()
            .filter(|e| !base.join(&e.source).exists())
            .map(|e| ManifestError::MissingInstalledSource {
                logical_id: e.logical_id.clone(),
                source: e.source.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PublishManifest {
        PublishManifest {
            schema_version: 1,
            profile: "dev".into(),
            generated_at: "fixture".into(),
            runtime_roots: vec!["crates/ambition_actors/assets/sprites".into()],
            installed: vec![InstalledEntry {
                logical_id: "sprite.goblin.high.sheet_record".into(),
                kind: "sheet_record".into(),
                quality: Quality::High,
                source: "high/goblin_spritesheet.ron".into(),
                destination: "crates/ambition_actors/assets/sprites/goblin_spritesheet.ron".into(),
            }],
            diagnostics: vec![DiagnosticEntry {
                kind: "preview_sheet".into(),
                path: "diagnostics/goblin_preview_labeled.png".into(),
                installed: false,
            }],
        }
    }

    #[test]
    fn round_trips_through_ron() {
        let manifest = sample();
        let text = manifest.to_ron().unwrap();
        let parsed = PublishManifest::parse(&text).unwrap();
        assert_eq!(manifest, parsed);
    }

    #[test]
    fn clean_manifest_has_no_shape_errors() {
        assert!(sample().validate_shape().is_empty());
    }

    #[test]
    fn rejects_destination_outside_runtime_roots() {
        let mut m = sample();
        m.installed[0].destination = "some/other/place/goblin_spritesheet.ron".into();
        let errors = m.validate_shape();
        assert!(matches!(
            errors.as_slice(),
            [ManifestError::DestinationOutsideRuntimeRoots { .. }]
        ));
    }

    #[test]
    fn rejects_diagnostic_marked_installed() {
        let mut m = sample();
        m.diagnostics[0].installed = true;
        let errors = m.validate_shape();
        assert!(matches!(
            errors.as_slice(),
            [ManifestError::DiagnosticMarkedInstalled { .. }]
        ));
    }
}
