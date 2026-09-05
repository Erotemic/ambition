use super::*;
use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};

fn app() -> App {
    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    let mut time = app
        .world_mut()
        .resource_mut::<ambition_platformer2d::time::WorldTime>();
    time.scaled_dt = 1.0 / 60.0;
    time.raw_dt = 1.0 / 60.0;
    app.add_systems(
        Update,
        (apply_authored_time_dilations, expire_time_dilations).chain(),
    );
    app
}

fn victim(app: &mut App) -> Entity {
    app.world_mut()
        .spawn(ambition_platformer2d::time::ProperTimeScale::default())
        .id()
}

fn ask(app: &mut App, who: Entity, scale: f32, seconds: f32) {
    app.world_mut().write_message(ActorActionMessage {
        actor: who,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::Special(TIME_DILATION.to_string()),
            params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                &TimeDilationParams { scale, seconds },
            )
            .expect("dilation params serialize"),
        },
    });
    app.update();
}

fn scale_of(app: &App, who: Entity) -> f32 {
    app.world()
        .get::<ambition_platformer2d::time::ProperTimeScale>(who)
        .expect("the body kept its clock")
        .0
}

/// ⭐⭐ A DILATION SLOWS A BODY AND THEN GIVES ITS TIME BACK ON THE WORLD'S CLOCK.
///
/// ⛔ THE MIDDLE ASSERTION IS THE ONE THAT MATTERS: still slow at half the
/// duration. Without it, "slow then normal" is satisfied by a dilation cleared on
/// the very next tick — a move that does nothing, and one that looks exactly like
/// a move that worked.
#[test]
fn a_dilation_slows_a_body_and_expires_on_world_time() {
    let mut app = app();
    let who = victim(&mut app);
    ask(&mut app, who, 0.25, 0.40);
    assert_eq!(scale_of(&app, who), 0.25, "the body did not take the slow");

    for _ in 0..12 {
        app.update();
    }
    assert_eq!(
        scale_of(&app, who),
        0.25,
        "the slow was gone at half its authored duration — it is being cleared \
         rather than counted down, so every number an author writes is a lie"
    );

    for _ in 0..18 {
        app.update();
    }
    assert_eq!(
        scale_of(&app, who),
        1.0,
        "the body never got its own clock back — nobody else owes a restore, so \
         a fighter slowed once is slow for the rest of the match"
    );
}

/// ⛔⛔ AND A SECOND DILATION DOES NOT NEST.
///
/// Two overlapping slows would multiply into a body that is barely moving, and
/// each would try to restore a prior the other had already overwritten. ⇒ The
/// newest wins and keeps the ORIGINAL prior, so however many land, one restore
/// returns the body to the clock it started on.
#[test]
fn a_second_dilation_replaces_the_first_and_still_restores_the_original() {
    let mut app = app();
    let who = victim(&mut app);
    ask(&mut app, who, 0.5, 0.40);
    ask(&mut app, who, 0.25, 0.40);
    assert_eq!(
        scale_of(&app, who),
        0.25,
        "the second dilation did not take, or the two multiplied"
    );
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        scale_of(&app, who),
        1.0,
        "the restore put the body back on the FIRST dilation's scale rather than \
         the clock it started on"
    );
}

/// ⛔ AN AUTHORED SPEED-UP IS REFUSED, and a zero-second one is too.
#[test]
fn a_speed_up_or_a_zero_duration_is_refused_rather_than_applied() {
    let mut app = app();
    let who = victim(&mut app);
    ask(&mut app, who, 2.0, 0.40);
    assert_eq!(
        scale_of(&app, who),
        1.0,
        "an authored SPEED-UP was applied — the direction nobody has designed"
    );
    ask(&mut app, who, 0.25, 0.0);
    assert_eq!(
        scale_of(&app, who),
        1.0,
        "a zero-second dilation was applied, so nothing would ever take it away"
    );
    // ⛔ POISON GUARD. Both assertions hold for an adapter that refuses
    // EVERYTHING, including the dilations it exists to apply.
    ask(&mut app, who, 0.25, 0.40);
    assert_eq!(
        scale_of(&app, who),
        0.25,
        "poison: this adapter applies nothing at all, so the refusals above say \
         nothing about the guard"
    );
}
