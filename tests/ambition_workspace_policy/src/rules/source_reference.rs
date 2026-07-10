//! `forbidden-source-reference`: no `.rs` file under `roots` may name any
//! identifier in `forbid`. The workhorse of the architecture guards — it powers
//! "presentation never names live sim state", "host never names content",
//! "foundation crate never names the sim heart", and the legacy-runtime ban.
//!
//! Knobs (all optional, defaulting to the simplest behavior):
//!   * `whole_ident`     — match whole identifiers only (`GroundItemVisual` does
//!                         not trip `GroundItem`); default is substring.
//!   * `production_only` — scan only the pre-`#[cfg(test)]` half of each file.
//!   * `allow_marker`    — a per-line opt-out token (e.g. `ALLOW_LEGACY_RUNTIME`).
//!   * `allow_lines`     — substrings that, if present on a line, exempt it (the
//!                         reviewed cross-boundary / string-data exceptions).
//!   * `skip_paths`      — path substrings whose files are skipped.
//!
//! Comment lines and trailing `//` comments are always stripped first, so prose
//! that names a forbidden type never trips the scan.

use crate::model::{Policy, Report};
use crate::workspace::{self, Workspace};

pub fn check(ws: &Workspace, policy: &Policy, report: &mut Report) {
    assert!(
        !policy.roots.is_empty(),
        "policy `{}` (forbidden-source-reference) lists no roots — vacuous",
        policy.id
    );
    assert!(
        !policy.forbid.is_empty(),
        "policy `{}` (forbidden-source-reference) forbids nothing — vacuous",
        policy.id
    );

    let mut scanned = 0usize;
    for root in &policy.roots {
        for file in ws.rust_sources(root) {
            let rel = file
                .strip_prefix(ws.root())
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            if policy.skip_paths.iter().any(|s| rel.contains(s)) {
                continue;
            }
            if policy.skip_tests && workspace::is_test_path(&rel) {
                continue;
            }
            let text = std::fs::read_to_string(&file).expect("read rust source");
            let scan: &str = if policy.production_only {
                workspace::production_slice(&text)
            } else {
                &text
            };
            scanned += 1;

            for (idx, raw) in scan.lines().enumerate() {
                if workspace::is_comment_line(raw) {
                    continue;
                }
                if let Some(marker) = &policy.allow_marker {
                    if raw.contains(marker.as_str()) {
                        continue;
                    }
                }
                if policy.allow_lines.iter().any(|a| raw.contains(a.as_str())) {
                    continue;
                }
                let code = workspace::code_only(raw);
                for needle in &policy.forbid {
                    let hit = if policy.whole_ident {
                        workspace::contains_ident(code, needle)
                    } else {
                        code.contains(needle.as_str())
                    };
                    if hit {
                        report.push(policy.diag(
                            format!("{}:{}", rel, idx + 1),
                            format!("names forbidden `{needle}`: {}", raw.trim()),
                        ));
                    }
                }
            }
        }
    }

    // A source scan that reads zero files is a green that means nothing — the
    // exact false-green shape the whole campaign is guarding against.
    assert!(
        scanned > 0,
        "policy `{}` (forbidden-source-reference) scanned 0 files under {:?} — \
         the roots are wrong and the rule would pass vacuously",
        policy.id,
        policy.roots
    );
}
