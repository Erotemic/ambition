//! Recorded rollback wire schema.
//! Moving registrations between domain adapters must leave `schema_dump()` byte-identical;
//! encoding/projection changes are wire-format changes and require an explicit schema-version bump.
//! The readable baseline omits registration-owner labels because ownership is organizational,
//! not part of the snapshot schema.

use ambition_app::{AmbitionSim, Platformer2dSimHarness, TimestepMode};

/// Committed readable schema baseline.
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

    // Causal recorder channels carry no snapshot bytes, so compiling the instrument must not
    // change the state-schema baseline.
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
