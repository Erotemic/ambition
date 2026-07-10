//! `workspace-member`: every named crate must be a registered workspace member.
//! Guards "the extracted crate is actually in the build", not just on disk.

use crate::model::{Policy, Report};
use crate::workspace::Workspace;

pub fn check(ws: &Workspace, policy: &Policy, report: &mut Report) {
    assert!(
        !policy.members.is_empty(),
        "policy `{}` (workspace-member) lists no members — vacuous",
        policy.id
    );
    let members = ws.member_names();
    let member_dirs = ws.member_dirs();
    for name in &policy.members {
        // A member may be named by crate name (`package.name`) or by its
        // registered directory path (`crates/ambition_foo`). Accept either.
        let by_name = members.contains(name);
        let by_dir = member_dirs.iter().any(|d| d == name || d.ends_with(name));
        if !by_name && !by_dir {
            report.push(policy.diag(name.clone(), "is not a registered workspace member"));
        }
    }
}
