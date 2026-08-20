//! **THE FIRST CHARACTER-OWNED `smash.fighter` FACET — values authored as
//! content, prepared into runtime fighter data.**
//!
//! ⭐⭐ **this is D166's restitch point, and it is deliberately ONE SECTION
//! wide.** [`smash_repertoire`](crate::smash_repertoire) and
//! [`smash_capture`](crate::smash_capture) both carry the same standing note —
//! *"do not move it for purity; the restitch point is the first real
//! character-owned `smash.fighter` facet"* — and this module is that facet
//! arriving. What it carries is the CAPTURE kit and nothing else, because the
//! capture kit is the part the 2026-08-18 grab campaign proved is pure authored
//! VALUES: six numbers of geometry, four timings, and three payloads, with no
//! helper composition and no per-move design law in the middle of them.
//!
//! ⛔ **the sixteen-slot repertoire is NOT here and must not be dragged in to
//! make the facet look complete.** A fighter's ordinary moves are authored
//! through composed Rust helpers (`strike`, `impulse`, `on_hit`,
//! `committed_tail`, `feel`) whose composition IS the design, and George's own
//! file carries a `debug_assert` stating a law about the shape of his table.
//! Flattening that into RON would turn authored reasoning into a wall of
//! numbers and buy nothing this row asked for. The capture kit came out clean;
//! the rest has not yet been shown to.
//!
//! ## The three layers, as the queue's D166 row draws them
//!
//! ```text
//! Smash capability   defines the facet's SEMANTICS      this module
//!         ^
//! character package  authors George/Alice/... VALUES    a pack's .ron
//!         v
//! Smash preparation  produces runtime MoveSpecs         into_repertoire()
//! ```
//!
//! The generic engine below is unchanged and learns nothing: preparation hands
//! it the same [`MoveSpec`](ambition_entity_catalog::MoveSpec)s the Rust
//! literals produced, through the same
//! [`smash_capture`](crate::smash_capture) helpers, so the taxonomy stops here
//! exactly as it did before.
//!
//! ## ⚠ The capability is named before it is a crate
//!
//! [`SMASH_FIGHTER_CAPABILITY`] declares an owner for this schema today, inside
//! the crate the types transitionally live in. That is the point: a schema
//! registration must name a capability, so registering one WRITES DOWN the
//! ownership the eventual carve will make structural. ⛔ it is not a licence to
//! start the carve — see the same refusal on both sibling modules.
//!
//! ## What replaced the param registry, and why that note changed
//!
//! [`smash_capture`](crate::smash_capture) used to end with *"if a capture param
//! ever becomes authorable as loose RON, wiring
//! [`ParamSchemaRegistry`](ambition_entity_catalog::ParamSchemaRegistry) is a
//! precondition of that change"*. Capture params are authorable now — and they
//! are not LOOSE. They arrive as typed serde structs with
//! `deny_unknown_fields`, checked by the content compiler before a pack is
//! allowed to reach a runtime, so a typo is a diagnostic naming the file and the
//! field at compile time rather than a param that silently never fires. That is
//! the check the precondition asked for, taken by a stronger road than the one
//! it named.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CapturePummelParams, CaptureThrowParams, SmashCaptureRepertoire,
};

/// The `smash_fighter` schema this capability owns. Behind `content_pack`: a
/// game that never validates its content must not link a compiler.
#[cfg(feature = "content_pack")]
pub mod content_schema;

/// **The capability that owns platform-fighter authoring.**
///
/// Today it is a name on a schema registration rather than a crate, and that is
/// the honest state: D166's ruling is that the types stay where they are until a
/// facet makes the ownership load-bearing, and this constant is what makes it
/// load-bearing. A composition that installs the schema installs this
/// capability, so "who owns a fighter's authored values" has one answer that a
/// tool can print.
pub const SMASH_FIGHTER_CAPABILITY: &str = "smash_fighter";

/// The authored FILE kind: one character's platform-fighter facet.
pub const SMASH_FIGHTER_SCHEMA: &str = "smash_fighter";

/// **One character's platform-fighter values.**
///
/// One file, one character — not a book keyed by id. A character package owns
/// its own file, so adding a fighter never edits another fighter's, and a merge
/// conflict names a character instead of a line number.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmashFighterFacet {
    /// The character id these values belong to — the same string the game
    /// registers the character under.
    pub character: String,
    /// The capture kit: grab, pummel, throws.
    pub capture: CaptureKitAuthoring,
}

/// **A fighter's capture kit, as VALUES.**
///
/// The authored mirror of [`SmashCaptureRepertoire`], which is the same kit as
/// runtime [`MoveSpec`](ambition_entity_catalog::MoveSpec)s.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureKitAuthoring {
    pub grab: GrabAuthoring,
    pub pummel: PummelAuthoring,
    /// Required. A capture kit whose only throw is unauthored would acquire a
    /// captive it can never dispose of except by the hold timing out.
    pub forward_throw: ThrowAuthoring,
    /// ⛔ an unauthored throw does NOTHING, and deliberately does not fall back
    /// to a pummel — see [`SmashCaptureRepertoire`]. `None` is the authored way
    /// to say "this fighter has no back throw".
    #[serde(default)]
    pub back_throw: Option<ThrowAuthoring>,
    #[serde(default)]
    pub up_throw: Option<ThrowAuthoring>,
    #[serde(default)]
    pub down_throw: Option<ThrowAuthoring>,
}

/// The standing grab: a three-window shell plus the reach it sustains.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrabAuthoring {
    /// The move id. Unique within the kit — the contract looks a move up by it.
    pub id: String,
    /// The animation clip. Falls back to `attack` then `idle`.
    pub clip: String,
    pub startup_s: f32,
    /// How long the reach is LIVE. The whole difference between a grab and a
    /// recovery animation.
    pub active_s: f32,
    pub recover_s: f32,
    pub reach: CaptureAttemptParams,
}

/// The pummel: a beat with one impact instant on it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PummelAuthoring {
    pub id: String,
    pub clip: String,
    pub duration_s: f32,
    /// Where on its own timeline the damage lands.
    pub impact_at_s: f32,
    pub impact: CapturePummelParams,
}

/// One throw: a beat with the RELEASE instant on it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThrowAuthoring {
    pub id: String,
    pub clip: String,
    pub duration_s: f32,
    /// ⭐ the release is a timeline instant, not the button press — the captive
    /// stays constrained through the wind-up and leaves here, which is what
    /// makes a throw readable and punishable rather than instantaneous.
    pub release_at_s: f32,
    pub launch: CaptureThrowParams,
}

/// Every character's facet in one pack, keyed by character id.
pub type SmashFighterBook = BTreeMap<String, SmashFighterFacet>;

impl ThrowAuthoring {
    fn spec(self) -> ambition_entity_catalog::MoveSpec {
        author_throw(
            capture_beat(&self.id, &self.clip, self.duration_s),
            self.release_at_s,
            self.launch,
        )
    }
}

impl CaptureKitAuthoring {
    /// **Preparation: authored values become the runtime kit.**
    ///
    /// Through the same [`smash_capture`](crate::smash_capture) helpers a Rust
    /// literal uses, deliberately — a second lowering road would be a second
    /// place for the effect keys and the window shape to drift.
    ///
    /// # Panics
    ///
    /// If the values are ones [`Self::problems`] refuses (a grab with no live
    /// window is the one `author_standing_grab` itself asserts on). The content
    /// compiler runs `problems` before lowering, so a pack cannot reach here
    /// carrying them; a hand-built value can, and finding out loudly is right.
    pub fn into_repertoire(self) -> SmashCaptureRepertoire {
        let Self {
            grab,
            pummel,
            forward_throw,
            back_throw,
            up_throw,
            down_throw,
        } = self;
        SmashCaptureRepertoire {
            // A facet-authored kit takes the shipped rows; a fighter whose art
            // is its own overrides them where it builds its repertoire.
            cues: crate::smash_capture::CaptureCues::GENERIC,
            grab: author_standing_grab(
                grab_shell(
                    &grab.id,
                    &grab.clip,
                    grab.startup_s,
                    grab.active_s,
                    grab.recover_s,
                ),
                grab.reach,
            ),
            pummel: author_pummel(
                capture_beat(&pummel.id, &pummel.clip, pummel.duration_s),
                pummel.impact_at_s,
                pummel.impact,
            ),
            forward_throw: forward_throw.spec(),
            back_throw: back_throw.map(ThrowAuthoring::spec),
            up_throw: up_throw.map(ThrowAuthoring::spec),
            down_throw: down_throw.map(ThrowAuthoring::spec),
        }
    }
}

impl SmashFighterFacet {
    /// **Everything this facet can SAY that the runtime cannot USE.**
    ///
    /// The list is deliberately not a taste filter. Every entry names a value
    /// whose consequence is that some authored thing never happens at all —
    /// a reach that cannot overlap, an impact past the end of its own move, a
    /// release the captive never reaches — because that is the class of fault
    /// that is invisible in a playtest and looks like the mechanic being bad.
    ///
    /// ⚠ what is NOT here: balance. A 400-knockback pummel and a two-second grab
    /// are both usable and both terrible, and refusing them would make this
    /// module the designer.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.character.trim().is_empty() {
            out.push("`character` is empty, so nothing can look this facet up".to_string());
        }
        self.capture.problems(&mut out);
        out
    }
}

impl CaptureKitAuthoring {
    fn problems(&self, out: &mut Vec<String>) {
        let grab = &self.grab;
        named(&grab.id, "the grab", out);
        clipped(&grab.id, &grab.clip, out);
        finite_non_negative(&grab.id, "startup_s", grab.startup_s, out);
        finite_non_negative(&grab.id, "recover_s", grab.recover_s, out);
        if grab.active_s.is_nan() || grab.active_s <= 0.0 {
            out.push(format!(
                "grab `{}` is live for {}s, so its reach is never asked about: it would \
                 play, cost its recovery, and be unable to catch anybody",
                grab.id, grab.active_s
            ));
        }
        let (hw, hh) = grab.reach.half_extents;
        if hw.is_nan() || hh.is_nan() || hw <= 0.0 || hh <= 0.0 {
            out.push(format!(
                "grab `{}` reaches a box of half-extents ({hw}, {hh}), which has no area \
                 and can never overlap a body",
                grab.id
            ));
        }

        let pummel = &self.pummel;
        named(&pummel.id, "the pummel", out);
        clipped(&pummel.id, &pummel.clip, out);
        finite_non_negative(&pummel.id, "duration_s", pummel.duration_s, out);
        instant_inside(
            &pummel.id,
            "impact_at_s",
            pummel.impact_at_s,
            pummel.duration_s,
            "so the pummel's damage never lands",
            out,
        );

        for (slot, throw) in self.throws() {
            named(&throw.id, slot, out);
            clipped(&throw.id, &throw.clip, out);
            finite_non_negative(&throw.id, "duration_s", throw.duration_s, out);
            instant_inside(
                &throw.id,
                "release_at_s",
                throw.release_at_s,
                throw.duration_s,
                "so the captive is never released and the hold outlives the throw",
                out,
            );
            let (dx, dy) = throw.launch.launch_dir;
            if dx == 0.0 && dy == 0.0 {
                out.push(format!(
                    "throw `{}` launches in no direction, so its knockback has nowhere to \
                     send the captive",
                    throw.id
                ));
            }
            if !dx.is_finite() || !dy.is_finite() {
                out.push(format!(
                    "throw `{}` has a non-finite launch direction ({dx}, {dy})",
                    throw.id
                ));
            }
        }

        // ⛔ **two capture moves sharing an id is a COLLISION, not a duplicate
        // row.** A `MovesetContract` resolves a move by its id, so the second
        // one would be reachable through its verb and unreachable through
        // everything else that names a move — cancel windows, hitlag
        // bookkeeping, and the fighter's own frame-data assertions.
        let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
        let mut ids: Vec<(&str, &str)> = vec![
            (self.grab.id.as_str(), "the grab"),
            (self.pummel.id.as_str(), "the pummel"),
        ];
        ids.extend(self.throws().map(|(slot, throw)| (throw.id.as_str(), slot)));
        for (id, slot) in ids {
            if let Some(first) = seen.insert(id, slot) {
                out.push(format!(
                    "move id `{id}` is used by both {first} and {slot}; one capture kit, \
                     one id per move"
                ));
            }
        }
    }

    /// Every authored throw with the slot name it fills.
    fn throws(&self) -> impl Iterator<Item = (&'static str, &ThrowAuthoring)> {
        [
            ("the forward throw", Some(&self.forward_throw)),
            ("the back throw", self.back_throw.as_ref()),
            ("the up throw", self.up_throw.as_ref()),
            ("the down throw", self.down_throw.as_ref()),
        ]
        .into_iter()
        .filter_map(|(slot, throw)| throw.map(|throw| (slot, throw)))
    }
}

fn named(id: &str, slot: &str, out: &mut Vec<String>) {
    if id.trim().is_empty() {
        out.push(format!("{slot} has no move id, so nothing can select it"));
    }
}

fn clipped(id: &str, clip: &str, out: &mut Vec<String>) {
    if clip.trim().is_empty() {
        out.push(format!(
            "move `{id}` names no animation clip; a capture move falls back to \
             `attack` then `idle`, but the fallback is a LAST resort and an empty \
             binding is an authoring slip rather than a choice"
        ));
    }
}

fn finite_non_negative(id: &str, field: &str, value: f32, out: &mut Vec<String>) {
    if !value.is_finite() || value < 0.0 {
        out.push(format!(
            "move `{id}` has {field} = {value}, which is not a duration"
        ));
    }
}

fn instant_inside(
    id: &str,
    field: &str,
    at_s: f32,
    duration_s: f32,
    consequence: &str,
    out: &mut Vec<String>,
) {
    if !at_s.is_finite() || at_s < 0.0 {
        out.push(format!(
            "move `{id}` has {field} = {at_s}, which is not a point on a timeline"
        ));
    } else if at_s > duration_s {
        out.push(format!(
            "move `{id}` is {duration_s}s long and its {field} is {at_s}, {consequence}"
        ));
    }
}

#[cfg(test)]
mod tests;
