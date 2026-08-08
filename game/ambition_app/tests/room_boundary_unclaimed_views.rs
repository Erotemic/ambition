//! **Crossing a room boundary must not leave a repeating unclaimed-view
//! population.**
//!
//! Jon, 2026-08-05: his screen stayed black for the room-transition cover's full
//! 8-second give-up deadline, and his log carried eight lines of
//! ``no render family claimed `coin:EnemySpawn-…` `` per transition, repeating
//! forever.
//!
//! The mechanism is a LIFETIME MISMATCH, and it is general. The sim publishes a
//! `FeatureView` for every live feature; each render family discovers its own
//! population; and `draw_unclaimed_feature_views` is the floor beneath them — a
//! view nobody claimed gets a deliberately-ugly magenta stand-in
//! (`UnclaimedBodyPlaceholder`). Those stand-ins are ALSO what the transition
//! cover waits on: `room_transition_presentation` holds the cover until none
//! remain. So any entity that OUTLIVES the room while its picture does not
//! parks a permanent stand-in in every room you walk into afterwards, and the
//! cover sits out its deadline over a black screen, every crossing.
//!
//! `dd73a3087` fixed one instance by giving two enemy drops `RoomScopedEntity`.
//!
//! ⛔ **this is deliberately NOT a test that a coin dies with its room.** That
//! test names the two spawn sites that commit already touched, passes forever,
//! and says nothing about the third site somebody adds next month — the same
//! death path still mints a `GroundItem` weapon under session scope alone, and
//! every future spawner is a fresh chance at the same mismatch. What is worth
//! defending is the OBSERVABLE Jon actually had: cross a
//! boundary and the stand-in population must settle back to where it was, and no
//! single unclaimed id may survive the crossing. A stand-in that follows you into
//! the next room is the defect *whatever entity caused it*.
//!
//! **A drop is unclaimed in the room it FELL in too** — that was a second, live
//! defect when this file landed, and it is now the second thing asserted here.
//! Measured 2026-08-07: `proving_grounds` settled at 0 stand-ins clean and at 8
//! after seven defeats, one per coin and heart, because
//! `rebuild_dynamic_feature_views` selects dropped pickups by
//! `SpawnOrigin::Dynamic` and `drop_currency_coin` stamped no `SpawnOrigin` at
//! all. No family claimed a drop, and the player walked over a magenta box
//! instead of a coin. Fixed 2026-08-08 by giving the drops their provenance.
//!
//! ⚠ that history is also why the baseline below is the DESTINATION room's own
//! clean population rather than the source room's: comparing against a source
//! that is itself carrying stand-ins would let them ride into the next room
//! unnoticed.
//!
//! ⚠ **it needs a real room-unload path, so it pays for the real app.** The
//! drop-path tests build their worlds by hand, which cannot unload a room at all;
//! this drives `build_visible_app` into gameplay and crosses two authored doors
//! the way `hall_transition_cover` does, so the `RoomScopedEntity` sweep, the
//! room-visual respawn and the unclaimed floor are all the shipped articles.

use std::collections::BTreeSet;
use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use ambition_app::app::{build_visible_app, shell_host, VisibleRenderMode};
use ambition_platformer2d::actors::actor::BodyKinematics;
use ambition_platformer2d::actors::combat::components::ActorDisposition;
use ambition_platformer2d::actors::features::FeatureId;
use ambition_platformer2d::actors::rooms::{RoomSet, RoomTransitionRequested};
use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::render::rendering::{FeatureVisual, UnclaimedBodyPlaceholder};

/// The hub the Ambition gameplay route opens in.
const HUB_ROOM: &str = "central_hub_complex";
/// A room with a cast to defeat and a two-way door back to the hub. Named
/// rather than "any room with enemies" so this fails loudly if the route stops
/// existing, instead of quietly measuring a crossing that carries nothing.
const COMBAT_ROOM: &str = "proving_grounds";
/// The hub's authored door into the combat room, and the combat room's back.
const HUB_TO_COMBAT_ZONE: &str = "proving_grounds_hub_door";
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
    let from = active_room(app);
    let transition = {
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
        room_set
            .transition_for_player(zone.aabb, ae::Vec2::ZERO, true)
            .unwrap_or_else(|| panic!("`{zone_id}` does not resolve to a transition"))
    };
    app.world_mut()
        .write_message(RoomTransitionRequested::new(transition, None));
    let mut trace: Vec<String> = Vec::new();
    let mut last = String::new();
    for frame in 0..CROSSING_CAP {
        step(app);
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
            source: HitSource::PlayerProjectile,
            attacker: None,
            target: HitTarget::Actor(*entity),
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

#[test]
fn crossing_a_room_boundary_leaves_no_repeating_unclaimed_population() {
    let mut app = gameplay_app();
    // The hub's OWN settled population, before anything has been dropped
    // anywhere. This is what "returns to baseline" is measured against when we
    // come back to it — not the population of the room the drops happened in.
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
    // Once is a race; twice is a population. The bug Jon hit repeated on EVERY
    // transition, so one crossing could be excused as a slow room and two
    // cannot.
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
/// Two clauses, because they fail on different shapes of the same defect: no
/// unclaimed id may CARRY across the boundary, and the destination's population
/// must come back to what that room settled at before anything was dropped.
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
