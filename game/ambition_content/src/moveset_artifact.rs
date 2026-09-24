//! Can a loadable artifact carry the shipped roster's move tables exactly?
//!
//! This is I2's precondition, and the only question a codec can answer alone.
//! Before the compiled move table stops being the host's authoritative input,
//! the artifact must carry everything the compiled table says (every
//! character, verb binding, window and volume) with nothing normalized away.
//!
//! The subject is `authored_movesets::tables()`, the whole shipped set, not
//! one character: one fighter exercises only what its author used, and the
//! roster grows when someone adds a move with no codec arm.
//!
//! A round trip is not admission. Whether the host can honour the techniques
//! these tables reference is a separate question, asked by the host.

#![cfg(test)]

use ambition_content_pack::artifact::{ArtifactSection, ContentArtifact};
use ambition_entity_catalog::move_section::{
    decode, encode, MoveSectionData, MOVE_SECTION_KIND, MOVE_SECTION_VERSION,
};

/// Every shipped table, in the section's canonical shape.
fn shipped_section() -> MoveSectionData {
    crate::authored_movesets::tables()
        .into_iter()
        .map(|(id, contract)| (id.to_string(), contract))
        .collect()
}

/// One artifact carrying the shipped roster's moves.
fn shipped_artifact() -> ContentArtifact {
    ContentArtifact::new(vec![ArtifactSection {
        kind: MOVE_SECTION_KIND.to_string(),
        section_version: MOVE_SECTION_VERSION,
        payload: encode(&shipped_section()).expect("the shipped roster encodes"),
    }])
}

/// The whole roster survives, compared structurally.
///
/// The floor is first. An empty `tables()` (a provider list no longer
/// registered, a feature that gated the roster out) would make every
/// comparison below trivially true.
#[test]
fn every_shipped_move_table_survives_the_artifact_exactly() {
    let before = shipped_section();
    assert!(
        before.len() >= 10,
        "{} shipped move table(s): the roster this compares is not the roster, \
         so the equality below would certify an empty map",
        before.len()
    );
    let total_moves: usize = before.values().map(|c| c.moves.len()).sum();
    assert!(
        total_moves >= 100,
        "{total_moves} moves across {} tables — too few for this to be the \
         shipped set",
        before.len()
    );

    let artifact = shipped_artifact();
    let text = artifact.to_ron().expect("the artifact serializes");
    eprintln!(
        "ARTIFACT: {} tables, {total_moves} moves, {} bytes",
        before.len(),
        text.len()
    );
    let parsed = ContentArtifact::parse(&text).expect("and parses back");
    assert_eq!(
        parsed.admit(&|kind| (kind == MOVE_SECTION_KIND).then_some(MOVE_SECTION_VERSION)),
        vec![],
        "the artifact this repository writes is not one it would admit"
    );

    let section = parsed
        .section(MOVE_SECTION_KIND)
        .expect("the move section is there");
    let after = decode(&section.payload).expect("the payload decodes");

    // Per character, so a failure names who instead of printing the roster.
    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "the artifact lost or invented a character"
    );
    for (id, contract) in &before {
        assert_eq!(
            &after[id], contract,
            "`{id}`'s move table did not survive the artifact intact"
        );
    }
}

/// This makes the test above meaningful. A codec that returned its input
/// unchanged, or a comparison of a value with itself, passes a round trip.
/// This edits a real shipped move and asserts the artifact carries the edit.
#[test]
fn editing_a_shipped_move_changes_what_the_artifact_says() {
    let mut edited = shipped_section();
    let (who, contract) = edited
        .iter_mut()
        .find(|(_, c)| !c.moves.is_empty())
        .expect("some shipped table has a move in it");
    let who = who.clone();
    let original = contract.moves[0].duration_s;
    contract.moves[0].duration_s = original + 0.25;

    let payload = encode(&edited).expect("encodes");
    let back = decode(&payload).expect("decodes");
    assert_eq!(
        back[&who].moves[0].duration_s,
        original + 0.25,
        "a changed move timing did not reach the other side of the artifact"
    );
    assert_ne!(
        back[&who].moves[0].duration_s, original,
        "the artifact handed back the compiled value, not the edited one"
    );
}
