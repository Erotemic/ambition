//! References — the two safe forms, and the one that is not.
//!
//! ```text
//! ContentRef<T>                       generated/typed, for shipped Rust consumers
//! UnresolvedContentRef<T> → ResolvedContentRef<T>   for tools, authored data, mods
//! ```
//!
//! raw runtime string lookup must not become gameplay authority. A
//! `HashMap<String, _>` consulted during a tick is a reference that was never
//! validated, and its failure mode is a silent default — which this repo has
//! now paid for twice in one week (a CPU seat naming a brain profile the
//! composition never had, and a demo naming another provider's archetype row).
//! Resolution happens ONCE, at compile time, and what reaches the runtime is a
//! [`ResolvedContentRef`] that cannot be constructed without a target.

use std::marker::PhantomData;

use crate::diagnostic::{CompileStage, Diagnostic, DiagnosticCode};
use crate::identity::{ContentId, SchemaId};

/// A Rust type that names an authored content family.
///
/// Implement it on a marker beside the runtime type a capability lowers to;
/// `UnresolvedContentRef<Character>` then reads as what it is at every call
/// site, and a ref to the wrong family stops compiling instead of resolving to
/// nothing at runtime.
pub trait ContentKind {
    /// The schema whose entries this reference points into.
    const SCHEMA: &'static str;
    /// What to call it in a diagnostic ("character", "preset", "world").
    const NOUN: &'static str;
}

/// A reference as authored: a name, not yet known to point at anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedContentRef<T: ContentKind> {
    pub name: String,
    kind: PhantomData<fn() -> T>,
}

impl<T: ContentKind> UnresolvedContentRef<T> {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: PhantomData,
        }
    }

    pub fn schema(&self) -> SchemaId {
        SchemaId::new(T::SCHEMA)
    }

    /// Erase to the form the compiler resolves in bulk. `declared_by` and
    /// `field` are what turn "unresolved reference" into a sentence somebody
    /// can act on without opening the compiler.
    pub fn pending(self, declared_by: ContentId, field: &'static str) -> PendingRef {
        PendingRef {
            schema: self.schema(),
            name: self.name,
            noun: T::NOUN,
            declared_by,
            field,
            local: false,
        }
    }
}

/// A reference the compiler proved. The only constructor is resolution, so
/// holding one IS the proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedContentRef<T: ContentKind> {
    target: ContentId,
    kind: PhantomData<fn() -> T>,
}

impl<T: ContentKind> ResolvedContentRef<T> {
    pub(crate) fn prove(target: ContentId) -> Self {
        Self {
            target,
            kind: PhantomData,
        }
    }

    pub fn target(&self) -> &ContentId {
        &self.target
    }

    pub fn name(&self) -> &str {
        &self.target.name
    }
}

/// The dynamic form the compiler works in. Handlers emit these; the reference
/// resolution stage matches them against everything the pack defined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingRef {
    pub schema: SchemaId,
    pub name: String,
    /// "character", "preset", "world" — used in the message so the refusal
    /// reads as English rather than as a schema id.
    pub noun: &'static str,
    /// The content that declared it.
    pub declared_by: ContentId,
    /// The authored field it was declared in.
    pub field: &'static str,
    /// A LOCAL reference points inside the same source (a character naming a
    /// preset defined in the same catalog file), so an unresolved one is a
    /// typo rather than a missing dependency — and the fix line differs.
    pub local: bool,
}

impl PendingRef {
    pub fn new(
        schema: SchemaId,
        name: impl Into<String>,
        noun: &'static str,
        declared_by: ContentId,
        field: &'static str,
    ) -> Self {
        Self {
            schema,
            name: name.into(),
            noun,
            declared_by,
            field,
            local: false,
        }
    }

    /// Mark this as pointing inside the same authored source.
    pub fn local(mut self) -> Self {
        self.local = true;
        self
    }

    /// The identity this would resolve to in `namespace`.
    pub fn target_in(&self, namespace: &crate::identity::ModuleNamespace) -> ContentId {
        ContentId::new(namespace, &self.schema, self.name.clone())
    }

    pub(crate) fn unresolved(&self, available: &[String]) -> Diagnostic {
        let code = if self.local {
            DiagnosticCode::UnknownPreset
        } else {
            DiagnosticCode::UnresolvedReference
        };
        let mut diagnostic = Diagnostic::error(
            code,
            CompileStage::ReferenceResolution,
            format!(
                "`{}` names {} `{}`, which this pack does not define",
                self.declared_by, self.noun, self.name
            ),
        )
        .about(self.declared_by.clone())
        .at_field(self.field);
        if let Some(near) = nearest(&self.name, available) {
            diagnostic = diagnostic.fix(format!("did you mean `{near}`?"));
        }
        diagnostic.fix(if self.local {
            format!(
                "define `{}` in the same source, or point `{}` at one that exists",
                self.name, self.field
            )
        } else {
            format!(
                "add a `{}` entry named `{}`, or point `{}` at one that exists",
                self.schema, self.name, self.field
            )
        })
    }
}

/// An authored asset path, with the content that asked for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetRequirement {
    /// The path as authored, relative to an asset root.
    pub path: String,
    pub declared_by: ContentId,
    pub field: &'static str,
}

impl AssetRequirement {
    pub fn new(path: impl Into<String>, declared_by: ContentId, field: &'static str) -> Self {
        Self {
            path: path.into(),
            declared_by,
            field,
        }
    }
}

/// Where a prepared asset was found — provenance, so a packaging step can copy
/// exactly what the pack depends on and an inspector can say which root won.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetProvenance {
    pub path: String,
    /// The root it resolved under, as the resolver reported it.
    pub root: String,
    /// Every content id that asked for this asset, in canonical order.
    pub required_by: Vec<ContentId>,
}

/// Where the compiler looks for authored assets.
///
/// A trait because the answer differs per caller and each answer is legitimate:
/// the CLI walks real directories, a unit test names three strings, and a
/// schema-only check declines to look at all. Making that a choice the CALLER
/// states beats a compiler that silently skips asset checks when it cannot find
/// a root — which reads identical to "every asset is present".
pub trait AssetSource: Send + Sync {
    /// The root's name, for provenance. Not a path unless a path is meaningful.
    fn label(&self) -> String;
    /// Whether `path` exists under this source.
    fn contains(&self, path: &str) -> bool;

    /// How hard a missing asset is.
    ///
    /// this is a real choice, not a knob. AGENTS.md's stance is that
    /// binary payloads are git-ignored but present, and that a feature owes
    /// only "degrade visibly when a file is absent" — so on a fresh clone,
    /// missing art is a DOCUMENTED state, not a broken pack. A compiler that
    /// refused it by default would contradict a project rule and get waived,
    /// which this repo has already learned costs more than the check is worth.
    ///
    /// So: [`DirectoryAssets`] refuses (packaging, release, CI-with-art), and
    /// [`AdvisoryAssets`] warns (an ordinary content edit on an ordinary
    /// checkout). Both REPORT; they differ only in whether the pack compiles.
    fn severity(&self) -> crate::diagnostic::Severity {
        crate::diagnostic::Severity::Error
    }
}

/// Every asset is missing. Useful only to prove the missing-asset path.
pub struct NoAssets;

impl AssetSource for NoAssets {
    fn label(&self) -> String {
        "<no asset roots>".into()
    }
    fn contains(&self, _path: &str) -> bool {
        false
    }
}

/// Asset checks are not performed. Explicit, so a pack prepared this way is
/// visibly not making a claim about its assets.
pub struct AssetsUnchecked;

impl AssetSource for AssetsUnchecked {
    fn label(&self) -> String {
        "<unchecked>".into()
    }
    fn contains(&self, _path: &str) -> bool {
        true
    }
}

/// Real directories, tried in order. First hit wins and is recorded.
pub struct DirectoryAssets {
    pub roots: Vec<std::path::PathBuf>,
}

impl DirectoryAssets {
    pub fn new(roots: impl IntoIterator<Item = std::path::PathBuf>) -> Self {
        Self {
            roots: roots.into_iter().collect(),
        }
    }
}

impl AssetSource for DirectoryAssets {
    fn label(&self) -> String {
        self.roots
            .iter()
            .map(|r| r.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn contains(&self, path: &str) -> bool {
        self.roots.iter().any(|root| root.join(path).exists())
    }
}

/// Real directories, but a miss is a WARNING and the pack still compiles.
///
/// The default for interactive validation: an author editing a character's
/// stats on a checkout whose art was never generated wants to hear about the
/// missing sheet, and does not want their edit called invalid because of it.
pub struct AdvisoryAssets(pub DirectoryAssets);

impl AdvisoryAssets {
    pub fn new(roots: impl IntoIterator<Item = std::path::PathBuf>) -> Self {
        Self(DirectoryAssets::new(roots))
    }
}

impl AssetSource for AdvisoryAssets {
    fn label(&self) -> String {
        self.0.label()
    }
    fn contains(&self, path: &str) -> bool {
        self.0.contains(path)
    }
    fn severity(&self) -> crate::diagnostic::Severity {
        crate::diagnostic::Severity::Warning
    }
}

/// A named set, for tests that want the asset stage exercised without a disk.
pub struct FixedAssets(pub std::collections::BTreeSet<String>);

impl FixedAssets {
    pub fn new<I, S>(paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self(paths.into_iter().map(Into::into).collect())
    }
}

impl AssetSource for FixedAssets {
    fn label(&self) -> String {
        "<fixed set>".into()
    }
    fn contains(&self, path: &str) -> bool {
        self.0.contains(path)
    }
}

/// Cheap "did you mean" over a candidate list: shortest edit distance, and only
/// when it is close enough to be worth saying.
pub(crate) fn nearest<'a>(needle: &str, haystack: &'a [String]) -> Option<&'a str> {
    let budget = (needle.len() / 3).max(2);
    haystack
        .iter()
        .map(|candidate| (edit_distance(needle, candidate), candidate))
        .filter(|(distance, _)| *distance <= budget)
        .min_by_key(|(distance, candidate)| (*distance, candidate.len()))
        .map(|(_, candidate)| candidate.as_str())
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut row = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            row[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(row[j] + 1);
        }
        std::mem::swap(&mut prev, &mut row);
    }
    prev[b.len()]
}
