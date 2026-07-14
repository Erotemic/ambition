//! **S1/S4 — the playable-persona architecture, exercised by an assembled demo.**
//!
//! A standalone demo (no `ambition_app`) proves the canonical path end to end:
//! the selected character becomes a simulation-owned `WornCharacter` identity ON
//! the canonical player, gameplay derives from it, and the identity does NOT
//! depend on the app-local `StartingCharacter` startup resource. This is the same
//! `WornCharacter` component + derive systems the full app uses — the demo just
//! assembles them through the `ambition` umbrella.

use bevy::prelude::*;

use ambition::actors::avatar::StartingCharacter;
use ambition::characters::actor::WornCharacter;

fn worn_of_primary(app: &mut App) -> Option<WornCharacter> {
    let mut q = app
        .world_mut()
        .query_filtered::<&WornCharacter, With<ambition::actors::actor::PrimaryPlayer>>();
    q.iter(app.world()).next().cloned()
}

fn primary_name(app: &mut App) -> Option<String> {
    let mut q = app
        .world_mut()
        .query_filtered::<&Name, With<ambition::actors::actor::PrimaryPlayer>>();
    q.iter(app.world()).next().map(|n| n.as_str().to_string())
}

fn settle_until_primary_player(app: &mut App) {
    for _ in 0..8 {
        app.update();
        if worn_of_primary(app).is_some() {
            return;
        }
    }
    panic!("the provider load plan did not activate a primary player");
}

/// **S1.1 + S1.2:** after startup the canonical player carries the selected
/// character as a `WornCharacter` identity, and gameplay (its display name) is
/// derived from that identity.
#[test]
fn canonical_player_carries_the_selected_identity_and_derives_gameplay() {
    let mut app = ambition_demo_sanic_app::build_demo_app();
    app.update();
    for _ in 0..2 {
        app.update();
    }

    let worn = worn_of_primary(&mut app).expect("the primary player carries a WornCharacter");
    assert_eq!(
        worn.id(),
        "sanic",
        "the demo's selected character became the canonical identity"
    );
    assert_eq!(
        primary_name(&mut app).as_deref(),
        Some("Sanic"),
        "gameplay (the display name) is derived from the worn identity"
    );
}

/// **S1.4:** the canonical identity is INDEPENDENT of the app-local
/// `StartingCharacter` startup resource. Once captured on the entity at spawn,
/// nothing re-reads the resource for identity — mutating it afterwards does not
/// change the player's `WornCharacter`, so presentation/gameplay never rediscover
/// the selection from app-local startup state. (The resource stays live because
/// other systems, e.g. the dialogue default, read it — this proves independence
/// without pretending the resource is unused.)
#[test]
fn identity_does_not_track_the_startup_selection_resource_after_spawn() {
    let mut app = ambition_demo_sanic_app::build_demo_app();
    settle_until_primary_player(&mut app);
    assert_eq!(worn_of_primary(&mut app).unwrap().id(), "sanic");

    // Change the startup selection resource to a DIFFERENT id after spawn.
    app.world_mut()
        .resource_mut::<StartingCharacter>()
        .character_id = "goblin".to_string();
    for _ in 0..5 {
        app.update();
    }

    // The entity-owned identity is unmoved: presentation/gameplay derive from the
    // component, not from the mutated app-local resource.
    assert_eq!(
        worn_of_primary(&mut app).unwrap().id(),
        "sanic",
        "the canonical identity does not track the startup resource after spawn"
    );
    assert_eq!(primary_name(&mut app).as_deref(), Some("Sanic"));
}

/// **The demo genuinely uses Sanic MOVEMENT, not just the name.** The catalog's
/// `sanic` momentum profile puts the worn home box on `MotionModel::SurfaceMomentum`
/// (rides the speedway + loop), and the ball-dash rule — inert on any body without
/// that model — attaches to it. Guards against the demo silently degrading to an
/// axis-swept Ambition player wearing the label "Sanic".
#[test]
fn the_demo_body_rides_surface_momentum_and_arms_ball_dash() {
    let mut app = ambition_demo_sanic_app::build_demo_app();
    app.update();
    for _ in 0..3 {
        app.update();
    }

    let has_momentum = {
        let mut q = app.world_mut().query_filtered::<
            &ambition::actors::features::MotionModel,
            With<ambition::actors::actor::PrimaryPlayer>,
        >();
        matches!(
            q.iter(app.world()).next(),
            Some(ambition::actors::features::MotionModel::SurfaceMomentum(_))
        )
    };
    assert!(
        has_momentum,
        "the worn `sanic` momentum profile must put the body on SurfaceMomentum"
    );

    let ball_dash_armed = {
        let mut q = app
            .world_mut()
            .query::<&ambition_demo_sanic::ball_dash::BallDash>();
        q.iter(app.world()).next().is_some()
    };
    assert!(
        ball_dash_armed,
        "the ball-dash rule attaches to the momentum body (inert without SurfaceMomentum)"
    );
}

/// **The demo body wears SANIC'S authored kit, not Ambition's protagonist kit.**
/// `sanic` is the demo's default/only character, so under the old rule (kit skipped
/// for the content default) it kept the code-side `sandbox_all` kit — Swipe, Bolt,
/// bubble_shield — a peaceful speedster that secretly shot fireballs. With the
/// `default_character_id`↔code-kit coupling removed, `sanic` is an `Authored` row,
/// so its `"peaceful"` ActionSet (no melee / ranged / special) IS the worn kit, and
/// the derived directional moveset is empty. This is the assembled proof of the
/// architecture fix — asserted on `ActionSet` + `ActorMoveset`, not just movement.
#[test]
fn the_demo_body_wears_the_authored_peaceful_kit_not_the_host_protagonist_kit() {
    use ambition::actors::combat::moveset::ActorMoveset;
    use ambition::characters::brain::ActionSet;

    let mut app = ambition_demo_sanic_app::build_demo_app();
    app.update();
    for _ in 0..3 {
        app.update();
    }

    let (player, action_set, moveset_len) = {
        let mut q = app.world_mut().query_filtered::<
            (Entity, &ActionSet, &ActorMoveset),
            With<ambition::actors::actor::PrimaryPlayer>,
        >();
        let (entity, set, moveset) = q
            .iter(app.world())
            .next()
            .expect("primary player has a kit");
        (entity, set.clone(), moveset.0.moves.len())
    };

    assert!(
        action_set.melee.is_none(),
        "Sanic's authored peaceful kit has no melee — the code-side Swipe is gone"
    );
    assert!(
        action_set.ranged.is_none(),
        "Sanic's peaceful kit has no ranged — the protagonist's Bolt/fireball is gone"
    );
    assert!(
        action_set.special.is_none(),
        "Sanic's peaceful kit has no special — the bubble_shield is gone"
    );
    assert_eq!(
        moveset_len, 0,
        "an empty melee derives an empty directional moveset"
    );
    assert!(
        app.world()
            .get::<ambition::characters::brain::ChargesProjectiles>(player)
            .is_none(),
        "an authored peaceful persona does not retain the host charge capability"
    );
    assert!(
        app.world()
            .get::<ambition::projectiles::PlayerProjectileState>(player)
            .is_none(),
        "the protagonist-only charge state is removed with its capability"
    );
}
