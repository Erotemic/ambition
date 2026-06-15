//! Intro cutscene scripts + room→cutscene bindings.
//!
//! Inserted into the shared [`ambition_render::cutscene::CutsceneLibrary`] and
//! [`ambition_render::cutscene::RoomCutsceneBindings`] by
//! [`crate::intro::plugin::install_intro_cutscenes_system`] at startup,
//! so the sandbox cutscene/system runtime picks them up the moment the
//! player enters the matching room.
//!
//! Beats are intentionally short — the design doc is firm that the
//! intro should not become a long cutscene wall. Each room gets at
//! most a few banner/dialogue beats before control returns.

use ambition_render::cutscene::CutsceneLibrary;

/// Room → cutscene bindings for the intro slice. Mirrors the shape of
/// [`ambition_render::cutscene::RoomCutsceneBindings::defaults`] — `(room_id,
/// cutscene_id)` pairs walked once per room change in
/// `auto_trigger_room_cutscenes`.
pub const INTRO_ROOM_CUTSCENE_BINDINGS: &[(&str, &str)] = &[
    ("intro_wake_room", "intro_wake"),
    ("intro_raid_corridor", "intro_raid"),
    ("drain_alley", "drain_market_arrival"),
    // Removed 2026-05-22: the gate-stack reveal cutscene interrupted
    // the player on every entry without earning the pause. The PA
    // "Gate Six delayed" line lives in the dialogue layer; the
    // banner reveal isn't needed when the room itself reads as a
    // gate stack on sight. If we want a one-time arrival beat later
    // it should be a quieter banner with no dialogue.
];

pub fn intro_room_cutscene_bindings() -> &'static [(&'static str, &'static str)] {
    INTRO_ROOM_CUTSCENE_BINDINGS
}

/// Insert every intro cutscene script into the shared library. Idempotent
/// at the script-id level — re-running replaces existing scripts.
pub fn install_intro_cutscenes(library: &mut CutsceneLibrary) {
    library.insert(intro_wake_script());
    library.insert(intro_raid_script());
    library.insert(drain_market_arrival_script());
    library.insert(first_ripple_script());
    library.insert(creator_final_fragment_script());
}

fn intro_wake_script() -> ambition_render::cutscene::script::CutsceneScript {
    ambition_render::cutscene::script::CutsceneScript::new(
        "intro_wake",
        vec![
            ambition_render::cutscene::script::CutsceneBeat::Fade {
                to_alpha: 0.0,
                seconds: 0.8,
            },
            ambition_render::cutscene::script::CutsceneBeat::Banner {
                text: "// boot".into(),
                seconds: 1.0,
            },
            ambition_render::cutscene::script::CutsceneBeat::Dialogue {
                speaker: "Creator".into(),
                text: "Hey you, you're finally awake.".into(),
            },
            ambition_render::cutscene::script::CutsceneBeat::SetFlag {
                id: "intro_started".into(),
                on: true,
            },
            ambition_render::cutscene::script::CutsceneBeat::SetFlag {
                id: "intro_wake_seen".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("intro_wake_seen")
}

fn intro_raid_script() -> ambition_render::cutscene::script::CutsceneScript {
    ambition_render::cutscene::script::CutsceneScript::new(
        "intro_raid",
        vec![
            ambition_render::cutscene::script::CutsceneBeat::Banner {
                text: "// PERIMETER BREACH".into(),
                seconds: 1.2,
            },
            ambition_render::cutscene::script::CutsceneBeat::Dialogue {
                speaker: "Salvage Lead".into(),
                text: "Wrong room. Secure anything that boots.".into(),
            },
            ambition_render::cutscene::script::CutsceneBeat::Dialogue {
                speaker: "Lab Raider".into(),
                text: "Prototype is awake. Keep it away from the doors.".into(),
            },
            ambition_render::cutscene::script::CutsceneBeat::Dialogue {
                speaker: "Salvage Lead".into(),
                text: "This one isn't on the manifest. Tag it and move.".into(),
            },
            ambition_render::cutscene::script::CutsceneBeat::SetFlag {
                id: "intro_raid_started".into(),
                on: true,
            },
            ambition_render::cutscene::script::CutsceneBeat::SetFlag {
                id: "wrong_list_clue_1".into(),
                on: true,
            },
            ambition_render::cutscene::script::CutsceneBeat::SetFlag {
                id: "intro_raid_seen".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("intro_raid_seen")
}

fn drain_market_arrival_script() -> ambition_render::cutscene::script::CutsceneScript
{
    ambition_render::cutscene::script::CutsceneScript::new(
        "drain_market_arrival",
        vec![
            ambition_render::cutscene::script::CutsceneBeat::Fade {
                to_alpha: 0.0,
                seconds: 0.6,
            },
            ambition_render::cutscene::script::CutsceneBeat::Banner {
                text: "Drain Market — STAFF ONLY".into(),
                seconds: 1.2,
            },
            ambition_render::cutscene::script::CutsceneBeat::Dialogue {
                speaker: "Oiler".into(),
                text: "Well. That's not a rat. You came out of the bad pipe.".into(),
            },
            ambition_render::cutscene::script::CutsceneBeat::SetFlag {
                id: "drain_market_arrival_seen".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("drain_market_arrival_seen")
}

fn first_ripple_script() -> ambition_render::cutscene::script::CutsceneScript {
    // Not bound to a room (the player triggers it by interacting with
    // the ripple). Wiring the interaction lives in v1.1; the script is
    // here so the trigger can be `request`-ed once that lands.
    ambition_render::cutscene::script::CutsceneScript::new(
        "first_ripple",
        vec![
            ambition_render::cutscene::script::CutsceneBeat::Dialogue {
                speaker: "Gate Janitor".into(),
                text: "Don't touch that. That's not a gate.".into(),
            },
            ambition_render::cutscene::script::CutsceneBeat::Wait { seconds: 0.4 },
            ambition_render::cutscene::script::CutsceneBeat::Dialogue {
                speaker: "Gate Janitor".into(),
                text: "Okay. That's a problem.".into(),
            },
            ambition_render::cutscene::script::CutsceneBeat::SetFlag {
                id: "first_ripple_used".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("first_ripple_seen")
}

fn creator_final_fragment_script(
) -> ambition_render::cutscene::script::CutsceneScript {
    // Played on the creator's interrupted final lines. v1 plays the
    // normal route; the fast/impossible variants are listed in the
    // design doc for a v1.x pass.
    ambition_render::cutscene::script::CutsceneScript::new(
        "creator_final_fragment",
        vec![
            ambition_render::cutscene::script::CutsceneBeat::Dialogue {
                speaker: "Creator".into(),
                text: "There's a question you were made to—".into(),
            },
            ambition_render::cutscene::script::CutsceneBeat::SetFlag {
                id: "intro_creator_dead".into(),
                on: true,
            },
            ambition_render::cutscene::script::CutsceneBeat::SetFlag {
                id: "intro_creator_fragment_normal".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("intro_creator_fragment_seen")
}
