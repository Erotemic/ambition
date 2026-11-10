//! Check boot image demand and duplicate schedule registrations.
//!
//! Image dimensions measure asset demand independently of machine speed.
//! Schedule checks identify the repeated system itself.

use ambition_app::app::{build_visible_app, VisibleRenderMode};

/// Decoded megapixels at boot.
const BOOT_MEGAPIXEL_BUDGET: f64 = 40.0;

/// Boot the shipped shell composition and let asset loading settle.
///
/// Settling is what makes the assertion mean anything. This runs until the image count has been
/// unchanged for a stretch, which is the honest "loading is done" signal available without
/// reaching into the asset server.
fn boot_and_settle() -> bevy::app::App {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));

    let images = |app: &bevy::app::App| {
        app.world()
            .get_resource::<ambition_platformer2d::render::asset_census::ImageCensus>()
            .map(|census| census.total_images())
            .unwrap_or(0)
    };

    let mut last_seen = u64::MAX;
    let mut quiet_polls = 0u32;
    let deadline = std::time::Instant::now() + SETTLE_DEADLINE;
    while std::time::Instant::now() < deadline {
        app.update();
        // REAL time, not simulated. Decoding happens on IO threads and the app
        // clock is pinned to `ManualDuration`, so spinning `update()` advances the
        // simulation without giving the decoders any wall-clock in which to
        // finish. See `SETTLE_POLL`.
        std::thread::sleep(SETTLE_POLL);
        let seen = images(&app);
        if seen == last_seen {
            quiet_polls += 1;
            if quiet_polls >= SETTLE_QUIET_POLLS {
                break;
            }
        } else {
            last_seen = seen;
            quiet_polls = 0;
        }
    }
    app
}

/// Poll interval in real time, matching the clock used by asset decoders.
/// This title-screen composition measures fewer assets than full desktop startup,
/// which also demands startup-room characters and eager boss sheets.
const SETTLE_POLL: std::time::Duration = std::time::Duration::from_millis(25);
/// Consecutive quiet polls before the world counts as settled — 500ms of real
/// silence, which is longer than any single sheet takes to decode.
const SETTLE_QUIET_POLLS: u32 = 20;
/// Hard stop, so a composition that never stops loading fails the budget rather
/// than hanging the suite.
const SETTLE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(20);

#[test]
fn boot_decodes_no_more_than_its_budget() {
    let app = boot_and_settle();
    let census = app
        .world()
        .get_resource::<ambition_platformer2d::render::asset_census::ImageCensus>()
        .expect("the image census is always-on; without it this guard is blind");

    let megapixels = census.total_megapixels();
    let images = census.total_images();

    // The vacuity guard. A composition that decodes nothing passes any ceiling,
    // and a headless harness is exactly the kind of thing that quietly stops
    // decoding — at which point this would report "budget met" forever.
    assert!(
        images > 0,
        "nothing was decoded at all, so this budget is measuring an empty world \
         rather than the shipped one"
    );
    assert!(
        megapixels <= BOOT_MEGAPIXEL_BUDGET,
        "boot decoded {megapixels:.1} MP across {images} images, over the \
         {BOOT_MEGAPIXEL_BUDGET:.0} MP budget. Something is decoding at startup \
         that should decode on demand — this was 627 MP before the character seam \
         made sheet decoding lazy, and the symptom is seconds of launch time in \
         `png::filter::paeth::unfilter` that no profile attributes to an asset. \
         Run the app and read the `[image]` lines: they name the sheet and its \
         megapixels."
    );
    eprintln!(
        "[boot-budget] {megapixels:.1} MP / {images} images \
         (budget {BOOT_MEGAPIXEL_BUDGET:.0} MP)"
    );
}

#[test]
fn no_system_is_registered_twice_in_one_schedule() {
    let mut app = boot_and_settle();

    // ⛔⛔ THIS USED TO READ THE SCHEDULES AS THEY LAY, AND THE ONE THAT MATTERS
    // MOST WAS NOT AMONG THEM. `Schedule::systems()` fails until the graph is
    // built, and a schedule only builds it when it first RUNS — so an app that
    // boots to the launcher and never enters a match has never run `GgrsSchedule`.
    // The skip carried a comment saying an uninitialized schedule "holds no
    // registrations to duplicate", which is the plausible-sounding half of the
    // truth: it holds every registration ever added to it, it simply has not
    // sorted them yet.
    //
    // ⇒ Measured 2026-09-06, the schedules this test was silently skipping:
    // `GgrsSchedule` (the ENTIRE rollback simulation — every gameplay system in
    // the game), `PhysicsSchedule`, `SubstepSchedule`, `ReadInputs`, `Render`,
    // `AdvanceWorld`, `LoadWorld`, `SaveWorld`. The guard was pinned to `Update`
    // and `Startup` and reported the whole app clean.
    //
    // ⚠ AND IT WAS FOUND BY A POISON, NOT BY READING. A deliberate double-install
    // of `ambition_mount::steer_mount_from_rider` — the exact slip of copying an
    // install instead of moving it, while moving capability installs into
    // capability-owned plugins — left this test GREEN. ⇒ A guard that skips a
    // subject reports on the subjects it kept, in the same words it would use if
    // it had checked everything.
    //
    // `update_schedule_census.rs` had the answer already: initialize, then read.
    let mut counts = std::collections::BTreeMap::<(String, String), usize>::new();
    app.world_mut()
        .resource_scope(|world, mut schedules: bevy::prelude::Mut<bevy::ecs::schedule::Schedules>| {
    for (label, schedule) in schedules.iter_mut() {
        // Building the graph can legitimately fail (a schedule whose ordering
        // constraints reference systems this composition never installed). Such a
        // schedule is skipped for a REASON now, rather than because it had not
        // been asked to run.
        let _ = schedule.initialize(world);
        let Ok(systems) = schedule.systems() else {
            continue;
        };
        for system in systems {
            let name = format!("{}", system.1.name());
            if name.ends_with("apply_deferred") || name.contains("{{closure}}") {
                continue;
            }
            *counts.entry((format!("{label:?}"), name)).or_default() += 1;
        }
    }
        });

    assert!(
        !counts.is_empty(),
        "no systems at all — this is measuring an app that was never composed"
    );

    // ⭐⭐ THE DELIBERATE ONES, EACH WITH THE REASON ITS OWN SITE GIVES.
    //
    // Widening the guard to `GgrsSchedule` turned up ten duplicates, and every
    // one of them was intentional and already argued for in the code that wrote
    // it. Five of the ten have since stopped existing rather than stopped being
    // excused — see the note where they used to be listed. ⇒ The allow-list is not an exemption for them; it is the point at which
    // an accident becomes a DECISION. Anything doubled that is not on this list is
    // a system nobody decided to run twice.
    //
    // ⚠ AND THE LIST IS CHECKED IN BOTH DIRECTIONS, which matters more than it
    // looks: an entry that has stopped being duplicated is a stale exemption, and
    // a stale exemption silently re-opens the hole the day somebody duplicates
    // that system for real. A reason expires exactly like a measurement.
    let deliberate: &[(&str, &str, &str)] = &[
        (
            "Update",
            "ambition_load_presentation::plugin::apply_presentation_commands",
            "observe host commands before drive; apply action results before cleanup and render",
        ),
        // `ambition_platformer2d_runtime/src/lib.rs` — the identity trio runs at
        // the HEAD of the frame and again at the TAIL, after the last in-tick
        // spawner, so a GGRS save at a transition boundary cannot capture a
        // freshly-lowered body without its `SimId`. `Without<SimId>` makes the
        // second pass a no-op on everything the first pass reached.
        ("GgrsSchedule", "sim_identity::ensure_sim_id", "head and tail of the frame"),
        ("GgrsSchedule", "sim_identity::mint_spawned_sim_ids", "head and tail of the frame"),
        ("GgrsSchedule", "sim_identity::heal_projectile_owners", "head and tail of the frame"),
        // `combat_schedule.rs` — the second stamp catches a bolt that materialized
        // THIS tick, whose firer can still be eliminated later in `Settle`.
        (
            "GgrsSchedule",
            "allegiance::stamp_attack_allegiance",
            "delayed bolts take their side inside Materialize, not next frame",
        ),
        // `body/mod.rs` — once in `WorldPrep` for the pogo derivation and the
        // collision overlay, once between `Playback` and `Resolve` for the damage
        // pass. Same rule, two consumers with different timing needs.
        (
            "GgrsSchedule",
            "target_volumes::refresh_body_damageable_volumes",
            "two consumers with different timing needs",
        ),
        // ⭐ A2a: BOSSES JOINED THE SECOND PUBLICATION, and the decision is the
        // same one, made deliberately rather than discovered here. A boss
        // published once, in `WorldPrep`, chained AHEAD of its own brain and
        // animator — so the value described the previous frame and all three
        // damage-side consumers re-derived the geometry instead. They read the
        // publication now, and it is republished in the window they were
        // hand-rolling: after `Playback`, before `Resolve`. The `WorldPrep` copy
        // stays because the pogo derivation and the collision overlay are
        // rebuilt in that set and need it there.
        (
            "GgrsSchedule",
            "target_volumes::refresh_boss_damageable_volumes",
            "two consumers with different timing needs; damage reads the second",
        ),
        // ⛔⛤ **FIVE `LoadWorld` ENTRIES LEFT THIS LIST ON 2026-09-17, AND THE
        // STALE-DIRECTION CHECK IS WHAT SENT ME HERE.** `ClearPortals`,
        // `DropPortalGun`, `FirePortalGun`, `PickUpPortalGun` and
        // `TogglePortalGun` were each registered under both an old and a new
        // rollback name *"so the compatibility registration keeps the rollback
        // schema byte-for-byte"*, and this list excused the second
        // `clear_message_channel` that came with each alias. The aliases are
        // gone (schema v196 → v197), so the exemptions had to go in the same
        // commit: an excuse that no longer describes anything is a hole with a
        // comment over it, and this list's own doc says so.
        // ⇒ What replaced them is not another exemption but an impossibility:
        // `no_two_schema_rows_describe_the_same_type_the_same_way`
        // (`rollback_schema_baseline.rs`) refuses a second registration of one
        // type under one kind, which is where the duplicate systems came from.
    ];
    let excused = |schedule: &str, name: &str| {
        deliberate
            .iter()
            .any(|(s, fragment, _)| *s == schedule && name.contains(fragment))
    };

    let doubled: Vec<String> = counts
        .iter()
        .filter(|((schedule, name), count)| **count > 1 && !excused(schedule, name))
        .map(|((schedule, name), count)| format!("  {count}x  {schedule}  {name}"))
        .collect();

    // ⛔ THE OTHER DIRECTION. Every excuse must still describe something real.
    let stale: Vec<String> = deliberate
        .iter()
        .filter(|(schedule, fragment, _)| {
            !counts.iter().any(|((s, name), count)| {
                s == schedule && name.contains(fragment) && *count > 1
            })
        })
        .map(|(schedule, fragment, why)| format!("  {schedule}  {fragment}  ({why})"))
        .collect();
    assert!(
        stale.is_empty(),
        "{} entr(ies) on the deliberate-duplicate list are no longer duplicated:\n{}\n\n\
         Delete them. A stale exemption is indistinguishable from a live one and \
         re-opens the hole the day somebody duplicates that system by accident.",
        stale.len(),
        stale.join("\n"),
    );

    assert!(
        doubled.is_empty(),
        "{} system(s) are registered more than once in a single schedule:\n{}\n\n\
         A non-unique plugin installed by two owners does this silently — Bevy \
         panics on a duplicate UNIQUE plugin and says nothing about the rest. The \
         doubled system runs twice per frame, which for anything that drains, \
         decays, or advances state is a rate bug that looks like bad tuning.",
        doubled.len(),
        doubled.join("\n"),
    );
}

/// The initial title has no actor cast to materialize.
#[test]
fn the_initial_title_stages_no_character_art() {
    let app = boot_and_settle();
    let staged = app.world()
        .get_resource::<ambition_platformer2d::assets::materialization::CharacterLoadStates>()
        .expect("the host installs character materialization state")
        .cast().len();
    assert_eq!(staged, 0, "the initial title demanded character art with no actor cast");
}

/// What one test boot costs, by phase: the harness built (App, plugins,
/// content), the first update, and the next 59. Every `app_it` test that
/// boots a room pays the first two.
#[test]
#[ignore = "probe: prints what one boot costs; run with --ignored --nocapture --test-threads=1"]
fn probe_what_one_boot_costs() {
    use ambition_app::AmbitionSim as _;
    for round in 0..3 {
        let started = std::time::Instant::now();
        let mut sim = ambition_app::Platformer2dSimHarness::new_with_options(
            crate::common::fixed_60hz_room_options(crate::two_players_two_live_rooms::ROOM),
        )
        .expect("the room boots");
        let built = started.elapsed();
        sim.app_mut().update();
        let first = started.elapsed() - built;
        for _ in 0..59 {
            sim.app_mut().update();
        }
        let rest = started.elapsed() - built - first;
        println!(
            "[boot] round {round}: harness built {:.2} s, first update {:.2} s, next 59 updates {:.2} s ({:.1} ms each)",
            built.as_secs_f64(),
            first.as_secs_f64(),
            rest.as_secs_f64(),
            rest.as_secs_f64() * 1e3 / 59.0
        );
    }
}
