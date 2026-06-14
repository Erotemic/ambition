//! "Tie a knot" — a verlet rope the player drags behind them (Jon's polish
//! list: "the player emits a trail, that might drape down onto the collision,
//! and perhaps be pulled taught"). The rope is a chain of verlet points pinned
//! at the player; gravity makes it hang, distance constraints keep it rope-like,
//! and (next increment) world collision makes it drape over ledges.
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
    players: Query<
        (Entity, &crate::player::BodyKinematics),
        (
            With<crate::player::PlayerEntity>,
            With<crate::player::PrimaryPlayer>,
            Without<PlayerTrailRope>,
        ),
    >,
) {
    for (entity, kin) in &players {
        let anchor = rope_anchor(kin);
        commands
            .entity(entity)
            .insert(PlayerTrailRope::hanging_from(anchor, ROPE_SEGMENTS, ROPE_SEGMENT_LEN));
    }
}

/// Advance the player's rope one sim step, pinned to the player's hip. Uses
/// `scaled_dt` so bullet-time / pause slow the rope with the rest of the world
/// (`[[feedback_world_time_pattern]]`).
pub fn update_player_rope(
    world_time: Res<crate::WorldTime>,
    mut players: Query<
        (&crate::player::BodyKinematics, &mut PlayerTrailRope),
        With<crate::player::PlayerEntity>,
    >,
) {
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
    }
}

/// The rope's pin point: the player's hip (centre, lowered slightly) so the
/// trail reads as dragged from the body rather than the head.
fn rope_anchor(kin: &crate::player::BodyKinematics) -> ae::Vec2 {
    use crate::engine_core::AabbExt;
    kin.aabb().center()
}

/// Passive plugin: keeps the player carrying a verlet trail rope. Renderer +
/// world-collision drape + portal transit are the documented next increments
/// (the rope simulates correctly today; it's just not yet drawn or collided).
pub struct PlayerRopePlugin;

impl Plugin for PlayerRopePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (ensure_player_rope, update_player_rope).chain());
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
    fn an_empty_rope_is_a_no_op() {
        // Degenerate guard — never panics on an empty point list.
        let mut points: Vec<RopePoint> = Vec::new();
        verlet_step(&mut points, ae::Vec2::ZERO, ROPE_GRAVITY, 1.0 / 60.0, ROPE_SEGMENT_LEN, 4);
        assert!(points.is_empty());
    }
}
