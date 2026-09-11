//! The move family's own artifact section — fast-iteration packet I2, step 1/3.
//!
//! ⭐⭐ **THE CODEC LIVES BESIDE THE VALUE IT ENCODES, NOT BESIDE THE ENVELOPE.**
//! `ambition_content_pack`'s envelope treats a payload as opaque text, so adding
//! a content family is adding a section kind rather than editing the envelope.
//! The consequence is the important half: this module needs NO dependency on the
//! pack crate, and the pack crate needs none on this one. They meet at a
//! `(kind, version, payload)` triple that neither owns.
//!
//! ⭐ **AND THE CODEC ALREADY EXISTED — THAT WAS THE FINDING.** I2's step 1 asks
//! for a "canonical numeric/key encoding". MEASURED 2026-09-11 before writing
//! any of it: a `MoveSpec` built by the authoring helpers round-trips through
//! `ron` losslessly today (2,072 bytes for a chargeable smash with a technique
//! reference), because every type on the timeline already derives
//! `Serialize + Deserialize` for the authored RON catalog road. What was missing
//! was never the encoding; it was a VERSIONED, REFUSABLE envelope around it.
//!
//! ⚠ **A LOSSLESS ROUND TRIP IS NOT AN ADMISSIBLE MOVE.** This module carries
//! values; whether the host can honour the techniques they reference is the
//! host's admission question and is deliberately not asked here.

use crate::MovesetContract;

/// What a move section carries: every character's move table, by character id.
///
/// ⛔⛤ **IT IS THE CONTRACT, NOT `Vec<MoveSpec>`, AND I HAD IT WRONG FIRST.**
/// A `MovesetContract` is `(verbs, moves)`, and the verbs are the half that
/// decides WHICH move a press plays. A section carrying only the move list would
/// round-trip losslessly, pass every arm below, and deliver a fighter whose
/// buttons are unbound — "the table survived" being false in exactly the half
/// that matters. Reading a shipped table is what found it.
///
/// ⭐ `BTreeMap` FOR A CANONICAL ENCODING, which step 1 asks for by name: a
/// `HashMap` serializes in an arbitrary order, so two byte-identical packs would
/// produce different files and nothing downstream could compare them.
pub type MoveSectionData = std::collections::BTreeMap<String, MovesetContract>;

/// The section kind a move table travels under.
///
/// ⛔ A LOGICAL NAME, not a Rust path: renaming `MoveSpec` must not invalidate
/// every artifact on disk.
pub const MOVE_SECTION_KIND: &str = "moves";

/// This module's payload encoding version.
///
/// ⛔ SEPARATE FROM THE ENVELOPE'S. Bump it when the payload's SHAPE changes —
/// a new required field, a renamed variant — not when the envelope changes and
/// not when a move's values change. A host refuses a section newer than the
/// version it understands, which is the whole point of the number.
pub const MOVE_SECTION_VERSION: u32 = 1;

/// Encode a move section as a payload.
pub fn encode(data: &MoveSectionData) -> Result<String, ron::Error> {
    ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
}

/// Decode a section payload back into a move table.
///
/// ⚠ The CALLER checks the section version first. Handing an unversioned
/// payload straight to this function is the silent-misread the envelope's two
/// version numbers exist to prevent.
pub fn decode(payload: &str) -> Result<MoveSectionData, ron::error::SpannedError> {
    ron::from_str(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authoring::{charge, on_hit, strike, Charge, Strike};

    fn a_real_move() -> crate::MoveSpec {
        let m = strike(Strike {
            id: "section_smash",
            clip: "smash_forward",
            startup_s: 0.2,
            active_s: 0.08,
            recover_s: 0.3,
            offset: (18.0, 0.0),
            half_extents: (20.0, 14.0),
            damage: 14,
            knockback: 120.0,
            knockback_growth: 0.8,
            launch_dir: None,
            on_hit: None,
        });
        let m = on_hit(m, "pogo_bounce");
        charge(
            m,
            Charge {
                hold_at_s: 0.1,
                max_hold_s: 1.0,
                stores: false,
                roots: true,
                sustain: crate::ChargeSustain::WhileHeld,
                gesture: crate::ChargeGesture::Smash,
                multiplier: 1.4,
            },
        )
    }

    fn a_real_section() -> MoveSectionData {
        let mut verbs = std::collections::BTreeMap::new();
        verbs.insert("attack_forward".to_string(), "section_smash".to_string());
        let mut out = MoveSectionData::new();
        out.insert(
            "section_fighter".to_string(),
            MovesetContract {
                verbs,
                moves: vec![a_real_move()],
            },
        );
        out
    }

    /// ⛔ THE WHOLE SECTION, NOT A FIELD SAMPLE. Comparing a handful of fields
    /// would pass over a variant the codec silently dropped, and "the timing
    /// survived" is exactly the claim a partial comparison makes falsely.
    #[test]
    fn a_move_section_round_trips_through_its_payload() {
        let data = a_real_section();
        let back = decode(&encode(&data).expect("encodes")).expect("decodes");
        assert_eq!(data, back);
    }

    /// ⛔⛔ AND THE VERB BINDING SURVIVES, asserted separately because it is the
    /// half a `Vec<MoveSpec>` section would have dropped in silence.
    #[test]
    fn the_verbs_survive_and_not_just_the_moves() {
        let back = decode(&encode(&a_real_section()).expect("encodes")).expect("decodes");
        assert_eq!(
            back["section_fighter"].verbs.get("attack_forward").map(String::as_str),
            Some("section_smash"),
            "the move list came back and the press that plays it did not"
        );
    }

    /// ⭐ THE ANTI-VACUITY ARM. An encoder that emitted an empty map and a
    /// decoder that returned one would pass the round trip above with nothing
    /// in it.
    #[test]
    fn the_payload_actually_carries_the_move() {
        let payload = encode(&a_real_section()).expect("encodes");
        assert!(
            payload.contains("section_smash") && payload.contains("pogo_bounce"),
            "the payload names neither the move nor its technique, so it is not \
             carrying what the round trip claims: {payload}"
        );
        assert_eq!(decode(&payload).expect("decodes").len(), 1);
    }

    /// ⛔ AND A CHANGED TIMING CHANGES THE PAYLOAD. Without this, a codec that
    /// wrote a constant would satisfy every arm above.
    #[test]
    fn editing_a_move_changes_what_the_section_carries() {
        let before = encode(&a_real_section()).expect("encodes");
        let mut edited = a_real_section();
        edited.get_mut("section_fighter").unwrap().moves[0].duration_s += 0.5;
        let after = encode(&edited).expect("encodes");
        assert_ne!(before, after, "the payload is not a function of the moves");
        assert_eq!(decode(&after).expect("decodes"), edited);
    }
}
