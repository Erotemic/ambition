//! Authored visual effects are named rows.
//!
//! [`FxId`] is the FNV-1a hash of that authored name, matching the shape of
//! [`ambition_sfx::SfxId`]. Presentation resolves the id to its sheet, row, and cue.

/// Use the same hash implementation as [`ambition_sfx::SfxId`].
use ambition_sfx::fnv1a_64_str;

/// A stable id for one authored visual effect, hashed from its row name.
///
/// `Copy` and 8 bytes: a `VfxMessage` carrying one allocates nothing, which is
/// what lets simulation emit effects at RL rollout rates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FxId(u64);

impl FxId {
    /// From a `'static` name. Hashed at compile time in `const` context.
    pub const fn from_static(s: &'static str) -> Self {
        Self(fnv1a_64_str(s))
    }

    /// From a runtime name — an authored `Vfx { effect }` id out of RON.
    pub fn new(s: &str) -> Self {
        Self(fnv1a_64_str(s))
    }

    pub const fn from_hash(hash: u64) -> Self {
        Self(hash)
    }

    pub const fn hash(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for FxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match ids::name_of(*self) {
            Some(name) => write!(f, "`{name}`"),
            None => write!(f, "FxId(0x{:016x})", self.0),
        }
    }
}

/// Declare an [`FxId`] constant and record its authored spelling in
/// [`ids::NAMED`].
///
/// Exactly [`ambition_sfx::sfx_ids!`]'s shape, and for exactly its reason: an
/// [`FxId`] is a one-way hash, so a diagnostic holding only the id could print
/// `FxId(0x9f3c…)` and nothing else. One declaration emits the constant AND the
/// `(id, name)` row beside it, so the two cannot drift.
///
///  this is not the vocabulary. The vocabulary is the shipped sheets; this
/// is the subset that Rust call sites name, so they name a constant instead of a
/// string literal. Declaring a row here neither creates nor blesses it — a name
/// with no row on any registered sheet is a counted miss at draw time, which is
/// the open-vocabulary policy SFX has run under since it shipped.
#[macro_export]
macro_rules! fx_ids {
    ($($(#[$note:meta])* $name:ident => $spelling:literal),* $(,)?) => {
        $($(#[$note])* pub const $name: $crate::fx::FxId = $crate::fx::FxId::from_static($spelling);)*

        /// Every declared effect as `(id, authored spelling)`, in declaration order.
        pub const NAMED: &[($crate::fx::FxId, &str)] = &[$(($name, $spelling)),*];

        /// The authored spelling behind a declared id, for diagnostics.
        pub fn name_of(id: $crate::fx::FxId) -> Option<&'static str> {
            NAMED.iter().find(|(known, _)| *known == id).map(|(_, name)| *name)
        }
    };
}

/// The effect ids the engine's own Rust names.
///
///  deliberately short. Content names its effects as authored strings (a move's
/// `Vfx { effect }` comes out of RON), and 189 rows ship; a constant here earns
/// its place only when engine code — not content — has to say the name.
pub mod ids {
    // The five rows of `generic_explosions`. These were `ExplosionKind`'s five
    // variants; a blink, a bomb, a recall and a pickup pop name one directly now.
    fx_ids! {
        /// The default point detonation: bomb, grenade, blink arrival.
        CLASSIC_BURST => "classic_burst",
        /// A rounder, softer pop — a launcher's "up you go".
        BURST_ROUND => "burst_round",
        /// The expanding ring a committed heavy throws.
        SHOCKWAVE => "shockwave",
        /// A dirty grey puff: something arrived fast and heavy.
        SMOKE_BURST => "smoke_burst",
        /// Radiating spikes — the signature-move flash.
        STARBURST => "starburst",
        /// A compression cone from `generic_exotic_fx` — the first effect the
        /// engine names from OUTSIDE the old five, and the reason this module
        /// is a list rather than an enum.
        SONIC_BOOM => "sonic_boom",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The id is the hash of the name, and the name comes back out. Both halves
    /// in one test because a table that cannot answer the second question is
    /// how a miss report degrades to a bare hash.
    #[test]
    fn a_declared_id_hashes_its_own_name_and_is_recoverable() {
        for (id, spelling) in ids::NAMED {
            assert_eq!(*id, FxId::new(spelling), "`{spelling}` hashes to itself");
            assert_eq!(ids::name_of(*id), Some(*spelling));
        }
        assert_eq!(ids::name_of(FxId::new("no_such_effect")), None);
    }
}
