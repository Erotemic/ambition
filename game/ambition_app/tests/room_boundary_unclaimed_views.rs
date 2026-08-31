//! Crossing a room boundary must not leave unclaimed feature views behind.
//!
//! `UnclaimedFeatureViews` is the immediate census used by the transition cover;
//! persistent unclaimed views later receive magenta placeholders. This test uses
//! the real room-unload path and compares against the destination room's clean
//! population so leaked source-room views cannot hide in the baseline.

use std::collections::BTreeSet;
use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use ambition_app::app::{build_visible_app, shell_host, VisibleRenderMode};
use ambition_platformer2d::combat::components::ActorDisposition;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::render::rendering::{FeatureVisual, UnclaimedBodyPlaceholder};
use ambition_platformer2d::world::rooms::RoomSet;

/// The hub the Ambition gameplay route opens in.
const HUB_ROOM: &str = "central_hub_complex";
/// A room with a cast to defeat and a two-way door back to the hub. Named
/// rather than "any room with enemies" so this fails loudly if the route stops
/// existing, instead of quietly measuring a crossing that carries nothing.
const COMBAT_ROOM: &str = "proving_grounds";
/// The hub's authored door into the combat room, and the combat room's back.
const HUB_TO_COMBAT_ZONE: &str = "proving_grounds_hub_door";
/// Into the **Hall of Characters** — 129 bodies, the heaviest room the app has.
/// ⭐ the flash test needs this one: the hub→combat crossing draws **no
/// unclaimed placeholder at all**, so it cannot say
/// anything about a cover hiding one.
const HUB_TO_HALL_ZONE: &str = "hall_of_characters_door";
const COMBAT_TO_HUB_ZONE: &str = "proving_grounds_to_hub";

/// Frames a crossing may take to change the active room before we call it wedged.
/// A liveness backstop, never a measurement.
const CROSSING_CAP: usize = 900;
/// Frames a settle may take before we stop waiting for the population to go
/// quiet. The cover's own give-up deadline is 8s, which is 480 frames here.
const SETTLE_CAP: usize = 700;
/// Consecutive unchanged frames that count as settled.
const QUIET_FRAMES: usize = 40;

/// Step one frame, with a sliver of wall clock so the asset threads make
/// progress — a room whose textures never decode never draws its visuals, and
/// this test would then be measuring the asset pipeline.
fn step(app: &mut App) {
    app.update();
    std::thread::sleep(Duration::from_millis(4));
}

/// Step one frame without sleeping for asset threads.
/// This keeps transient unclaimed views observable; [`step`] may let them settle first.
fn step_fast(app: &mut App) {
    app.update();
}

fn active_room(app: &mut App) -> String {
    let mut query = app.world_mut().query::<&RoomSet>();
    let world = app.world();
    query
        .iter(world)
        .next()
        .map(|set| set.rooms[set.active].id.clone())
        .unwrap_or_default()
}

/// The ids the unclaimed floor is currently standing in for.
fn stand_in_ids(app: &mut App) -> BTreeSet<String> {
    let mut query = app
        .world_mut()
        .query_filtered::<&FeatureVisual, With<UnclaimedBodyPlaceholder>>();
    let world = app.world();
    query.iter(world).map(|visual| visual.id.clone()).collect()
}

/// Step until the stand-in population stops changing (or the cap runs out), and
/// report what it settled to.
fn settle(app: &mut App) -> BTreeSet<String> {
    let mut last = stand_in_ids(app);
    let mut quiet = 0;
    for _ in 0..SETTLE_CAP {
        step(app);
        let now = stand_in_ids(app);
        if now == last {
            quiet += 1;
            if quiet >= QUIET_FRAMES {
                return now;
            }
        } else {
            last = now;
            quiet = 0;
        }
    }
    last
}

/// Is a gameplay session actually LIVE — a body in a room — rather than merely
/// a room set sitting in a dormant sandbox?
///
/// ⛔ **this distinction cost two runs.** A `RoomSet` naming the hub exists long
/// before the shell activates the session, so "the active room is the hub" is
/// true on the frame after `GoTo` and means nothing. A `RoomTransitionRequested`
/// written then is read by nobody — the transaction never even opens — and the
/// crossing silently does not happen.
fn gameplay_is_live(app: &mut App) -> bool {
    let scoped = app
        .world()
        .get_resource::<ambition_platformer2d::platformer::lifecycle::ActiveSessionScope>()
        .and_then(|scope| scope.current())
        .is_some();
    let mut query = app
        .world_mut()
        .query_filtered::<&BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    let world = app.world();
    scoped && query.iter(world).next().is_some()
}

/// Boot the shipped app, walk it to the gameplay route, and land in the hub with
/// a live session.
fn gameplay_app() -> App {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    // ⛔ **a windowless host cannot settle a room's ASSET barrier, and a
    // transition that never commits crosses no boundary.** Measured here: the
    // transaction opens, reaches `AwaitingReadiness`, and sits at
    // `asset_progress = (0, 33)` for 900 frames — the same dead end
    // `hall_transition_cover` records in its own header ("`NoWindow` decodes
    // almost nothing … so the barrier here waits on handles that never settle").
    //
    // `RoomTransitionAssetContributor` is exactly the switch for that: its own
    // doc says *"Absent ⇒ the engine skips it as it always did for headless"*,
    // and a windowless test host IS headless. Removing it takes the honest
    // headless path instead of pretending an asset loaded. Nothing this file
    // asserts lives on that side of the barrier — the room still unloads, the
    // `RoomScopedEntity` sweep still runs, the room-visual respawn still runs,
    // and the unclaimed floor still draws — a room simply commits with fallback
    // art rather than waiting for textures no rasterizer will ever ask for.
    app.world_mut()
        .remove_resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionAssetContributor>();
    for _ in 0..ambition_app::app::shared_host_startup_ticks() * 2 {
        step(&mut app);
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    for _ in 0..CROSSING_CAP {
        step(&mut app);
        if gameplay_is_live(&mut app) && active_room(&mut app) == HUB_ROOM {
            return app;
        }
    }
    panic!(
        "the app never reached a LIVE session in `{HUB_ROOM}` on the gameplay \
         route (last saw room '{}', session live = {}), so no crossing can be \
         driven",
        active_room(&mut app),
        gameplay_is_live(&mut app),
    );
}

/// Cross an authored loading zone the way `hall_transition_cover` does —
/// resolved through the live room graph, not synthesised — and return the room
/// it landed in.
fn cross(app: &mut App, zone_id: &str) -> String {
    cross_observing(app, zone_id, &mut |_| {})
}

/// [`cross`], with a per-frame observer.
///
/// A test that wants to know "was the cover ever actually up" cannot ask afterwards.
fn cross_observing(app: &mut App, zone_id: &str, observe: &mut dyn FnMut(&mut App)) -> String {
    cross_observing_with(app, zone_id, observe, step)
}

/// [`cross_observing`], with the frame driver chosen by the caller.
///
/// ⛔ **the driver is not an implementation detail here, it is the experiment.**
/// [`step`] sleeps 4 ms per frame *"so the asset threads make progress"*, which
/// is right for a test about a SETTLED population — and structurally fatal for a
/// test about a TRANSIENT one, because it hands the decode exactly the time the
/// unclaimed window needs to not exist. See
/// [`no_magenta_placeholder_is_visible_while_the_cover_is_down`].
fn cross_observing_with(
    app: &mut App,
    zone_id: &str,
    observe: &mut dyn FnMut(&mut App),
    drive: fn(&mut App),
) -> String {
    let from = active_room(app);
    let (target_room, arrival) = {
        let world = app.world_mut();
        let mut query = world.query::<&RoomSet>();
        let room_set = query
            .iter(world)
            .next()
            .expect("gameplay has a session room set");
        let zone = room_set
            .active_loading_zones()
            .iter()
            .find(|zone| zone.id == zone_id || zone.name == zone_id)
            .unwrap_or_else(|| {
                panic!(
                    "the active room '{from}' has no `{zone_id}` loading zone, so \
                     this test cannot drive a real crossing"
                )
            })
            .clone();
        let transition = room_set
            .transition_for_player(zone.aabb, ae::Vec2::ZERO, true)
            .unwrap_or_else(|| panic!("`{zone_id}` does not resolve to a transition"));
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
            With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        q.iter(world)
            .next()
            .expect("the room has a primary avatar to send across its boundary")
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
    let mut trace: Vec<String> = Vec::new();
    let mut last = String::new();
    for frame in 0..CROSSING_CAP {
        drive(app);
        observe(app);
        let report = transaction_report(app);
        if report != last {
            trace.push(format!("  frame {frame}: {report}"));
            last = report;
        }
        let now = active_room(app);
        if now != from {
            return now;
        }
    }
    panic!(
        "crossing `{zone_id}` out of '{from}' never changed the active room \
         within {CROSSING_CAP} frames. Transaction trace:\n{}",
        trace.join("\n")
    );
}

/// What the room-transition transaction thinks it is doing — so a wedged
/// crossing says WHICH stage is holding rather than "it did not happen".
fn transaction_report(app: &mut App) -> String {
    let state = app
        .world()
        .get_resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>();
    match state.and_then(|state| state.active.as_ref()) {
        Some(active) => format!(
            "seq={} {} -> {} phase={:?} failure={:?} cover_required={} cover_presented={} \
             assets_complete={} asset_progress={:?} plan={} committed_at={:?}",
            active.sequence,
            active.source_room_id,
            active.target_room_id,
            active.phase,
            active.failure,
            active.cover_required,
            active.cover_presented,
            active.asset_readiness_complete,
            active.last_asset_progress,
            active.construction_plan.is_some(),
            active.committed_at,
        ),
        None => "no active transition at all (the request was never taken up)".to_string(),
    }
}

/// Defeat every hostile body standing in the active room, through the real
/// damage channel, and report their ids.
///
/// A hand-zeroed health bar proves nothing here: the death CONSEQUENCE — the
/// drops, which are what outlive the room — lives inside the damage pass, so the
/// hit has to be a real `HitEvent`.
fn defeat_the_rooms_hostiles(app: &mut App) -> Vec<String> {
    let victims: Vec<(Entity, String, ae::Vec2)> = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &FeatureId, &ActorDisposition, &BodyKinematics)>();
        query
            .iter(world)
            .filter(|(_, _, disposition, _)| disposition.is_hostile())
            .map(|(entity, id, _, kin)| (entity, id.as_str().to_string(), kin.pos))
            .collect()
    };
    for (entity, _, pos) in &victims {
        let volume: ae::CombatVolume = ae::Aabb::new(*pos, ae::Vec2::new(48.0, 48.0)).into();
        app.world_mut().write_message(HitEvent {
            strike_sfx: None,
            volume,
            damage: 9_999,
            // Attacker-side, so the feature/actor drain consumes it; the victim
            // is pre-resolved, so the volume only has to be somewhere sane.
            source: HitSource::Projectile,
            attacker: None,
            target: HitTarget::Body(*entity),
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
    }
    victims.into_iter().map(|(_, id, _)| id).collect()
}

/// How many features the sim is publishing views for right now. Used only to
/// prove the defeats actually MINTED something — a crossing that carries nothing
/// is a crossing this test learns nothing from.
fn published_view_ids(app: &mut App) -> BTreeSet<String> {
    app.world()
        .resource::<ambition_platformer2d::sim_view::FeatureViewIndex>()
        .iter()
        .map(|(id, _)| id.to_string())
        .collect()
}

/// Is the opaque transition cover on screen right now?
///
/// ⚠ **matched on the debug `Name`, because `RoomTransitionCoverRoot` is
/// private** and widening it for a test would put a presentation marker in the
/// app's public surface. The name is authored one line below the marker
/// (`"room transition cover {sequence}"`), so the two move together — but this
/// IS a coupling to a string, and if it ever silently returns `false` the
/// assertion below goes vacuous in the reassuring direction. The companion test
/// is what stops that.
fn cover_is_up(app: &mut App) -> bool {
    let mut query = app.world_mut().query::<&Name>();
    let world = app.world();
    query
        .iter(world)
        .any(|name| name.as_str().starts_with("room transition cover"))
}

/// Sample frames after a room change and fail if an unclaimed placeholder is
/// visible after the transition cover retires.
///
/// Ignored because this harness currently produces no unclaimed placeholders; the
/// `saw_placeholders` guard prevents a vacuous pass. Do not tune
/// `presentation_settle_deadline` from this test until it observes the transient.
#[test]
#[ignore = "cannot observe the magenta flash: this harness draws no unclaimed placeholder at all — see the doc comment, queue D46"]
fn no_magenta_placeholder_is_visible_while_the_cover_is_down() {
    let mut app = gameplay_app();
    settle(&mut app);

    let before = active_room(&mut app);

    // ⛔ **THE TWO FACTS THAT KEEP THIS FROM BEING VACUOUS.** A green here means
    // nothing unless the cover was ACTUALLY OBSERVED UP (else `cover_is_up`'s
    // name match is broken and the guard can never fire) and placeholders were
    // ACTUALLY DRAWN at some point (else there was never anything to expose).
    // Both are asserted below.
    let mut saw_cover_up = false;
    let mut saw_placeholders = false;
    let mut exposed: Vec<String> = Vec::new();
    let mut frame = 0usize;

    let mut sample = |app: &mut App| {
        let up = cover_is_up(app);
        let stand_ins = stand_in_ids(app);
        saw_cover_up |= up;
        saw_placeholders |= !stand_ins.is_empty();
        if !stand_ins.is_empty() && !up {
            exposed.push(format!(
                "  frame {frame}: {} uncovered — {:?}",
                stand_ins.len(),
                stand_ins
            ));
        }
        frame += 1;
    };

    let after = cross_observing_with(&mut app, HUB_TO_HALL_ZONE, &mut sample, step_fast);

    // Keep sampling well past the commit: the placeholders that matter are the
    // ones a late `Commands` flush spawns once the cover has already gone.
    for _ in 0..SETTLE_CAP {
        step_fast(&mut app);
        sample(&mut app);
    }
    drop(sample);

    assert_ne!(before, after, "the crossing did not change rooms");
    assert!(
        saw_cover_up,
        "the transition cover was never observed during a real crossing, so the \
         assertion below could not have failed. Either `cover_is_up`'s name match \
         (`\"room transition cover\"`) has drifted from what \
         `room_transition_presentation.rs` spawns, or this crossing gets no cover \
         at all — the module doc says only a VISIBLE transition gets one. Fix the \
         handle before trusting a green here."
    );
    assert!(
        saw_placeholders,
        "no unclaimed-body placeholder was drawn at any point in this crossing, so \
         there was never anything for the cover to hide and this test proves \
         nothing about flashes. If the room genuinely resolves all its art \
         same-frame now, pick a heavier room."
    );

    assert!(
        exposed.is_empty(),
        "a magenta unclaimed-body placeholder was on screen with NO transition \
         cover over it, entering '{after}':\n{}\n\n\
         That is the flash Jon reported. ⛔ Do NOT lengthen \
         `presentation_settle_deadline`: the cover has already legitimately \
         retired by then, and a longer deadline cannot reach this. Since \
         2026-08-09 the cover waits on the `UnclaimedFeatureViews` census rather \
         than on this diagnostic's population, so if this fires the census went \
         empty while a view was still undrawn — check the ordering edge \
         (`the_room_transition_cover_is_ordered_after_the_unclaimed_census`) \
         before anything else. See queue D46.",
        exposed.join("\n")
    );
}

/// **⛔⛔ THE COVER MUST READ THIS FRAME'S CENSUS, NOT LAST FRAME'S.**
///
/// ⚠ **and `.after` across SCHEDULES is vacuous in Bevy**, which is why the
/// non-vacuity clauses below matter more than the edge itself: they assert both
/// ends have members *in `Update`*. `632ecf1b4` recorded a cross-schedule
/// warning for this exact pair of sets that turned out to be wrong, and the
/// wrong warning cost the next person the fix — so this test asserts where the
/// members ARE rather than reasoning from a set's name.
#[test]
fn the_room_transition_cover_is_ordered_after_the_unclaimed_census() {
    use ambition_app::app::{Platformer2dSimulationPhaseMonolith, RoomTransitionCoverSet};
    use bevy::ecs::schedule::{NodeId, Schedules, SystemSet};

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);

    // `systems_in_set` answers only once the graph has been BUILT; an unbuilt
    // one reports `Uninitialized` rather than an empty set, which is the good
    // failure direction. Build it without running anything.
    app.world_mut()
        .resource_scope(|world, mut schedules: Mut<Schedules>| {
            schedules
                .get_mut(Update)
                .expect("the app registers systems in Update")
                .initialize(world)
                .expect("the Update schedule builds");
        });

    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(Update).expect("Update exists").graph();

    let census_members = graph
        .systems_in_set(Platformer2dSimulationPhaseMonolith::PresentationVisualSync.intern())
        .expect("PresentationVisualSync is a registered set")
        .len();
    assert!(
        census_members > 0,
        "PresentationVisualSync has NO members in `Update`, so the ordering edge \
         below is silently vacuous and the cover reads whatever the resource \
         happened to hold. The census is published by \
         `draw_unclaimed_feature_views`, the tail of that chain."
    );
    let cover_members = graph
        .systems_in_set(RoomTransitionCoverSet.intern())
        .expect("RoomTransitionCoverSet is a registered set")
        .len();
    assert_eq!(
        cover_members, 1,
        "RoomTransitionCoverSet must hold exactly the cover driver in `Update`. \
         Zero means the ordering constrains nothing; more than one means \
         something else joined the seam and this test stopped naming the cover."
    );

    let census_key = graph
        .system_sets
        .get_key(Platformer2dSimulationPhaseMonolith::PresentationVisualSync.intern())
        .expect("PresentationVisualSync must be a registered SystemSet");
    let cover_key = graph
        .system_sets
        .get_key(RoomTransitionCoverSet.intern())
        .expect("RoomTransitionCoverSet must be a registered SystemSet");
    assert!(
        graph
            .dependency()
            .graph()
            .contains_edge(NodeId::Set(census_key), NodeId::Set(cover_key)),
        "the `Update` dependency graph must carry PresentationVisualSync -> \
         RoomTransitionCoverSet. Without it the cover reads a census published \
         LAST frame: a stale zero retires it over art that has not arrived, \
         which is the magenta flash (queue D46), and nothing about that failure \
         is loud — it is one frame of the wrong number."
    );
}

#[test]
fn crossing_a_room_boundary_leaves_no_repeating_unclaimed_population() {
    let mut app = gameplay_app();
    // The hub's OWN settled population, before anything has been dropped anywhere.
    let hub_clean = settle(&mut app);

    // Into the room with a cast, and let it draw itself.
    let landed = cross(&mut app, HUB_TO_COMBAT_ZONE);
    assert_eq!(
        landed, COMBAT_ROOM,
        "`{HUB_TO_COMBAT_ZONE}` no longer leads to `{COMBAT_ROOM}`"
    );
    let combat_clean = settle(&mut app);

    // Kill everything standing there, so the crossings below carry whatever the
    // death path minted — coins, hearts, dropped weapons, split offspring.
    let published_before = published_view_ids(&mut app);
    let victims = defeat_the_rooms_hostiles(&mut app);
    assert!(
        !victims.is_empty(),
        "`{COMBAT_ROOM}` staged no hostile body, so nothing died and this test \
         carries nothing across a room boundary"
    );
    let after_drops = settle(&mut app);
    let published_after = published_view_ids(&mut app);
    let minted: BTreeSet<&String> = published_after.difference(&published_before).collect();
    assert!(
        !minted.is_empty(),
        "defeating {} hostile bodies in `{COMBAT_ROOM}` published no new feature \
         view at all, so the death path minted nothing for a crossing to carry \
         and this test is measuring an empty room. Victims: {victims:?}",
        victims.len(),
    );

    // ── What the death path minted must be DRAWN, where it fell ─────────────
    //
    // Before any boundary is involved. A drop is a feature the room-load visual
    // pass could not have seen — it did not exist when the room loaded — so the
    // only pass that can draw it is the dynamic-discovery one, and that pass
    // selects loot by construction provenance. Both drop spawners stamped none,
    // so nothing claimed a coin and the player collected the magenta diagnostic
    // box `draw_unclaimed_feature_views` puts under everything.
    //
    // ⚠ **stated against the MINTED ids, not against a count.** The population
    // is re-spawned per room, so "it grew" is answerable by a room that simply
    // has more in it; what is wrong is specifically that a thing this death path
    // published is a thing nothing drew.
    let unclaimed_drops: Vec<&&String> = minted
        .iter()
        .filter(|id| after_drops.contains(**id))
        .collect();
    assert!(
        unclaimed_drops.is_empty(),
        "{} of the {} feature views the death path published in `{COMBAT_ROOM}` \
         are UNDRAWN in the room they fell in: {unclaimed_drops:?}\n\
         \n\
         No render family claimed them, so each one is a magenta \
         `UnclaimedBodyPlaceholder` the player walks over and collects as if it \
         were the coin it stands in for — and the room-transition cover holds the \
         screen black until none remain.\n\
         minted by {} defeats: {minted:?}\n\
         stand-ins now: {after_drops:?} ('{COMBAT_ROOM}' settles at {} with \
         nothing dropped in it)\n\
         bodies defeated to produce this: {victims:?}",
        unclaimed_drops.len(),
        minted.len(),
        victims.len(),
        combat_clean.len(),
    );
    println!(
        "clean stand-ins: {HUB_ROOM}={} {hub_clean:?} | {COMBAT_ROOM}={} \
         {combat_clean:?}; after {} defeats in {COMBAT_ROOM}={} {after_drops:?}",
        hub_clean.len(),
        combat_clean.len(),
        victims.len(),
        after_drops.len(),
    );

    // ── Crossing one ────────────────────────────────────────────────────────
    let landed = cross(&mut app, COMBAT_TO_HUB_ZONE);
    assert_eq!(landed, HUB_ROOM);
    let after_first = settle(&mut app);
    check_crossing(
        COMBAT_ROOM,
        HUB_ROOM,
        &hub_clean,
        &after_drops,
        &after_first,
        &victims,
    );

    // ── Crossing two, immediately ───────────────────────────────────────────
    //
    // Once is a race; twice is a population.
    let landed = cross(&mut app, HUB_TO_COMBAT_ZONE);
    assert_eq!(landed, COMBAT_ROOM);
    let after_second = settle(&mut app);
    check_crossing(
        HUB_ROOM,
        COMBAT_ROOM,
        &combat_clean,
        &after_first,
        &after_second,
        &victims,
    );
}

/// The whole assertion, applied to one crossing.
///
/// `clean` is the DESTINATION room's own settled population from a visit with
/// nothing dropped in it. Measuring against the source room's population instead
/// would be much weaker — see the note in the header about the drops being
/// unclaimed in their own room too.
fn check_crossing(
    from: &str,
    to: &str,
    clean: &BTreeSet<String>,
    before: &BTreeSet<String>,
    after: &BTreeSet<String>,
    victims: &[String],
) {
    let survivors: Vec<&String> = before.intersection(after).collect();
    assert!(
        survivors.is_empty(),
        "{} unclaimed-view stand-in(s) SURVIVED the crossing '{from}' -> '{to}': \
         {survivors:?}\n\
         \n\
         Those ids were undrawn in '{from}' and are STILL undrawn in '{to}', so \
         something is published by the sim on both sides of a room boundary while \
         its picture is room-scoped and died with the room. The floor redraws a \
         magenta stand-in for it in the new room, every crossing, forever — and \
         the room-transition cover holds the screen BLACK until no stand-in \
         remains, which is the 8-second black screen Jon reported on \
         2026-08-05.\n\
         population: before={} after={} ('{to}' settles at {} with nothing \
         dropped in it)\n\
         before: {before:?}\n\
         after:  {after:?}\n\
         bodies defeated to produce this: {victims:?}",
        survivors.len(),
        before.len(),
        after.len(),
        clean.len(),
    );
    assert!(
        after.len() <= clean.len(),
        "the unclaimed-view population did not RETURN TO BASELINE across the \
         crossing '{from}' -> '{to}': {} stand-in(s) in '{to}' now, against the {} \
         that room settles at with nothing dropped in it (and {} on the near side \
         of this crossing). A population that grows per transition is a spawn path \
         whose entity and whose picture have different lifetimes; the cover waits \
         on exactly these, so it holds the screen black to its 8-second \
         deadline.\n\
         over baseline: {:?}\n\
         after: {after:?}\n\
         bodies defeated to produce this: {victims:?}",
        after.len(),
        clean.len(),
        before.len(),
        after.difference(clean).collect::<Vec<_>>(),
    );
}
