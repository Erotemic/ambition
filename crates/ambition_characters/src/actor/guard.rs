//! A GUARD: a part of a body that turns strikes aside, such as a riot shield.
//!
//! A character's catalog row authors its guards (`guards`). Its art says where
//! each one is, frame by frame: the guard is a [`PartBox`] its part flipbook
//! publishes. The combat sweep checks a strike against the guards before the
//! body. A strike that meets a guard from a side it covers is DEFLECTED: it does
//! no damage, the body that swung is thrown back and locked out of acting for a
//! moment, and the guarded body knows it, so its brain can answer
//! (`MeleeTactics::counter`).
//!
//! Each guard states which strikes get past it: from behind, over the top, and
//! during the rows it is lowered in. A guard that turns too many strikes too
//! fast, or one strike that is too heavy, BREAKS, and the body is open while it
//! staggers. A body can have more than one guard.
//!
//! Jon, 2026-10-19: "We will make more sophisticated shield enemies in the
//! future. So make sure we make the mechanic expressive enough to be reused
//! later."

use serde::{Deserialize, Serialize};

use super::landmarks::PartBox;

/// One guard of a body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardSpec {
    /// The part of the body's art that guards. Its flipbook publishes where it
    /// is in each frame; a frame that does not draw it guards nothing.
    pub part: PartBox,
    /// The strikes it covers, by the side they come from.
    #[serde(default)]
    pub faces: GuardFaces,
    /// A strike from above the top of the guard goes over it: a body that
    /// jumps over the shield and strikes down reaches the head behind it.
    #[serde(default = "open")]
    pub open_above: bool,
    /// The rows of the body's art in which the guard is lowered and covers
    /// nothing, such as its own swing.
    #[serde(default)]
    pub lowered_in: Vec<String>,
    /// How far in from the bounds of the drawn part the guard is, in sheet
    /// pixels: the transparent margin around the part.
    #[serde(default)]
    pub inset_px: f32,
    /// What a deflected strike does to the two bodies.
    #[serde(default)]
    pub deflect: Deflect,
    /// When it breaks. `None`: never.
    #[serde(default)]
    pub breaks: Option<GuardBreak>,
}

fn open() -> bool {
    true
}

/// The sides a guard covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardFaces {
    /// Strikes from the side the body faces. A strike from behind reaches the
    /// body.
    #[default]
    Front,
    /// Strikes from any side.
    Around,
}

/// What a deflected strike does.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Deflect {
    /// How fast the body that swung is thrown back from the guard (px/s).
    pub rebound_speed: f32,
    /// How fast it is thrown up as it bounces off (px/s). A rebound along the
    /// floor alone is stopped dead by the body's own footing; off its feet,
    /// it carries the rebound for the lockout.
    pub rebound_lift: f32,
    /// How long it can do nothing after (s): the guarded body's opening.
    pub lockout_s: f32,
    /// The freeze both bodies take at the contact (s).
    pub hitstop_s: f32,
    /// How far the guarded body is pushed back (px/s).
    pub push: f32,
}

impl Default for Deflect {
    fn default() -> Self {
        Self {
            rebound_speed: 280.0,
            rebound_lift: 160.0,
            lockout_s: 0.45,
            hitstop_s: 0.06,
            push: 60.0,
        }
    }
}

/// When a guard breaks.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardBreak {
    /// It breaks on this many deflects...
    pub blocks: u8,
    /// ...each within this long of the one before (s).
    pub within_s: f32,
    /// A strike of at least this much damage breaks it at once. `None`: none
    /// does.
    #[serde(default)]
    pub heavy: Option<i32>,
    /// Broken, the body staggers this long with its guards down (s).
    pub stagger_s: f32,
}
