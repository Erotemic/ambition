//! An archetype can say GROUNDED, distinctly from saying nothing.
//!
//! So did a deliberate `false`.
//!
//!  the three-state now lives on `CharacterLocomotion::flies`, which is where the fact belongs,
//! and shipped content authors all three cases.

#[test]
fn shipped_characters_state_their_flight_answer_explicitly() {
    use ambition_platformer2d::character::CharacterDefinition;

    let authored = |id: &str| {
        ambition_content::character_catalog::authored_intrinsics(
            id,
            CharacterDefinition::new(id, id, "ambition_content"),
        )
        .locomotion
        .and_then(|locomotion| locomotion.baseline_free_flight)
    };

    // a character that says it FLIES. Without one, `Some(true)` is
    // unreachable from shipped content and the type is decorative.
    assert_eq!(
        authored("stochastic_parrot"),
        Some(true),
        "the parrot flies and must say so itself — its catalog row's \
         `body_kind: Floating` stopped deciding locomotion (D89)"
    );

    // a character that says it does NOT, which is the case that could not
    // be expressed at all before: the PCA's row still says `Floating`, and that
    // is now a claim about its SILHOUETTE only.
    assert_eq!(
        authored("perfect_cellular_automaton"),
        Some(false),
        "the PCA is a grounded-base hybrid and its own definition has to say so, \
         or a presentation enum decides its locomotion again"
    );

    // and SILENCE, distinct from both. A character that never mentions
    // flight leaves the question open at the source layer; preparation resolves
    // it once, so nothing downstream has to re-ask.
    assert_eq!(
        authored("npc_exploding_mite"),
        None,
        "a character that authors no flight answer must read as SILENT here — if \
         this becomes `Some(false)`, `None` is unreachable and the three-state is \
         a two-state wearing three names"
    );
}

// It swept every archetype row's `is_aerial: Option<bool>` and asserted that silence still
// resolved to grounded — that making the question expressible had changed no behaviour.

/// THE PULSE SURVIVED ITS ARCHETYPE ROW.
///
/// the numbers are the row's verbatim — a migration that retuned on the way
/// would be a retune wearing a migration's commit.
#[test]
fn the_cellular_pulse_survived_its_archetype_row() {
    let moveset = ambition_content::cellular_automaton_moveset::cellular_pulse_moveset();
    assert_eq!(
        moveset.verbs.get("special").map(String::as_str),
        Some("cellular_pulse"),
        "the `special` verb must still resolve the pulse, or the PCA presses a \
         button and nothing happens"
    );
    let pulse = moveset
        .moves
        .iter()
        .find(|m| m.id == "cellular_pulse")
        .expect("the verb names a move that exists");
    let active = pulse
        .windows
        .iter()
        .find(|w| {
            matches!(
                w.tag,
                ambition_platformer2d::entity_catalog::WindowTag::Active
            )
        })
        .expect("a move with no ACTIVE window is a telegraph that never lands");
    assert!(
        !active.volumes.is_empty(),
        "the active window carries no hit volume, so the pulse cannot damage \
         anybody through the shared moveset runtime"
    );
    assert_eq!((active.start_s, active.end_s), (0.40, 0.54));
}
