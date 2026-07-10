//! `required-path` / `forbidden-path`: assert repo-relative paths exist / are
//! absent. The bread-and-butter of "this crate was extracted / this legacy file
//! is gone" ratchets.

use crate::model::{Policy, Report};
use crate::workspace::Workspace;

/// Every path in `policy.paths` must exist.
pub fn required(ws: &Workspace, policy: &Policy, report: &mut Report) {
    assert!(
        !policy.paths.is_empty(),
        "policy `{}` (required-path) lists no paths — an empty required-path scan passes vacuously",
        policy.id
    );
    for rel in &policy.paths {
        if !ws.abs(rel).exists() {
            report.push(policy.diag(rel.clone(), "required path is missing"));
        }
    }
}

/// No path in `policy.paths` may exist.
pub fn forbidden(ws: &Workspace, policy: &Policy, report: &mut Report) {
    assert!(
        !policy.paths.is_empty(),
        "policy `{}` (forbidden-path) lists no paths — an empty forbidden-path scan passes vacuously",
        policy.id
    );
    for rel in &policy.paths {
        if ws.abs(rel).exists() {
            report.push(policy.diag(rel.clone(), "forbidden path still exists"));
        }
    }
}
