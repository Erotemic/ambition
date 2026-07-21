//! Engine-generic debug visualizations — the F1 gizmo layers any game opts
//! into.
//!
//! The movement engine exposes simulation state; this module decides how to
//! visualize the GENERIC part of it: world collision blocks (color-keyed by
//! `BlockKind`), momentum surface chains with their normal/tangent quills,
//! rebound-pad impulse vectors, moving platforms, room bounds, grids, and a
//! body/feature layer drawn from the sim-view read-models (`BodyPoseView`,
//! `FeatureViewIndex`) — never from live sim components, so the layer works in
//! any host that renders at all.
//!
//! Two consumers:
//!
//! - [`DebugVizPlugin`] — the whole package for a game that has no debug stack
//!   of its own (the demo apps): an F1 toggle on the shared
//!   [`SandboxDevState::debug`] seam plus one draw system over these layers.
//!   Games start with the viz OFF and press F1 to opt in.
//! - The sandbox's own richer overlay (`ambition_app::dev::debug_overlay`)
//!   imports the layer/primitive functions from here and composes them with
//!   its game-specific layers (authored combat volumes, boss clusters, LDtk
//!   spine, portals). It does NOT add the plugin — it owns its own hotkeys.
//!
//! NOT a dev HUD: this module draws shapes, nothing else.

use ambition_dev_tools::dev_tools::DeveloperTools;
use ambition_dev_tools::SandboxDevState;
use ambition_engine_core as ae;
use ambition_engine_core::config::world_to_bevy;
use ambition_engine_core::{AabbExt, RoomGeometry};
use ambition_platformer_primitives::feature_kind::FeatureVisualKind;
use ambition_platformer_primitives::lifecycle::{session_world_exists, SessionWorldRef};
use ambition_sim_view::{BodyPoseView, FeatureViewIndex};
use ambition_world::collision::MovingPlatformSet;
use ambition_world::platforms::MovingPlatformState;
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

/// World-space DELTA (direction/offset) → Bevy 2D (y-flip, no origin shift).
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
    // Bevy gizmos' `rect_2d` draws the outline by default. We want a
    // filled appearance, so draw a stack of horizontal lines spaced
    // 2px apart — works on every Bevy gizmo backend without needing a
    // separate mesh path. The cost is bounded (each AABB is small in
    // pixel terms and we only call this when the toggle is on).
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
/// polygon. Lets an overlay show the ACTUAL shaped hitbox (a blade-arc poly)
/// instead of its bounding box.
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

/// Draw a live hitbox's TRUE damage volume — the shape damage resolution
/// actually tests, not a re-derived preview. When the hitbox authors a hull
/// (a convex attack blade, an OBB, a circle) the hull is drawn prominently and
/// its bounding box is reduced to a faint, vestigial broad-phase outline. A
/// bare `Aabb` volume has no separate hull, so the box IS the volume (normal
/// styled fill).
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

/// Lightweight coarse grid drawn straight through gizmos. Used when
/// `hide_sprites` strips the authored sprite grid so the player still has a
/// spatial reference. Spacing matches
/// [`ambition_engine_core::config::GRID_STEP`] (the same step the sprite grid
/// uses).
pub fn draw_world_grid(gizmos: &mut Gizmos, world: &ae::World) {
    let step = ambition_engine_core::config::GRID_STEP;
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
            ae::BlockKind::Hazard => red(),
            ae::BlockKind::PogoOrb => green(),
            ae::BlockKind::Rebound { .. } => orange(),
        };
        draw_aabb_styled(gizmos, world, block.aabb, color, developer_tools);
    }
}

/// Momentum-surface debug: draw every `SurfaceChain` — its segments, and at
/// each segment midpoint its TANGENT (green, along increasing arc length) and
/// its outward NORMAL (yellow, the `+normal` side a body rides). Vertices get
/// a small dot, so the ride geometry (slopes, a loop's interior winding) is
/// legible without playing it.
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
            // Normal + tangent quills (world-space lengths; w2 handles the flip).
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
/// the shared [`SandboxDevState::debug`] seam plus one draw pass over the
/// generic layers above and a body/feature layer from the sim-view
/// read-models. No dev HUD, no inspectors — shapes only. The per-layer
/// [`DeveloperTools`] flags (already in the debug-first posture on desktop)
/// choose what F1 reveals.
///
/// The sandbox app does NOT add this plugin — it composes the same layer
/// functions inside its own richer overlay and owns its own hotkeys.
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
        // plugin must not panic in a host that draws without it.
        app.add_message::<ambition_platformer_primitives::developer_hotkeys::DeveloperAction>();
        app.init_resource::<SandboxDevState>();
        app.init_resource::<DeveloperTools>();
        app.init_resource::<FeatureViewIndex>();
        app.init_resource::<MovingPlatformSet>();
        let start_enabled = self.start_enabled;
        app.add_systems(Startup, move |mut dev_state: ResMut<SandboxDevState>| {
            // Shared state defaults clean for every game; an embedding host
            // may still opt in explicitly for a dedicated diagnostic build.
            dev_state.debug = start_enabled;
        });
        app.add_systems(
            Update,
            (
                toggle_debug_viz,
                draw_debug_viz.run_if(session_world_exists),
            )
                .chain(),
        );
    }
}

/// F1 flips the shared debug flag — the same seam the sandbox's hotkeys and
/// the portal debug overlay bridge read.
pub fn toggle_debug_viz(
    mut actions: MessageReader<ambition_platformer_primitives::developer_hotkeys::DeveloperAction>,
    mut dev_state: ResMut<SandboxDevState>,
) {
    if actions.read().any(|action| {
        *action
            == ambition_platformer_primitives::developer_hotkeys::DeveloperAction::ToggleDebugOverlay
    }) {
        dev_state.debug = !dev_state.debug;
    }
}

/// One pass over the generic layers. Bodies and features are drawn from the
/// sim-view read-models — presentation reads facts, never live sim clusters.
#[allow(clippy::too_many_arguments)]
pub fn draw_debug_viz(
    mut gizmos: Gizmos,
    world: SessionWorldRef<RoomGeometry>,
    dev_state: Res<SandboxDevState>,
    developer_tools: Res<DeveloperTools>,
    platform_set: Res<MovingPlatformSet>,
    features: Res<FeatureViewIndex>,
    // Gizmos are drawn THROUGH the camera, and the camera advances on the
    // render clock. A box placed at the raw tick pose is therefore a step
    // function sampled by a smoothly-moving observer, which reads as a
    // horizontal sawtooth at the tick rate — the box shakes even though the
    // simulation is perfectly regular. Sampling the same frame clock as the
    // camera and the sprite is what makes the overlay STILL, and it costs no
    // truthfulness: the size, the shape, and the box's relationship to the art
    // are all unchanged. Only the sub-tick sampling phase matches its viewer.
    presented_features: Res<ambition_sim_view::PresentedFeaturePoses>,
    bodies: Query<(&BodyPoseView, Option<&ambition_sim_view::PresentedPose>)>,
) {
    if !dev_state.debug_enabled() || !developer_tools.gizmos_enabled {
        return;
    }
    let world = &world.0;
    if developer_tools.show_room_bounds {
        draw_room_bounds(&mut gizmos, world);
    }
    if developer_tools.show_world_blocks {
        draw_world_blocks(&mut gizmos, world, &developer_tools);
        // Momentum ride-surfaces live alongside the blocks: the SurfaceChains
        // + their normals/tangents share the toggle.
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
            // Same frame clock as the body box above: an enemy's gizmo would
            // otherwise shake against the camera exactly as the player's did.
            let aabb = ae::Aabb::new(presented_features.presented(id, view.pos), view.size * 0.5);
            draw_aabb_styled(&mut gizmos, world, aabb, color, &developer_tools);
        }
    }
}
