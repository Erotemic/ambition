//! `SmashHoldState` — the platform-fighter RULES of a hold, as runtime state.
//!
//! ⭐⭐ **IT LIVES ALONE BECAUSE IT IS THE ONE THING IN THE CAPTURE FAMILY THAT
//! IS NOT AUTHORING.** Every other item `smash_capture` holds is a pure
//! `MoveSpec` constructor — a value an offline builder can produce without an
//! engine. This is a rollback-registered Bevy `Component`, and while it sat in
//! that file the whole capture vocabulary was pinned to a Bevy-linked crate for
//! the sake of one derive (fast-iteration I1: *"move only functions which
//! construct pure MoveSpec values"*).
//!
//! ⚠ Its lifetime is `CapturedBy`'s: it rides BESIDE that component on the
//! captive, and a hold with no `SmashHoldState` is a hold this ruleset has no
//! opinion about.

/// why this is not on `CapturedBy` (`ambition_combat::capture::CapturedBy`,
/// re-exported as `ambition_platformer2d::capture::CapturedBy`) any more. That
/// component is the RELATION: who holds whom, where, and what physical state release must give
/// back. Every field of it is answerable without knowing what genre is being played.
///
///  they were fine on the relation while the mechanic was being proven, and
/// they are not convincing final owners. The split is not cosmetic: it is why a
/// capture in another game does not pay to rewind a pummel counter it has no
/// rule for.
///
///  it rides BESIDE `CapturedBy` on the captive, and its lifetime is that
/// component's. A hold with no `SmashHoldState` is a hold this ruleset has no
/// opinion about, which is the honest reading for a game that constrains bodies
/// without pummelling them.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct SmashHoldState {
    pub pummels_landed: u8,
    /// How long this hold has lasted, in the same scaled seconds a move
    /// timeline advances in — so a capture does not age during hitstop.
    ///
    /// Without an age, a fighter who grabs and then does nothing holds a body for the rest of
    /// the match.
    pub held_for: f32,
    /// What the captive's OWN input has bought toward getting out, in the
    /// same seconds [`Self::held_for`] counts.
    ///
    ///  the shape matters more than the number. A captive is not a body
    /// whose input ceased to exist — it is a body whose input reaches a
    /// restricted channel, and this is that channel's accumulator.
    pub mash_credit: f32,
    /// How long THIS hold lasts, decided when it began.
    ///
    ///  stored rather than recomputed, and that is the genre's rule rather
    /// than a caching trick. Ultimate reads the captive's percent AT THE GRAB;
    /// a hold that re-read it every tick would grow every time its captor
    /// pummelled, which turns a pummel from a decision into a free extension of
    /// the advantage you already have.
    pub escape_seconds: f32,
    /// Has the captor's stick returned to NEUTRAL since this hold began?
    ///
    /// ⛔⛔ A DIRECTION ALONE THROWS, SO IT HAS TO BE A NEW DIRECTION. You walk
    /// into a grab, so the stick that reached it is usually already pointing
    /// somewhere — reading the live axis on the first held tick threw the
    /// victim instantly, before the captor could pummel or choose.
    ///
    /// ⭐ ARMED BY NEUTRAL rather than by remembering the direction at capture:
    /// a captor who grabs holding forward and keeps holding forward has not
    /// pressed anything, and one who centres and pushes forward again has —
    /// same final direction, different input.
    ///
    /// ⛔ AND IT LIVES HERE, NOT ON `CapturedBy`, which is the whole reason this
    /// component exists. "Centre the stick before a direction throws" is a
    /// platform-fighter INPUT rule, not a fact about who holds whom or what
    /// release must restore. A game that constrains bodies without a throw
    /// vocabulary should not pay to rewind this, exactly as it does not pay to
    /// rewind `pummels_landed`.
    pub throw_armed: bool,
    /// May this hold's captor WALK while holding?
    ///
    /// ⭐⭐ THE CARGO CARRY, and it lives HERE for the reason the note on
    /// [`Self::throw_armed`] gives about itself: "may the captor move" is a
    /// platform-fighter rule, not a fact about who holds whom. `CapturedBy` is
    /// the generic relation and has no opinion about locomotion — a game that
    /// constrains bodies without a throw vocabulary should not pay to rewind
    /// this, exactly as it does not pay to rewind `pummels_landed`.
    ///
    /// ⛔ IT REWINDS. It is decided once and then constant, which is precisely
    /// why a rollback that restored the hold without it would be wrong: the
    /// resimulated timeline would hand the captor back a hold they can no
    /// longer walk with, and a carry that ends on one peer and not the other is
    /// a divergence in position, not just in state.
    ///
    /// ⚠ FALSE BY DEFAULT, and `Default` is how every existing hold gets it. An
    /// ordinary grab pins its captor, which is the genre's rule and was this
    /// engine's only behaviour before the carry existed.
    pub carrying: bool,
}

impl SmashHoldState {
    /// A fresh hold that lasts `escape_seconds`.
    ///
    ///  the only way to start one, and `Default` is not it. A default row
    /// has `escape_seconds == 0.0`, which [`Self::escaped`] correctly reads as a
    /// hold already over — so a fixture that reached for `default()` would watch
    /// its capture end on tick one and call that a timeout.
    pub fn lasting(escape_seconds: f32) -> Self {
        Self {
            escape_seconds,
            ..Default::default()
        }
    }

    /// Is this hold over? The ONE place the two clocks are compared, so no
    /// caller can end a hold by half the rule.
    pub fn escaped(&self) -> bool {
        self.held_for + self.mash_credit >= self.escape_seconds
    }
}
