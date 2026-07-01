//! Runtime-root hygiene: scan a runtime asset root and report files that do not
//! belong there. Diagnostics leaking into a runtime root are hard errors;
//! author-time intermediates are warnings (acceptable during migration).
//!
//! This is the validator that gives the publish boundary teeth: even before the
//! whole pipeline routes through the publisher, the hygiene check fails CI if a
//! generator dumps a `*_canonical.png` or `*_preview_labeled.png` back into a
//! runtime root.

use std::io;
use std::path::{Path, PathBuf};

use super::classify::{classify, ArtifactClass};
use super::walk::walk_files;

/// One file flagged by the hygiene scan, with the class that flagged it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HygieneFinding {
    /// Path relative to the scanned runtime root.
    pub path: PathBuf,
    pub class: ArtifactClass,
}

impl std::fmt::Display for HygieneFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({})",
            self.path.display(),
            self.class.manifest_kind()
        )
    }
}

/// The result of scanning one runtime root.
#[derive(Debug, Clone, Default)]
pub struct HygieneReport {
    /// Diagnostics found under the runtime root — a boundary violation.
    pub errors: Vec<HygieneFinding>,
    /// Author-time intermediates found under the runtime root — tolerated
    /// during migration but worth surfacing.
    pub warnings: Vec<HygieneFinding>,
}

impl HygieneReport {
    pub fn is_clean(&self) -> bool {
        self.errors.is_empty()
    }

    /// A multi-line human summary of the errors, for test failure messages.
    pub fn error_summary(&self) -> String {
        self.errors
            .iter()
            .map(|f| format!("  - {f}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn merge(&mut self, other: HygieneReport) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

/// Scan a single runtime root. A non-existent root (e.g. a gitignored quality
/// variant on a fresh clone) yields an empty, clean report.
pub fn scan_runtime_root(root: &Path) -> io::Result<HygieneReport> {
    let mut report = HygieneReport::default();
    for rel in walk_files(root)? {
        match classify(&rel) {
            ArtifactClass::Diagnostic => report.errors.push(HygieneFinding {
                path: rel,
                class: ArtifactClass::Diagnostic,
            }),
            ArtifactClass::Intermediate => report.warnings.push(HygieneFinding {
                path: rel,
                class: ArtifactClass::Intermediate,
            }),
            _ => {}
        }
    }
    Ok(report)
}

/// Scan several runtime roots and merge their reports.
pub fn scan_runtime_roots(roots: &[PathBuf]) -> io::Result<HygieneReport> {
    let mut report = HygieneReport::default();
    for root in roots {
        report.merge(scan_runtime_root(root)?);
    }
    Ok(report)
}
