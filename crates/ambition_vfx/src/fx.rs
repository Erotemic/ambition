//! **An effect is a NAME.**
//!
//! The shipped art already carries the vocabulary: twelve FX spritesheets whose
//! rows are named (`sonic_boom`, `shield_break`, `reductio_impact`), and one
//! `vfx.<family>.<row>` cue in the packed bank for every one of those rows —
//! 189 ↔ 189, no sheet off by one. So the name addresses the clip AND its
//! paired sound together, in the data, and the engine owes exactly one mapping
//! (name → which sheet holds that row), not a Rust enum per look.
//!
//! [`FxId`] is that name on the wire: an FNV-1a hash of the authored row name,
//! the same shape as [`ambition_sfx::SfxId`], so a message stays `Copy` and
//! allocation-free on the RL-hot path. Resolution back to a sheet + row + cue
//! is a presentation job and lives there ([`ambition_sprite_sheet::fx`] holds
//! the authored index, the render layer keys it by hash).
//!
//! ⛔ **this replaced `ExplosionKind`**, a five-variant enum that was a
//! transliteration of the five rows of `generic_explosions` — reconstructed by
//! three hand-kept tables (`move_vfx_kind` name→enum, `explosion_anim`
//! enum→`CharacterAnim`, `explosion_sfx` enum→cue) plus five aliases inside
//! `CharacterAnim::from_name` that spelled effect rows *Idle/Walk/Run/Hit/Slash*.
//! Every one of those tables existed only to get back to the string the content
//! already had.

/// ⭐ **the SAME hash [`ambition_sfx::SfxId`] uses, borrowed rather than
/// re-typed.** These two id spaces name the two halves of one authored thing —
/// a clip and the cue packed beside it — so a second copy of FNV-1a here would
/// be a place for them to silently disagree, and there is no version of that
/// disagreement anyone wants.
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
/// ⚠ **this is not the vocabulary.** The vocabulary is the shipped sheets; this
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
/// ⚠ deliberately short. Content names its effects as authored strings (a move's
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
