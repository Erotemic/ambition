//! `ContentPackDraft` — the authored side, read but not yet meaningful.
//!
//! A draft is inert on purpose (the same posture `ModuleDraft` takes one layer
//! up): reading it installs nothing and proves nothing. Only [`crate::compile`]
//! turns one into a [`crate::PreparedContentPack`], and only a prepared pack is
//! allowed to reach a runtime.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::diagnostic::{CompileFailure, CompileStage, Diagnostic, DiagnosticCode};
use crate::identity::{CapabilityId, ModuleNamespace, PackId, PackVersion, SchemaId, SchemaVersion};

/// `pack.ron` — the pack's identity and its EXPLICIT source manifest.
///
/// Explicit rather than a directory glob, deliberately: a glob makes the pack's
/// contents depend on filesystem enumeration order and on whatever a tool
/// happened to leave in the tree, which is the opposite of a fingerprint that
/// means something. Adding content is one line here, and that line is also
/// where the file's schema is declared — so a content file never has to carry a
/// header, and its existing shape survives migration unchanged.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ContentPackManifest {
    pub id: PackId,
    pub version: PackVersion,
    pub namespace: ModuleNamespace,
    /// Capabilities this pack's content cannot work without. The schemas' own
    /// owners are added automatically; this is for the ones a schema needs but
    /// does not own.
    #[serde(default)]
    pub requires: Vec<CapabilityId>,
    pub sources: Vec<SourceDeclaration>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct SourceDeclaration {
    /// Relative to the pack root.
    pub path: String,
    pub schema: SchemaId,
    pub version: SchemaVersion,
}

/// One authored file, read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    pub declared_path: String,
    /// Symlinks and `..` resolved. Two declarations reaching the same file are
    /// the same source, whatever they were spelled as.
    pub canonical_path: PathBuf,
    pub schema: SchemaId,
    pub version: SchemaVersion,
    pub text: String,
}

/// The authored pack, read off disk and deduplicated. Nothing here is valid yet.
#[derive(Clone, Debug)]
pub struct ContentPackDraft {
    pub root: PathBuf,
    pub manifest: ContentPackManifest,
    pub sources: Vec<SourceFile>,
    /// Aliases collapsed during reading, kept so a prepared pack can say what
    /// it merged rather than quietly reporting fewer sources than the manifest
    /// declares.
    pub collapsed_aliases: Vec<(String, String)>,
}

impl ContentPackDraft {
    /// Read `pack.ron` and every source it declares.
    ///
    /// ## Aliases and symlinks
    ///
    /// Two declarations resolving to one file is not an error by itself — a
    /// pack may legitimately reach the same catalog through a symlinked shared
    /// directory, and this repo already ships sprite trees that do exactly
    /// that. The rule:
    ///
    /// * same canonical file, same schema → deterministic dedup, first
    ///   declaration wins (it is first in a hand-written manifest, so it is
    ///   stable), and the collapse is RECORDED;
    /// * same canonical file, different schema → hard error, because
    ///   deduplication would have to pick a meaning for the file and either
    ///   choice is a guess.
    pub fn read_from_dir(root: impl AsRef<Path>) -> Result<Self, CompileFailure> {
        let root = root.as_ref().to_path_buf();
        let manifest_path = root.join("pack.ron");
        let manifest_text = std::fs::read_to_string(&manifest_path).map_err(|error| {
            CompileFailure::new(
                CompileStage::Parse,
                vec![
                    Diagnostic::error(
                        DiagnosticCode::MalformedSource,
                        CompileStage::Parse,
                        format!("cannot read {}: {error}", manifest_path.display()),
                    )
                    .fix("a content pack is a directory containing `pack.ron`"),
                ],
            )
        })?;
        let manifest: ContentPackManifest = ron::from_str(&manifest_text).map_err(|error| {
            CompileFailure::new(
                CompileStage::Parse,
                vec![
                    Diagnostic::error(
                        DiagnosticCode::MalformedSource,
                        CompileStage::Parse,
                        format!("{} is not a valid pack manifest: {error}", "pack.ron"),
                    )
                    .in_source("pack.ron"),
                ],
            )
        })?;
        Self::read_manifest(root, manifest)
    }

    /// Build a draft from in-memory sources such as embedded content, editor
    /// buffers, or test literals. These sources enter the same compiler as
    /// filesystem content. No filesystem canonicalization or alias collapsing
    /// occurs on this path.
    pub fn from_sources(
        manifest: ContentPackManifest,
        sources: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, CompileFailure> {
        let texts: BTreeMap<String, String> = sources.into_iter().collect();
        let mut diagnostics = Vec::new();
        let mut read = Vec::new();
        for declaration in &manifest.sources {
            match texts.get(&declaration.path) {
                Some(text) => read.push(SourceFile {
                    declared_path: declaration.path.clone(),
                    canonical_path: PathBuf::from(&declaration.path),
                    schema: declaration.schema.clone(),
                    version: declaration.version,
                    text: text.clone(),
                }),
                None => diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::MalformedSource,
                        CompileStage::Parse,
                        format!(
                            "the manifest declares `{}` but no source was supplied for it",
                            declaration.path
                        ),
                    )
                    .in_source(&declaration.path)
                    .fix("embed it beside the others, or remove its line from the manifest"),
                ),
            }
        }
        if !diagnostics.is_empty() {
            return Err(CompileFailure::new(CompileStage::Parse, diagnostics));
        }
        Ok(Self {
            root: PathBuf::new(),
            manifest,
            sources: read,
            collapsed_aliases: Vec::new(),
        })
    }

    /// Parse an embedded manifest and build a draft from the supplied source
    /// texts, reporting malformed manifests through normal compiler diagnostics.
    pub fn from_manifest_ron(
        manifest_ron: &str,
        sources: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, CompileFailure> {
        let manifest: ContentPackManifest = ron::from_str(manifest_ron).map_err(|error| {
            CompileFailure::new(
                CompileStage::Parse,
                vec![Diagnostic::error(
                    DiagnosticCode::MalformedSource,
                    CompileStage::Parse,
                    format!("the pack manifest does not parse: {error}"),
                )
                .in_source("pack.ron")
                .fix(
                    "a manifest is `(id: …, version: …, namespace: …, requires: [], sources: [])`",
                )],
            )
        })?;
        Self::from_sources(manifest, sources)
    }

    /// The same reading, from a manifest already in hand. Used by tests and by
    /// a future editor that holds an unsaved manifest.
    pub fn read_manifest(
        root: PathBuf,
        manifest: ContentPackManifest,
    ) -> Result<Self, CompileFailure> {
        let mut diagnostics = Vec::new();
        let mut sources: Vec<SourceFile> = Vec::new();
        let mut collapsed_aliases = Vec::new();
        let mut by_canonical: BTreeMap<PathBuf, usize> = BTreeMap::new();

        for declaration in &manifest.sources {
            let path = root.join(&declaration.path);
            let canonical = match std::fs::canonicalize(&path) {
                Ok(canonical) => canonical,
                Err(error) => {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::MalformedSource,
                            CompileStage::Parse,
                            format!("declared source is not readable: {error}"),
                        )
                        .in_source(&declaration.path)
                        .fix(format!(
                            "create {}, or remove its line from pack.ron",
                            path.display()
                        )),
                    );
                    continue;
                }
            };

            if let Some(&first) = by_canonical.get(&canonical) {
                let first_source = &sources[first];
                if first_source.schema == declaration.schema {
                    collapsed_aliases
                        .push((declaration.path.clone(), first_source.declared_path.clone()));
                } else {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::ConflictingSourceAlias,
                            CompileStage::Parse,
                            format!(
                                "`{}` and `{}` are the same file ({}) but declare different \
                                 schemas (`{}` and `{}`)",
                                declaration.path,
                                first_source.declared_path,
                                canonical.display(),
                                declaration.schema,
                                first_source.schema,
                            ),
                        )
                        .in_source(&declaration.path)
                        .fix(
                            "one file has one meaning: drop one declaration, or split the file so \
                             each schema has its own",
                        ),
                    );
                }
                continue;
            }

            let text = match std::fs::read_to_string(&canonical) {
                Ok(text) => text,
                Err(error) => {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::MalformedSource,
                            CompileStage::Parse,
                            format!("cannot read declared source: {error}"),
                        )
                        .in_source(&declaration.path),
                    );
                    continue;
                }
            };

            by_canonical.insert(canonical.clone(), sources.len());
            sources.push(SourceFile {
                declared_path: declaration.path.clone(),
                canonical_path: canonical,
                schema: declaration.schema.clone(),
                version: declaration.version,
                text,
            });
        }

        if diagnostics.iter().any(Diagnostic::is_error) {
            return Err(CompileFailure::new(CompileStage::Parse, diagnostics));
        }

        Ok(Self {
            root,
            manifest,
            sources,
            collapsed_aliases,
        })
    }
}
