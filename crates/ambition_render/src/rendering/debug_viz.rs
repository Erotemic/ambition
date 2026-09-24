//! Engine-generic debug visualizations: the F1 gizmo layers any game can use.
//!
//! The movement engine exposes simulation state; this module draws the
//! generic part: world collision blocks (colour-keyed by `BlockKind`),
//! momentum surface chains with normal and tangent quills, rebound-pad
//! impulses, moving platforms, room bounds, grids, and a body/feature layer.
//! That layer reads the sim-view read models (`BodyPoseView`,
//! `FeatureViewIndex`), never live sim components, so it works in any host
//! that renders.
//!
//! Two consumers:
//!
//! - [`DebugVizPlugin`]: the whole package for a game with no debug stack of
//!   its own (the demo apps). An F1 toggle on the shared
//!   [`DeveloperRuntimeState::debug`] seam, plus one draw system. The viz
//!   starts off.
//! - The sandbox's overlay (`ambition_app::dev::debug_overlay`) imports the
//!   layer functions and adds game-specific layers (authored combat volumes,
//!   boss clusters, LDtk spine, portals). It does not add the plugin; it owns
//!   its hotkeys.
//!
//! This module draws shapes only; it is not a dev HUD.

use ambition_dev_tools::dev_tools::DeveloperTools;
use ambition_dev_tools::DeveloperRuntimeState;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::world_to_bevy;
use ambition_platformer2d_core::{AabbExt, RoomGeometry};
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_platformer2d_shared_tangle::lifecycle::{session_world_exists, SessionWorldRef};
use ambition_platformer2d_world::collision::MovingPlatformSet;
use ambition_platformer2d_world::platforms::MovingPlatformState;
use ambition_sim_view::{BodyPoseView, CombatGeometryView, FeatureViewIndex};
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

// ───────────────────────────── palette ─────────────────────────────

pub fn cyan() -> Color {
    Color::srgba(0.30, 0.92, 1.00, 0.92)
}
pub fn blue() -> Color {
    Color::srgba(0.30, 0.55, 1.00, 0.90)
}
pub fn green() -> Color {
    Color::srgba(0.25, 1.00, 0.45, 0.90)
}
pub fn yellow() -> Color {
    Color::srgba(1.00, 0.92, 0.22, 0.95)
}
pub fn orange() -> Color {
    Color::srgba(1.00, 0.55, 0.16, 0.90)
}
pub fn magenta() -> Color {
    Color::srgba(1.00, 0.32, 0.92, 0.88)
}
pub fn red() -> Color {
    Color::srgba(1.00, 0.18, 0.22, 0.82)
}
pub fn white_dim() -> Color {
    Color::srgba(0.90, 0.95, 1.00, 0.40)
}
pub fn gray() -> Color {
    Color::srgba(0.62, 0.66, 0.75, 0.46)
}

// ──────────────────────────── primitives ────────────────────────────

pub fn with_alpha(color: Color, alpha: f32) -> Color {
    let srgba = color.to_srgba();
    Color::srgba(srgba.red, srgba.green, srgba.blue, alpha.clamp(0.0, 1.0))
}

/// World-space point → Bevy 2D gizmo space (the y-flip in one place).
pub fn w2(world: &ae::World, p: ae::Vec2) -> BVec2 {
    world_to_bevy(world, p, 0.0).truncate()
}

/// World-space delta (direction/offset) → Bevy 2D (y-flip, no origin shift).
pub fn engine_delta_to_bevy(delta: ae::Vec2) -> BVec2 {
    BVec2::new(delta.x, -delta.y)
}

pub fn draw_aabb(gizmos: &mut Gizmos, world: &ae::World, aabb: ae::Aabb, color: Color) {
    let min = aabb.min;
    let max = aabb.max;
    let tl = w2(world, ae::Vec2::new(min.x, min.y));
    let tr = w2(world, ae::Vec2::new(max.x, min.y));
    let br = w2(world, ae::Vec2::new(max.x, max.y));
    let bl = w2(world, ae::Vec2::new(min.x, max.y));
    gizmos.line_2d(tl, tr, color);
    gizmos.line_2d(tr, br, color);
    gizmos.line_2d(br, bl, color);
    gizmos.line_2d(bl, tl, color);
}

/// Outline + optional translucent fill. Fills are controlled directly by the
/// current debug view mode instead of being coupled to sprite hiding; choose
/// Collision/Combat/Triggers when the filled volume view is useful.
pub fn draw_aabb_styled(
    gizmos: &mut Gizmos,
    world: &ae::World,
    aabb: ae::Aabb,
    color: Color,
    developer_tools: &DeveloperTools,
) {
    draw_aabb(gizmos, world, aabb, color);
    if !developer_tools.fill_debug_boxes {
        return;
    }
    let size = aabb.half_size() * 2.0;
    let center = w2(world, aabb.center());
    let fill = with_alpha(color, 0.22);
    // `rect_2d` draws only an outline. For a fill, draw horizontal lines
    // 2px apart. This works on every gizmo backend without a mesh path, and
    // the cost is small (AABBs are small and the toggle is usually off).
    let step = 2.0;
    let half_h = (size.y * 0.5).max(0.5);
    let mut y = -half_h;
    while y < half_h {
        let a = BVec2::new(center.x - size.x * 0.5, center.y + y);
        let b = BVec2::new(center.x + size.x * 0.5, center.y + y);
        gizmos.line_2d(a, b, fill);
        y += step;
    }
}

/// Draw a [`ae::CombatVolume`] outline — a box, rotated box, disc, or convex
/// polygon. Shows the actual hitbox shape (for example a blade-arc polygon),
/// not its bounding box.
pub fn draw_combat_volume(
    gizmos: &mut Gizmos,
    world: &ae::World,
    vol: &ae::CombatVolume,
    color: Color,
) {
    let outline = |gizmos: &mut Gizmos, pts: &[ae::Vec2]| {
        let n = pts.len();
        for i in 0..n {
            gizmos.line_2d(w2(world, pts[i]), w2(world, pts[(i + 1) % n]), color);
        }
    };
    match vol {
        ae::CombatVolume::Aabb(a) => draw_aabb(gizmos, world, *a, color),
        ae::CombatVolume::Obb {
            center,
            half,
            rotation,
        } => {
            let (s, c) = rotation.sin_cos();
            let rot = |x: f32, y: f32| *center + ae::Vec2::new(x * c - y * s, x * s + y * c);
            outline(
                gizmos,
                &[
                    rot(-half.x, -half.y),
                    rot(half.x, -half.y),
                    rot(half.x, half.y),
                    rot(-half.x, half.y),
                ],
            );
        }
        ae::CombatVolume::Circle { center, radius } => {
            const N: usize = 24;
            let pts: Vec<ae::Vec2> = (0..N)
                .map(|i| {
                    let a = i as f32 / N as f32 * std::f32::consts::TAU;
                    *center + ae::Vec2::new(a.cos() * radius, a.sin() * radius)
                })
                .collect();
            outline(gizmos, &pts);
        }
        ae::CombatVolume::Convex { points, .. } => {
            if points.len() >= 2 {
                outline(gizmos, points);
            }
        }
    }
}

/// Draw a live hitbox's true damage volume: the shape damage resolution tests.
/// When the hitbox has a hull (a convex blade, an OBB, a circle), the hull is
/// drawn prominently and the bounding box is a faint broad-phase outline. A
/// bare `Aabb` volume is its own hull, so it gets the normal styled fill.
pub fn draw_hitbox_volume(
    gizmos: &mut Gizmos,
    world: &ae::World,
    vol: &ae::CombatVolume,
    color: Color,
    developer_tools: &DeveloperTools,
) {
    match vol {
        ae::CombatVolume::Aabb(a) => draw_aabb_styled(gizmos, world, *a, color, developer_tools),
        shaped => {
            draw_combat_volume(gizmos, world, shaped, color);
            draw_aabb(gizmos, world, shaped.bounds(), with_alpha(color, 0.16));
        }
    }
}

/// Where a body-anchored strike is drawn, as opposed to where it was resolved.
///
/// The translation is the owner's [`PresentedPose::delta`] (`presented -
/// authoritative`), the same number every other row of that body uses this
/// frame. Not `presented - owner_anchor`: that would hide a real mismatch
/// when a volume was resolved against a position the body has since left.
/// Presentation moves geometry; it does not repair it.
///
/// [`PresentedPose::delta`]: ambition_sim_view::presented_pose::PresentedPose::delta
///
/// The shape is not recomputed: `CombatVolume::translated` preserves it, and
/// presentation must not read the authoritative `Hitbox`.
pub fn presented_strike_volume(
    strike: &ambition_sim_view::CombatStrikeGeometryView,
    owner_delta: ae::Vec2,
) -> ae::CombatVolume {
    if !strike.anchored_to_body {
        return strike.volume.clone();
    }
    strike.volume.translated(owner_delta)
}

/// This frame's presentation translation for every body the overlay draws,
/// keyed by body. A caller builds it once and passes it to the shared draw.
///
/// One delta per body: everything attached to one body (collision envelope,
/// hurtboxes, strikes) must move by the same amount in the same frame, or the
/// diagnostic misreports attachment. `PresentedPose` follows
/// `BodyKinematics`, so bosses and actors are included.
///
/// A body with no entry has no presented history yet (its first frame), so
/// `ZERO` is correct.
pub fn presentation_deltas(
    combat: &CombatGeometryView,
    bodies: &bevy::prelude::Query<&ambition_sim_view::presented_pose::PresentedPose>,
) -> std::collections::HashMap<bevy::prelude::Entity, ae::Vec2> {
    combat
        .bodies
        .iter()
        .map(|body| body.body)
        .chain(combat.strikes.iter().map(|strike| strike.owner))
        .filter_map(|body| bodies.get(body).ok().map(|pose| (body, pose.delta())))
        .collect()
}

/// Draw authoritative body-combat geometry from the simulation-view boundary.
///
/// Orange is the body's coarse collision envelope, cyan is the effective
/// Orange is the body's coarse collision envelope, cyan is the damageable
/// silhouette, and red is a live strike. Rows carry no controller or
/// primary-player distinction: a fighter is debugged by its geometry.
///
/// Every row of one body takes the same translation from `deltas` (see
/// [`presentation_deltas`]). The envelope, hurtboxes, and body-anchored
/// strikes are one rigid group; moving a subset would misreport attachment.
/// World-anchored strikes take no translation.
///
/// An empty map draws the authoritative geometry, which is correct for a host
/// that publishes no presented poses.
pub fn draw_combat_geometry_view(
    gizmos: &mut Gizmos,
    world: &ae::World,
    combat: &CombatGeometryView,
    developer_tools: &DeveloperTools,
    deltas: &std::collections::HashMap<bevy::prelude::Entity, ae::Vec2>,
) {
    let delta_for =
        |body: bevy::prelude::Entity| deltas.get(&body).copied().unwrap_or(ae::Vec2::ZERO);
    if developer_tools.show_player_hitbox || developer_tools.show_feature_hitboxes {
        let collision_color = with_alpha(orange(), 0.52);
        for body in &combat.bodies {
            let delta = delta_for(body.body);
            draw_aabb(
                gizmos,
                world,
                body.collision.translated(delta),
                collision_color,
            );
            for hurtbox in &body.hurtboxes {
                draw_hitbox_volume(
                    gizmos,
                    world,
                    &hurtbox.translated(delta),
                    cyan(),
                    developer_tools,
                );
            }
        }
    }
    if developer_tools.show_combat_preview || developer_tools.show_feature_hitboxes {
        for strike in &combat.strikes {
            let volume = presented_strike_volume(strike, delta_for(strike.owner));
            draw_hitbox_volume(gizmos, world, &volume, red(), developer_tools);
        }
        for body in &combat.bodies {
            draw_combat_tuning_readout(gizmos, world, body, delta_for(body.body));
        }
    }
}

/// The tuning readout: what a designer reads instead of a log.
///
/// Drawn per body with gizmos only, so it needs no font. Four facts:
///
/// * A phase bar above the body: the move's duration as a track, filled to
///   the clock, coloured by the authored window (Startup yellow, Active red,
///   Recovery blue). Shows whether a connect happened in active or in
///   recovery.
/// * A launch arrow during hitstun: the velocity the body was thrown with,
///   which is what knockback tuning is about.
/// * Two facing ticks: the body's live facing above, the move's committed
///   attack orientation below. When they differ, it is visible.
/// * Lock bars under the body: hitstun, hitlag, and landing lag as three
///   lengths. On screen all three look like "the fighter is not moving".
///
/// No controller, faction, or primary-player check: it draws every combat
/// body the read model publishes.
fn draw_combat_tuning_readout(
    gizmos: &mut Gizmos,
    world: &ae::World,
    body: &ambition_sim_view::CombatBodyGeometryView,
    // The readout hangs off the body's box, so it takes the same translation
    // as the box.
    delta: ae::Vec2,
) {
    /// Width of the phase / lock tracks, in world px.
    const TRACK_W: f32 = 44.0;
    /// Gap above the body's box for the phase track.
    const TRACK_GAP: f32 = 10.0;

    let center = body.collision.center() + delta;
    let half = body.collision.half_size();
    let left = center.x - TRACK_W * 0.5;

    // ── the move timeline ────────────────────────────────────────────────
    if let Some(state) = &body.move_state {
        let y = center.y - half.y - TRACK_GAP;
        let track_l = w2(world, ae::Vec2::new(left, y));
        let track_r = w2(world, ae::Vec2::new(left + TRACK_W, y));
        gizmos.line_2d(track_l, track_r, with_alpha(white_dim(), 0.55));

        let progress = if state.duration_s > 0.0 {
            (state.elapsed_s / state.duration_s).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let phase_color = match state.phase {
            Some(ambition_entity_catalog::WindowTag::Startup) => yellow(),
            Some(ambition_entity_catalog::WindowTag::Active) => red(),
            Some(ambition_entity_catalog::WindowTag::Recovery) => blue(),
            // Invuln, Armor, and Cancel windows read as "some authored window"
            // until a designer needs them told apart.
            Some(_) => cyan(),
            None => white_dim(),
        };
        let filled = w2(world, ae::Vec2::new(left + TRACK_W * progress, y));
        gizmos.line_2d(track_l, filled, phase_color);
        // A connect already banked — why a cancel became available.
        if state.landed_hit {
            let tick_top = w2(world, ae::Vec2::new(left + TRACK_W * progress, y - 3.0));
            gizmos.line_2d(filled, tick_top, cyan());
        }

        // The move's committed orientation, below the body's live one.
        let facing_y = center.y + half.y + 6.0;
        gizmos.line_2d(
            w2(world, ae::Vec2::new(center.x, facing_y)),
            w2(
                world,
                ae::Vec2::new(center.x + state.attack_facing * 12.0, facing_y),
            ),
            red(),
        );
    }

    // The body's live facing.
    let live_y = center.y - half.y - 4.0;
    gizmos.line_2d(
        w2(world, ae::Vec2::new(center.x, live_y)),
        w2(world, ae::Vec2::new(center.x + body.facing * 12.0, live_y)),
        with_alpha(white_dim(), 0.8),
    );

    // ── the launch that put this body in hitstun ─────────────────────────
    if body.hitstun_s > 0.0 && body.velocity.length_squared() > 1.0 {
        draw_arrow(
            gizmos,
            w2(world, center),
            w2(world, center + body.velocity * 0.12),
            orange(),
        );
    }

    // ── three locks that look the same on screen ─────────────────────────
    let locks = [
        (body.hitstun_s, cyan()),
        (body.hitlag_s, red()),
        (body.landing_lag_s, yellow()),
        (body.jump_squat_s, green()),
    ];
    for (row, (seconds, color)) in locks.iter().enumerate() {
        if *seconds <= 0.0 {
            continue;
        }
        // A half-second lock fills the track; longer values are clamped so a
        // very long lock stays on screen.
        let width = (seconds / 0.5).clamp(0.0, 1.0) * TRACK_W;
        let y = center.y + half.y + 10.0 + row as f32 * 3.0;
        gizmos.line_2d(
            w2(world, ae::Vec2::new(left, y)),
            w2(world, ae::Vec2::new(left + width, y)),
            *color,
        );
    }
}

pub fn draw_arrow(gizmos: &mut Gizmos, start: BVec2, end: BVec2, color: Color) {
    gizmos.line_2d(start, end, color);
    let delta = end - start;
    let len = delta.length();
    if len <= 1.0 {
        return;
    }
    let dir = delta / len;
    let side = BVec2::new(-dir.y, dir.x);
    let head = 9.0_f32.min(len * 0.28);
    gizmos.line_2d(end, end - dir * head + side * head * 0.55, color);
    gizmos.line_2d(end, end - dir * head - side * head * 0.55, color);
}

// ─────────────────────────── world layers ───────────────────────────

pub fn draw_room_bounds(gizmos: &mut Gizmos, world: &ae::World) {
    let room = ae::aabb_from_min_size(ae::Vec2::ZERO, world.size);
    draw_aabb(gizmos, world, room, white_dim());
}

/// Where the world ends, drawn beside where it is bounded. Shares
/// `show_room_bounds` with the room bounds.
///
/// `gravity_dir` matters: the gate measures every margin along the body's own
/// `down`, so a line at `y = size.y + margin` is correct only under
/// down-gravity (wrong in the Noether Chamber). The lines are rotated through
/// the live frame, like the gate.
///
/// The fall line is always drawn: every room has one. Side and ceiling lines
/// appear only when the stage opts in, so a missing line means that direction
/// does not kill.
pub fn draw_world_edges(gizmos: &mut Gizmos, world: &ae::World, gravity_dir: ae::Vec2) {
    // Red: crossing this is death. The opt-in pair is dimmer, so the
    // always-live direction reads as the default.
    let fall = Color::srgba(1.0, 0.25, 0.25, 0.55);
    let opt_in = Color::srgba(1.0, 0.45, 0.30, 0.42);
    for line in world_edge_lines(world, gravity_dir) {
        let color = if line.always_lethal { fall } else { opt_in };
        gizmos.line_2d(w2(world, line.from), w2(world, line.to), color);
    }
}

/// One drawn world-edge boundary, in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldEdgeLine {
    pub from: ae::Vec2,
    pub to: ae::Vec2,
    /// Whether this direction ends a body unconditionally (the fall margin) or
    /// only because the stage opted in (the side and rise margins).
    pub always_lethal: bool,
}

/// The out-of-bounds boundaries of `world` as world-space segments, in the
/// body's frame.
///
/// Pure and separate from drawing, so the geometry can be tested: margins are
/// measured along the body's own `down`, which a fixed `+y` line gets wrong
/// under other gravity. A segment can be asserted; a gizmo call cannot.
pub fn world_edge_lines(world: &ae::World, gravity_dir: ae::Vec2) -> Vec<WorldEdgeLine> {
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let centre = world.size * 0.5;
    // Half-extents along the body's own axes, so a sideways-gravity room
    // reports its own "how far down".
    let half_side = centre.dot(frame.side).abs();
    let half_fall = centre.dot(frame.down).abs();

    // A segment perpendicular to `axis`, `distance` along it from the
    // centre, extended past the room so it reads as a boundary.
    let segment = |axis: ae::Vec2, distance: f32, span: f32, always_lethal: bool| {
        let along = axis * distance;
        let across = ae::Vec2::new(-axis.y, axis.x) * (span + 240.0);
        WorldEdgeLine {
            from: centre + along - across,
            to: centre + along + across,
            always_lethal,
        }
    };

    // The fall line is always present. The other two appear only when the
    // stage opts in.
    let mut lines = vec![segment(
        frame.down,
        half_fall + world.edges.fall,
        half_side,
        true,
    )];
    if let Some(margin) = world.edges.rise {
        lines.push(segment(-frame.down, half_fall + margin, half_side, false));
    }
    if let Some(margin) = world.edges.side {
        lines.push(segment(frame.side, half_side + margin, half_fall, false));
        lines.push(segment(-frame.side, half_side + margin, half_fall, false));
    }
    lines
}

pub fn draw_micro_grid(gizmos: &mut Gizmos, world: &ae::World, minor: f32, major: f32) {
    if minor <= 0.0 || major <= 0.0 {
        return;
    }
    let minor_color = Color::srgba(0.45, 0.55, 0.70, 0.13);
    let major_color = Color::srgba(0.70, 0.80, 1.00, 0.23);
    let cols = (world.size.x / minor).ceil() as i32;
    let rows = (world.size.y / minor).ceil() as i32;
    for i in 0..=cols {
        let x = (i as f32 * minor).min(world.size.x);
        let is_major = (x / major).fract().abs() < 0.01;
        let color = if is_major { major_color } else { minor_color };
        gizmos.line_2d(
            w2(world, ae::Vec2::new(x, 0.0)),
            w2(world, ae::Vec2::new(x, world.size.y)),
            color,
        );
    }
    for i in 0..=rows {
        let y = (i as f32 * minor).min(world.size.y);
        let is_major = (y / major).fract().abs() < 0.01;
        let color = if is_major { major_color } else { minor_color };
        gizmos.line_2d(
            w2(world, ae::Vec2::new(0.0, y)),
            w2(world, ae::Vec2::new(world.size.x, y)),
            color,
        );
    }
}

/// Coarse grid drawn with gizmos. Used when `hide_sprites` removes the
/// authored sprite grid, so there is still a spatial reference. Spacing is
/// [`ambition_platformer2d_core::config::GRID_STEP`], like the sprite grid.
pub fn draw_world_grid(gizmos: &mut Gizmos, world: &ae::World) {
    let step = ambition_platformer2d_core::config::GRID_STEP;
    if step <= 0.0 {
        return;
    }
    let color = Color::srgba(0.45, 0.55, 0.70, 0.32);
    let cols = (world.size.x / step).ceil() as i32;
    let rows = (world.size.y / step).ceil() as i32;
    for i in 0..=cols {
        let x = (i as f32 * step).min(world.size.x);
        gizmos.line_2d(
            w2(world, ae::Vec2::new(x, 0.0)),
            w2(world, ae::Vec2::new(x, world.size.y)),
            color,
        );
    }
    for j in 0..=rows {
        let y = (j as f32 * step).min(world.size.y);
        gizmos.line_2d(
            w2(world, ae::Vec2::new(0.0, y)),
            w2(world, ae::Vec2::new(world.size.x, y)),
            color,
        );
    }
}

pub fn draw_world_blocks(gizmos: &mut Gizmos, world: &ae::World, developer_tools: &DeveloperTools) {
    for block in &world.blocks {
        let color = match block.kind {
            ae::BlockKind::Solid => gray(),
            ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Soft,
            } => magenta(),
            ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Hard,
            } => red(),
            ae::BlockKind::OneWay => blue(),
            // Developer overlay: draw hidden blocks, because that is what the
            // overlay is for.
            ae::BlockKind::BonkOnly => green(),
            ae::BlockKind::Hazard => red(),
            ae::BlockKind::PogoOrb => green(),
            ae::BlockKind::Rebound { .. } => orange(),
        };
        draw_aabb_styled(gizmos, world, block.aabb, color, developer_tools);
    }
}

/// Momentum-surface debug: draw every `SurfaceChain` segment, and at each
/// segment midpoint its tangent (green, toward increasing arc length) and its
/// outward normal (yellow, the `+normal` side a body rides). Vertices get a
/// small dot, so slopes and a loop's winding are readable.
pub fn draw_surface_chains(gizmos: &mut Gizmos, world: &ae::World) {
    let seg_color = Color::srgba(0.30, 0.90, 1.00, 0.85); // cyan — the surface line
    let normal_color = Color::srgba(1.00, 0.90, 0.20, 0.85); // yellow — ridden side
    let tangent_color = Color::srgba(0.40, 1.00, 0.55, 0.75); // green — arc direction
    let vertex_color = Color::srgba(1.00, 1.00, 1.00, 0.60);
    for chain in &world.chains {
        for &p in &chain.points {
            let c = w2(world, p);
            gizmos.line_2d(
                c + ae::Vec2::new(-3.0, 0.0),
                c + ae::Vec2::new(3.0, 0.0),
                vertex_color,
            );
            gizmos.line_2d(
                c + ae::Vec2::new(0.0, -3.0),
                c + ae::Vec2::new(0.0, 3.0),
                vertex_color,
            );
        }
        for i in 0..chain.segment_count() {
            let (a, b) = chain.segment(i);
            gizmos.line_2d(w2(world, a), w2(world, b), seg_color);
            let mid = (a + b) * 0.5;
            // Normal + tangent quills (world-space lengths; handles the flip).
            let n = chain.normal(i);
            let t = chain.tangent(i);
            gizmos.line_2d(w2(world, mid), w2(world, mid + n * 22.0), normal_color);
            gizmos.line_2d(w2(world, mid), w2(world, mid + t * 14.0), tangent_color);
        }
    }
}

pub fn draw_rebound_vectors(gizmos: &mut Gizmos, world: &ae::World) {
    for block in &world.blocks {
        let ae::BlockKind::Rebound { impulse } = block.kind else {
            continue;
        };
        draw_aabb(gizmos, world, block.aabb, orange());
        let start = w2(world, block.aabb.center());
        let direction = impulse.normalize_or(ae::Vec2::new(0.0, -1.0));
        let end = start + engine_delta_to_bevy(direction * 70.0);
        draw_arrow(gizmos, start, end, orange());
    }
}

pub fn draw_moving_platform_debug(
    gizmos: &mut Gizmos,
    world: &ae::World,
    moving_platforms: &[MovingPlatformState],
) {
    for platform in moving_platforms {
        let aabb = platform.aabb();
        draw_aabb(gizmos, world, aabb, blue());
        let center = w2(world, aabb.center());
        draw_arrow(gizmos, center, center + BVec2::new(44.0, 0.0), blue());
    }
}

// ─────────────────────────── the plugin ───────────────────────────

/// The opt-in F1 debug-visualization package for a game host: an F1 toggle on
/// the shared [`DeveloperRuntimeState::debug`] seam plus one draw pass over
/// the generic layers and a body/feature layer from the sim-view read models.
/// Shapes only. The per-layer [`DeveloperTools`] flags choose what F1 shows.
///
/// The sandbox app does not add this plugin; it uses the layer functions in
/// its own overlay and owns its hotkeys.
pub struct DebugVizPlugin {
    /// Whether the viz starts enabled. Games default to `false`: gameplay
    /// first, F1 to peek under the hood.
    pub start_enabled: bool,
}

impl Default for DebugVizPlugin {
    fn default() -> Self {
        Self {
            start_enabled: false,
        }
    }
}

impl Plugin for DebugVizPlugin {
    fn build(&self, app: &mut App) {
        // Thin-host safety: the shared sim stack normally owns these, but the
        // plugin must not panic without it.
        app.add_message::<ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction>(
        );
        app.init_resource::<DeveloperRuntimeState>();
        app.init_resource::<DeveloperTools>();
        app.init_resource::<FeatureViewIndex>();
        app.init_resource::<CombatGeometryView>();
        app.init_resource::<MovingPlatformSet>();
        let start_enabled = self.start_enabled;
        app.add_systems(
            Startup,
            move |mut dev_state: ResMut<DeveloperRuntimeState>| {
                // Shared state defaults to off; an embedding host can opt in for a
                // diagnostic build.
                dev_state.debug = start_enabled;
            },
        );
        // `.after(PresentedPoseSet)`: the overlay draws bodies and features at
        // their presented positions, so the resample must run first. Without
        // the edge Bevy picks an order for the conflict, and here it picked the
        // wrong one: the box used last frame's pose through this frame's camera
        // and shook beside a still sprite.
        app.add_systems(
            Update,
            (
                toggle_debug_viz,
                draw_debug_viz
                    .after(ambition_sim_view::PresentedPoseSet)
                    .run_if(session_world_exists)
                    // No gizmo stack means a no-op, not a panic. `Gizmos` needs
                    // `GizmoConfigStore` from `GizmoPlugin`, which comes with a
                    // renderer. A headless composition that adds this plugin (for
                    // example `-p ambition_demo_smash_app --features visible`) would
                    // otherwise fail parameter validation. `avatar::trail.rs` uses
                    // the same guard.
                    .run_if(bevy::ecs::schedule::common_conditions::resource_exists::<
                        bevy::gizmos::config::GizmoConfigStore,
                    >),
            )
                .chain(),
        );
    }
}

/// F1 toggles the shared debug flag, the same seam the sandbox's hotkeys and
/// the portal debug overlay bridge read.
pub fn toggle_debug_viz(
    mut actions: MessageReader<
        ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction,
    >,
    mut dev_state: ResMut<DeveloperRuntimeState>,
) {
    if actions.read().any(|action| {
        *action
            == ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction::ToggleDebugOverlay
    }) {
        dev_state.debug = !dev_state.debug;
    }
}

/// One pass over the generic layers. Bodies and features come from the
/// sim-view read models, never live sim clusters.
#[allow(clippy::too_many_arguments)]
pub fn draw_debug_viz(
    mut gizmos: Gizmos,
    world: SessionWorldRef<RoomGeometry>,
    dev_state: Res<DeveloperRuntimeState>,
    developer_tools: Res<DeveloperTools>,
    platform_set: Res<MovingPlatformSet>,
    features: Res<FeatureViewIndex>,
    combat_geometry: Res<CombatGeometryView>,
    // Gizmos are drawn through the camera, which moves on the render clock.
    // A box at the raw tick pose is a step function seen by a smooth camera,
    // so it shakes at the tick rate. Sampling the same frame clock as the
    // camera and sprite keeps it still; size, shape, and relation to the
    // art are unchanged.
    presented_features: Res<ambition_sim_view::PresentedFeaturePoses>,
    bodies: Query<(&BodyPoseView, Option<&ambition_sim_view::PresentedPose>)>,
    // The body presentation translation for every body in the combat view,
    // including bosses and actors, which `bodies` cannot reach
    // (`BodyPoseView` is player-bodied only).
    presented_bodies: Query<&ambition_sim_view::PresentedPose>,
    // Live gravity for the world-edge lines. `Option` because headless and
    // test apps do not insert it; then "down" is used.
    gravity: Option<Res<ambition_platformer2d_shared_tangle::gravity::GravityField>>,
) {
    if !dev_state.debug_enabled() || !developer_tools.gizmos_enabled {
        return;
    }
    let world = &world.0;
    if developer_tools.show_room_bounds {
        draw_room_bounds(&mut gizmos, world);
        draw_world_edges(
            &mut gizmos,
            world,
            ambition_platformer2d_shared_tangle::gravity::gravity_dir_or_default(
                gravity.as_deref(),
            ),
        );
    }
    if developer_tools.show_world_blocks {
        draw_world_blocks(&mut gizmos, world, &developer_tools);
        // Momentum ride surfaces share the block toggle.
        draw_surface_chains(&mut gizmos, world);
    }
    if developer_tools.show_micro_grid {
        draw_micro_grid(&mut gizmos, world, 8.0, 16.0);
    }
    if developer_tools.hide_sprites {
        draw_world_grid(&mut gizmos, world);
    }
    if developer_tools.show_rebound_vectors {
        draw_rebound_vectors(&mut gizmos, world);
    }
    if developer_tools.show_moving_platform {
        draw_moving_platform_debug(&mut gizmos, world, &platform_set.0);
    }
    // Two boxes for one player body, on purpose. This cyan one is the
    // collision box from the player pose view. `draw_combat_geometry_view`
    // draws the orange coarse envelope from the combat model. They match for
    // an ordinary body and differ for a boss, whose envelope is much larger.
    if developer_tools.show_player_hitbox || developer_tools.show_player_vectors {
        for (pose, presented) in &bodies {
            let draw_pos = ambition_sim_view::presented_pose::draw_pos(pose, presented);
            let body = ae::Aabb::new(draw_pos, pose.size * 0.5);
            if developer_tools.show_player_hitbox {
                draw_aabb_styled(&mut gizmos, world, body, cyan(), &developer_tools);
            }
            if developer_tools.show_player_vectors {
                let start = w2(world, draw_pos);
                // Velocity at ~0.15s of travel; facing as a short baseline tick.
                draw_arrow(
                    &mut gizmos,
                    start,
                    start + engine_delta_to_bevy(pose.vel * 0.15),
                    green(),
                );
                let facing = ae::Vec2::new(pose.facing.signum() * 18.0, 0.0);
                draw_arrow(
                    &mut gizmos,
                    start,
                    start + engine_delta_to_bevy(facing),
                    yellow(),
                );
            }
        }
    }
    if developer_tools.show_feature_hitboxes {
        for (id, view) in features.iter() {
            let color = match view.kind {
                FeatureVisualKind::Actor if !view.alive => gray(),
                FeatureVisualKind::Actor if view.fighting => red(),
                FeatureVisualKind::Actor => yellow(),
                FeatureVisualKind::Hazard => red(),
                FeatureVisualKind::Breakable => orange(),
                FeatureVisualKind::Chest => green(),
                FeatureVisualKind::Pickup => cyan(),
                FeatureVisualKind::Switch if view.switch_on => green(),
                FeatureVisualKind::Switch => red(),
            };
            // Same frame clock as the body box above, so an enemy's gizmo does not
            // shake against the camera.
            let aabb = ae::Aabb::new(presented_features.presented(id, view.pos), view.size * 0.5);
            draw_aabb_styled(&mut gizmos, world, aabb, color, &developer_tools);
        }
    }
    draw_combat_geometry_view(
        &mut gizmos,
        world,
        &combat_geometry,
        &developer_tools,
        &presentation_deltas(&combat_geometry, &presented_bodies),
    );
}

#[cfg(test)]
mod world_edge_overlay_tests {
    use super::*;

    fn stage(side: Option<f32>, ceiling: Option<f32>) -> ae::World {
        let mut world = ae::World::new(
            "overlay rig",
            ae::Vec2::new(960.0, 540.0),
            ae::Vec2::new(480.0, 270.0),
            Vec::new(),
        )
        .with_fall_out_margin(96.0);
        world.edges.side = side;
        world.edges.rise = ceiling;
        world
    }

    const DOWN: ae::Vec2 = ae::Vec2::new(0.0, 1.0);

    /// A room that opts into nothing shows one line. A side boundary here would
    /// tell a stage author that the corridor kills.
    #[test]
    fn a_room_that_opted_into_nothing_draws_only_its_pit() {
        let lines = world_edge_lines(&stage(None, None), DOWN);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].always_lethal);
        // 270 (half height) + 96 (margin) below the centre.
        assert!((lines[0].from.y - (270.0 + 270.0 + 96.0)).abs() < 0.01);
        assert!((lines[0].to.y - lines[0].from.y).abs() < 0.01, "level line");
    }

    /// Opting in adds exactly the opted-in boundaries: two sides and a ceiling,
    /// none marked always lethal.
    #[test]
    fn opting_in_draws_the_directions_that_were_opted_into() {
        let lines = world_edge_lines(&stage(Some(160.0), Some(64.0)), DOWN);
        assert_eq!(lines.len(), 4);
        assert_eq!(lines.iter().filter(|l| l.always_lethal).count(), 1);
        let xs: Vec<f32> = lines
            .iter()
            .filter(|l| (l.from.x - l.to.x).abs() < 0.01)
            .map(|l| l.from.x)
            .collect();
        assert_eq!(xs.len(), 2, "two vertical boundaries, one per side");
        // 480 (half width) + 160, either side of the centre.
        assert!(xs
            .iter()
            .any(|x| (*x - (480.0 + 480.0 + 160.0)).abs() < 0.01));
        assert!(xs.iter().any(|x| (*x + 160.0).abs() < 0.01));
    }

    /// The lines follow gravity, like the gate.
    ///
    /// Rotate gravity a quarter turn and the pit boundary becomes vertical.
    /// Otherwise the Noether Chamber would show a line the simulation does not
    /// use.
    #[test]
    fn the_boundaries_rotate_with_gravity() {
        let world = stage(None, None);
        let sideways = world_edge_lines(&world, ae::Vec2::new(1.0, 0.0));
        assert_eq!(sideways.len(), 1);
        let line = sideways[0];
        assert!(
            (line.from.x - line.to.x).abs() < 0.01,
            "under rightward gravity the pit boundary is a VERTICAL line, not a \
             horizontal one: {line:?}"
        );
        // Half-width along the fall axis is 480, plus the 96px margin.
        assert!(
            (line.from.x - (480.0 + 480.0 + 96.0)).abs() < 0.01,
            "{line:?}"
        );
    }
}

#[cfg(test)]
mod presented_strike_tests {
    use super::*;
    use ambition_sim_view::CombatStrikeGeometryView;

    fn strike(anchored_to_body: bool) -> CombatStrikeGeometryView {
        CombatStrikeGeometryView {
            volume: ae::CombatVolume::aabb(ae::Aabb::new(
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::splat(10.0),
            )),
            strike: bevy::prelude::Entity::from_raw_u32(1).unwrap(),
            owner: bevy::prelude::Entity::from_raw_u32(2).unwrap(),
            damage: 4,
            anchored_to_body,
            hit: Vec::new(),
        }
    }

    /// A body-anchored strike follows its owner's presented position. Drawing
    /// the raw tick volume beside a frame-clock body makes the box step while
    /// the fighter glides.
    #[test]
    fn a_body_anchored_strike_follows_its_owners_presented_position() {
        let volume = presented_strike_volume(&strike(true), ae::Vec2::new(7.5, 0.0));
        assert_eq!(
            volume.bounds().center(),
            ae::Vec2::new(107.5, 100.0),
            "the strike must move by the owner's presentation delta"
        );
        assert_eq!(
            volume.bounds().half_size(),
            ae::Vec2::splat(10.0),
            "translation must preserve the authoritative SHAPE exactly — the \
             overlay reports geometry, it does not recompute it"
        );
    }

    #[test]
    fn a_world_anchored_strike_stays_where_the_simulation_put_it() {
        let volume = presented_strike_volume(&strike(false), ae::Vec2::new(999.0, 0.0));
        assert_eq!(volume.bounds().center(), ae::Vec2::new(100.0, 100.0));
    }

    /// A zero delta is the correct answer: a body with no presented history is
    /// drawn at its simulated position.
    #[test]
    fn without_a_presented_owner_the_authoritative_geometry_is_drawn() {
        let volume = presented_strike_volume(&strike(true), ae::Vec2::ZERO);
        assert_eq!(volume.bounds().center(), ae::Vec2::new(100.0, 100.0));
    }

    /// One body, one translation.
    #[test]
    fn every_row_of_one_body_takes_the_same_translation() {
        let delta = ae::Vec2::new(6.0, -2.0);
        let collision = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(12.0, 24.0));
        let hurtbox = ae::CombatVolume::aabb(ae::Aabb::new(
            ae::Vec2::new(104.0, 90.0),
            ae::Vec2::splat(6.0),
        ));
        let strike_volume = presented_strike_volume(&strike(true), delta);

        let drawn_collision = collision.translated(delta);
        let drawn_hurtbox = hurtbox.translated(delta);

        for (label, before, after) in [
            ("collision", collision.center(), drawn_collision.center()),
            (
                "hurtbox",
                hurtbox.bounds().center(),
                drawn_hurtbox.bounds().center(),
            ),
            (
                "strike",
                strike(true).volume.bounds().center(),
                strike_volume.bounds().center(),
            ),
        ] {
            assert_eq!(
                after - before,
                delta,
                "{label} must move by the body's one delta, not its own",
            );
        }
        assert_eq!(
            drawn_collision.half_size(),
            collision.half_size(),
            "and nothing is resized on the way",
        );
        assert_eq!(
            drawn_hurtbox.bounds().half_size(),
            hurtbox.bounds().half_size(),
        );
    }

    /// The overlay and the unauthored-attack visual use the same rule.
    ///
    /// `draw_unauthored_attack_volumes` translates its polygon by the owner's
    /// `PresentedPose::delta()`. If the two disagreed, the polygon and the debug
    /// box would be in different places for one strike. Both take the same number
    /// from the same component.
    #[test]
    fn the_overlay_matches_the_unauthored_attack_visuals_rule() {
        let row = strike(true);
        let delta = ae::Vec2::new(12.0, -3.0);
        let overlay = presented_strike_volume(&row, delta);
        // The rule as `draw_unauthored_attack_volumes` spells it: mesh built
        // about the volume's own centre, transform placed at centre + delta.
        let unauthored_centre = row.volume.bounds().center() + delta;
        assert_eq!(overlay.bounds().center(), unauthored_centre);
    }
}
