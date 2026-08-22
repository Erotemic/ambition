use bevy::prelude::{App, Resource};

use super::*;
use crate::{
    ActiveShellExperience, PendingShellRoute, ShellActivationId, ShellRouteCatalog, ShellRouteId,
    ShellRouteSpec,
};

#[derive(Resource, Debug, PartialEq)]
struct LobbyCursor(u8);

/// A resource two experiences publish into, which knows who published it.
#[derive(Resource, Debug, PartialEq)]
struct Roster {
    published_by: &'static str,
}

fn active(route: &str, experience: &str) -> ActiveShellExperience {
    ActiveShellExperience {
        activation_id: ShellActivationId(1),
        route_id: ShellRouteId::new(route),
        experience_id: ShellExperienceId::new(experience),
        parameters: Default::default(),
        load_authorization: None,
        prepared_session: None,
    }
}

fn pending(route: &str) -> PendingShellRoute {
    PendingShellRoute {
        route_id: ShellRouteId::new(route),
        push_history: true,
        barrier: ambition_load::LoadBarrierRef::new(
            ambition_load::LoadId::new("load"),
            ambition_load::LoadBarrierId::new("ready"),
        ),
        requires_prepared_session: false,
        terminal_reported: false,
    }
}

/// An app whose catalog knows the two routes of the `game` provider plus a
/// route belonging to somebody else.
fn app_with_scope() -> App {
    let mut app = App::new();
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("game_select", "game.select"));
    catalog.register(ShellRouteSpec::new("game_play", "game"));
    catalog.register(ShellRouteSpec::new("other_play", "other"));
    app.insert_resource(catalog);
    app.insert_resource(ShellRouter::default());
    app.experience_owns("game")
        .covering("game.select")
        .releasing::<LobbyCursor>()
        .releasing_owned::<Roster>(|roster, owner| roster.published_by == owner.as_str());
    app
}

fn go(app: &mut App, route: &str, experience: &str) {
    let mut router = app.world_mut().resource_mut::<ShellRouter>();
    router.active = Some(active(route, experience));
    router.pending = None;
    release_departed_experience_state(app.world_mut());
}

/// The leak this exists for. A resource a provider published survived the
/// route that published it, and the next experience inherited it — which is how
/// picking Oni Leader in the smash lobby redressed the body Ambition controls.
#[test]
fn state_a_provider_published_leaves_with_it() {
    let mut app = app_with_scope();
    go(&mut app, "game_play", "game");
    app.world_mut().insert_resource(LobbyCursor(3));
    app.world_mut().insert_resource(Roster {
        published_by: "game",
    });

    // Still inside: nothing is released while the provider is on screen.
    release_departed_experience_state(app.world_mut());
    assert!(app.world().contains_resource::<LobbyCursor>());

    go(&mut app, "other_play", "other");
    assert!(
        !app.world().contains_resource::<LobbyCursor>(),
        "the provider's own state outlived its experience"
    );
    assert!(
        !app.world().contains_resource::<Roster>(),
        "the roster the provider published outlived it"
    );
}

/// A provider's own screens are not a departure.
///
/// The select screen and the match are two experiences of one provider, and the
/// roster is published by the first FOR the second. A scope that released on any
/// change of experience id would delete it on the frame it was handed over.
#[test]
fn moving_between_a_providers_own_experiences_releases_nothing() {
    let mut app = app_with_scope();
    go(&mut app, "game_select", "game.select");
    app.world_mut().insert_resource(Roster {
        published_by: "game",
    });
    go(&mut app, "game_play", "game");
    assert!(
        app.world().contains_resource::<Roster>(),
        "the lobby's roster was released on the way into the match it was for"
    );
}

/// A route waiting on its load barrier has not left yet.
///
/// The premise the release rule is allowed to be this simple on: `activate`
/// takes the old activation and installs the new one in one call, so while the
/// match route waits for its barrier, `active` still names the lobby. Nothing
/// observes a gap — which is why this asks `active` alone and does not consult
/// `pending`.
#[test]
fn a_route_waiting_on_its_barrier_has_not_left_its_experience() {
    let mut app = app_with_scope();
    go(&mut app, "game_select", "game.select");
    app.world_mut().insert_resource(Roster {
        published_by: "game",
    });

    // The command was accepted and the match route is waiting on its load
    // barrier: `pending` is set and `active` is untouched.
    app.world_mut().resource_mut::<ShellRouter>().pending = Some(pending("game_play"));
    release_departed_experience_state(app.world_mut());
    assert!(
        app.world().contains_resource::<Roster>(),
        "the roster was released while the match route it belongs to was loading"
    );
}

/// Cleanup removes what this owner published, not what the resource is.
///
/// The roster is a global another experience also publishes into. Releasing it
/// by type would be one game deleting another's match.
#[test]
fn a_strangers_value_in_a_shared_resource_is_left_alone() {
    let mut app = app_with_scope();
    go(&mut app, "game_play", "game");
    app.world_mut().insert_resource(Roster {
        published_by: "other",
    });
    go(&mut app, "other_play", "other");
    assert_eq!(
        app.world().get_resource::<Roster>(),
        Some(&Roster {
            published_by: "other"
        }),
        "leaving one experience deleted another experience's published state"
    );
}

/// The run condition reads the router, so it is correct wherever it is
/// scheduled — and says NO in a composition with no routes at all.
#[test]
fn the_active_experience_run_condition_answers_from_the_router() {
    let mut app = App::new();
    let condition = shell_experience_is_active("game");
    assert!(
        !app.world_mut().run_system_once(condition.clone()).unwrap(),
        "a system gated on an experience ran in a host that has no routes"
    );

    app.insert_resource(ShellRouter::default());
    assert!(!app.world_mut().run_system_once(condition.clone()).unwrap());

    app.world_mut().resource_mut::<ShellRouter>().active = Some(active("game_play", "game"));
    assert!(app.world_mut().run_system_once(condition.clone()).unwrap());

    app.world_mut().resource_mut::<ShellRouter>().active = Some(active("other_play", "other"));
    assert!(!app.world_mut().run_system_once(condition).unwrap());
}

use bevy::ecs::system::RunSystemOnce;

/// The activation whose owner is written on the `Roster` rather than on itself.
#[derive(Resource, Debug, PartialEq)]
struct Activation(u8);

/// A witnessed release leaves a stranger's state alone, and takes its own.
///
/// The shape `ActiveMatch` needs: rollback state that deliberately carries no
/// identity, released on the word of the plan it came from.
#[test]
fn a_witnessed_release_asks_the_witness_who_owns_it() {
    let mut app = App::new();
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("game_play", "game"));
    catalog.register(ShellRouteSpec::new("other_play", "other"));
    app.insert_resource(catalog);
    app.insert_resource(ShellRouter::default());
    app.experience_owns("game")
        .releasing_witnessed::<Activation, Roster>(|roster, owner| {
            roster.published_by == owner.as_str()
        })
        .releasing_owned::<Roster>(|roster, owner| roster.published_by == owner.as_str());

    // A stranger's roster stands with a stranger's activation.
    app.insert_resource(Roster {
        published_by: "other",
    });
    app.insert_resource(Activation(7));
    go(&mut app, "game_play", "game");
    go(&mut app, "other_play", "other");
    release_departed_experience_state(app.world_mut());
    assert_eq!(
        app.world().get_resource::<Activation>(),
        Some(&Activation(7)),
        "leaving `game` deleted an activation whose witness names `other`"
    );

    // And its own leaves with it.
    app.insert_resource(Roster {
        published_by: "game",
    });
    app.insert_resource(Activation(1));
    go(&mut app, "game_play", "game");
    go(&mut app, "other_play", "other");
    release_departed_experience_state(app.world_mut());
    assert!(
        app.world().get_resource::<Activation>().is_none(),
        "`game`'s own activation outlived its route"
    );
}

/// a witness released before the thing that reads it is a release that
/// silently stops working, so declaring them in that order is refused where
/// the mistake is made rather than discovered later as a leak.
#[test]
#[should_panic(expected = "already released earlier in this scope")]
fn a_witness_may_not_be_released_before_its_dependent() {
    let mut app = App::new();
    app.insert_resource(ShellRouter::default());
    app.experience_owns("game")
        .releasing_owned::<Roster>(|roster, owner| roster.published_by == owner.as_str())
        .releasing_witnessed::<Activation, Roster>(|roster, owner| {
            roster.published_by == owner.as_str()
        });
}
