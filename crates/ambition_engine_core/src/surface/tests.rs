//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::world::{SurfaceChain, SurfaceJunction, SurfacePort};

const DT: f32 = 1.0 / 60.0;
const G: Vec2 = Vec2::new(0.0, 1450.0);

fn frictionless() -> MomentumParams {
    MomentumParams {
        friction: 0.0,
        ..Default::default()
    }
}

fn world_with_chains(chains: Vec<SurfaceChain>) -> World {
    World::new(
        "surface-test",
        Vec2::new(4000.0, 4000.0),
        Vec2::ZERO,
        Vec::new(),
    )
    .with_chains(chains)
}

/// A V-valley: down-slope, flat bottom, up-slope (authored left→right so
/// normals face up).
fn valley() -> SurfaceChain {
    SurfaceChain::open(
        "valley",
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(400.0, 300.0),
            Vec2::new(800.0, 300.0),
            Vec2::new(1200.0, 0.0),
        ],
    )
}

/// [`valley`] with its left ramp extended 500px further up the SAME line, so a
/// body braking back up it never reaches the chain's open end. Only the C4
/// symmetry rig needs this; the open-end launch has its own tests.
fn long_valley() -> SurfaceChain {
    SurfaceChain::open(
        "valley-long",
        vec![
            Vec2::new(-400.0, -300.0), // collinear with (0,0)->(400,300)
            Vec2::new(0.0, 0.0),
            Vec2::new(400.0, 300.0),
            Vec2::new(800.0, 300.0),
            Vec2::new(1200.0, 0.0),
        ],
    )
}

/// A 16-gon loop, interior-rideable winding (negative shoelace area),
/// centered at `c` with radius `r`, starting at the bottom point.
fn loop_chain(c: Vec2, r: f32) -> SurfaceChain {
    let n = 16;
    let mut pts = Vec::new();
    for k in 0..n {
        // Start at the bottom; wind so the interior stays on the +normal
        // side (negative shoelace = interior-rideable).
        let ang = std::f32::consts::TAU * (k as f32) / (n as f32);
        let (sin, cos) = ang.sin_cos();
        pts.push(c + Vec2::new(r * sin, r * cos));
    }
    let chain = SurfaceChain::closed_loop("loop", pts);
    assert!(chain.validate().is_empty(), "{:?}", chain.validate());
    assert!(chain.signed_area() < 0.0, "interior-rideable winding");
    chain
}

fn route_switch_chain() -> SurfaceChain {
    // Two topological visits to the same screen-space junction. The first
    // occurrence leads up into the loop proxy; the second leads right onto the
    // runout. The left branch slopes down when traversed in reverse, so vertical
    // steering can distinguish "take the loop" from "take the ramp" in both
    // directions.
    SurfaceChain::open(
        "route-switch",
        vec![
            Vec2::new(-100.0, 50.0),
            Vec2::ZERO,
            Vec2::new(100.0, -50.0),
            Vec2::new(-100.0, -50.0),
            Vec2::ZERO,
            Vec2::new(100.0, 0.0),
        ],
    )
    .with_junctions(vec![SurfaceJunction::new(vec![1, 4])])
}

fn shared_tangent_route_switch_chain() -> SurfaceChain {
    // Both choices leave the switch horizontally for 16px, then diverge. A
    // tangent-only router cannot distinguish them; route lookahead can.
    SurfaceChain::open(
        "shared-tangent-route-switch",
        vec![
            Vec2::new(-100.0, 0.0),
            Vec2::ZERO,
            Vec2::new(16.0, 0.0),
            Vec2::new(100.0, -80.0),
            Vec2::ZERO,
            Vec2::new(16.0, 0.0),
            Vec2::new(100.0, 80.0),
        ],
    )
    .with_junctions(vec![SurfaceJunction::new(vec![1, 4])])
}

fn ride(chain_idx: usize, s: f32, v_t: f32, world: &World, radius: f32) -> SurfaceBody {
    let f = world.chains[chain_idx].frame_at(s);
    let mut b = SurfaceBody::new(f.point + f.normal * radius, radius);
    b.motion = SurfaceMotion::Riding {
        on: SurfaceRef::Chain(chain_idx),
        s,
        v_t,
    };
    b
}

#[test]
fn slope_accelerates_downhill_and_energy_never_grows() {
    let world = world_with_chains(vec![valley()]);
    // Start at rest near the top of the left slope.
    let mut body = ride(0, 10.0, 0.0, &world, 14.0);
    let start_height = -body.pos.y; // y-down: height = -y
    let params = frictionless();
    let mut max_speed: f32 = 0.0;
    let mut max_height_after_start = f32::MIN;
    let mut left_start = false;
    for _ in 0..600 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            None,
        );
        max_speed = max_speed.max(body.vel.length());
        if let SurfaceMotion::Riding { s, .. } = body.motion {
            if s > 100.0 {
                left_start = true;
            }
            if left_start {
                max_height_after_start = max_height_after_start.max(-body.pos.y);
            }
        }
    }
    assert!(left_start, "gravity pulled the body down the slope");
    assert!(max_speed > 500.0, "gained real speed downhill: {max_speed}");
    // Energy sanity: it never climbs ABOVE its start height (small
    // integration slack allowed).
    assert!(
        max_height_after_start <= start_height + 2.0,
        "energy grew: start height {start_height}, reached {max_height_after_start}"
    );
}

#[test]
fn uphill_decelerates_and_the_body_oscillates_in_the_valley() {
    let world = world_with_chains(vec![valley()]);
    let mut body = ride(0, 10.0, 0.0, &world, 14.0);
    let params = frictionless();
    // Track the sign of v_t: it must flip at least twice (down, up the
    // far slope, back down) within a few seconds.
    let mut flips = 0;
    let mut last_sign = 0.0f32;
    for _ in 0..900 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            None,
        );
        if let SurfaceMotion::Riding { v_t, .. } = body.motion {
            let sign = if v_t > 10.0 {
                1.0
            } else if v_t < -10.0 {
                -1.0
            } else {
                last_sign
            };
            if last_sign != 0.0 && sign != last_sign {
                flips += 1;
            }
            last_sign = sign;
        }
    }
    assert!(flips >= 2, "valley oscillation (got {flips} turn flips)");
}

#[test]
fn input_cannot_exceed_top_speed_but_slopes_can() {
    let flat = SurfaceChain::open(
        "flat",
        vec![Vec2::new(0.0, 300.0), Vec2::new(20000.0, 300.0)],
    );
    let world = world_with_chains(vec![flat]);
    let mut body = ride(0, 10.0, 0.0, &world, 14.0);
    let params = frictionless();
    for _ in 0..600 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            G,
            SurfaceInputs {
                run: 1.0,
                ..Default::default()
            },
            DT,
            None,
        );
    }
    let SurfaceMotion::Riding { v_t, .. } = body.motion else {
        panic!("still riding the flat");
    };
    assert!(
        (v_t - params.top_speed).abs() < 20.0,
        "input holds top speed: {v_t}"
    );
    // A steep downhill exceeds it.
    let world2 = world_with_chains(vec![valley()]);
    let mut fast = ride(0, 10.0, params.top_speed, &world2, 14.0);
    for _ in 0..25 {
        step_surface_body(
            &mut fast,
            &world2,
            &params,
            G,
            SurfaceInputs {
                run: 1.0,
                ..Default::default()
            },
            DT,
            None,
        );
    }
    if let SurfaceMotion::Riding { v_t, .. } = fast.motion {
        assert!(v_t > params.top_speed + 50.0, "slope exceeds cap: {v_t}");
    }
}

#[test]
fn loop_completes_above_threshold_speed_and_sheds_below() {
    let world = world_with_chains(vec![loop_chain(Vec2::new(500.0, 300.0), 150.0)]);
    let params = frictionless();
    // FAST: launch around the loop from the bottom.
    let mut fast = ride(0, 1.0, 1400.0, &world, 14.0);
    let mut reached_top = false;
    let mut always_riding = true;
    for _ in 0..240 {
        step_surface_body(
            &mut fast,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            None,
        );
        match fast.motion {
            SurfaceMotion::Riding { .. } => {
                // Top of the loop = body above the center.
                if fast.pos.y < 300.0 - 100.0 {
                    reached_top = true;
                }
            }
            SurfaceMotion::Airborne => always_riding = false,
        }
        if reached_top && matches!(fast.motion, SurfaceMotion::Riding { .. }) {
            break;
        }
    }
    assert!(
        reached_top && always_riding,
        "fast body rides through the loop top (reached_top={reached_top}, always_riding={always_riding})"
    );
    // SLOW: cannot complete the loop — it either sheds off the wall or
    // oscillates in the bowl (halfpipe-style); it must never reach the top.
    let mut slow = ride(0, 1.0, 600.0, &world, 14.0);
    let mut slow_reached_top = false;
    for _ in 0..240 {
        step_surface_body(
            &mut slow,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            None,
        );
        if matches!(slow.motion, SurfaceMotion::Riding { .. }) && slow.pos.y < 300.0 - 100.0 {
            slow_reached_top = true;
        }
    }
    assert!(!slow_reached_top, "slow body must not complete the loop");
}

#[test]
fn open_ramp_end_launches_with_the_end_tangent() {
    let ramp = SurfaceChain::open(
        "launch-ramp",
        vec![
            Vec2::new(0.0, 300.0),
            Vec2::new(300.0, 300.0),
            Vec2::new(500.0, 200.0),
        ],
    );
    let world = world_with_chains(vec![ramp]);
    let params = frictionless();
    let mut body = ride(0, 10.0, 800.0, &world, 14.0);
    let mut launched_vel = None;
    for _ in 0..120 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            None,
        );
        if matches!(body.motion, SurfaceMotion::Airborne) {
            launched_vel = Some(body.vel);
            break;
        }
    }
    let v = launched_vel.expect("ran off the ramp end");
    // The end tangent points up-and-right: (200,-100)/|.| — the launch
    // velocity must be along it (within a step of gravity).
    let t = (Vec2::new(500.0, 200.0) - Vec2::new(300.0, 300.0)).normalize();
    let along = v.normalize().dot(t);
    assert!(
        along > 0.98,
        "launched along the ramp tangent: {v:?} vs {t:?}"
    );
}

#[test]
fn convex_crest_launches_at_speed_and_follows_at_a_walk() {
    // A flat run into a gentle downhill: convex joint.
    let crest = SurfaceChain::open(
        "crest",
        vec![
            Vec2::new(0.0, 300.0),
            Vec2::new(400.0, 300.0),
            Vec2::new(800.0, 500.0),
            Vec2::new(1600.0, 900.0),
        ],
    );
    let world = world_with_chains(vec![crest]);
    let params = frictionless();
    // FAST over the crest: leaves the ground.
    let mut fast = ride(0, 10.0, 1200.0, &world, 14.0);
    let mut went_airborne = false;
    for _ in 0..60 {
        step_surface_body(
            &mut fast,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            None,
        );
        if matches!(fast.motion, SurfaceMotion::Airborne) {
            went_airborne = true;
            break;
        }
    }
    assert!(went_airborne, "fast body launches off the crest");
    // Slow walk: follows the surface over the joint.
    let mut slow = ride(0, 380.0, 120.0, &world, 14.0);
    let mut crossed_riding = false;
    for _ in 0..120 {
        step_surface_body(
            &mut slow,
            &world,
            &params,
            G,
            SurfaceInputs {
                run: 1.0,
                ..Default::default()
            },
            DT,
            None,
        );
        if let SurfaceMotion::Riding { s, .. } = slow.motion {
            if s > 450.0 {
                crossed_riding = true;
                break;
            }
        } else {
            break;
        }
    }
    assert!(crossed_riding, "slow body follows the crest joint");
}

#[test]
fn airborne_body_lands_on_a_chain_and_starts_riding() {
    let floor = SurfaceChain::open(
        "floor",
        vec![Vec2::new(0.0, 400.0), Vec2::new(1000.0, 400.0)],
    );
    let world = world_with_chains(vec![floor]);
    let params = MomentumParams::default();
    let mut body = SurfaceBody::new(Vec2::new(500.0, 200.0), 14.0);
    let mut contacts = Vec::new();
    for _ in 0..240 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            Some(&mut contacts),
        );
        if body.riding() {
            break;
        }
    }
    assert!(body.riding(), "fell onto the chain and stuck");
    // Snapped to the surface: center = point + normal*radius.
    assert!(
        (body.pos.y - (400.0 - 14.0)).abs() < 0.5,
        "pos {:?}",
        body.pos
    );
    // The landing contact is a Chain source with an up normal.
    assert!(contacts
        .iter()
        .any(|c| matches!(c.source, ContactSource::Chain { .. })
            && (c.normal - Vec2::new(0.0, -1.0)).length() < 1e-3));
}

#[test]
fn chains_are_one_sided_a_body_passes_from_behind() {
    let floor = SurfaceChain::open(
        "floor",
        vec![Vec2::new(0.0, 400.0), Vec2::new(1000.0, 400.0)],
    );
    let world = world_with_chains(vec![floor]);
    let params = MomentumParams::default();
    // Below the floor moving up: approaches from the solid side.
    let mut body = SurfaceBody::new(Vec2::new(500.0, 500.0), 14.0);
    body.vel = Vec2::new(0.0, -900.0);
    for _ in 0..30 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            None,
        );
    }
    assert!(
        !body.riding() && body.pos.y < 400.0,
        "passed through from the back side: {:?}",
        body.pos
    );
}

#[test]
fn airborne_body_sweeps_into_solid_blocks_no_tunneling() {
    let mut world = world_with_chains(vec![]);
    world.blocks.push(crate::world::Block::solid(
        "wall",
        Vec2::new(600.0, 0.0),
        Vec2::new(50.0, 1000.0),
    ));
    let params = MomentumParams::default();
    let mut body = SurfaceBody::new(Vec2::new(100.0, 500.0), 14.0);
    body.vel = Vec2::new(30000.0, 0.0); // 500px per frame: must not tunnel
    let mut contacts = Vec::new();
    step_surface_body(
        &mut body,
        &world,
        &params,
        G,
        SurfaceInputs::default(),
        DT,
        Some(&mut contacts),
    );
    assert!(
        body.pos.x <= 600.0 - 14.0 + 0.5,
        "stopped at the wall face: {:?}",
        body.pos
    );
    assert!(body.vel.x.abs() < 1.0, "into-wall velocity killed");
    assert!(contacts
        .iter()
        .any(|c| (c.normal - Vec2::new(-1.0, 0.0)).length() < 1e-3));
}

#[test]
fn jump_leaves_along_the_surface_normal_with_tangent_momentum() {
    let floor = SurfaceChain::open(
        "floor",
        vec![Vec2::new(0.0, 400.0), Vec2::new(2000.0, 400.0)],
    );
    let world = world_with_chains(vec![floor]);
    let params = MomentumParams::default();
    let mut body = ride(0, 500.0, 600.0, &world, 14.0);
    step_surface_body(
        &mut body,
        &world,
        &params,
        G,
        SurfaceInputs {
            run: 0.0,
            steer: Vec2::ZERO,
            jump_pressed: true,
        },
        DT,
        None,
    );
    assert!(!body.riding());
    assert!(body.vel.x > 550.0, "kept tangent momentum: {:?}", body.vel);
    assert!(
        body.vel.y < -500.0,
        "launched along +normal (up): {:?}",
        body.vel
    );
}

/// THE C4 rig: the whole scenario rotated 90° (points and gravity) must
/// produce the SAME trajectory rotated 90°. The follower is pure vector
/// math — no cardinal-axis assumptions — so this holds tightly.
#[test]
fn c4_rotation_symmetry_the_rotated_valley_matches() {
    // Exact 90° rotation in f32: (x, y) -> (y, -x) (plus a translation to
    // keep coordinates positive; translations are exact for these values).
    let rot = |p: Vec2| Vec2::new(p.y, -p.x) + Vec2::new(0.0, 2000.0);
    let rot_v = |p: Vec2| Vec2::new(p.y, -p.x);

    // The valley with 500px of extra runway PREPENDED along the same line as
    // its left ramp — geometrically identical where the body travels, and
    // `ride(.., 510.0, ..)` starts it at the exact world point `s = 10` named
    // on the bare `valley()`. The runway exists because the braking phase
    // below used to park the body on the chain's OPEN LEFT END, where whether
    // it sheds this frame or next is decided by f32 rounding. A symmetry test
    // must not straddle a discrete knife edge: the two rotated runs would
    // disagree by a whole frame of state, which says nothing about symmetry.
    // (Found when the airborne air-control sign was corrected below; the old
    // mirrored sign happened to shove the body back onto the ramp.)
    let chain_a = long_valley();
    let chain_b = SurfaceChain::open(
        "valley-rot",
        chain_a.points.iter().map(|&p| rot(p)).collect(),
    );
    let world_a = world_with_chains(vec![chain_a]);
    let world_b = world_with_chains(vec![chain_b]);
    let params = frictionless();
    let g_a = G;
    let g_b = rot_v(G);

    let mut a = ride(0, 510.0, 0.0, &world_a, 14.0);
    let mut b = ride(0, 510.0, 0.0, &world_b, 14.0);
    for frame in 0..600 {
        let input = SurfaceInputs {
            // Scripted input: run right for 2s, coast, then brake.
            run: if frame < 120 {
                1.0
            } else if frame < 300 {
                0.0
            } else {
                -1.0
            },
            steer: Vec2::ZERO,
            jump_pressed: frame == 360,
        };
        step_surface_body(&mut a, &world_a, &params, g_a, input, DT, None);
        step_surface_body(&mut b, &world_b, &params, g_b, input, DT, None);
        let mapped = rot(a.pos);
        // Sub-pixel agreement: the translation in `rot` shifts f32
        // rounding between the two runs, so exact bit-equality is not
        // available — but half a pixel over 10 seconds of riding, joint
        // crossings, launches, and ballistic fall IS the symmetry.
        assert!(
            (mapped - b.pos).length() < 0.5,
            "frame {frame}: rotated trajectory diverged: {:?} vs {:?} (orig {:?})",
            mapped,
            b.pos,
            a.pos
        );
        assert_eq!(a.riding(), b.riding(), "frame {frame}: state diverged");
    }
}

// ---- blocks are surfaces (the Sanic-in-a-normal-room fix) ----

fn world_with_blocks(blocks: Vec<crate::world::Block>) -> World {
    World::new("block-test", Vec2::new(4000.0, 4000.0), Vec2::ZERO, blocks)
}

fn floor_block(min: Vec2, size: Vec2) -> crate::world::Block {
    crate::world::Block::solid("floor", min, size)
}

#[test]
fn body_lands_runs_and_jumps_on_a_block_floor() {
    // A plain solid floor — no authored chains anywhere. The momentum
    // body must land (ride), accelerate under input, and jump: the exact
    // capabilities that were chain-only before blocks became surfaces.
    let world = world_with_blocks(vec![floor_block(
        Vec2::new(0.0, 500.0),
        Vec2::new(2000.0, 100.0),
    )]);
    let params = frictionless();
    let mut body = SurfaceBody::new(Vec2::new(200.0, 400.0), 14.0);

    // Fall on: within a second it must be riding the block's top face.
    for _ in 0..60 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            None,
        );
    }
    assert!(body.riding(), "body never grounded on the block floor");
    assert!(
        matches!(
            body.motion,
            SurfaceMotion::Riding {
                on: SurfaceRef::Block(0),
                ..
            }
        ),
        "riding the block, not a phantom chain: {:?}",
        body.motion
    );
    assert!(
        (body.pos.y - (500.0 - 14.0)).abs() < 1.0,
        "resting on the top face"
    );

    // Run right: real horizontal progress (the old slide-only block path
    // dropped the frame remainder at toi≈0 — near-zero advance).
    let x0 = body.pos.x;
    for _ in 0..60 {
        let input = SurfaceInputs {
            run: 1.0,
            steer: Vec2::ZERO,
            jump_pressed: false,
        };
        step_surface_body(&mut body, &world, &params, G, input, DT, None);
    }
    assert!(body.riding(), "still grounded while running");
    assert!(
        body.pos.x - x0 > 200.0,
        "ran along the floor: {} -> {}",
        x0,
        body.pos.x
    );

    // Jump: leaves the surface along the normal, moving up.
    let input = SurfaceInputs {
        run: 0.0,
        steer: Vec2::ZERO,
        jump_pressed: true,
    };
    step_surface_body(&mut body, &world, &params, G, input, DT, None);
    assert!(!body.riding(), "jump detaches");
    assert!(
        body.vel.y < -200.0,
        "jump launches against gravity: {:?}",
        body.vel
    );
}

#[test]
fn flush_block_seams_do_not_stop_a_runner() {
    // Two flush blocks forming one continuous floor. Crossing the seam
    // costs at most a micro-launch + same-frame reattach; speed carries.
    let world = world_with_blocks(vec![
        floor_block(Vec2::new(0.0, 500.0), Vec2::new(400.0, 100.0)),
        floor_block(Vec2::new(400.0, 500.0), Vec2::new(2000.0, 100.0)),
    ]);
    let params = frictionless();
    let mut body = SurfaceBody::new(Vec2::new(100.0, 470.0), 14.0);
    for _ in 0..30 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            None,
        );
    }
    assert!(body.riding(), "grounded on the first block");
    for _ in 0..150 {
        let input = SurfaceInputs {
            run: 1.0,
            steer: Vec2::ZERO,
            jump_pressed: false,
        };
        step_surface_body(&mut body, &world, &params, G, input, DT, None);
    }
    assert!(body.pos.x > 500.0, "crossed the seam: {:?}", body.pos);
    assert!(
        matches!(
            body.motion,
            SurfaceMotion::Riding {
                on: SurfaceRef::Block(1),
                ..
            }
        ),
        "riding the second block: {:?}",
        body.motion
    );
    assert!(
        (body.pos.y - (500.0 - 14.0)).abs() < 2.0,
        "still on the floor plane"
    );
}

#[test]
fn walking_off_a_block_edge_launches_and_never_wraps() {
    // A block corner is a convex joint whose wall face carries no
    // pressing load — walking off the edge must LAUNCH (fall), never
    // wrap around onto the wall.
    let world = world_with_blocks(vec![floor_block(
        Vec2::new(0.0, 500.0),
        Vec2::new(400.0, 100.0),
    )]);
    let params = frictionless();
    let mut body = SurfaceBody::new(Vec2::new(300.0, 470.0), 14.0);
    for _ in 0..30 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            None,
        );
    }
    assert!(body.riding());
    let mut went_airborne = false;
    for _ in 0..120 {
        let input = SurfaceInputs {
            run: 1.0,
            steer: Vec2::ZERO,
            jump_pressed: false,
        };
        step_surface_body(&mut body, &world, &params, G, input, DT, None);
        if !body.riding() {
            went_airborne = true;
        }
        if body.riding() {
            // Any riding position must stay on the TOP face — never the
            // right wall (x would pin to 400+radius) or the underside.
            assert!(
                body.pos.y <= 500.0 - 14.0 + 1.0,
                "wrapped off the top face: {:?}",
                body.pos
            );
        }
    }
    assert!(went_airborne, "never launched off the edge");
    assert!(body.pos.x > 400.0, "carried past the edge: {:?}", body.pos);
}

#[test]
fn ceiling_bonk_deflects_and_never_sticks() {
    // Jumping up into a block's underside: landing is load-bearing
    // (press <= 0 on the bottom face), so the body deflects and falls —
    // it never glues to a corridor roof.
    let world = world_with_blocks(vec![floor_block(
        Vec2::new(0.0, 0.0),
        Vec2::new(2000.0, 100.0),
    )]);
    let params = frictionless();
    let mut body = SurfaceBody::new(Vec2::new(500.0, 300.0), 14.0);
    body.vel = Vec2::new(300.0, -900.0); // fast up + sideways
    for _ in 0..120 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            None,
        );
        assert!(!body.riding(), "stuck to the ceiling: {:?}", body.motion);
    }
    assert!(body.vel.y > 0.0, "falling again after the bonk");
}

#[test]
fn rotated_gravity_lands_on_the_gravity_side_face_of_a_block() {
    // C4 discipline: with gravity pointing +x, a block's LEFT face is
    // "the floor" (press > 0) and the body lands and rides it.
    let world = world_with_blocks(vec![floor_block(
        Vec2::new(1000.0, 0.0),
        Vec2::new(200.0, 2000.0),
    )]);
    let params = frictionless();
    let g = Vec2::new(1450.0, 0.0);
    let mut body = SurfaceBody::new(Vec2::new(800.0, 900.0), 14.0);
    for _ in 0..90 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            g,
            SurfaceInputs::default(),
            DT,
            None,
        );
    }
    assert!(body.riding(), "grounded on the gravity-side face");
    assert!(
        (body.pos.x - (1000.0 - 14.0)).abs() < 1.0,
        "resting on the left face: {:?}",
        body.pos
    );
    // "Run" along the wall-floor: tangent motion works in the rotated frame.
    let y0 = body.pos.y;
    for _ in 0..60 {
        let input = SurfaceInputs {
            run: 1.0,
            steer: Vec2::ZERO,
            jump_pressed: false,
        };
        step_surface_body(&mut body, &world, &params, g, input, DT, None);
    }
    assert!(body.riding());
    assert!(
        (body.pos.y - y0).abs() > 100.0,
        "ran along the face: {} -> {}",
        y0,
        body.pos.y
    );
}

/// **Air control must push the way the stick points.** `tangent_of` is the ONE
/// handedness definition in the engine: the along-surface axis of a FLOOR is
/// `tangent_of(floor_normal)`, and a floor's normal is `-gravity`. The airborne
/// branch built its side axis from `tangent_of(gravity)` instead — the exact
/// negation — so holding right in mid-air accelerated a momentum body LEFT.
///
/// Nothing caught it because no test held a direction in the air: every
/// airborne test here is ballistic or a landing. Sanic's ball dash is what
/// finally read the airborne side axis (demos/sanic.md), and it read it wrong
/// on purpose, to match the kernel — which is how the kernel got audited.
#[test]
fn airborne_air_control_pushes_toward_the_held_direction() {
    let world = world_with_chains(vec![]);
    let params = MomentumParams::default();
    let gravity = Vec2::new(0.0, 900.0); // +y is down

    for (run, expect_sign) in [(1.0_f32, 1.0_f32), (-1.0, -1.0)] {
        let mut body = SurfaceBody::new(Vec2::new(0.0, 0.0), 14.0);
        for _ in 0..30 {
            step_surface_body(
                &mut body,
                &world,
                &params,
                gravity,
                SurfaceInputs {
                    run,
                    steer: Vec2::ZERO,
                    jump_pressed: false,
                },
                1.0 / 60.0,
                None,
            );
        }
        assert!(
            body.vel.x * expect_sign > 0.0,
            "held run={run}, drifted vel.x={} — air control is mirrored",
            body.vel.x
        );
    }
}

/// The same statement, frame-agnostically: under rotated gravity the side axis
/// is still `tangent_of(-gravity)`, so `run` means "toward the body's own
/// local right", never "toward screen +x".
#[test]
fn airborne_air_control_is_gravity_relative() {
    let world = world_with_chains(vec![]);
    let params = MomentumParams::default();
    // Gravity points LEFT: local "down" is -x, so local "right" is -y (up-screen).
    let gravity = Vec2::new(-900.0, 0.0);
    let expected_side = crate::frame::tangent_of(-gravity.normalize());

    let mut body = SurfaceBody::new(Vec2::ZERO, 14.0);
    for _ in 0..30 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            gravity,
            SurfaceInputs {
                run: 1.0,
                steer: Vec2::ZERO,
                jump_pressed: false,
            },
            1.0 / 60.0,
            None,
        );
    }
    assert!(
        body.vel.dot(expected_side) > 0.0,
        "vel={:?} should have a positive component along the local side axis {expected_side:?}",
        body.vel
    );
}

/// **A body that runs off the end of a flat chain must FALL, not hover.**
///
/// The launch places it exactly at the end vertex, one radius above the
/// surface, moving horizontally. `project` clamps arc length to the chain, so
/// the next airborne sweep re-attached it at that same vertex, from which the
/// ride step launched it again — a two-frame limit cycle with the position
/// frozen at the lip. Nothing caught it because the ONE flat-chain-end
/// scenario in the suite ran a MIRRORED air-control sign (fixed above) that
/// shoved the body back over the chain instead of off it. Two bugs holding
/// each other up.
#[test]
fn running_off_a_flat_chains_end_falls_instead_of_hovering_at_the_lip() {
    let floor = SurfaceChain::open(
        "floor",
        vec![Vec2::new(0.0, 600.0), Vec2::new(1500.0, 600.0)],
    );
    let world = world_with_chains(vec![floor]);
    let params = MomentumParams::default();
    // Riding at the top speed the params allow, one tick from the end.
    let mut body = ride(0, 1480.0, params.top_speed, &world, 15.0);

    let mut left_the_chain_at = None;
    for frame in 0..240 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            Vec2::new(0.0, 1450.0),
            SurfaceInputs {
                run: 1.0,
                steer: Vec2::ZERO,
                jump_pressed: false,
            },
            DT,
            None,
        );
        if !body.riding() && left_the_chain_at.is_none() {
            left_the_chain_at = Some(frame);
        }
    }
    let launched = left_the_chain_at.expect("must leave the chain");
    assert!(launched < 10, "left the chain at frame {launched}");
    assert!(!body.riding(), "and it never re-attaches to the lip");
    assert!(
        body.pos.x > 1500.0,
        "it carried its momentum past the end: x={}",
        body.pos.x
    );
    assert!(
        body.pos.y > 600.0,
        "and gravity took it BELOW the floor plane rather than pinning it at \
         the lip: y={}",
        body.pos.y
    );
}

/// The guard is one-directional: a body landing on a chain's last vertex while
/// moving back ONTO the chain still attaches. Otherwise every ramp tip would
/// become un-standable.
#[test]
fn landing_on_the_tip_of_a_ramp_while_moving_inward_still_attaches() {
    let floor = SurfaceChain::open(
        "floor",
        vec![Vec2::new(0.0, 600.0), Vec2::new(1500.0, 600.0)],
    );
    let world = world_with_chains(vec![floor]);
    let params = MomentumParams::default();
    // Falling onto the very end of the chain, drifting LEFT (back onto it).
    let mut body = SurfaceBody::new(Vec2::new(1499.0, 560.0), 15.0);
    body.vel = Vec2::new(-50.0, 200.0);
    for _ in 0..30 {
        step_surface_body(
            &mut body,
            &world,
            &params,
            Vec2::new(0.0, 1450.0),
            SurfaceInputs {
                run: 0.0,
                steer: Vec2::ZERO,
                jump_pressed: false,
            },
            DT,
            None,
        );
    }
    assert!(body.riding(), "a landing at the tip is a landing");
}

/// Arc length does not name a unique frame at a polyline joint. A zero-speed
/// rider must settle onto the adjacent branch that actually supports it, rather
/// than inherit `frame_at`'s arbitrary pre-join tie and lose every grounded
/// affordance.
#[test]
fn zero_speed_at_a_joint_chooses_support_and_keeps_jump_and_walk_available() {
    // Authored upward wall into a rightward floor. At the joint `frame_at`
    // historically returns the wall segment (`s <= len`), whose horizontal
    // normal carries no gravity load. The floor immediately after the same
    // vertex is valid support.
    let chain = SurfaceChain::open(
        "wall-to-floor",
        vec![
            Vec2::new(200.0, 400.0),
            Vec2::new(200.0, 250.0),
            Vec2::new(600.0, 250.0),
        ],
    );
    let joint_s = chain.segment_length(0);
    let floor_frame = chain.frame_at(joint_s + joint_nudge(joint_s));
    assert_eq!(floor_frame.segment, 1);
    let world = world_with_chains(vec![chain]);
    let radius = 14.0;
    let resting = SurfaceBody {
        pos: floor_frame.point + floor_frame.normal * radius,
        vel: Vec2::ZERO,
        radius,
        depth_lane: 0,
        motion: SurfaceMotion::Riding {
            on: SurfaceRef::Chain(0),
            s: joint_s,
            v_t: 0.0,
        },
    };
    let params = MomentumParams::default();

    let mut idle = resting;
    let mut contacts = Vec::new();
    step_surface_body(
        &mut idle,
        &world,
        &params,
        G,
        SurfaceInputs::default(),
        DT,
        Some(&mut contacts),
    );
    assert!(
        idle.riding(),
        "resting on the supported side must stay grounded"
    );
    assert!(contacts
        .iter()
        .any(|contact| matches!(contact.source, ContactSource::Chain { segment: 1, .. })));

    let mut walking = resting;
    step_surface_body(
        &mut walking,
        &world,
        &params,
        G,
        SurfaceInputs {
            run: 1.0,
            steer: Vec2::ZERO,
            jump_pressed: false,
        },
        DT,
        None,
    );
    let SurfaceMotion::Riding { s, v_t, .. } = walking.motion else {
        panic!("walking onto the floor branch must stay attached");
    };
    assert!(
        s > joint_s && v_t > 0.0,
        "run intent advances off the joint"
    );

    let mut jumping = resting;
    step_surface_body(
        &mut jumping,
        &world,
        &params,
        G,
        SurfaceInputs {
            run: 0.0,
            steer: Vec2::ZERO,
            jump_pressed: true,
        },
        DT,
        None,
    );
    assert!(!jumping.riding(), "jump must leave the supported joint");
    assert!(jumping.vel.y < 0.0, "jump launches away from the floor");

    let mut leaving = resting;
    step_surface_body(
        &mut leaving,
        &world,
        &params,
        G,
        SurfaceInputs {
            run: -1.0,
            steer: Vec2::ZERO,
            jump_pressed: false,
        },
        DT,
        None,
    );
    assert!(
        !leaving.riding(),
        "intent toward the unsupported wall branch must shed instead of pinning the body"
    );
    assert!(
        leaving.vel.x < 0.0 && leaving.vel.y.abs() < 1.0,
        "walking off keeps the supporting floor tangent instead of launching down the adjacent wall: {:?}",
        leaving.vel
    );
}

/// **A body must be able to cross a joint anywhere on a long chain.**
///
/// `advance_riding` nudges past a joint so `frame_at` resolves the segment it
/// entered. The nudge was a fixed `1e-4` — under one f32 ULP once the arc
/// length passes ~800px. On a long chain the nudge rounded back to the joint,
/// `to_join` stayed 0, and the bounded walk spun out without advancing: the
/// body froze ON the joint, still `Riding`, still carrying its velocity. That
/// last detail is why it read as a physics puzzle instead of a rounding bug.
///
/// The valley tests never caught it because their joints sit at s ≈ 500, where
/// `1e-4` is comfortably many ULPs.
#[test]
fn a_body_crosses_a_joint_far_along_a_long_chain_in_both_directions() {
    // A flat run, then a gentle bend, then more flat — the joint at s ≈ 1500.
    let chain = SurfaceChain::open(
        "long",
        vec![
            Vec2::new(0.0, 400.0),
            Vec2::new(1500.0, 400.0),
            Vec2::new(2500.0, 380.0),
        ],
    );
    let world = world_with_chains(vec![chain]);
    let params = frictionless();

    for (start_s, v_t, name) in [(1400.0, 600.0, "forward"), (1600.0, -600.0, "backward")] {
        let mut body = ride(0, start_s, v_t, &world, 14.0);
        let x0 = body.pos.x;
        for _ in 0..60 {
            step_surface_body(
                &mut body,
                &world,
                &params,
                G,
                SurfaceInputs {
                    run: v_t.signum(),
                    steer: Vec2::ZERO,
                    jump_pressed: false,
                },
                DT,
                None,
            );
        }
        let travelled = (body.pos.x - x0) * v_t.signum();
        assert!(
            travelled > 400.0,
            "{name}: the body moved {travelled:.1}px in one second at 600px/s — \
             it stalled on the joint at s≈1500"
        );
        assert!(body.riding(), "{name}: it should still be on the chain");
    }
}

/// The nudge is representable wherever it is applied. A nudge smaller than one
/// ULP is a nudge that never happened.
#[test]
fn the_joint_nudge_always_moves_the_arc_length() {
    for s in [0.0_f32, 1.0, 100.0, 857.0, 10_000.0, 1_000_000.0] {
        let n = joint_nudge(s);
        assert!(s + n > s, "nudge {n} vanished at s={s}");
        assert!(
            s - n < s || s == 0.0,
            "nudge {n} vanished downward at s={s}"
        );
    }
}

/// Joint stabilization is a zero-speed ambiguity resolver, not an alternate
/// crossing path. A moving rider must remain exactly on the joint so
/// `advance_riding` can apply the convex corner rule; nudging it onto the wall
/// is what regressed both flush-block handoff and ordinary ledge departure.
#[test]
fn moving_riders_are_not_preselected_onto_a_joint_branch() {
    let block = floor_block(Vec2::new(0.0, 500.0), Vec2::new(400.0, 100.0));
    let chain = block.boundary_chain();
    let top_right_s = chain.segment_length(0);

    for speed in [-600.0, 600.0] {
        assert_eq!(
            stabilize_joint_rest(&chain, top_right_s, speed, speed.signum(), G),
            top_right_s,
            "moving riders must reach the joint classifier unchanged"
        );
    }
}

#[test]
fn route_bias_is_acceleration_frame_vertical_not_surface_transverse() {
    assert!(
        route_bias_direction(G, Vec2::X).is_none(),
        "Right is locomotion even while the ridden tangent is diagonal"
    );
    assert!(
        route_bias_direction(G, -Vec2::X).is_none(),
        "Left is locomotion even while the ridden tangent is diagonal"
    );

    let up = route_bias_direction(G, Vec2::new(1.0, -1.0)).expect("Up supplies a route override");
    assert!(up.dot(Vec2::new(0.0, -1.0)) > 0.999);

    // Under sideways gravity, screen/world Y is the locomotion-transverse axis,
    // but it is not body Up/Down. Route bias rotates with the acceleration frame.
    let sideways_down = Vec2::new(1450.0, 0.0);
    assert!(route_bias_direction(sideways_down, Vec2::Y).is_none());
    let body_up = route_bias_direction(sideways_down, Vec2::new(-1.0, 1.0))
        .expect("body-local Up rotates with gravity");
    assert!(body_up.dot(-Vec2::X) > 0.999);
}

#[test]
fn route_junction_uses_vertical_steering_and_preserves_default_continuation() {
    let chain = route_switch_chain();
    assert!(chain.validate().is_empty(), "{:?}", chain.validate());
    let world = world_with_chains(vec![chain.clone()]);
    let on = SurfaceRef::Chain(0);

    // Forward arrival from the left ramp: up-right enters the loop occurrence,
    // down-right skips to the runout occurrence.
    let loop_branch = choose_route_branch(&world, on, 1, 0, 1.0, G, Vec2::new(1.0, -1.0))
        .expect("entry is an authored junction");
    assert_eq!(loop_branch.vertex, 1);
    assert_eq!(loop_branch.direction, 1.0);
    assert!(loop_branch.is_default);

    let bypass = choose_route_branch(&world, on, 1, 0, 1.0, G, Vec2::new(1.0, 1.0))
        .expect("entry is an authored junction");
    assert_eq!(bypass.vertex, 4);
    assert_eq!(bypass.direction, 1.0);
    assert!(!bypass.is_default);

    // Reverse arrival from the right runout: up-left takes the loop; down-left
    // chooses the descending ramp. Neutral steering keeps the authored loop
    // continuation on first arrival, then the authored ramp continuation after
    // one reverse revolution rather than spinning again.
    let reverse_loop = choose_route_branch(&world, on, 4, 4, -1.0, G, Vec2::new(-1.0, -1.0))
        .expect("closure is an authored junction");
    assert_eq!(reverse_loop.vertex, 4);
    assert_eq!(reverse_loop.direction, -1.0);
    assert!(reverse_loop.is_default);

    let reverse_ramp = choose_route_branch(&world, on, 4, 4, -1.0, G, Vec2::new(-1.0, 1.0))
        .expect("closure is an authored junction");
    assert_eq!(reverse_ramp.vertex, 1);
    assert_eq!(reverse_ramp.direction, -1.0);
    assert!(!reverse_ramp.is_default);

    let after_reverse_lap = choose_route_branch(&world, on, 1, 1, -1.0, G, Vec2::new(-1.0, 0.0))
        .expect("entry is an authored junction");
    assert_eq!(after_reverse_lap.vertex, 1);
    assert_eq!(after_reverse_lap.direction, -1.0);
    assert!(
        after_reverse_lap.is_default,
        "world-horizontal Left on a diagonal loop tangent is locomotion, not a downward route request"
    );

    let after_forward_lap = choose_route_branch(&world, on, 4, 3, 1.0, G, Vec2::new(1.0, 0.0))
        .expect("closure is an authored junction");
    assert_eq!(after_forward_lap.vertex, 4);
    assert_eq!(after_forward_lap.direction, 1.0);
    assert!(
        after_forward_lap.is_default,
        "holding only along the travel direction must preserve the forward exit"
    );

    let entry_s = chain.arc_at_vertex(1);
    let closure_s = chain.arc_at_vertex(4);
    let (stopped_on, stopped_bypass) =
        choose_route_branch_at_rest(&world, on, entry_s, 0.0, Vec2::new(1.0, 1.0))
            .expect("held direction selects a route from rest");
    assert_eq!(stopped_on, on);
    assert!(
        stopped_bypass > closure_s,
        "down-right selects the runout even with zero momentum"
    );
}

#[test]
fn route_junction_looks_past_a_shared_tangent_before_honoring_steering() {
    let chain = shared_tangent_route_switch_chain();
    let world = world_with_chains(vec![chain]);
    let on = SurfaceRef::Chain(0);

    let up = choose_route_branch(&world, on, 1, 0, 1.0, G, Vec2::new(1.0, -1.0))
        .expect("entry is an authored junction");
    assert_eq!(up.vertex, 1, "up-right selects the rising route");

    let down = choose_route_branch(&world, on, 1, 0, 1.0, G, Vec2::new(1.0, 1.0))
        .expect("entry is an authored junction");
    assert_eq!(down.vertex, 4, "down-right selects the descending route");

    let horizontal = choose_route_branch(&world, on, 1, 0, 1.0, G, Vec2::X)
        .expect("entry is an authored junction");
    assert_eq!(horizontal.vertex, 1);
    assert!(
        horizontal.is_default,
        "horizontal input cannot distinguish routes with a shared tangent, so it preserves authored continuation"
    );
}

#[test]
fn cross_chain_junction_selects_a_ramp_without_an_airborne_hop() {
    let floor = SurfaceChain::open(
        "floor-route",
        vec![Vec2::new(-100.0, 0.0), Vec2::ZERO, Vec2::new(120.0, 0.0)],
    );
    let ramp = SurfaceChain::open(
        "raised-route",
        vec![Vec2::ZERO, Vec2::new(16.0, 0.0), Vec2::new(100.0, -80.0)],
    )
    .with_junctions(vec![SurfaceJunction::across(vec![
        SurfacePort::local(0),
        SurfacePort::chain(0, 1),
    ])]);
    let world = world_with_chains(vec![floor, ramp]);
    let floor_on = SurfaceRef::Chain(0);
    let branch_s = world.chains[0].arc_at_vertex(1);
    let params = frictionless();

    let raised = advance_riding(
        &world,
        floor_on,
        branch_s - 1.0,
        4.0,
        240.0,
        G,
        &params,
        14.0,
        Vec2::new(1.0, -1.0),
    );
    let RideOutcome::Riding { on, s, v_t } = raised else {
        panic!("a route junction transfers while riding; it is not a jump");
    };
    assert_eq!(on, SurfaceRef::Chain(1));
    assert!(s > 0.0, "the rider advanced onto the raised chain");
    assert!(v_t > 0.0, "the transfer preserves forward momentum");

    let flat = advance_riding(
        &world,
        floor_on,
        branch_s - 1.0,
        4.0,
        240.0,
        G,
        &params,
        14.0,
        Vec2::X,
    );
    let RideOutcome::Riding { on, s, .. } = flat else {
        panic!("the flat continuation remains rideable");
    };
    assert_eq!(on, floor_on);
    assert!(
        s > branch_s,
        "holding horizontally continues along the floor"
    );
}

#[test]
fn route_junction_changes_arc_occurrence_without_reversing_speed() {
    let chain = route_switch_chain();
    let entry_s = chain.arc_at_vertex(1);
    let closure_s = chain.arc_at_vertex(4);
    let world = world_with_chains(vec![chain]);
    let on = SurfaceRef::Chain(0);
    let params = frictionless();

    let bypass = advance_riding(
        &world,
        on,
        entry_s - 1.0,
        4.0,
        240.0,
        G,
        &params,
        14.0,
        Vec2::new(1.0, 1.0),
    );
    let RideOutcome::Riding {
        on: result_on,
        s,
        v_t,
    } = bypass
    else {
        panic!("an authored switch is a guided route, not a launch");
    };
    assert_eq!(result_on, on);
    assert!(s > closure_s, "down-right selected the runout occurrence");
    assert!(v_t > 0.0, "route transfer preserves forward speed");

    let ramp = advance_riding(
        &world,
        on,
        closure_s + 1.0,
        -4.0,
        -240.0,
        G,
        &params,
        14.0,
        Vec2::new(-1.0, 1.0),
    );
    let RideOutcome::Riding {
        on: result_on,
        s,
        v_t,
    } = ramp
    else {
        panic!("an authored switch is a guided route, not a launch");
    };
    assert_eq!(result_on, on);
    assert!(
        s < entry_s,
        "down-left selected the descending ramp occurrence"
    );
    assert!(v_t < 0.0, "route transfer preserves reverse speed");
}

#[test]
fn depth_lanes_are_discrete_collision_planes() {
    assert!(depth_lanes_collide(-1, -1));
    assert!(depth_lanes_collide(0, 0));
    assert!(depth_lanes_collide(1, 1));
    assert!(!depth_lanes_collide(-1, 0));
    assert!(!depth_lanes_collide(0, 1));
    assert!(!depth_lanes_collide(-1, 1));
    assert!(!depth_lanes_collide(1, -1));
}

#[test]
fn airborne_sweep_ignores_tracks_on_other_depth_lanes() {
    let rail = |name: &str, lane: i8| {
        SurfaceChain::open(
            name,
            vec![Vec2::new(-100.0, 100.0), Vec2::new(100.0, 100.0)],
        )
        .with_segment_depths(vec![lane])
    };
    let center = Vec2::new(0.0, 60.0);
    let delta = Vec2::new(0.0, 80.0);

    let other_lanes = world_with_chains(vec![rail("back", -1), rail("center", 0)]);
    assert!(
        first_circle_hit(&other_lanes, center, 12.0, 1, delta).is_none(),
        "a foreground rider must pass the coincident back/center rails"
    );

    let matching = world_with_chains(vec![rail("back", -1), rail("center", 0), rail("front", 1)]);
    let hit = first_circle_hit(&matching, center, 12.0, 1, delta)
        .expect("the matching foreground rail remains collidable");
    assert!(
        matches!(
            hit.what,
            CircleHitTarget::Chain {
                chain: 2,
                segment: 0
            }
        ),
        "the sweep must select only the matching depth lane"
    );
}

#[test]
fn toi_zero_tangent_scrapes_are_not_recaptured_as_landings() {
    let normal = Vec2::new(0.0, -1.0);
    let center = Vec2::new(0.0, -15.8);
    let tangent_scrape = Vec2::new(10.0, 0.5);
    assert!(grazing_chain_contact_at_release(
        center,
        16.0,
        Vec2::ZERO,
        normal,
        tangent_scrape,
        0.0,
    ));

    assert!(
        !grazing_chain_contact_at_release(
            Vec2::new(0.0, -16.0),
            16.0,
            Vec2::ZERO,
            normal,
            tangent_scrape,
            0.0,
        ),
        "an exactly tangent contact is not an overlap artifact"
    );

    assert!(
        !grazing_chain_contact_at_release(
            center,
            16.0,
            Vec2::ZERO,
            normal,
            Vec2::new(1.0, 10.0),
            0.0,
        ),
        "a meaningful into-surface impact is a real landing"
    );
    assert!(
        !grazing_chain_contact_at_release(center, 16.0, Vec2::ZERO, normal, tangent_scrape, 0.25,),
        "a later impact is not the numerical release contact"
    );
}

#[test]
fn segment_scoped_projection_preserves_the_route_occurrence_at_a_crossover() {
    let chain = route_switch_chain();
    let point = Vec2::ZERO;
    let entry_s = project_to_segment(&chain, 0, point);
    let runout_s = project_to_segment(&chain, 4, point);
    assert_eq!(chain.frame_at(entry_s).segment, 0);
    assert_eq!(chain.frame_at(runout_s).segment, 4);
    assert!(
        runout_s - entry_s > 100.0,
        "the same screen-space point retains distinct topological arc positions"
    );
}
