//! Clock-arbitration tests for regime permissions and reset behavior.

use super::*;
use crate::{ClockDomain, ClockObserver, ClockState, ProperTimeScale};

#[test]
fn proper_time_scale_default_is_one() {
    let pts = ProperTimeScale::default();
    assert_eq!(pts, ProperTimeScale::ONE);
    assert_eq!(pts.value(), 1.0);
}

#[test]
fn proper_time_scale_or_default_falls_back_to_one() {
    let some = ProperTimeScale(2.5);
    assert_eq!(ProperTimeScale::or_default(Some(&some)).value(), 2.5);
    assert_eq!(ProperTimeScale::or_default(None), ProperTimeScale::ONE);
}

#[test]
fn solo_regime_grants_every_requester_every_domain() {
    let policy = RegimePolicy {
        regime: Regime::Solo,
    };
    for requester in [
        ClockRequester::Player(ClockObserver::PRIMARY),
        ClockRequester::DevTool,
        ClockRequester::Scripted,
        ClockRequester::Engine,
        ClockRequester::Boss,
    ] {
        for domain in [
            ClockDomain::SimClock,
            ClockDomain::PlayerClock(crate::ClockObserver::PRIMARY),
            ClockDomain::WallClock,
        ] {
            assert_eq!(
                policy.permission_for(requester, domain),
                Permission::Grant,
                "Solo must grant {:?} -> {:?}",
                requester,
                domain,
            );
        }
    }
}

#[test]
fn rl_deterministic_denies_every_request() {
    let policy = RegimePolicy {
        regime: Regime::RLDeterministic,
    };
    assert_eq!(
        policy.permission_for(
            ClockRequester::Player(ClockObserver::PRIMARY),
            ClockDomain::SimClock,
        ),
        Permission::Deny,
    );
    assert_eq!(
        policy.permission_for(ClockRequester::Scripted, ClockDomain::SimClock),
        Permission::Deny,
    );
}

#[test]
fn cinematic_grants_scripted_denies_player() {
    let policy = RegimePolicy {
        regime: Regime::Cinematic,
    };
    assert_eq!(
        policy.permission_for(ClockRequester::Scripted, ClockDomain::SimClock),
        Permission::Grant,
    );
    assert_eq!(
        policy.permission_for(
            ClockRequester::Player(ClockObserver::PRIMARY),
            ClockDomain::SimClock,
        ),
        Permission::Deny,
    );
    // Engine retains authority during cinematics so pause /
    // suspended-gameplay still works.
    assert_eq!(
        policy.permission_for(ClockRequester::Engine, ClockDomain::SimClock),
        Permission::Grant,
    );
}

/// A granted request updates the requested scale through the dispatch pipeline.
#[test]
fn solo_grant_writes_requested_clock_scale() {
    let mut app = App::new();
    app.add_message::<ClockScaleRequest>()
        .insert_resource(RegimePolicy::default())
        .insert_resource(RequestedClockScale::default())
        .add_systems(Update, apply_clock_scale_requests);

    app.world_mut().write_message(ClockScaleRequest {
        domain: ClockDomain::SimClock,
        scale: 0.125,
        requester: ClockRequester::Player(ClockObserver::PRIMARY),
        reason: "bullet_blink_test",
    });

    app.update();

    let target = app.world().resource::<RequestedClockScale>();
    assert!((target.sim_clock - 0.125).abs() < 1e-6);
}

/// Multiple requests reduce by strength rather than scheduler order.
/// Assert both input orders to keep the result order-independent.
#[test]
fn the_strongest_slow_wins_regardless_of_who_asked_last() {
    for reversed in [false, true] {
        let mut app = App::new();
        app.add_message::<ClockScaleRequest>()
            .insert_resource(RegimePolicy::default())
            .insert_resource(RequestedClockScale::default())
            .add_systems(Update, apply_clock_scale_requests);

        let beat = ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 0.35,
            requester: ClockRequester::Engine,
            reason: "transform_beat",
        };
        let idle = ClockScaleRequest {
            scale: 1.0,
            reason: "default",
            ..beat
        };
        let order = if reversed { [idle, beat] } else { [beat, idle] };
        for request in order {
            app.world_mut().write_message(request);
        }

        app.update();

        let target = app.world().resource::<RequestedClockScale>();
        assert!(
            (target.sim_clock - 0.35).abs() < 1e-6,
            "the idle 1.0 overrode a live dilation (reversed: {reversed}, got {})",
            target.sim_clock,
        );
    }
}

#[test]
fn rl_regime_denies_blocks_the_scale_change() {
    let mut app = App::new();
    app.add_message::<ClockScaleRequest>()
        .insert_resource(RegimePolicy {
            regime: Regime::RLDeterministic,
        })
        .insert_resource(RequestedClockScale::default())
        .add_systems(Update, apply_clock_scale_requests);

    app.world_mut().write_message(ClockScaleRequest {
        domain: ClockDomain::SimClock,
        scale: 0.125,
        requester: ClockRequester::Player(ClockObserver::PRIMARY),
        reason: "denied_test",
    });

    app.update();

    let target = app.world().resource::<RequestedClockScale>();
    assert!(
        (target.sim_clock - 1.0).abs() < 1e-6,
        "RL must keep sim-clock target at 1.0",
    );
}

/// Wall clock is by definition unscaled. A grant targeting it is
/// a no-op so the host's real-time tick keeps advancing.
#[test]
fn wall_clock_grant_does_not_mutate_target() {
    let mut app = App::new();
    app.add_message::<ClockScaleRequest>()
        .insert_resource(RegimePolicy::default())
        .insert_resource(RequestedClockScale::default())
        .add_systems(Update, apply_clock_scale_requests);

    app.world_mut().write_message(ClockScaleRequest {
        domain: ClockDomain::WallClock,
        scale: 0.25,
        requester: ClockRequester::DevTool,
        reason: "wall_noop_test",
    });

    app.update();

    let target = app.world().resource::<RequestedClockScale>();
    assert!((target.sim_clock - 1.0).abs() < 1e-6);
}

#[test]
fn reset_request_snaps_current_and_requested_clock_scale() {
    let mut app = App::new();
    app.add_message::<ClockResetRequest>()
        .insert_resource(RegimePolicy::default())
        .insert_resource(RequestedClockScale { sim_clock: 0.125 })
        .insert_resource(ClockState { time_scale: 0.125 })
        .add_systems(Update, apply_clock_reset_requests);

    app.world_mut().write_message(ClockResetRequest::sim_clock(
        ClockRequester::Engine,
        "reset_test",
    ));

    app.update();

    let target = app.world().resource::<RequestedClockScale>();
    let clock = app.world().resource::<ClockState>();
    assert!((target.sim_clock - 1.0).abs() < 1e-6);
    assert!((clock.time_scale - 1.0).abs() < 1e-6);
}
