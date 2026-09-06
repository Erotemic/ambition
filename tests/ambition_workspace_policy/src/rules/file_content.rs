//! `file-contains` / `file-omits`: single-file raw-text assertions. The workhorse
//! for facade re-export checks ("this file re-exports X"), required plugin
//! composition wiring, and burned-down-facade guards ("this file must not name Y
//! anymore"). Raw `.contains` — matches the original `fs::read + .contains`
//! assertions faithfully (including their treatment of comments as significant).

use crate::model::{Policy, Report};
use crate::workspace::Workspace;

fn file_of(policy: &Policy) -> &str {
    policy
        .file
        .as_deref()
        .unwrap_or_else(|| panic!("policy `{}` (file-* rule) has no `file` path", policy.id))
}

/// `file` must contain every string in `contains`.
pub fn contains(ws: &Workspace, policy: &Policy, report: &mut Report) {
    let file = file_of(policy);
    assert!(
        !policy.contains.is_empty(),
        "policy `{}` (file-contains) lists nothing to require — vacuous",
        policy.id
    );
    let text = std::fs::read_to_string(ws.abs(file))
        .unwrap_or_else(|e| panic!("policy `{}`: read `{file}`: {e}", policy.id));
    for needle in &policy.contains {
        if !text.contains(needle) {
            report.push(policy.diag(
                file.to_string(),
                format!("must contain `{needle}` but does not"),
            ));
        }
    }
}

/// `file` must contain none of the strings in `forbid`.
pub fn omits(ws: &Workspace, policy: &Policy, report: &mut Report) {
    let file = file_of(policy);
    assert!(
        !policy.forbid.is_empty(),
        "policy `{}` (file-omits) forbids nothing — vacuous",
        policy.id
    );
    let text = std::fs::read_to_string(ws.abs(file))
        .unwrap_or_else(|e| panic!("policy `{}`: read `{file}`: {e}", policy.id));
    for needle in &policy.forbid {
        if text.contains(needle) {
            report.push(policy.diag(
                file.to_string(),
                format!("must not name `{needle}` but does"),
            ));
        }
    }
}
