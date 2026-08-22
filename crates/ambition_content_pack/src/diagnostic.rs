//! Structured diagnostics — the compiler's output when it refuses.
//!
//! A diagnostic is a VALUE with a code, a subject and a stage, not a formatted
//! string. Tools filter by code; the CLI renders; a test asserts on the code
//! rather than on prose, so improving a message never breaks a test.
//!
//! ## Every refusal names the stage it stopped at
//!
//! Learned the expensive way one layer up (ADR 0032's composition funnel): a
//! pipeline whose later checks need the earlier ones to have SUCCEEDED cannot
//! report everything at once, and the failure mode is not the funnel — it is
//! the funnel being SILENT. "This is everything" and "this is everything I
//! could see from here" must not look identical, so [`CompileStage`] travels
//! with the refusal and [`CompileFailure::stopped_before`] says what never ran.

use std::fmt;

use crate::identity::ContentId;

/// Where in the pipeline a refusal happened. Ordered: a failure at stage N
/// means every check owned by stages after N did not run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompileStage {
    /// Reading the manifest and the sources off disk; RON syntax.
    Parse,
    /// Every declared schema resolves to an installed, version-compatible
    /// handler.
    SchemaResolution,
    /// Every capability the pack (or a schema) requires is installed.
    CapabilityValidation,
    /// Handlers read their facets and declare what they define and need.
    FacetValidation,
    /// A schema lowered by SEVERAL sources merges their fragments into the one
    /// artifact the runtime consumes, and judges the aggregate.
    ///
    /// Its own stage because it is the first check that can only be made once
    /// every source of a schema has been read — a per-facet handler cannot see
    /// two files at all, so "these nine encounters have two claiming the same
    /// id" has nowhere else to live.
    Aggregation,
    /// Every declared reference finds its target.
    ReferenceResolution,
    /// Duplicate identities, conflicting module contributions.
    ConflictDetection,
}

impl CompileStage {
    pub const ORDER: [Self; 7] = [
        Self::Parse,
        Self::SchemaResolution,
        Self::CapabilityValidation,
        Self::FacetValidation,
        Self::Aggregation,
        Self::ReferenceResolution,
        Self::ConflictDetection,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::SchemaResolution => "schema resolution",
            Self::CapabilityValidation => "capability validation",
            Self::FacetValidation => "facet validation",
            Self::Aggregation => "aggregation",
            Self::ReferenceResolution => "reference resolution",
            Self::ConflictDetection => "conflict detection",
        }
    }

    /// The stages that never ran because this one refused.
    pub fn later_stages(self) -> Vec<Self> {
        Self::ORDER.into_iter().filter(|s| *s > self).collect()
    }
}

impl fmt::Display for CompileStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// What went wrong, as a code a tool can branch on.
///
/// `UnknownField` is an ERROR, not a warning. Silently ignoring an
/// unconsumed authored field is how a typo becomes a mechanic that never fires,
/// which is the single most expensive class of content bug: everything looks
/// authored and nothing happens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticCode {
    /// The manifest or a source is not readable / not valid RON.
    MalformedSource,
    /// A source declares a schema no installed capability owns.
    UnknownSchema,
    /// The schema is installed at a different version than the source declares.
    SchemaVersionMismatch,
    /// Two registrations claim the same schema id.
    AmbiguousSchemaOwnership,
    /// The pack (or a schema it uses) requires a capability nobody installed.
    MissingCapability,
    /// An authored field names a preset that does not exist.
    UnknownPreset,
    /// An authored field names a character/role/action/world that does not exist.
    UnresolvedReference,
    /// An authored asset path resolves to no file under any asset root.
    MissingAsset,
    /// Two sources define the same canonical identity.
    DuplicateIdentity,
    /// Two module contributions disagree (same key, different value).
    ConflictingModuleContribution,
    /// An authored field no schema consumes.
    UnknownField,
    /// A provider binding is present but not usable (empty id, wrong shape).
    MalformedProviderBinding,
    /// Two declared sources resolve to the same file through aliases/symlinks
    /// with DIFFERENT declared schemas, so deduplication would have to guess.
    ConflictingSourceAlias,
}

impl DiagnosticCode {
    pub fn label(self) -> &'static str {
        match self {
            Self::MalformedSource => "malformed-source",
            Self::UnknownSchema => "unknown-schema",
            Self::SchemaVersionMismatch => "schema-version-mismatch",
            Self::AmbiguousSchemaOwnership => "ambiguous-schema-ownership",
            Self::MissingCapability => "missing-capability",
            Self::UnknownPreset => "unknown-preset",
            Self::UnresolvedReference => "unresolved-reference",
            Self::MissingAsset => "missing-asset",
            Self::DuplicateIdentity => "duplicate-identity",
            Self::ConflictingModuleContribution => "conflicting-module-contribution",
            Self::UnknownField => "unknown-field",
            Self::MalformedProviderBinding => "malformed-provider-binding",
            Self::ConflictingSourceAlias => "conflicting-source-alias",
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// One structured finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub stage: CompileStage,
    /// What the finding is ABOUT: the content identity, when there is one.
    pub subject: Option<ContentId>,
    /// The declared source path the finding came from, verbatim as authored so
    /// a reader can find the line in the file they edited.
    pub source: Option<String>,
    /// The authored field, when the finding is about one.
    pub field: Option<String>,
    pub message: String,
    /// Concrete next actions. Every fix the compiler can see goes here, not
    /// just the first — a refusal that names one of three fixes is a funnel a
    /// consumer walks one rebuild at a time.
    pub fixes: Vec<String>,
}

impl Diagnostic {
    pub fn error(code: DiagnosticCode, stage: CompileStage, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            stage,
            subject: None,
            source: None,
            field: None,
            message: message.into(),
            fixes: Vec::new(),
        }
    }

    pub fn warning(code: DiagnosticCode, stage: CompileStage, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(code, stage, message)
        }
    }

    pub fn about(mut self, subject: ContentId) -> Self {
        self.subject = Some(subject);
        self
    }

    pub fn in_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn at_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fixes.push(fix.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// One line, for a terminal. The structured fields stay available; this is
    /// rendering, never the finding itself.
    pub fn render(&self) -> String {
        let mut out = format!("[{}] {}", self.code, self.message);
        if let Some(subject) = &self.subject {
            out.push_str(&format!("\n    content: {subject}"));
        }
        if let Some(source) = &self.source {
            match &self.field {
                Some(field) => out.push_str(&format!("\n    at: {source} · field `{field}`")),
                None => out.push_str(&format!("\n    at: {source}")),
            }
        }
        for fix in &self.fixes {
            out.push_str(&format!("\n    fix: {fix}"));
        }
        out
    }
}

/// A refused compilation: the stage it stopped at, and every finding it had.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileFailure {
    pub stage: CompileStage,
    pub diagnostics: Vec<Diagnostic>,
}

impl CompileFailure {
    pub fn new(stage: CompileStage, diagnostics: Vec<Diagnostic>) -> Self {
        Self { stage, diagnostics }
    }

    pub fn has(&self, code: DiagnosticCode) -> bool {
        self.diagnostics.iter().any(|d| d.code == code)
    }

    pub fn codes(&self) -> Vec<DiagnosticCode> {
        let mut codes: Vec<_> = self.diagnostics.iter().map(|d| d.code).collect();
        codes.sort();
        codes.dedup();
        codes
    }

    /// The checks that did not run. Say this out loud. A consumer who fixes
    /// everything listed and hits a second wall on the next run was not told
    /// the first list was partial.
    pub fn stopped_before(&self) -> Vec<CompileStage> {
        self.stage.later_stages()
    }

    pub fn render(&self) -> String {
        let mut out = format!(
            "content pack refused at {} — {} problem(s):\n",
            self.stage,
            self.diagnostics.len()
        );
        for diagnostic in &self.diagnostics {
            out.push_str("  ");
            out.push_str(&diagnostic.render().replace('\n', "\n  "));
            out.push('\n');
        }
        let later = self.stopped_before();
        if !later.is_empty() {
            let names: Vec<_> = later.iter().map(|s| s.label()).collect();
            out.push_str(&format!(
                "\nThese checks did NOT run and may report more once the above is \
                 fixed: {}.\nThey depend on this stage succeeding, so the list above is \
                 everything visible from here — not necessarily everything.\n",
                names.join(", ")
            ));
        }
        out
    }
}

impl fmt::Display for CompileFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl std::error::Error for CompileFailure {}
