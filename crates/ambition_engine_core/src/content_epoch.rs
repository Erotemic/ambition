//! **`ContentEpoch` — the app-local activation generation stamp.**
//!
//! An epoch answers "which committed activation of prepared content does this
//! belong to". It is deliberately *not* a fingerprint: two activations of a
//! byte-identical definition share a fingerprint and differ in epoch, which is
//! what lets an app tell "the same content, prepared again" from "different
//! content".
//!
//! It lives in the neutral foundation for the same reason [`ControlFrame`] and
//! [`ConfirmedFrameBoundary`] do: several layers that must not name each other
//! all need to state it. Preparation ([`ambition_runtime`]'s content identity)
//! ALLOCATES epochs; construction planning, which sits far below that crate,
//! only ever STAMPS the epoch it was planned against so a stale plan can be
//! rejected before it mutates anything.
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
    Ord,
    PartialOrd,
    Hash,
)]
pub struct ContentEpoch(pub u64);

impl fmt::Display for ContentEpoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "epoch:{}", self.0)
    }
}
