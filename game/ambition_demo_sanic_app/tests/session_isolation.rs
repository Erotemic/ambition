//! Sequential-session isolation gate (session-root exclusivity).
//!
//! This drives the REAL Sanic host (`build_demo_app`: foundation + engine + host
//! + shell + the Sanic provider) headlessly. The player body is
//! `simulation_world`'s real output; teardown is the shell's real
//! `SessionScopeRetired` sweep plus the provider-installed
//! `SessionTeardownPlugin` that resets the session-scoped resource mirrors.

use bevy::prelude::*;

use ambition_demo_sanic_app::build_demo_app;
use ambition_platformer2d::actors::control::possession::PossessionState;
use ambition_platformer2d::game_shell::{ShellCommand, ShellLauncherCommand, ShellRouter};
use ambition_platformer2d::platformer::lifecycle::{
    ActiveSessionScope, SessionScopeId, SessionScopedEntity,
};
use ambition_platformer2d::platformer::markers::ControlledSubject;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::collision::MovingPlatformSet;
use ambition_platformer2d::world::platforms::MovingPlatformState;

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

fn active_route(app: &App) -> Option<String> {
    app.world()
        .resource::<ShellRouter>()
        .active
        .as_ref()
        .map(|active| active.route_id.as_str().to_owned())
}

fn live_scope(app: &App) -> Option<SessionScopeId> {
    app.world().resource::<ActiveSessionScope>().current()
}

fn session_scoped_entities(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&SessionScopedEntity>();
    q.iter(app.world()).count()
}

fn primary_player(app: &mut App) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryPlayer>>();
    let mut it = q.iter(app.world());
    let first = it.next();
    // Exactly one, or none.
    assert!(it.next().is_none(), "more than one primary player is live");
    first
}


#[test]
fn a_second_session_shares_no_entity_handle_cache_or_view_with_the_first() {
    let mut app = build_demo_app();
    settle(&mut app);

    // ── Session A is live ──────────────────────────────────────────────────
    assert_eq!(active_route(&app), Some("sanic_gameplay".to_owned()));
    let scope_a = live_scope(&app).expect("a session is live during gameplay");
    let player_a = primary_player(&mut app).expect("session A has a home avatar");

    // Populate the session-scoped resource MIRRORS, and the live room's
    // platforms, with distinctive session-A live state. These are exactly the process-global handles the entity sweep
    // does NOT touch, so seeding them proves teardown — not the sweep — clears
    // them. Using the real player entity makes each a genuine dangling handle
    // the instant the sweep despawns it.
    app.world_mut().resource_mut::<PossessionState>().possessed = Some(player_a);
    app.world_mut().resource_mut::<ControlledSubject>().0 = Some(player_a);
    ambition_platformer2d::session::sole_live_room_component_mut::<MovingPlatformSet>(app.world_mut()).expect("the live room has moving platforms")
        .0
        .push(MovingPlatformState::from_authored(
            ambition_platformer2d::engine_core::Vec2::new(1.0, 2.0),
            ambition_platformer2d::engine_core::Vec2::new(16.0, 4.0),
            32.0,
            20.0,
        ));

    // ── Tear session A down through the supported lifecycle ────────────────
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("sanic_launcher".to_owned()));

    // At the launcher, with A retired and B not yet activated, NOTHING may
    // refer to the retired scope — not entities, and not the resource mirrors.
    assert_eq!(
        session_scoped_entities(&mut app),
        0,
        "a session-scoped entity survived teardown"
    );
    assert_eq!(
        primary_player(&mut app),
        None,
        "the home avatar survived teardown"
    );
    assert_eq!(
        live_scope(&app),
        None,
        "a scope is still live at the launcher"
    );

    // The platforms are on session A's live room root, which leaves with A.
    assert!(
        ambition_platformer2d::session::sole_live_room_component::<MovingPlatformSet>(app.world())
            .is_none(),
        "session A's live room, and its platform state, survived to the launcher"
    );
    assert_eq!(
        app.world().resource::<PossessionState>().possessed,
        None,
        "PossessionState still points at the despawned session-A body"
    );
    assert_eq!(
        app.world().resource::<ControlledSubject>().0,
        None,
        "ControlledSubject still names the despawned session-A body \
         (the sim sleeps at the launcher, so only teardown can clear it)"
    );

    // ── Activate session B (a fresh scope for the same provider) ───────────
    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("sanic_gameplay".to_owned()));

    let scope_b = live_scope(&app).expect("a fresh session is live after relaunch");
    assert_ne!(
        scope_a, scope_b,
        "relaunch reused the retired session scope"
    );

    let player_b = primary_player(&mut app).expect("session B has a home avatar");
    assert_ne!(
        player_a, player_b,
        "session B reused session A's home-avatar entity"
    );

    // The controlled subject belongs to the NEW session, rediscovered from B's
    // player brain — not the stale A handle.
    assert_eq!(
        app.world().resource::<ControlledSubject>().0,
        Some(player_b),
        "the controlled subject does not name session B's home avatar"
    );
    // No mirror carries session-A state into B.
    assert_eq!(
        app.world().resource::<PossessionState>().possessed,
        None,
        "session B inherited session A's possession handle"
    );
    // Session B's live room root carries B's own platforms (no authored
    // platforms in the Sanic demo), so the session-A probe platform is gone.
    assert!(
        ambition_platformer2d::session::sole_live_room_component::<MovingPlatformSet>(app.world()).expect("the live room has moving platforms").0.is_empty(),
        "session B inherited session A's moving-platform state"
    );
}

/// The occurrence ledger and its three checkpoint baselines die with the world
/// they describe.
///
/// ⭐ WHAT WAS ALREADY TRUE, MEASURED FIRST. Session B does NOT inherit these
/// rows even without the resets this case pins, and the mechanism is the one
/// `retirement_clears_the_save_applied_latch` already documents: retirement
/// clears `SaveRestored`, so B re-runs its restore, and `adopt_rows` REPLACES
/// rather than merges — an empty file empties all four. Verified 2026-08-31 by
/// poisoning all four resets and dropping the launcher assertions below: the
/// post-relaunch arm stayed green. The planning row that sent me here was
/// written off a schedule reading and was wrong about the consequence.
///
/// ⛔ SO THE ASSERTION THAT IS ACTUALLY LOAD-BEARING IS THE LAUNCHER ONE, and
/// that is why it comes first. Between retirement and B's restore, these four
/// resources still described a world that no longer exists — dangling in exactly
/// the way the module doc calls hygiene. Each of the four resets is red on that
/// assertion when poisoned alone.
///
/// ⭐ AND IT BUYS ONE THING BEYOND HYGIENE: the correctness no longer rests on
/// the SAVE road running. A composition with no durable horizon re-runs no
/// restore, so nothing would have rewritten these; and the incoming candidate's
/// `CandidateDurableHorizon::install` — which seeds the ledger BEFORE the first
/// room is built — runs after this reset, so the two are one ordered pair rather
/// than two opinions.
///
/// ⚠ THE POST-RELAUNCH ASSERTION IS NOT ATTRIBUTABLE to these resets. It is kept
/// because it states the contract a reader cares about, not because it covers
/// this change.
///
/// ⭐ SEEDED, LIKE EVERY OTHER MIRROR IN THIS FILE. The Sanic demo authors no
/// occurrence-bearing object of its own, and the question is not how a row gets
/// written — it is whether a row SURVIVES a session boundary it has no business
/// crossing.
#[test]
fn a_second_session_does_not_inherit_the_first_sessions_occurrence_ledger() {
    use ambition_platformer2d::platformer::lifecycle::{
        AuthoredOccurrences, OccurrenceWhereabouts,
    };
    use ambition_platformer2d::platformer::sim_id::SimId;

    let mut app = build_demo_app();
    settle(&mut app);
    let scope_a = live_scope(&app).expect("a session is live during gameplay");

    // A row session A could plausibly have written: an object it carried into
    // some room and put down.
    let probe = SimId::placement("session_isolation_ledger_probe");
    app.world_mut()
        .resource_mut::<AuthoredOccurrences>()
        .adopt_rows(
            [(
                probe.clone(),
                OccurrenceWhereabouts::Placed {
                    room: "somewhere_in_session_a".to_owned(),
                    at: ambition_platformer2d::engine_core::Vec2::new(200.0, 200.0),
                },
            )]
            .into_iter()
            .collect(),
        );
    // ⭐ AND THE THREE CHECKPOINT COPIES OF THE SAME FACTS. They describe the
    // same one world, so they get the same arm — a baseline from the previous
    // session is a baseline for a world that no longer exists. They are the
    // session root's (C03), so they are seeded on A's root.
    {
        let world = app.world_mut();
        let ledger = world.resource::<AuthoredOccurrences>().clone();
        ambition_platformer2d::platformer::lifecycle::session_world_component_mut::<
            ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline,
        >(world)
        .expect("A's root carries the occurrence baseline")
        .adopt(ledger);
        ambition_platformer2d::platformer::lifecycle::session_world_component_mut::<
            ambition_platformer2d::platformer::lifecycle::CustodyBaseline,
        >(world)
        .expect("A's root carries the custody baseline")
        .adopt(
                [(
                    probe.clone(),
                    SimId::placement("session_isolation_custodian"),
                )]
                .into_iter()
                .collect(),
            );
        ambition_platformer2d::platformer::lifecycle::session_world_component_mut::<
            ambition_platformer2d::actors::items::pickup::minted_horizon::MintedItemBaseline,
        >(world)
        .expect("A's root carries the minted-item baseline")
        .adopt(
                [(
                    probe.clone(),
                    ambition_platformer2d::actors::items::pickup::minted_horizon::MintedItemDescription {
                        origin: ambition_platformer2d::platformer::construction::SpawnOrigin::Dynamic {
                            parent: SimId::placement("session_isolation_spawner"),
                            sequence: 0,
                        },
                        held_item: "axe".to_owned(),
                    },
                )]
                .into_iter()
                .collect(),
            );
    }

    // ⛔ THE PREMISE: every seed has to be readable, or "gone later" is vacuous.
    assert!(
        ledger_ids(&app).contains(&probe.as_str().to_owned()),
        "the fixture failed to seed the ledger it is about to ask a session \
         boundary to clear"
    );
    assert_eq!(
        checkpoint_rows_for(&app, &probe),
        [true, true, true],
        "the fixture failed to seed the three checkpoint baselines (occurrence, \
         custody, minted item)"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("sanic_launcher".to_owned()));

    // ⭐ THE DIRECT ASSERTION. At the launcher, with A retired and B not yet
    // activated, nothing may still describe A's world.
    assert!(
        ledger_ids(&app).is_empty(),
        "the occurrence ledger still describes the retired session's world at \
         the launcher: {:?}",
        ledger_ids(&app)
    );
    assert_eq!(
        checkpoint_rows_for(&app, &probe),
        [false, false, false],
        "a checkpoint baseline (occurrence, custody, minted item) still describes \
         the retired session's world at the launcher"
    );

    // ⛔⛔ AND THE FILE IS A SECOND ROAD, WHICH IS NOT A DEFECT. The durable
    // mirror wrote the seeded row into `AmbitionGameSave` while A was live —
    // legitimately, because that is what "continue" is for — and a load adopts
    // the file's rows at activation. Measured: at this point the live ledger is
    // empty and the file holds the row, so leaving it there would test the SAVE
    // road and call it the resource road. Emptying it is what isolates the
    // question this case is about.
    // Clears the PAIR: a custody row whose occurrence row is gone names
    // nothing, so the two empty together.
    app.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .0
        .set_durable_horizon(Vec::new(), Vec::new());

    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("sanic_gameplay".to_owned()));
    let scope_b = live_scope(&app).expect("a fresh session is live after relaunch");
    assert_ne!(
        scope_a, scope_b,
        "relaunch reused the retired session scope, so there is no second \
         session here to inherit anything"
    );

    assert!(
        !ledger_ids(&app).contains(&probe.as_str().to_owned()),
        "session B is standing in a world session A's ledger still describes. A \
         row saying an object is lying in one of A's rooms SUPPRESSES that \
         object when B builds it, so an inherited ledger deletes things from \
         the next session's world. Ledger was {:?}",
        ledger_ids(&app)
    );
    assert_eq!(
        checkpoint_rows_for(&app, &probe),
        [false, false, false],
        "session B was born with session A's checkpoint (occurrence, custody, \
         minted item): its first death restores A's world"
    );
}

/// Whether the live session's occurrence, custody and minted-item checkpoint
/// baselines each hold a row for `id`. No live root holds none.
fn checkpoint_rows_for(
    app: &App,
    id: &ambition_platformer2d::platformer::sim_id::SimId,
) -> [bool; 3] {
    use ambition_platformer2d::platformer::lifecycle::{
        session_world_component, CustodyBaseline, OccurrenceBaseline,
    };
    let world = app.world();
    [
        session_world_component::<OccurrenceBaseline>(world)
            .is_some_and(|baseline| baseline.remembered().whereabouts(id).is_some()),
        session_world_component::<CustodyBaseline>(world)
            .is_some_and(|baseline| baseline.was_carried(id)),
        session_world_component::<
            ambition_platformer2d::actors::items::pickup::minted_horizon::MintedItemBaseline,
        >(world)
        .is_some_and(|baseline| baseline.description_of(id).is_some()),
    ]
}

/// Every identity the live occurrence ledger holds a row for.
fn ledger_ids(app: &App) -> Vec<String> {
    app.world()
        .resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
        .rows()
        .map(|(id, _)| id.as_str().to_owned())
        .collect()
}
