//! **S1/S4 — the playable-persona architecture, exercised by an assembled demo.**
//!
//! A standalone demo (no `ambition_app`) proves the canonical path end to end:
//! the selected character becomes a simulation-owned `WornCharacter` identity ON
//! the canonical player, gameplay derives from it, and the identity does NOT
//! depend on the session-owned `StartingCharacter` startup component. This is the same
//! `WornCharacter` component + derive systems the full app uses — the demo just
//! assembles them through the `ambition_platformer2d` umbrella.

use bevy::prelude::*;

use ambition_platformer2d::actors::avatar::StartingCharacter;
use ambition_platformer2d::characters::actor::WornCharacter;

fn worn_of_primary(app: &mut App) -> Option<WornCharacter> {
    let mut q = app
        .world_mut()
        .query_filtered::<&WornCharacter, With<ambition_platformer2d::actors::actor::PrimaryPlayer>>();
    q.iter(app.world()).next().cloned()
}

fn primary_name(app: &mut App) -> Option<String> {
    let mut q = app
        .world_mut()
        .query_filtered::<&Name, With<ambition_platformer2d::actors::actor::PrimaryPlayer>>();
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

/// **S1.4:** the canonical identity is independent of the session world's
/// startup-selection component. Once captured on the entity at spawn, changing
/// that launch input does not rewrite the player's `WornCharacter`.
#[test]
fn identity_does_not_track_the_startup_selection_resource_after_spawn() {
    let mut app = ambition_demo_sanic_app::build_demo_app();
    settle_until_primary_player(&mut app);
    assert_eq!(worn_of_primary(&mut app).unwrap().id(), "sanic");

    // Change the session's startup selection to a DIFFERENT id after spawn.
    ambition_platformer2d::platformer::lifecycle::session_world_component_mut::<StartingCharacter>(
        app.world_mut(),
    )
    .expect("Sanic session world")
    .character_id = "goblin".into();
    for _ in 0..5 {
        app.update();
    }

    // The entity-owned identity is unmoved: presentation/gameplay derive from the
    // component, not from the mutated app-local resource.
    assert_eq!(
        worn_of_primary(&mut app).unwrap().id(),
        "sanic",
        "the canonical identity does not track startup selection after spawn"
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
            &ambition_platformer2d::actors::features::MotionModel,
            With<ambition_platformer2d::actors::actor::PrimaryPlayer>,
        >();
        matches!(
            q.iter(app.world()).next(),
            Some(ambition_platformer2d::actors::features::MotionModel::SurfaceMomentum(_))
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
    use ambition_platformer2d::actors::combat::moveset::ActorMoveset;
    use ambition_platformer2d::characters::brain::ActionSet;

    let mut app = ambition_demo_sanic_app::build_demo_app();
    app.update();
    for _ in 0..3 {
        app.update();
    }

    let (player, action_set, moveset_len) = {
        let mut q = app.world_mut().query_filtered::<
            (Entity, &ActionSet, &ActorMoveset),
            With<ambition_platformer2d::actors::actor::PrimaryPlayer>,
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
            .get::<ambition_platformer2d::characters::brain::ChargesProjectiles>(player)
            .is_none(),
        "an authored peaceful persona does not retain the host charge capability"
    );
    assert!(
        app.world()
            .get::<ambition_platformer2d::projectiles::PlayerProjectileState>(player)
            .is_none(),
        "the protagonist-only charge state is removed with its capability"
    );
}

/// **The Utility control is NAMED by the worn persona, like every other control.**
///
/// Jon, 2026-08-08: *"Sanic's transform button still reads 'fly'."* Each on-screen
/// control takes its word from the controlled subject's own action scheme
/// (`derive_action_scheme` → `ControlPrompt` → the touch button's `ButtonVerb`),
/// and the spawn label is only the fallback for a slot the scheme says nothing
/// about. The form toggle used to consume the raw Utility edge without DECLARING
/// itself, so Sanic's scheme left that slot empty and the fallback — "Fly" —
/// was, correctly, what showed.
///
/// Asserted on the ASSEMBLED demo on purpose: the declaration (content) and the
/// labelling (engine) live in different crates and each is inert without the
/// other, so a test on either half alone passes while the button still lies.
#[test]
fn the_utility_control_is_named_by_the_worn_persona_not_by_a_generic_fly_verb() {
    use ambition_platformer2d::entity_catalog::action_scheme::ControlSlot;
    use ambition_platformer2d::sim_view::ControlPrompt;

    let mut app = ambition_demo_sanic_app::build_demo_app();
    settle_until_primary_player(&mut app);
    for _ in 0..3 {
        app.update();
    }

    let prompt = app.world().resource::<ControlPrompt>();
    assert_eq!(
        prompt.label_for(ControlSlot::Utility),
        Some("Transform"),
        "the controlled Sanic body must say what its Utility slot DOES; \
         published prompt was {:?}",
        prompt.entries,
    );
}

/// **Every authored badnik declares whether it sleeps.**
///
/// The dormancy seam was built for Jon's Mary-O report — *"ai slop will just walk
/// off the edge of the level before she even gets to that part of the level"* —
/// and for a day it was wired to exactly one enemy in one game. A speedway is the
/// case that needs it most: the level is long, the badniks are spread down its
/// whole length, and Sanic reaches the far end seconds after the near one.
///
/// ⚠ **this also proves the tagger is REGISTERED**, which is the failure mode a
/// compile cannot catch. `tag_sanic_badniks` could be perfectly written and
/// simply never added to a schedule, and everything would still build.
#[test]
fn every_authored_badnik_declares_whether_it_sleeps() {
    use ambition_platformer2d::actors::features::ecs::dormancy::DormancyPolicy;
    use ambition_platformer2d::actors::features::ActorConfig;

    let mut app = ambition_demo_sanic_app::build_demo_app();
    for _ in 0..240 {
        app.update();
    }

    let mut q = app
        .world_mut()
        .query::<(&ActorConfig, Option<&DormancyPolicy>)>();
    let badniks: Vec<bool> = q
        .iter(app.world())
        .filter(|(config, _)| {
            matches!(
                &config.brain,
                ambition_platformer2d::entity_catalog::placements::CharacterBrain::Custom(key)
                    if key == ambition_demo_sanic::badnik::BADNIK_BRAIN_KEY
            )
        })
        .map(|(_, policy)| policy.is_some())
        .collect();

    assert!(
        !badniks.is_empty(),
        "the speedway authors badniks; if it stops, this test checks nothing"
    );
    assert!(
        badniks.iter().all(|declared| *declared),
        "{} of {} badniks declare no DormancyPolicy — they think for the whole \
         speedway and can walk off a ledge before Sanic arrives",
        badniks.iter().filter(|d| !**d).count(),
        badniks.len()
    );
}
