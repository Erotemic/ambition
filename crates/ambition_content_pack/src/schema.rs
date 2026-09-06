//! Schema registration — how a capability contributes an authored content
//! family WITHOUT editing one central closed enum.
//!
//! A registration is the complete answer to "may this facet be authored, and
//! what does it mean": identity, version, owning capability, the handler that
//! reads it, and whether the result reaches the runtime at all.
//!
//! An installed schema with no instances in a pack is fine — a capability
//! offering something nobody authored is the ordinary state of a library. An
//! authored facet with no complete handler is not.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::diagnostic::{CompileStage, Diagnostic, DiagnosticCode};
use crate::identity::{CapabilityId, ContentId, ModuleNamespace, SchemaId, SchemaVersion};
use crate::refs::{AssetRequirement, PendingRef};

/// Whether prepared content of this schema reaches the running simulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeDisposition {
    /// Lowered into runtime state by the owning capability.
    Runtime,
    /// Consumed by tooling only (hall generators, docs, editor metadata).
    AuthoringOnly,
}

/// One authored file, handed to its schema's handler.
pub struct FacetSource<'a> {
    /// The path exactly as the manifest declared it — what a reader will look
    /// for in their editor.
    pub declared_path: &'a str,
    pub text: &'a str,
    pub schema: &'a SchemaId,
    pub namespace: &'a ModuleNamespace,
}

impl FacetSource<'_> {
    /// Mint an identity in this pack's namespace under this facet's schema.
    pub fn content_id(&self, name: impl Into<String>) -> ContentId {
        ContentId::new(self.namespace, self.schema, name)
    }

    /// Mint an identity under a DIFFERENT schema than the file's.
    ///
    /// One authored file routinely defines several identity KINDS: a character
    /// catalog defines `character`, `brain_preset` and `action_set_preset`; a
    /// seed library defines `boss_seed`. Those are different questions, and
    /// keeping them apart is what lets a refusal list the presets that exist
    /// rather than the characters.
    ///
    /// Hand-rolled as `ContentId::new(facet.namespace, &SchemaId::new(…), name)`
    /// in three handlers before it lived here.
    pub fn content_id_in(&self, schema: &str, name: impl Into<String>) -> ContentId {
        ContentId::new(self.namespace, &SchemaId::new(schema), name)
    }

    /// A diagnostic already carrying this facet's source path.
    pub fn diagnostic(&self, code: DiagnosticCode, message: impl Into<String>) -> Diagnostic {
        Diagnostic::error(code, CompileStage::FacetValidation, message)
            .in_source(self.declared_path)
    }
}

/// One piece of content a handler declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinedContent {
    pub id: ContentId,
    /// The canonical form this entry contributes to the pack fingerprint.
    ///
    /// It is the HANDLER's job to make this canonical — stable field order,
    /// defaults materialised — because only the handler knows which authored
    /// differences are semantic. A fingerprint over raw file bytes would move
    /// when somebody reflows a comment, which makes it useless for "did the
    /// content actually change".
    pub canonical: String,
}

/// What a handler learned from one facet.
#[derive(Default)]
pub struct FacetOutcome {
    pub defines: Vec<DefinedContent>,
    pub references: Vec<PendingRef>,
    pub assets: Vec<AssetRequirement>,
    /// Capabilities this particular facet needs, beyond the schema's owner.
    /// (A character authoring a combat kit needs combat installed even though
    /// the character schema itself belongs to the character capability.)
    pub requires: Vec<CapabilityId>,
    pub diagnostics: Vec<Diagnostic>,
    /// The LOWERED artifact: the runtime value this facet prepares to.
    ///
    /// This is what makes `RuntimeDisposition::Runtime` mean something. Without
    /// it the compiler proves content correct and the runtime parses the same
    /// bytes a second time — two readers of one file, which is the shape this
    /// whole crate exists to remove.
    ///
    /// `Arc<dyn Any>` because the compiler cannot name a capability's runtime
    /// type (it must not depend on any domain crate) and because a prepared
    /// pack is cloned. The owning capability downcasts it back; nobody else has
    /// any business looking.
    pub lowered: Option<Arc<dyn Any + Send + Sync>>,
}

impl FacetOutcome {
    pub fn define(&mut self, id: ContentId, canonical: impl Into<String>) {
        self.defines.push(DefinedContent {
            id,
            canonical: canonical.into(),
        });
    }

    pub fn refer(&mut self, reference: PendingRef) {
        self.references.push(reference);
    }

    pub fn need_asset(&mut self, asset: AssetRequirement) {
        self.assets.push(asset);
    }

    pub fn require(&mut self, capability: CapabilityId) {
        self.requires.push(capability);
    }

    pub fn report(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Publish the runtime value this facet prepares to.
    pub fn lower<T: Any + Send + Sync>(&mut self, value: T) {
        self.lowered = Some(Arc::new(value));
    }

    pub fn failed(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// One source's contribution to a schema whose artifact spans several files.
pub struct LoweredFragment<'a> {
    /// The path exactly as the manifest declared it — so a merge conflict names
    /// the FILE an author has to open, never an index into a list.
    pub declared_path: &'a str,
    pub(crate) value: &'a Arc<dyn Any + Send + Sync>,
}

impl LoweredFragment<'_> {
    /// This fragment as the type its handler lowered.
    ///
    /// `None` means the handler lowered something else from that file, which is
    /// a bug in the handler rather than in the content — it is the only code
    /// that produces these and the only code that reads them.
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.value.downcast_ref::<T>()
    }
}

/// What a handler made of ALL of one schema's sources.
#[derive(Default)]
pub struct AggregateOutcome {
    pub diagnostics: Vec<Diagnostic>,
    /// The merged artifact — the runtime value, lowered ONCE.
    pub lowered: Option<Arc<dyn Any + Send + Sync>>,
}

impl AggregateOutcome {
    /// Publish the merged runtime value.
    pub fn lower<T: Any + Send + Sync>(&mut self, value: T) {
        self.lowered = Some(Arc::new(value));
    }

    pub fn report(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// A refusal already carrying the aggregation stage — report it once it
    /// carries its source and its fix.
    pub fn refusal(code: DiagnosticCode, message: impl Into<String>) -> Diagnostic {
        Diagnostic::error(code, CompileStage::Aggregation, message)
    }

    pub fn failed(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// Whether a schema defines how source fragments combine. The handler's return
/// value is the declaration, so merge behavior has one source of truth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aggregation {
    /// This schema has no merge rule: one source lowers the artifact and a
    /// second is a refusal. The default, and what a single-file family wants.
    Undefined,
    /// The handler merged the fragments; whatever it lowered is in the outcome.
    Defined,
}

/// The behaviour half of a schema registration.
pub trait ContentSchemaHandler: Send + Sync {
    /// Read one facet: declare what it defines, what it references, what assets
    /// and capabilities it needs, and what is wrong with it.
    ///
    ///  a handler MUST report an authored field it does not consume
    /// ([`DiagnosticCode::UnknownField`]). Serde's `deny_unknown_fields` is the
    /// cheapest way to get this right; rolling your own field walk and
    /// forgetting is how a typo becomes a mechanic that silently never fires.
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome);

    /// Merge every lowered fragment into the runtime artifact. Aggregation is
    /// invoked even for one source so artifact type is independent of source
    /// count. Fragments arrive in manifest declaration order.
    fn aggregate(
        &self,
        fragments: &[LoweredFragment<'_>],
        out: &mut AggregateOutcome,
    ) -> Aggregation {
        let _ = (fragments, out);
        Aggregation::Undefined
    }
}

/// Identity + version + owner + behaviour + disposition. All five, or the facet
/// has no complete handler.
#[derive(Clone)]
pub struct SchemaRegistration {
    pub id: SchemaId,
    pub version: SchemaVersion,
    pub capability: CapabilityId,
    pub disposition: RuntimeDisposition,
    /// One line, shown by `--list-schemas`. Documentation metadata belongs with
    /// the registration or it is not documentation, it is a wiki.
    pub doc: &'static str,
    pub handler: Arc<dyn ContentSchemaHandler>,
}

impl std::fmt::Debug for SchemaRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaRegistration")
            .field("id", &self.id)
            .field("version", &self.version)
            .field("capability", &self.capability)
            .field("disposition", &self.disposition)
            .finish_non_exhaustive()
    }
}

/// What one composition installs: its capabilities, and the schemas they own.
///
/// This is the thing that differs between the shipped app, a demo, a fixture
/// and the CLI — and the reason "uninstalled required capability" is a real
/// error rather than a hypothetical one.
#[derive(Default, Clone)]
pub struct SchemaRegistry {
    schemas: BTreeMap<SchemaId, SchemaRegistration>,
    capabilities: BTreeSet<CapabilityId>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare a capability installed. Registering a schema does this for its
    /// owner implicitly; call it directly for a capability that installs
    /// behaviour without owning authored content.
    pub fn install_capability(&mut self, capability: CapabilityId) -> &mut Self {
        self.capabilities.insert(capability);
        self
    }

    /// Install one schema.
    ///
    /// Two registrations for one id is [`DiagnosticCode::AmbiguousSchemaOwnership`]
    /// and it fails HERE rather than at compile time: an ambiguous registry is
    /// a composition bug, and letting it through means one pack validates
    /// against a handler chosen by map iteration order.
    pub fn register(&mut self, registration: SchemaRegistration) -> Result<(), Diagnostic> {
        if let Some(existing) = self.schemas.get(&registration.id) {
            return Err(Diagnostic::error(
                DiagnosticCode::AmbiguousSchemaOwnership,
                CompileStage::SchemaResolution,
                format!(
                    "schema `{}` is claimed by capability `{}` and capability `{}`",
                    registration.id, existing.capability, registration.capability
                ),
            )
            .fix(format!(
                "install only one of the two capabilities, or give one of them its own schema id \
                 (a schema is owned by exactly one capability)"
            )));
        }
        self.capabilities.insert(registration.capability.clone());
        self.schemas.insert(registration.id.clone(), registration);
        Ok(())
    }

    pub fn get(&self, id: &SchemaId) -> Option<&SchemaRegistration> {
        self.schemas.get(id)
    }

    pub fn has_capability(&self, capability: &CapabilityId) -> bool {
        self.capabilities.contains(capability)
    }

    pub fn capabilities(&self) -> impl Iterator<Item = &CapabilityId> {
        self.capabilities.iter()
    }

    pub fn schemas(&self) -> impl Iterator<Item = &SchemaRegistration> {
        self.schemas.values()
    }

    /// Schema ids in canonical order — used by the "did you mean" half of an
    /// unknown-schema refusal, so a typo is answered rather than only rejected.
    pub fn known_schema_ids(&self) -> Vec<String> {
        self.schemas.keys().map(|id| id.0.clone()).collect()
    }
}
