//! The policy data model: what a policy IS, and what a failure looks like.

use serde::Deserialize;

/// Which slice of the workspace a policy belongs to. The scope decides which
/// integration test runs the policy, so scopes stay independently filterable
/// (`cargo test -p ambition_workspace_policy engine_policies`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    /// Whole-repository structure: workspace membership, top-level layout,
    /// umbrella/demo homes, cross-crate composition not owned by one crate.
    Repository,
    /// Engine crates (`crates/*`): layering, foundation purity, determinism,
    /// control-frame ownership, module-size.
    Engine,
    /// Game / content / app crates (`game/*`): content ownership, app
    /// composition, named-content registration.
    Game,
}

impl Scope {
    pub fn label(self) -> &'static str {
        match self {
            Scope::Repository => "repository",
            Scope::Engine => "engine",
            Scope::Game => "game",
        }
    }

    /// The policy file that carries this scope's declarative policies.
    pub fn policy_file(self) -> &'static str {
        match self {
            Scope::Repository => "repository.toml",
            Scope::Engine => "engine.toml",
            Scope::Game => "game.toml",
        }
    }
}

/// How severe a violation is. Everything is `Error` today; `Warn` exists so a
/// future ratchet can land advisory before it bites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Error,
    Warn,
}

impl Severity {
    pub(crate) fn error() -> Self {
        Severity::Error
    }
}

/// The declarative rule kinds. Each maps to one function under [`crate::rules`].
/// Deliberately small: add a kind only when a migration needs it (do not design
/// a universal policy language in advance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleKind {
    /// Every path in `paths` must exist (repo-relative).
    RequiredPath,
    /// No path in `paths` may exist (repo-relative).
    ForbiddenPath,
    /// Every name in `members` must be a registered workspace member.
    WorkspaceMember,
    /// The `ambition*` dependencies of `manifest` must be a subset of `allow`.
    DependencyAllowlist,
    /// `manifest` must not depend on any crate in `deny`.
    DependencyDenylist,
    /// No `.rs` file under `roots` may name any identifier in `forbid`.
    ForbiddenSourceReference,
}

/// One declarative policy, parsed from a `policies/*.toml` `[[policy]]` entry.
///
/// The metadata block (`id`, `scope`, `owners`, `watch_paths`, `kind`,
/// `rationale`, `source_doc`, `severity`) is common to every kind; the
/// kind-specific fields below are validated against `kind` by the rule module.
/// `deny_unknown_fields` turns a typo in a policy file into a parse error rather
/// than a silently ignored rule.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Stable, searchable identifier (e.g. `engine.host-names-no-content`).
    /// Derived from the old test name where one existed, so `git log -S` still
    /// finds the history.
    pub id: String,
    pub scope: Scope,
    pub kind: RuleKind,
    /// Workspace crate(s) that own this invariant. Every owner must be a real
    /// workspace package (self-tested).
    #[serde(default)]
    pub owners: Vec<String>,
    /// Repo-relative paths whose change should re-run this policy. Every watch
    /// path must exist (self-tested). Enables a future `xtask test-affected`.
    #[serde(default)]
    pub watch_paths: Vec<String>,
    /// Why the rule exists — printed on failure.
    pub rationale: String,
    /// The doc that decided the invariant — printed on failure.
    #[serde(default)]
    pub source_doc: String,
    #[serde(default = "Severity::error")]
    pub severity: Severity,

    // ── kind-specific fields (validated against `kind`) ──────────────────────
    /// `required-path` / `forbidden-path`: repo-relative paths.
    #[serde(default)]
    pub paths: Vec<String>,
    /// `workspace-member`: crate names that must be registered members.
    #[serde(default)]
    pub members: Vec<String>,
    /// `dependency-*`: repo-relative path to the `Cargo.toml` to inspect.
    #[serde(default)]
    pub manifest: Option<String>,
    /// `dependency-allowlist`: the complete set of allowed `ambition*` deps.
    #[serde(default)]
    pub allow: Vec<String>,
    /// `dependency-denylist`: crate names that must not be depended on.
    #[serde(default)]
    pub deny: Vec<String>,
    /// `forbidden-source-reference`: repo-relative dirs to scan.
    #[serde(default)]
    pub roots: Vec<String>,
    /// `forbidden-source-reference`: identifiers/substrings that must not appear
    /// in code.
    #[serde(default)]
    pub forbid: Vec<String>,
    /// `forbidden-source-reference`: substrings that, if present on a line,
    /// exempt that line (the reviewed cross-boundary exceptions).
    #[serde(default)]
    pub allow_lines: Vec<String>,
    /// `forbidden-source-reference`: path substrings whose files are skipped.
    #[serde(default)]
    pub skip_paths: Vec<String>,
    /// `forbidden-source-reference`: match whole identifiers only (so
    /// `GroundItemVisual` does not trip `GroundItem`). Default is substring.
    #[serde(default)]
    pub whole_ident: bool,
    /// `forbidden-source-reference`: scan only the production half of each file
    /// (everything before the first `#[cfg(test)]`).
    #[serde(default)]
    pub production_only: bool,
    /// `forbidden-source-reference`: a per-line opt-out marker (e.g.
    /// `ALLOW_LEGACY_RUNTIME`); a line carrying it is exempt.
    #[serde(default)]
    pub allow_marker: Option<String>,
}

impl Policy {
    /// A diagnostic pre-filled with this policy's identity/context. Rules only
    /// supply the offending `location` and the `detail` of what is wrong.
    pub fn diag(&self, location: impl Into<String>, detail: impl Into<String>) -> Diagnostic {
        Diagnostic {
            policy_id: self.id.clone(),
            owners: self.owners.clone(),
            source_doc: self.source_doc.clone(),
            rationale: self.rationale.clone(),
            location: location.into(),
            detail: detail.into(),
        }
    }
}

/// One policy violation. Carries everything a reader needs to act without
/// opening the runner: the policy ID, the owners, the offending
/// file/dependency/path, the rationale, and the source doc.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub policy_id: String,
    pub owners: Vec<String>,
    pub source_doc: String,
    pub rationale: String,
    pub location: String,
    pub detail: String,
}

/// A scope's collected diagnostics. `assert_ok` turns them into a single
/// grouped, actionable panic — one failure per policy, each naming its owners,
/// rationale, source doc, and every offending location.
#[derive(Debug)]
pub struct Report {
    scope: Scope,
    diags: Vec<Diagnostic>,
}

impl Report {
    pub fn new(scope: Scope) -> Self {
        Report {
            scope,
            diags: Vec::new(),
        }
    }

    pub fn push(&mut self, diag: Diagnostic) {
        self.diags.push(diag);
    }

    pub fn is_empty(&self) -> bool {
        self.diags.is_empty()
    }

    pub fn len(&self) -> usize {
        self.diags.len()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diags
    }

    /// Panic with a grouped, actionable report if anything was found. Called at
    /// the end of every scope test.
    pub fn assert_ok(self) {
        if self.diags.is_empty() {
            return;
        }
        panic!("{}", self.render());
    }

    /// The formatted failure text (also used by self-tests that assert a rule
    /// reacts to poison without aborting the process).
    pub fn render(&self) -> String {
        use std::collections::BTreeMap;
        let mut by_policy: BTreeMap<&str, Vec<&Diagnostic>> = BTreeMap::new();
        for d in &self.diags {
            by_policy.entry(&d.policy_id).or_default().push(d);
        }
        let mut out = format!(
            "\n{} policy failure(s) in the `{}` scope \
             (tests/ambition_workspace_policy/policies/{}):\n",
            self.diags.len(),
            self.scope.label(),
            self.scope.policy_file(),
        );
        for (id, ds) in by_policy {
            let head = ds[0];
            out.push_str(&format!("\n── policy `{id}` ──\n"));
            if !head.owners.is_empty() {
                out.push_str(&format!("   owners:     {}\n", head.owners.join(", ")));
            }
            if !head.source_doc.is_empty() {
                out.push_str(&format!("   source_doc: {}\n", head.source_doc));
            }
            out.push_str(&format!("   rationale:  {}\n", head.rationale));
            out.push_str("   violations:\n");
            for d in ds {
                out.push_str(&format!("     - {}: {}\n", d.location, d.detail));
            }
        }
        out
    }
}
