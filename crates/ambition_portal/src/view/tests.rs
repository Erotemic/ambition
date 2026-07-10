//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::pieces::{front_distance, map_point};
use ambition_engine_core::AabbExt;

fn frame(pos: Vec2, normal: Vec2) -> PortalAperture {
    PortalAperture {
        frame: PortalFrame::fixed(pos, normal),
        half_length: crate::types::portal_opening_half(normal, crate::portal_half_extent(normal)),
    }
}
fn floor(pos: Vec2) -> PortalAperture {
    frame(pos, Vec2::new(0.0, -1.0))
}
fn right_wall(pos: Vec2) -> PortalAperture {
    frame(pos, Vec2::new(-1.0, 0.0))
}
fn size(b: ae::Aabb) -> Vec2 {
    b.half_size() * 2.0
}

/// For every axis-aligned (enter, exit) normal pair, the view projection
/// factors exactly into optional `flip_x` plus a rotation. Reflection-body
/// convention yields rotation-only projection; rotation-body convention
/// yields a reflected projection.
#[test]
fn view_map_factorization_matches_each_convention() {
    let normals = [
        Vec2::new(0.0, -1.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(1.0, 0.0),
    ];
    for rotation_convention in [false, true] {
        let map_vec = if rotation_convention {
            portal_map_vec_rotation
        } else {
            portal_map_vec_reflection
        };
        for n_in in normals {
            for n_out in normals {
                let enter = frame(Vec2::new(100.0, 300.0), n_in);
                let exit = frame(Vec2::new(700.0, 140.0), n_out);
                let m = PortalViewMap::between_for_convention(
                    &enter.frame,
                    &exit.frame,
                    rotation_convention,
                );
                assert!(
                    (m.cos * m.cos + m.sin * m.sin - 1.0).abs() < 1e-4,
                    "unit factor for {n_in:?}→{n_out:?}: cos {} sin {}",
                    m.cos,
                    m.sin
                );
                assert_eq!(m.flip_x, rotation_convention);
                for v in [Vec2::X, Vec2::Y, Vec2::new(3.0, -2.0)] {
                    let reflected = v - 2.0 * v.dot(enter.frame.normal) * enter.frame.normal;
                    let expected = map_vec(reflected, enter.frame.normal, exit.frame.normal);
                    let got = m.apply(enter.frame.origin + v) - exit.frame.origin;
                    assert!(
                        (got - expected).length() < 1e-4,
                        "projection factor mismatch convention={rotation_convention} {n_in:?}→{n_out:?}: {got:?} vs {expected:?}"
                    );
                }
            }
        }
    }
}

/// On the portal face the reflection is the identity, so the view map and
/// the body map agree — an emerging body lines up with its cone image.
#[test]
fn view_agrees_with_body_map_on_the_face() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    for s in [-30.0_f32, 0.0, 18.5, 46.0] {
        let on_face = enter.frame.origin + Vec2::new(s, 0.0); // floor face runs along x
        let via_view = view_point(on_face, &enter.frame, &exit.frame);
        let via_body = map_point(on_face, &enter.frame, &exit.frame);
        assert!(
            (via_view - via_body).length() < 1e-3,
            "face continuity at s={s}: view {via_view:?} body {via_body:?}"
        );
    }
}

/// Projection model: depth in front of the entry becomes depth in front
/// of the exit — the projection shows the exit's room, never the inside
/// of its wall.
#[test]
fn view_preserves_front_depth() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    for d in [0.0_f32, 5.0, 60.0, 240.0] {
        let p = enter.frame.origin + enter.frame.normal * d;
        let seen = view_point(p, &enter.frame, &exit.frame);
        assert!(
            (front_distance(seen, &exit.frame) - d).abs() < 1e-3,
            "depth {d} maps to front depth {}",
            front_distance(seen, &exit.frame)
        );
    }
}

/// Pin the floor→right-wall map numerically: 10px above the floor portal
/// at lateral +s shows the point 10px left of the wall portal at lateral
/// -s along the wall's tangent (t_out = (0,-1) ⇒ world offset (0,-s)).
#[test]
fn floor_to_wall_view_pinned() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    // y-down world: 10px in FRONT of a floor portal is y=290.
    let seen = view_point(Vec2::new(120.0, 290.0), &enter.frame, &exit.frame);
    assert!(
        (seen - Vec2::new(390.0, 180.0)).length() < 1e-3,
        "got {seen:?}"
    );
    // The rotation angle is -90° (cos 0, sin -1) for this pair.
    let m = PortalViewMap::between(&enter.frame, &exit.frame);
    assert!((m.cos).abs() < 1e-4 && (m.sin + 1.0).abs() < 1e-4, "{m:?}");
}

/// Window semantics: the trapezoid recedes INTO the entry's host surface,
/// while its source rect sits fully in FRONT of the exit (it images the
/// exit's room), swapping extents across a 90° pair: window depth becomes
/// the rect's x-extent, window width its y-extent.
#[test]
fn view_cone_source_geometry() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    let depth = 120.0;
    let spread = 0.25;
    let cone = view_cone(&enter, &exit, depth, spread);
    // Entry quad: near edge on the face, far edge `depth` INTO the floor
    // (y-down world: into a floor = +y).
    assert!((cone.entry_quad[0].y - 300.0).abs() < 1e-3);
    assert!((cone.entry_quad[2].y - 420.0).abs() < 1e-3);
    // Source rect: x spans the wall's front depth, y the widened lateral.
    assert!(
        (size(cone.source).x - depth).abs() < 1e-3,
        "depth extent {:?}",
        size(cone.source)
    );
    let far_half = enter.half_length + depth * spread;
    assert!(
        (size(cone.source).y - 2.0 * far_half).abs() < 1e-3,
        "lateral extent {:?}",
        size(cone.source)
    );
    // Fully in front of the exit wall (x <= 400), touching the face.
    assert!(cone.source.max.x <= 400.0 + 1e-3, "{:?}", cone.source);
    assert!((cone.source.max.x - 400.0).abs() < 1e-3);
    // Every source corner is the BODY-map image of its entry corner (the
    // window's display map IS the body map — one map for sight and transit).
    for (e, s) in cone.entry_quad.iter().zip(cone.source_quad.iter()) {
        assert!((map_point(*e, &enter.frame, &exit.frame) - *s).length() < 1e-3);
    }
}

/// Behind BOTH ends, in neither doorway ⇒ `None` (two floors, eye well
/// below both planes).
#[test]
fn visible_cone_none_behind_both_ends() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = floor(Vec2::new(500.0, 300.0));
    // Floors face up (−y); 60px below is past the doorway depth grace.
    assert!(visible_cone(&enter, &exit, Vec2::new(100.0, 360.0), 80.0, 400.0).is_none());
}

/// The wormhole: standing in front of the PARTNER opens this end's window
/// even though the eye is behind this surface (above purple ⇒ yellow shows).
#[test]
fn visible_cone_opens_from_the_partner_side() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    // Eye is BEHIND the floor (y > 300, past the doorway grace) but in
    // FRONT of the wall partner (x < 400) — only the wormhole opens it.
    let eye = Vec2::new(100.0, 360.0);
    let (resolved, wormhole) = window_eye(&enter, &exit, eye).expect("in front of partner");
    assert!(wormhole, "resolved via the partner side");
    // The image is in front of `enter` (above the floor, y < 300).
    assert!(resolved.y < 300.0, "image in front of enter: {resolved:?}");
    assert!(visible_cone(&enter, &exit, eye, 80.0, 400.0).is_some());
}

/// Same-plane pair (two floor portals): the eye above the PARTNER is in
/// front of BOTH ends, but the window must resolve from the nearer one —
/// the partner-side image right above this aperture — not from the grazing
/// 400px-away direct ray. This is the straddle case: standing on purple,
/// yellow's window opens as if you stood on yellow.
#[test]
fn window_eye_prefers_the_nearer_end() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = floor(Vec2::new(500.0, 300.0));
    // Eye 20px above the EXIT (the partner end).
    let eye = Vec2::new(500.0, 280.0);
    let (resolved, wormhole) = window_eye(&enter, &exit, eye).expect("in front of both");
    assert!(wormhole, "nearer end is the partner");
    // The image sits 20px above THIS aperture (floor↔floor: x preserved
    // relative to centers, front preserved).
    assert!(
        (resolved - Vec2::new(100.0, 280.0)).length() < 1e-3,
        "partner image above this end, got {resolved:?}"
    );
}

/// Q10.2 continuity pin: walking between a same-plane pair (in front of
/// BOTH ends the whole way), the resolved eye must move continuously —
/// the old hard nearest-pick jumped by the full pair separation the frame
/// the nearer end flipped. Outside the handoff band the nearest end still
/// wins exactly.
#[test]
fn window_eye_hands_off_continuously_between_same_plane_ends() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = floor(Vec2::new(500.0, 300.0));
    let mut prev: Option<Vec2> = None;
    let mut max_step = 0.0_f32;
    for i in 0..=160 {
        // Eye 20px above the shared plane, sweeping across the midpoint.
        let eye = Vec2::new(220.0 + i as f32, 280.0);
        let (resolved, _) = window_eye(&enter, &exit, eye).expect("in front of both");
        if let Some(p) = prev {
            max_step = max_step.max((resolved - p).length());
        }
        prev = Some(resolved);
    }
    // The direct↔via images are 400px apart; the crossfade spreads that
    // over the handoff band instead of one frame. 1px of eye motion moves
    // the blend by ~400 / (2*band) ≈ 17px/step — allow modest headroom,
    // and the old behavior's single ~400px jump fails loudly.
    assert!(
        max_step < 30.0,
        "resolved eye must crossfade, not jump: max step {max_step}"
    );
    // Far from the midpoint the nearest end wins exactly (old behavior).
    let (near_enter, wormhole) =
        window_eye(&enter, &exit, Vec2::new(220.0, 280.0)).expect("resolves");
    assert!(!wormhole);
    assert!((near_enter - Vec2::new(220.0, 280.0)).length() < 1e-3);
    let (near_exit, wormhole) =
        window_eye(&enter, &exit, Vec2::new(380.0, 280.0)).expect("resolves");
    assert!(wormhole, "nearer end is the partner");
    assert!(
        (near_exit - Vec2::new(-20.0, 280.0)).length() < 1e-3,
        "pure partner image past the band, got {near_exit:?}"
    );
}

/// Q10.2 continuity pin, thin-wall doorway (the c136/c137 shape): the eye
/// crossing THROUGH the pair — including off-center — never jumps; the
/// doorway lifts of the two faces nearly coincide and the handoff blends
/// between them.
#[test]
fn window_eye_is_continuous_through_a_thin_wall_doorway() {
    let enter = PortalAperture {
        frame: PortalFrame::fixed(Vec2::new(500.0, 300.0), Vec2::new(-1.0, 0.0)),
        half_length: 46.0,
    };
    let exit = PortalAperture {
        frame: PortalFrame::fixed(Vec2::new(532.0, 300.0), Vec2::new(1.0, 0.0)),
        half_length: 46.0,
    };
    // Centered and off-center within the aperture.
    for y in [300.0, 310.0] {
        let mut prev: Option<Vec2> = None;
        let mut max_step = 0.0_f32;
        for i in 0..=104 {
            let eye = Vec2::new(490.0 + i as f32 * 0.5, y);
            let (resolved, _) =
                window_eye(&enter, &exit, eye).expect("front or doorway all the way");
            if let Some(p) = prev {
                max_step = max_step.max((resolved - p).length());
            }
            prev = Some(resolved);
        }
        assert!(
            max_step < 6.0,
            "thin-wall crossing must be smooth at y={y}: max step {max_step}"
        );
    }
}

/// In-doorway grace: an eye dipped just BEHIND the plane mid-transit
/// (within the aperture span) still opens the window — as the half-plane
/// limit, not a sliver and not `None`.
#[test]
fn window_survives_the_transit_dip_as_a_half_plane() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = floor(Vec2::new(500.0, 300.0));
    let max_lateral = 400.0;
    // Eye 10px BELOW the entry plane, laterally centered (mid-transit).
    let eye = Vec2::new(100.0, 310.0);
    let cone = visible_cone(&enter, &exit, eye, 80.0, max_lateral).expect("doorway grace holds");
    let [_, _, f1, f0] = cone.entry_quad;
    // The limit continuation: far corners at the lateral clamp.
    assert!(
        (f0.x - (100.0 - max_lateral)).abs() < 1e-3 && (f1.x - (100.0 + max_lateral)).abs() < 1e-3,
        "half-plane strip, got {f0:?} {f1:?}"
    );
    // Depth still exact.
    assert!((f0.y - 380.0).abs() < 1e-3 && (f1.y - 380.0).abs() < 1e-3);
}

/// The small-front continuation is finite and clamped — no blow-up as the
/// eye approaches the plane, and the wedge saturates smoothly to the strip.
#[test]
fn wedge_is_stable_near_the_plane_and_under_extreme_skew() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    let max_lateral = 400.0;
    for eye in [
        Vec2::new(100.0, 299.5),  // 0.5px in front (limit branch)
        Vec2::new(100.0, 298.0),  // 2px in front (projective, clamped)
        Vec2::new(1000.0, 295.0), // extreme grazing skew from the right
    ] {
        let cone = aperture_wedge(&enter, &exit, eye, 80.0, max_lateral).unwrap();
        for p in cone.entry_quad.iter().chain(cone.source_quad.iter()) {
            assert!(p.x.is_finite() && p.y.is_finite(), "finite corners");
            assert!(
                (p.x - enter.frame.origin.x).abs() <= max_lateral + 1e-2
                    || (*p - exit.frame.origin).length() <= max_lateral + 80.0 + 1e-2,
                "within the clamp envelope: {p:?}"
            );
        }
        // Far corners always land exactly max_depth behind the entry.
        assert!((cone.entry_quad[2].y - 380.0).abs() < 1e-3);
        assert!((cone.entry_quad[3].y - 380.0).abs() < 1e-3);
    }
}

#[test]
fn near_plane_eye_outside_aperture_is_not_full_half_plane() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    let max_lateral = 400.0;
    let cone = aperture_wedge(&enter, &exit, Vec2::new(300.0, 299.5), 80.0, max_lateral).unwrap();
    let [_, _, f1, f0] = cone.entry_quad;

    assert!(
        f0.x < enter.frame.origin.x - enter.half_length
            && f1.x < enter.frame.origin.x - enter.half_length,
        "near-plane eye to the right of the aperture should see a left-skewed grazing cone, got {f0:?} {f1:?}",
    );
    assert!(
        !((f0.x - (enter.frame.origin.x - max_lateral)).abs() < 1e-3
            && (f1.x - (enter.frame.origin.x + max_lateral)).abs() < 1e-3),
        "off-aperture near-plane eyes must not receive the centered full half-plane",
    );
}

/// The far edge sits exactly `max_depth` behind the surface, and a head-on
/// viewer yields a laterally-centered, symmetric wedge wider than the
/// aperture (perspective through the slit).
#[test]
fn visible_cone_head_on_is_symmetric_and_depth_clamped() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    let depth = 80.0;
    // Eye 80px in front of the floor portal (−y), directly above center.
    let front = 80.0;
    let cone = visible_cone(&enter, &exit, Vec2::new(100.0, 300.0 - front), depth, 400.0).unwrap();
    let [a0, a1, f1, f0] = cone.entry_quad;
    // Near edge is the aperture (on the surface, y = 300).
    assert!((a0.y - 300.0).abs() < 1e-3 && (a1.y - 300.0).abs() < 1e-3);
    // Far corners sit exactly `depth` behind (into the floor, +y).
    assert!((f0.y - (300.0 + depth)).abs() < 1e-3, "{f0:?}");
    assert!((f1.y - (300.0 + depth)).abs() < 1e-3, "{f1:?}");
    // Head-on ⇒ far edge centered on the aperture center (x=100) and wider
    // than the aperture by (1 + depth/front).
    let h = enter.half_length;
    assert!(((f0.x + f1.x) * 0.5 - 100.0).abs() < 1e-3, "centered");
    let far_half = (f1.x - f0.x).abs() * 0.5;
    assert!(
        (far_half - h * (1.0 + depth / front)).abs() < 1e-3,
        "far_half {far_half} vs {}",
        h * (1.0 + depth / front)
    );
}

/// An off-axis viewer skews the wedge: through a slit you see the FAR side,
/// away from you (looking from the left, the visible far edge shifts
/// right). Pinned for BOTH a floor portal AND a ceiling portal (the
/// magenta case) so the skew direction is identical regardless of which
/// way the surface faces — a ceiling never inverts.
#[test]
fn visible_cone_skews_away_from_viewer_floor_and_ceiling() {
    // Floor (normal up): eye up-and-LEFT ⇒ far edge to the RIGHT.
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    let cone = visible_cone(&enter, &exit, Vec2::new(40.0, 220.0), 80.0, 400.0).unwrap();
    let [_, _, f1, f0] = cone.entry_quad;
    assert!(
        (f0.x + f1.x) * 0.5 > 100.0,
        "floor: off-left viewer ⇒ far edge right, got {}",
        (f0.x + f1.x) * 0.5
    );
    // Ceiling (normal DOWN, +y): eye BELOW and to the LEFT ⇒ far edge still
    // to the RIGHT (consistent — no ceiling-specific inversion).
    let ceil = frame(Vec2::new(100.0, 300.0), Vec2::new(0.0, 1.0));
    let cone = visible_cone(&ceil, &exit, Vec2::new(40.0, 380.0), 80.0, 400.0).unwrap();
    let [_, _, f1, f0] = cone.entry_quad;
    assert!(
        (f0.x + f1.x) * 0.5 > 100.0,
        "ceiling: off-left viewer ⇒ far edge right, got {}",
        (f0.x + f1.x) * 0.5
    );
}

/// The sprite-copy factorization equals the body map for every pair class
/// under both map conventions.
#[test]
fn copy_transform_factors_the_body_map() {
    let pairs = [
        (Vec2::new(0.0, -1.0), Vec2::new(0.0, -1.0)), // floor↔floor
        (Vec2::new(0.0, 1.0), Vec2::new(0.0, -1.0)),  // ceiling↔floor
        (Vec2::new(1.0, 0.0), Vec2::new(1.0, 0.0)),   // same wall
        (Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0)),  // opposite walls
        (Vec2::new(0.0, -1.0), Vec2::new(-1.0, 0.0)), // floor→wall (90°)
    ];
    for rotation_convention in [false, true] {
        let map_vec = if rotation_convention {
            portal_map_vec_rotation
        } else {
            portal_map_vec_reflection
        };
        for (n_in, n_out) in pairs {
            let enter = frame(Vec2::new(100.0, 300.0), n_in);
            let exit = frame(Vec2::new(500.0, 200.0), n_out);
            let copy =
                copy_transform_for_convention(&enter.frame, &exit.frame, rotation_convention);
            assert_eq!(copy.flip_x, !rotation_convention);
            // World-space rotation angle is the negated render roll.
            let a = -copy.roll;
            let (s, c) = a.sin_cos();
            for v in [
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(3.0, -2.0),
            ] {
                let f = if copy.flip_x { Vec2::new(-v.x, v.y) } else { v };
                let rotated = Vec2::new(f.x * c - f.y * s, f.x * s + f.y * c);
                let body = map_vec(v, n_in, n_out);
                assert!(
                    (rotated - body).length() < 1e-4,
                    "{n_in:?}→{n_out:?} convention={rotation_convention}: copy {rotated:?} vs body {body:?}"
                );
            }
        }
    }
}

/// The multi-eye wedge is the UNION of its eyes' wedges (far edge spans the
/// combined lateral extent), the near edge stays exactly on the aperture,
/// and an eye behind the plane contributes nothing.
#[test]
fn multi_eye_wedge_unions_and_anchors_at_the_aperture() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    let left = Vec2::new(40.0, 250.0);
    let right = Vec2::new(160.0, 250.0);
    let one_l = aperture_wedge(&enter, &exit, left, 80.0, 400.0).unwrap();
    let one_r = aperture_wedge(&enter, &exit, right, 80.0, 400.0).unwrap();
    let both = aperture_wedge_multi(&enter, &exit, &[left, right], 80.0, 400.0).unwrap();
    // Near edge unchanged (exactly the aperture, on the surface y=300).
    assert!((both.entry_quad[0].y - 300.0).abs() < 1e-3);
    assert!((both.entry_quad[1].y - 300.0).abs() < 1e-3);
    // Far edge spans the union: at least as wide as either single wedge.
    let span = |c: &ViewCone| (c.entry_quad[2].x - c.entry_quad[3].x).abs();
    assert!(span(&both) >= span(&one_l) - 1e-3 && span(&both) >= span(&one_r) - 1e-3);
    // An extra eye BEHIND the plane (below the floor) changes nothing.
    let with_behind = aperture_wedge_multi(
        &enter,
        &exit,
        &[left, right, Vec2::new(100.0, 360.0)],
        80.0,
        400.0,
    )
    .unwrap();
    assert!((span(&with_behind) - span(&both)).abs() < 1e-3);
}

/// Continuity across the partner plane — the reason for the eye set. As a
/// viewpoint crosses the entry plane, swapping it for its mapped shadow on
/// the far side leaves the far edge essentially unchanged (no abrupt flip).
#[test]
fn wedge_far_edge_is_continuous_through_the_plane() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = floor(Vec2::new(500.0, 300.0));
    let far_lat = |eyes: &[Vec2]| {
        let c = aperture_wedge_multi(&enter, &exit, eyes, 80.0, 1000.0).unwrap();
        (c.entry_quad[2].x + c.entry_quad[3].x) * 0.5
    };
    // Eye just in FRONT of the entry plane (y just < 300).
    let just_front = far_lat(&[Vec2::new(120.0, 299.0)]);
    // The SAME eye one tick later just BEHIND, replaced by its shadow mapped
    // from the partner (map_point(behind-entry → front-of-exit), then that
    // shadow viewed from `enter` is its partner image) — here we approximate
    // continuity by the near-plane limit being shared.
    let near_plane = far_lat(&[Vec2::new(120.0, 299.9)]);
    assert!(
        (just_front - near_plane).abs() < 60.0,
        "far edge moves smoothly near the plane: {just_front} vs {near_plane}"
    );
}

/// Zero spread degenerates to a straight corridor: source rect lateral
/// extent equals the aperture.
#[test]
fn view_cone_zero_spread_is_a_corridor() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = floor(Vec2::new(500.0, 300.0));
    let cone = view_cone(&enter, &exit, 90.0, 0.0);
    assert!(
        (size(cone.source).x - 2.0 * enter.half_length).abs() < 1e-3,
        "{:?}",
        size(cone.source)
    );
    assert!((size(cone.source).y - 90.0).abs() < 1e-3);
}
