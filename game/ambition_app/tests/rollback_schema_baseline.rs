//! The rollback schema, written down.
//!
//! domain-owned adapters, and its rule for each move is: *"record the existing
//! descriptor list and fingerprint; move registrations to the domain adapter;
//! preserve registration order and projections; verify the resulting schema
//! fingerprint is unchanged."*
//!
//! This is the recording. `RollbackRegistry::deterministic_dump` is byte-identical
//! under equivalent insertion orders, so a migration that only MOVES a
//! registration leaves it untouched, and one that changes what is registered — or
//! what it projects — shows the exact line.
//!
//! The whole point during a migration is to see WHICH line, so the baseline is the readable
//! form and the fingerprint is derived from it.
//!
//! `schema_dump`, not `deterministic_dump`. The latter carries the
//! registration OWNER, which is an organisational label nothing reads — and
//! owner column would go red on every relocation and be worthless for the one
//! question it exists to answer: did the SCHEMA move?
//!
//! a change here is a WIRE-FORMAT change. The fingerprint is part of content identity; two
//! peers whose schemas differ cannot agree about a snapshot.
//!
//! So the version constant is not redundant bookkeeping beside this file — it is the only
//! signal for a whole class of wire change, and bumping it by hand is what makes that class
//! visible at all. A commit that changes an encoding and does not bump it produces two peers
//! that disagree about a snapshot while every test here is green.

use ambition_app::{AmbitionSim, Platformer2dSimHarness, TimestepMode};

/// Where the recorded schema lives. Text, and committed, because reading the
/// diff is the point.
const BASELINE: &str = include_str!("rollback_schema_baseline.txt");

#[test]
fn the_rollback_schema_matches_its_recorded_baseline() {
    let sim = Platformer2dSimHarness::new_with_options(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz()),
    )
    .expect("sandbox sim builds");

    let dump = sim
        .world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("rollback registry is installed by the engine plugins")
        .schema_dump();

    // THE INSTRUMENT'S CHANNELS ARE ADDITIVE, and the schema is a claim
    // about STATE. Building with `causal` registers three message-clear rows
    // for the causal recorders' own channels. They encode nothing — clearing a
    // channel carries no bytes into a snapshot — so two peers that differ only
    // in whether the instrument is compiled still agree about every value in the
    // wire format, which is what this baseline exists to protect.
    //
    // Recording them in the baseline instead would make the file unreadable in
    // the default build (three rows that are never present) and would state that
    // turning an inspector on is a wire-format change. It is not.
    let dump: String = dump
        .lines()
        .filter(|line| !line.starts_with("message.causal_"))
        .collect::<Vec<_>>()
        .join("\n");

    if dump.trim() != BASELINE.trim() {
        let recorded: Vec<&str> = BASELINE.trim().lines().collect();
        let live: Vec<&str> = dump.trim().lines().collect();
        let added: Vec<&&str> = live.iter().filter(|l| !recorded.contains(l)).collect();
        let removed: Vec<&&str> = recorded.iter().filter(|l| !live.contains(l)).collect();
        panic!(
            "the rollback schema no longer matches its baseline.\n\
             \n\
             This is a WIRE-FORMAT change: the fingerprint is part of content\n\
             identity, and two peers whose schemas differ cannot agree about a\n\
             snapshot. If you MOVED a registration (Campaign 2) the dump should\n\
             be byte-identical — a diff here means the move changed what is\n\
             registered or what it projects.\n\
             \n\
             {} added:\n{}\n\n{} removed:\n{}\n\n\
             If the change is deliberate, rewrite tests/rollback_schema_baseline.txt\n\
             with the live dump and say why in the commit.",
            added.len(),
            added
                .iter()
                .map(|l| format!("  + {l}"))
                .collect::<Vec<_>>()
                .join("\n"),
            removed.len(),
            removed
                .iter()
                .map(|l| format!("  - {l}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
}

/// The schema does not depend on how the app was composed.
#[test]
fn the_schema_is_the_same_from_a_second_build() {
    let dump = |room: Option<&str>| {
        let mut options = ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz());
        if let Some(room) = room {
            options = options.with_required_start_room(room);
        }
        let sim = Platformer2dSimHarness::new_with_options(options).expect("sandbox sim builds");
        sim.world()
            .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
            .expect("rollback registry is installed")
            .deterministic_dump()
    };

    assert_eq!(
        dump(None),
        dump(Some("combat_calibration_lab")),
        "the rollback SCHEMA changed with the starting room. It describes what \
         can be rewound, not what happens to exist, so a room-dependent schema \
         means a peer's snapshot compatibility depends on where it booted."
    );
}
