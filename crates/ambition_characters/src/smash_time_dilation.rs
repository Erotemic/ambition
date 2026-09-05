//! The time-dilation technique: "for a moment, you are slower than the world."
//!
//! ⭐⭐ THE SAME SPLIT `smash_counter`, `smash_teleport` AND `smash_capture` USE.
//! A key and its params are what a MOVESET authors, so they live where movesets
//! can name them; applying the scale and spending its clock are the GAME's job,
//! and that half sits in the smash demo beside the capture adapter.
//!
//! ⛔⛔ THIS ADDS NO TIME AUTHORITY, AND THAT IS THE WHOLE POINT.
//! `ambition_time::ProperTimeScale` is a per-body component the engine ALREADY
//! integrates against: `WorldTime::entity_dt` is what move playback, hurtbox
//! resolution and the animation clock all read (ADR 0011), and the component is
//! already rollback-canonical as `actor.proper_time_scale`. ⇒ What was missing
//! was a way for an authored move to SAY a number into it. Nothing here decides
//! how time works; it decides who is slow and for how long.
//!
//! ⭐ AND IT IS THE THIRD COUNTER THE COUNTER MODULE NAMES. `smash_counter`'s own
//! header lists the three a platform fighter wants — *"an ordinary counter
//! answers with an attack; a Revenge-style counter answers with a lasting
//! character modifier; a Witch-Time-style counter answers by SLOWING THE
//! ATTACKER"* — and the third had no vocabulary at all while the first two
//! shipped. A counter's `response` is a key, so this makes the third expressible
//! without the counter learning anything new.
//!
//! ⚠ WHAT IT DOES NOT SLOW, MEASURED RATHER THAN GUESSED: the movement kernel
//! does not read `entity_dt`, so a dilated body still WALKS at ordinary speed.
//! Its moves, its hurtbox resolution and its animation all slow. ⇒ For the
//! counter case that is the effect that matters — the fighter you caught is
//! mid-swing, and their swing is what stretches — but a move that dilated a body
//! in neutral would look wrong, and this doc is where the next author finds that
//! out instead of the playtest.

use serde::{Deserialize, Serialize};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const TIME_DILATION: &str = "smash.time_dilation";

/// Authored parameters of one dilation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeDilationParams {
    /// The victim's clock multiplier while it lasts. `1.0` is no change, `0.35`
    /// is a hard slow, `0.0` is a freeze.
    ///
    /// ⛔ BELOW ONE IS THE ONLY DIRECTION WITH A CUSTOMER. Above one would speed
    /// a body up, which the engine supports and no move asks for — so the
    /// authoring guard refuses it rather than shipping a knob whose effect
    /// nobody has designed.
    pub scale: f32,
    /// How long the victim stays on that clock, in WORLD seconds.
    ///
    /// ⭐ WORLD SECONDS, NOT THE VICTIM'S OWN. A duration measured on a slowed
    /// body's own clock would stretch itself — a 0.5s slow at `0.35` would last
    /// 1.4s of real time, and halving the scale would more than double the
    /// effect. The author writes how long the OTHER player waits.
    pub seconds: f32,
}

impl TimeDilationParams {
    /// Everything wrong with these params, as sentences an author can act on.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if !(0.0..1.0).contains(&self.scale) {
            problems.push(format!(
                "scale {} is not a slow: below 1.0 is the only direction with a \
                 customer, and 0.0..1.0 is the range that has one",
                self.scale
            ));
        }
        if self.seconds <= 0.0 {
            problems.push(format!(
                "seconds {} would apply a scale nothing ever takes away",
                self.seconds
            ));
        }
        problems
    }
}
