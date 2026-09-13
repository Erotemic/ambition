//! Immutable prepared-content identity shared by preparation, activation,
//! snapshots, and transactional hot reload.
//!
//! Fingerprints are BLAKE3 over an explicitly versioned, length-delimited list
//! of named canonical sections. Section order is normalized before hashing.
//! Debug output, map iteration order, entity ids, handles, timestamps, and
//! mutable session state are never inputs.

use std::fmt;
use std::sync::Arc;

use bevy::prelude::*;

use crate::session_world::PreparedPlatformerSource;

pub const CONTENT_FINGERPRINT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ContentFingerprintSchemaVersion(pub u32);

impl ContentFingerprintSchemaVersion {
    pub const CURRENT: Self = Self(CONTENT_FINGERPRINT_SCHEMA_VERSION);
}

impl fmt::Display for ContentFingerprintSchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "content-schema-v{}", self.0)
    }
}

macro_rules! digest_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, $prefix)?;
                for byte in self.0 {
                    write!(f, "{byte:02x}")?;
                }
                Ok(())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, f)
            }
        }
    };
}

digest_type!(ContentFingerprint, "cfp1:");
digest_type!(SnapshotSchemaFingerprint, "ssp1:");

/// The activation-generation stamp, owned by the neutral foundation because
/// construction planning sits far below this crate and must be able to state
/// which epoch a plan was prepared against. Allocation stays here
/// ([`ContentEpochSequence`]); only the stamp is shared.
pub use ambition_platformer2d_core::ContentEpoch;

/// App-local generation allocator. Allocation happens only after a candidate
/// prepared definition has fully validated and is about to be published or
/// committed. Routing/load transaction ids are deliberately unrelated.
#[derive(Resource, Clone, Debug)]
pub struct ContentEpochSequence {
    next: u64,
}

impl Default for ContentEpochSequence {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl ContentEpochSequence {
    pub fn allocate(&mut self) -> ContentEpoch {
        let epoch = ContentEpoch(self.next);
        self.next = self.next.checked_add(1).expect("content epoch exhausted");
        epoch
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ContentOwner {
    pub provider_id: String,
    pub source_id: String,
    pub domain: String,
}

impl ContentOwner {
    pub fn new(
        provider_id: impl Into<String>,
        source_id: impl Into<String>,
        domain: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            source_id: source_id.into(),
            domain: domain.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentDiagnostic {
    pub section: String,
    pub message: String,
}

impl ContentDiagnostic {
    pub fn new(section: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            section: section.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ContentDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "prepared-content section '{}': {}",
            self.section, self.message
        )
    }
}

impl std::error::Error for ContentDiagnostic {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedContentSection {
    pub name: String,
    pub digest: ContentFingerprint,
    canonical: Arc<[u8]>,
}

impl PreparedContentSection {
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }
}

/// The identity of the authored content PACK this App selected, if it selected one.
///
/// ⭐⭐ **THE COMPOSITION'S ANSWER TO "WHICH AUTHORED CONTENT IS THIS", HANDED TO
/// THE ENGINE AS AN OPAQUE STRING.** Content-pack compilation lives far above this
/// crate and must stay there; what the engine needs is not the pack but its
/// IDENTITY, so that a session prepared under one pack and a session prepared
/// under another are different content generations.
///
/// ⛔⛤ **WITHOUT IT THEY WERE NOT.** Every section of a `PreparedContent` was an
/// App REGISTRY, so the authored pack — move tables, item catalog, encounter
/// waves — reached the game without reaching this fingerprint. Two sessions
/// prepared under different packs shared one `PreparedContentIdentity`, and the
/// rollback timeline contract that exists to refuse *"prepared content changed
/// while the session was active"* compares exactly that identity. The guard
/// could not see the content most likely to change during development.
///
/// ⚠ ABSENT IS A REAL ANSWER, not a missing value: a composition with no content
/// pack (a demo, a fixture) selects none, and its prepared content says so.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct SelectedContentIdentity(pub String);

/// The candidate MECHANICAL INPUTS one preparation transaction must build from.
///
/// ⛔⛤ **A PENDING GENERATION USED TO OVERWRITE `SelectedContentIdentity`, AND
/// THAT MADE IT EVERY TRANSACTION'S ANSWER.** A hot reload has to tell the
/// preparation it is asking for which content to fingerprint, and the only road
/// was the App-wide selection — so while a reload was in flight the App reported
/// `SelectedContentPack = N` and `SelectedContentIdentity = N+1`, and any
/// UNRELATED route preparation running in that window was fingerprinted with a
/// candidate identity it had nothing to do with. The identity is what the
/// rollback timeline contract compares, so a stranger's transaction inherited a
/// generation stamp for content it never prepared.
///
/// ⭐ **THE CLAIM IS THE LOAD ID, BECAUSE THAT IS WHAT BOTH ENDS HAVE.** The
/// router mints `shell.{route}.{counter}` per transaction, and
/// `PlatformerPreparation::prepare` already receives the
/// `ProviderLoadTransaction` at the line that reads the identity. A preparation
/// uses this value iff the claim names ITS load; every other preparation keeps
/// reading the App's active selection.
///
/// ⚠ ONE AT A TIME, DELIBERATELY. The coordinator upstream refuses a second
/// generation while one is pending, so a single claim is the whole truth rather
/// than a first-past-the-post over a map.
/// ⛔⛤ **IT CARRIES THE CANDIDATE CAST TOO, AND THAT IS WHY IT IS NO LONGER
/// CALLED `PendingContentIdentity`.** A review of `bdbddfe` found the freeze
/// landed on the wrong generation: a cast-changing reload computes the N+1 cast
/// at REQUEST time (`take_admitted_revision`) and deliberately withholds it from
/// the App until the commit boundary, so while the transaction is pending the
/// published `PreparedCharacterRegistry` is still N. Preparation froze THAT —
/// giving a session whose identity names N+1 and whose fighters are N's.
///
/// ⇒ **THE CANDIDATE MUST NOT HAVE TO BE PUBLISHED GLOBALLY FOR PREPARATION TO
/// SEE IT** — that is exactly the authority bug `PendingContentIdentity` was
/// carved out to remove, and restoring it under another name would undo this
/// whole file. The transaction owns its candidate values and hands them to its
/// own preparation, keyed by the same `load_id` claim the identity already uses.
///
/// ⚠ **ONLY THE CAST RIDES HERE, AND THAT IS MEASURED RATHER THAN ASSUMED.** The
/// participating families are the moveset, `fighter_brain_ladder` and
/// `encounter_waves`; a candidate that changes `AuthoredSheets` or the
/// `BossCatalog` is REFUSED by `ReloadRequest` rather than published (see
/// `PACK_DERIVED_FAMILIES`). Those two therefore cannot move across the
/// prepare→activate window, so freezing them from the App is not a second
/// generation — it is the same one. A family that gains a reload road gains a
/// field here, and the compiler will not ask for it: the guard is
/// `participates`.
#[derive(Resource, Clone, Debug)]
pub struct PendingGenerationInputs {
    /// The `LoadId` of the transaction these inputs belong to.
    pub load_id: String,
    /// The identity line that transaction must fingerprint against.
    pub identity: String,
    /// The ALREADY-ADMITTED candidate cast this transaction will publish at its
    /// commit boundary, or `None` when the candidate does not change the cast.
    ///
    /// ⚠ `None` IS NOT "USE THE APP'S". It is *"this transaction publishes no
    /// new cast"*, which for a non-participating candidate makes the App's
    /// published cast the transaction's own value — the same one, not a
    /// fallback to a stranger's.
    pub characters: Option<ambition_characters::prepared::PreparedCharacterRegistry>,
}

impl PendingGenerationInputs {
    /// The identity for `load_id`, or `None` when this claim is a stranger's.
    pub fn identity_for(&self, load_id: &str) -> Option<&str> {
        (self.load_id == load_id).then_some(self.identity.as_str())
    }

    /// The candidate cast for `load_id`.
    ///
    /// ⛔ TWO `None`s AND THEY ARE NOT THE SAME, which is why this returns a
    /// nested option: the OUTER `None` means *"this claim is a stranger's, do
    /// not use it at all"*; the inner means *"this transaction is mine and it
    /// changes no cast"*. Flattening them would let a stranger's transaction
    /// silently fall through to the App-global registry, which is the shape of
    /// the defect this type exists to prevent.
    #[allow(clippy::option_option)]
    pub fn characters_for(
        &self,
        load_id: &str,
    ) -> Option<Option<&ambition_characters::prepared::PreparedCharacterRegistry>> {
        (self.load_id == load_id).then_some(self.characters.as_ref())
    }
}

/// Canonical, order-independent input builder. Duplicate section names are a
/// structured assembly error, not last-registration-wins behavior.
#[derive(Default)]
pub struct PreparedContentBuilder {
    sections: std::collections::BTreeMap<String, Vec<u8>>,
    owners: Vec<ContentOwner>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreparedContentBuildError {
    EmptySectionName,
    DuplicateSection { name: String },
}

impl fmt::Display for PreparedContentBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySectionName => write!(f, "prepared-content section name must not be empty"),
            Self::DuplicateSection { name } => {
                write!(f, "prepared-content section '{name}' was contributed twice")
            }
        }
    }
}
impl std::error::Error for PreparedContentBuildError {}

impl PreparedContentBuilder {
    pub fn add_owner(&mut self, owner: ContentOwner) {
        self.owners.push(owner);
    }

    pub fn add_section(
        &mut self,
        name: impl Into<String>,
        canonical: impl Into<Vec<u8>>,
    ) -> Result<(), PreparedContentBuildError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(PreparedContentBuildError::EmptySectionName);
        }
        match self.sections.entry(name.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(canonical.into());
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(_) => {
                Err(PreparedContentBuildError::DuplicateSection { name })
            }
        }
    }

    pub fn finish(
        self,
        epoch: ContentEpoch,
        snapshot_schema: SnapshotSchemaFingerprint,
        source: PreparedPlatformerSource,
    ) -> PreparedContent {
        let mut aggregate = CanonicalDigest::new(b"ambition.prepared-content");
        aggregate.u32(CONTENT_FINGERPRINT_SCHEMA_VERSION);
        let mut sections = Vec::with_capacity(self.sections.len());
        for (name, bytes) in self.sections {
            aggregate.str(&name);
            aggregate.bytes(&bytes);
            let mut section = CanonicalDigest::new(b"ambition.prepared-content.section");
            section.str(&name);
            section.bytes(&bytes);
            sections.push(PreparedContentSection {
                name,
                digest: ContentFingerprint::from_bytes(section.finish()),
                canonical: Arc::from(bytes),
            });
        }
        let fingerprint = ContentFingerprint::from_bytes(aggregate.finish());
        let mut owners = self.owners;
        owners.sort();
        owners.dedup();
        PreparedContent(Arc::new(PreparedContentData {
            source,
            fingerprint_schema: ContentFingerprintSchemaVersion::CURRENT,
            fingerprint,
            snapshot_schema,
            epoch,
            owners,
            sections,
        }))
    }
}

/// Exact immutable prepared definition attached to the canonical session root.
#[derive(Component, Clone)]
pub struct PreparedContent(Arc<PreparedContentData>);

struct PreparedContentData {
    source: PreparedPlatformerSource,
    fingerprint_schema: ContentFingerprintSchemaVersion,
    fingerprint: ContentFingerprint,
    snapshot_schema: SnapshotSchemaFingerprint,
    epoch: ContentEpoch,
    owners: Vec<ContentOwner>,
    sections: Vec<PreparedContentSection>,
}

impl fmt::Debug for PreparedContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedContent")
            .field("identity", &self.identity())
            .field("owners", &self.0.owners)
            .field(
                "sections",
                &self
                    .0
                    .sections
                    .iter()
                    .map(|s| (&s.name, s.digest))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl PreparedContent {
    pub fn source(&self) -> &PreparedPlatformerSource {
        &self.0.source
    }
    pub fn fingerprint_schema(&self) -> ContentFingerprintSchemaVersion {
        self.0.fingerprint_schema
    }
    pub fn fingerprint(&self) -> ContentFingerprint {
        self.0.fingerprint
    }
    pub fn snapshot_schema(&self) -> SnapshotSchemaFingerprint {
        self.0.snapshot_schema
    }
    pub fn epoch(&self) -> ContentEpoch {
        self.0.epoch
    }
    pub fn owners(&self) -> &[ContentOwner] {
        &self.0.owners
    }
    pub fn sections(&self) -> &[PreparedContentSection] {
        &self.0.sections
    }
    /// Rebind an already assembled immutable definition to a newly committed
    /// app-local activation generation without changing its fingerprint.
    pub fn with_epoch(&self, epoch: ContentEpoch) -> Self {
        Self(Arc::new(PreparedContentData {
            source: self.0.source.clone(),
            fingerprint_schema: self.0.fingerprint_schema,
            fingerprint: self.0.fingerprint,
            snapshot_schema: self.0.snapshot_schema,
            epoch,
            owners: self.0.owners.clone(),
            sections: self.0.sections.clone(),
        }))
    }

    pub fn identity(&self) -> PreparedContentIdentity {
        PreparedContentIdentity {
            fingerprint_schema: self.fingerprint_schema(),
            fingerprint: self.fingerprint(),
            snapshot_schema: self.snapshot_schema(),
            epoch: self.epoch(),
        }
    }

    /// Byte-stable inspection surface used by logs, tests, and developer tools.
    pub fn deterministic_dump(&self) -> String {
        let mut out = format!(
            "{}\n{}\n{}\n{}\n",
            self.epoch(),
            self.fingerprint_schema(),
            self.fingerprint(),
            self.snapshot_schema()
        );
        for owner in self.owners() {
            out.push_str(&format!(
                "owner\t{}\t{}\t{}\n",
                owner.domain, owner.provider_id, owner.source_id
            ));
        }
        for section in self.sections() {
            out.push_str(&format!("section\t{}\t{}\n", section.name, section.digest));
        }
        out
    }
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparedContentIdentity {
    pub fingerprint_schema: ContentFingerprintSchemaVersion,
    pub fingerprint: ContentFingerprint,
    pub snapshot_schema: SnapshotSchemaFingerprint,
    pub epoch: ContentEpoch,
}

pub(crate) struct CanonicalDigest(blake3::Hasher);

impl CanonicalDigest {
    pub(crate) fn new(domain: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&(domain.len() as u64).to_le_bytes());
        hasher.update(domain);
        Self(hasher)
    }
    pub(crate) fn u32(&mut self, value: u32) {
        self.0.update(&value.to_le_bytes());
    }
    pub(crate) fn str(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }
    pub(crate) fn bytes(&mut self, value: &[u8]) {
        self.0.update(&(value.len() as u64).to_le_bytes());
        self.0.update(value);
    }
    pub(crate) fn finish(self) -> [u8; 32] {
        *self.0.finalize().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_section_order_is_not_authority() {
        fn fingerprint(order: &[(&str, &[u8])]) -> ContentFingerprint {
            let mut b = PreparedContentBuilder::default();
            for (name, bytes) in order {
                b.add_section(*name, bytes.to_vec()).unwrap();
            }
            let mut h = CanonicalDigest::new(b"test");
            for (name, bytes) in b.sections {
                h.str(&name);
                h.bytes(&bytes);
            }
            ContentFingerprint::from_bytes(h.finish())
        }
        assert_eq!(
            fingerprint(&[("b", b"2"), ("a", b"1")]),
            fingerprint(&[("a", b"1"), ("b", b"2")])
        );
    }

    #[test]
    fn canonical_section_content_is_sensitive() {
        let mut a = CanonicalDigest::new(b"test");
        a.str("room");
        a.bytes(b"geometry-a");
        let mut b = CanonicalDigest::new(b"test");
        b.str("room");
        b.bytes(b"geometry-b");
        assert_ne!(a.finish(), b.finish());
    }
}
