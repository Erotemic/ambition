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
