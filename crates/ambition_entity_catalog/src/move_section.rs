//! The move family's own artifact section.
//!
//! The codec lives beside the value it encodes, not beside the envelope.
//! `ambition_content_pack`'s envelope treats a payload as opaque text, so a new
//! content family is a new section kind. Neither this module nor the pack
//! crate depends on the other; they meet at a `(kind, version, payload)`
//! triple.
//!
//! The encoding is the existing `ron` serde road: every type on the timeline
//! derives `Serialize + Deserialize`, so a `MoveSpec` round-trips losslessly.
//! This module adds the versioned, refusable envelope.
//!
//! A lossless round trip is not an admissible move, and this module must not
//! decide admission. That is answered by
//! `ambition_characters::prepared::unsupported_authored_effects`, which walks
//! `MoveSpec::effect_refs`, calls `TechniqueSupport::admit_at` with the site,
//! and checks nested references. The artifact road admits by hydrating into
//! the prepared registry and running that pass, so there is one validator.
//!
//! That pass needs a `PreparedCharacterRegistry` in a Bevy-linked crate, so an
//! outside builder cannot run it. This is by design: the builder emits and the
//! host admits, so content can be built without compiling the engine.

use crate::MovesetContract;

/// What a move section carries: every character's move table, by character id.
///
/// The whole contract, not only `Vec<MoveSpec>`: the verbs decide which move a
/// press plays, and without them a fighter's buttons would be unbound.
///
/// `BTreeMap` for a canonical encoding: a `HashMap` serializes in arbitrary
/// order, so identical packs would produce different files.
pub type MoveSectionData = std::collections::BTreeMap<String, MovesetContract>;

/// The section kind a move table travels under.
///
/// A logical name, not a Rust path: renaming `MoveSpec` must not invalidate
/// artifacts on disk.
pub const MOVE_SECTION_KIND: &str = "moves";

/// This module's payload encoding version.
///
/// Separate from the envelope's version. Bump it when the payload's shape
/// changes (a new required field, a renamed variant), not when the envelope or
/// a move's values change. A host refuses a section newer than it
/// understands.
pub const MOVE_SECTION_VERSION: u32 = 1;

/// Encode a move section as a payload.
pub fn encode(data: &MoveSectionData) -> Result<String, ron::Error> {
    ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
}

/// Decode a section payload back into a move table.
///
/// The caller checks the section version first. Decoding an unversioned
/// payload directly risks a silent misread.
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

    /// Compare the whole section, not a sample of fields: a sample would miss
    /// a variant the codec silently dropped.
    #[test]
    fn a_move_section_round_trips_through_its_payload() {
        let data = a_real_section();
        let back = decode(&encode(&data).expect("encodes")).expect("decodes");
        assert_eq!(data, back);
    }

    /// The verb binding survives. Asserted separately because a
    /// `Vec<MoveSpec>` section would drop it silently.
    #[test]
    fn the_verbs_survive_and_not_just_the_moves() {
        let back = decode(&encode(&a_real_section()).expect("encodes")).expect("decodes");
        assert_eq!(
            back["section_fighter"].verbs.get("attack_forward").map(String::as_str),
            Some("section_smash"),
            "the move list came back and the press that plays it did not"
        );
    }

    /// Positive control: an encoder that emitted an empty map and a decoder
    /// that returned one would pass the round trip above.
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

    /// A changed timing changes the payload. Without this, a codec that
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
