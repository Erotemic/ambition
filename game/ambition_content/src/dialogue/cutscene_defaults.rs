//! Default Ambition cutscene library + room→cutscene bindings.

use ambition_cutscene::{CutsceneBeat, CutsceneLibrary, CutsceneScript, RoomCutsceneBindings};

/// Default sandbox cutscenes shipped with the sandbox.
pub fn default_cutscene_library() -> CutsceneLibrary {
    let mut lib = CutsceneLibrary::default();
    lib.insert(
        CutsceneScript::new(
            "test_intro",
            vec![
                CutsceneBeat::Banner {
                    text: "// boot sequence".into(),
                    seconds: 1.4,
                },
                // ⛔ THE FADE THAT USED TO SIT HERE IS GONE (`Q143`). It was
                // the SECOND beat, after a visible banner, and it targeted
                // clear — so once a fade actually draws, the only honest start
                // alpha for it is the clear screen it already had, which makes
                // it 0.8 s of nothing. The other two shipped fades open their
                // scripts and mean "up from black"; this one never meant
                // anything, and the ruling says these are not worth preserving
                // effort. Deleted rather than given an invented intent.
                CutsceneBeat::Dialogue {
                    speaker: "WARDEN".into(),
                    text: "Instance online. You'll know your purpose when you find it.".into(),
                },
                CutsceneBeat::SetFlag {
                    id: "test_intro_seen".into(),
                    on: true,
                },
            ],
        )
        .with_seen_flag("test_intro_seen"),
    );
    lib.insert(
        CutsceneScript::new(
            "cutscene_lab_intro",
            vec![
                CutsceneBeat::Banner {
                    text: "// cutscene proof".into(),
                    seconds: 1.0,
                },
                CutsceneBeat::Dialogue {
                    speaker: "WARDEN".into(),
                    text: "This is the cutscene-proof room. The seen-flag stops me from talking twice."
                        .into(),
                },
                CutsceneBeat::Wait { seconds: 0.4 },
                CutsceneBeat::Dialogue {
                    speaker: "WARDEN".into(),
                    text: "Hold Reset to skip cutscenes -- useful when you've heard a beat already."
                        .into(),
                },
                CutsceneBeat::SetFlag {
                    id: "cutscene_lab_intro_seen".into(),
                    on: true,
                },
            ],
        )
        .with_seen_flag("cutscene_lab_intro_seen"),
    );
    lib.insert(
        CutsceneScript::new(
            "boss_intro_gradient_sentinel",
            vec![
                CutsceneBeat::Banner {
                    text: "GRADIENT SENTINEL".into(),
                    seconds: 1.6,
                },
                CutsceneBeat::Wait { seconds: 0.4 },
                CutsceneBeat::Dialogue {
                    speaker: "SENTINEL".into(),
                    text: "Your loss surface is steep. I am its slope.".into(),
                },
            ],
        )
        .with_seen_flag("boss_intro_gradient_sentinel_seen"),
    );
    lib
}

/// Default room → cutscene bindings: which cutscene plays the first time the
/// player enters a given room (the `seen_flag` guards replays).
pub fn default_room_cutscene_bindings() -> RoomCutsceneBindings {
    RoomCutsceneBindings {
        bindings: vec![
            // Plays the first time the player enters the hub.
            //
            // ⛔ `central_hub_main` (an LDtk LEVEL id) was named here until
            // 2026-09-18: `auto_trigger_room_cutscenes` compares against the
            // RUNTIME room id, `central_hub_main` and `central_hub_basement`
            // both merge into `central_hub_complex` at LDtk conversion, and
            // the row could never match — see `resources.rs`'s panic message
            // for the same trap. Caught by inspection, not by a test; the
            // guard for this is `room_cutscene_bindings_resolve.rs`.
            ("central_hub_complex".into(), "test_intro".into()),
            // Plays the first time the player enters the (existing)
            // basement boss arena. The `seen_flag` guards against replays.
            (
                "basement_boss".into(),
                "boss_intro_gradient_sentinel".into(),
            ),
            // Cutscene proof room reachable from the basement.
            // Demonstrates the entry-trigger + seen-flag + skip flow on a
            // non-default cutscene.
            ("cutscene_lab".into(), "cutscene_lab_intro".into()),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cutscene_library_includes_test_intro() {
        let lib = default_cutscene_library();
        assert!(lib.get("test_intro").is_some());
    }

    #[test]
    fn default_cutscene_library_includes_boss_intro() {
        let lib = default_cutscene_library();
        assert!(lib.get("boss_intro_gradient_sentinel").is_some());
    }

    #[test]
    fn default_room_cutscene_bindings_link_hub_to_test_intro() {
        let bindings = default_room_cutscene_bindings();
        // Hub plays the test_intro cutscene on first entry. `central_hub_complex`
        // is the RUNTIME room id `auto_trigger_room_cutscenes` compares against
        // -- not `central_hub_main`, which is only the LDtk level identifier.
        assert!(bindings
            .bindings
            .iter()
            .any(|(room, cs)| room == "central_hub_complex" && cs == "test_intro"));
    }
}
