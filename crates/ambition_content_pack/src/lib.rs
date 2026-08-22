//! Content-pack compiler.
//!
//! [`compile`] is the single parse/resolve/validate/canonicalize/fingerprint path
//! used by tests, CI, reload, packaging, and CLI tooling. This crate stays free
//! of Bevy and game-domain types; capability owners register their schemas via
//! [`SchemaRegistry`]. Ordinary content changes therefore do not rebuild Rust.

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
    AggregateOutcome, Aggregation, ContentSchemaHandler, DefinedContent, FacetOutcome, FacetSource,
    LoweredFragment, RuntimeDisposition, SchemaRegistration, SchemaRegistry,
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
    let mut defining_schemas: BTreeSet<SchemaId> = BTreeSet::new();
    //  EVERY fragment, in DECLARED order — not the first, and not the last.
    // Which of them becomes the runtime artifact is the aggregation stage's
    // question, and it cannot be answered while only one source has been read.
    let mut fragments: BTreeMap<SchemaId, Vec<(String, std::sync::Arc<dyn std::any::Any + Send + Sync>)>> =
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
            fragments
                .entry(source.schema.clone())
                .or_default()
                .push((source.declared_path.clone(), artifact));
        }
        pending_refs.extend(outcome.references);
        asset_needs.extend(outcome.assets);
        facet_requirements.extend(outcome.requires);

        let mut source_canonical = String::new();
        if !outcome.defines.is_empty() {
            defining_schemas.insert(source.schema.clone());
        }
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

    //  A `Runtime` schema must PRODUCE a runtime artifact.
    //
    // Otherwise a pack compiles while carrying authored runtime content that
    // has no runtime representation — "validated and then ignored", which is the
    // one thing a content compiler must not certify. `AuthoringOnly` is the way
    // to say a schema deliberately reaches no runtime, and saying it explicitly
    // is the point.
    for schema in schemas.keys() {
        let Some(registration) = registry.get(schema) else {
            continue;
        };
        if registration.disposition == RuntimeDisposition::Runtime
            && !fragments.contains_key(schema)
            // Only meaningful when the handler otherwise succeeded: a facet that
            // failed validation is already refused, and demanding an artifact
            // from it would bury the real diagnostic under a consequence.
            && !diagnostics.iter().any(Diagnostic::is_error)
        {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::MalformedProviderBinding,
                    CompileStage::FacetValidation,
                    format!(
                        "schema `{schema}` is registered `Runtime` but lowered no runtime                          artifact, so its authored content would validate and then be ignored"
                    ),
                )
                .fix("call `FacetOutcome::lower` with the value the runtime will consume")
                .fix(
                    "or register the schema `AuthoringOnly`, if reaching no runtime is what it                      means",
                ),
            );
        }
    }

    //  WHAT A SCHEMA LOWERS MUST ALSO BE DEFINED, or it is invisible to the
    // pack's IDENTITY.
    //
    // `canonical_bytes` is built from `define`d rows, so a schema that lowers a
    // runtime artifact and declares no content contributes NOTHING to the
    // fingerprint: its authored values can change the running game while the
    // pack reports the same identity. That defeats cache invalidation, packaging,
    // session compatibility, and peer-content comparison.
    //
    //  this checks the LINK, not the CONTENT of the canonical form. A handler
    // can still define a row whose canonical string omits the field it lowered,
    // and no compiler check can see that. What this removes is the whole silent
    // CLASS — lowering with no identity at all — so the remaining mistake has to
    // be made one field at a time, in a canonical form somebody wrote on purpose.
    for schema in fragments.keys() {
        if !defining_schemas.contains(schema) && !diagnostics.iter().any(Diagnostic::is_error) {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::MalformedProviderBinding,
                    CompileStage::FacetValidation,
                    format!(
                        "schema `{schema}` lowers a runtime artifact and defines no content, so \
                         its authored values reach the game without reaching the pack's identity"
                    ),
                )
                .fix(
                    "call `FacetOutcome::define` with a canonical form covering what was \
                     lowered — one row per authored thing is the usual shape",
                )
                .fix(
                    "the fingerprint is built from defined rows; a schema absent from them can \
                     change the running game without changing the pack",
                ),
            );
        }
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

    // ── aggregation ──────────────────────────────────────────────────────
    //
    //  NEVER last-wins. Overwriting silently means the content INDEX knows
    // about both sources while the runtime artifact holds only the last one —
    // validation and the running game seeing different content.
    //
    // A schema lowered by several sources must define how its fragments combine,
    // because only it knows whether they union, override, or conflict. A generic
    // merge here would make the compiler guess.
    let mut lowered: BTreeMap<SchemaId, std::sync::Arc<dyn std::any::Any + Send + Sync>> =
        BTreeMap::new();
    let mut aggregation_failures: Vec<Diagnostic> = Vec::new();
    for (schema, parts) in fragments {
        let registration = registry
            .get(&schema)
            .expect("schema resolution proved every source has one");
        let borrowed: Vec<schema::LoweredFragment<'_>> = parts
            .iter()
            .map(|(declared_path, value)| schema::LoweredFragment {
                declared_path,
                value,
            })
            .collect();
        let mut outcome = schema::AggregateOutcome::default();
        match registration.handler.aggregate(&borrowed, &mut outcome) {
            schema::Aggregation::Defined => {
                //  ask THIS handler whether it refused, before its diagnostics
                // join the pile — `aggregation_failures` already holds every
                // earlier schema's, so testing that would let one schema's
                // refusal hide the next one's missing artifact.
                let refused = outcome.failed();
                aggregation_failures.extend(outcome.diagnostics);
                match outcome.lowered {
                    Some(artifact) => {
                        lowered.insert(schema, artifact);
                    }
                    None if refused => {}
                    // Distinct from the `Runtime`-lowered nothing check above — there, no
                    // source lowered at all.
                    None => aggregation_failures.push(
                        Diagnostic::error(
                            DiagnosticCode::MalformedProviderBinding,
                            CompileStage::Aggregation,
                            format!(
                                "schema `{schema}` defines an aggregation that reported no \
                                 problems and published no artifact"
                            ),
                        )
                        .fix("call `AggregateOutcome::lower` with the merged runtime value"),
                    ),
                }
            }
            // No merge rule. One fragment is the artifact; two is the refusal
            // this stage was carved out of.
            schema::Aggregation::Undefined => {
                let mut parts = parts.into_iter();
                let (first_path, first) = parts.next().expect("a schema with no fragments is absent");
                lowered.insert(schema.clone(), first);
                for (declared_path, _) in parts {
                    aggregation_failures.push(
                        Diagnostic::error(
                            DiagnosticCode::ConflictingModuleContribution,
                            CompileStage::Aggregation,
                            format!(
                                "schema `{schema}` is lowered by `{first_path}` and by \
                                 `{declared_path}`, and it has not defined how two of its \
                                 artifacts combine"
                            ),
                        )
                        .in_source(&declared_path)
                        .fix("put this schema's content in ONE source")
                        .fix(
                            "or implement `ContentSchemaHandler::aggregate`: merge the schema's \
                             own fragments, validate the aggregate, lower once",
                        ),
                    );
                }
            }
        }
    }
    if aggregation_failures.iter().any(Diagnostic::is_error) {
        return Err(CompileFailure::new(
            CompileStage::Aggregation,
            aggregation_failures
                .into_iter()
                .filter(Diagnostic::is_error)
                .collect(),
        ));
    }
    diagnostics.extend(aggregation_failures);

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
