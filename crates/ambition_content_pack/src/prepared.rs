//! `PreparedContentPack` — the value the pipeline produces.
//!
//! A real value, not a directory-loading convenience: everything a consumer,
//! a packager, an inspector or a mod loader needs to know about this pack is
//! answerable from here without touching the filesystem again.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::fmt::Write as _;

use crate::diagnostic::Diagnostic;
use crate::identity::{
    CapabilityId, ContentId, ModuleNamespace, PackId, PackVersion, SchemaId, SchemaVersion,
};
use crate::refs::{AssetProvenance, ContentKind, ResolvedContentRef};

/// One source, as it contributed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedSource {
    pub declared_path: String,
    pub canonical_path: String,
    pub schema: SchemaId,
    pub version: SchemaVersion,
    /// Fingerprint of the canonical content this source produced — NOT of its
    /// bytes. Reflowing a comment must not move it; changing a value must.
    pub content_fingerprint: u64,
}

/// One prepared entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedContent {
    pub id: ContentId,
    /// Which declared source it came from.
    pub source: String,
    /// The handler's canonical form. This is what the fingerprint hashes and
    /// what a diff between two pack versions compares.
    pub canonical: String,
}

/// A reference the compiler proved, keyed by who declared it and where.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResolvedReference {
    pub declared_by: ContentId,
    pub field: String,
    pub target: ContentId,
}

/// A stable 64-bit content fingerprint.
///
/// Determinism is the entire point — this repo already treats a moving hash as a desync.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentFingerprint(pub u64);

impl ContentFingerprint {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x1000_0000_01b3;

    pub fn of(bytes: &[u8]) -> Self {
        let mut hash = Self::OFFSET;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(Self::PRIME);
        }
        Self(hash)
    }

    pub fn hex(self) -> String {
        format!("{:016x}", self.0)
    }
}

impl std::fmt::Display for ContentFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.hex())
    }
}

/// The compiled pack.
#[derive(Clone, Debug)]
pub struct PreparedContentPack {
    pub id: PackId,
    pub version: PackVersion,
    pub namespace: ModuleNamespace,
    /// Every source that contributed, in manifest order.
    pub sources: Vec<PreparedSource>,
    /// Declarations collapsed because they named the same file.
    pub collapsed_aliases: Vec<(String, String)>,
    /// Capabilities this pack cannot run without — declared plus every schema
    /// owner plus every per-facet requirement, unioned.
    pub required_capabilities: BTreeSet<CapabilityId>,
    /// Schema versions this pack was compiled against. A runtime that installs
    /// a different version must recompile, not adapt.
    pub schemas: BTreeMap<SchemaId, SchemaVersion>,
    /// Canonical content ordering. `BTreeMap<ContentId, _>` by construction.
    pub content: BTreeMap<ContentId, PreparedContent>,
    pub assets: BTreeMap<String, AssetProvenance>,
    pub resolved_references: Vec<ResolvedReference>,
    /// Findings that did not refuse the pack (warnings).
    pub diagnostics: Vec<Diagnostic>,
    /// The LOWERED artifacts, keyed by the schema that produced them.
    ///
    /// The runtime reads these instead of re-parsing the authored bytes. Two
    /// readers of one file is the shape this crate exists to remove, and
    /// leaving the runtime on its own parser would have kept it — the compiler
    /// would prove the content correct and the game would load it separately,
    /// with nothing guaranteeing they read it the same way.
    pub lowered: BTreeMap<SchemaId, Arc<dyn Any + Send + Sync>>,
    pub fingerprint: ContentFingerprint,
}

impl PreparedContentPack {
    /// The runtime value a schema lowered to.
    ///
    /// `None` when the schema contributed nothing to this pack, or when the
    /// caller asked for the wrong type — which is a programming error in the
    /// OWNING capability, since nobody else should be asking.
    pub fn lowered<T: Any + Send + Sync>(&self, schema: &SchemaId) -> Option<&T> {
        self.lowered.get(schema)?.downcast_ref::<T>()
    }

    /// Look up an identity this pack defines.
    pub fn get(&self, schema: &SchemaId, name: &str) -> Option<&PreparedContent> {
        self.content
            .get(&ContentId::new(&self.namespace, schema, name))
    }

    /// Resolve a typed reference against this pack. The only way to obtain a
    /// [`ResolvedContentRef`], which is what makes holding one a proof.
    pub fn resolve<T: ContentKind>(&self, name: &str) -> Option<ResolvedContentRef<T>> {
        let id = ContentId::new(&self.namespace, &SchemaId::new(T::SCHEMA), name);
        self.content
            .contains_key(&id)
            .then(|| ResolvedContentRef::prove(id))
    }

    /// Every identity of one schema, in canonical order.
    pub fn ids_of(&self, schema: &SchemaId) -> Vec<&ContentId> {
        self.content
            .keys()
            .filter(|id| &id.schema == schema)
            .collect()
    }

    /// How many identities of each kind this pack defines, in canonical order.
    pub fn identity_kinds(&self) -> BTreeMap<SchemaId, usize> {
        let mut counts = BTreeMap::new();
        for id in self.content.keys() {
            *counts.entry(id.schema.clone()).or_insert(0) += 1;
        }
        counts
    }

    pub fn content_count(&self) -> usize {
        self.content.len()
    }

    /// The canonical bytes the fingerprint is taken over. Public because a
    /// packaging step wants to write them, and because a fingerprint whose
    /// input cannot be inspected is a number nobody can debug.
    pub fn canonical_bytes(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "pack {} {}", self.id, self.version);
        let _ = writeln!(out, "namespace {}", self.namespace);
        for (schema, version) in &self.schemas {
            let _ = writeln!(out, "schema {schema} {version}");
        }
        for capability in &self.required_capabilities {
            let _ = writeln!(out, "requires {capability}");
        }
        for (id, entry) in &self.content {
            let _ = writeln!(out, "content {id}\n{}", entry.canonical);
        }
        // Asset identity uses the authored path, not the machine-specific
        // resolved root. Resolution provenance remains available separately.
        for path in self.assets.keys() {
            let _ = writeln!(out, "asset {path}");
        }
        for reference in &self.resolved_references {
            let _ = writeln!(
                out,
                "ref {} .{} -> {}",
                reference.declared_by, reference.field, reference.target
            );
        }
        out
    }

    /// A one-screen summary for a CLI.
    pub fn summary(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{} {} (namespace `{}`)  fingerprint {}",
            self.id, self.version, self.namespace, self.fingerprint
        );
        let _ = writeln!(
            out,
            "  {} source(s), {} content identities, {} asset(s), {} resolved reference(s)",
            self.sources.len(),
            self.content.len(),
            self.assets.len(),
            self.resolved_references.len()
        );
        // The FILE schemas the pack was compiled against...
        for (schema, version) in &self.schemas {
            let _ = writeln!(out, "  source schema {schema} {version}");
        }
        // ...and the IDENTITY kinds those files minted, which are a different
        // question: one `character_catalog` file defines characters AND the
        // presets they share, and an author counting "how many characters" is
        // asking about the second.
        for (kind, count) in self.identity_kinds() {
            let _ = writeln!(out, "  {count} × {kind}");
        }
        if !self.required_capabilities.is_empty() {
            let names: Vec<_> = self
                .required_capabilities
                .iter()
                .map(|c| c.0.as_str())
                .collect();
            let _ = writeln!(out, "  requires: {}", names.join(", "));
        }
        for (alias, canonical) in &self.collapsed_aliases {
            let _ = writeln!(
                out,
                "  note: `{alias}` is the same file as `{canonical}` — collapsed"
            );
        }
        for diagnostic in &self.diagnostics {
            let _ = writeln!(out, "  warning: {}", diagnostic.render());
        }
        out
    }
}
