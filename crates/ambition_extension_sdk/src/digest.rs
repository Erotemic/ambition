//! A small deterministic 64-bit digest (FNV-1a).
//!
//! The digest is for identity and checksums of canonical bytes. It is the same
//! on every platform and in every process: it has no random seed. It is not a
//! cryptographic hash and it is not a signature.

/// FNV-1a, 64 bit.
#[derive(Clone, Copy, Debug)]
pub struct Digest(u64);

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

impl Default for Digest {
    fn default() -> Self {
        Self(OFFSET)
    }
}

impl Digest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(PRIME);
        }
        self
    }

    /// Writes the length first, so `"ab" + "c"` and `"a" + "bc"` differ.
    pub fn str(&mut self, s: &str) -> &mut Self {
        self.u64(s.len() as u64);
        self.bytes(s.as_bytes())
    }

    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.bytes(&[v])
    }

    pub fn u16(&mut self, v: u16) -> &mut Self {
        self.bytes(&v.to_le_bytes())
    }

    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.bytes(&v.to_le_bytes())
    }

    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.bytes(&v.to_le_bytes())
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

/// The digest of one byte string.
pub fn digest_bytes(bytes: &[u8]) -> u64 {
    Digest::new().bytes(bytes).finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_digest_is_the_published_fnv1a_value() {
        // Published FNV-1a 64 test vectors.
        assert_eq!(digest_bytes(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(digest_bytes(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(digest_bytes(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn a_string_boundary_changes_the_digest() {
        let ab_c = Digest::new().str("ab").str("c").finish();
        let a_bc = Digest::new().str("a").str("bc").finish();
        assert_ne!(ab_c, a_bc);
    }
}
