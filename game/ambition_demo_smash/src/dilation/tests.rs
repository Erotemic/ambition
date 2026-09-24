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
    // The plugin's order: expire, then apply.
    app.add_systems(
        Update,
        (expire_time_dilations, apply_authored_time_dilations).chain(),
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
        move_instance: None,
    });
    app.update();
}

fn scale_of(app: &App, who: Entity) -> f32 {
    app.world()
        .get::<ambition_platformer2d::time::ProperTimeScale>(who)
        .expect("the body kept its clock")
        .0
}

/// A dilation slows a body, then gives its time back on the world clock.
///
/// The middle assertion matters most: still slow at half the duration. Without
/// it, a dilation cleared on the next tick would pass.
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

/// A second dilation does not nest.
///
/// Overlapping slows would multiply, and each would restore a prior the other
/// overwrote. The newest wins and keeps the original prior.
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

/// An authored speed-up is refused, and so is a zero-second one.
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
    // Poison guard: both assertions above hold for an adapter that refuses
    // everything.
    ask(&mut app, who, 0.25, 0.40);
    assert_eq!(
        scale_of(&app, who),
        0.25,
        "poison: this adapter applies nothing at all, so the refusals above say \
         nothing about the guard"
    );
}

/// The join: a Witch-Time stance slows the fighter who swung, through both
/// systems in the shipped order.
///
/// The counter's tests prove it dispatches to the attacker; the others here
/// prove the adapter applies a scale. In `lib.rs` the dilation adapter runs
/// before the counter's answer, so the `ActorActionMessage` is read on the
/// following tick. That works because messages survive a frame. The fixture
/// uses the shipped order.
#[test]
fn a_witch_time_stance_slows_the_attacker_across_the_frame_boundary() {
    use ambition_platformer2d::entity_catalog::smash_counter::CounterParams;
    use ambition_platformer2d::combat::hitbox::ParriedBodyHit;
    use ambition_platformer2d::combat::moveset::MovePlayback;

    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.add_message::<ParriedBodyHit>();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    {
        let mut time = app
            .world_mut()
            .resource_mut::<ambition_platformer2d::time::WorldTime>();
        time.scaled_dt = 1.0 / 60.0;
        time.raw_dt = 1.0 / 60.0;
    }
    // The shipped order: the adapter first, the counter's answer second.
    app.add_systems(
        Update,
        (
            apply_authored_time_dilations,
            expire_time_dilations,
            crate::counter::hold_counter_parry_windows,
            crate::counter::answer_a_parry_with_the_authored_counter,
        )
            .chain(),
    );

    let stance_params = CounterParams {
        window_s: 0.05,
        answers_the_attacker: true,
        response: TIME_DILATION.to_string(),
        response_params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
            &TimeDilationParams {
                scale: 0.35,
                seconds: 0.45,
            },
        )
        .expect("dilation params serialize"),
        absorbs_projectiles: false,
    };
    let mut spec = ambition_platformer2d::entity_catalog::smash_counter::counter_move(
        "probe_stance",
        "special",
        0.05,
        0.20,
        0.20,
        stance_params,
    );
    spec.duration_s = 4.0;
    let stance = spec.windows[1].clone();
    let mut playback = MovePlayback::new(spec, 1.0);
    playback.t = (stance.start_s + stance.end_s) * 0.5;

    let defender = app
        .world_mut()
        .spawn((
            playback,
            ae::BodyShieldState::default(),
            ambition_platformer2d::time::ProperTimeScale::default(),
        ))
        .id();
    let attacker = app
        .world_mut()
        .spawn(ambition_platformer2d::time::ProperTimeScale::default())
        .id();

    app.world_mut().write_message(ParriedBodyHit {
        defender,
        attacker,
        hitbox: attacker,
        contact: ae::Vec2::ZERO,
    });
    // Two ticks: the answer is written on the first and read on the second.
    app.update();
    app.update();

    assert_eq!(
        scale_of(&app, attacker),
        0.35,
        "the fighter who swung is running at full speed — the counter's answer \
         never reached the dilation adapter, which is the join neither side's \
         tests can see"
    );
    assert_eq!(
        scale_of(&app, defender),
        1.0,
        "the STANCE'S OWNER was slowed instead of, or as well as, the attacker — \
         a Witch-Time that stuns its own caster"
    );
}

/// Set the world's tick to an exact binary fraction, so `remaining_s -= dt`
/// repeated N times reaches zero exactly. `1/60` is not representable in `f32`
/// and can leave a crumb that buys a spurious extra tick.
fn tick_exactly(app: &mut App, dt: f32) {
    let mut time = app
        .world_mut()
        .resource_mut::<ambition_platformer2d::time::WorldTime>();
    time.scaled_dt = dt;
    time.raw_dt = dt;
}

/// A one-tick dilation is still in force when the next tick reads it.
///
/// `PlayerSimulation` runs before `Combat` (where `ContentSpecials` is), so a
/// scale written here is first observed on the next tick. If the same tick
/// also spent a tick of the countdown, N authored ticks would give N-1 slowed
/// ticks, and N=1 none. The duration test above samples only a midpoint and
/// an end, so it cannot see this.
#[test]
fn a_one_tick_dilation_is_still_in_force_next_tick() {
    let dt = 1.0 / 64.0;
    let mut app = app();
    tick_exactly(&mut app, dt);
    let who = victim(&mut app);

    ask(&mut app, who, 0.5, dt);

    assert_eq!(
        scale_of(&app, who),
        0.5,
        "a dilation authored for exactly one tick was already gone when the tick \
         that applied it ended, so no tick ever ran at the slowed scale. The \
         sweep must not spend the tick that armed the countdown."
    );
}

/// The general form: an authored duration buys exactly that many slowed ticks.
/// Only a count can tell N from N-1; several durations, because an off-by-one
/// can hide at one length.
#[test]
fn an_authored_duration_buys_exactly_that_many_slowed_ticks() {
    let dt = 1.0 / 64.0;
    for ticks in [1_u32, 2, 5, 16] {
        let mut app = app();
        tick_exactly(&mut app, dt);
        let who = victim(&mut app);

        ask(&mut app, who, 0.5, dt * ticks as f32);

        // The scale a tick leaves behind is what the next tick's player
        // simulation reads (it runs before this phase), so each loop pass is
        // one tick at the slow scale.
        let mut slowed = 0_u32;
        while scale_of(&app, who) < 1.0 {
            slowed += 1;
            assert!(
                slowed <= 64,
                "a {ticks}-tick dilation never expired; the countdown is not \
                 spending the world's clock"
            );
            app.update();
        }

        assert_eq!(
            slowed, ticks,
            "a dilation authored for {ticks} tick(s) slowed {slowed}. A sweep \
             chained after the apply eats the first one, so this reads {} \
             whenever the order regresses.",
            ticks.saturating_sub(1)
        );
    }
}
