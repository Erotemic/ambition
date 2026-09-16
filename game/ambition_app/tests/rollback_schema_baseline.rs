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

/// ⛔⛤ THE RECORDED BASELINE IS ONE COMPOSITION'S DUMP, AND THE PLAYER RUNS A
/// DIFFERENT COMPOSITION FROM THE ONE THAT RECORDS IT.
///
/// The arm above builds `Platformer2dSimHarness`; the shipped binary builds
/// `build_visible_app`. Everything that reads this baseline — the ratchet in
/// `scripts/check_absence_contracts.py`, the fingerprint two peers would
/// negotiate, this file's own byte-for-byte comparison — is describing the
/// sandbox unless the two compositions register the same schema. A schema that
/// differed between them would mean the guarded identity is not the shipped
/// identity, and no arm in the repository could see it: the sandbox arm stays
/// green precisely because it never asks the shipped app.
///
/// ⚠ This compares `schema_dump`, not `deterministic_dump`, because the peer
/// question is what CROSSES: `deterministic_dump` carries the registration
/// OWNER, and a domain moving a registration between adapters is an
/// organizational change that must not be a wire-format one.
#[test]
fn the_shipped_app_registers_the_same_schema_as_the_sandbox() {
    let shipped =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let shipped_dump = shipped
        .world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("the shipped app installs the rollback registry")
        .schema_dump();

    let sim = Platformer2dSimHarness::new_with_options(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz()),
    )
    .expect("sandbox sim builds");
    let sandbox_dump = sim
        .world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("rollback registry is installed by the engine plugins")
        .schema_dump();

    // ⛔ THE PREMISE, FIRST. Two empty registries are byte-identical, and a
    // comparison between them would report the success condition while saying
    // nothing. The recorded baseline is 493 rows; either side collapsing is a
    // broken build, not a passing claim about composition. 494 lines today: the
    // version header plus the 493 recorded rows.
    let rows = |dump: &str| dump.lines().count();
    assert!(
        rows(&shipped_dump) > 400 && rows(&sandbox_dump) > 400,
        "a registry collapsed: shipped has {} rows, sandbox {}. Both sides of \
         this comparison must be populated or it certifies nothing.",
        rows(&shipped_dump),
        rows(&sandbox_dump),
    );

    if shipped_dump != sandbox_dump {
        let ship: Vec<&str> = shipped_dump.lines().collect();
        let sand: Vec<&str> = sandbox_dump.lines().collect();
        let only_shipped: Vec<&&str> = ship.iter().filter(|l| !sand.contains(l)).collect();
        let only_sandbox: Vec<&&str> = sand.iter().filter(|l| !ship.contains(l)).collect();
        panic!(
            "the shipped app and the sandbox register DIFFERENT rollback schemas, \
             so `rollback_schema_baseline.txt` — and every guard reading it — \
             describes a composition no player runs.\n\n\
             {} only in the SHIPPED app:\n{}\n\n{} only in the SANDBOX:\n{}\n\n\
             Two peers running the shipped binary would agree with each other and \
             disagree with every recorded identity in this repository.",
            only_shipped.len(),
            only_shipped
                .iter()
                .map(|l| format!("  + {l}"))
                .collect::<Vec<_>>()
                .join("\n"),
            only_sandbox.len(),
            only_sandbox
                .iter()
                .map(|l| format!("  - {l}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
}
