//! **Boot budgets: the numbers the always-on censuses print, enforced.**
//!
//! `[schedule-census]`, `[frame-spike]` and `[image]` print on every launch and
//! nothing has ever failed when one regressed. That is how the boot decode
//! reached **627 megapixels across 2.5 GB** before anyone looked — the profile
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
//!
//! These are budgets, not golden values. A ceiling with real headroom fails on
//! the CLASS of regression that matters — a sheet decoded eagerly at boot, a
//! plugin registered twice — and stays quiet for ordinary growth. An exact
//! assertion would fail on every legitimate addition and be raised without being
//! read, which is the same as not having one.

use ambition_app::app::{build_visible_app, VisibleRenderMode};

/// Decoded megapixels at boot.
///
/// Measured 2026-07-27: **11.1 MP across 19 images**. The ceiling is ~3.5x that —
/// enormous headroom for new art, and still an order of magnitude below the
/// 627 MP this used to be. Crossing it means something started decoding whole
/// character sheets at startup again rather than on demand.
const BOOT_MEGAPIXEL_BUDGET: f64 = 40.0;

/// Systems registered across every schedule after the shell host is composed.
///
/// Measured 2026-07-27: **2645**. The ceiling catches runaway registration — a
/// plugin added twice, a group installed by two owners — which is a live defect
/// class here: a duplicated `add_message` installs a second update system for one
/// channel, drains it twice a frame, and silently eats unread messages. That was
/// a real bug in `AmbitionLoadPlugin` (closed 2026-07-27) and its symptom was
/// invisible.
const SYSTEM_COUNT_BUDGET: usize = 3400;

/// Boot the shipped shell composition and let asset loading settle.
///
/// Settling is what makes the assertion mean anything. Decoding happens on IO
/// threads, so a fixed frame count samples a partly-loaded world — and an
/// upper-bound assertion on an understated number passes vacuously. This runs
/// until the image count has been unchanged for a stretch, which is the honest
/// "loading is done" signal available without reaching into the asset server.
fn boot_and_settle() -> bevy::app::App {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));

    let mut last_seen = u64::MAX;
    let mut quiet_updates = 0;
    for _ in 0..600 {
        app.update();
        let images = app
            .world()
            .get_resource::<ambition::render::asset_census::ImageCensus>()
            .map(|census| census.total_images())
            .unwrap_or(0);
        if images == last_seen {
            quiet_updates += 1;
            if quiet_updates >= 60 {
                break;
            }
        } else {
            last_seen = images;
            quiet_updates = 0;
        }
    }
    app
}

#[test]
fn boot_decodes_no_more_than_its_budget() {
    let app = boot_and_settle();
    let census = app
        .world()
        .get_resource::<ambition::render::asset_census::ImageCensus>()
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
