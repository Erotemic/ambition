//! Ambition's adaptive music, handed to the reusable music director.
//!
//! The cues and the encounters that bind to them are content:
//! `assets/audio/music_cues.ron` (schema `music_cue_catalog`, owned by
//! `ambition_audio`). The content compiler checks every reference in it and
//! requires each section's audio file. This module only builds the director's
//! catalog from what the pack lowered. Gated behind the `audio` feature.

use ambition_audio::music::MusicCueCatalog;

/// The director's catalog for the adaptive music `pack` lowered, or `None` for a
/// pack that carries no cue file (it compiles; the provider's preparation then
/// refuses a session that expects adaptive cues).
///
/// A composition passes its selected pack; a reload passes its candidate.
pub fn music_cue_catalog_from(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<MusicCueCatalog> {
    let authored = ambition_audio::content_schema::lowered_music_cues(pack).cloned()?;
    Some(MusicCueCatalog::from_parts(authored.cues, authored.encounter_bindings))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUES: &str = "audio/music_cues.ron";

    /// The pack with one edit to the cue file.
    fn compiled_with(
        from: &'static str,
        to: &'static str,
    ) -> Result<ambition_content_pack::PreparedContentPack, ambition_content_pack::CompileFailure>
    {
        crate::pack::compile_pack_with(move |path: &str, text: String| {
            if path != CUES {
                return text;
            }
            assert_eq!(text.matches(from).count(), 1, "the edit anchor {from:?}");
            text.replace(from, to)
        })
    }

    /// ⭐ A pack without its cue file COMPILES, and carries no catalog. Measured
    /// 2026-10-07 after the lowering's `expect` ("for every pack that compiles")
    /// claimed otherwise. So "does the provider have adaptive cues" can differ
    /// between generation N and N+1, and a reload's preparation reads the
    /// candidate's answer (`adaptive_cues_ready_for`), not the App's.
    #[test]
    fn a_pack_without_its_cue_file_compiles_and_carries_no_catalog() {
        let pack = crate::pack::compile_pack_omitting(&[CUES])
            .expect("a pack that stops declaring its cue file compiles");
        assert!(music_cue_catalog_from(&pack).is_none());
        assert!(music_cue_catalog_from(crate::pack::shipped()).is_some());
    }

    /// The goblin lab's binding is what the cue file says. An edit to the
    /// file changes the catalog the game registers, and a binding that names
    /// a state the cue does not have refuses the pack.
    #[test]
    fn the_adaptive_music_the_game_plays_is_authored_in_the_pack() {
        let binding = |catalog: &MusicCueCatalog| {
            let binding = catalog
                .encounter_bindings()
                .iter()
                .find(|binding| binding.encounter_id == "goblin_encounter")
                .expect("the goblin lab binds a cue")
                .clone();
            (binding.cue_id, binding.starting_state)
        };
        assert_eq!(
            binding(&music_cue_catalog_from(crate::pack::shipped()).expect("the shipped cues")),
            ("first_goblin_tune_v2".to_string(), "intro".to_string())
        );
        assert!(music_cue_catalog_from(crate::pack::shipped())
            .expect("the shipped cues")
            .validate_references()
            .is_empty());

        let edited = compiled_with(r#"starting_state: "intro""#, r#"starting_state: "wave1""#)
            .expect("a binding that starts on another state compiles");
        assert_eq!(
            binding(&music_cue_catalog_from(&edited).expect("the edited cues")),
            ("first_goblin_tune_v2".to_string(), "wave1".to_string())
        );

        let dangling = compiled_with(r#"cleared_state: "outro""#, r#"cleared_state: "missing""#);
        let failure = format!("{:?}", dangling.expect_err("a dangling state refuses the pack"));
        assert!(failure.contains("unknown state 'missing'"), "{failure}");
    }
}
