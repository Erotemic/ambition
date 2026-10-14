//! Content activation identity and peer-stable content identity.
//!
//! `ContentEpoch` distinguishes activations within one app, even when
//! the prepared definitions have identical bytes. It is not a content
//! fingerprint, and it cannot establish cross-peer agreement. Gaps are
//! valid: an attempted activation may consume an epoch without publishing.
//!
//! `PeerContentIdentity` carries the prepared content digest used by
//! independent peers. Construction stamps the identity supplied by
//! preparation; it must not derive peer-visible identity from the
//! app-local activation counter.

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

/// Digest identifying prepared content across independent peers.
///
/// The producer supplies all 32 digest bytes. The all-zero default
/// means that no content identity was stated, not that the digest of
/// an admitted definition is zero. Construction must preserve this
/// distinction in peer-visible provenance.
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
pub struct PeerContentIdentity([u8; 32]);

impl PeerContentIdentity {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The value a road with no prepared fingerprint states.
    ///
    /// Spelled, rather than left to `Default`, so that "nobody named the
    /// content" reads as a decision at the call site — see [`Self::is_stated`].
    pub const fn unstated() -> Self {
        Self([0u8; 32])
    }

    /// Whether any content was stated at all.
    ///
    /// ⚠ A fixture's all-zero value is "unstated", not "stated as zero" — see the
    /// `Default` note above. A projection asks this before hashing.
    pub fn is_stated(&self) -> bool {
        self.0 != [0u8; 32]
    }
}

impl fmt::Display for PeerContentIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "content:")?;
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
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
        let mut bytes = [0u8; 32];
        bytes[31] = 7;
        assert_eq!(format!("{}", ContentEpoch(7)), "epoch:7");
        assert_eq!(
            format!("{}", PeerContentIdentity::from_bytes(bytes)),
            "content:0000000000000000000000000000000000000000000000000000000000000007"
        );
        assert_ne!(
            format!("{}", ContentEpoch(7)),
            format!("{}", PeerContentIdentity::from_bytes(bytes))
        );
    }

    /// ⚠ AND THE RENDERING IS FIXED-WIDTH, so two identities cannot be re-split
    /// when they sit next to another field in a tab-joined identity string. A
    /// variable-width decimal would let `content:1` + `\t23` and `content:12` +
    /// `\t3` produce the same bytes if a separator were ever dropped.
    #[test]
    fn the_peer_rendering_is_fixed_width() {
        let mut low = [0u8; 32];
        low[31] = 1;
        assert_eq!(
            format!("{}", PeerContentIdentity::from_bytes(low)).len(),
            format!("{}", PeerContentIdentity::from_bytes([0xff; 32])).len()
        );
    }

    /// ⛔ AN ALL-ZERO DIGEST IS "UNSTATED", NOT A STATED VALUE — the distinction a
    /// fixture depends on, and the one a projection must encode rather than fold.
    #[test]
    fn an_unstated_identity_is_distinguishable_from_a_stated_one() {
        assert!(!PeerContentIdentity::default().is_stated());
        let mut bytes = [0u8; 32];
        bytes[0] = 1;
        assert!(PeerContentIdentity::from_bytes(bytes).is_stated());
    }
}
