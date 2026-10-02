//! Intro cutscene scripts.
//!
//! Inserted into the shared [`ambition_cutscene::CutsceneLibrary`] by
//! [`crate::intro::IntroPlugin`] when it builds, so they are there before the
//! first tick. Which room starts which of them is said by each intro room's
//! `entry_cutscene` level field in `intro.ldtk`.
//!
//! Beats are intentionally short — the design doc is firm that the
//! intro should not become a long cutscene wall. Each room gets at
//! most a few banner/dialogue beats before control returns.

use ambition_cutscene::CutsceneLibrary;

/// Insert every intro cutscene script into the shared library. Idempotent
/// at the script-id level — re-running replaces existing scripts.
pub fn install_intro_cutscenes(library: &mut CutsceneLibrary) {
    library.insert(intro_wake_script());
    library.insert(intro_raid_script());
    library.insert(drain_market_arrival_script());
    library.insert(first_ripple_script());
    library.insert(creator_final_fragment_script());
}

fn intro_wake_script() -> ambition_cutscene::CutsceneScript {
    ambition_cutscene::CutsceneScript::new(
        "intro_wake",
        vec![
            // Comes up FROM BLACK. This used to be `to_alpha: 0.0` alone,
            // which said "end clear" and left where it started to a convention
            // nothing in the data carried (`Q143`).
            ambition_cutscene::CutsceneBeat::Fade {
                from_alpha: 1.0,
                to_alpha: 0.0,
                seconds: 0.8,
            },
            ambition_cutscene::CutsceneBeat::Banner {
                text: "// boot".into(),
                seconds: 1.0,
            },
            ambition_cutscene::CutsceneBeat::Dialogue {
                speaker: "Creator".into(),
                text: "Hey you, you're finally awake.".into(),
            },
            ambition_cutscene::CutsceneBeat::SetFlag {
                id: "intro_started".into(),
                on: true,
            },
            ambition_cutscene::CutsceneBeat::SetFlag {
                id: "intro_wake_seen".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("intro_wake_seen")
}

fn intro_raid_script() -> ambition_cutscene::CutsceneScript {
    ambition_cutscene::CutsceneScript::new(
        "intro_raid",
        vec![
            ambition_cutscene::CutsceneBeat::Banner {
                text: "// PERIMETER BREACH".into(),
                seconds: 1.2,
            },
            ambition_cutscene::CutsceneBeat::Dialogue {
                speaker: "Salvage Lead".into(),
                text: "Wrong room. Secure anything that boots.".into(),
            },
            ambition_cutscene::CutsceneBeat::Dialogue {
                speaker: "Lab Raider".into(),
                text: "Prototype is awake. Keep it away from the doors.".into(),
            },
            ambition_cutscene::CutsceneBeat::Dialogue {
                speaker: "Salvage Lead".into(),
                text: "This one isn't on the manifest. Tag it and move.".into(),
            },
            ambition_cutscene::CutsceneBeat::SetFlag {
                id: "intro_raid_started".into(),
                on: true,
            },
            ambition_cutscene::CutsceneBeat::SetFlag {
                id: "wrong_list_clue_1".into(),
                on: true,
            },
            ambition_cutscene::CutsceneBeat::SetFlag {
                id: "intro_raid_seen".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("intro_raid_seen")
}

fn drain_market_arrival_script() -> ambition_cutscene::CutsceneScript {
    ambition_cutscene::CutsceneScript::new(
        "drain_market_arrival",
        vec![
            // Comes up FROM BLACK. This used to be `to_alpha: 0.0` alone,
            // which said "end clear" and left where it started to a convention
            // nothing in the data carried (`Q143`).
            ambition_cutscene::CutsceneBeat::Fade {
                from_alpha: 1.0,
                to_alpha: 0.0,
                seconds: 0.6,
            },
            ambition_cutscene::CutsceneBeat::Banner {
                text: "Drain Market — STAFF ONLY".into(),
                seconds: 1.2,
            },
            ambition_cutscene::CutsceneBeat::Dialogue {
                speaker: "Oiler".into(),
                text: "Well. That's not a rat. You came out of the bad pipe.".into(),
            },
            ambition_cutscene::CutsceneBeat::SetFlag {
                id: "drain_market_arrival_seen".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("drain_market_arrival_seen")
}

fn first_ripple_script() -> ambition_cutscene::CutsceneScript {
    // Not bound to a room (the player triggers it by interacting with
    // the ripple). Wiring the interaction lives in v1.1; the script is
    // here so the trigger can be `request`-ed once that lands.
    ambition_cutscene::CutsceneScript::new(
        "first_ripple",
        vec![
            ambition_cutscene::CutsceneBeat::Dialogue {
                speaker: "Gate Janitor".into(),
                text: "Don't touch that. That's not a gate.".into(),
            },
            ambition_cutscene::CutsceneBeat::Wait { seconds: 0.4 },
            ambition_cutscene::CutsceneBeat::Dialogue {
                speaker: "Gate Janitor".into(),
                text: "Okay. That's a problem.".into(),
            },
            ambition_cutscene::CutsceneBeat::SetFlag {
                id: "first_ripple_used".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("first_ripple_seen")
}

fn creator_final_fragment_script() -> ambition_cutscene::CutsceneScript {
    // Played on the creator's interrupted final lines. v1 plays the
    // normal route; the fast/impossible variants are listed in the
    // design doc for a v1.x pass.
    ambition_cutscene::CutsceneScript::new(
        "creator_final_fragment",
        vec![
            ambition_cutscene::CutsceneBeat::Dialogue {
                speaker: "Creator".into(),
                text: "There's a question you were made to—".into(),
            },
            ambition_cutscene::CutsceneBeat::SetFlag {
                id: "intro_creator_dead".into(),
                on: true,
            },
            ambition_cutscene::CutsceneBeat::SetFlag {
                id: "intro_creator_fragment_normal".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("intro_creator_fragment_seen")
}
