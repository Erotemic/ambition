//! Despite the name, NOT a grapple rope — a generic verlet **trail** the player
//! drags behind them. A chain of verlet points pinned at the player; gravity
//! makes it hang, distance constraints keep it rope-like, and (next increment)
//! world collision makes it drape over ledges. Intended as the substrate for
//! future "homotopy" skills/quests (a deformable curve through the world), not
//! a traversal/pull mechanic.
//!
//! This module is the **pure, deterministic simulation** — no Bevy, no RNG — so
//! the rope's shape is headless-testable even though its on-screen line is not.
//! The ECS system + spawn + render wiring layer on top of [`verlet_step`].
//!
//! Sim convention: **+Y is down** (so gravity is `+y` and the rope hangs toward
//! larger `y`), matching the rest of the sandbox physics.

use crate::engine_core as ae;
use bevy::prelude::*;

/// Number of rope segments (so `SEGMENTS + 1` points). Short enough to verlet
/// cheaply every tick, long enough to drape a body-height ledge.
pub const ROPE_SEGMENTS: usize = 14;
/// Rest length of each segment (px). `SEGMENTS * SEGMENT_LEN` ≈ the rope's reach.
pub const ROPE_SEGMENT_LEN: f32 = 18.0;
/// Downward acceleration on the free points (px/s²). Heavier than player gravity
/// so the rope settles quickly into a readable drape rather than floating.
pub const ROPE_GRAVITY: f32 = 1400.0;
/// Jakobsen constraint-relaxation iterations per tick. More = stiffer / less
/// stretchy rope; 16 keeps a 14-segment rope visually inextensible.
pub const ROPE_CONSTRAINT_ITERS: usize = 16;
/// Velocity retention per tick (verlet implicit damping). `< 1.0` bleeds energy
/// so the rope doesn't oscillate forever after the player stops.
pub const ROPE_DAMPING: f32 = 0.97;

/// Feature gate for the rope trail.
///
/// The trail is disabled by default. We can wire an explicit toggle later
/// without changing the simulation plumbing again.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PlayerTrailRopeEnabled {
    pub enabled: bool,
}

/// One verlet point: current + previous position (velocity is implicit in their
/// difference, the verlet integration trick).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RopePoint {
    pub pos: ae::Vec2,
    pub prev: ae::Vec2,
}

/// The player's dragged rope. `points[0]` is pinned to the player each tick; the
/// rest are free verlet points.
#[derive(Component, Clone, Debug)]
pub struct PlayerTrailRope {
    pub points: Vec<RopePoint>,
}

impl PlayerTrailRope {
    /// A fresh rope hanging straight down from `anchor` (all points at rest, so
    /// the first frame doesn't snap).
    pub fn hanging_from(anchor: ae::Vec2, segments: usize, seg_len: f32) -> Self {
        let points = (0..=segments)
            .map(|i| {
                let p = ae::Vec2::new(anchor.x, anchor.y + i as f32 * seg_len);
                RopePoint { pos: p, prev: p }
            })
            .collect();
        Self { points }
    }
}

/// Advance the rope one step: pin `points[0]` to `anchor`, integrate the free
/// points under gravity, then relax the segment distance constraints toward
/// `seg_len`. Pure + deterministic — the testable core of the rope.
pub fn verlet_step(
    points: &mut [RopePoint],
    anchor: ae::Vec2,
    gravity: f32,
    dt: f32,
    seg_len: f32,
    iters: usize,
) {
    if points.is_empty() {
        return;
    }
    // Pin the head to the player (carry no velocity into it).
    points[0].pos = anchor;
    points[0].prev = anchor;

    // Verlet integrate the free points: x' = x + (x - x_prev) * damping + a*dt².
    let accel = ae::Vec2::new(0.0, gravity);
    let dt2 = dt * dt;
    for p in points.iter_mut().skip(1) {
        let vel = (p.pos - p.prev) * ROPE_DAMPING;
        p.prev = p.pos;
        p.pos = p.pos + vel + accel * dt2;
    }

    // Jakobsen relaxation: pull each segment back toward its rest length,
    // keeping the head pinned. A few iterations make the rope ~inextensible.
    for _ in 0..iters {
        points[0].pos = anchor;
        for i in 0..points.len() - 1 {
            let a = points[i].pos;
            let b = points[i + 1].pos;
            let delta = b - a;
            let dist = delta.length();
            if dist < 1e-6 {
                continue;
            }
            let diff = (dist - seg_len) / dist;
            let correction = delta * (0.5 * diff);
            if i == 0 {
                // Head is pinned — move only the tail point of this segment, by
                // the full correction.
                points[i + 1].pos = b - correction * 2.0;
            } else {
                points[i].pos = a + correction;
                points[i + 1].pos = b - correction;
            }
        }
    }
}

/// Give the primary player a rope if it doesn't have one yet (so the trail
/// starts hanging from wherever the player currently is, no first-frame snap).
pub fn ensure_player_rope(
    mut commands: Commands,
    enabled: Option<Res<PlayerTrailRopeEnabled>>,
    players: Query<
        (Entity, &crate::player::BodyKinematics),
        (
            With<crate::player::PlayerEntity>,
            With<crate::player::PrimaryPlayer>,
            Without<PlayerTrailRope>,
        ),
    >,
) {
    if !enabled.is_some_and(|enabled| enabled.enabled) {
        return;
    }
    for (entity, kin) in &players {
        let anchor = rope_anchor(kin);
        commands
            .entity(entity)
            .insert(PlayerTrailRope::hanging_from(
                anchor,
                ROPE_SEGMENTS,
                ROPE_SEGMENT_LEN,
            ));
    }
}

/// Drape the rope over world solids: each free point sweeps from its previous to
/// its new position; if that move crosses a solid surface, the point rests on
/// the surface instead of tunnelling through (and its velocity into the surface
/// is killed, so it drapes rather than bounces). This is the "drape down onto the
/// collision" half of the feature. Pure given a world — testable with a fixture.
pub fn resolve_rope_collisions(points: &mut [RopePoint], world: &ae::World) {
    // Skip the pinned head (it lives wherever the player is, even inside geometry
    // briefly during a transit).
    for p in points.iter_mut().skip(1) {
        let delta = p.pos - p.prev;
        let dist = delta.length();
        if dist < 1e-4 {
            continue;
        }
        let dir = delta / dist;
        if let Some((hit, _normal)) =
            crate::platformer_runtime::collision::raycast_solids(world, p.prev, dir, dist, false)
        {
            // Rest on the surface; pin prev=pos so it doesn't carry momentum into
            // the solid next tick (a drape, not a bounce).
            p.pos = hit;
            p.prev = hit;
        }
    }
}

/// Advance the player's rope one sim step, pinned to the player's hip, then drape
/// it over the world. Uses `sim_dt` so bullet-time / pause slow the rope with the
/// rest of the world (`[[feedback_world_time_pattern]]`).
pub fn update_player_rope(
    world_time: Res<crate::WorldTime>,
    world: Option<Res<crate::GameWorld>>,
    enabled: Option<Res<PlayerTrailRopeEnabled>>,
    mut players: Query<
        (&crate::player::BodyKinematics, &mut PlayerTrailRope),
        With<crate::player::PlayerEntity>,
    >,
) {
    if !enabled.is_some_and(|enabled| enabled.enabled) {
        return;
    }
    let dt = world_time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (kin, mut rope) in &mut players {
        let anchor = rope_anchor(kin);
        verlet_step(
            &mut rope.points,
            anchor,
            ROPE_GRAVITY,
            dt,
            ROPE_SEGMENT_LEN,
            ROPE_CONSTRAINT_ITERS,
        );
        if let Some(w) = world.as_deref() {
            resolve_rope_collisions(&mut rope.points, &w.0);
        }
    }
}

/// The rope's pin point: the player's hip (centre, lowered slightly) so the
/// trail reads as dragged from the body rather than the head.
fn rope_anchor(kin: &crate::player::BodyKinematics) -> ae::Vec2 {
    use crate::engine_core::AabbExt;
    kin.aabb().center()
}

/// Hemp/manila rope colour — warm brown, fully opaque so the line reads against
/// both bright and dark backgrounds.
const ROPE_COLOR: Color = Color::srgb(0.62, 0.47, 0.30);

/// Draw the rope as a single gizmo linestrip through its points, mapped from sim
/// space into Bevy world space. This is presentation-only (no sim state touched),
/// so it is harness-blind but replay-neutral. Drawn just behind the player so the
/// rope reads as trailing from the body.
pub fn render_player_rope(
    world: Option<Res<crate::GameWorld>>,
    enabled: Option<Res<PlayerTrailRopeEnabled>>,
    ropes: Query<&PlayerTrailRope, With<crate::player::PlayerEntity>>,
    mut gizmos: Gizmos,
) {
    if !enabled.is_some_and(|enabled| enabled.enabled) {
        return;
    }
    let Some(world) = world.as_deref() else {
        return;
    };
    let z = crate::config::WORLD_Z_PLAYER - 0.1;
    for rope in &ropes {
        if rope.points.len() < 2 {
            continue;
        }
        let pts = rope
            .points
            .iter()
            .map(|p| crate::config::world_to_bevy(&world.0, p.pos, z).truncate());
        gizmos.linestrip_2d(pts, ROPE_COLOR);
    }
}

/// Passive plugin: the rope trail is disabled by default, but once enabled it
/// keeps the player carrying a verlet trail rope, simulates its drape against
/// world solids, and draws it as a gizmo linestrip. Portal transit (the rope
/// threading an aperture) is the documented next increment.
pub struct PlayerRopePlugin;

impl Plugin for PlayerRopePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerTrailRopeEnabled>();
        app.add_systems(
            Update,
            (
                (ensure_player_rope, update_player_rope).chain(),
                // Gizmo rendering only runs once the GizmoPlugin's config store
                // exists — headless apps (replay/tests) carry no gizmos, so the
                // draw system simply doesn't run there.
                render_player_rope
                    .run_if(resource_exists::<bevy::gizmos::config::GizmoConfigStore>),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(anchor: ae::Vec2, frames: usize) -> Vec<RopePoint> {
        // Start the rope laid out HORIZONTALLY (all at anchor.y) so we can watch
        // it fall and drape, not just confirm a pre-hung shape.
        let mut points: Vec<RopePoint> = (0..=ROPE_SEGMENTS)
            .map(|i| {
                let p = ae::Vec2::new(anchor.x + i as f32 * ROPE_SEGMENT_LEN, anchor.y);
                RopePoint { pos: p, prev: p }
            })
            .collect();
        for _ in 0..frames {
            verlet_step(
                &mut points,
                anchor,
                ROPE_GRAVITY,
                1.0 / 60.0,
                ROPE_SEGMENT_LEN,
                ROPE_CONSTRAINT_ITERS,
            );
        }
        points
    }

    #[test]
    fn rope_head_stays_pinned_to_the_anchor() {
        let anchor = ae::Vec2::new(640.0, 400.0);
        let points = settle(anchor, 5);
        assert_eq!(points[0].pos, anchor, "head is pinned to the player");
    }

    #[test]
    fn rope_falls_and_hangs_below_the_anchor() {
        let anchor = ae::Vec2::new(640.0, 400.0);
        let points = settle(anchor, 240); // ~4s — plenty to settle
                                          // +Y is down: a settled rope hangs, so the tail sits well below the head.
        let tail = points.last().unwrap();
        assert!(
            tail.pos.y > anchor.y + ROPE_SEGMENT_LEN * 4.0,
            "tail {:?} should hang well below the anchor {anchor:?}",
            tail.pos,
        );
        // And it hangs roughly straight down (no sideways drift without forces).
        assert!(
            (tail.pos.x - anchor.x).abs() < ROPE_SEGMENT_LEN * 2.0,
            "settled rope hangs ~straight down, tail x={} vs anchor x={}",
            tail.pos.x,
            anchor.x,
        );
    }

    #[test]
    fn segments_stay_near_their_rest_length() {
        let anchor = ae::Vec2::new(640.0, 400.0);
        let points = settle(anchor, 240);
        for w in points.windows(2) {
            let len = (w[1].pos - w[0].pos).length();
            assert!(
                (len - ROPE_SEGMENT_LEN).abs() < ROPE_SEGMENT_LEN * 0.25,
                "segment length {len} drifted from rest {ROPE_SEGMENT_LEN} — rope is too stretchy",
            );
        }
    }

    #[test]
    fn rope_drapes_over_a_floor_instead_of_tunnelling_through() {
        // A floor solid at y[200,220] spanning the arena; the anchor sits above
        // it at y=100. Free-hanging the rope would reach y ≈ 100 + 14*18 = 352,
        // well past the floor — so a correct drape must rest it on top (~y=200).
        let world = ae::World::new(
            "rope_drape_test",
            ae::Vec2::new(900.0, 400.0),
            ae::Vec2::new(450.0, 50.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 200.0),
                ae::Vec2::new(900.0, 20.0),
            )],
        );
        let anchor = ae::Vec2::new(450.0, 100.0);
        // Lay the rope out HORIZONTALLY just above the floor so it has to fall
        // onto it (spread points so the segment constraints engage — coincident
        // points have no direction to relax along).
        let mut points: Vec<RopePoint> = (0..=ROPE_SEGMENTS)
            .map(|i| {
                let p = ae::Vec2::new(anchor.x + i as f32 * ROPE_SEGMENT_LEN, anchor.y);
                RopePoint { pos: p, prev: p }
            })
            .collect();
        for _ in 0..300 {
            verlet_step(
                &mut points,
                anchor,
                ROPE_GRAVITY,
                1.0 / 60.0,
                ROPE_SEGMENT_LEN,
                ROPE_CONSTRAINT_ITERS,
            );
            resolve_rope_collisions(&mut points, &world);
        }
        // No point sinks meaningfully past the floor's top face (y=200).
        for p in &points {
            assert!(
                p.pos.y <= 200.0 + ROPE_SEGMENT_LEN,
                "rope point {:?} tunnelled through the floor (top y=200)",
                p.pos,
            );
        }
        // And the rope actually reached the floor (it didn't just float) — the
        // tail rests near it, not way up at the anchor.
        assert!(
            points.last().unwrap().pos.y > 150.0,
            "the rope should have fallen down onto the floor",
        );
    }

    #[test]
    fn rope_is_disabled_by_default() {
        assert!(
            !PlayerTrailRopeEnabled::default().enabled,
            "the trail should start disabled until an explicit toggle exists",
        );
    }

    #[test]
    fn an_empty_rope_is_a_no_op() {
        // Degenerate guard — never panics on an empty point list.
        let mut points: Vec<RopePoint> = Vec::new();
        verlet_step(
            &mut points,
            ae::Vec2::ZERO,
            ROPE_GRAVITY,
            1.0 / 60.0,
            ROPE_SEGMENT_LEN,
            4,
        );
        assert!(points.is_empty());
    }
}
