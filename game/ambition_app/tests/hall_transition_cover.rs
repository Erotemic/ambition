//! Verify the loading cover on the large `hall_of_characters` transition.
//!
//! The destination cast must be demanded up front, the foreground must become
//! visible after its configured reveal delay, and the cover must remain visible
//! while assets are outstanding. This headless test does not measure GPU upload
//! cost.

use bevy::prelude::*;

use ambition_app::app::{build_visible_app, shell_host, VisibleRenderMode};
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::load_presentation::{
    BasicLoadRoot, LoadForegroundPhase, LoadForegroundState,
};

/// The authored door from the hub. Named rather than "any zone" so this test
/// fails loudly if the Hall stops being reachable, instead of quietly covering
/// some other room's cheap transition.
const HALL_DOOR_ZONE: &str = "hall_of_characters_door";

/// Minimum staged cast that demonstrates the Hall is demanded up front.
const MINIMUM_HALL_CAST: usize = 100;

fn staged_cast_len(app: &App) -> usize {
    app.world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::CharacterLoadStates>()
        .map(|states| states.cast().len())
        .unwrap_or(0)
}

fn cover_entities(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&BasicLoadRoot>();
    query.iter(app.world()).count()
}

fn foreground_visible(app: &App) -> bool {
    app.world()
        .get_resource::<LoadForegroundState>()
        .and_then(|state| state.active.as_ref())
        .is_some_and(|active| active.phase != LoadForegroundPhase::HiddenGrace)
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
fn the_halls_transition_bills_its_whole_cast_and_covers_the_wait() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    settle_cast(&mut app, 10);

    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle_cast(&mut app, 20);
    let before = staged_cast_len(&app);

    // The REAL transition, resolved through the room graph rather than
    // synthesised: stand in the Hall door and press interact.
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
                    "the active room '{}' has no `{HALL_DOOR_ZONE}`, so this test is \
                     measuring nothing",
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

    // This one is recorded rather than walked, so the test names the avatar the same way
    // detection would.
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
    // the loop must not be the assertion. It waits for the cast to grow AT
    // ALL and then measures that same frame, so a Hall that trickled its
    // characters in ten at a time still fails the bill below. A loop that waited
    // for `>= MINIMUM_HALL_CAST` would pass by waiting, which is the shape this
    // whole file exists to catch.
    let mut arrived = false;
    for _ in 0..120 {
        step(&mut app);
        if staged_cast_len(&app) > before {
            arrived = true;
            break;
        }
    }
    assert!(
        arrived,
        "the Hall transition never demanded a single new character in 120 frames, \
         so this test measured no bill at all"
    );

    // ── The bill arrives at once ────────────────────────────────────────────
    let staged = staged_cast_len(&app);
    assert!(
        staged - before >= MINIMUM_HALL_CAST,
        "the Hall transition staged only {} new character(s) on its first frame \
         (from {before} to {staged}). The room authors 129 NpcSpawn placements, \
         so the rest are \
         being demanded later — after their actors spawn, in frame, uncovered, \
         which is the defect this file exists to keep closed",
        staged - before,
    );

    // ── And the wait is covered ─────────────────────────────────────────────
    //
    // Not on the FIRST frame: `loading_reveal_after` is 250ms, deliberately, so
    // a cheap room does not flash a loading screen. What matters is that an
    // expensive one reaches it.
    let mut visible_at = None;
    for frame in 0..120 {
        step(&mut app);
        if foreground_visible(&app) {
            visible_at = Some(frame);
            break;
        }
    }
    let visible_at = visible_at.expect(
        "the load foreground never became visible for a transition that stages \
         141 characters. Either the reveal grace outlives the load, or the \
         barrier released before the assets were ready — both present to a \
         player as 'it froze and there was no loading screen'",
    );
    assert!(
        cover_entities(&mut app) > 0,
        "the foreground reports visible but no `BasicLoadRoot` is on screen, so \
         nothing is actually drawn over the transition"
    );

    // Still covered a moment later: the barrier is holding the reveal, not
    // blinking a cover and committing behind it.
    for _ in 0..10 {
        step(&mut app);
    }
    assert!(
        foreground_visible(&app) && cover_entities(&mut app) > 0,
        "the cover appeared at frame {visible_at} and was gone ten frames later \
         while the Hall's assets were still outstanding — a cover that does not \
         outlast the load it covers is a flash, not a loading screen"
    );

    // ── And it can say WHAT it is waiting for ───────────────────────────────
    //
    // The poll computed `RoomAssetReadiness`, which names every pending asset, and kept `(settled,
    // total)` — throwing the names away every frame while the player stared at a number that could
    // not move.
    //
    // this test is the natural customer because a `NoWindow` host decodes almost nothing (see
    // the header), so the Hall's barrier here genuinely never settles.
    let mut report = None;
    let mut outcome = String::from("ran out of frames while the barrier was still un-Ready");
    for _ in 0..600 {
        step(&mut app);
        let state = app
            .world()
            .resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>(
        );
        let Some(active) = state.active.as_ref() else {
            outcome = "the transition finished and released".into();
            break;
        };
        if active.asset_readiness_complete {
            outcome = format!(
                "the barrier reached readiness (phase={:?}, progress={:?})",
                active.phase, active.last_asset_progress
            );
            break;
        }
        if let Some(explained) = active.asset_stall_report.clone() {
            report = Some(explained);
            break;
        }
        outcome = format!(
            "phase={:?} progress={:?} since={:?} complete={}",
            active.phase,
            active.last_asset_progress,
            active.asset_progress_since,
            active.asset_readiness_complete
        );
    }
    // Read them from the run output when the burst is what you are studying.
    {
        let state = app
            .world()
            .resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>(
        );
        if let Some(active) = state.active.as_ref() {
            println!(
                "[hall-transition] preflight={:?} manifest={:?} barrier={:?} prefetch_hit={}",
                active.construction_preflight_duration,
                active.asset_manifest_duration,
                active.last_asset_progress,
                active.prefetch_hit,
            );
        }
    }

    let report = report.unwrap_or_else(|| {
        panic!(
            "no stall explanation, and the loop exited because: {outcome}.\n{}",
            "the Hall's asset barrier sat un-Ready for the whole test and the transition \
         never produced an explanation. `RoomAssetReadiness::pending` names every \
         outstanding asset on every poll; if this is `None`, those names are being \
         computed and dropped again and a stuck load is back to reporting 99%",
        )
    });
    // and the explanation has to NAME things. A stall report that says only
    // "still waiting" is the 99% problem with more words.
    assert!(
        report.contains("Still pending:") && report.contains("hall_of_characters"),
        "the stall report does not name the room and its outstanding assets: {report}"
    );
}
