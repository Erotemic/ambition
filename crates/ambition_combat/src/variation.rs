//! Stable per-actor variation helpers for ECS feature actors.
//!
//! Enemy brain configs should be deterministic for a given authored actor id,
//! but visibly different across siblings spawned in the same room. Keep the
//! seed and jitter helpers here so spawn, mount/dismount, and future ECS actor
//! construction paths share one variation source instead of reviving the old
//! attack-choreography evaluator.

/// Stable, allocation-free 32-bit hash of an actor id.
///
/// FNV-1a (32-bit) is small, deterministic, and distinct enough for the
/// handful of authored actors we need to vary per arena. The helper never
/// returns zero so callers may reserve zero as an "unset" sentinel.
pub fn seed_from_id(id: &str) -> u32 {
    let mut hash: u32 = 0x811C9DC5;
    for byte in id.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    if hash == 0 {
        1
    } else {
        hash
    }
}

/// Derive five deterministic f32s in `[0, 1)` from a u32 seed.
///
/// This gives each per-actor brain a stable but distinct random signature
/// without carrying a PRNG in spawn code. The values are independent enough for
/// jittering cooldowns, initial staggers, standoff radii, orbit phases, and
/// drift rates.
pub fn five_f32s_from_seed(seed: u32) -> (f32, f32, f32, f32, f32) {
    // Mix the seed via xorshift to get a sequence of uncorrelated draws. The
    // shift amounts are from a standard xorshift32 implementation.
    let mut x = seed.wrapping_mul(0x9E3779B1).wrapping_add(0xDEADBEEF);
    let mut take = || {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        (x as f32) / (u32::MAX as f32)
    };
    (take(), take(), take(), take(), take())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_from_id_is_stable_nonzero_and_distinct() {
        let a = seed_from_id("Burning Flying Shark:0");
        let b = seed_from_id("Burning Flying Shark:0");
        assert_eq!(a, b);
        assert_ne!(seed_from_id(""), 0);
        assert_ne!(seed_from_id("a"), seed_from_id("b"));
    }

    #[test]
    fn five_f32s_from_seed_are_stable_and_unit_interval() {
        let values = five_f32s_from_seed(seed_from_id("actor"));
        assert_eq!(values, five_f32s_from_seed(seed_from_id("actor")));
        for value in [values.0, values.1, values.2, values.3, values.4] {
            assert!((0.0..1.0).contains(&value));
        }
    }
}
