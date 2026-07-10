//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

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

/// Give a fixture portal CC6 host motion: it moved by `delta` this frame
/// and carries `vel` px/s. (The machine only reads `host.is_some()` +
/// the pos/prev_pos/vel caches; resolution is the adapter's concern.)
fn hosted_moving(mut portal: PlacedPortal, delta: Vec2, vel: Vec2) -> PlacedPortal {
    portal.host = Some(ae::GeoFaceRef::new(ae::GeoId::anon(), ae::Face::Top, 0.0));
    portal.prev_pos = portal.pos - delta;
    portal.vel = vel;
    portal
}

/// §5-P2 "scoop": a moving aperture sweeping over a STATIONARY body
/// transits it — the relative segment is nonzero because the aperture
/// moved over the body. The body's own sweep sample shows no motion.
#[test]
fn a_moving_portal_scoops_a_stationary_body() {
    // A RISING floor aperture (elevator floor, portal in it) sweeps up
    // past a stationary body's center: in the aperture's frame the body
    // crossed front -> behind through the opening this frame.
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
            // Emerges at the exit's image of the crossing, riding OUT of
            // the exit face: the Galilean map hands the stationary body
            // the aperture-relative approach speed.
            assert!((pos.x - 500.0).abs() < 60.0, "exit x, got {pos:?}");
            assert!(
                vel.dot(Vec2::new(0.0, -1.0)) > 0.0,
                "exits OUT of the exit face, got {vel:?}"
            );
        }
        other => panic!("expected the scoop to Transfer, got {other:?}"),
    }
}

/// A body CO-MOVING with its aperture (standing on the same host) has a
/// zero relative segment — no spurious swept transfer, ever.
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

/// The Galilean transfer (§7): velocity maps RELATIVE to the entry
/// aperture and composes with the exit aperture's motion; the
/// min-exit-speed floor applies in the exit's REST frame.
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

    // A dead-slow approach (5 px/s relative) floors to min_exit_speed IN
    // THE REST FRAME — the exit's frame velocity must not satisfy the
    // floor on the aperture's behalf.
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

/// A fast fall can cross the whole straddle window between two sampled
/// frames (1900 px/s terminal ≈ 63 px at the 1/30 s sim-step clamp vs a
/// ~40 px body), leaving the body FULLY below the entry plane inside the
/// open carve, with the Begin path cooldown-blocked. The rescue must still
/// transfer it — the carve volume is the gate, not a same-frame straddle.
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

/// Thin-wall geometric guard: with the host wall measured at 24px, the
/// rescue's aperture volume ends at the wall's far face — a body standing
/// in the open room BEHIND the wall (well within the unclipped 60px carve
/// reach) is never teleported, while a genuine deep crossing inside the
/// material still transfers.
#[test]
fn rescue_is_bounded_by_the_measured_host_depth() {
    use crate::types::PortalHostDepths;
    // Left face of a 24px wall spanning x ∈ [500, 524].
    let a = wall_portal(PURPLE, Vec2::new(500.0, 450.0), Vec2::new(-1.0, 0.0));
    let b = wall_portal(YELLOW, Vec2::new(100.0, 450.0), Vec2::new(-1.0, 0.0));
    let portals = [a, b];
    let depths = PortalHostDepths(vec![(PURPLE, 24.0), (YELLOW, 24.0)]);
    // A body in the room BEHIND the wall (centroid 40px past A's plane —
    // inside the UNCLIPPED 60px hole) moving deeper: must stay Idle.
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

/// A body pressed against the BACK of a thin host wall must not Begin a
/// transit into a portal it cannot see — the capture box reaches through
/// thin material, so Begin gates on the FRONT side of the plane.
#[test]
fn begin_requires_the_front_side_of_the_plane() {
    // Portal on the left face of a thin wall; body just BEHIND the face
    // (12px past the plane — within the capture box's through-reach),
    // moving away from the face (vel·n < 0 reads as "entering").
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

/// §7.6 — the swept (CCD) transit tier, on the exact failing configuration:
/// a floor→ceiling translation pair forming an ACCELERATING fall loop under
/// a relaxed fall cap. The discrete tiers are sized for ~63 px/frame
/// (`APPROACH_CARVE_REACH` / `CARVE_DEPTH`); past that, one frame's step
/// jumps the body clean over the capture box AND the carve volume, no tier
/// fires, and the body lands embedded in the floor with its momentum
/// killed. The swept tier must transfer EVERY cycle, up past 800 px/frame,
/// with the pair cooldown latched exactly as the live system latches it.
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

    // 140 frames at g=4000 peaks ~630px/frame — past the ~500px/frame the
    // §7.6 report asked for, under the documented one-crossing-per-step
    // bound (a segment longer than the whole 680px loop can out-run one
    // transfer per frame; that regime is physically off the map).
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

        // The no-embed invariant: after the machine ran, the body may
        // overshoot the floor plane only within the frame it crossed it —
        // the NEXT machine call must have transferred it back out. A body
        // still below the plane here means every tier missed: embedded.
        assert!(
            pos.y <= floor_y + 1.0,
            "frame {frame}: body ended {}px past the floor plane at \
             {:.0}px/frame — the transit trigger tunneled",
            pos.y - floor_y,
            vel.y * dt,
        );

        // Anchor + physics (the pure-machine mirror of the live system:
        // record post-step pos/vel, then integrate one ballistic frame).
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

/// The swept tier's teleport guard: a prev→now segment far longer than one
/// frame of the previous velocity's ballistic travel (a respawn / reset /
/// scripted teleport) must NEVER read as travel through an aperture, even
/// when the straight line between the two points crosses the portal plane
/// inside the opening.
#[test]
fn swept_tier_ignores_teleport_sized_segments() {
    let portals = [
        floor(PURPLE, Vec2::new(100.0, 300.0)),
        floor(YELLOW, Vec2::new(500.0, 300.0)),
    ];
    // "Respawned" from far above the portal to far below it; the previous
    // velocity was a gentle 100 px/s — the 800px segment is two orders of
    // magnitude past one frame of that motion.
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

/// The swept tier carries the ENTRY momentum even when the integrator
/// already stopped the body (grounded at the carve bottom, velocity
/// zeroed) after it crossed — the exact §7.6 embed: the previous sample
/// proves the crossing and supplies the velocity the exit must emit.
#[test]
fn swept_tier_transfers_a_stopped_body_with_its_entry_momentum() {
    let portals = [
        floor(PURPLE, Vec2::new(100.0, 300.0)),
        ceiling(YELLOW, Vec2::new(100.0, -380.0)),
    ];
    // Last frame: 90px above the plane falling 15000 px/s (500 px/frame).
    // This frame: the integrator stopped it 110px past the plane (beyond
    // the 60px carve — the rescue can't see it) and zeroed its velocity.
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
