//! Default Ambition cutscene library + room→cutscene bindings.
//!
//! These are authored Ambition *content* — named scripts (`test_intro`,
//! `boss_intro_gradient_sentinel`, `cutscene_lab_intro`) and the room ids they
//! play in. They were previously hosted by `ambition_render::cutscene`; they now
//! live with the rest of the named dialogue content. The reusable runtime types
//! ([`CutsceneLibrary`], [`RoomCutsceneBindings`]) live in `ambition_cutscene`;
//! the playback systems live in `ambition_actors::cutscene`. The intro
//! slice layers more scripts/bindings on top via `crate::intro`.

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
                CutsceneBeat::Fade {
                    to_alpha: 0.0,
                    seconds: 0.8,
                },
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
            ("central_hub_main".into(), "test_intro".into()),
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
        // Hub plays the test_intro cutscene on first entry.
        assert!(bindings
            .bindings
            .iter()
            .any(|(room, cs)| room == "central_hub_main" && cs == "test_intro"));
    }
}
