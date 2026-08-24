use super::*;
use crate::feel::Platformer2dFeelTuningMonolith;
use crate::hitbox::LandedBodyHit;
use ambition_time::SimTick;

fn app() -> App {
    let mut app = App::new();
    app.add_message::<LandedBodyHit>();
    app.init_resource::<ImpactHitstop>();
    app.init_resource::<SimTick>();
    app.insert_resource(Platformer2dFeelTuningMonolith::default());
    app.add_systems(Update, request_impact_hitstop_on_landed_hits);
    app
}

fn a_volume() -> ambition_platformer2d_core::CombatVolume {
    ambition_platformer2d_core::CombatVolume::Aabb(
        ambition_platformer2d_core::CenteredAabb::new(
            ambition_platformer2d_core::Vec2::ZERO,
            ambition_platformer2d_core::Vec2::new(8.0, 8.0),
        )
        .aabb(),
    )
}

fn land_a_hit(app: &mut App, hitlag: f32) {
    let e = app
        .world_mut()
        .spawn(ambition_characters::actor::BodyCombat {
            hitstop_timer: hitlag,
            ..Default::default()
        })
        .id();
    app.world_mut().write_message(LandedBodyHit {
        hitbox: e,
        attacker: e,
        victim: e,
        volume: a_volume(),
        contact: ambition_platformer2d_core::Vec2::ZERO,
    });
}

fn now(app: &App) -> SimTick {
    *app.world().resource::<SimTick>()
}

fn advance(app: &mut App, ticks: u64) {
    app.world_mut().resource_mut::<SimTick>().0 += ticks;
}

/// ⭐⭐ THE POINT OF THE WHOLE MODULE: A CONNECT BETWEEN TWO BODIES NOBODY IS
/// PLAYING STILL STOPS THE WORLD.
///
/// ⛔ there is no `PrimaryPlayer` anywhere in this fixture, deliberately:
/// inventing one is the fix this refuses.
#[test]
fn two_cpus_trading_hits_freeze_the_world_with_no_player_in_it() {
    let mut app = app();
    let hitlag = Platformer2dFeelTuningMonolith::default().hitlag_time;
    land_a_hit(&mut app, hitlag);
    app.update();
    assert!(
        app.world().resource::<ImpactHitstop>().is_freezing(now(&app)),
        "two CPUs connected and the world did not freeze — the beat is still \
         scoped to a seat"
    );
}

/// ⛔⛔ THE FREEZE ENDS WHILE THE CLOCK IT STOPS IS STOPPED.
///
/// This is the design constraint the whole shape exists for. `SimTick` advances
/// whether or not `sim_dt` is zero, so an expiry measured against it cannot be
/// frozen by the freeze. A remaining-seconds counter aged on the sim clock —
/// the obvious implementation — never reaches zero.
#[test]
fn a_freeze_expires_against_a_tick_that_keeps_advancing() {
    let mut app = app();
    let hitlag = Platformer2dFeelTuningMonolith::default().hitlag_time;
    land_a_hit(&mut app, hitlag);
    app.update();
    let bound = (hitlag * TICKS_PER_SECOND).round() as u64;
    // One short of the expiry: still frozen.
    advance(&mut app, bound - 1);
    app.update();
    assert!(
        app.world().resource::<ImpactHitstop>().is_freezing(now(&app)),
        "the freeze ended early, before its authored length"
    );
    // ...and on the expiry tick it is over, with nothing handing the clock back.
    advance(&mut app, 1);
    app.update();
    assert!(
        !app.world().resource::<ImpactHitstop>().is_freezing(now(&app)),
        "the freeze outlived its bound — the world is stopped and nothing will \
         start it again"
    );
}

/// ⭐ OVERLAPPING CONNECTS ARE `max`, WHICH NEEDS NO ORDERING RULE. A second hit
/// mid-freeze extends the expiry to its own; it never sums, so a multi-hit move
/// cannot stop the world for longer than its hardest connect.
#[test]
fn overlapping_connects_extend_rather_than_accumulate() {
    let mut app = app();
    let hitlag = Platformer2dFeelTuningMonolith::default().hitlag_time;
    let bound = (hitlag * TICKS_PER_SECOND).round() as u64;

    land_a_hit(&mut app, hitlag);
    app.update();
    let first = app.world().resource::<ImpactHitstop>().until_tick;

    // Five more connects on the same tick buy exactly the same expiry.
    for _ in 0..5 {
        land_a_hit(&mut app, hitlag);
    }
    app.update();
    assert_eq!(
        app.world().resource::<ImpactHitstop>().until_tick,
        first,
        "connects on one tick accumulated instead of combining with max"
    );

    // A connect LATER in the freeze extends it, and by its own length.
    advance(&mut app, 2);
    land_a_hit(&mut app, hitlag);
    app.update();
    assert_eq!(
        app.world().resource::<ImpactHitstop>().until_tick,
        Some(now(&app).0 + bound),
        "a connect during a freeze did not extend it to its own length"
    );
}

/// ⛔ A WEAK CONNECT BARELY STOPS THE SCREEN, and an over-long authored hitlag
/// buys exactly what the hardest hit buys. A flat rate was measured making every
/// jab stop the world as long as a smash.
#[test]
fn the_freeze_is_proportional_to_the_hit_and_bounded_above() {
    let read = |hitlag: f32| {
        let mut app = app();
        land_a_hit(&mut app, hitlag);
        app.update();
        app.world().resource::<ImpactHitstop>().until_tick
    };
    let hard = Platformer2dFeelTuningMonolith::default().hitlag_time;
    assert!(
        read(hard * 0.25) < read(hard),
        "a weak connect stopped the world as long as a hard one"
    );
    assert_eq!(
        read(hard * 4.0),
        read(hard),
        "an over-long authored hitlag escaped the bound"
    );
}
