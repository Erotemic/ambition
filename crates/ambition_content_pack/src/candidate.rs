//! A prepared-but-unpublished content generation — fast-iteration I3's
//! complete-candidate transaction.
//!
//! ⭐⭐ **ONE CANDIDATE, ONE COMPLETE MECHANICAL IDENTITY.** A reload is not a
//! move edit, an item edit, or an audio edit; it is a whole pack arriving. The
//! question *"did anything mechanical change"* therefore has exactly one honest
//! answer — the pack's own [`ContentFingerprint`], computed over every content
//! id, schema, capability, asset and resolved reference — and every family's
//! view of it must be derived from that one answer rather than asserted
//! alongside it.
//!
//! ⛔⛤ **THIS EXISTS BECAUSE THE FIRST VERSION INFERRED THE WHOLE FROM A PART,
//! AND I WROTE IT.** `reload_move_tables_selecting` concluded `Unchanged` when
//! the MOVE material was identical and then installed the entire newly-loaded
//! pack as the App's selection:
//!
//! ```text
//! generation N   moves = A   items = X
//! candidate      moves = A   items = Y
//! → "Unchanged", and the whole candidate pack becomes selected.
//! ```
//!
//! One subsystem believes nothing changed while another can observe new
//! mechanical content. ⇒ **The fix is not to special-case `Unchanged`** — that
//! hides the missing abstraction. It is to make the complete identity the thing
//! the decision is made on, which is what this module is.
//!
//! ⚠ **PREPARING A CANDIDATE MUST NOT MUTATE THE ACTIVE GAME.** Nothing here
//! allocates an epoch, installs a registry, or moves a selection. A candidate is
//! a VALUE; publication is a separate act by whoever owns the live generation.
//!
//! ⛔ **WHAT THIS IS NOT, YET.** It does not establish a `ContentEpoch`, a
//! `PreparedContentIdentity`, or a rollback timeline boundary — those live in
//! `ambition_platformer2d_runtime`, which this crate must not depend on. Binding
//! them is I3's remaining half, and the shape here is chosen so that binding is
//! an addition rather than a rewrite: a publisher that must also advance an
//! epoch does it at the one place [`CandidatePublication::Publish`] is returned.

use std::sync::Arc;

use crate::{ContentFingerprint, PreparedContentPack};

/// A complete content generation, prepared and not published.
///
/// ⭐ THE BASE IS PART OF THE VALUE, not of the call that publishes it. A
/// candidate that does not remember what it was prepared against cannot be
/// stale, and "apply this to whatever happens to exist now" is hidden merge
/// semantics in whoever publishes it.
#[derive(Clone, Debug)]
pub struct CandidateGeneration {
    pack: Arc<PreparedContentPack>,
    base: Option<ContentFingerprint>,
}

impl CandidateGeneration {
    /// A candidate prepared against the generation identified by `base`.
    ///
    /// ⚠ `None` MEANS "NO CLAIM", not "against nothing". A caller that compiled
    /// and published without yielding has nothing to be stale against; a caller
    /// that read the live identity, did file I/O, and came back must pass what
    /// it read. The distinction is the whole of [`CandidateVerdict::Stale`].
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

    /// This candidate's COMPLETE mechanical identity.
    pub fn fingerprint(&self) -> ContentFingerprint {
        self.pack.fingerprint
    }

    /// What it was prepared against, if it made a claim.
    pub fn base(&self) -> Option<ContentFingerprint> {
        self.base
    }

    /// What a publisher must do with this candidate, given what is live now.
    ///
    /// ⛔⛔ **THE ORDER OF THESE THREE ARMS IS THE CONTRACT.** Staleness is asked
    /// FIRST, because a candidate prepared against a generation that is no longer
    /// live is refused whatever it contains — including when it happens to be
    /// mechanically identical to what is live. Reporting that as a no-op would be
    /// correct about the bytes and wrong about the transaction: the caller's base
    /// disappeared, and it must re-read rather than be told nothing happened.
    ///
    /// ⚠ `active` is `None` for a host that has selected nothing yet, which is a
    /// first publication and not a no-op.
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

/// The three answers a complete candidate can have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateVerdict {
    /// The candidate was prepared against a generation that is no longer live.
    /// **Nothing may be published**; the caller re-reads and prepares again.
    Stale {
        prepared_against: ContentFingerprint,
        active: ContentFingerprint,
    },
    /// The candidate is mechanically identical to what is live, WHOLE PACK.
    ///
    /// ⭐⭐ **A COMPLETE NO-OP, AND THAT IS THE POINT OF COMPUTING IT HERE.** It
    /// must consume no epoch, no catalog generation, no rollback timeline and no
    /// reconstruction — and it may be claimed only from the COMPLETE identity. A
    /// family that finds its own section unchanged has learned nothing about the
    /// pack, which is the defect this module was written to remove.
    Unchanged { fingerprint: ContentFingerprint },
    /// The candidate differs and its base is current: admit it, and publish or
    /// refuse as one act.
    Publish { fingerprint: ContentFingerprint },
}

#[cfg(test)]
#[path = "candidate_tests.rs"]
mod candidate_tests;
