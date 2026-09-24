//! D-C exit check: two mode-scoped rules plugins coexist in one app.
//!
//! This is the demo-hosting seam's contract, executable. Ambition hosts several
//! demos' rulesets in ONE binary; each is awake only inside the rooms its mode
//! tag claims, and the state it owns dies when the player leaves those rooms.
//!
//! Both halves are asserted here:
//!   1. `in_mode("a")`-gated systems do not run while the active room says `b`.
//!   2. `ModeScopedEntity("a")` entities are despawned when the active room's
//!      mode changes away from `a` — while `b`'s entities, and `b`'s own rules,
//!      keep running across that same transition.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{ModeScopedEntity, SpawnScopedExt as _};
use ambition_platformer2d_runtime::{despawn_departed_mode_entities, in_base_mode, in_mode};
use ambition_platformer2d_world::rooms::{RoomMetadata, RoomSet, RoomSpec};

/// How many times each mode's gated rule has run.
#[derive(Resource, Default, Debug, PartialEq, Eq)]
struct RuleTicks {
    a: u32,
    b: u32,
}

/// A demo's rules plugin, in the shape the seam prescribes: ONE system list,
/// and a constructor flag deciding whether it is gated on a mode (hosted inside
/// Ambition) or runs unconditionally (the demo standing alone).
struct DemoRulesPlugin {
    mode: &'static str,
    hosted: bool,
}

impl DemoRulesPlugin {
    fn hosted(mode: &'static str) -> Self {
        Self { mode, hosted: true }
    }
}

impl Plugin for DemoRulesPlugin {
    /// Two hosted demos are two instances of this same fixture type. The seam's
    /// whole claim is that rulesets coexist, so they must not dedup by TypeId.
    fn is_unique(&self) -> bool {
        false
    }

    fn build(&self, app: &mut App) {
        let mode = self.mode;
        let rule = move |mut ticks: ResMut<RuleTicks>| match mode {
            "a" => ticks.a += 1,
            "b" => ticks.b += 1,
            other => panic!("unknown demo mode {other}"),
        };
        if self.hosted {
            app.add_systems(Update, rule.run_if(in_mode(mode)));
        } else {
            app.add_systems(Update, rule);
        }
    }
}

/// One room per mode the tests visit, plus a second room in mode `a`, so a
/// mode change is a real room change of the session's one `RoomSet`.
fn session_rooms() -> RoomSet {
    let room = |id: &str, mode: Option<&str>| {
        let mut spec = RoomSpec::new(
            id,
            ambition_platformer2d_core::World::new(
                id,
                ambition_platformer2d_core::Vec2::splat(64.0),
                ambition_platformer2d_core::Vec2::ZERO,
                Vec::new(),
            ),
        );
        spec.metadata = RoomMetadata {
            mode: mode.map(str::to_string),
            ..Default::default()
        };
        spec
    };
    RoomSet::from_parts_or_panic(
        "base",
        vec![
            room("base", None),
            room("a", Some("a")),
            room("a_second_room", Some("a")),
            room("b", Some("b")),
            room("mary_o", Some("mary_o")),
        ],
        Vec::new(),
    )
}

fn insert_session_rooms(app: &mut App) {
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        session_rooms(),
    );
}

fn enter_room(app: &mut App, id: &str) {
    ambition_platformer2d_shared_tangle::lifecycle::session_world_component_mut::<RoomSet>(
        app.world_mut(),
    )
    .expect("session room set")
    .set_active_by_id(id)
    .expect("the fixture set holds every room the tests visit");
}

fn set_mode(app: &mut App, mode: Option<&str>) {
    enter_room(app, mode.unwrap_or("base"));
}

fn mode_scoped_entities(app: &mut App) -> Vec<String> {
    let mut query = app.world_mut().query::<&ModeScopedEntity>();
    let mut modes: Vec<String> = query
        .iter(app.world())
        .map(|scope| scope.0.clone())
        .collect();
    modes.sort();
    modes
}

/// Both rulesets installed; only the active room's mode is awake. Neither
/// plugin knows the other exists, and neither owns a global state.
fn two_hosted_demos() -> App {
    let mut app = App::new();
    insert_session_rooms(&mut app);
    app.init_resource::<RuleTicks>();
    // The sweep as the engine group schedules it, minus the sim-schedule
    // plumbing this test does not need.
    app.add_systems(Update, despawn_departed_mode_entities);
    app.add_plugins(DemoRulesPlugin::hosted("a"));
    app.add_plugins(DemoRulesPlugin::hosted("b"));
    app
}

#[test]
fn a_mode_gated_rule_runs_only_inside_its_own_mode() {
    let mut app = two_hosted_demos();

    // No world / no mode: the base game. Neither hosted ruleset wakes.
    app.update();
    assert_eq!(
        *app.world().resource::<RuleTicks>(),
        RuleTicks { a: 0, b: 0 }
    );

    set_mode(&mut app, Some("a"));
    app.update();
    app.update();
    assert_eq!(
        *app.world().resource::<RuleTicks>(),
        RuleTicks { a: 2, b: 0 }
    );

    set_mode(&mut app, Some("b"));
    app.update();
    assert_eq!(
        *app.world().resource::<RuleTicks>(),
        RuleTicks { a: 2, b: 1 },
        "mode `a`'s systems must not run while the active room says `b`"
    );

    // Back to the base game: both sleep again. A mode is a room property, not a
    // latch some plugin owns.
    set_mode(&mut app, None);
    app.update();
    assert_eq!(
        *app.world().resource::<RuleTicks>(),
        RuleTicks { a: 2, b: 1 }
    );
}

#[test]
fn leaving_a_mode_despawns_only_that_modes_entities() {
    let mut app = two_hosted_demos();
    set_mode(&mut app, Some("a"));

    // Each hosted ruleset spawns its mode-owner entity.
    app.world_mut().commands().spawn_mode_scoped("a", ());
    app.world_mut().commands().spawn_mode_scoped("b", ());
    let survivor = app.world_mut().commands().spawn(()).id();
    app.world_mut().flush();
    assert_eq!(mode_scoped_entities(&mut app), vec!["a", "b"]);

    // Entering `b` retires `a`'s state and leaves `b`'s standing.
    set_mode(&mut app, Some("b"));
    app.update();
    assert_eq!(
        mode_scoped_entities(&mut app),
        vec!["b"],
        "the departed mode's entities are swept; the entered mode's are not"
    );
    assert!(
        app.world().get_entity(survivor).is_ok(),
        "an unscoped entity is not a mode's to despawn"
    );

    // Returning to the base game retires the last mode too.
    set_mode(&mut app, None);
    app.update();
    assert!(mode_scoped_entities(&mut app).is_empty());
}

/// A room transition WITHIN a mode (metadata changes, mode does not) must not
/// tear the mode's state down — that is the whole difference between a
/// mode-scoped entity and a room-scoped one.
#[test]
fn a_room_change_inside_the_same_mode_spares_the_modes_entities() {
    let mut app = two_hosted_demos();
    set_mode(&mut app, Some("a"));
    app.world_mut().commands().spawn_mode_scoped("a", ());
    app.world_mut().flush();

    enter_room(&mut app, "a_second_room");
    app.update();
    assert_eq!(mode_scoped_entities(&mut app), vec!["a"]);
}

/// `in_base_mode` is the mirror of [`in_mode`]: it wakes a host-only system ONLY
/// when the live session is Ambition's own (an active room with no demo mode tag).
/// This is the gate the inventory/pause toggle needs — it stays asleep on the
/// title screen (no session) AND inside a hosted Sanic/Mary-O session.
#[test]
fn in_base_mode_wakes_only_in_ambitions_own_gameplay() {
    #[derive(Resource, Default)]
    struct ChromeTicks(u32);

    fn count_chrome(mut ticks: ResMut<ChromeTicks>) {
        ticks.0 += 1;
    }

    // A live session in the base game (no mode tag): Ambition's own chrome wakes.
    let mut app = App::new();
    insert_session_rooms(&mut app);
    app.init_resource::<ChromeTicks>();
    app.add_systems(Update, count_chrome.run_if(in_base_mode));

    app.update();
    assert_eq!(
        app.world().resource::<ChromeTicks>().0,
        1,
        "a live session with no mode tag IS Ambition's own gameplay"
    );

    // A hosted demo mode: the host chrome sleeps — the session is the demo's.
    set_mode(&mut app, Some("mary_o"));
    app.update();
    assert_eq!(
        app.world().resource::<ChromeTicks>().0,
        1,
        "a hosted demo mode is not Ambition's mode, so host chrome must not wake"
    );

    // Back to the base game: it wakes again.
    set_mode(&mut app, None);
    app.update();
    assert_eq!(app.world().resource::<ChromeTicks>().0, 2);

    // No session world at all (title screen / frontend): the toggle can never
    // fire — the exact leak this gate closes.
    let mut title = App::new();
    title.init_resource::<ChromeTicks>();
    title.add_systems(Update, count_chrome.run_if(in_base_mode));
    title.update();
    assert_eq!(
        title.world().resource::<ChromeTicks>().0,
        0,
        "no live session ⇒ host chrome cannot open on the title screen"
    );
}

/// The standalone half of the constructor flag: ungated, the demo's rules run
/// everywhere, because when the demo IS the game there is no mode to leave.
#[test]
fn a_standalone_ruleset_runs_with_no_mode_at_all() {
    let mut app = App::new();
    insert_session_rooms(&mut app);
    app.init_resource::<RuleTicks>();
    app.add_plugins(DemoRulesPlugin {
        mode: "a",
        hosted: false,
    });
    app.update();
    assert_eq!(app.world().resource::<RuleTicks>().a, 1);
}
