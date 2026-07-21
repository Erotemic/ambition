//! Walk/run, the latch, and the shared movement kernel's response to the throttle.

use super::*;
use ambition::characters::equipment::WornEquipment;
use ambition::engine_core::ControlFrame;

use crate::powerups::{grow_cap, spark_blossom};

fn body(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            PrimaryPlayer,
            MaryOGait::default(),
            ActorControl::default(),
            ae::BodyKinematics {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(30.0, 48.0),
                facing: 1.0,
            },
        ))
        .id()
}

fn app_with_policy() -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(ambition::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    let body = body(&mut app);
    app.add_systems(Update, walk_by_default_run_while_held);
    (app, body)
}

/// Set this tick's brain-produced intent, as `tick_player_brains` would.
fn intend(app: &mut App, body: Entity, x: f32, run_held: bool) {
    let mut control = app.world_mut().get_mut::<ActorControl>(body).unwrap();
    control.0.locomotion.x = x;
    control.0.modifier_held = run_held;
}

fn throttle(app: &App, body: Entity) -> f32 {
    app.world().get::<ActorControl>(body).unwrap().0.locomotion.x
}

/// Direction ALONE is a walk: the throttle reaching the movement kernel is
/// reduced, so the target speed it accelerates toward is the walk speed.
#[test]
fn direction_alone_selects_walk_speed() {
    let (mut app, body) = app_with_policy();
    intend(&mut app, body, 1.0, false);
    app.update();

    assert_eq!(
        throttle(&app, body),
        WALK_THROTTLE,
        "no run held -> the walk throttle reaches the body"
    );
    assert!(!app.world().get::<MaryOGait>(body).unwrap().running);
}

/// Sustaining the semantic run action selects the full run speed. Nothing about
/// the DEVICE is involved — the policy reads the slot off the body's own frame.
#[test]
fn holding_the_run_action_selects_run_speed() {
    let (mut app, body) = app_with_policy();
    intend(&mut app, body, 1.0, true);
    app.update();

    assert_eq!(
        throttle(&app, body),
        1.0,
        "run held -> full throttle reaches the body"
    );
    assert!(app.world().get::<MaryOGait>(body).unwrap().running);
}

/// The throttle is a pure scale, so it is sign-preserving and works leftward.
#[test]
fn the_walk_throttle_is_symmetric() {
    let (mut app, body) = app_with_policy();
    intend(&mut app, body, -1.0, false);
    app.update();
    assert_eq!(throttle(&app, body), -WALK_THROTTLE);
}

/// **The held state survives the frame->tick latch.** A sustained technique that
/// evaporated on a catch-up tick would drop the player out of a run mid-stride, so
/// the modifier must be carried as a LEVEL (retained) and not as an EDGE (consumed).
#[test]
fn the_held_run_survives_the_frame_to_tick_latch() {
    use ambition::engine_core::ControlFrameLatch;

    let mut latch = ControlFrameLatch::default();
    latch.accumulate(ControlFrame {
        axis_x: 1.0,
        modifier_held: true,
        modifier_pressed: true,
        ..ControlFrame::default()
    });

    let first = latch.take();
    assert!(first.modifier_held, "the first tick sees the hold");
    assert!(first.modifier_pressed, "and the press edge");

    // A second tick in the same frame (the sim catching up) with no new sample.
    let second = latch.take();
    assert!(
        second.modifier_held,
        "the HOLD is retained — she stays running through a catch-up tick"
    );
    assert!(
        !second.modifier_pressed,
        "but the press fires exactly once, so one tap is one spark"
    );
}

/// **Releasing run does not erase accumulated speed**, and reaching top speed
/// takes real time rather than snapping.
///
/// Both properties are consequences of Mary-O's AUTHORED acceleration meeting the
/// shared kernel, which accelerates toward `throttle * max_run_speed` at
/// `run_accel`. So the thing to check is the authored relationship: her run_accel
/// must be low enough that the wind-up and the decay are perceptible, which is
/// exactly what separates her from a body that teleports to top speed.
#[test]
fn her_authored_gait_makes_speed_something_she_builds_and_keeps() {
    let mut app = App::new();
    crate::add_demo_content(&mut app);
    let catalog = app
        .world()
        .resource::<ambition::characters::actor::character_catalog::CharacterCatalog>();
    let tuning = catalog
        .axis_tuning(crate::provider::MARY_O_CHARACTER_ID)
        .expect("Mary-O authors her gait");

    // Wind-up: time to go from a standstill to her run speed.
    let wind_up = tuning.max_run_speed / tuning.run_accel;
    assert!(
        wind_up > 0.2,
        "top speed must be BUILT, not snapped to (wind-up {wind_up}s)"
    );

    // Release: one tick after dropping to the walk target she is still faster
    // than a walk — the momentum decays over time instead of being erased.
    let dt = 1.0 / 60.0;
    let walk_target = WALK_THROTTLE * tuning.max_run_speed;
    let after_one_tick = tuning.max_run_speed - tuning.run_accel * dt;
    assert!(
        after_one_tick > walk_target,
        "releasing run decays toward the walk speed ({after_one_tick} > {walk_target}), \
         it does not snap to it"
    );

    // Reversal: crossing from full run to a standstill is a visible slide.
    let skid = tuning.max_run_speed / tuning.run_accel;
    assert!(skid > 0.2, "a reversal at speed is a readable skid ({skid}s)");

    // And she is meaningfully faster than the walk she defaults to.
    assert!(
        tuning.max_run_speed > walk_target * 1.5,
        "the two gaits are distinguishable"
    );
}

/// Reversing at speed spends real time crossing zero — the readable skid. The
/// gait flag that presentation reads is raised for exactly that window.
#[test]
fn reversing_at_speed_reads_as_a_skid() {
    let (mut app, body) = app_with_policy();
    app.world_mut().get_mut::<ae::BodyKinematics>(body).unwrap().vel.x = 300.0;
    intend(&mut app, body, -1.0, true);
    app.update();

    assert!(
        app.world().get::<MaryOGait>(body).unwrap().skidding,
        "input opposing a fast velocity is a skid"
    );

    // The same reversal at a crawl is just a turn.
    app.world_mut().get_mut::<ae::BodyKinematics>(body).unwrap().vel.x = 10.0;
    intend(&mut app, body, -1.0, true);
    app.update();
    assert!(
        !app.world().get::<MaryOGait>(body).unwrap().skidding,
        "a slow turn is not a skid"
    );
}

/// The run policy is a throttle, never an impulse: it may only REDUCE intent, and
/// it never touches velocity. This is what separates it from the dash.
#[test]
fn the_run_policy_never_adds_velocity_or_amplifies_intent() {
    let (mut app, body) = app_with_policy();
    intend(&mut app, body, 1.0, true);
    let before = app.world().get::<ae::BodyKinematics>(body).unwrap().vel;
    app.update();
    let after = app.world().get::<ae::BodyKinematics>(body).unwrap().vel;

    assert_eq!(before, after, "the policy writes no velocity — not a dash");
    assert!(
        throttle(&app, body) <= 1.0,
        "and never amplifies intent past the body's own capability"
    );
}

// ---------------------------------------------------------------------------
// Firing on the press edge
// ---------------------------------------------------------------------------

fn app_with_fire(worn: WornEquipment) -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(ambition::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    let body = body(&mut app);
    app.world_mut().entity_mut(body).insert(worn);
    app.add_systems(Update, fire_spark_on_run_press);
    (app, body)
}

fn press_run(app: &mut App, body: Entity) {
    let mut control = app.world_mut().get_mut::<ActorControl>(body).unwrap();
    control.0.modifier_pressed = true;
    control.0.modifier_held = true;
}

fn fired(app: &App, body: Entity) -> bool {
    app.world().get::<ActorControl>(body).unwrap().0.fire.is_some()
}

/// **Firing uses the press edge** — no charge, no release to wait for. The shot
/// is requested on the very tick the button goes down.
#[test]
fn firing_uses_the_press_edge_and_needs_no_charge() {
    let (mut app, body) = app_with_fire(WornEquipment::new(vec![spark_blossom()]));
    press_run(&mut app, body);
    app.update();

    assert!(
        fired(&app, body),
        "the press alone raises the fire intent — no hold-and-release"
    );
}

/// Without the blossom the same button is run-only: pressing it fires nothing.
#[test]
fn without_the_blossom_the_run_button_does_not_fire() {
    let (mut app, body) = app_with_fire(WornEquipment::new(vec![grow_cap()]));
    press_run(&mut app, body);
    app.update();
    assert!(!fired(&app, body), "grown but sparkless: the button only runs");
}

/// The authored cooldown gates the cadence; a second press inside it is refused.
#[test]
fn the_authored_cooldown_gates_the_next_spark() {
    let (mut app, body) = app_with_fire(WornEquipment::new(vec![spark_blossom()]));
    press_run(&mut app, body);
    app.update();
    assert!(fired(&app, body));

    // Clear the intent and press again immediately.
    app.world_mut().get_mut::<ActorControl>(body).unwrap().0.fire = None;
    press_run(&mut app, body);
    app.update();
    assert!(
        !fired(&app, body),
        "a second press inside the cooldown is refused"
    );
}

/// Holding run without a fresh press does not machine-gun sparks: the LEVEL means
/// run, only the EDGE means fire.
#[test]
fn holding_run_does_not_repeat_fire() {
    let (mut app, body) = app_with_fire(WornEquipment::new(vec![spark_blossom()]));
    let mut control = app.world_mut().get_mut::<ActorControl>(body).unwrap();
    control.0.modifier_held = true;
    control.0.modifier_pressed = false;
    app.update();

    assert!(
        !fired(&app, body),
        "sustaining the slot runs; it does not fire"
    );
}

/// The scheme names the slot's CURRENT role, so the prompt can describe one button
/// doing two things.
#[test]
fn the_slot_label_follows_the_power_state() {
    use ambition::characters::action_scheme::ActorTechniques;
    use ambition::entity_catalog::action_scheme::ControlSlot;

    let mut app = App::new();
    let body = body(&mut app);
    app.world_mut()
        .entity_mut(body)
        .insert(WornEquipment::new(vec![grow_cap()]));
    app.add_systems(Update, sync_run_action_scheme);
    app.update();

    let label = |app: &App| {
        app.world()
            .get::<ActorTechniques>(body)
            .unwrap()
            .0
            .iter()
            .find(|a| a.slot == ControlSlot::Modifier)
            .and_then(|a| a.display_name.clone())
    };
    assert_eq!(label(&app).as_deref(), Some("Run"), "sparkless: run only");

    app.world_mut()
        .get_mut::<WornEquipment>(body)
        .unwrap()
        .equip(spark_blossom());
    app.update();
    assert_eq!(
        label(&app).as_deref(),
        Some("Run / Spark"),
        "with the blossom the same slot advertises both roles"
    );
}
