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
//! ⚠ The fact that lets this be cheap: an epoch is **not rollback-registered**.
//! Two peers never compare sequences, so a gap on one host is invisible.
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
