//! The content-pack compiler.
//!
//! ```text
//! ContentPackDraft
//!     ↓ parse
//!     ↓ schema resolution
//!     ↓ capability validation
//!     ↓ facet validation
//!     ↓ reference resolution
//!     ↓ conflict detection
//!     ↓ canonical ordering
//!     ↓ fingerprint
//! PreparedContentPack
//! ```
//!
//! ## Why this exists
//!
//! Adding or editing ordinary content should not require rebuilding Rust.
//! Before this crate, Ambition's content reached the runtime through
//! `include_str!` and its validator was a fixed-arity function over three
//! hardcoded content families with one `#[cfg(test)]` caller. Both are fine
//! shapes for a shipped binary; neither is a shape an author can iterate in.
//!
//! ADR 0032's public facade already named `ContentPackDraft` as its next slice
//! and deferred it *"because nothing can yet validate it"*. This is that
//! validator.
//!
//! ## The one implementation rule
//!
//! [`compile`] is the ONLY validator. The standard test, CI, the CLI,
//! development reload, and packaging all call it. A second implementation —
//! "the quick check the CLI does" — is how a pack passes one gate and fails
//! another, and the two disagreeing is worse than either being wrong.
//!
//! ## Foundations only
//!
//! No Bevy, no engine domain types, no game content. Capabilities own their
//! schemas and register them ([`SchemaRegistry`]), which is what keeps a new
//! content family from being an edit to one central closed enum.

mod diagnostic;
mod draft;
mod identity;
mod prepared;
mod refs;
mod schema;

pub use diagnostic::{CompileFailure, CompileStage, Diagnostic, DiagnosticCode, Severity};
pub use draft::{ContentPackDraft, ContentPackManifest, SourceDeclaration, SourceFile};
pub use identity::{
    CapabilityId, ContentId, ModuleNamespace, PackId, PackVersion, SchemaId, SchemaVersion,
};
pub use prepared::{
    ContentFingerprint, PreparedContent, PreparedContentPack, PreparedSource, ResolvedReference,
};
pub use refs::{
    AdvisoryAssets, AssetProvenance, AssetRequirement, AssetSource, AssetsUnchecked, ContentKind,
    DirectoryAssets, FixedAssets, NoAssets, PendingRef, ResolvedContentRef, UnresolvedContentRef,
};
pub use schema::{
    ContentSchemaHandler, DefinedContent, FacetOutcome, FacetSource, RuntimeDisposition,
    SchemaRegistration, SchemaRegistry,
};

use std::collections::{BTreeMap, BTreeSet};

use prepared::{PreparedContent as Entry, PreparedSource as Source};

/// Compile a draft against one composition's installed schemas.
///
/// Every stage runs to completion before the next one is judged, so a refusal
/// names every problem that stage could see rather than the first. Stages
/// cannot be merged past a dependency: reference resolution needs every
/// definition, and facet validation needs the handler, so a pack that fails
/// schema resolution genuinely cannot be asked whether its references resolve.
/// [`CompileFailure::stopped_before`] reports exactly that instead of letting a
/// partial list look complete.
pub fn compile(
    draft: &ContentPackDraft,
    registry: &SchemaRegistry,
    assets: &dyn AssetSource,
) -> Result<PreparedContentPack, CompileFailure> {
    let namespace = &draft.manifest.namespace;

    // ── schema resolution ────────────────────────────────────────────────
    let mut diagnostics = Vec::new();
    let mut schemas: BTreeMap<SchemaId, SchemaVersion> = BTreeMap::new();
    for source in &draft.sources {
        let Some(registration) = registry.get(&source.schema) else {
            let known = registry.known_schema_ids();
            let mut diagnostic = Diagnostic::error(
                DiagnosticCode::UnknownSchema,
                CompileStage::SchemaResolution,
                format!(
                    "no installed capability owns schema `{}`",
                    source.schema
                ),
            )
            .in_source(&source.declared_path);
            if let Some(near) = refs::nearest(&source.schema.0, &known) {
                diagnostic = diagnostic.fix(format!("did you mean `{near}`?"));
            }
            diagnostics.push(diagnostic.fix(format!(
                "installed schemas: {}",
                if known.is_empty() {
                    "<none>".to_string()
                } else {
                    known.join(", ")
                }
            )));
            continue;
        };
        if registration.version != source.version {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::SchemaVersionMismatch,
                    CompileStage::SchemaResolution,
                    format!(
                        "`{}` declares schema `{}` {} but the installed handler is {}",
                        source.declared_path, source.schema, source.version, registration.version
                    ),
                )
                .in_source(&source.declared_path)
                .fix(
                    "migrate the source to the installed version — a handler reading a different \
                     version reads different fields, so there is no safe fallback",
                ),
            );
            continue;
        }
        schemas.insert(source.schema.clone(), registration.version);
    }
    if diagnostics.iter().any(Diagnostic::is_error) {
        return Err(CompileFailure::new(
            CompileStage::SchemaResolution,
            diagnostics,
        ));
    }

    // ── capability validation ────────────────────────────────────────────
    let mut required: BTreeSet<CapabilityId> = draft.manifest.requires.iter().cloned().collect();
    for schema in schemas.keys() {
        if let Some(registration) = registry.get(schema) {
            required.insert(registration.capability.clone());
        }
    }
    let mut missing: Vec<&CapabilityId> = required
        .iter()
        .filter(|capability| !registry.has_capability(capability))
        .collect();
    missing.sort();
    if !missing.is_empty() {
        let installed: Vec<_> = registry.capabilities().map(|c| c.0.as_str()).collect();
        let diagnostics = missing
            .into_iter()
            .map(|capability| {
                Diagnostic::error(
                    DiagnosticCode::MissingCapability,
                    CompileStage::CapabilityValidation,
                    format!(
                        "this pack requires capability `{capability}`, which this composition \
                         does not install"
                    ),
                )
                .in_source("pack.ron")
                .fix(format!(
                    "install `{capability}` in the composition, or remove the content that \
                     needs it"
                ))
                .fix(format!(
                    "installed capabilities: {}",
                    if installed.is_empty() {
                        "<none>".to_string()
                    } else {
                        installed.join(", ")
                    }
                ))
            })
            .collect();
        return Err(CompileFailure::new(
            CompileStage::CapabilityValidation,
            diagnostics,
        ));
    }

    // ── facet validation ─────────────────────────────────────────────────
    let mut diagnostics = Vec::new();
    let mut defined: BTreeMap<ContentId, Entry> = BTreeMap::new();
    let mut duplicates: Vec<Diagnostic> = Vec::new();
    let mut pending_refs: Vec<PendingRef> = Vec::new();
    let mut asset_needs: Vec<AssetRequirement> = Vec::new();
    let mut prepared_sources: Vec<Source> = Vec::new();
    let mut facet_requirements: BTreeSet<CapabilityId> = BTreeSet::new();
    let mut lowered: BTreeMap<SchemaId, std::sync::Arc<dyn std::any::Any + Send + Sync>> =
        BTreeMap::new();

    for source in &draft.sources {
        let registration = registry
            .get(&source.schema)
            .expect("schema resolution proved every source has one");
        let facet = FacetSource {
            declared_path: &source.declared_path,
            text: &source.text,
            schema: &source.schema,
            namespace,
        };
        let mut outcome = FacetOutcome::default();
        registration.handler.check(&facet, &mut outcome);

        diagnostics.extend(outcome.diagnostics);
        if let Some(artifact) = outcome.lowered {
            // Last source of a schema wins. Deliberate and narrow: today every
            // schema has one source per pack, and a schema with two would need
            // to say how its artifacts MERGE — which is the handler's question,
            // not the compiler's. When one does, it grows a merge and this line
            // goes away.
            lowered.insert(source.schema.clone(), artifact);
        }
        pending_refs.extend(outcome.references);
        asset_needs.extend(outcome.assets);
        facet_requirements.extend(outcome.requires);

        let mut source_canonical = String::new();
        for entry in outcome.defines {
            source_canonical.push_str(&entry.id.to_string());
            source_canonical.push('\n');
            source_canonical.push_str(&entry.canonical);
            source_canonical.push('\n');
            match defined.get(&entry.id) {
                // Same identity twice. Reported even when the two definitions
                // are byte-identical: two sources both claiming to own a name
                // means an edit to one of them silently does nothing.
                Some(existing) => duplicates.push(
                    Diagnostic::error(
                        DiagnosticCode::DuplicateIdentity,
                        CompileStage::ConflictDetection,
                        format!(
                            "`{}` is defined twice: in `{}` and in `{}`",
                            entry.id, existing.source, source.declared_path
                        ),
                    )
                    .about(entry.id.clone())
                    .in_source(&source.declared_path)
                    .fix("rename one of them, or delete the duplicate definition"),
                ),
                None => {
                    defined.insert(
                        entry.id.clone(),
                        Entry {
                            id: entry.id,
                            source: source.declared_path.clone(),
                            canonical: entry.canonical,
                        },
                    );
                }
            }
        }

        prepared_sources.push(Source {
            declared_path: source.declared_path.clone(),
            canonical_path: source.canonical_path.display().to_string(),
            schema: source.schema.clone(),
            version: source.version,
            content_fingerprint: ContentFingerprint::of(source_canonical.as_bytes()).0,
        });
    }

    if diagnostics.iter().any(Diagnostic::is_error) {
        return Err(CompileFailure::new(
            CompileStage::FacetValidation,
            diagnostics.into_iter().filter(Diagnostic::is_error).collect(),
        ));
    }

    // A facet may need a capability the manifest never declared — that is a
    // finding about the MANIFEST, and it is checked here rather than at the
    // capability stage because only the handler knows.
    let mut facet_missing: Vec<&CapabilityId> = facet_requirements
        .iter()
        .filter(|capability| !registry.has_capability(capability))
        .collect();
    facet_missing.sort();
    if !facet_missing.is_empty() {
        let diagnostics = facet_missing
            .into_iter()
            .map(|capability| {
                Diagnostic::error(
                    DiagnosticCode::MissingCapability,
                    CompileStage::CapabilityValidation,
                    format!(
                        "authored content needs capability `{capability}`, which this \
                         composition does not install"
                    ),
                )
                .fix(format!(
                    "install `{capability}`, or remove the authored field that needs it"
                ))
                .fix(format!(
                    "declare it in pack.ron's `requires` so this is caught before any facet is \
                     read"
                ))
            })
            .collect();
        return Err(CompileFailure::new(
            CompileStage::CapabilityValidation,
            diagnostics,
        ));
    }
    required.extend(facet_requirements);

    // ── reference resolution ─────────────────────────────────────────────
    let mut resolved_references = Vec::new();
    let mut reference_failures = Vec::new();
    for pending in &pending_refs {
        let target = pending.target_in(namespace);
        if defined.contains_key(&target) {
            resolved_references.push(ResolvedReference {
                declared_by: pending.declared_by.clone(),
                field: pending.field.to_string(),
                target,
            });
        } else {
            let available: Vec<String> = defined
                .keys()
                .filter(|id| id.schema == pending.schema)
                .map(|id| id.name.clone())
                .collect();
            reference_failures.push(pending.unresolved(&available));
        }
    }

    // ── asset resolution ─────────────────────────────────────────────────
    let mut prepared_assets: BTreeMap<String, AssetProvenance> = BTreeMap::new();
    for need in &asset_needs {
        if assets.contains(&need.path) {
            prepared_assets
                .entry(need.path.clone())
                .or_insert_with(|| AssetProvenance {
                    path: need.path.clone(),
                    root: assets.label(),
                    required_by: Vec::new(),
                })
                .required_by
                .push(need.declared_by.clone());
        } else {
            let finding = Diagnostic {
                severity: assets.severity(),
                ..Diagnostic::error(
                    DiagnosticCode::MissingAsset,
                    CompileStage::ReferenceResolution,
                    format!(
                        "`{}` names asset `{}`, which is not present under {}",
                        need.declared_by,
                        need.path,
                        assets.label()
                    ),
                )
            }
            .about(need.declared_by.clone())
            .at_field(need.field)
            .fix(format!("add the file at `{}`", need.path))
            .fix("or point the field at an asset that exists");
            // A warning still travels — on the prepared pack, where a packaging
            // step or a release gate can refuse it later. What it must never do
            // is vanish.
            match finding.severity {
                Severity::Error => reference_failures.push(finding),
                Severity::Warning => diagnostics.push(finding),
            }
        }
    }
    for provenance in prepared_assets.values_mut() {
        provenance.required_by.sort();
        provenance.required_by.dedup();
    }

    if !reference_failures.is_empty() {
        return Err(CompileFailure::new(
            CompileStage::ReferenceResolution,
            reference_failures,
        ));
    }

    // ── conflict detection ───────────────────────────────────────────────
    if !duplicates.is_empty() {
        return Err(CompileFailure::new(
            CompileStage::ConflictDetection,
            duplicates,
        ));
    }
    resolved_references.sort();

    // ── canonical ordering + fingerprint ─────────────────────────────────
    let mut pack = PreparedContentPack {
        id: draft.manifest.id.clone(),
        version: draft.manifest.version.clone(),
        namespace: namespace.clone(),
        sources: prepared_sources,
        collapsed_aliases: draft.collapsed_aliases.clone(),
        required_capabilities: required,
        schemas,
        content: defined,
        assets: prepared_assets,
        resolved_references,
        diagnostics,
        lowered,
        fingerprint: ContentFingerprint(0),
    };
    pack.fingerprint = ContentFingerprint::of(pack.canonical_bytes().as_bytes());
    Ok(pack)
}

/// Read a pack directory and compile it. The whole front door, for the CLI and
/// for a standard test that owns a pack path.
pub fn compile_dir(
    root: impl AsRef<std::path::Path>,
    registry: &SchemaRegistry,
    assets: &dyn AssetSource,
) -> Result<PreparedContentPack, CompileFailure> {
    let draft = ContentPackDraft::read_from_dir(root)?;
    compile(&draft, registry, assets)
}

#[cfg(test)]
mod tests;
