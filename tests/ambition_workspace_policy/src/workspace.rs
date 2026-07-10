//! Centralized workspace inspection: repository-root discovery, workspace-member
//! parsing, manifest dependency parsing, Rust source walking, and the shared
//! scanning primitives every rule reuses. One home for these means a rule file
//! is just its policy logic, never a re-implemented directory walk.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::model::{Policy, Scope};

/// A discovered workspace. Carries the repository root and (lazily-read) member
/// data so a rule resolves repo-relative paths against one place.
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// Discover the workspace by walking up from this package's manifest dir
    /// until a `Cargo.toml` declaring `[workspace]` is found. Centralizes
    /// root discovery so nothing reconstructs paths by prepending `crates/`.
    pub fn discover() -> Self {
        let start = Path::new(env!("CARGO_MANIFEST_DIR"));
        for dir in start.ancestors() {
            let manifest = dir.join("Cargo.toml");
            if manifest.is_file() {
                if let Ok(text) = std::fs::read_to_string(&manifest) {
                    if text.contains("[workspace]") {
                        return Workspace {
                            root: dir.to_path_buf(),
                        };
                    }
                }
            }
        }
        panic!(
            "could not find the workspace root above {} — no ancestor Cargo.toml declares [workspace]",
            start.display()
        );
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Absolute path for a repo-relative path.
    pub fn abs(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    /// The registered workspace members, exactly as listed in the root
    /// `[workspace] members` array (repo-relative dir paths).
    pub fn member_dirs(&self) -> Vec<String> {
        parse_workspace_members(&self.read_root_manifest())
    }

    /// The set of workspace crate NAMES (each member's `package.name`). Owners
    /// and `workspace-member` rules are stated as crate names, so this is the
    /// authority a name is checked against.
    pub fn member_names(&self) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        for dir in self.member_dirs() {
            let manifest = self.abs(&dir).join("Cargo.toml");
            if let Ok(text) = std::fs::read_to_string(&manifest) {
                if let Some(name) = package_name(&text) {
                    names.insert(name);
                }
            }
        }
        names
    }

    fn read_root_manifest(&self) -> String {
        std::fs::read_to_string(self.root.join("Cargo.toml")).expect("read root Cargo.toml")
    }

    /// The `ambition*` dependency names of a manifest, unioned across the
    /// standard dependency tables (`dependencies`, `dev-dependencies`,
    /// `build-dependencies`, and their `target.<cfg>.*` forms). Parsed TOML, not
    /// a line grep — an optional/renamed/table dep is just a key.
    pub fn ambition_deps(&self, manifest_rel: &str) -> BTreeSet<String> {
        let text = std::fs::read_to_string(self.abs(manifest_rel)).unwrap_or_else(|e| {
            panic!("read manifest `{manifest_rel}`: {e}");
        });
        ambition_deps_of(&text)
    }

    /// Every non-`target/` `.rs` file under a repo-relative root, sorted.
    pub fn rust_sources(&self, root_rel: &str) -> Vec<PathBuf> {
        rust_sources_under(&self.abs(root_rel))
    }
}

/// Parse the `[workspace] members = [...]` array. Robust to comments and
/// trailing commas by reading string literals between the brackets.
fn parse_workspace_members(root_manifest: &str) -> Vec<String> {
    let Some(ws_start) = root_manifest.find("[workspace]") else {
        return Vec::new();
    };
    let after = &root_manifest[ws_start..];
    let Some(m_start) = after.find("members") else {
        return Vec::new();
    };
    let after_members = &after[m_start..];
    let Some(open) = after_members.find('[') else {
        return Vec::new();
    };
    let Some(close) = after_members[open..].find(']') else {
        return Vec::new();
    };
    let list = &after_members[open + 1..open + close];
    list.split(',')
        .filter_map(|tok| {
            let tok = tok.trim();
            let tok = tok.strip_prefix('#').map_or(tok, |_| "");
            let tok = tok.trim_matches('"').trim();
            if tok.is_empty() {
                None
            } else {
                Some(tok.to_string())
            }
        })
        .collect()
}

/// The `package.name` of a manifest, via parsed TOML.
fn package_name(manifest: &str) -> Option<String> {
    let table: toml::Table = manifest.parse().ok()?;
    table
        .get("package")?
        .as_table()?
        .get("name")?
        .as_str()
        .map(str::to_string)
}

/// Collect `ambition`/`ambition_*` dependency names from every standard
/// dependency table of a parsed manifest.
fn ambition_deps_of(manifest: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Ok(table) = manifest.parse::<toml::Table>() else {
        return out;
    };
    let is_ambition = |name: &str| name == "ambition" || name.starts_with("ambition_");

    let mut collect = |t: Option<&toml::Value>| {
        if let Some(deps) = t.and_then(|v| v.as_table()) {
            for name in deps.keys() {
                if is_ambition(name) {
                    out.insert(name.clone());
                }
            }
        }
    };

    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        collect(table.get(key));
    }
    // `target.<cfg>.{dependencies,dev-dependencies,build-dependencies}`.
    if let Some(targets) = table.get("target").and_then(|v| v.as_table()) {
        for cfg in targets.values() {
            if let Some(cfg_t) = cfg.as_table() {
                for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
                    collect(cfg_t.get(key));
                }
            }
        }
    }
    out
}

/// Recursively collect `.rs` files under `dir`, skipping any `target` dir.
pub fn rust_sources_under(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| e.file_name() != "target")
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    files
}

// ── central production/test path classification ──────────────────────────────

/// Whether a workspace-relative `.rs` path is a TEST module rather than a
/// production module. THE single classifier — the module-size gate uses it to
/// keep test LOC out of the production count, so extracting a large inline test
/// into an adjacent `src/foo/tests.rs` (Task 11) is reclassified automatically,
/// by explicit path shape, not a filename-substring guess:
///   * a standalone integration test lives under a `tests/` directory;
///   * an adjacent unit-test module is `…/tests.rs` (or `…/tests/*.rs`).
/// Inline `#[cfg(test)]` modules are intentionally NOT excluded here — they count
/// toward their file's size (a 3.7k-line file is hard to navigate no matter how
/// much of it is tests, and this matches how audit H6 counted).
///
/// Explicit path rules, anchored on the basename, not a loose `contains("test")`
/// heuristic — so a production `attests.rs` is NOT misread as a test:
///   * an adjacent unit-test module: basename exactly `tests.rs`;
///   * a sibling test file: basename ends with `_tests.rs` (legacy convention);
///   * anything under a `tests/` directory.
pub fn is_test_path(rel: &str) -> bool {
    let rel = rel.replace('\\', "/");
    let base = rel.rsplit('/').next().unwrap_or(&rel);
    base == "tests.rs"
        || base.ends_with("_tests.rs")
        || rel.contains("/tests/")
        || rel.starts_with("tests/")
}

// ── shared scanning primitives ───────────────────────────────────────────────

/// A whole-line comment (`//`, `/*`, `*`, doc lines). Prose that names a
/// forbidden identifier must never trip a source scan.
pub fn is_comment_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("/*") || t.starts_with('*')
}

/// Strip a trailing line comment so a name mentioned in a `//` comment on a code
/// line is not matched. String-literal-aware parsing is unnecessary here: the
/// forbidden names are type/crate identifiers, and the reviewed string-data
/// exceptions are handled by `allow_lines`.
pub fn code_only(line: &str) -> &str {
    match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
    }
}

/// The production half of a file: everything before the first `#[cfg(test)]`.
pub fn production_slice(text: &str) -> &str {
    text.split("#[cfg(test)]")
        .next()
        .expect("split yields at least one piece")
}

fn is_ident_char(c: u8) -> bool {
    c == b'_' || c.is_ascii_alphanumeric()
}

/// Whole-identifier containment: `needle` appears in `hay` bounded by non-ident
/// characters (or string edges) on both sides.
pub fn contains_ident(hay: &str, needle: &str) -> bool {
    let bytes = hay.as_bytes();
    let mut start = 0;
    while let Some(pos) = hay[start..].find(needle) {
        let at = start + pos;
        let end = at + needle.len();
        let left_ok = at == 0 || !is_ident_char(bytes[at - 1]);
        let right_ok = end == bytes.len() || !is_ident_char(bytes[end]);
        if left_ok && right_ok {
            return true;
        }
        start = at + 1;
    }
    false
}

// ── policy loading ───────────────────────────────────────────────────────────

/// The `policies/` directory of this package (compile-time constant).
pub fn policies_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("policies")
}

#[derive(serde::Deserialize)]
struct PolicyFile {
    #[serde(default)]
    policy: Vec<Policy>,
}

/// Load the declarative policies for one scope from its `policies/*.toml` file.
/// Every loaded policy is asserted to actually carry that scope (a policy in the
/// wrong file is a bug, not a silent skip).
pub fn load_scope_policies(scope: Scope) -> Vec<Policy> {
    let path = policies_dir().join(scope.policy_file());
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read policy file {}: {e}", path.display()));
    let parsed: PolicyFile = toml::from_str(&text)
        .unwrap_or_else(|e| panic!("parse policy file {}: {e}", path.display()));
    for p in &parsed.policy {
        assert_eq!(
            p.scope,
            scope,
            "policy `{}` is in {} but declares scope `{}`",
            p.id,
            scope.policy_file(),
            p.scope.label()
        );
    }
    parsed.policy
}

/// Load every declarative policy across all three scope files (used by the
/// self-tests that validate owners/watch-paths/roots for the whole set).
pub fn load_all_policies() -> Vec<Policy> {
    [Scope::Repository, Scope::Engine, Scope::Game]
        .into_iter()
        .flat_map(load_scope_policies)
        .collect()
}
