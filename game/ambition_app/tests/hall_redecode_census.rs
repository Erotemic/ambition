//! How much of the Hall's art is decoded MORE THAN ONCE on the way in.
//!
//! `asset-preparation-and-residency.md` Open work 5 asks for an audit of
//! accidental re-preparation. The ledger already counts it — `inserted()` bumps
//! `re_decodes` whenever a path is inserted for the second time — but nothing
//! had ever run that counter over the Hall entry, which is the one transition
//! big enough for a repeat to cost anything (129 authored NpcSpawns).
//!
//! ⚠⚠ WHAT THIS CAN AND CANNOT ANSWER, up front so no reader takes its green for
//! more than it is. `NoWindow` decodes 22 images / 4.5 MP over a 126-character
//! Hall entry; the HOST run of the same entry took 434 MP. This is ~5% of the
//! art, so a clean result here is a regression guard on the headless road and is
//! NOT evidence that the shipped game re-decodes nothing. The run that can answer
//! that boots with `AMBITION_PROFILE_CENSUS=1` and reads `re-decodes N` from the
//! `[image-census]` line.
//!
//! ⛔⛔ `#[ignore]`, AND RUN ALONE BY A SCRIPT. THAT IS THE WHOLE DESIGN, and it
//! is forced by the ledger being a process-global `static` behind a `Mutex`.
//! `ambition_app` has ONE `[[test]]` target, so every file under `tests/` is a
//! MODULE of `app_it` sharing one process — and cargo runs those tests as
//! parallel threads. A sibling test booting its own app mid-window would land
//! its decodes in this count.
//!
//! The before/after delta below handles SEQUENTIAL contamination completely: it
//! brackets only this test's own window, so tests that ran earlier in the
//! process cannot inflate it. What the delta cannot handle is a test running
//! CONCURRENTLY, which is why this is `#[ignore]`d and driven by
//! `scripts/measure_hall_redecodes.sh` with an exact filter — one test, alone.
//!
//! ⚠ A second `[[test]]` target would give it a private process for free, and
//! was rejected on cost: it relinks the whole app, and this repository has
//! already measured that link as the dominant cost of a filtered test run (see
//! `ambition_app_tools`'s Cargo.toml). ⇒ DO NOT REMOVE THE `#[ignore]` to "make
//! it run in CI"; it would go flaky rather than red, which is worse.
//!
//! The Hall walk is the same road `hall_transition_cover.rs` drives — through
//! the room graph, standing in the authored door — rather than a synthesised
//! transition, so what is measured is what a player's entry costs.

use bevy::prelude::*;

use ambition_app::app::{VisibleRenderMode, build_visible_app, shell_host};
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::sprite_sheet::game_assets::image_stages;

/// The authored door from the hub, named for the same reason its sibling names
/// it: a test that walked "any zone" would quietly measure a cheap room.
const HALL_DOOR_ZONE: &str = "hall_of_characters_door";

/// The Hall is only worth measuring if it actually staged its cast. Below this
/// the run is a no-op and a re-decode count of zero would mean "nothing
/// happened", not "nothing repeated" — the two readings this file must never
/// confuse.
const MINIMUM_HALL_CAST: usize = 100;

fn staged_cast_len(app: &App) -> usize {
    app.world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::CharacterLoadStates>()
        .map(|states| states.cast().len())
        .unwrap_or(0)
}

fn step(app: &mut App) {
    app.update();
    std::thread::sleep(std::time::Duration::from_millis(8));
}

fn settle_cast(app: &mut App, secs: u64) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    let mut last = usize::MAX;
    let mut quiet = 0;
    while std::time::Instant::now() < deadline {
        step(app);
        let now = staged_cast_len(app);
        if now == last {
            quiet += 1;
            if quiet >= 25 {
                break;
            }
        } else {
            last = now;
            quiet = 0;
        }
    }
}

#[test]
#[ignore = "reads the process-global image ledger: valid only when run alone, via scripts/measure_hall_redecodes.sh"]
fn the_halls_entry_is_counted_for_art_it_decodes_twice() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    settle_cast(&mut app, 10);

    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle_cast(&mut app, 20);
    let before_cast = staged_cast_len(&app);
    let before_redecodes = image_stages::ledger().re_decodes;

    let (target_room, arrival) = {
        let mut query = app
            .world_mut()
            .query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query.iter(app.world()).next().expect("a session room set");
        let zone = room_set
            .active_loading_zones()
            .iter()
            .find(|zone| zone.id == HALL_DOOR_ZONE || zone.name == HALL_DOOR_ZONE)
            .unwrap_or_else(|| {
                panic!(
                    "the active room '{}' has no `{HALL_DOOR_ZONE}`, so this \
                     measurement is counting nothing",
                    room_set.active_spec().id
                )
            })
            .clone();
        let transition = room_set
            .transition_for_player(
                zone.aabb,
                ambition_platformer2d::engine_core::Vec2::ZERO,
                true,
            )
            .expect("the hall door resolves to a transition");
        (
            room_set.rooms[transition.target_room].id.clone(),
            transition.arrival,
        )
    };

    let subject = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d::platformer::sim_id::SimId,
            bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        q.iter(world)
            .next()
            .expect("the hall has a primary avatar to send through its door")
            .clone()
    };
    let _ = app.world_mut()
        .resource_mut::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
        .record(
            0,
            ambition_platformer2d::actors::session::lifecycle_commit::LifecycleIntent::Transition(
                ambition_platformer2d::actors::session::lifecycle_commit::RoomTransitionIntent {
                    subject,
                    target_room,
                    arrival,
                    edge_exit: false,
                    zone_sfx: None,
                },
            ),
        );

    // Let the transition demand and decode. Long enough for the whole cast, not
    // a fixed frame count: a re-decode that happens on the tenth frame after the
    // reveal is still a re-decode a player paid for.
    settle_cast(&mut app, 30);
    for _ in 0..120 {
        step(&mut app);
    }

    let staged = staged_cast_len(&app);
    // ⛔ THE FIRST PREMISE: the Hall's cast was actually staged.
    assert!(
        staged - before_cast >= MINIMUM_HALL_CAST,
        "the Hall entry staged only {} new character(s) (from {before_cast} to \
         {staged}), so this run demanded no art and its re-decode count says \
         nothing",
        staged - before_cast
    );

    let ledger = image_stages::ledger();
    let re_decodes = ledger.re_decodes - before_redecodes;
    let (total, megapixels) = ledger
        .resident_by_road()
        .values()
        .fold((0usize, 0f64), |(n, mp), (count, road_mp)| {
            (n + count, mp + road_mp)
        });
    drop(ledger);

    println!(
        "hall-entry re-decode census: {re_decodes} path(s) decoded more than once \
         during the transition; {total} image(s) resident ({megapixels:.1}MP) \
         across {} newly staged character(s)",
        staged - before_cast
    );

    // ⛔⛔ THE SECOND PREMISE, AND THE ONE THAT MATTERS. Staging is a DEMAND;
    // `re_decodes` counts INSERTIONS. The first draft of this test guarded only
    // the cast size, so it would have reported a confident zero over a run that
    // decoded nothing at all — the count and the guard were measuring different
    // things.
    //
    // ⚠ AND THE GUARD BELOW IS DELIBERATELY WEAK, because the honest reading is
    // that THIS ROAD CANNOT ANSWER THE HOST'S QUESTION. Measured 2026-09-02:
    // `NoWindow` decodes 22 images / 4.5 MP over a 126-character Hall entry,
    // where the host run of the same entry took 434 MP. A `NoWindow` composition
    // has no render world and decodes almost nothing on purpose
    // (`hall_transition_cover.rs` says so). So a green here is a REGRESSION GUARD
    // ON THE HEADLESS ROAD and nothing more: it is 5% of the host's art, and a
    // repeat that only happens under real GPU residency would not appear in it.
    // ⇒ THE HOST TELL, for the run that can answer it: boot with
    // `AMBITION_PROFILE_CENSUS=1` and read `re-decodes N` in the `[image-census]`
    // line. That number over a real Hall entry is the measurement Open work 5
    // actually wants; this one only proves the headless road stays clean.
    const MINIMUM_DECODED_IMAGES: usize = 15;
    assert!(
        total >= MINIMUM_DECODED_IMAGES,
        "only {total} image(s) were decoded during this run, so a re-decode count \
         of {re_decodes} is measuring an empty population rather than a clean one"
    );

    // The number this measurement was written to establish. It is asserted at
    // the value it MEASURES rather than at zero-on-principle: a ceiling picked
    // without a reading is a guess, and one picked above the reading cannot fail.
    // MEASURED 0 on 2026-09-02 over the population the guard above pins, so this
    // is asserted at its reading rather than at zero-on-principle.
    assert_eq!(
        re_decodes, 0,
        "the Hall entry decoded {re_decodes} image path(s) a second time on the \
         HEADLESS road, where the population is only {total} image(s). Each is a \
         full decode of art the process already held — see `asset-preparation-and-\
         residency.md` Open work 5. If this is a deliberate re-tier, the ledger \
         should say so rather than counting it as a repeat."
    );
}
