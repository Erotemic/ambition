//! Can a loadable artifact carry the SHIPPED roster's move tables, exactly?
//!
//! ⭐⭐ **THIS IS I2'S PRECONDITION, AND IT IS THE ONLY QUESTION A CODEC CAN
//! ANSWER ON ITS OWN.** Steps 4 and 5 remove the compiled move table as the
//! host's authoritative input; before that is worth attempting, the artifact has
//! to be shown to carry what the compiled table currently says — every
//! character, every verb binding, every window, every volume — with nothing
//! normalized away.
//!
//! ⛔⛔ **THE SUBJECT IS `authored_movesets::tables()`, THE WHOLE SHIPPED SET,
//! NOT ONE CHARACTER.** A single fighter exercises whatever that fighter's author
//! happened to use. The roster is what actually has to survive, and it is the
//! population that grows when somebody adds a move nobody wrote a codec arm for.
//!
//! ⚠ **AND A ROUND TRIP IS NOT ADMISSION.** Whether the host can honour the
//! techniques these tables reference is a different question, asked by the host.

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

/// ⛔ THE WHOLE ROSTER SURVIVES, COMPARED STRUCTURALLY.
///
/// ⚠ THE FLOOR IS FIRST. An empty `tables()` — a provider list that stopped
/// being registered, a feature that gated the roster out — makes every
/// comparison below trivially true over nothing, which is this repository's most
/// repeated instrument failure.
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

    // ⛔ PER CHARACTER, so a failure names WHO rather than printing the roster.
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

/// ⭐ THE ARM THAT MAKES THE ONE ABOVE MEAN SOMETHING. A codec that returned its
/// input unchanged — or a comparison that compared a value with itself — passes
/// a round trip. This one edits a real shipped move and asserts the artifact
/// carries the EDIT, which is the whole promise of a loadable move.
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
