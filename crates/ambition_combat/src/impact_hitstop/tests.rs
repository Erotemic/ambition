use super::*;
use crate::feel::Platformer2dFeelTuningMonolith;
use crate::hitbox::ResolvedBodyHit;
use ambition_time::SimTick;

fn app() -> App {
    let mut app = App::new();
    app.add_message::<ResolvedBodyHit>();
    app.init_resource::<ImpactHitstop>();
    app.init_resource::<SimTick>();
    app.insert_resource(Platformer2dFeelTuningMonolith::default());
    app.add_systems(Update, request_impact_hitstop_on_resolved_hits);
    app
}

/// ⛔⛔ AND NOTHING HERE INJECTS A VICTIM'S TIMER ANY MORE, which is what every
/// arm in this file used to do: spawn a `BodyCombat`, set `hitstop_timer` on it
/// by hand, and fire a `LandedBodyHit` naming it. That fixture could not see the
/// bug the whole channel exists for — the timer is written on a DIFFERENT FRAME
/// for a player victim, and a test that writes it first has already answered the
/// question. The hitlag is the message's now, so these arms state it and the
/// production ordering is pinned where it lives, in
/// `ambition_app/tests/a_hit_on_the_player_freezes_the_match.rs`.
fn land_a_hit(app: &mut App, hitlag: f32) {
    land_a(app, hitlag, crate::HitSource::Melee);
}

fn land_a(app: &mut App, hitlag: f32, source: crate::HitSource) {
    let victim = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(ResolvedBodyHit {
        victim,
        // The hitstop consumer does not ask WHO landed it — a connect freezes
        // the match whoever threw it — so this fixture states the absence.
        attacker: None,
        hitlag_seconds: hitlag,
        source,
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
        app.world()
            .resource::<ImpactHitstop>()
            .is_freezing(now(&app)),
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
        app.world()
            .resource::<ImpactHitstop>()
            .is_freezing(now(&app)),
        "the freeze ended early, before its authored length"
    );
    // ...and on the expiry tick it is over, with nothing handing the clock back.
    advance(&mut app, 1);
    app.update();
    assert!(
        !app.world()
            .resource::<ImpactHitstop>()
            .is_freezing(now(&app)),
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

/// ⛔⛔ STANDING IN LAVA IS NOT A HIT CONNECTING.
///
/// `ResolvedBodyHit` comes off the RESOLVER, which serves contact attrition,
/// hazards and the blast zone as well as strikes — a broader channel than the
/// `LandedBodyHit` this used to read, which was strikes by construction.
/// Measured 2026-08-25: a fighter leaning on another produced SEVENTEEN
/// `Contact` resolutions in twenty-three ticks, and a freeze armed by those
/// alternated frozen and moving until three smash fixtures timed out waiting for
/// a gait no gameplay time was reaching.
///
/// ⭐ THE ARMS STRADDLE THE FILTER: the same hitlag, the same everything, one
/// source that is a connect and three that are not.
#[test]
fn attrition_and_the_blast_zone_do_not_stop_the_world() {
    let hitlag = Platformer2dFeelTuningMonolith::default().hitlag_time;
    let froze = |source: crate::HitSource| {
        let mut app = app();
        land_a(&mut app, hitlag, source);
        app.update();
        app.world().resource::<ImpactHitstop>().until_tick.is_some()
    };

    assert!(
        froze(crate::HitSource::Melee),
        "a swing connecting did not stop the world, so the refusals below are \
         a system that never freezes rather than one that is choosing"
    );
    assert!(
        froze(crate::HitSource::Projectile),
        "a shot connecting is a connect too"
    );
    for attrition in [
        crate::HitSource::Contact,
        crate::HitSource::Hazard,
        crate::HitSource::LeftTheWorld,
    ] {
        assert!(
            !froze(attrition.clone()),
            "{attrition:?} stopped the world — it fires once per overlapping \
             TICK, so a freeze armed by it never lets go"
        );
    }
}

/// ⛔⛔ A GUST STOPS THE WORLD FOR NOBODY, AND IT IS THE HITLAG THAT SAYS SO.
///
/// A windbox is authored as "pushes its victim and does nothing else", and a
/// melee windbox is still `HitSource::Melee` — so `is_a_connect` cannot tell it
/// from a punch and was never meant to. What separates them is the BEAT: a gust
/// owes its victim no hitlag (`apply_body_hit_reaction` declines to charge it
/// for the windbox reaction kind), and this system returns early when the
/// resolved hitlag is zero.
///
/// ⭐ SO THE FREEZE NEEDED NO RULE ABOUT WIND. That is why this arm exists
/// rather than a `ResolvedReactionKind` on the message: one fact — did this hit
/// earn a beat — answers both the victim's freeze and the match's, and a second
/// classification carried alongside it could disagree with the first.
///
/// ⭐ THE STRIKE ARM IS THE PREMISE GUARD: without it this passes against a
/// system that has stopped arming for anybody.
#[test]
fn a_connect_that_earned_no_hitlag_arms_no_match_freeze() {
    let mut gust = app();
    land_a_hit(&mut gust, 0.0);
    gust.update();
    assert_eq!(
        gust.world().resource::<ImpactHitstop>().until_tick,
        None,
        "a melee pulse that earned its victim no hitlag froze the whole match \
         anyway — which is what a windbox did, because the source alone says \
         `Melee` for a gust and a punch alike"
    );

    let mut blow = app();
    land_a_hit(&mut blow, 0.05);
    blow.update();
    assert!(
        blow.world().resource::<ImpactHitstop>().until_tick.is_some(),
        "an ordinary connect stopped arming the freeze, so the arm above proves \
         nothing about gusts"
    );
}
