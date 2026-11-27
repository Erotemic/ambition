//! Stable hashing of byte sequences.

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// Incremental FNV-1a for bytes. The caller specifies the byte order and
/// boundaries between values.
#[derive(Clone, Copy, Debug)]
pub struct StateHasher(u64);

impl Default for StateHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHasher {
    pub const fn new() -> Self {
        Self(OFFSET)
    }

    pub const fn write(&mut self, bytes: &[u8]) {
        let mut index = 0;
        while index < bytes.len() {
            self.0 = (self.0 ^ bytes[index] as u64).wrapping_mul(PRIME);
            index += 1;
        }
    }

    pub const fn finish(self) -> u64 {
        self.0
    }
}

pub const fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = StateHasher::new();
    hash.write(bytes);
    hash.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_const_and_incremental_hash_match_known_fnv1a_values() {
        const FOOBAR: u64 = fnv1a(b"foobar");
        assert_eq!(FOOBAR, 0x8594_4171_f739_67e8);
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        let mut incremental = StateHasher::new();
        incremental.write(b"foo");
        incremental.write(b"bar");
        assert_eq!(incremental.finish(), FOOBAR);
    }
}
