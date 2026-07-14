use super::*;
use bevy::ecs::system::RunSystemOnce;

fn session_app() -> App {
    let mut app = App::new();
    app.add_plugins(SessionScopePlugin);
    app
}

fn count_scoped(app: &mut App, scope: SessionScopeId) -> usize {
    let mut query = app.world_mut().query::<&SessionScopedEntity>();
    query
        .iter(app.world())
        .filter(|owner| owner.0 == scope)
        .count()
}

fn nested_spawn_helper(commands: &mut Commands, scope: SessionSpawnScope) {
    commands.spawn_session_scoped(scope, Name::new("nested-child"));
}

#[test]
fn direct_and_nested_spawns_share_the_captured_scope() {
    let mut app = session_app();
    let scope = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    let spawn_scope = app.world().resource::<ActiveSessionScope>().spawn_scope();

    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.spawn_session_scoped(spawn_scope, Name::new("root"));
            commands.spawn_session_scoped(spawn_scope, Name::new("sibling"));
            nested_spawn_helper(&mut commands, spawn_scope);
        })
        .unwrap();

    assert_eq!(count_scoped(&mut app, scope), 3);
}

#[test]
fn captured_scope_survives_a_later_ambient_change() {
    let mut app = session_app();
    let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    let captured_a = app.world().resource::<ActiveSessionScope>().spawn_scope();
    let b = app.world_mut().resource_mut::<ActiveSessionScope>().begin();

    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.spawn_session_scoped(captured_a, Name::new("late-a"));
        })
        .unwrap();

    assert_eq!(count_scoped(&mut app, a), 1);
    assert_eq!(count_scoped(&mut app, b), 0);
    assert_eq!(
        app.world().resource::<ActiveSessionScope>().current(),
        Some(b)
    );
}

#[test]
fn one_command_queue_preserves_multiple_captured_owners() {
    let mut app = session_app();
    let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    let captured_a = app.world().resource::<ActiveSessionScope>().spawn_scope();
    let b = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    let captured_b = app.world().resource::<ActiveSessionScope>().spawn_scope();

    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.spawn_session_scoped(captured_a, Name::new("queued-a"));
            commands.spawn_session_scoped(captured_b, Name::new("queued-b"));
        })
        .unwrap();

    assert_eq!(count_scoped(&mut app, a), 1);
    assert_eq!(count_scoped(&mut app, b), 1);
}

#[test]
fn room_session_spawn_carries_both_lifetimes() {
    let mut app = session_app();
    let scope = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.spawn_room_in_session(scope.into(), Name::new("room-feature"));
        })
        .unwrap();

    let mut query = app
        .world_mut()
        .query::<(&RoomScopedEntity, &SessionScopedEntity)>();
    let owners: Vec<_> = query.iter(app.world()).collect();
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].1 .0, scope);
}

#[test]
fn begin_mints_distinct_ids() {
    let mut app = session_app();
    let mut active = app.world_mut().resource_mut::<ActiveSessionScope>();
    let a = active.begin();
    active.clear();
    let b = active.begin();
    let c = active.begin();
    assert_eq!((a.0, b.0, c.0), (0, 1, 2));
}

#[test]
fn retiring_one_scope_is_exact() {
    let mut app = session_app();
    let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.spawn_in_session(a, Name::new("a1"));
            commands.spawn_in_session(a, Name::new("a2"));
        })
        .unwrap();
    let b = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.spawn_in_session(b, Name::new("b1"));
        })
        .unwrap();

    app.world_mut().write_message(SessionScopeRetired(a));
    app.update();

    assert_eq!(count_scoped(&mut app, a), 0);
    assert_eq!(count_scoped(&mut app, b), 1);
}

#[test]
fn retiring_a_stale_scope_leaves_the_live_one_active() {
    let mut app = session_app();
    let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    let b = app.world_mut().resource_mut::<ActiveSessionScope>().begin();

    app.world_mut().write_message(SessionScopeRetired(a));
    app.update();

    assert_eq!(
        app.world().resource::<ActiveSessionScope>().current(),
        Some(b)
    );
}

#[test]
fn retiring_the_live_scope_clears_it() {
    let mut app = session_app();
    let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    app.world_mut().write_message(SessionScopeRetired(a));
    app.update();
    assert_eq!(app.world().resource::<ActiveSessionScope>().current(), None);
}

#[test]
fn optional_session_policy_distinguishes_legacy_from_shell_home() {
    assert_eq!(
        SessionSpawnScope::for_optional_active_session(None),
        Some(SessionSpawnScope::UNSCOPED),
    );

    let mut active = ActiveSessionScope::default();
    assert_eq!(
        SessionSpawnScope::for_optional_active_session(Some(&active)),
        None,
    );
    let scope = active.begin();
    assert_eq!(
        SessionSpawnScope::for_optional_active_session(Some(&active)),
        Some(SessionSpawnScope::scoped(scope)),
    );
}

#[derive(Component, Debug, PartialEq, Eq)]
struct SessionWorldFixture(u32);

#[test]
fn canonical_world_components_share_one_exact_root() {
    let mut app = App::new();
    assert!(session_world_entity(app.world()).is_none());

    let first = insert_session_world_component(app.world_mut(), SessionWorldFixture(3));
    let second = insert_session_world_component(app.world_mut(), Name::new("same root"));
    assert_eq!(first, second);
    assert_eq!(session_world_entity(app.world()), Some(first));
    assert_eq!(
        session_world_component::<SessionWorldFixture>(app.world()),
        Some(&SessionWorldFixture(3)),
    );

    session_world_component_mut::<SessionWorldFixture>(app.world_mut())
        .expect("fixture is on the canonical root")
        .0 = 7;
    assert_eq!(
        session_world_component::<SessionWorldFixture>(app.world()),
        Some(&SessionWorldFixture(7)),
    );
}

#[test]
fn shell_simulation_waits_for_the_exact_world_root() {
    let mut app = App::new();
    app.init_resource::<SessionGatedSimulation>()
        .init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();

    let authorized = app
        .world_mut()
        .run_system_once(simulation_authorized)
        .expect("run condition evaluates");
    assert!(
        !authorized,
        "scope publication alone is not world authority"
    );

    insert_session_world_component(app.world_mut(), SessionWorldFixture(1));
    let authorized = app
        .world_mut()
        .run_system_once(simulation_authorized)
        .expect("run condition evaluates");
    assert!(authorized, "the exact root completes gameplay authority");
}

#[test]
fn stale_root_cannot_authorize_a_newer_shell_scope() {
    let mut app = App::new();
    app.init_resource::<SessionGatedSimulation>()
        .init_resource::<ActiveSessionScope>();
    let stale = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    insert_session_world_component(app.world_mut(), SessionWorldFixture(1));
    let current = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    assert_ne!(stale, current);

    assert!(!app
        .world_mut()
        .run_system_once(simulation_authorized)
        .expect("run condition evaluates"));
    assert!(!app
        .world_mut()
        .run_system_once(session_world_exists)
        .expect("world condition evaluates"));

    assert!(
        session_world_entity(app.world()).is_none(),
        "imperative access rejects the stale root too"
    );
    assert!(
        session_world_component::<SessionWorldFixture>(app.world()).is_none(),
        "stale world components are structurally unreadable"
    );
    let root = {
        let world = app.world();
        let mut roots = world
            .try_query_filtered::<Entity, With<SessionRoot>>()
            .expect("SessionRoot is registered");
        roots
            .iter(world)
            .next()
            .expect("stale root still exists for poison repair")
    };
    app.world_mut()
        .entity_mut(root)
        .insert(SessionRoot(current));
    assert!(app
        .world_mut()
        .run_system_once(simulation_authorized)
        .expect("run condition evaluates"));
    assert!(app
        .world_mut()
        .run_system_once(session_world_exists)
        .expect("world condition evaluates"));
}

#[test]
fn direct_simulation_requires_the_same_canonical_root_without_shell_identity() {
    let mut app = App::new();
    assert!(!app
        .world_mut()
        .run_system_once(simulation_authorized)
        .expect("run condition evaluates"));

    insert_session_world_component(app.world_mut(), SessionWorldFixture(1));
    assert!(app
        .world_mut()
        .run_system_once(simulation_authorized)
        .expect("run condition evaluates"));
}

#[test]
fn cleanup_leaves_unscoped_frontend_entities_intact() {
    let mut app = session_app();
    let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    let frontend = app.world_mut().spawn(Name::new("launcher-root")).id();
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.spawn_in_session(a, Name::new("session-entity"));
        })
        .unwrap();

    app.world_mut().write_message(SessionScopeRetired(a));
    app.update();

    assert!(app.world().get_entity(frontend).is_ok());
    assert_eq!(count_scoped(&mut app, a), 0);
}
