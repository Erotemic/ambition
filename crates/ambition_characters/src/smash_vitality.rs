//! A move that changes its own mover's health: the authored vocabulary.
//!
//! ⭐ THE SAME SPLIT `smash_teleport`, `smash_capture` AND `smash_ride` USE, and
//! for the same reason. A key and its params are what a MOVESET authors, so they
//! live where movesets can name them; applying the change to a live body is the
//! ENGINE's job, and that half sits in `ambition_combat` beside the damage it is
//! the mirror of.
//!
//! ⭐⭐ JON'S DESIGN, 2026-08-26: *"the medic could have a self healing move."*
//! Her sprite library had been carrying the other half of the answer since it
//! was forked: `charge.clip.json` is captioned *"Down special — FIELD DRESSING.
//! She goes to one knee, puts both hands on her own ribs and holds pressure. It
//! gives back what she spent"*, and `special.clip.json` is ADRENALINE, *"she
//! spends her own margin to buy tempo"*. One technique answers both, because
//! paying and repaying are one operation with a sign.
//!
//! ⛔⛔ AND NOTHING IN THE ENGINE COULD DO EITHER. `BodyHealth::heal` existed
//! and no move could reach it; there was no way at all for a move to charge its
//! own owner, because `damage` is an INJURY — attributed, refused by
//! invulnerability, and able to report a kill. `BodyHealth::spend` is the
//! primitive this needed and is documented where it lives.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const VITALITY: &str = "smash.vitality";

/// What one authored change to the mover's own health costs or gives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VitalityParams {
    /// Signed change to the MOVER'S OWN health. Positive restores; negative is
    /// a price the move charges for itself.
    ///
    /// ⭐ ONE SIGNED FIELD AND NOT A `heal` PLUS A `cost`, because two fields
    /// that may never both be set is a rule the type cannot state and every
    /// reader has to remember — "exclusive in meaning, not in type" is the
    /// shape this repository has been bitten by before. A sign is exclusive by
    /// construction.
    ///
    /// ⛔ A RESTORE REPAYS THE METER TOO, and a price charges it. That is
    /// `BodyHealth`'s own rule, not this technique's: in a platform fighter the
    /// accumulated-damage meter is the currency that decides how far you launch,
    /// so a heal that refilled the pool and left the meter would be a heal you
    /// could not feel.
    pub change: i32,
    /// The lowest health a PRICE may leave the mover at. Ignored by a restore.
    ///
    /// ⛔⛔ A MOVE THAT CAN KILL YOU BY BEING PRESSED IS NOT A COST. `1` is the
    /// floor that means "never self-KO", and it is what every authored price
    /// should want; the field exists so a character can be more cautious than
    /// that, never less. The engine clamps it up to `1` regardless.
    #[serde(default)]
    pub floor: i32,
    /// The effect drawn on the mover when the change lands.
    pub vfx: String,
    /// The cue played when the change lands.
    pub sfx: String,
}

/// Author a health change onto a move's timeline.
///
/// # Panics
///
/// If `at_s` is past the move's own duration — a change scheduled after the move
/// ends never fires, and the move would spend its recovery to do nothing — or if
/// `change` is zero, which is a move that costs frames and means nothing.
pub fn author_vitality(mut spec: MoveSpec, at_s: f32, params: VitalityParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` changes health at {at_s}s but only lasts {}s, so the change \
         would never fire and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    assert!(
        params.change != 0,
        "move `{}` authors a health change of zero, which is a move that costs \
         frames and means nothing",
        spec.id,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: VITALITY.to_string(),
            params: ParamValue::from_typed(&params).expect("vitality params serialize"),
        }),
    });
    spec
}
