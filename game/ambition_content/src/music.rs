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
    Some(
        MusicCueCatalog::from_parts(authored.cues, authored.encounter_bindings)
            .with_room_bindings(authored.room_bindings)
            .with_boss_bindings(authored.boss_bindings)
            .with_room_phase_tracks(authored.room_phase_tracks),
    )
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
    /// A room's track by the time of day reaches the catalog the intent reads,
    /// and a phase the clock does not have is refused when the pack compiles.
    #[test]
    fn a_rooms_track_by_the_time_of_day_is_read_and_its_phase_checked() {
        let end = "fight_state: Some(\"fight\")),\n    ],\n)";
        let row = |phase: &str| {
            format!(
                "fight_state: Some(\"fight\")),\n    ],\n    room_phase_tracks: [(room_id: \"math_town_street\", phase: \"{phase}\", track: \"shoulders_at_rest\")],\n)"
            )
        };
        let night: &'static str = Box::leak(row("night").into_boxed_str());
        let pack = compiled_with(end, night).expect("a row with a phase of the day compiles");
        let catalog = music_cue_catalog_from(&pack).expect("the pack carries its catalog");
        assert_eq!(catalog.room_phase_track("math_town_street", "night"), Some("shoulders_at_rest"));
        assert_eq!(catalog.room_phase_track("math_town_street", "day"), None);
        let teatime: &'static str = Box::leak(row("teatime").into_boxed_str());
        assert!(compiled_with(end, teatime).is_err(), "no phase of the day is called teatime");
    }

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

    /// The rooms of the primeval trail, in the order of the level: the levels
    /// of the world whose name starts `primeval_`, by where they are.
    fn rooms_of_the_trail() -> Vec<String> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/worlds/sandbox.ldtk");
        let world: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("the world file")).expect("the world is JSON");
        let mut rooms: Vec<(i64, String)> = world["levels"]
            .as_array()
            .expect("the world has levels")
            .iter()
            .filter_map(|level| {
                let id = level["identifier"].as_str()?;
                id.starts_with("primeval_").then(|| (level["worldX"].as_i64().unwrap_or(0), id.to_string()))
            })
            .collect();
        rooms.sort();
        rooms.into_iter().map(|(_, id)| id).collect()
    }

    /// The music of the primeval trail is never cut (Jon, 2026-10-12: "The
    /// user should never hear a discontinuity in the music. New instruments
    /// should layer onto the loop making each section progressively rock
    /// harder").
    ///
    /// A room of the trail with no binding plays its `music_track` from the
    /// top: that was the cut, in the two rooms the trail got on 2026-10-11.
    /// And a state on another section starts that section from its top. So:
    /// each room binds the cue, each state a room can ask for is on the one
    /// loop, and each room has every layer of the room before it, as loud or
    /// louder, and more.
    #[test]
    fn each_room_of_the_trail_plays_the_one_loop_and_a_layer_more() {
        let authored = ambition_audio::content_schema::lowered_music_cues(crate::pack::shipped()).expect("the shipped cues");
        let rooms = rooms_of_the_trail();
        assert!(rooms.len() >= 6, "the trail is six rooms or more: {rooms:?}");

        let mut cue_id: Option<&str> = None;
        let mut before: Option<(&str, &ambition_audio::music::MusicStateSpec)> = None;
        for room in &rooms {
            let binding = authored
                .room_bindings
                .iter()
                .find(|binding| &binding.room_id == room)
                .unwrap_or_else(|| panic!("`{room}` binds no cue: its music_track would start from the top there"));
            assert_eq!(*cue_id.get_or_insert(binding.cue_id.as_str()), binding.cue_id, "`{room}` binds another cue: a change of cue is a cut");
            let cue = authored.cues.iter().find(|cue| cue.id == binding.cue_id).expect("the cue of the binding");
            let state_of = |id: &str| cue.states.iter().find(|state| state.id == id).unwrap_or_else(|| panic!("no state `{id}`"));
            let looped = |state: &ambition_audio::music::MusicStateSpec| {
                cue.sections.iter().find(|section| section.id == state.section_id).is_some_and(|section| section.looped)
            };
            let state = state_of(&binding.state);
            assert!(looped(state), "`{room}` plays `{}`, which is not on a loop", state.id);
            for fight in binding.fight_state.iter().map(|id| state_of(id)) {
                assert_eq!(fight.section_id, state.section_id, "the fight of `{room}` is on another section: its start would cut the song");
            }
            if let Some((room_before, state_before)) = before {
                assert_eq!(state.section_id, state_before.section_id, "`{room}` and `{room_before}` are on two sections: the door between them would cut the song");
                let gain = |state: &ambition_audio::music::MusicStateSpec, layer: &str| {
                    state.gains.iter().find(|gain| gain.layer_id == layer).map_or(0.0, |gain| gain.gain)
                };
                for kept in &state_before.gains {
                    assert!(gain(state, &kept.layer_id) >= kept.gain, "`{room}` has less of `{}` than `{room_before}`: it does not rock harder", kept.layer_id);
                }
            }
            before = Some((room.as_str(), state));
        }
        // The level adds something on the way: its last room has more layers
        // than its first.
        let layers = |room: &String| {
            let binding = authored.room_bindings.iter().find(|binding| &binding.room_id == room).expect("checked above");
            let cue = authored.cues.iter().find(|cue| cue.id == binding.cue_id).expect("checked above");
            cue.states.iter().find(|state| state.id == binding.state).map_or(0, |state| state.gains.iter().filter(|gain| gain.gain > 0.0).count())
        };
        assert!(layers(rooms.last().expect("rooms")) > layers(&rooms[0]), "the last room of the trail has no more layers than the first");
    }
}
