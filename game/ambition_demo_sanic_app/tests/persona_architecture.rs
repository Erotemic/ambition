//! S1/S4 — the playable-persona architecture, exercised by an assembled demo.
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

/// S1.1 + S1.2: after startup the canonical player carries the selected
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

/// S1.4: the canonical identity is independent of the session world's
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

/// The demo genuinely uses Sanic MOVEMENT, not just the name. The catalog's
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
            &ambition_platformer2d::actor::MotionModel,
            With<ambition_platformer2d::actors::actor::PrimaryPlayer>,
        >();
        matches!(
            q.iter(app.world()).next(),
            Some(ambition_platformer2d::actor::MotionModel::SurfaceMomentum(_))
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

/// The demo body wears SANIC'S authored kit, not Ambition's protagonist kit. `sanic` is the
/// demo's default/only character, so under the old rule (kit skipped for the content default)
/// it kept the code-side `sandbox_all` kit — Swipe, Bolt, bubble_shield — a peaceful speedster
/// that secretly shot fireballs. This is the assembled proof of the architecture fix — asserted
/// on `ActionSet` + `ActorMoveset`, not just movement.
///
/// He now authors a smash table — the crossover grid was pressing silence at him — and an AUTHORED
/// moveset OVERLAYS the derived one instead of being derived from the action set, so the body
/// carries seventeen moves and not one of them came from a protagonist kit.
///
/// Nothing consulted that: `combat_actions` derived the Attack / Special slots from the MOVESET
/// and the `ActionSet` alone, so the slots appeared, the persona gate kept the verbs, and every
/// press answered.
///
/// the ability is a real gate now (`ambition_characters::action_scheme`), and
/// what it gates is proved BEHAVIOURALLY next door in
/// [`the_demo_body_cannot_trigger_a_single_move_from_its_own_smash_table`] —
/// a count and an equality can only ever say what the body CARRIES. This one
/// keeps its own job: nothing of Ambition's protagonist leaked in, asserted as
/// exact equality with the table he authored.
#[test]
fn the_demo_body_wears_the_authored_peaceful_kit_not_the_host_protagonist_kit() {
    use ambition_platformer2d::combat::moveset::ActorMoveset;
    use ambition_platformer2d::characters::brain::ActionSet;

    let mut app = ambition_demo_sanic_app::build_demo_app();
    app.update();
    for _ in 0..3 {
        app.update();
    }

    let (player, action_set, worn_move_ids) = {
        let mut q = app.world_mut().query_filtered::<
            (Entity, &ActionSet, &ActorMoveset),
            With<ambition_platformer2d::actors::actor::PrimaryPlayer>,
        >();
        let (entity, set, moveset) = q
            .iter(app.world())
            .next()
            .expect("primary player has a kit");
        (
            entity,
            set.clone(),
            moveset
                .0
                .moves
                .iter()
                .map(|m| m.id.clone())
                .collect::<Vec<_>>(),
        )
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
    let authored: Vec<String> = ambition_demo_sanic::smash_moveset::sanic_moveset()
        .moves
        .iter()
        .map(|m| m.id.clone())
        .collect();
    assert_eq!(
        worn_move_ids, authored,
        "the body wears moves that are not the ones Sanic authored, so something \
         else supplied a swing"
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

/// Sanic at home cannot trigger a single move from his own smash table.
///
/// Sanic carries seventeen authored moves so a crossover ruleset has something to consume; his own
/// game grants him `RunJump`, which has no `attack`, so no press may start one.
///
/// asserted on what a press STARTS, never on a field. The spin-dash is a
/// TECHNIQUE on his Attack slot, so the melee edge is still routed and consumed
/// here — this passing means the technique kept its button while the repertoire
/// behind it stayed unreachable.
///
/// `app.update()` is a frame, not a tick: every press is held across a window
/// and then released, and the whole sweep runs under a ceiling.
#[test]
fn the_demo_body_cannot_trigger_a_single_move_from_its_own_smash_table() {
    use ambition_platformer2d::combat::moveset::MovePlayback;
    use ambition_platformer2d::engine_core::ControlFrame;

    let mut app = ambition_demo_sanic_app::build_demo_app();
    // the ordering lives in ONE place now — after the participant pipeline's routing stage and
    // before the frame→tick latch.
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    settle_until_primary_player(&mut app);
    for _ in 0..30 {
        app.update();
    }
    let body = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<ambition_platformer2d::actors::actor::PrimaryPlayer>>();
        q.iter(app.world()).next().expect("Sanic is seated")
    };
    assert!(
        app.world()
            .get::<ambition_platformer2d::combat::moveset::ActorMoveset>(body)
            .is_some_and(|m| !m.0.moves.is_empty()),
        "the gate must be the ABILITY — detaching his repertoire would break the \
         crossover grid that wants it (D146)"
    );

    #[allow(clippy::type_complexity)]
    let buttons: [(&str, fn(&mut ControlFrame)); 5] = [
        ("attack", |f| {
            f.attack_pressed = true;
            f.attack_held = true;
        }),
        ("smash", |f| {
            f.attack_pressed = true;
            f.attack_held = true;
            f.attack_strong_hint = true;
        }),
        ("special", |f| f.special_pressed = true),
        ("pogo", |f| f.pogo_pressed = true),
        ("projectile", |f| {
            f.projectile_pressed = true;
            f.projectile_held = true;
        }),
    ];
    let aims: [(&str, f32, f32); 5] = [
        ("neutral", 0.0, 0.0),
        ("forward", 1.0, 0.0),
        ("back", -1.0, 0.0),
        ("up", 0.0, 1.0),
        ("down", 0.0, -1.0),
    ];

    let mut triggered: Vec<String> = Vec::new();
    for (button, arm) in buttons {
        for (direction, ax, ay) in aims {
            for tick in 0..30 {
                let mut frame = ControlFrame {
                    axis_x: ax,
                    axis_y: ay,
                    aim_x: ax,
                    aim_y: ay,
                    left_pressed: ax < 0.0,
                    right_pressed: ax > 0.0,
                    up_pressed: ay > 0.0,
                    down_pressed: ay < 0.0,
                    ..ControlFrame::default()
                };
                if tick < 4 {
                    arm(&mut frame);
                }
                app.world_mut()
                    .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
                    .0 = frame;
                app.update();
                if let Some(playback) = app.world().get::<MovePlayback>(body) {
                    let entry = format!("{button}/{direction} -> {}", playback.spec.id);
                    if !triggered.contains(&entry) {
                        triggered.push(entry);
                    }
                }
            }
        }
    }
    assert!(
        triggered.is_empty(),
        "Sanic's own speedway answered a combat press with a smash move: {triggered:#?}"
    );

    // THE PAIRED POSITIVE TERM, AND WITHOUT IT THE ASSERTION ABOVE CANNOT
    // FAIL. Every claim in this test is that something does NOT happen, so a
    // run in which no press reaches Sanic at all satisfies it perfectly — which
    // is exactly what this file did while its stick was written in `PreUpdate`.
    // Holding a direction and watching him travel is what makes the silence
    // above mean "the ability gate held" rather than "the buttons went nowhere".
    let x_before = app
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(body)
        .expect("Sanic has a body")
        .pos
        .x;
    for _ in 0..90 {
        app.world_mut()
            .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
            .0 = ControlFrame {
            axis_x: 1.0,
            aim_x: 1.0,
            right_pressed: true,
            ..ControlFrame::default()
        };
        app.update();
    }
    let x_after = app
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(body)
        .expect("Sanic has a body")
        .pos
        .x;
    assert!(
        x_after > x_before + 8.0,
        "held right for 90 frames and Sanic moved from {x_before} to {x_after} — \
         no input reached him, so the combat-press sweep above proved nothing"
    );
}

/// The Utility control is NAMED by the worn persona, like every other control.
///
/// control takes its word from the controlled subject's own action scheme (`derive_action_scheme` →
/// `ControlPrompt` → the touch button's `ButtonVerb`), and the spawn label is only the fallback for
/// a slot the scheme says nothing about.
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

/// Every authored badnik declares whether it sleeps.
///
/// this also proves the tagger is REGISTERED, which is the failure mode a
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
