//! Intro cutscene scripts + room→cutscene bindings.
//!
//! Inserted into the shared [`crate::cutscene::CutsceneLibrary`] and
//! [`crate::cutscene::RoomCutsceneBindings`] by
//! [`crate::intro::plugin::install_intro_cutscenes_system`] at startup,
//! so the sandbox cutscene/system runtime picks them up the moment the
//! player enters the matching room.
//!
//! Beats are intentionally short — the design doc is firm that the
//! intro should not become a long cutscene wall. Each room gets at
//! most a few banner/dialogue beats before control returns.

use ambition_engine as ae;

use crate::cutscene::CutsceneLibrary;

/// Room → cutscene bindings for the intro slice. Mirrors the shape of
/// [`crate::cutscene::RoomCutsceneBindings::defaults`] — `(room_id,
/// cutscene_id)` pairs walked once per room change in
/// `auto_trigger_room_cutscenes`.
pub const INTRO_ROOM_CUTSCENE_BINDINGS: &[(&str, &str)] = &[
    ("intro_wake_room", "intro_wake"),
    ("intro_raid_corridor", "intro_raid"),
    ("drain_alley", "drain_market_arrival"),
    ("gate_stack_lower", "gate_stack_reveal"),
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
    library.insert(gate_stack_reveal_script());
    library.insert(first_ripple_script());
    library.insert(creator_final_fragment_script());
}

fn intro_wake_script() -> ae::CutsceneScript {
    ae::CutsceneScript::new(
        "intro_wake",
        vec![
            ae::CutsceneBeat::Fade {
                to_alpha: 0.0,
                seconds: 0.8,
            },
            ae::CutsceneBeat::Banner {
                text: "// boot".into(),
                seconds: 1.0,
            },
            ae::CutsceneBeat::Dialogue {
                speaker: "Creator".into(),
                text: "Hey you, you're finally awake.".into(),
            },
            ae::CutsceneBeat::SetFlag {
                id: "intro_started".into(),
                on: true,
            },
            ae::CutsceneBeat::SetFlag {
                id: "intro_wake_seen".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("intro_wake_seen")
}

fn intro_raid_script() -> ae::CutsceneScript {
    ae::CutsceneScript::new(
        "intro_raid",
        vec![
            ae::CutsceneBeat::Banner {
                text: "// PERIMETER BREACH".into(),
                seconds: 1.2,
            },
            ae::CutsceneBeat::Dialogue {
                speaker: "Salvage Lead".into(),
                text: "Wrong room. Take anything that boots.".into(),
            },
            ae::CutsceneBeat::Dialogue {
                speaker: "Framebreaker".into(),
                text: "Kill the Clanker before it learns our names.".into(),
            },
            ae::CutsceneBeat::Dialogue {
                speaker: "Salvage Lead".into(),
                text: "This one isn't on the manifest. — Then manifest it.".into(),
            },
            ae::CutsceneBeat::SetFlag {
                id: "intro_raid_started".into(),
                on: true,
            },
            ae::CutsceneBeat::SetFlag {
                id: "wrong_list_clue_1".into(),
                on: true,
            },
            ae::CutsceneBeat::SetFlag {
                id: "intro_raid_seen".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("intro_raid_seen")
}

fn drain_market_arrival_script() -> ae::CutsceneScript {
    ae::CutsceneScript::new(
        "drain_market_arrival",
        vec![
            ae::CutsceneBeat::Fade {
                to_alpha: 0.0,
                seconds: 0.6,
            },
            ae::CutsceneBeat::Banner {
                text: "Drain Market — STAFF ONLY".into(),
                seconds: 1.2,
            },
            ae::CutsceneBeat::Dialogue {
                speaker: "Oiler".into(),
                text: "Well. That's not a rat. You came out of the bad pipe.".into(),
            },
            ae::CutsceneBeat::SetFlag {
                id: "drain_market_arrival_seen".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("drain_market_arrival_seen")
}

fn gate_stack_reveal_script() -> ae::CutsceneScript {
    ae::CutsceneScript::new(
        "gate_stack_reveal",
        vec![
            ae::CutsceneBeat::Banner {
                text: "GATE STACK (LOWER)".into(),
                seconds: 1.4,
            },
            ae::CutsceneBeat::Dialogue {
                speaker: "PA".into(),
                text: "Gate Six delayed. Shark traffic. Please declare all impossible luggage.".into(),
            },
            ae::CutsceneBeat::SetFlag {
                id: "gate_stack_reveal_seen".into(),
                on: true,
            },
            ae::CutsceneBeat::SetFlag {
                id: "player_unmanifested_hardware".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("gate_stack_reveal_seen")
}

fn first_ripple_script() -> ae::CutsceneScript {
    // Not bound to a room (the player triggers it by interacting with
    // the ripple). Wiring the interaction lives in v1.1; the script is
    // here so the trigger can be `request`-ed once that lands.
    ae::CutsceneScript::new(
        "first_ripple",
        vec![
            ae::CutsceneBeat::Dialogue {
                speaker: "Gate Janitor".into(),
                text: "Don't touch that. That's not a gate.".into(),
            },
            ae::CutsceneBeat::Wait { seconds: 0.4 },
            ae::CutsceneBeat::Dialogue {
                speaker: "Gate Janitor".into(),
                text: "Okay. That's a problem.".into(),
            },
            ae::CutsceneBeat::SetFlag {
                id: "first_ripple_used".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("first_ripple_seen")
}

fn creator_final_fragment_script() -> ae::CutsceneScript {
    // Played on the creator's interrupted final lines. v1 plays the
    // normal route; the fast/impossible variants are listed in the
    // design doc for a v1.x pass.
    ae::CutsceneScript::new(
        "creator_final_fragment",
        vec![
            ae::CutsceneBeat::Dialogue {
                speaker: "Creator".into(),
                text: "There's a question you were made to—".into(),
            },
            ae::CutsceneBeat::SetFlag {
                id: "intro_creator_dead".into(),
                on: true,
            },
            ae::CutsceneBeat::SetFlag {
                id: "intro_creator_fragment_normal".into(),
                on: true,
            },
        ],
    )
    .with_seen_flag("intro_creator_fragment_seen")
}
