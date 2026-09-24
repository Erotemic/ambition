//! A prepared-but-unpublished content generation — fast-iteration I3's
//! complete-candidate transaction.
//!
//! A reload is a whole pack arriving, not a move, item or audio edit. So
//! "did anything mechanical change" has one answer: the pack's own
//! [`ContentFingerprint`], computed over every content id, schema, capability,
//! asset and resolved reference. Every family's view derives from it.
//!
//! Do not infer the whole from a part. A move-only comparison gets this wrong:
//!
//! ```text
//! generation N   moves = A   items = X
//! candidate      moves = A   items = Y
//! → "Unchanged", and the whole candidate pack becomes selected.
//! ```
//!
//! Preparing a candidate does not change the active game. Nothing here
//! allocates an epoch, installs a registry, or moves a selection. A candidate
//! is a value; publication is a separate act by the owner of the live generation.
//!
//! This crate does not establish a `ContentEpoch`, a `PreparedContentIdentity`,
//! or a rollback timeline boundary; those live in `ambition_platformer2d_runtime`,
//! which this crate must not depend on. A publisher that must advance an epoch
//! does it where [`CandidatePublication::Publish`] is returned.

use std::sync::Arc;

use crate::{ContentFingerprint, PreparedContentPack};

/// A complete content generation, prepared and not published.
///
/// The base is part of the value, not of the publish call. Without it a
/// candidate cannot be stale, and the publisher would merge silently.
#[derive(Clone, Debug)]
pub struct CandidateGeneration {
    pack: Arc<PreparedContentPack>,
    base: Option<ContentFingerprint>,
}

impl CandidateGeneration {
    /// A candidate prepared against the generation identified by `base`.
    ///
    /// `None` means "no claim", not "against nothing". A caller that read the live
    /// identity and then yielded (for example, for file I/O) must pass what it
    /// read. This distinction is the whole of [`CandidateVerdict::Stale`].
    pub fn prepared_against(
        pack: Arc<PreparedContentPack>,
        base: Option<ContentFingerprint>,
    ) -> Self {
        Self { pack, base }
    }

    pub fn pack(&self) -> &PreparedContentPack {
        &self.pack
    }

    /// The whole pack, for a publisher that must install it.
    pub fn into_pack(self) -> Arc<PreparedContentPack> {
        self.pack
    }

    /// This candidate's complete mechanical identity.
    pub fn fingerprint(&self) -> ContentFingerprint {
        self.pack.fingerprint
    }

    /// What it was prepared against, if it made a claim.
    pub fn base(&self) -> Option<ContentFingerprint> {
        self.base
    }

    /// What a publisher must do with this candidate, given what is live now.
    ///
    /// The arm order is the contract. Staleness is checked first: a candidate
    /// prepared against a generation that is no longer live is refused, even if
    /// it is identical to what is live. The caller must re-read.
    ///
    /// `active` is `None` when the host has selected nothing yet. That is a first
    /// publication, not a no-op.
    pub fn verdict(&self, active: Option<ContentFingerprint>) -> CandidateVerdict {
        if let (Some(base), Some(active)) = (self.base, active) {
            if base != active {
                return CandidateVerdict::Stale {
                    prepared_against: base,
                    active,
                };
            }
        }
        if active == Some(self.fingerprint()) {
            return CandidateVerdict::Unchanged {
                fingerprint: self.fingerprint(),
            };
        }
        CandidateVerdict::Publish {
            fingerprint: self.fingerprint(),
        }
    }
}

/// Which mechanical domains differ between two packs.
///
/// The domain is the file schema ([`PreparedSource::schema`]), not the identity
/// kind it mints. A source declares `item_catalog` and mints ids under `item`,
/// so a diff over [`PreparedContentPack::content`] would need a reverse map.
///
/// This folds [`PreparedSource::content_fingerprint`], which the compiler
/// already computes per source from its canonical text (a reflowed comment
/// does not move it; a changed value does).
///
/// Sound per domain, not per field. The compiler refuses a schema that lowers
/// a runtime artifact and defines no content, but a handler can still define a
/// row whose canonical string omits a lowered field.
///
/// A domain present in only one pack counts as changed.
pub fn changed_domains(
    base: &PreparedContentPack,
    candidate: &PreparedContentPack,
) -> std::collections::BTreeSet<crate::SchemaId> {
    fn by_domain(
        pack: &PreparedContentPack,
    ) -> std::collections::BTreeMap<crate::SchemaId, Vec<u64>> {
        let mut out: std::collections::BTreeMap<crate::SchemaId, Vec<u64>> = Default::default();
        for source in &pack.sources {
            out.entry(source.schema.clone())
                .or_default()
                .push(source.content_fingerprint);
        }
        // Sort, so manifest order within a family does not count as a change.
        for digests in out.values_mut() {
            digests.sort_unstable();
        }
        out
    }
    let (base, candidate) = (by_domain(base), by_domain(candidate));
    base.keys()
        .chain(candidate.keys())
        .filter(|schema| base.get(*schema) != candidate.get(*schema))
        .cloned()
        .collect()
}

/// The three answers a complete candidate can have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateVerdict {
    /// The candidate was prepared against a generation that is no longer live.
    /// Nothing may be published; the caller re-reads and prepares again.
    Stale {
        prepared_against: ContentFingerprint,
        active: ContentFingerprint,
    },
    /// The candidate is mechanically identical to what is live, as a whole pack.
    ///
    /// This is a complete no-op: no epoch, catalog generation, rollback timeline
    /// or reconstruction. Only the complete identity may claim it; one family's
    /// unchanged section says nothing about the pack.
    Unchanged { fingerprint: ContentFingerprint },
    /// The candidate differs and its base is current: admit it, and publish or
    /// refuse as one act.
    Publish { fingerprint: ContentFingerprint },
}

#[cfg(test)]
#[path = "candidate_tests.rs"]
mod candidate_tests;
