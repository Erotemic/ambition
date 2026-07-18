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
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

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

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ContentEpoch(pub u64);

impl fmt::Display for ContentEpoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "epoch:{}", self.0)
    }
}

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
/// The `Arc` is authoritative shared ownership, not a synchronized mirror: no
/// mutation API exists and candidates remain detached until commit.
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
