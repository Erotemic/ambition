use super::*;
use bevy::prelude::App;

fn reg(id: &str, name: &str, route: &str) -> ExperienceRegistration {
    ExperienceRegistration::new(id, name, route)
}

#[test]
fn identical_re_registration_is_idempotent() {
    let mut app = App::new();
    app.register_experience(
        reg("sanic", "Sanic", "sanic_gameplay"),
        ShellRouteSpec::new("sanic_gameplay", "sanic"),
    );
    app.register_experience(
        reg("sanic", "Sanic", "sanic_gameplay"),
        ShellRouteSpec::new("sanic_gameplay", "sanic"),
    );
    assert_eq!(
        app.world().resource::<ShellExperienceRegistry>().len(),
        1,
        "an identical re-registration is a no-op, not a second entry"
    );
}

#[test]
#[should_panic(expected = "duplicate shell experience id 'sanic'")]
fn conflicting_duplicate_experience_id_panics() {
    let mut app = App::new();
    app.register_experience(
        reg("sanic", "Sanic", "sanic_gameplay"),
        ShellRouteSpec::new("sanic_gameplay", "sanic"),
    );
    // Same id, different owner/route — a genuine conflict.
    app.register_experience(
        reg("sanic", "Impostor", "impostor_route"),
        ShellRouteSpec::new("impostor_route", "sanic"),
    );
}

/// Capture the panic message from `build`, suppressing the default hook so
/// the test output stays clean.
fn capture_panic(build: impl FnOnce() + std::panic::UnwindSafe) -> String {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(build);
    std::panic::set_hook(previous);
    let payload = result.expect_err("expected a panic");
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .expect("panic payload is a string")
}

/// Issue 7: two different experiences claiming one route id is a collision,
/// not a silent clobber. Issue 8: the diagnostic is byte-identical regardless
/// of which registered first.
#[test]
fn duplicate_route_id_is_rejected_in_both_orders_with_one_message() {
    let forward = capture_panic(|| {
        let mut app = App::new();
        app.register_experience(
            reg("alpha", "Alpha", "shared_route"),
            ShellRouteSpec::new("shared_route", "alpha"),
        );
        app.register_experience(
            reg("beta", "Beta", "shared_route"),
            ShellRouteSpec::new("shared_route", "beta"),
        );
    });
    let reverse = capture_panic(|| {
        let mut app = App::new();
        app.register_experience(
            reg("beta", "Beta", "shared_route"),
            ShellRouteSpec::new("shared_route", "beta"),
        );
        app.register_experience(
            reg("alpha", "Alpha", "shared_route"),
            ShellRouteSpec::new("shared_route", "alpha"),
        );
    });
    assert!(
        forward.contains("duplicate shell route id 'shared_route'"),
        "message names the colliding route: {forward}"
    );
    assert_eq!(
        forward, reverse,
        "the route-collision diagnostic is registration-order-independent"
    );
}

/// Issue 8: the duplicate-experience-id diagnostic is also order-independent.
#[test]
fn duplicate_experience_id_diagnostic_is_order_independent() {
    let forward = capture_panic(|| {
        let mut app = App::new();
        app.register_experience(
            reg("dup", "First", "route_a"),
            ShellRouteSpec::new("route_a", "dup"),
        );
        app.register_experience(
            reg("dup", "Second", "route_b"),
            ShellRouteSpec::new("route_b", "dup"),
        );
    });
    let reverse = capture_panic(|| {
        let mut app = App::new();
        app.register_experience(
            reg("dup", "Second", "route_b"),
            ShellRouteSpec::new("route_b", "dup"),
        );
        app.register_experience(
            reg("dup", "First", "route_a"),
            ShellRouteSpec::new("route_a", "dup"),
        );
    });
    assert!(forward.contains("duplicate shell experience id 'dup'"));
    assert_eq!(forward, reverse);
}

/// A route registered by a host directly (e.g. a non-gameplay home route)
/// still collides deterministically with a later experience claiming it.
#[test]
fn preexisting_route_blocks_a_later_experience_claiming_it() {
    let message = capture_panic(|| {
        let mut app = App::new();
        app.world_mut()
            .get_resource_or_insert_with(ShellRouteCatalog::default)
            .register(ShellRouteSpec::new("home", "host_home"));
        app.register_experience(
            reg("game", "Game", "home"),
            ShellRouteSpec::new("home", "game"),
        );
    });
    assert!(
        message.contains("duplicate shell route id 'home'"),
        "a manually-registered route is still protected: {message}"
    );
}

/// The launcher opens the SCREEN, the session stays on its own route.
///
/// A character select is the case: the row says "Smash" and leads to a
/// question, and the stage route it eventually reaches still owns the
/// session, the preparation plan and the completion policy.
#[test]
fn an_experience_can_enter_at_a_screen_of_its_own() {
    let mut app = App::new();
    app.world_mut()
        .get_resource_or_insert_with(ShellRouteCatalog::default)
        .register(ShellRouteSpec::new("smash_select", "smash.select"));
    app.register_experience(
        reg("smash", "Smash", "smash_gameplay").entered_at("smash_select"),
        ShellRouteSpec::new("smash_gameplay", "smash"),
    );
    let entries = app
        .world()
        .resource::<ShellExperienceRegistry>()
        .launch_entries();
    assert_eq!(
        entries[0].route_id,
        ShellRouteId::new("smash_select"),
        "the launcher row opens the select screen, not the stage"
    );
    assert!(
        app.world()
            .resource::<ShellRouteCatalog>()
            .contains(&ShellRouteId::new("smash_gameplay")),
        "the session route is registered all the same"
    );
}

#[test]
fn entering_at_an_unregistered_route_is_refused_by_name() {
    let message = capture_panic(|| {
        let mut app = App::new();
        app.register_experience(
            reg("smash", "Smash", "smash_gameplay").entered_at("smash_select"),
            ShellRouteSpec::new("smash_gameplay", "smash"),
        );
    });
    assert!(
        message.contains("enters at route 'smash_select'"),
        "the refusal names the missing entry route: {message}"
    );
}

#[test]
fn launcher_entries_stay_unique_and_ordered() {
    let mut app = App::new();
    for (id, name) in [
        ("ambition", "Ambition"),
        ("sanic", "Sanic"),
        ("mary_o", "Mary-O"),
    ] {
        let route = format!("{id}_gameplay");
        app.register_experience(
            reg(id, name, &route),
            ShellRouteSpec::new(route.as_str(), id),
        );
    }
    let registry = app.world().resource::<ShellExperienceRegistry>();
    let ids: Vec<_> = registry.iter().map(|e| e.id.as_str().to_owned()).collect();
    assert_eq!(
        ids,
        vec!["ambition", "sanic", "mary_o"],
        "registration order is stable"
    );
    assert_eq!(
        registry.launch_entries().len(),
        3,
        "each provider appears exactly once"
    );
}
