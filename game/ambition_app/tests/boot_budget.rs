//! Boot budgets: the numbers the always-on censuses print, enforced.
//!
//! `[schedule-census]`, `[frame-spike]` and `[image]` print on every launch and
//! nothing has ever failed when one regressed. That is how the boot decode
//! reached 627 megapixels across 2.5 GB before anyone looked — the profile
//! showed `png::filter::paeth::unfilter` burning seconds and named no asset, so
//! the cost was visible and unattributable for as long as it existed.
//!
//! It is 11 MP now, because the character-definition seam made startup decode
//! sheets on demand instead of eagerly. Nothing protected that.
//!
//! ## What is enforced, and what deliberately is not
//!
//! Only DETERMINISTIC numbers. Decoded pixels and registered systems are
//! functions of the composition and the asset tree, so they mean the same thing
//! on any machine. `[frame-spike]` milliseconds are not — a wall-clock budget in
//! a suite that runs on loaded machines is a flake generator, and a flaky guard
//! gets muted, which is worse than no guard.
//!
//! ## Why ceilings and not exact values

use ambition_app::app::{build_visible_app, VisibleRenderMode};

/// Decoded megapixels at boot.
const BOOT_MEGAPIXEL_BUDGET: f64 = 40.0;

/// Systems registered across every schedule after the shell host is composed.
const SYSTEM_COUNT_BUDGET: usize = 3400;

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
fn the_composition_registers_no_more_systems_than_its_budget() {
    let app = boot_and_settle();
    let schedules = app.world().resource::<bevy::ecs::schedule::Schedules>();
    let total: usize = schedules
        .iter()
        .map(|(_, schedule)| schedule.systems_len())
        .sum();

    assert!(
        total > 0,
        "no systems at all — this is measuring an app that was never composed"
    );
    assert!(
        total <= SYSTEM_COUNT_BUDGET,
        "the shell composition registers {total} systems, over the \
         {SYSTEM_COUNT_BUDGET} budget. The usual cause is a plugin or plugin \
         group installed by two owners: Bevy panics on a duplicate UNIQUE plugin, \
         but a non-unique one silently doubles its systems, and a doubled \
         message-update system drains its channel twice a frame and eats unread \
         messages. Compare `[schedule-census]` output before and after the change \
         — it prints per-schedule counts on every launch."
    );
    eprintln!("[boot-budget] {total} systems (budget {SYSTEM_COUNT_BUDGET})");
}
/// The precise version of the system-count budget.
///
/// The 3400 ceiling catches a doubled plugin only once the double is ~750 systems large, which is
/// why it is described in its own doc comment as a blunt instrument.
///
/// Two exclusions, both principled rather than convenient:
///
/// * `apply_deferred` is the SCHEDULER's own sync point. It inserts one wherever
///   ordering demands it — 19 in `Update` alone — and their count is a fact about
///   command ordering, not about registration.
/// * a `{{closure}}` has no name to be identified by. Eight distinct startup
///   phase marks share one string, so equality there is a collision rather than a
///   duplicate.
///
/// Anything else appearing twice in one schedule ran twice.
#[test]
fn no_system_is_registered_twice_in_one_schedule() {
    let app = boot_and_settle();
    let schedules = app.world().resource::<bevy::ecs::schedule::Schedules>();

    let mut counts = std::collections::BTreeMap::<(String, String), usize>::new();
    for (label, schedule) in schedules.iter() {
        // An uninitialized schedule (an `OnEnter` for a state never entered) has
        // no system list to read. Skipped rather than unwrapped: it holds no
        // registrations to duplicate.
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

    assert!(
        !counts.is_empty(),
        "no systems at all — this is measuring an app that was never composed"
    );

    let doubled: Vec<String> = counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|((schedule, name), count)| format!("  {count}x  {schedule}  {name}"))
        .collect();

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

/// What entering a gameplay session stages, budgeted.
///
/// This measures the STAGED CAST rather than megapixels, and that is the whole
/// design: the cast is a deterministic function of the composition and the room
/// graph, so it means the same thing on every machine, while decoded megapixels
/// headless depend on which variants the quality profile resolves. The cast is
/// also the thing that CAUSES the megapixels — one staged character is one sheet.
#[test]
fn a_gameplay_session_stages_no_more_of_the_cast_than_its_budget() {
    use ambition_platformer2d::game_shell::ShellCommand;

    let mut app = boot_and_settle();
    let staged_at_title = staged_cast_len(&app);
    assert_eq!(
        staged_at_title, 0,
        "the title screen staged {staged_at_title} character(s). Nothing is being \
         played yet, so this should be empty — a non-zero count here means the \
         cost this guard exists to bound has moved back into boot"
    );

    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_app::app::shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    let mut settled = app;
    settle_in_place(&mut settled);

    let staged = staged_cast_len(&settled);
    assert!(
        staged > 0,
        "the session staged no characters at all, so this budget is measuring an \
         empty world rather than the shipped one"
    );
    assert!(
        staged <= SESSION_STAGED_CAST_BUDGET,
        "entering the Ambition route staged {staged} characters, over the \
         {SESSION_STAGED_CAST_BUDGET} budget. One staged character is one sheet \
         decode, so this is the launch stutter in its cheapest observable form. \
         The usual cause is a prefetch reaching further than it should — see \
         `NEIGHBOR_PREFETCH_ROOM_BUDGET`. Run the game and read `[image-census]`."
    );
    eprintln!("[session-budget] {staged} staged (budget {SESSION_STAGED_CAST_BUDGET})");
}

/// The active room's own cast, plus a bounded neighbourhood's worth of prefetch.
const SESSION_STAGED_CAST_BUDGET: usize = 40;

fn staged_cast_len(app: &bevy::app::App) -> usize {
    app.world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::CharacterLoadStates>()
        .map(|states| states.cast().len())
        .unwrap_or(0)
}

/// `boot_and_settle`'s loop against an app that is already running.
fn settle_in_place(app: &mut bevy::app::App) {
    let mut last_seen = usize::MAX;
    let mut quiet_polls = 0u32;
    let deadline = std::time::Instant::now() + SETTLE_DEADLINE;
    while std::time::Instant::now() < deadline {
        app.update();
        std::thread::sleep(SETTLE_POLL);
        let seen = staged_cast_len(app);
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
}
