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
//! `LdtkVocabulary` lets a game hand its own entity converters to a conversion.
//! That seam has existed, documented and tested, with **no users**; Mary-O is
//! the first, and being its first real user is what turned up that it was a
//! process-global `OnceLock` rather than a parameter.
//!
//! A converter's only output is `RoomEmission`, whose channels are engine-owned
//! types — so a game-specific PAYLOAD still has nowhere typed to land, and the
//! authored block reaches the runtime carrying a `name` and nothing else. The
//! trade-off is written up in
//! `docs/planning/proposal-authored-vocabulary-2026-08-04.md` §4.
//!
//! ⭐ **so the name convention did not go away — it stopped being something a
//! HUMAN types.** The author picks `kind: Power` from a dropdown; this converter
//! encodes it; [`block_look_of`] decodes it. A convention two pieces of Mary-O
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

/// **What one of Mary-O's reactive blocks LOOKS LIKE.**
///
/// ⛔ **this used to decide what the block DID as well, and that was the bug.**
/// Jon, 2026-08-04: *"It should be possible to spawn a block that looks like a
/// brick but really has a powerup."* With one enum answering both questions that
/// was unsayable — `Brick` meant masonry art AND breaks AND holds nothing, all
/// at once. Appearance and contents are separate fields now, and the classic
/// hidden-powerup brick is the case that proves they had to be.
///
/// ⚠ deliberately Mary-O's enum and not an engine one. The spec is explicit that
/// the engine must not interpret Mary-O's progression: LDtk authors WHICH block
/// and WHERE, and this crate decides what that means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaryOBlockLook {
    /// The ?-block. Wears its own texture, and an inert one once spent.
    Question,
    /// The pocket-quasar block, which wears its own texture too.
    Quasar,
    /// Masonry. ⭐ **breakable only when it holds NOTHING** — see
    /// [`MaryOBlockContents::breaks_when_empty`]. A brick with something in it
    /// behaves like a ?-block wearing brick art, which is exactly the classic
    /// behaviour and exactly what Jon asked for.
    Brick,
}

impl MaryOBlockLook {
    /// The word an author picks in the editor.
    pub fn authored(self) -> &'static str {
        match self {
            Self::Question => "Question",
            Self::Quasar => "Quasar",
            Self::Brick => "Brick",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        // Case-insensitive because an author typing into a free-text field is a
        // real possibility until the enum def lands in the project.
        //
        // ⚠ `power` and `power_block` still parse. The field was called `kind`
        // with a `Power` value in every block the shipped level authors, and
        // renaming a concept must not silently invalidate a file Jon has already
        // edited by hand.
        match value.trim().to_ascii_lowercase().as_str() {
            "question" | "power" | "power_block" | "bonus" | "?" => Some(Self::Question),
            "quasar" | "quasar_block" | "star" => Some(Self::Quasar),
            "brick" => Some(Self::Brick),
            _ => None,
        }
    }
}

/// **A thing a block can hold.**
///
/// ⭐ **open on purpose.** Jon: *"In the future we could level towards something
/// else (e.g. bubble flowers or other maryo pickups, so leave that seam open)."*
/// Adding a rung is adding a variant and the one match arm that builds its
/// reward — no other code in this file knows how many there are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaryOPickup {
    /// The star wand: small → tall.
    Wand,
    /// The cinder beacon: tall → fire.
    Lantern,
    /// The pocket quasar. ⚠ **not a rung** — gating invincibility behind two
    /// powerups would mean a small Mary-O could never have it, so any form takes
    /// one (Jon: *"a quasar is not part of the wand → lantern item progression.
    /// Any form of maryo should be able to get the quasar"*).
    Quasar,
}

impl MaryOPickup {
    pub fn authored(self) -> &'static str {
        match self {
            Self::Wand => "Wand",
            Self::Lantern => "Lantern",
            Self::Quasar => "Quasar",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "wand" | "star_wand" => Some(Self::Wand),
            "lantern" | "beacon" | "cinder_beacon" => Some(Self::Lantern),
            "quasar" => Some(Self::Quasar),
            _ => None,
        }
    }
}

/// **What a block holds**, independent of what it looks like.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaryOBlockContents {
    /// Nothing. A `Brick` that holds nothing is the one that BREAKS.
    Empty,
    /// This exact pickup, whatever form she is in. *"E.g. always a wand, always
    /// a lantern"* — and it is how the quasar block is expressed now, rather
    /// than by being its own kind of block.
    Always(MaryOPickup),
    /// The next rung TOWARD this pickup, given the form she is in. *"or a
    /// level-towards lantern powerup"* — a small Mary-O gets the wand, a tall
    /// one gets the lantern. This is what every ?-block in 1-1 does.
    Toward(MaryOPickup),
}

impl MaryOBlockContents {
    /// The word an author picks, round-tripped by [`Self::parse`].
    pub fn authored(self) -> String {
        match self {
            Self::Empty => "Empty".to_string(),
            Self::Always(p) => format!("Always{}", p.authored()),
            Self::Toward(p) => format!("Toward{}", p.authored()),
        }
    }

    fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("empty") || value.is_empty() {
            return Some(Self::Empty);
        }
        // Both `AlwaysWand` and the friendlier `always wand` / `always=wand`.
        let lower = value.to_ascii_lowercase();
        for (prefix, wrap) in [
            ("always", Self::Always as fn(MaryOPickup) -> Self),
            ("toward", Self::Toward as fn(MaryOPickup) -> Self),
        ] {
            if let Some(rest) = lower.strip_prefix(prefix) {
                let rest = rest.trim_start_matches([' ', '=', '_', '-']);
                return MaryOPickup::parse(rest).map(wrap);
            }
        }
        None
    }

    /// Nothing pops out of this block.
    pub fn is_empty(self) -> bool {
        matches!(self, Self::Empty)
    }

    /// ⭐ **breakability is DERIVED, not authored.** A brick with something in
    /// it must not shatter — the item would have nowhere to come from — so the
    /// author never has to keep two fields consistent. This is the rule that
    /// makes "a block that looks like a brick but really has a powerup" work
    /// without a third field to get wrong.
    pub fn breaks_when_empty(self) -> bool {
        self.is_empty()
    }
}

/// One authored reactive block: what it looks like, and what it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaryOBlock {
    pub look: MaryOBlockLook,
    pub contents: MaryOBlockContents,
}

impl MaryOBlock {
    /// What a block of this look holds when the author says nothing.
    ///
    /// ⚠ **these defaults are chosen to leave the SHIPPED LEVEL unchanged.**
    /// Every block in `mary_o.ldtk` today authors a `kind` and no `contents`, so
    /// the field has to be optional and its default has to reproduce exactly
    /// what that block did before the split — a ?-block levels toward the
    /// lantern, a quasar block always yields a quasar, a brick is empty.
    pub fn default_contents(look: MaryOBlockLook) -> MaryOBlockContents {
        match look {
            MaryOBlockLook::Question => MaryOBlockContents::Toward(MaryOPickup::Lantern),
            MaryOBlockLook::Quasar => MaryOBlockContents::Always(MaryOPickup::Quasar),
            MaryOBlockLook::Brick => MaryOBlockContents::Empty,
        }
    }

    pub fn new(look: MaryOBlockLook, contents: MaryOBlockContents) -> Self {
        Self { look, contents }
    }

    /// A block of this look, holding whatever that look holds by default.
    pub fn plain(look: MaryOBlockLook) -> Self {
        Self::new(look, Self::default_contents(look))
    }
}

/// The LDtk entity identifier an author places.
pub const MARY_O_BLOCK: &str = "MaryOBlock";

/// The prefix every converted block name carries.
const ENCODED_PREFIX: &str = "maryo_block:";

/// Encode one authored block into the only channel a `Block` has.
///
/// ```text
/// maryo_block:<look>:<contents>:<iid>
/// ```
///
/// ⚠ **three fields now, and the iid stays LAST** so it keeps being the only
/// part that may contain anything. [`decode`] splits from the front exactly
/// twice for the same reason.
///
/// Public so a FIXTURE course can build the same blocks the converter does —
/// a test course that spelled its own names would be testing a vocabulary
/// nothing else speaks.
pub fn encoded_name(block: MaryOBlock, iid: &str) -> String {
    format!(
        "{ENCODED_PREFIX}{}:{}:{iid}",
        block.look.authored(),
        block.contents.authored()
    )
}

/// A reactive block, built the way [`convert_mary_o_block`] builds one: encoded
/// name, and the durable placement identity that `Block::solid` does not set.
pub fn reactive_block(block: MaryOBlock, iid: &str, min: ae::Vec2, size: ae::Vec2) -> ae::Block {
    let mut solid = ae::Block::solid(encoded_name(block, iid), min, size);
    solid.id = ae::GeoId::placement(ae::PlacementId::new(iid.to_string()), 0);
    solid
}

/// The block this authored NAME describes, or `None` when the name did not come
/// from [`convert_mary_o_block`].
///
/// This is the whole decode side: every Mary-O system that used to ask
/// `power_block_index_for(id) -> Option<usize>` asks this instead, which is a
/// question about the block rather than about its position in a Rust array.
pub fn block_of(name: &str) -> Option<MaryOBlock> {
    let rest = name.strip_prefix(ENCODED_PREFIX)?;
    let (look, rest) = rest.split_once(':')?;
    let look = MaryOBlockLook::parse(look)?;
    // ⚠ **a name with only two fields is the OLD encoding**, and it decodes to
    // this look's default contents rather than to `None`. Nothing shipped writes
    // one — but a `RoomGeometry` restored from a snapshot taken before the split
    // would, and answering `None` there would turn every reactive block in that
    // save into a plain wall.
    let Some((contents, _iid)) = rest.split_once(':') else {
        return Some(MaryOBlock::plain(look));
    };
    Some(MaryOBlock::new(look, MaryOBlockContents::parse(contents)?))
}

/// What this authored name LOOKS like — the common question, since most callers
/// only care whether to draw a ?-block.
pub fn block_look_of(name: &str) -> Option<MaryOBlockLook> {
    block_of(name).map(|block| block.look)
}

/// `MaryOBlock` → a one-tile solid carrying its kind in its name.
///
/// ⚠ **a missing or unknown `kind` is a REFUSAL, not a default.** A block that
/// silently became a plain wall is the worst outcome available: it still stops
/// the player, so the level looks whole and one bonus is quietly gone. The
/// author gets told at load which entity and what the choices are.
pub fn convert_mary_o_block(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, _name, min, size) = ctx.parts();
    let authored =
        ambition_platformer2d::ldtk_map::field_string(entity, "kind").unwrap_or_default();
    let Some(look) = MaryOBlockLook::parse(&authored) else {
        return Err(format!(
            "MaryOBlock `{}` has kind {authored:?}, which is not one of Question, Quasar, Brick",
            entity.iid
        ));
    };
    // ⚠ **`contents` is OPTIONAL and its default depends on the look**, which is
    // what lets the shipped level — every block of which authors a `kind` and no
    // `contents` — keep behaving exactly as it did before the split. An author
    // opts in to a hidden powerup; nobody has to migrate.
    let contents = match ambition_platformer2d::ldtk_map::field_string(entity, "contents") {
        Some(authored) if !authored.trim().is_empty() => {
            let Some(contents) = MaryOBlockContents::parse(&authored) else {
                return Err(format!(
                    "MaryOBlock `{}` has contents {authored:?}. Say `Empty`, or `Always<Pickup>` \
                     or `Toward<Pickup>` where Pickup is Wand, Lantern or Quasar",
                    entity.iid
                ));
            };
            contents
        }
        _ => MaryOBlock::default_contents(look),
    };
    let block = MaryOBlock::new(look, contents);
    // ⚠ **`reactive_block` stamps the durable identity, because `Block::solid`
    // does not.** Its own doc says so — *"fixture constructors default to
    // `GeoSource::Anon`; the IR emission paths assign real sources"* — and this
    // IS an IR emission path. Every authored block shared ONE anonymous id until
    // that existed, so a head-bonk on any reactive block resolved to whichever
    // came first.
    let mut emission = RoomEmission::default();
    emission
        .blocks
        .push(reactive_block(block, &entity.iid, min, size));
    Ok(emission)
}

/// **Mary-O's LDtk vocabulary**: the engine's nouns plus her own.
///
/// ⭐ **this is a VALUE now, and that is the whole change.** It used to be
/// `install()`, writing into a process-global `OnceLock` where the first caller
/// won and every later one was ignored with an error log — so two games, a game
/// and a tool, or two test Apps in one process could not disagree. The reason
/// given for the global was that conversion runs from pure non-system code with
/// no `World` in hand, which is true and argues for a PARAMETER rather than for
/// ambient state: a value passed in is exactly as reachable from a tool as from
/// a system.
///
/// ⚠ **it still belongs to the READER, not to the plugin.** `MaryOBlock` is
/// hers and conversion refuses an identifier it cannot convert, loudly and by
/// design — so every test, tool and probe that loads her level has to say this,
/// or get nine refusals. Handing it over at the load is what makes that
/// impossible to forget, where a build-time install was something you could
/// simply not have done yet.
pub fn vocabulary() -> ambition_platformer2d::ldtk_map::LdtkVocabulary {
    ambition_platformer2d::ldtk_map::LdtkVocabulary::extended_by([(
        MARY_O_BLOCK.to_string(),
        convert_mary_o_block as ambition_platformer2d::ldtk_map::LdtkEntityConverter,
    )])
}

/// **Every LDtk noun Mary-O owns**, and the list [`install`] is checked against.
///
/// ⭐ **this exists so the AUTHORING TOOLS can be told the truth.** The Python
/// validator cannot run Rust, so it reads a declared manifest —
/// `assets/worlds/mary_o.entities.json` — to know that `MaryOBlock` is
/// convertible. A declaration nobody checks is just a wish, so this slice and
/// that manifest are pinned against each other by a test below. Neither is
/// derived from the other, which is the whole point: the first attempt let
/// validation accept anything the project DEFINED, and a `BogusEntity`
/// definition with no converter anywhere passed.
pub const MARY_O_LDTK_ENTITY_IDENTIFIERS: &[&str] = &[MARY_O_BLOCK];

/// The manifest the LDtk tooling reads, compiled in so the pin below cannot go
/// looking for a file that moved.
#[cfg(test)]
const MARY_O_ENTITY_MANIFEST: &str = include_str!("../assets/worlds/mary_o.entities.json");

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `"identifier": "..."` in the tooling manifest.
    ///
    /// ⚠ **a hand-rolled scan, on purpose.** This crate's manifest is
    /// deliberately two dependencies wide — its own comment: *"a downstream game
    /// names `ambition_platformer2d` + `bevy`, and NOTHING ELSE"* — and pulling
    /// in a JSON parser to read one list of strings would spend that claim on a
    /// test. The file's shape is fixed and we own it.
    fn manifest_identifiers(json: &str) -> Vec<String> {
        let mut found = Vec::new();
        for chunk in json.split("\"identifier\"").skip(1) {
            let Some(rest) = chunk.split_once(':') else {
                continue;
            };
            let Some(open) = rest.1.find('"') else {
                continue;
            };
            let tail = &rest.1[open + 1..];
            if let Some(close) = tail.find('"') {
                found.push(tail[..close].to_string());
            }
        }
        found
    }

    /// **What the tools are told Mary-O owns is what Mary-O installs.**
    ///
    /// ⛔ **this is the half that makes the LDtk validator's vocabulary honest.**
    /// The validator cannot run Rust, so it trusts
    /// `assets/worlds/mary_o.entities.json` when it sees a `MaryOBlock`. Its
    /// first version instead trusted the project's own `defs.entities` — and a
    /// GPT 5.6 review reproduced the hole that opens: a `BogusEntity` definition
    /// plus an instance of it validated clean with no converter anywhere,
    /// because `defs` is written by the same generator that writes the
    /// instances. The file was being compared against itself.
    ///
    /// ⭐ **so the manifest is a DECLARATION and this is the audit.** A noun
    /// declared here with no converter behind it fails here; a converter
    /// installed without being declared makes every level using it fail
    /// validation. Neither list is derived from the other, so a lie in either
    /// one is a red test rather than a level that quietly loads wrong.
    #[test]
    fn the_declared_manifest_matches_the_converters_actually_installed() {
        let mut declared = manifest_identifiers(MARY_O_ENTITY_MANIFEST);
        declared.sort();
        let mut installed: Vec<String> = MARY_O_LDTK_ENTITY_IDENTIFIERS
            .iter()
            .map(|id| (*id).to_string())
            .collect();
        installed.sort();
        assert_eq!(
            declared, installed,
            "assets/worlds/mary_o.entities.json declares {declared:?} but \
             MARY_O_LDTK_ENTITY_IDENTIFIERS installs {installed:?} — the LDtk \
             validator believes the manifest, so a difference here is either a \
             noun the tools accept that nothing can convert, or a converter no \
             authored level is allowed to use"
        );
        assert!(
            !declared.is_empty(),
            "the scan found no identifiers at all, which would make this test \
             pass by finding nothing on both sides"
        );
    }

    /// Round-trip: what the converter encodes, the runtime decodes.
    ///
    /// ⚠ this is the ONE thing worth pinning about the encoding — that the two
    /// halves agree — because they are the only two readers and a silent
    /// disagreement makes an authored bonus block into a plain wall.
    #[test]
    fn every_kind_survives_the_name_it_is_encoded_into() {
        for look in [
            MaryOBlockLook::Question,
            MaryOBlockLook::Quasar,
            MaryOBlockLook::Brick,
        ] {
            // ⭐ **every look crossed with every contents**, because the whole
            // point of the split is that the two are independent — and a
            // round-trip that only ever tried a look with its own default would
            // be green over an encoder that dropped the contents field entirely.
            for contents in [
                MaryOBlockContents::Empty,
                MaryOBlockContents::Always(MaryOPickup::Wand),
                MaryOBlockContents::Always(MaryOPickup::Lantern),
                MaryOBlockContents::Always(MaryOPickup::Quasar),
                MaryOBlockContents::Toward(MaryOPickup::Wand),
                MaryOBlockContents::Toward(MaryOPickup::Lantern),
                MaryOBlockContents::Toward(MaryOPickup::Quasar),
            ] {
                let block = MaryOBlock::new(look, contents);
                let name = encoded_name(block, "Solid-1234");
                assert_eq!(
                    block_of(&name),
                    Some(block),
                    "`{name}` must decode back to the block it was encoded from"
                );
            }
        }
    }

    /// A block that is not one of Mary-O's is not one of Mary-O's — the decoder
    /// must not claim the level's ordinary terrain.
    #[test]
    fn an_ordinary_block_name_is_not_a_mary_o_block() {
        for name in ["ldtk solid", "goal_pole", "vault_floor", "maryo_block:", ""] {
            assert_eq!(block_look_of(name), None, "`{name}` is not a Mary-O block");
        }
    }

    /// An author's typo has to be REFUSED at load rather than becoming a wall.
    #[test]
    fn an_unknown_kind_is_not_silently_a_plain_block() {
        assert_eq!(MaryOBlockLook::parse("Powr"), None);
        assert_eq!(MaryOBlockLook::parse(""), None);
        // ...and the spellings an author might reasonably reach for DO work.
        assert_eq!(
            MaryOBlockLook::parse("power"),
            Some(MaryOBlockLook::Question)
        );
        assert_eq!(
            MaryOBlockLook::parse(" Brick "),
            Some(MaryOBlockLook::Brick)
        );
    }
}
