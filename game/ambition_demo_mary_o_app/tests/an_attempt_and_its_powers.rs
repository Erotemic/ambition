//! What an ATTEMPT owns: her posture across a ledge, and her powers across a death.
//!
//! Both were reported by Jon on 2026-09-21 and both are the same shape — a fact
//! that belongs to the attempt being decided somewhere that cannot see the
//! attempt.

use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::actors::features::empowerment::Empowered;
use ambition_platformer2d::characters::equipment::WornEquipment;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

fn boot() -> App {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    for _ in 0..600 {
        app.update();
        let mut q = app
            .world_mut()
            .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
        if q.iter(app.world()).next().is_some() {
            return app;
        }
    }
    panic!("the demo never activated a playable body");
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}

/// Grow her through the shared equipment seam — the authority `sync_grown_form`
/// reads, so the tall form is reached the way the game reaches it.
fn give_her_the_wand(app: &mut App) {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Entity, With<PrimaryPlayer>>();
    let player = q.iter(world).next().expect("a player");
    world
        .entity_mut(player)
        .insert(WornEquipment::new(vec![
            ambition_demo_mary_o::powerups::star_wand(),
        ]));
}

fn body(app: &mut App) -> ae::BodyKinematics {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    *q.iter(world).next().expect("a player body")
}

fn mode(app: &mut App) -> Option<ae::player_state::BodyMode> {
    let world = app.world_mut();
    let mut q = world
        .query_filtered::<&ae::body_clusters::BodyModeState, With<PrimaryPlayer>>();
    q.iter(world).next().map(|m| m.body_mode)
}

/// **HER CROUCH SURVIVES LEAVING THE GROUND.**
///
/// World 1-2: walking off a ledge with DOWN held stood her up in mid-air, and
/// the full-height body then caught the platform edge she was stepping off and
/// shoved her back onto it. The ground test gated the whole crouch rather than
/// just its ENTRY.
#[test]
fn her_crouch_survives_leaving_the_ground() {
    let mut app = boot();
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    give_her_the_wand(&mut app);

    let mut down = ControlFrame::default();
    down.axis_y = 1.0;
    for _ in 0..20 {
        step(&mut app, down.clone());
    }
    let crouched = body(&mut app).size;
    let standing_height = crouched.y * 2.0;
    // ANTI-VACUITY: she must actually be crouched, and crouching must actually
    // change her height, or "still crouched in the air" proves nothing.
    assert_eq!(
        mode(&mut app),
        Some(ae::player_state::BodyMode::Crouching),
        "she never crouched on the ground, so the airborne arm below is vacuous"
    );

    // Off the ledge, DOWN still held. Relocated through the engine's own
    // authority (ADR 0024) rather than by poking `pos`.
    {
        let mut query = app.world_mut().query_filtered::<(
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
        ), With<PrimaryPlayer>>();
        let world = app.world_mut();
        let (mut cluster_item, mut motion_model) =
            query.iter_mut(world).next().expect("a player");
        let mut clusters = cluster_item.as_clusters_mut();
        ae::movement::transit_body(
            &mut motion_model,
            &mut clusters,
            ae::Vec2::new(300.0, 200.0),
            ae::movement::TransitVelocity::Zero,
        );
    }

    for tick in 0..8 {
        step(&mut app, down.clone());
        let size = body(&mut app).size;
        assert_eq!(
            mode(&mut app),
            Some(ae::player_state::BodyMode::Crouching),
            "airborne tick {tick}: she stood up in mid-air with DOWN still held"
        );
        assert!(
            (size.y - crouched.y).abs() < 0.01,
            "airborne tick {tick}: she is {:.1} tall, not her crouched {:.1} — a body \
             that inflates to {standing_height:.1} in mid-air catches the ledge it is \
             leaving",
            size.y,
            crouched.y
        );
    }

    // THE CONTROL: releasing DOWN in the air DOES stand her up, so the arm above
    // is about the crouch being carried rather than about the mode being stuck.
    for _ in 0..3 {
        step(&mut app, ControlFrame::default());
    }
    assert_eq!(
        mode(&mut app),
        Some(ae::player_state::BodyMode::Standing),
        "releasing DOWN in the air left her crouched, so the crouch is now a trap"
    );
}

/// **A DEATH TAKES BACK WHAT THE ATTEMPT EARNED.**
///
/// Her form is derived from `WornEquipment`, which is persistent player state
/// that room replay preserves on purpose — so the death restart rebuilt the body
/// and the surviving equipment immediately grew it back.
#[test]
fn a_death_takes_back_her_powers() {
    let mut app = boot();
    give_her_the_wand(&mut app);
    {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<PrimaryPlayer>>();
        let player = q.iter(world).next().expect("a player");
        world.entity_mut(player).insert(Empowered::for_seconds(
            ambition_demo_mary_o::star::COSMIC_QUASAR_SUPER_STATE,
            999.0,
        ));
    }
    for _ in 0..20 {
        app.update();
    }

    let form_of = |app: &mut App| -> String {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d::characters::actor::WornCharacter,
            With<PrimaryPlayer>,
        >();
        q.iter(world).next().map_or("-".into(), |w| w.id().to_string())
    };
    // ANTI-VACUITY: she is genuinely powered up before the death.
    assert_ne!(
        form_of(&mut app),
        "mary_o",
        "she never grew, so a death that leaves her small proves nothing"
    );

    // ⛔ UNTOUCHABLE refuses the kill, so the super-state is dropped first. The
    // EQUIPMENT — the thing under test — is left exactly as the player earned it.
    {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<PrimaryPlayer>>();
        let player = q.iter(world).next().expect("a player");
        world.entity_mut(player).remove::<Empowered>();
    }
    // The pit rule is what kills her, so the death is the real one.
    {
        let mut query = app.world_mut().query_filtered::<(
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
        ), With<PrimaryPlayer>>();
        let world = app.world_mut();
        let (mut cluster_item, mut motion_model) =
            query.iter_mut(world).next().expect("a player");
        let mut clusters = cluster_item.as_clusters_mut();
        ae::movement::transit_body(
            &mut motion_model,
            &mut clusters,
            ae::Vec2::new(300.0, 5000.0),
            ae::movement::TransitVelocity::Zero,
        );
    }
    for _ in 0..400 {
        app.update();
    }

    assert_eq!(
        form_of(&mut app),
        "mary_o",
        "she restarted the attempt still grown"
    );
    let world = app.world_mut();
    let mut q = world
        .query_filtered::<(Option<&WornEquipment>, Option<&Empowered>), With<PrimaryPlayer>>();
    let (worn, empowered) = q.iter(world).next().expect("a player");
    assert!(
        worn.is_none_or(|w| w.rows.is_empty()),
        "the attempt's equipment survived her death: {:?}",
        worn.map(|w| w.rows.iter().map(|r| r.id.clone()).collect::<Vec<_>>())
    );
    assert!(
        empowered.is_none(),
        "the quasar super-state survived her death"
    );
}
