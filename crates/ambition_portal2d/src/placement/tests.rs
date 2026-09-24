use super::*;
use crate::color::{PortalChannel, PortalChannelColor};
use crate::types::portal_half_extent;

fn floor(channel: PortalChannel, pos: Vec2) -> PlacedPortal {
    PlacedPortal::fixed(
        channel,
        pos,
        Vec2::new(0.0, -1.0),
        portal_half_extent(Vec2::new(0.0, -1.0)),
    )
}

const PURPLE: PortalChannel = PortalChannel::Authored(PortalChannelColor::Purple);
const YELLOW: PortalChannel = PortalChannel::Authored(PortalChannelColor::Yellow);

/// Give a fixture portal host motion: it moved by `delta` this frame and has
/// velocity `vel` px/s. The machine reads only `host.is_some()` and the
/// pos/prev_pos/vel caches.
fn hosted_moving(mut portal: PlacedPortal, delta: Vec2, vel: Vec2) -> PlacedPortal {
    portal.host = Some(ae::GeoFaceRef::new(ae::GeoId::anon(), ae::Face::Top, 0.0));
    portal.prev_pos = portal.pos - delta;
    portal.vel = vel;
    portal
}

/// "Scoop": a moving aperture that sweeps over a stationary body transits
/// it. The relative segment is nonzero, although the body did not move.
#[test]
fn a_moving_portal_scoops_a_stationary_body() {
    // A rising floor aperture (e.g. on an elevator) sweeps up past a still
    // body's center: in the aperture's frame, the body crossed front→behind.
    let enter = hosted_moving(
        floor(PURPLE, Vec2::new(100.0, 288.0)), // rose from y=300
        Vec2::new(0.0, -12.0),
        Vec2::new(0.0, -720.0),
    );
    let exit = floor(YELLOW, Vec2::new(500.0, 600.0));
    let portals = [enter, exit];
    let center = Vec2::new(100.0, 294.0); // stationary body, plane rose past it
    let step = transit_step_with_tuning(
        center,
        Vec2::new(24.0, 40.0),
        Vec2::ZERO,
        Some(SweptSample {
            pos: center,
            vel: Vec2::ZERO,
        }), // no body motion
        None,
        None,
        &portals,
        Vec2::new(0.0, 1.0),
        &super::super::types::PortalHostDepths::default(),
        &PortalTuning::default(),
    );
    match step {
        TransitStep::Transfer { pos, vel, .. } => {
            // Emerges at the exit's image of the crossing, moving out of the
            // exit face with the aperture-relative approach speed.
            assert!((pos.x - 500.0).abs() < 60.0, "exit x, got {pos:?}");
            assert!(
                vel.dot(Vec2::new(0.0, -1.0)) > 0.0,
                "exits OUT of the exit face, got {vel:?}"
            );
        }
        other => panic!("expected the scoop to Transfer, got {other:?}"),
    }
}

/// A body moving with its aperture (on the same host) has a zero relative
/// segment, so no swept transfer happens.
#[test]
fn a_body_co_moving_with_the_aperture_never_swept_transfers() {
    let delta = Vec2::new(0.0, 12.0);
    let enter = hosted_moving(
        floor(PURPLE, Vec2::new(100.0, 300.0)),
        delta,
        Vec2::new(0.0, 720.0),
    );
    let exit = floor(YELLOW, Vec2::new(500.0, 600.0));
    let portals = [enter, exit];
    // Body 60px above the opening (outside the capture box), riding the
    // same host: it moved by exactly the aperture's delta this frame.
    let center = Vec2::new(100.0, 240.0);
    let step = transit_step_with_tuning(
        center,
        Vec2::new(24.0, 40.0),
        Vec2::new(0.0, 720.0),
        Some(SweptSample {
            pos: center - delta,
            vel: Vec2::new(0.0, 720.0),
        }),
        None,
        None,
        &portals,
        Vec2::new(0.0, 1.0),
        &super::super::types::PortalHostDepths::default(),
        &PortalTuning::default(),
    );
    assert!(
        matches!(step, TransitStep::Idle),
        "co-moving body must not transit, got {step:?}"
    );
}

/// Galilean transfer: velocity maps relative to the entry aperture and adds
/// the exit aperture's motion. The min-exit-speed floor applies in the exit's
/// rest frame.
#[test]
fn transfer_velocity_composes_galilean_and_floors_in_the_exit_rest_frame() {
    let tuning = PortalTuning::default();
    // Entry aperture rising to meet a slowly falling body.
    let enter = hosted_moving(
        floor(PURPLE, Vec2::new(100.0, 300.0)),
        Vec2::new(0.0, -2.0),
        Vec2::new(0.0, -120.0),
    );
    // Exit aperture itself moving along +x at 90 px/s.
    let exit = hosted_moving(
        floor(YELLOW, Vec2::new(500.0, 600.0)),
        Vec2::new(1.5, 0.0),
        Vec2::new(90.0, 0.0),
    );
    // Approach at 180 px/s down: relative approach = 300 px/s into the
    // entry (the aperture rises 120 to meet it). The mapped rest-frame
    // exit speed (300) clears the default floor (220); the exit's own +x
    // motion rides on top.
    let vel_out = match transfer_step(
        Vec2::new(100.0, 301.0),
        Vec2::new(0.0, 180.0),
        enter.clone(),
        exit.clone(),
        Vec2::new(0.0, 1.0),
        &tuning,
    ) {
        TransitStep::Transfer { vel, .. } => vel,
        other => panic!("expected Transfer, got {other:?}"),
    };
    let exit_normal = Vec2::new(0.0, -1.0);
    let rest_frame_out = (vel_out - exit.vel).dot(exit_normal);
    assert!(
        (rest_frame_out - 300.0).abs() < 1e-3,
        "rest-frame exit speed should be the relative approach speed, got {rest_frame_out}"
    );
    assert!(
        (vel_out.x - exit.vel.x).abs() < 1e-3,
        "the exit aperture's own motion rides on top, got {vel_out:?}"
    );

    // A very slow approach (5 px/s relative) floors to min_exit_speed in the
    // rest frame. The exit's own velocity must not count toward the floor.
    let slow_enter = floor(PURPLE, Vec2::new(100.0, 300.0));
    let vel_out = match transfer_step(
        Vec2::new(100.0, 301.0),
        Vec2::new(0.0, 5.0),
        slow_enter,
        exit.clone(),
        Vec2::new(0.0, 1.0),
        &tuning,
    ) {
        TransitStep::Transfer { vel, .. } => vel,
        other => panic!("expected Transfer, got {other:?}"),
    };
    let rest_frame_out = (vel_out - exit.vel).dot(exit_normal);
    assert!(
        (rest_frame_out - tuning.min_exit_speed).abs() < 1e-3,
        "rest-frame floor, got {rest_frame_out} (floor {})",
        tuning.min_exit_speed
    );
}

/// A fast fall (about 63 px per step vs a ~40 px body) can skip the straddle
/// frames and end fully below the entry plane inside the open carve, with
/// Begin blocked by the cooldown. The rescue must still transfer it.
#[test]
fn rescue_transfers_a_deep_crossing_inside_the_carve_even_on_cooldown() {
    let portals = [
        floor(PURPLE, Vec2::new(100.0, 300.0)),
        floor(YELLOW, Vec2::new(500.0, 300.0)),
    ];
    // Body (24x40) entirely below the plane (top edge y=315 > 300) but
    // within the carve volume, still falling in, mid ping-pong cooldown.
    let step = transit_step(
        Vec2::new(100.0, 335.0),
        Vec2::new(24.0, 40.0),
        Vec2::new(0.0, 1600.0),
        None,
        Some(PURPLE), // cooldown latched — Begin blocked, only the rescue can act
        &portals,
        Vec2::new(0.0, 1.0),
    );
    match step {
        TransitStep::Transfer { pos, .. } => {
            assert!(
                pos.y < 300.0,
                "the transfer emerges in FRONT of the exit plane, got {pos:?}"
            );
        }
        other => panic!("a deep carve crossing must transfer, got {other:?}"),
    }
}

fn wall_portal(channel: PortalChannel, pos: Vec2, normal: Vec2) -> PlacedPortal {
    PlacedPortal::fixed(channel, pos, normal, portal_half_extent(normal))
}

/// Thin-wall guard: with the host wall measured at 24 px, the rescue volume
/// ends at the wall's far face. A body in the room behind the wall is not
/// teleported; a deep crossing inside the material still transfers.
#[test]
fn rescue_is_bounded_by_the_measured_host_depth() {
    use crate::types::PortalHostDepths;
    // Left face of a 24px wall spanning x ∈ [500, 524].
    let a = wall_portal(PURPLE, Vec2::new(500.0, 450.0), Vec2::new(-1.0, 0.0));
    let b = wall_portal(YELLOW, Vec2::new(100.0, 450.0), Vec2::new(-1.0, 0.0));
    let portals = [a, b];
    let depths = PortalHostDepths(vec![(PURPLE, 24.0), (YELLOW, 24.0)]);
    // A body behind the wall (40 px past A's plane, inside the unclipped
    // 60 px hole) moving deeper must stay Idle.
    let step = transit_step_with_tuning(
        Vec2::new(540.0, 450.0),
        Vec2::new(24.0, 40.0),
        Vec2::new(80.0, 0.0), // moving +x = away from A's face = vel·n < 0
        None,
        None,
        None,
        &portals,
        Vec2::new(0.0, 1.0),
        &depths,
        &PortalTuning::default(),
    );
    assert!(
        matches!(step, TransitStep::Idle),
        "a body in the open room behind a thin wall must never be rescued, got {step:?}"
    );
}

/// A body against the back of a thin host wall must not Begin transit. The
/// capture box reaches through thin material, so Begin checks the front side.
#[test]
fn begin_requires_the_front_side_of_the_plane() {
    // Portal on the left face of a thin wall. The body is 12 px behind the
    // face (inside the capture box) and moves away from it (vel·n < 0).
    let a = wall_portal(PURPLE, Vec2::new(500.0, 450.0), Vec2::new(-1.0, 0.0));
    let b = wall_portal(YELLOW, Vec2::new(100.0, 450.0), Vec2::new(-1.0, 0.0));
    let portals = [a, b];
    let step = transit_step(
        Vec2::new(512.0, 450.0),
        Vec2::new(4.0, 4.0), // small so it fits + overlaps the thin box
        Vec2::new(80.0, 0.0),
        None,
        None,
        &portals,
        Vec2::new(0.0, 1.0),
    );
    assert!(
        !matches!(step, TransitStep::Begin { .. }),
        "no Begin from behind the plane, got {step:?}"
    );
}

/// The post-crossing cooldown is scoped to the crossed pair: it blocks
/// re-Begin into that pair but leaves a DIFFERENT pair enterable.
#[test]
fn cooldown_is_pair_scoped() {
    use crate::color::PortalChannelColor;
    const TEAL: PortalChannel = PortalChannel::Authored(PortalChannelColor::Teal);
    const RED: PortalChannel = PortalChannel::Authored(PortalChannelColor::Red);
    let portals = [
        floor(PURPLE, Vec2::new(100.0, 300.0)),
        floor(YELLOW, Vec2::new(500.0, 300.0)),
        floor(TEAL, Vec2::new(900.0, 300.0)),
        floor(RED, Vec2::new(1300.0, 300.0)),
    ];
    // Body resting on the TEAL portal, latched against the PURPLE pair.
    let step = transit_step(
        Vec2::new(900.0, 285.0),
        Vec2::new(24.0, 40.0),
        Vec2::new(0.0, 40.0),
        None,
        Some(PURPLE),
        &portals,
        Vec2::new(0.0, 1.0),
    );
    assert!(
        matches!(step, TransitStep::Begin { channel, .. } if channel == TEAL),
        "a different pair must stay enterable during the cooldown, got {step:?}"
    );
    // The latched pair itself (either end) is refused.
    let step = transit_step(
        Vec2::new(500.0, 285.0),
        Vec2::new(24.0, 40.0),
        Vec2::new(0.0, 40.0),
        None,
        Some(PURPLE),
        &portals,
        Vec2::new(0.0, 1.0),
    );
    assert!(
        matches!(step, TransitStep::Idle),
        "the crossed pair stays latched during the cooldown, got {step:?}"
    );
}

/// The carve volume bounds the rescue: a body genuinely below the surface
/// (past the carve depth) is never teleported.
#[test]
fn rescue_never_grabs_a_body_past_the_carve_depth() {
    let portals = [
        floor(PURPLE, Vec2::new(100.0, 300.0)),
        floor(YELLOW, Vec2::new(500.0, 300.0)),
    ];
    let step = transit_step(
        Vec2::new(100.0, 420.0), // top edge y=400, past the 60px carve
        Vec2::new(24.0, 40.0),
        Vec2::new(0.0, 400.0),
        None,
        Some(PURPLE),
        &portals,
        Vec2::new(0.0, 1.0),
    );
    assert!(
        matches!(step, TransitStep::Idle),
        "a body below the carve volume must not be rescued, got {step:?}"
    );
}

fn ceiling(channel: PortalChannel, pos: Vec2) -> PlacedPortal {
    PlacedPortal::fixed(
        channel,
        pos,
        Vec2::new(0.0, 1.0),
        portal_half_extent(Vec2::new(0.0, 1.0)),
    )
}

/// Swept (CCD) transit on a floor→ceiling pair that forms an accelerating
/// fall loop with a relaxed fall cap. The discrete tiers are sized for about
/// 63 px per frame (`APPROACH_CARVE_REACH`, `CARVE_DEPTH`); faster steps skip
/// both. The swept tier must transfer every cycle, beyond 800 px per frame,
/// with the pair cooldown latched as in the live system.
#[test]
fn swept_tier_transfers_the_accelerating_fall_loop_at_any_speed() {
    let floor_y = 300.0;
    let ceiling_y = floor_y - 680.0;
    let portals = [
        floor(PURPLE, Vec2::new(100.0, floor_y)),
        ceiling(YELLOW, Vec2::new(100.0, ceiling_y)),
    ];
    let size = Vec2::new(24.0, 40.0);
    let dt = 1.0 / 30.0;
    let gravity = 4000.0; // px/s², no fall cap — the loop accelerates forever
    let tuning = PortalTuning::default();
    let depths = crate::types::PortalHostDepths::default();

    let mut pos = Vec2::new(100.0, ceiling_y + 40.0);
    let mut vel = Vec2::new(0.0, 200.0);
    let mut prev = SweptSample { pos, vel };
    let mut transit: Option<PortalTransit> = None;
    let mut cooldown: Option<(PortalChannel, f32)> = None;
    let mut transfers = 0u32;
    let mut peak_step = 0.0f32;

    // 140 frames at g=4000 peaks near 630 px per frame. This stays under the
    // one-crossing-per-step bound (the loop is 680 px).
    for frame in 0..140 {
        let step = transit_step_with_tuning(
            pos,
            size,
            vel,
            Some(prev),
            transit,
            cooldown.map(|(c, _)| c),
            &portals,
            Vec2::new(0.0, 1.0),
            &depths,
            &tuning,
        );
        match step {
            TransitStep::Begin { channel, .. } => {
                transit = Some(PortalTransit {
                    straddling: channel,
                    crossed: false,
                });
            }
            TransitStep::Transfer {
                pos: p,
                vel: v,
                exit_channel,
                ..
            } => {
                pos = p;
                vel = v;
                transfers += 1;
                cooldown = Some((exit_channel, tuning.teleport_cooldown_s));
                transit = transit.map(|mut t| {
                    t.crossed = true;
                    t.straddling = exit_channel;
                    t
                });
            }
            TransitStep::Clear => transit = None,
            TransitStep::Idle | TransitStep::Continue => {}
        }

        // No-embed invariant: the body may be past the floor plane only in
        // the frame it crossed. Still below it here means every tier missed.
        assert!(
            pos.y <= floor_y + 1.0,
            "frame {frame}: body ended {}px past the floor plane at \
             {:.0}px/frame — the transit trigger tunneled",
            pos.y - floor_y,
            vel.y * dt,
        );

        // As in the live system: record post-step pos/vel, then integrate one
        // ballistic frame.
        prev = SweptSample { pos, vel };
        vel.y += gravity * dt;
        pos.y += vel.y * dt;
        peak_step = peak_step.max(vel.y * dt);
        cooldown = cooldown.and_then(|(c, t)| {
            let t = t - dt;
            (t > 0.0).then_some((c, t))
        });
    }

    assert!(
        peak_step > 500.0,
        "the loop must actually reach tunneling speeds, peaked at {peak_step:.0}px/frame",
    );
    assert!(
        transfers > 40,
        "the loop must keep cycling (one transfer per crossing), got {transfers}",
    );
}

/// Swept-tier teleport guard: a prev→now segment much longer than one frame
/// of travel (a respawn, reset, or teleport) is not travel through an
/// aperture, even if the line crosses the portal plane inside the opening.
#[test]
fn swept_tier_ignores_teleport_sized_segments() {
    let portals = [
        floor(PURPLE, Vec2::new(100.0, 300.0)),
        floor(YELLOW, Vec2::new(500.0, 300.0)),
    ];
    // "Respawned" from far above the portal to far below it. At 100 px/s, the
    // 800 px segment is far more than one frame of motion.
    let step = transit_step_with_tuning(
        Vec2::new(100.0, 700.0),
        Vec2::new(24.0, 40.0),
        Vec2::ZERO,
        Some(SweptSample {
            pos: Vec2::new(100.0, -100.0),
            vel: Vec2::new(0.0, 100.0),
        }),
        None,
        None,
        &portals,
        Vec2::new(0.0, 1.0),
        &crate::types::PortalHostDepths::default(),
        &PortalTuning::default(),
    );
    assert!(
        matches!(step, TransitStep::Idle),
        "a teleport-sized segment must not sweep through a portal, got {step:?}"
    );
}

/// The swept tier keeps the entry momentum even when the integrator already
/// stopped the body at the carve bottom. The previous sample proves the
/// crossing and gives the exit velocity.
#[test]
fn swept_tier_transfers_a_stopped_body_with_its_entry_momentum() {
    let portals = [
        floor(PURPLE, Vec2::new(100.0, 300.0)),
        ceiling(YELLOW, Vec2::new(100.0, -380.0)),
    ];
    // Last frame: 90 px above the plane, falling 15000 px/s (500 px/frame).
    // This frame: stopped 110 px past the plane (beyond the 60 px carve, so
    // the rescue cannot see it) with zero velocity.
    let step = transit_step_with_tuning(
        Vec2::new(100.0, 410.0),
        Vec2::new(24.0, 40.0),
        Vec2::ZERO,
        Some(SweptSample {
            pos: Vec2::new(100.0, 210.0),
            vel: Vec2::new(0.0, 15000.0),
        }),
        None,
        Some(PURPLE), // even mid ping-pong cooldown
        &portals,
        Vec2::new(0.0, 1.0),
        &crate::types::PortalHostDepths::default(),
        &PortalTuning::default(),
    );
    match step {
        TransitStep::Transfer { vel, .. } => {
            assert!(
                vel.y > 10000.0,
                "the exit must emit the ENTRY momentum, not the zeroed \
                 post-stop velocity; got {vel:?}"
            );
        }
        other => panic!("a swept crossing must transfer a stopped body, got {other:?}"),
    }
}

/// The transit uses the convention in its `PortalTuning`, not a process
/// global. Two sessions in one process can use different conventions, and
/// the order of the calls does not matter.
#[test]
fn two_sessions_in_one_process_keep_their_own_portal_conventions() {
    use crate::tuning::PortalConvention;

    // A pair whose two conventions genuinely disagree — same-facing walls.
    let enter = wall_portal(PURPLE, Vec2::new(500.0, 450.0), Vec2::new(-1.0, 0.0));
    let exit = wall_portal(YELLOW, Vec2::new(900.0, 450.0), Vec2::new(-1.0, 0.0));
    let gravity_dir = Vec2::new(0.0, 1.0);

    let roll_under = |convention| match transfer_step(
        Vec2::new(495.0, 450.0),
        Vec2::new(-200.0, 0.0),
        enter.clone(),
        exit.clone(),
        gravity_dir,
        &PortalTuning {
            convention,
            ..PortalTuning::default()
        },
    ) {
        TransitStep::Transfer { roll_delta, .. } => roll_delta,
        other => panic!("expected Transfer, got {other:?}"),
    };

    // Session A first, then B; then B first, then A. Four calls, two answers.
    let (a_first, b_second) = (
        roll_under(PortalConvention::Reflection),
        roll_under(PortalConvention::Rotation),
    );
    let (b_first, a_second) = (
        roll_under(PortalConvention::Rotation),
        roll_under(PortalConvention::Reflection),
    );
    assert_ne!(
        a_first, b_second,
        "the two conventions agree on this pair, so the fixture cannot show \
         contamination and this test proves nothing"
    );
    assert_eq!(
        (a_first, b_second),
        (a_second, b_first),
        "a session's portal physics changed because ANOTHER session ran first — \
         which is what a process-global convention does"
    );
}

#[test]
fn a_transit_takes_its_convention_from_tuning() {
    use crate::tuning::PortalConvention;
    use ambition_platformer2d_core::frame::MapConvention;

    // Two walls that face the same way: `portal_transit_roll` maps
    // `into_render = (-n_in.x, n_in.y)` against `out_render = (n_out.x,
    // -n_out.y)`, so both facing -x gives `atan2(0, -1) = π` under Rotation,
    // and Reflection's wall↔wall case gives 0.0. Two facing walls give 0
    // under both, so they cannot tell the conventions apart.
    //
    // Call `transfer_step` directly; the crossing logic is not under test.
    let enter = wall_portal(PURPLE, Vec2::new(500.0, 450.0), Vec2::new(-1.0, 0.0));
    let exit = wall_portal(YELLOW, Vec2::new(900.0, 450.0), Vec2::new(-1.0, 0.0));
    let gravity_dir = Vec2::new(0.0, 1.0);

    let roll_under = |convention| match transfer_step(
        Vec2::new(495.0, 450.0),
        Vec2::new(-200.0, 0.0),
        enter.clone(),
        exit.clone(),
        gravity_dir,
        &PortalTuning {
            convention,
            ..PortalTuning::default()
        },
    ) {
        TransitStep::Transfer { roll_delta, .. } => roll_delta,
        other => panic!("expected Transfer, got {other:?}"),
    };

    let by_reflection = crate::somersault_roll_for_convention(
        MapConvention::Reflection,
        enter.normal,
        exit.normal,
        gravity_dir,
    );
    let by_rotation = crate::somersault_roll_for_convention(
        MapConvention::Rotation,
        enter.normal,
        exit.normal,
        gravity_dir,
    );
    assert_ne!(
        by_reflection, by_rotation,
        "the two conventions agree on this portal pair, so the fixture cannot \
         distinguish them and this probe proves nothing"
    );

    assert_eq!(
        roll_under(PortalConvention::Rotation),
        by_rotation,
        "a transit tuned to Rotation did not roll by Rotation — the convention \
         reached it from somewhere other than its own tuning"
    );
    assert_eq!(
        roll_under(PortalConvention::Reflection),
        by_reflection,
        "a transit tuned to Reflection did not follow its tuning either"
    );
}
