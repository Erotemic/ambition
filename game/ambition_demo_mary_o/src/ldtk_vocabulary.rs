//! Mary-O-specific LDtk vocabulary.
//!
//! `LdtkVocabulary` supplies game-owned entity converters. Converters emit
//! engine-owned `RoomEmission`, so Mary-O encodes typed block kind into the
//! emitted name while authors choose `kind` as a field.
//!
//! Encoded form: `maryo_block:<kind>:<iid>`. The LDtk IID provides durable
//! identity across moves and reordering; ordinals do not.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::ldtk_map::{LdtkEntityCtx, RoomEmission};

/// What one of Mary-O's reactive blocks LOOKS LIKE.
///
/// deliberately Mary-O's enum and not an engine one. The spec is explicit that
/// the engine must not interpret Mary-O's progression: LDtk authors WHICH block
/// and WHERE, and this crate decides what that means.
///
/// The two blocks that authored it now say `kind: Question, contents: AlwaysQuasar` and look
/// identical because they always did. A level still saying `Quasar` is REFUSED at load with
/// that replacement named, rather than quietly becoming a wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaryOBlockLook {
    /// The ?-block. Wears its own texture, and an inert one once spent.
    Question,
    /// Masonry. breakable only when it holds NOTHING — see
    /// [`MaryOBlockContents::breaks_when_empty`].
    Brick,
    /// Nothing at all, until it is struck. The cammo half needed no new variant — `Brick` +
    /// non-empty contents already says it — so this is only the invisible half.
    ///
    /// NOT a floor, and only a rising HEAD collides. `reactive_block` emits
    /// `BlockKind::BonkOnly` for this look: she cannot stand on it or bump it from the side.
    ///
    /// ▢ the classic block becomes a visible solid once struck, and that is
    /// NOT implemented. It stays pass-through forever today; the transition
    /// needs runtime geometry mutation, which is rollback-relevant. Documented
    /// as missing rather than as finished.
    Hidden,
}

/// What an author may put between a prefix and its argument. THE ONE SET.
///
/// ⛔⛔ THIS SET WAS WRITTEN TWICE INSIDE ONE `parse`, once for `coins…` and once
/// for the `always…` / `toward…` wrappers. It is a rule about the AUTHORING
/// SURFACE, not an implementation detail: it decides whether `Coins=3`,
/// `Coins_3`, `Coins 3` and `Coins-3` all mean the same thing.
/// ⇒ Two copies means an author can be told YES by one spelling and NO by
/// another. Adding `:` to one site would make `Coins:3` parse while
/// `Toward:Mushroom` did not, and nothing would report it -- an unparsed value is
/// not an error here, it is a `None` that reads as "not that variant".
///
/// ⚠ Deliberately permissive, which is the reason to NAME it: the set exists so a
/// level author who types the separator they expect is not silently wrong.
fn strip_separator(rest: &str) -> &str {
    rest.trim_start_matches([' ', '=', '_', '-'])
}

impl MaryOBlockLook {
    /// The word an author picks in the editor.
    pub fn authored(self) -> &'static str {
        match self {
            Self::Question => "Question",
            Self::Brick => "Brick",
            Self::Hidden => "Hidden",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        // Case-insensitive because an author typing into a free-text field is a
        // real possibility until the enum def lands in the project.
        //
        // `power` and `power_block` still parse.
        match value.trim().to_ascii_lowercase().as_str() {
            "question" | "power" | "power_block" | "bonus" | "?" => Some(Self::Question),
            "brick" => Some(Self::Brick),
            "hidden" | "invisible" | "cammo" => Some(Self::Hidden),
            _ => None,
        }
    }
}

/// A thing a block can hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaryOPickup {
    /// The star wand: small → tall.
    Wand,
    /// The cinder beacon: tall → fire.
    Lantern,
    /// Any form of maryo should be able to get the quasar"*).
    Quasar,
    Coin,
}

impl MaryOPickup {
    pub fn authored(self) -> &'static str {
        match self {
            Self::Wand => "Wand",
            Self::Lantern => "Lantern",
            Self::Quasar => "Quasar",
            Self::Coin => "Coin",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "wand" | "star_wand" => Some(Self::Wand),
            "lantern" | "beacon" | "cinder_beacon" => Some(Self::Lantern),
            "quasar" => Some(Self::Quasar),
            "coin" | "currency" => Some(Self::Coin),
            _ => None,
        }
    }
}

/// What a block holds, independent of what it looks like.
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
    /// `Coins(1)` is deliberately NOT the same authored word as
    /// `Always(Coin)`, and they behave identically. The count is the whole
    /// content of this variant, so one is the honest default rather than a
    /// special case — an author writes `Coins` and gets the classic single-coin
    /// block, or `Coins5` and gets the multi.
    Coins(u8),
}

impl MaryOBlockContents {
    /// The word an author picks, round-tripped by [`Self::parse`].
    pub fn authored(self) -> String {
        match self {
            Self::Empty => "Empty".to_string(),
            Self::Always(p) => format!("Always{}", p.authored()),
            Self::Toward(p) => format!("Toward{}", p.authored()),
            Self::Coins(n) => format!("Coins{n}"),
        }
    }

    fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("empty") || value.is_empty() {
            return Some(Self::Empty);
        }
        let lower = value.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("coins") {
            let rest = strip_separator(rest);
            if rest.is_empty() {
                return Some(Self::Coins(1));
            }
            return rest.parse::<u8>().ok().filter(|n| *n > 0).map(Self::Coins);
        }
        // Both `AlwaysWand` and the friendlier `always wand` / `always=wand`.
        for (prefix, wrap) in [
            ("always", Self::Always as fn(MaryOPickup) -> Self),
            ("toward", Self::Toward as fn(MaryOPickup) -> Self),
        ] {
            if let Some(rest) = lower.strip_prefix(prefix) {
                let rest = strip_separator(rest);
                return MaryOPickup::parse(rest).map(wrap);
            }
        }
        None
    }

    /// Nothing pops out of this block.
    pub fn is_empty(self) -> bool {
        matches!(self, Self::Empty)
    }

    /// breakability is DERIVED, not authored. A brick with something in
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
    /// these defaults are chosen to leave the SHIPPED LEVEL unchanged.
    /// Every block in `mary_o.ldtk` today authors a `kind` and no `contents`, so
    /// the field has to be optional and its default has to reproduce exactly
    /// what that block did before the split — a ?-block levels toward the
    /// lantern, a quasar block always yields a quasar, a brick is empty.
    pub fn default_contents(look: MaryOBlockLook) -> MaryOBlockContents {
        match look {
            MaryOBlockLook::Question => MaryOBlockContents::Toward(MaryOPickup::Lantern),
            MaryOBlockLook::Brick => MaryOBlockContents::Empty,
            // A hidden block that held nothing would be indistinguishable from
            // empty air, so its default is the classic one: a coin.
            MaryOBlockLook::Hidden => MaryOBlockContents::Always(MaryOPickup::Coin),
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
/// three fields now, and the iid stays LAST so it keeps being the only
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
    let mut lowered = ae::Block::solid(encoded_name(block, iid), min, size);
    lowered.id = ae::GeoId::placement(ae::PlacementId::new(iid.to_string()), 0);
    // not "no collision": the bonk is a `ContactKind::Head` contact the
    // collision system produces, so a block with nothing to hit cannot be struck
    // and the reward disappears with the floor. `BlockKind::BonkOnly` is the
    // mirror of `OneWay` — solid ONLY against a head coming up into it — which
    // keeps the strike and removes the ledge.
    if block.look == MaryOBlockLook::Hidden {
        lowered.kind = ae::BlockKind::BonkOnly;
    }
    lowered
}

/// The block this authored NAME describes, or `None` when the name did not come
/// from [`convert_mary_o_block`].
pub fn block_of(name: &str) -> Option<MaryOBlock> {
    let rest = name.strip_prefix(ENCODED_PREFIX)?;
    let (look, rest) = rest.split_once(':')?;
    let look = MaryOBlockLook::parse(look)?;
    // a name with only two fields is the OLD encoding, and it decodes to this look's
    // default contents rather than to `None`.
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
/// a missing or unknown `kind` is a REFUSAL, not a default. A block that
/// silently became a plain wall is the worst outcome available: it still stops
/// the player, so the level looks whole and one bonus is quietly gone. The
/// author gets told at load which entity and what the choices are.
pub fn convert_mary_o_block(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, _name, min, size) = ctx.parts();
    let authored =
        ambition_platformer2d::ldtk_map::field_string(entity, "kind").unwrap_or_default();
    let Some(look) = MaryOBlockLook::parse(&authored) else {
        // the retired word gets its own sentence. `Quasar` was a real
        // authored value in this very file until, so an author who
        // reaches for it is remembering correctly and needs the replacement
        // rather than a list they have to diff against their memory.
        let hint = if authored.trim().eq_ignore_ascii_case("quasar") {
            ". `Quasar` was retired: a look may not name its contents. Say \
             kind `Question` with contents `AlwaysQuasar`"
        } else {
            ""
        };
        return Err(format!(
            "MaryOBlock `{}` has kind {authored:?}, which is not one of Question, Brick, Hidden{hint}",
            entity.iid
        ));
    };
    // An author opts in to a hidden powerup; nobody has to migrate.
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
    // `reactive_block` stamps the durable identity, because `Block::solid`
    // does not. Its own doc says so — *"fixture constructors default to
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

// ── THE WARP TUBE ──────────────────────────────────────────────────────────
//
// schema (maybe similar to how we do portals)."*
//
// * `link` — who my partner is. Two halves sharing one are ONE tube.
// * `mouth` — which way my open face points, `Up` or `Down`. This is the portal's `normal` for a thing that is always vertical, and it is what decides the PRESS: you press DOWN into an up-facing mouth and UP into a
// down-facing one. the rule — a pipe answers UP or DOWN, never a generic Interact — is this field.
// * `role` — `Entrance` or `Exit`. A portal pair is symmetric and a warp tube is not: 1-1's descent tube swallows you at the surface and spits you out underground, and its ascent tube does the reverse. Without this the two vault halves are geometrically identical (both hang from the ceiling, mouth down) and nothing could say which one you may press UP into.
//
// `PlacementSchema` could NOT carry this. It is a closed enum in `ambition_entity_catalog` with
// a fingerprinted `PlacementKind::stable_id` beside it, so a `Pipe` variant would put one game's
// noun in the engine's construction-schema contract — exactly what `MaryOBlock`'s own header
// refuses ("the engine must not interpret Mary-O's progression"). A pipe is also SOLID GEOMETRY,
// which a placement is not: its collision is the tube you walk into.

/// Which way a pipe half's OPEN FACE points, and so which press enters it.
///
/// Screen-relative because the tube is: `Up` is the lip you stand on and drop
/// into, `Down` is the lip hanging overhead that you rise into or fall out of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaryOPipeMouth {
    /// Open face on TOP. You stand on it and press DOWN; you arrive standing on
    /// it.
    Up,
    /// Open face on the BOTTOM — a pipe hanging from a ceiling. You stand under
    /// it and press UP; you arrive falling out of it.
    Down,
}

impl MaryOPipeMouth {
    /// The word an author picks in the editor.
    pub fn authored(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Down => "Down",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "up" | "top" => Some(Self::Up),
            "down" | "bottom" => Some(Self::Down),
            _ => None,
        }
    }
}

/// Which END of a tube this half is.
///
/// the one place a warp tube is NOT a portal. A portal pair is symmetric, so
/// `link` alone is the whole relation; a warp tube is directed, and the two
/// halves of 1-1's descent are told apart by nothing else — both are 96×148
/// boxes in the same column, one hanging from the vault ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaryOPipeRole {
    /// The mouth you press INTO. Exactly one per link.
    Entrance,
    /// The mouth you come OUT of. Exactly one per link.
    Exit,
}

impl MaryOPipeRole {
    pub fn authored(self) -> &'static str {
        match self {
            Self::Entrance => "Entrance",
            Self::Exit => "Exit",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "entrance" | "enter" | "in" => Some(Self::Entrance),
            "exit" | "out" => Some(Self::Exit),
            _ => None,
        }
    }
}

/// One authored half of a warp tube: who it pairs with, which way it opens, and
/// which end of the trip it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaryOPipe {
    pub link: String,
    pub mouth: MaryOPipeMouth,
    pub role: MaryOPipeRole,
}

impl MaryOPipe {
    pub fn new(link: impl Into<String>, mouth: MaryOPipeMouth, role: MaryOPipeRole) -> Self {
        Self {
            link: link.into(),
            mouth,
            role,
        }
    }
}

/// The LDtk entity identifier an author places for one half of a tube.
pub const MARY_O_PIPE: &str = "MaryOPipe";

/// The prefix every converted pipe half carries.
const PIPE_ENCODED_PREFIX: &str = "maryo_pipe:";

/// Encode one authored pipe half into the only channel a `Block` has.
///
/// ```text
/// maryo_pipe:<link>:<mouth>:<role>:<iid>
/// ```
///
/// the iid stays LAST, as in `maryo_block:`, so it keeps being the only
/// field that may contain anything — which is why a `link` containing a colon
/// is refused at conversion rather than silently splitting into two.
pub fn pipe_encoded_name(pipe: &MaryOPipe, iid: &str) -> String {
    format!(
        "{PIPE_ENCODED_PREFIX}{}:{}:{}:{iid}",
        pipe.link,
        pipe.mouth.authored(),
        pipe.role.authored()
    )
}

/// A pipe half, built the way [`convert_mary_o_pipe`] builds one: encoded name,
/// and the durable placement identity that `Block::solid` does not set.
pub fn pipe_block(pipe: &MaryOPipe, iid: &str, min: ae::Vec2, size: ae::Vec2) -> ae::Block {
    let mut solid = ae::Block::solid(pipe_encoded_name(pipe, iid), min, size);
    solid.id = ae::GeoId::placement(ae::PlacementId::new(iid.to_string()), 0);
    solid
}

/// The pipe half this authored NAME describes, or `None` when the name did not
/// come from [`convert_mary_o_pipe`].
pub fn pipe_of(name: &str) -> Option<MaryOPipe> {
    let rest = name.strip_prefix(PIPE_ENCODED_PREFIX)?;
    let (link, rest) = rest.split_once(':')?;
    let (mouth, rest) = rest.split_once(':')?;
    let (role, _iid) = rest.split_once(':')?;
    if link.is_empty() {
        return None;
    }
    Some(MaryOPipe::new(
        link,
        MaryOPipeMouth::parse(mouth)?,
        MaryOPipeRole::parse(role)?,
    ))
}

/// `MaryOPipe` → a solid tube half carrying its link, mouth and role.
///
/// every field is REQUIRED and a bad one is a refusal, for the reason
/// `MaryOBlock`'s `kind` is: a pipe half that quietly stopped being half of a
/// tube is still a solid green box in the level, so nothing looks wrong and the
/// warp is simply gone. The author gets told at load which entity and why.
///
/// the PAIRING is not checked here and cannot be — a converter sees ONE
/// entity and has no idea what else the level holds. `pipe_tubes` in `lib.rs`
/// is the load-time check that a link has both its halves.
pub fn convert_mary_o_pipe(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, _name, min, size) = ctx.parts();
    let link = ambition_platformer2d::ldtk_map::field_string(entity, "link")
        .unwrap_or_default()
        .trim()
        .to_string();
    if link.is_empty() {
        return Err(format!(
            "MaryOPipe `{}` has no `link` — a pipe half is paired with its \
             partner by an explicit link id, and two halves sharing one are ONE \
             tube",
            entity.iid
        ));
    }
    if link.contains(':') {
        return Err(format!(
            "MaryOPipe `{}` has link {link:?}, which contains a `:` — that is \
             the separator the authored name is encoded with",
            entity.iid
        ));
    }
    let authored_mouth =
        ambition_platformer2d::ldtk_map::field_string(entity, "mouth").unwrap_or_default();
    let Some(mouth) = MaryOPipeMouth::parse(&authored_mouth) else {
        return Err(format!(
            "MaryOPipe `{}` has mouth {authored_mouth:?}, which is not Up or \
             Down. A mouth is the pipe's OPEN FACE, and it is what decides the \
             press: DOWN into an Up mouth, UP into a Down one",
            entity.iid
        ));
    };
    let authored_role =
        ambition_platformer2d::ldtk_map::field_string(entity, "role").unwrap_or_default();
    let Some(role) = MaryOPipeRole::parse(&authored_role) else {
        return Err(format!(
            "MaryOPipe `{}` has role {authored_role:?}, which is not Entrance \
             or Exit. Each link needs exactly one of each — the tube is a trip, \
             not a doorway",
            entity.iid
        ));
    };
    let pipe = MaryOPipe::new(link, mouth, role);
    let mut emission = RoomEmission::default();
    emission
        .blocks
        .push(pipe_block(&pipe, &entity.iid, min, size));
    Ok(emission)
}

/// Mary-O's LDtk vocabulary: the engine's nouns plus her own.
///
/// The reason given for the global was that conversion runs from pure non-system code with no
/// `World` in hand, which is true and argues for a PARAMETER rather than for ambient state: a value
/// passed in is exactly as reachable from a tool as from a system.
///
/// it still belongs to the READER, not to the plugin. `MaryOBlock` is
/// hers and conversion refuses an identifier it cannot convert, loudly and by
/// design — so every test, tool and probe that loads her level has to say this,
/// or get nine refusals. Handing it over at the load is what makes that
/// impossible to forget, where a build-time install was something you could
/// simply not have done yet.
pub fn vocabulary() -> ambition_platformer2d::ldtk_map::LdtkVocabulary {
    ambition_platformer2d::ldtk_map::LdtkVocabulary::extended_by([
        (
            MARY_O_BLOCK.to_string(),
            convert_mary_o_block as ambition_platformer2d::ldtk_map::LdtkEntityConverter,
        ),
        (
            MARY_O_PIPE.to_string(),
            convert_mary_o_pipe as ambition_platformer2d::ldtk_map::LdtkEntityConverter,
        ),
    ])
}

/// Every LDtk noun Mary-O owns, and the list [`install`] is checked against.
///
/// this exists so the AUTHORING TOOLS can be told the truth. The Python
/// validator cannot run Rust, so it reads a declared manifest —
/// `assets/worlds/mary_o.entities.json` — to know that `MaryOBlock` is
/// convertible. A declaration nobody checks is just a wish, so this slice and
/// that manifest are pinned against each other by a test below. Neither is
/// derived from the other, which is the whole point: the first attempt let
/// validation accept anything the project DEFINED, and a `BogusEntity`
/// definition with no converter anywhere passed.
pub const MARY_O_LDTK_ENTITY_IDENTIFIERS: &[&str] = &[MARY_O_BLOCK, MARY_O_PIPE];

/// The manifest the LDtk tooling reads, compiled in so the pin below cannot go
/// looking for a file that moved.
#[cfg(test)]
const MARY_O_ENTITY_MANIFEST: &str = include_str!("../assets/worlds/mary_o.entities.json");

/// The word is gone from every road that could have answered it.
#[test]
fn the_retired_quasar_look_resolves_to_nothing_at_all() {
    // Neither road answers it: not the author's word, and not an encoded name
    // left over in a saved room.
    assert_eq!(MaryOBlockLook::parse("Quasar"), None);
    assert_eq!(MaryOBlockLook::parse("quasar_block"), None);
    assert_eq!(block_of("maryo_block:Quasar:Solid-1"), None);
    // and the words it was NOT: `quasar` still names a PICKUP, which is the
    // whole point of the split. Deleting both would have been the easy mistake.
    assert_eq!(MaryOPickup::parse("quasar"), Some(MaryOPickup::Quasar));
    assert_eq!(
        MaryOBlockContents::parse("AlwaysQuasar"),
        Some(MaryOBlockContents::Always(MaryOPickup::Quasar))
    );
}

/// Only the HIDDEN look loses its floor.
///
/// the control is the point: a ?-block and a brick are both still `Solid`, so
/// this pins the DIFFERENCE rather than asserting that some block somewhere is
/// pass-through.
#[test]
fn a_hidden_block_is_the_only_one_that_is_not_a_floor() {
    for look in [MaryOBlockLook::Question, MaryOBlockLook::Brick] {
        let block = reactive_block(
            MaryOBlock::plain(look),
            "iid",
            ae::Vec2::ZERO,
            ae::Vec2::splat(32.0),
        );
        assert!(
            matches!(block.kind, ae::BlockKind::Solid),
            "{look:?} stopped being a floor, and only Hidden should have"
        );
    }
    let hidden = reactive_block(
        MaryOBlock::plain(MaryOBlockLook::Hidden),
        "iid",
        ae::Vec2::ZERO,
        ae::Vec2::splat(32.0),
    );
    assert!(
        matches!(hidden.kind, ae::BlockKind::BonkOnly),
        "a hidden block is still a floor you cannot see"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `"identifier": "..."` in the tooling manifest.
    ///
    /// a hand-rolled scan, on purpose. This crate's manifest is deliberately two
    /// dependencies wide — its own comment: *"a downstream game names `ambition_platformer2d` +
    /// `bevy`, and NOTHING ELSE"* — and pulling in a JSON parser to read one list of strings
    /// would spend that claim on a test.
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

    /// Audit that Mary-O installs converters for the entity vocabulary declared
    /// to external LDtk tooling. The manifest and converter list are independent
    /// authorities so disagreement fails validation.
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
    /// this is the ONE thing worth pinning about the encoding — that the two
    /// halves agree — because they are the only two readers and a silent
    /// disagreement makes an authored bonus block into a plain wall.
    #[test]
    fn every_kind_survives_the_name_it_is_encoded_into() {
        for look in [
            MaryOBlockLook::Question,
            MaryOBlockLook::Brick,
            MaryOBlockLook::Hidden,
        ] {
            // every look crossed with every contents, because the whole
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

    /// Round-trip for the tube half: what the converter encodes, the runtime
    /// decodes — every mouth crossed with every role, because the two are
    /// independent and an encoder that dropped one would still be green over a
    /// diagonal.
    #[test]
    fn every_pipe_half_survives_the_name_it_is_encoded_into() {
        for mouth in [MaryOPipeMouth::Up, MaryOPipeMouth::Down] {
            for role in [MaryOPipeRole::Entrance, MaryOPipeRole::Exit] {
                // a link with a hyphen and one with a digit, because a link is
                // whatever an author types and the encoding must not care.
                for link in ["descent", "ascent", "vault-2", "b3"] {
                    let pipe = MaryOPipe::new(link, mouth, role);
                    let name = pipe_encoded_name(&pipe, "MaryOPipe-4321");
                    assert_eq!(
                        pipe_of(&name).as_ref(),
                        Some(&pipe),
                        "`{name}` must decode back to the half it was encoded from"
                    );
                }
            }
        }
    }

    /// A pipe half's name is not a block's, and neither is the level's ordinary
    /// stone — the two decoders must not claim each other's blocks.
    #[test]
    fn the_two_mary_o_decoders_do_not_claim_each_others_blocks() {
        let pipe = pipe_encoded_name(
            &MaryOPipe::new("descent", MaryOPipeMouth::Up, MaryOPipeRole::Entrance),
            "MaryOPipe-1",
        );
        let block = encoded_name(MaryOBlock::plain(MaryOBlockLook::Question), "MaryOBlock-1");
        assert_eq!(block_of(&pipe), None, "a tube half is not a reactive block");
        assert_eq!(pipe_of(&block), None, "a reactive block is not a tube half");
        for name in [
            "vault_floor",
            "goal_pole",
            "maryo_pipe:",
            "maryo_pipe:a:b",
            "",
        ] {
            assert_eq!(pipe_of(name), None, "`{name}` is not a pipe half");
        }
    }

    /// A typo in a pipe's own fields is REFUSED too — a half that quietly
    /// stopped being a pipe is a solid green box standing in the level with the
    /// warp silently gone, which is the same failure `kind` has below.
    #[test]
    fn an_unknown_mouth_or_role_is_not_silently_a_default() {
        for bad in ["", "Upp", "sideways", "left", "Entrence"] {
            assert_eq!(MaryOPipeMouth::parse(bad), None, "`{bad}` is not a mouth");
        }
        for bad in ["", "Entrence", "Exti", "Up", "both"] {
            assert_eq!(MaryOPipeRole::parse(bad), None, "`{bad}` is not a role");
        }
        // ...and the spellings an author reasonably reaches for DO work.
        assert_eq!(MaryOPipeMouth::parse(" Down "), Some(MaryOPipeMouth::Down));
        assert_eq!(MaryOPipeRole::parse("exit"), Some(MaryOPipeRole::Exit));
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

#[cfg(test)]
mod multi_coin_block_tests {
    use super::*;

    /// ⛔⛔ THE SEPARATOR RULE APPLIES TO THE WRAPPERS TOO, and only the `Coins`
    /// side was tested.
    ///
    /// Found by poisoning `strip_separator`: it reddened exactly ONE test, the
    /// bare-coins one, so the `always…` / `toward…` call site was running
    /// unguarded. The set is a rule about the AUTHORING SURFACE — it decides
    /// whether `Toward=Mushroom`, `Toward_Mushroom` and `Toward Mushroom` all
    /// mean the same thing — and an author told YES by one spelling and NO by
    /// another gets no error, just a `None` that reads as "not that variant".
    #[test]
    fn a_wrapper_accepts_every_separator_the_coins_prefix_does() {
        let expected = MaryOBlockContents::parse("TowardWand");
        assert!(
            expected.is_some(),
            "the bare spelling must parse, or this test compares two Nones"
        );
        for spelling in ["Toward Wand", "Toward=Wand", "Toward_Wand", "Toward-Wand"] {
            assert_eq!(
                MaryOBlockContents::parse(spelling),
                expected,
                "`{spelling}` and `TowardWand` must mean the same thing; the \
                 separator set is one rule and an author cannot be told YES by one \
                 spelling and NO by another"
            );
        }
    }

    /// of that block."* So a bare `Coins` must be the classic single-coin block
    /// — an author who wants the common case writes no number.
    #[test]
    fn a_bare_coins_block_is_one_coin() {
        assert_eq!(
            MaryOBlockContents::parse("Coins"),
            Some(MaryOBlockContents::Coins(1))
        );
        assert_eq!(
            MaryOBlockContents::parse("coins = 5"),
            Some(MaryOBlockContents::Coins(5))
        );
        assert_eq!(
            MaryOBlockContents::parse("Coins12"),
            Some(MaryOBlockContents::Coins(12))
        );
    }

    /// zero is refused rather than accepted as an empty block. A
    /// `Coins0` would author a block that flinches, changes art and owes
    /// nothing — indistinguishable in play from a bug, and `Empty` already says
    /// that on purpose.
    #[test]
    fn a_zero_count_is_not_authorable() {
        assert_eq!(MaryOBlockContents::parse("Coins0"), None);
        assert_eq!(MaryOBlockContents::parse("coins = 0"), None);
    }

    /// The authored word round-trips, which is what keeps a map edit from
    /// silently becoming a different block on the next save.
    #[test]
    fn the_authored_word_round_trips() {
        for contents in [
            MaryOBlockContents::Coins(1),
            MaryOBlockContents::Coins(7),
            MaryOBlockContents::Empty,
            MaryOBlockContents::Always(MaryOPickup::Wand),
        ] {
            assert_eq!(
                MaryOBlockContents::parse(&contents.authored()),
                Some(contents),
                "`{}` did not survive a round trip",
                contents.authored()
            );
        }
    }

    /// POISON: `Coins` must not swallow another word that starts with it.
    /// The parser strips the prefix and then reads a number, so anything else
    /// following must be refused rather than silently read as a count.
    #[test]
    fn a_word_that_merely_starts_with_coins_is_refused() {
        assert_eq!(MaryOBlockContents::parse("CoinsWand"), None);
        assert_eq!(MaryOBlockContents::parse("AlwaysCoin").is_some(), true);
    }
}
