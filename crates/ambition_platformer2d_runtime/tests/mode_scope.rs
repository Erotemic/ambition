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
    ambition_platformer2d_world::rooms::insert_room_set(app.world_mut(), session_rooms());
}

fn enter_room(app: &mut App, id: &str) {
    ambition_platformer2d_world::rooms::seat_sole_live_room_by_id(app.world_mut(), id)
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

/// A hosted game's per-mode state, in the shape `install_mode_owner` builds.
#[derive(Component, Default, Debug, PartialEq)]
struct ActClock(u32);

fn a_game_with_a_mode_owner() -> App {
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

    let mut app = App::new();
    app.set_sim_schedule(Update);
    insert_session_rooms(&mut app);
    // The sweep and the room follow, as the engine group installs them.
    app.add_plugins(ambition_platformer2d_runtime::ModeScopePlugin);
    ambition_platformer2d_runtime::install_mode_owner(
        &mut app,
        ambition_combat::scoped_rules::RulesScope::Mode("a"),
        "a",
        ActClock::default,
    );
    app
}

fn owners(app: &mut App) -> Vec<(String, ambition_platformer2d_shared_tangle::sim_id::SimId)> {
    let mut query = app.world_mut().query_filtered::<(
        &ModeScopedEntity,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
    ), With<ActClock>>();
    query
        .iter(app.world())
        .map(|(scope, id)| (scope.0.clone(), id.clone()))
        .collect()
}

/// A declared mode owner is born on the first tick its mode is live, once, and
/// the mode sweep retires it when the mode ends. Coming back starts a fresh
/// one. Two games each carried their own copy of this spawner before the
/// engine owned it.
#[test]
fn a_declared_mode_owner_is_born_once_per_visit_to_its_mode() {
    let mut app = a_game_with_a_mode_owner();
    let owner_of_a = || {
        vec![(
            "a".to_string(),
            ambition_platformer2d_shared_tangle::sim_id::SimId::singleton("mode_owner", "a"),
        )]
    };

    app.update();
    assert!(owners(&mut app).is_empty(), "the base game has no owner for mode `a`");

    set_mode(&mut app, Some("a"));
    app.update();
    assert_eq!(owners(&mut app), owner_of_a(), "mode `a` is live and has no owner");
    app.world_mut()
        .query::<&mut ActClock>()
        .single_mut(app.world_mut())
        .expect("one owner")
        .0 = 7;
    app.update();
    app.update();
    assert_eq!(owners(&mut app), owner_of_a(), "a second owner was born for one visit");

    set_mode(&mut app, Some("b"));
    app.update();
    assert!(owners(&mut app).is_empty(), "the owner outlived its mode");

    set_mode(&mut app, Some("a"));
    app.update();
    assert_eq!(owners(&mut app), owner_of_a());
    assert_eq!(
        app.world_mut().query::<&ActClock>().single(app.world()).ok(),
        Some(&ActClock(0)),
        "a new visit must start from the authored state, not the last visit's"
    );
}

/// A standalone game's owner (`EveryRoom`) lives in every room, untagged rooms
/// included. The scope that lets the owner be born is the scope that keeps it:
/// a sweep that read the owner's mode name would retire it in an untagged room,
/// and the next tick would bear a fresh one.
#[test]
fn an_every_room_owner_lives_in_untagged_rooms() {
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

    let mut app = App::new();
    app.set_sim_schedule(Update);
    insert_session_rooms(&mut app);
    app.add_plugins(ambition_platformer2d_runtime::ModeScopePlugin);
    ambition_platformer2d_runtime::install_mode_owner(
        &mut app,
        ambition_combat::scoped_rules::RulesScope::EveryRoom,
        "standalone",
        ActClock::default,
    );
    let owner = || {
        vec![(
            "standalone".to_string(),
            ambition_platformer2d_shared_tangle::sim_id::SimId::singleton("mode_owner", "standalone"),
        )]
    };

    app.update();
    assert_eq!(owners(&mut app), owner(), "the untagged room has no owner");
    app.world_mut()
        .query::<&mut ActClock>()
        .single_mut(app.world_mut())
        .expect("one owner")
        .0 = 7;
    for room in ["b", "base"] {
        enter_room(&mut app, room);
        app.update();
        assert_eq!(owners(&mut app), owner(), "room `{room}`");
        assert_eq!(
            app.world_mut().query::<&ActClock>().single(app.world()).ok(),
            Some(&ActClock(7)),
            "the owner was retired and born again in room `{room}`"
        );
    }
}

/// A mode owner knows the room it is in and whether it has just arrived: its
/// first room on the tick it is born, then each room it comes into. Two games
/// declare owners here, and the engine follows the room once per tick for both.
#[test]
fn a_mode_owner_is_told_when_it_arrives_in_a_room() {
    use ambition_platformer2d_shared_tangle::lifecycle::{Arrival, ModeVisit};

    let mut app = a_game_with_a_mode_owner();
    ambition_platformer2d_runtime::install_mode_owner(
        &mut app,
        ambition_combat::scoped_rules::RulesScope::Mode("b"),
        "b",
        ActClock::default,
    );
    let mut visits = Vec::new();
    for room in ["a", "a", "a_second_room", "a_second_room"] {
        enter_room(&mut app, room);
        app.update();
        let visit = app
            .world_mut()
            .query::<&ModeVisit>()
            .single(app.world())
            .expect("mode `a`'s owner")
            .clone();
        visits.push((visit.room().map(str::to_owned), visit.arrival()));
    }
    let at = |room: &str, arrival| (Some(room.to_owned()), arrival);
    assert_eq!(
        visits,
        [
            at("a", Arrival::First),
            at("a", Arrival::Staying),
            at("a_second_room", Arrival::FromAnotherRoom),
            at("a_second_room", Arrival::Staying),
        ]
    );
}

/// Open live room `instance` beside the sole one, as the room `id` of the
/// fixture set: a second player who went there (OW1).
fn open_another_live_room(
    app: &mut App,
    instance: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    id: &str,
) -> Entity {
    let definition = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<RoomSet>(app.world())
        .and_then(|rooms| rooms.definition_by_id(id))
        .expect("the fixture set holds the room");
    ambition_platformer2d_shared_tangle::lifecycle::spawn_live_room(app.world_mut(), instance, definition)
}

/// OW1: a mode owner follows a live room of its mode while another live room
/// is in another mode. Bob stays in `a` (#0); Alice goes to `base` (#1), then
/// to `a_second_room` (#2); then Bob leaves `a`. The owner stays in `a` while
/// `a` is live, then goes to the other room of its mode. When the follow read
/// the sole live room, a second live room froze the visit: the owner stayed
/// at its `First` arrival, so a game that starts over on each arrival started
/// over on every tick.
#[test]
fn a_mode_owner_follows_its_own_room_beside_another_live_room() {
    use ambition_platformer2d_shared_tangle::lifecycle::{Arrival, LiveRoomInstance, ModeVisit};

    let mut app = a_game_with_a_mode_owner();
    let visit = |app: &mut App| {
        let visit = app
            .world_mut()
            .query::<&ModeVisit>()
            .single(app.world())
            .expect("mode `a`'s owner")
            .clone();
        (visit.room().map(str::to_owned), visit.arrival())
    };
    let at = |room: &str, arrival| (Some(room.to_owned()), arrival);
    enter_room(&mut app, "a");
    app.update();
    assert_eq!(visit(&mut app), at("a", Arrival::First), "precondition: the owner was not born in `a`");
    let alice = LiveRoomInstance::ACTIVATION.next();
    let base = open_another_live_room(&mut app, alice, "base");
    app.update();
    assert_eq!(visit(&mut app), at("a", Arrival::Staying), "with `base` live beside `a`");
    app.world_mut().despawn(base);
    open_another_live_room(&mut app, alice.next(), "a_second_room");
    app.update();
    assert_eq!(visit(&mut app), at("a", Arrival::Staying), "with `a_second_room` live beside `a`");
    let bob = ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_entity(app.world());
    assert!(bob.is_none(), "precondition: two rooms are live");
    let bob = app
        .world_mut()
        .query::<(Entity, &LiveRoomInstance)>()
        .iter(app.world())
        .find(|(_, room)| **room == LiveRoomInstance::ACTIVATION)
        .map(|(entity, _)| entity)
        .expect("Bob's room");
    app.world_mut().despawn(bob);
    app.update();
    assert_eq!(visit(&mut app), at("a_second_room", Arrival::FromAnotherRoom), "after `a` retired");
}

/// OW1: a mode's entities live while any live room is in the mode's scope,
/// and the sweep runs when a live room retires. Bob stays in `a` (#0) and
/// Alice goes to `b` (#1): both modes live. Then Bob leaves: `a` ends. When
/// the sweep read the sole live room, with two rooms live it swept nothing,
/// and a retirement that did not change the room set did not wake it.
#[test]
fn a_mode_ends_when_no_live_room_is_in_it() {
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;

    let mut app = two_hosted_demos();
    set_mode(&mut app, Some("a"));
    app.update();
    app.world_mut().commands().spawn_mode_scoped("a", ());
    app.world_mut().flush();
    open_another_live_room(&mut app, LiveRoomInstance::ACTIVATION.next(), "b");
    app.world_mut().commands().spawn_mode_scoped("b", ());
    app.world_mut().commands().spawn_mode_scoped("mary_o", ());
    app.world_mut().flush();
    app.update();
    assert_eq!(
        mode_scoped_entities(&mut app),
        vec!["a", "b"],
        "with `a` and `b` live, only the mode no live room is in ends"
    );
    let bob = app
        .world_mut()
        .query::<(Entity, &LiveRoomInstance)>()
        .iter(app.world())
        .find(|(_, room)| **room == LiveRoomInstance::ACTIVATION)
        .map(|(entity, _)| entity)
        .expect("Bob's room");
    app.world_mut().despawn(bob);
    app.update();
    assert_eq!(mode_scoped_entities(&mut app), vec!["b"], "after Bob left `a`");
}

/// No owner is born while no session is live (at the launcher), even when the
/// last room's metadata still names the mode.
#[test]
fn no_mode_owner_is_born_while_no_session_is_live() {
    let mut app = a_game_with_a_mode_owner();
    app.insert_resource(ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope::default());
    set_mode(&mut app, Some("a"));
    app.update();
    assert!(
        owners(&mut app).is_empty(),
        "an owner was born with no live session to own it"
    );
}

/// A rule kind that a crate with no view of rooms reads.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Tint(u8);

#[derive(Resource, Default, Debug, PartialEq)]
struct TintInForce(Option<Tint>);

impl From<Option<Tint>> for TintInForce {
    fn from(tint: Option<Tint>) -> Self {
        Self(tint)
    }
}

#[derive(Resource, Default)]
struct TintChanges(u32);

/// The projection follows the active room's rule, and it writes only when the
/// answer changes. A reader that gates on change detection then sees one change
/// per rule change, not one per frame or per room.
#[test]
fn a_room_rule_projects_into_its_read_model_and_changes_only_with_the_answer() {
    use ambition_combat::scoped_rules::{DeclareRulesExt as _, RulesScope};

    fn count(mut changes: ResMut<TintChanges>) {
        changes.0 += 1;
    }

    let mut app = App::new();
    insert_session_rooms(&mut app);
    app.declare_rules(RulesScope::Mode("a"), Tint(3));
    app.init_resource::<TintInForce>();
    app.init_resource::<TintChanges>();
    app.add_systems(
        Update,
        (
            ambition_platformer2d_runtime::project_room_rule::<Tint, TintInForce>,
            count.run_if(resource_changed::<TintInForce>),
        )
            .chain(),
    );
    let seen = |app: &mut App, room: &str| {
        enter_room(app, room);
        app.update();
        app.update();
        (
            app.world().resource::<TintInForce>().0,
            app.world().resource::<TintChanges>().0,
        )
    };

    // `init_resource` is itself a change, so the first frame counts one.
    assert_eq!(
        seen(&mut app, "base"),
        (None, 1),
        "(tint, changes): the base room declares no tint, and a projection that \
         rewrites an unchanged answer counts a change every frame"
    );
    assert_eq!(seen(&mut app, "a"), (Some(Tint(3)), 2), "room `a` reads its game's tint");
    assert_eq!(
        seen(&mut app, "a_second_room"),
        (Some(Tint(3)), 2),
        "a room change inside the same mode must not rewrite the same answer"
    );
    assert_eq!(seen(&mut app, "b"), (None, 3), "the tint followed the player out of its mode");
}

/// "No room" and "a live untagged room" are different facts, and a rule and
/// the gate for its scope agree on both.
///
/// The host's rules (`UntaggedRooms`) govern its live untagged rooms and
/// nothing before a session exists. A standalone game's rules (`EveryRoom`)
/// govern with no room too, so its setup runs under them.
#[test]
fn a_rule_and_its_gate_agree_about_no_room_an_untagged_room_and_a_mode() {
    use ambition_combat::scoped_rules::{DeclareRulesExt as _, RulesScope};
    use ambition_platformer2d_actor_monolith::session::governing_rules::GoverningRules;
    use bevy::ecs::system::RunSystemOnce as _;

    fn read(app: &mut App) -> (Option<u8>, bool, Option<u16>, bool) {
        app.world_mut()
            .run_system_once(
                |host: GoverningRules<u8>,
                 standalone: GoverningRules<u16>,
                 room: ambition_platformer2d_runtime::CurrentRoom| {
                    (
                        host.get(),
                        room.in_scope(RulesScope::UntaggedRooms),
                        standalone.get(),
                        room.in_scope(RulesScope::EveryRoom),
                    )
                },
            )
            .expect("the readers run")
    }

    let mut app = App::new();
    app.declare_rules(RulesScope::UntaggedRooms, 1u8);
    app.declare_rules(RulesScope::Mode("a"), 2u8);
    app.declare_rules(RulesScope::EveryRoom, 7u16);

    assert_eq!(
        read(&mut app),
        (None, false, Some(7), true),
        "(host rule, host gate, standalone rule, standalone gate) with no session: \
         the host's untagged-room rules must not govern before a room exists"
    );
    insert_session_rooms(&mut app);
    set_mode(&mut app, None);
    assert_eq!(read(&mut app), (Some(1), true, Some(7), true), "a live untagged room");
    set_mode(&mut app, Some("a"));
    assert_eq!(read(&mut app).0, Some(2), "a live room of mode `a` reads `a`'s rules");
}
