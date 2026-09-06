//! The dispatcher: route one declarative [`Policy`] to its rule kind, appending
//! any violations to the scope's [`Report`]. Kept tiny on purpose — all the
//! real work lives in [`crate::rules`].

use crate::model::{Policy, Report, RuleKind};
use crate::rules;
use crate::workspace::Workspace;

/// Run one policy against the workspace.
pub fn dispatch(ws: &Workspace, policy: &Policy, report: &mut Report) {
    match policy.kind {
        RuleKind::RequiredPath => rules::paths::required(ws, policy, report),
        RuleKind::ForbiddenPath => rules::paths::forbidden(ws, policy, report),
        RuleKind::WorkspaceMember => rules::workspace_member::check(ws, policy, report),
        RuleKind::DependencyAllowlist => rules::dependency::allowlist(ws, policy, report),
        RuleKind::DependencyDenylist => rules::dependency::denylist(ws, policy, report),
        RuleKind::ForbiddenSourceReference => rules::source_reference::check(ws, policy, report),
        RuleKind::FileContains => rules::file_content::contains(ws, policy, report),
        RuleKind::FileOmits => rules::file_content::omits(ws, policy, report),
    }
}
