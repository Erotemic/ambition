//! THE NAMESPACES CHARACTER PREPARATION RESOLVES AGAINST — the vocabulary
//! every cross-layer reference a character makes is checked in.
//!
//! These namespace contracts live in a small boundary crate so the canonical
//! character domain does not depend on broad platformer lifecycle infrastructure.
//!
//! a namespace is a MARKER, not a table. Each one names a kind of
//! reference and supplies the word a diagnostic prints; the vocabulary a
//! reference resolves against is supplied per-composition by `CharacterBindings`
//! — which is why `RangedPayload` belongs here despite having no lookup table at
//! all. What it shares with the others is the CHANNEL, not the mechanism.

use ambition_binding::Namespace;

/// The cues a session authorizes. A character's authored cues resolve against
/// this; §4.6 note — a session's authorized set is NOT merely the union over its
/// cast, it also includes stage, ruleset, announcer, world-object, UI and shell
/// dependencies, so the authority is assembled session-level and passed in.
pub struct SfxCueId;

impl Namespace for SfxCueId {
    const NAME: &'static str = "sfx cue";
}

/// The move ids one character's moveset declares. Character-scoped: `swat` in one
/// character has nothing to do with `swat` in another.
pub struct MoveId;

impl Namespace for MoveId {
    const NAME: &'static str = "move";
}

/// The input verbs the moveset runtime can actually press.
///
/// A moveset binding a verb outside it authors a perfectly valid move onto a button that does
/// not exist.
pub struct VerbId;

impl Namespace for VerbId {
    const NAME: &'static str = "input verb";
}

/// Sheet manifest targets the composition can actually resolve.
///
/// A character's `sheet` is the single most consequential cross-layer reference it
/// makes — get it wrong and the character draws a marked rectangle for the rest of
/// the session. It was never resolved at preparation, so a typo here was reported
/// only later, by the art pipeline, as `NoSheetResolved`: true, but at load time
/// and without a did-you-mean.
pub struct SheetTarget;

impl Namespace for SheetTarget {
    const NAME: &'static str = "sheet target";
}

/// Select-screen portrait targets.
pub struct PortraitTarget;

impl Namespace for PortraitTarget {
    const NAME: &'static str = "portrait target";
}

/// The ranged payload an authored `ranged` move needs to throw.
///
/// Not a lookup namespace like the others — there is no table of payloads to misspell.
pub struct RangedPayload;

impl Namespace for RangedPayload {
    const NAME: &'static str = "ranged payload";
}

/// The vfx tags a session's renderers know how to draw.
///
/// §4.6 derives the vfx inventory from the moves that request it, exactly like
/// cues — and then nothing resolved it, so a misspelled `vfx` on a hit volume was
/// derived faithfully into a dependency list nobody checked.
pub struct VfxTag;

impl Namespace for VfxTag {
    const NAME: &'static str = "vfx tag";
}
