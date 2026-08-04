//! **Mary-O's own LDtk nouns**, so a level is authored by filling in FIELDS
//! rather than by typing a name convention exactly right.
//!
//! Jon, 2026-08-04: *"make sure it is easy for authors to write blocks that
//! contain items and to place warp pipes and moving platforms."* Before this, a
//! ?-block was a `Solid` entity that had to be named `power_block_2` — the index
//! mattered, the spelling mattered, and what the block CONTAINED could not be
//! said at all. GPT 5.6's Mary-O spec asks for the same thing in §4.
//!
//! ## The seam, and why no engine change was needed
//!
//! `install_ldtk_entity_converters` lets a game register converters for its own
//! entity identifiers. It has existed, documented and tested, with **no users**.
//! Mary-O is the first.
//!
//! A converter's only output is `RoomEmission`, whose channels are engine-owned
//! types — so a game-specific PAYLOAD still has nowhere typed to land, and the
//! authored block reaches the runtime carrying a `name` and nothing else. The
//! trade-off is written up in
//! `docs/planning/proposal-authored-vocabulary-2026-08-04.md` §4.
//!
//! ⭐ **so the name convention did not go away — it stopped being something a
//! HUMAN types.** The author picks `kind: Power` from a dropdown; this converter
//! encodes it; [`block_kind_of`] decodes it. A convention two pieces of Mary-O
//! share is an implementation detail. A convention an author has to spell
//! correctly is a trap.
//!
//! ## The encoded name
//!
//! ```text
//! maryo_block:<kind>:<iid>
//! ```
//!
//! ⚠ **the iid, not an ordinal.** An index cannot come from here — a converter
//! sees ONE entity and has no idea how many others exist — and that is the good
//! kind of pressure: an ordinal was never a durable identity anyway, since
//! inserting a block renumbers every one after it. The LDtk iid survives moves,
//! edits and reordering, which is exactly what a runtime that remembers *"this
//! block is spent"* needs.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::ldtk_map::{LdtkEntityCtx, RoomEmission};

/// What one of Mary-O's reactive blocks DOES when struck from below.
///
/// ⚠ deliberately Mary-O's enum and not an engine one. The spec is explicit that
/// the engine must not interpret Mary-O's progression: LDtk authors WHICH block
/// and WHERE, and this crate decides what that means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaryOBlockKind {
    /// The ?-block: pops the next rung of the wand → lantern ladder.
    Power,
    /// The pocket quasar — any form can take one and be briefly untouchable.
    /// Not a rung: gating invincibility behind two powerups would mean a small
    /// Mary-O could never have it.
    Quasar,
    /// Breakable masonry: a bonk from a grown body removes it.
    Brick,
}

impl MaryOBlockKind {
    /// The word an author picks in the editor.
    pub fn authored(self) -> &'static str {
        match self {
            Self::Power => "Power",
            Self::Quasar => "Quasar",
            Self::Brick => "Brick",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        // Case-insensitive because an author typing into a free-text field is a
        // real possibility until the enum def lands in the project.
        match value.trim().to_ascii_lowercase().as_str() {
            "power" | "power_block" | "bonus" => Some(Self::Power),
            "quasar" | "quasar_block" | "star" => Some(Self::Quasar),
            "brick" => Some(Self::Brick),
            _ => None,
        }
    }
}

/// The LDtk entity identifier an author places.
pub const MARY_O_BLOCK: &str = "MaryOBlock";

/// The prefix every converted block name carries.
const ENCODED_PREFIX: &str = "maryo_block:";

/// Encode one authored block's identity into the only channel a `Block` has.
fn encoded_name(kind: MaryOBlockKind, iid: &str) -> String {
    format!("{ENCODED_PREFIX}{}:{iid}", kind.authored())
}

/// The kind of block this authored NAME describes, or `None` when the name did
/// not come from [`convert_mary_o_block`].
///
/// This is the whole decode side: every Mary-O system that used to ask
/// `power_block_index_for(id) -> Option<usize>` asks this instead, which is a
/// question about the block rather than about its position in a Rust array.
pub fn block_kind_of(name: &str) -> Option<MaryOBlockKind> {
    if let Some(rest) = name.strip_prefix(ENCODED_PREFIX) {
        let (kind, _iid) = rest.split_once(':')?;
        return MaryOBlockKind::parse(kind);
    }
    // ⚠ **the BOOTSTRAP names, still recognised on purpose.** The file the
    // migration generated authors `power_block_0` / `quasar_block_1` /
    // `brick_2`, because it predates this entity type. Reading both lets the
    // runtime move to kinds BEFORE the file is regenerated, which is what keeps
    // each step of the migration green instead of requiring one flip-day where
    // the level and the code change together.
    //
    // ▢ delete this arm once the shipped file authors `MaryOBlock` throughout —
    // it is scaffolding with an expiry, not a compatibility promise.
    for (prefix, kind) in [
        (crate::POWER_BLOCK_PREFIX, MaryOBlockKind::Power),
        (crate::QUASAR_BLOCK_PREFIX, MaryOBlockKind::Quasar),
        (crate::BRICK_PREFIX, MaryOBlockKind::Brick),
    ] {
        if name.starts_with(prefix) {
            return Some(kind);
        }
    }
    None
}

/// `MaryOBlock` → a one-tile solid carrying its kind in its name.
///
/// ⚠ **a missing or unknown `kind` is a REFUSAL, not a default.** A block that
/// silently became a plain wall is the worst outcome available: it still stops
/// the player, so the level looks whole and one bonus is quietly gone. The
/// author gets told at load which entity and what the choices are.
pub fn convert_mary_o_block(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, _name, min, size) = ctx.parts();
    let authored = ambition_platformer2d::ldtk_map::field_string(entity, "kind").unwrap_or_default();
    let Some(kind) = MaryOBlockKind::parse(&authored) else {
        return Err(format!(
            "MaryOBlock `{}` has kind {authored:?}, which is not one of Power, Quasar, Brick",
            entity.iid
        ));
    };
    let mut emission = RoomEmission::default();
    emission.blocks.push(ae::Block::solid(
        encoded_name(kind, &entity.iid),
        min,
        size,
    ));
    Ok(emission)
}

/// Install Mary-O's LDtk vocabulary. Called once at plugin-build time, before
/// any world load.
///
/// ⚠ **the registry behind this is process-global** (`OnceLock`, first install
/// wins, a DIFFERENT second set logs loudly). That is a known interim limitation
/// with its endpoint already written down — queue G3, and the GPT review's
/// finding 7 — and Mary-O being its first real user is what stops it being
/// hypothetical. It is not fixed here.
pub fn install() {
    ambition_platformer2d::ldtk_map::install_ldtk_entity_converters([(
        MARY_O_BLOCK.to_string(),
        convert_mary_o_block as ambition_platformer2d::ldtk_map::LdtkEntityConverter,
    )]);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip: what the converter encodes, the runtime decodes.
    ///
    /// ⚠ this is the ONE thing worth pinning about the encoding — that the two
    /// halves agree — because they are the only two readers and a silent
    /// disagreement makes an authored bonus block into a plain wall.
    #[test]
    fn every_kind_survives_the_name_it_is_encoded_into() {
        for kind in [
            MaryOBlockKind::Power,
            MaryOBlockKind::Quasar,
            MaryOBlockKind::Brick,
        ] {
            let name = encoded_name(kind, "Solid-1234");
            assert_eq!(
                block_kind_of(&name),
                Some(kind),
                "`{name}` must decode back to the kind it was encoded from"
            );
        }
    }

    /// A block that is not one of Mary-O's is not one of Mary-O's — the decoder
    /// must not claim the level's ordinary terrain.
    /// ⚠ the bootstrap names the generated file still uses decode too, so the
    /// runtime can move to kinds before the level is regenerated.
    #[test]
    fn the_bootstrap_names_decode_until_the_file_is_regenerated() {
        assert_eq!(block_kind_of("power_block_0"), Some(MaryOBlockKind::Power));
        assert_eq!(block_kind_of("quasar_block_2"), Some(MaryOBlockKind::Quasar));
        assert_eq!(block_kind_of("brick_1"), Some(MaryOBlockKind::Brick));
    }

    #[test]
    fn an_ordinary_block_name_is_not_a_mary_o_block() {
        for name in ["ldtk solid", "goal_pole", "vault_floor", "maryo_block:", ""] {
            assert_eq!(block_kind_of(name), None, "`{name}` is not a Mary-O block");
        }
    }

    /// An author's typo has to be REFUSED at load rather than becoming a wall.
    #[test]
    fn an_unknown_kind_is_not_silently_a_plain_block() {
        assert_eq!(MaryOBlockKind::parse("Powr"), None);
        assert_eq!(MaryOBlockKind::parse(""), None);
        // ...and the spellings an author might reasonably reach for DO work.
        assert_eq!(MaryOBlockKind::parse("power"), Some(MaryOBlockKind::Power));
        assert_eq!(MaryOBlockKind::parse(" Brick "), Some(MaryOBlockKind::Brick));
    }
}
