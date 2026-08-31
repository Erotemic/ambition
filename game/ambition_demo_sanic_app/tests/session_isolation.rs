//! Sequential-session isolation gate (session-root exclusivity).
//!
//! This drives the REAL Sanic host (`build_demo_app`: foundation + engine + host
//! + shell + the Sanic provider) headlessly. The player body is
//! `simulation_world`'s real output; teardown is the shell's real
//! `SessionScopeRetired` sweep plus the provider-installed
//! `SessionTeardownPlugin` that resets the session-scoped resource mirrors.

use bevy::prelude::*;

use ambition_demo_sanic_app::build_demo_app;
use ambition_platformer2d::actors::abilities::traversal::possession::PossessionState;
use ambition_platformer2d::encounter::EncounterRegistry;
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

const PROBE_ENCOUNTER: &str = "session_isolation_probe";

#[test]
fn a_second_session_shares_no_entity_handle_cache_or_view_with_the_first() {
    let mut app = build_demo_app();
    settle(&mut app);

    // ── Session A is live ──────────────────────────────────────────────────
    assert_eq!(active_route(&app), Some("sanic_gameplay".to_owned()));
    let scope_a = live_scope(&app).expect("a session is live during gameplay");
    let player_a = primary_player(&mut app).expect("session A has a home avatar");

    // Populate the session-scoped resource MIRRORS with distinctive session-A
    // live state. These are exactly the process-global handles the entity sweep
    // does NOT touch, so seeding them proves teardown — not the sweep — clears
    // them. Using the real player entity makes each a genuine dangling handle
    // the instant the sweep despawns it.
    app.world_mut().resource_mut::<PossessionState>().possessed = Some(player_a);
    app.world_mut()
        .resource_mut::<EncounterRegistry>()
        .ids
        .insert(PROBE_ENCOUNTER.to_owned(), player_a);
    app.world_mut().resource_mut::<ControlledSubject>().0 = Some(player_a);
    app.world_mut()
        .resource_mut::<MovingPlatformSet>()
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

    assert!(
        app.world().resource::<MovingPlatformSet>().0.is_empty(),
        "MovingPlatformSet still holds session-A platform state at the launcher"
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
    assert!(
        !app.world()
            .resource::<EncounterRegistry>()
            .ids
            .contains_key(PROBE_ENCOUNTER),
        "EncounterRegistry still maps an id to the dead session-A entity"
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
    assert!(
        !app.world()
            .resource::<EncounterRegistry>()
            .ids
            .contains_key(PROBE_ENCOUNTER),
        "session B inherited session A's encounter index probe"
    );
    // MovingPlatformSet was rebuilt from B's room (no authored platforms in the
    // Sanic demo), so the session-A probe platform is gone.
    assert!(
        app.world().resource::<MovingPlatformSet>().0.is_empty(),
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
/// restore, so nothing would have rewritten these; and
/// `adopt_the_occurrence_ledger_at_activation` — which now seeds the ledger
/// BEFORE the first room is built — runs `.after` this reset, so the two are one
/// ordered pair rather than two opinions.
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
    // same one world and carry the same defect, so they get the same answer and
    // the same arm — a baseline from the previous session is a baseline for a
    // world that no longer exists.
    {
        let world = app.world_mut();
        let ledger = world.resource::<AuthoredOccurrences>().clone();
        world
            .resource_mut::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>()
            .adopt(ledger);
        world
            .resource_mut::<ambition_platformer2d::platformer::lifecycle::CustodyBaseline>()
            .adopt(
                [(
                    probe.clone(),
                    SimId::placement("session_isolation_custodian"),
                )]
                .into_iter()
                .collect(),
            );
        world
            .resource_mut::<ambition_platformer2d::actors::items::pickup::minted_horizon::MintedItemBaseline>()
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
    assert!(
        !app.world()
            .resource::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>()
            .remembered()
            .is_empty(),
        "the fixture failed to seed the occurrence baseline"
    );
    assert!(
        !app.world()
            .resource::<ambition_platformer2d::platformer::lifecycle::CustodyBaseline>()
            .is_empty(),
        "the fixture failed to seed the custody baseline"
    );
    assert!(
        !app.world()
            .resource::<ambition_platformer2d::actors::items::pickup::minted_horizon::MintedItemBaseline>()
            .is_empty(),
        "the fixture failed to seed the minted-item baseline"
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
    assert!(
        app.world()
            .resource::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>()
            .remembered()
            .is_empty(),
        "the occurrence BASELINE still describes the retired session's world"
    );
    assert!(
        app.world()
            .resource::<ambition_platformer2d::platformer::lifecycle::CustodyBaseline>()
            .is_empty(),
        "the custody BASELINE still says who was holding what in the retired \
         session"
    );
    assert!(
        app.world()
            .resource::<ambition_platformer2d::actors::items::pickup::minted_horizon::MintedItemBaseline>()
            .is_empty(),
        "the minted-item BASELINE still says how to rebuild the retired \
         session's runtime mints"
    );

    // ⛔⛔ AND THE FILE IS A SECOND ROAD, WHICH IS NOT A DEFECT. The durable
    // mirror wrote the seeded row into `AmbitionGameSave` while A was live —
    // legitimately, because that is what "continue" is for — and a load adopts
    // the file's rows at activation. Measured: at this point the live ledger is
    // empty and the file holds the row, so leaving it there would test the SAVE
    // road and call it the resource road. Emptying it is what isolates the
    // question this case is about.
    app.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .0
        .occurrences
        .clear();

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
}

/// Every identity the live occurrence ledger holds a row for.
fn ledger_ids(app: &App) -> Vec<String> {
    app.world()
        .resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
        .rows()
        .map(|(id, _)| id.as_str().to_owned())
        .collect()
}
