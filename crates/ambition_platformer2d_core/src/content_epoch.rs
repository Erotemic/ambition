//! `ContentEpoch` — the app-local activation generation stamp.
//!
//! An epoch answers "which committed activation of prepared content does this
//! belong to". It is deliberately *not* a fingerprint: two activations of a
//! byte-identical definition share a fingerprint and differ in epoch, which is
//! what lets an app tell "the same content, prepared again" from "different
//! content".
//!
//! It lives in the neutral foundation for the same reason [`ControlFrame`] and
//! [`ConfirmedFrameBoundary`] do: several layers that must not name each other
//! all need to state it. Preparation ([`ambition_platformer2d_runtime`]'s content identity)
//! ALLOCATES epochs; construction planning, which sits far below that crate,
//! only ever STAMPS the epoch it was planned against so a stale plan can be
//! rejected before it mutates anything.
//!
//! ⛔⛤ **AN EPOCH IS A GAP-TOLERANT LINEAGE ID, NOT A COUNT OF ACTIVATIONS —
//! RULED 2026-09-13, AND THE TYPE ENFORCES IT.** `ContentEpochSequence`'s doc
//! used to say allocation happens *"only after a candidate prepared definition
//! has fully validated and is about to be published or committed"*. That stopped
//! being true when the hot-reload road moved its allocation ABOVE a preflight
//! that can still fail — deliberately, because the epoch has to be decided before
//! the room is planned or every rebuilt root stamps a `TransactionId` naming the
//! generation being REPLACED. ⇒ A refused reload now burns a number.
//!
//! ⭐⭐ **SO THE CHOICE IS MADE RATHER THAN LEFT AMBIGUOUS: gaps are legal, and
//! `Ord`/`PartialOrd` ARE NOT DERIVED.** The alternative was reservation
//! semantics — allocate, and hand the number back when the preflight refuses —
//! which is a second lifetime to get right in exchange for a property nothing
//! reads. MEASURED before removing the derives: the workspace and every test
//! target compile without them, so no code compared two epochs. ⇒ *"Epoch 7 is
//! newer than epoch 5"* is now a COMPILE ERROR, not a convention. `Eq` and `Hash`
//! remain, because *"is this the generation I was planned against"* is the only
//! question an epoch answers.
//!
//! ⛔⛔ **"TWO PEERS NEVER COMPARE SEQUENCES" WAS FALSE, AND IT WAS THE
//! JUSTIFICATION FOR BURNING NUMBERS.** This said: *"The fact that lets this be
//! cheap: an epoch is not rollback-registered. Two peers never compare
//! sequences, so a gap on one host is invisible."* The first clause is true and
//! the second does not follow from it. **Traced 2026-09-15, every link read:**
//!
//! ```text
//! ContentEpoch(7)  --Display-->  "epoch:7"
//!   ContentBinding::canonical_summary   (construction/mod.rs:689)
//!   ConstructionScope::transaction      (construction/mod.rs:716)
//!     TransactionId("epoch:7\t<room>\t<session>")
//!       component.construction_transaction_id, COMPONENT-CANONICAL
//!       (rollback_schema_baseline.txt:122)
//! ```
//!
//! ⇒ The epoch is not registered; its VALUE is compared anyway, character for
//! character, inside a canonical checksum. So a burned number is NOT invisible
//! to a peer: an App that refused one reload carries epoch 8 where a fresh App
//! carries 7, and the two stamp DIFFERENT canonical provenance on a
//! mechanically identical world. `a_transaction_identity_still_depends_on_host_local_lineage_counters`
//! (`shared_tangle::construction::tests`) is the arm that records exactly this,
//! and it has been green — as a recorded divergence — the whole time.
//!
//! ⚠ **THE RULING ABOVE MAY STILL BE RIGHT; ONLY ITS REASON WAS WRONG.** Gaps
//! being legal also rests on reservation semantics being a second lifetime to
//! get right, which is an argument this does not touch. What changed is the
//! COST: a gap is a peer-visible divergence, not a free local convenience, so
//! the trade is "cheaper allocation for a recorded desync" rather than
//! "cheaper allocation for nothing". ⇒ The fix is not to stop burning numbers.
//! It is to stop the epoch reaching canonical identity at all — ID-PEER step 3,
//! which needs a peer-stable content identity that `shared_tangle` can name.
//! ⛔ `ContentFingerprint` is that value and it is NOT reachable: it lives in
//! `ambition_content_pack`, which `shared_tangle` does not depend on. Deciding
//! where it belongs is a vocabulary-placement call, and this file — which
//! already owns the LOCAL half of the pair and explains the distinction two
//! paragraphs up — is the obvious candidate.
//!
//! [`ControlFrame`]: crate::ControlFrame
//! [`ConfirmedFrameBoundary`]: crate::ConfirmedFrameBoundary

use std::fmt;

/// One committed activation generation of prepared content.
///
/// `0` is deliberately not allocated by the runtime's sequence (it starts at
/// `1`), so it reads as "no epoch stated" for callers that construct plans
/// outside a prepared session — a headless fixture, a unit test.
#[derive(
    bevy_ecs::component::Component,
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    Hash,
)]
pub struct ContentEpoch(pub u64);

impl fmt::Display for ContentEpoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "epoch:{}", self.0)
    }
}

/// ⭐⭐ **THE PEER HALF OF THE PAIR: WHICH CONTENT, NOT WHICH ACTIVATION OF IT.**
///
/// [`ContentEpoch`] answers *"which committed activation of prepared content does
/// this belong to"* and is an app-local lineage id — two Apps that reloaded
/// different numbers of times hold different epochs for byte-identical content.
/// This answers *"which content"*, and two Apps holding the same prepared
/// definition agree on it no matter what either one did before.
///
/// ⛔⛤ **IT EXISTS BECAUSE A PROJECTION HAD NOTHING PEER-STABLE LEFT TO HASH.**
/// `TransactionId` is `{epoch}\t{room}\t{session}` and is registered
/// `component-canonical`, so the whole string is compared between peers. Two of
/// its three terms are per-App counts. Giving it a projection that excluded them
/// would leave only `{room}` — and projecting to that would hand every entity in
/// one room the SAME identity, which is a worse defect than the one being fixed.
/// A projection is only as good as the peer-stable term it has to keep.
///
/// ⭐ **WHY IT LIVES HERE, decided 2026-09-15 from this module's own stated
/// principle rather than by convenience.** The epoch is in the neutral
/// foundation because *"several layers that must not name each other all need to
/// state it"*, with preparation ALLOCATING and construction planning only
/// STAMPING. The peer term has exactly that shape: construction planning, far
/// below, must STAMP which content a plan was built against;
/// `ambition_platformer2d_runtime`'s content identity RENDERS the value. ⇒ Same
/// split, same reason, so the same home — and the local and peer halves of one
/// question sit adjacent instead of in two crates that cannot see each other.
///
/// ⚠ **IT IS NOT `ambition_content_pack::prepared::ContentFingerprint`, AND
/// COULD NOT BE.** Measured: `ambition_content_pack` declares NO ambition
/// dependencies at all — it is a leaf — so it cannot construct a type from this
/// crate, and this crate naming it would invert the graph. The fingerprint is
/// the SOURCE of this value; runtime's content identity is the one layer holding
/// both and is where the rendering belongs. Two names for one fact would be a
/// duplicate authority; one name, rendered once, at the only layer that can.
///
/// ⚠ **`Default` IS ZERO AND MEANS "NO CONTENT STATED"**, matching
/// [`ContentEpoch`]'s convention — a headless fixture or a unit test that builds
/// plans outside a prepared session. ⛔ It does NOT mean "content whose
/// fingerprint is zero", and a peer projection must therefore encode the
/// distinction; `PeerDigest::opt_u64` is how, which is why callers hold an
/// `Option` rather than leaning on the zero.
#[derive(
    bevy_ecs::component::Component,
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    Hash,
)]
pub struct PeerContentIdentity(pub u64);

impl fmt::Display for PeerContentIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "content:{:016x}", self.0)
    }
}

#[cfg(test)]
mod peer_content_identity_tests {
    use super::{ContentEpoch, PeerContentIdentity};

    /// ⛔ THE TWO HALVES MUST NOT RENDER ALIKE, because both end up inside
    /// canonical identity STRINGS and a reader — or a parser — that confused them
    /// would compare an app-local lineage id against a content identity.
    #[test]
    fn the_local_and_peer_halves_render_distinguishably() {
        assert_eq!(format!("{}", ContentEpoch(7)), "epoch:7");
        assert_eq!(
            format!("{}", PeerContentIdentity(7)),
            "content:0000000000000007"
        );
        assert_ne!(
            format!("{}", ContentEpoch(7)),
            format!("{}", PeerContentIdentity(7))
        );
    }

    /// ⚠ AND THE RENDERING IS FIXED-WIDTH, so two identities cannot be re-split
    /// when they sit next to another field in a tab-joined identity string. A
    /// variable-width decimal would let `content:1` + `\t23` and `content:12` +
    /// `\t3` produce the same bytes if a separator were ever dropped.
    #[test]
    fn the_peer_rendering_is_fixed_width() {
        assert_eq!(format!("{}", PeerContentIdentity(1)).len(), format!("{}", PeerContentIdentity(u64::MAX)).len());
    }
}
