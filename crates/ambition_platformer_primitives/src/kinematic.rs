//! Generic kinematic body — gravity + axis-separated sweep against a `World`.
//!
//! Why this exists: the player's `movement` module owns a sophisticated
//! sweep with jump-buffer, dash, blink, climb, and other player-only
//! affordances. Enemies and NPCs only need a small subset (gravity,
//! ground/wall collision, optional drop-through), but the sandbox used
//! to ship hand-rolled `blocked` / `blocked_y` predicates that diverged
//! from player physics in subtle ways — most visibly, hostile NPCs and
//! chasing enemies could not drop through one-way platforms or fall off
//! ledges in the same situations the player could.
//!
//! [`KinematicBody`] + [`step_kinematic`] are the shared sweep both
//! enemies and NPCs go through. Authored player physics still lives
//! in `movement`, but it agrees with this primitive on the load-bearing
//! semantics:
//!
//! - `Solid` and `BlinkWall` always block both axes.
//! - `OneWay` never blocks horizontal motion. Vertically, it blocks
//!   only when the body is *landing from above* (downward velocity AND
//!   the previous-frame bottom was at or above the platform top).
//! - `drop_through` set on a tick suppresses the OneWay vertical block
//!   so a chasing enemy can follow a player who dropped through the
//!   same platform a frame earlier.
//! - `Hazard`, `PogoOrb`, and `Rebound` are visited by gameplay logic
//!   (damage, bounce, impulse) elsewhere; they are not collision blockers
//!   for kinematic bodies.
//!
//! When/if the player migrates to this primitive, the player's tuning
//! gains a few abilities-shaped fields and we delete the duplicate
//! sweep helpers in `movement`.

use ambition_engine_core::Vec2;
use ambition_engine_core::{Aabb, AabbExt};
use ambition_engine_core::{BlockKind, World};

/// Per-tick configuration for [`step_kinematic`].
#[derive(Clone, Copy, Debug)]
pub struct KinematicTuning {
    pub gravity: f32,
    /// Maximum fall speed (pixels/sec), measured ALONG `gravity_dir`.
    pub max_fall_speed: f32,
    /// Unit gravity DIRECTION (cardinal): down `(0,1)`, up `(0,-1)`, or sideways
    /// `(±1,0)`. Gravity accelerates the body along this, and "ground" is a
    /// contact on this (feet) side — so actors fall the way the player does,
    /// including SIDEWAYS. (Supersedes the Y-only `gravity_sign`, which only
    /// handled down/up: the reason enemies/NPCs didn't fall under left/right
    /// gravity. Vertical gravity is byte-identical: `gravity_dir.y` is the old
    /// `gravity_sign`.)
    pub gravity_dir: Vec2,
}

/// Per-tick AI/control inputs to [`step_kinematic`].
#[derive(Clone, Copy, Debug, Default)]
pub struct KinematicInputs {
    /// Suppress the OneWay vertical block this tick so the body falls
    /// through the platform it is currently standing on. Mirrors the
    /// player's `drop_through_pressed` input.
    pub drop_through: bool,
}

/// A body that gravity pulls down and the world sweeps horizontally.
///
/// The shape is intentionally minimal: position, velocity, size,
/// `on_ground`, and `facing`. AI / brain code lives in callers (enemy
/// chase, NPC patrol, future RL agent inputs); this struct just owns
/// the axis-separated sweep.
#[derive(Clone, Copy, Debug)]
pub struct KinematicBody {
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub on_ground: bool,
    /// +1 right, -1 left. Updated by callers — this primitive does not
    /// flip facing, but it is a useful place for shared state.
    pub facing: f32,
}

impl KinematicBody {
    pub fn new(pos: Vec2, size: Vec2) -> Self {
        Self {
            pos,
            vel: Vec2::ZERO,
            size,
            on_ground: false,
            facing: 1.0,
        }
    }

    pub fn aabb(&self) -> Aabb {
        Aabb::new(self.pos, self.size * 0.5)
    }
}

/// Apply one frame of physics: gravity, then independent X/Y sweeps.
///
/// Returns the new `on_ground` state in `body.on_ground`. Does not
/// touch `facing`; callers update it from their AI signal (chase
/// direction, patrol bound bounce, etc).
pub fn step_kinematic(
    body: &mut KinematicBody,
    world: &World,
    tuning: KinematicTuning,
    inputs: KinematicInputs,
    dt: f32,
) {
    // 1. Gravity along the world's `gravity_dir` (down / up / sideways), capped
    //    so a long fall doesn't tunnel. `sign` = the vertical component, the old
    //    `gravity_sign`, used by the Y-axis one-way + ground logic; it's 0 under
    //    sideways gravity (so the Y sweep is pure collision and the X sweep owns
    //    landing instead).
    let g = tuning.gravity_dir;
    let sign = g.y;
    // Terminal velocity is an equilibrium gravity accelerates UP TO, not a brake
    // that decelerates a body already moving faster (e.g. one flung out of a
    // portal). Raise the effective cap to at least the body's pre-gravity
    // fall-direction speed: a normal fall (below the cap) is unchanged, while
    // an over-cap fling is preserved instead of clipped back on the next tick.
    let fall_before = body.vel.dot(g).max(0.0);
    let cap = tuning.max_fall_speed.max(fall_before);
    body.vel += tuning.gravity * g * dt;
    let along = body.vel.dot(g);
    if along > cap {
        body.vel -= (along - cap) * g;
    }

    // Capture the bottom edge BEFORE we move so the OneWay direction
    // check (was the body above the platform?) reads the previous-tick
    // position, not the post-step one. Same reference frame the
    // player's `sweep_player_y` uses.
    let prev_bottom = body.aabb().bottom();

    let mut grounded = false;

    // 2. X sweep. Solid + BlinkWall block; OneWay never blocks horizontally.
    //    Under SIDEWAYS gravity the wall the body falls into IS its floor, so a
    //    block while moving along gravity counts as landing.
    let old_x = body.pos.x;
    let falling_along_x = body.vel.x * g.x > 0.0;
    body.pos.x += body.vel.x * dt;
    if body_blocked_x(body.aabb(), world) {
        body.pos.x = old_x;
        body.vel.x = 0.0;
        if g.x != 0.0 && falling_along_x {
            grounded = true;
        }
    }

    // 3. Y sweep. Solid + BlinkWall always block. OneWay blocks only
    //    when (a) the body is moving downward and was above the
    //    platform top last frame, AND (b) drop_through is not set.
    let old_y = body.pos.y;
    body.pos.y += body.vel.y * dt;
    // "Falling" = moving along gravity, so a flipped-gravity body that rises
    // into a ceiling still registers as landing (on_ground).
    let was_falling = body.vel.y * sign >= 0.0;
    if body_blocked_y(
        body.aabb(),
        world,
        prev_bottom,
        was_falling,
        inputs.drop_through,
        sign,
    ) {
        body.pos.y = old_y;
        body.vel.y = 0.0;
        if g.y != 0.0 && was_falling {
            grounded = true;
        }
    }
    body.on_ground = grounded;

    // Emergent platform riding: a body resting on a MOVING solid is carried by that
    // solid's per-frame `velocity`. Only the gravity-PERPENDICULAR component is added
    // — the gravity-axis ride is already handled by gravity + the landing collision,
    // so adding it would double-count a vertical lift. Static solids carry `ZERO`, so
    // this is a no-op off moving platforms. No rider list, no per-actor flag: any body
    // that lands on a moving platform rides it because it is a body resting on a moving
    // solid. (Probe the body's own footprint nudged into the gravity side to read the
    // supporting block; orientation-correct, so wall-walking rides sideways platforms.)
    if grounded {
        let support_probe = Aabb::new(body.pos + g * 2.0, body.size * 0.5);
        if let Some(support) = world.first_overlapping_block(support_probe, |block| {
            matches!(
                block.kind,
                BlockKind::Solid | BlockKind::BlinkWall { .. } | BlockKind::OneWay
            )
        }) {
            let v = support.velocity;
            body.pos += v - v.dot(g) * g;
        }
    }

    // Gravity-axis depenetration runs on the VERTICAL axis (down/up gravity); the
    // sideways case relies on the X sweep above. Skip it under sideways gravity so
    // it never fights the horizontal landing.
    if g.y == 0.0 {
        return;
    }

    // Gravity-axis depenetration: if the body ENDS the tick overlapping a solid
    // (the revert above only undoes the current move, not a pre-existing overlap —
    // e.g. gravity flipped while it rested inside a platform, or it got nudged in by
    // the X sweep), push it out toward -gravity so its feet rest flush. Guarded to
    // the gravity side + the shallower (landing-type) axis so it never fights the X
    // (wall) resolver. Byte-identical when there is no overlap.
    let b = body.aabb();
    for block in &world.blocks {
        if !matches!(block.kind, BlockKind::Solid | BlockKind::BlinkWall { .. }) {
            continue;
        }
        let blk = block.aabb;
        if !b.strict_intersects(blk) {
            continue;
        }
        let y_overlap = (b.bottom().min(blk.bottom()) - b.top().max(blk.top())).max(0.0);
        let x_overlap = (b.right().min(blk.right()) - b.left().max(blk.left())).max(0.0);
        if y_overlap <= 0.0 || y_overlap > x_overlap {
            continue; // a wall hit (or no Y overlap) — leave it to the X resolver
        }
        let body_on_head_side = if sign >= 0.0 {
            b.center().y < blk.center().y
        } else {
            b.center().y > blk.center().y
        };
        if !body_on_head_side {
            continue;
        }
        body.pos.y += if sign >= 0.0 { -y_overlap } else { y_overlap };
        body.on_ground = true;
        body.vel.y = 0.0;
        break;
    }
}

fn body_blocked_x(aabb: Aabb, world: &World) -> bool {
    world.body_overlaps_any(aabb, |block| {
        matches!(block.kind, BlockKind::Solid | BlockKind::BlinkWall { .. })
    })
}

fn body_blocked_y(
    aabb: Aabb,
    world: &World,
    prev_bottom: f32,
    falling: bool,
    drop_through: bool,
    gravity_sign: f32,
) -> bool {
    let prev_top = prev_bottom - (aabb.bottom() - aabb.top());
    world.body_overlaps_any(aabb, |block| match block.kind {
        BlockKind::Solid | BlockKind::BlinkWall { .. } => true,
        BlockKind::OneWay => {
            // Drop-through: skip OneWay this tick entirely.
            if drop_through {
                return false;
            }
            // Land on the one-way's gravity-up face — its TOP under normal
            // gravity, its BOTTOM under flipped — matching the player's
            // `sweep_player_y` (gravity-relative), so enemies/NPCs land on the
            // correct side of a one-way under inverted gravity.
            if gravity_sign >= 0.0 {
                falling && prev_bottom <= block.aabb.top() + 8.0
            } else {
                falling && prev_top >= block.aabb.bottom() - 8.0
            }
        }
        // Hazards / pogo orbs / rebound surfaces are not collision
        // blockers for the kinematic sweep — gameplay layers handle
        // them as triggers (damage / bounce / impulse).
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core::Block;

    fn world_with(blocks: Vec<Block>) -> World {
        World {
            name: "kinematic-test".into(),
            size: Vec2::new(800.0, 600.0),
            spawn: Vec2::new(0.0, 0.0),
            blocks,
            water_regions: Vec::new(),
            climbable_regions: Vec::new(),
        }
    }

    fn body(pos: Vec2) -> KinematicBody {
        KinematicBody::new(pos, Vec2::new(28.0, 46.0))
    }

    fn tuning() -> KinematicTuning {
        KinematicTuning {
            gravity: 1450.0,
            max_fall_speed: 760.0,
            gravity_dir: Vec2::new(0.0, 1.0),
        }
    }

    #[test]
    fn flipped_gravity_makes_a_body_rise_and_land_on_a_ceiling() {
        // A ceiling block above the body; flipped gravity pulls the body UP onto
        // it and it registers as grounded (standing on the ceiling).
        let world = world_with(vec![Block::solid(
            "ceiling",
            Vec2::new(0.0, 0.0),
            Vec2::new(200.0, 32.0),
        )]);
        let mut b = body(Vec2::new(50.0, 300.0));
        let mut tuning = tuning();
        tuning.gravity_dir = Vec2::new(0.0, -1.0); // up
        for _ in 0..120 {
            step_kinematic(
                &mut b,
                &world,
                tuning,
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(
            b.pos.y < 300.0,
            "flipped gravity should pull the body up, got y={}",
            b.pos.y
        );
        assert!(
            b.on_ground,
            "the body should stand on the ceiling under flipped gravity"
        );
    }

    #[test]
    fn sideways_gravity_makes_a_body_fall_into_and_land_on_a_wall() {
        // A wall on the RIGHT; right-pointing gravity pulls the body into it and
        // it registers as grounded (standing on the wall). This is the enemy/NPC
        // bug — under the old Y-only `gravity_sign` a sideways-gravity body never
        // fell toward the wall at all.
        let world = world_with(vec![Block::solid(
            "right_wall",
            Vec2::new(400.0, -400.0),
            Vec2::new(40.0, 1200.0),
        )]);
        let mut b = body(Vec2::new(100.0, 50.0));
        let mut tuning = tuning();
        tuning.gravity_dir = Vec2::new(1.0, 0.0); // right
        let start_x = b.pos.x;
        for _ in 0..180 {
            step_kinematic(
                &mut b,
                &world,
                tuning,
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(
            b.pos.x > start_x + 100.0,
            "right gravity should pull the body toward the wall, got x={} (start {start_x})",
            b.pos.x
        );
        assert!(
            b.on_ground,
            "the body should land on (be grounded against) the wall it fell into",
        );
        assert!(
            b.pos.x <= 400.0,
            "the body should stop at the wall's left face, got x={}",
            b.pos.x
        );
    }

    #[test]
    fn gravity_caps_a_normal_fall_at_terminal_velocity() {
        // No floor: a body falling under gravity should accelerate UP TO the
        // terminal velocity and sit there (the equilibrium), never exceeding it.
        let world = world_with(vec![]);
        let mut b = body(Vec2::new(50.0, 0.0));
        for _ in 0..600 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(
            (b.vel.y - tuning().max_fall_speed).abs() < 1.0,
            "a normal fall should settle at terminal velocity {}, got {}",
            tuning().max_fall_speed,
            b.vel.y
        );
    }

    #[test]
    fn a_fling_above_terminal_is_preserved_not_braked() {
        // A body already moving faster than terminal (a portal fling) must NOT be
        // decelerated by the fall cap — gravity is an equilibrium it accelerates
        // toward, not a brake. The over-cap speed persists (no air drag on the
        // fall axis), so momentum carries through.
        let world = world_with(vec![]);
        let mut b = body(Vec2::new(50.0, 0.0));
        let fling = tuning().max_fall_speed * 2.0;
        b.vel.y = fling;
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
        assert!(
            b.vel.y >= fling,
            "an over-terminal fling ({fling}) should be preserved, got {}",
            b.vel.y
        );
    }

    #[test]
    fn lands_on_solid() {
        // Body falls and stops on a Solid floor.
        let world = world_with(vec![Block::solid(
            "floor",
            Vec2::new(0.0, 100.0),
            Vec2::new(200.0, 32.0),
        )]);
        let mut b = body(Vec2::new(50.0, 0.0));
        for _ in 0..30 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(b.on_ground, "expected to land on solid floor");
        assert!(b.vel.y.abs() < 0.01, "vel.y reset on landing");
    }

    #[test]
    fn lands_on_one_way_from_above() {
        // OneWay platform behaves like a floor when the body is
        // descending from above.
        let world = world_with(vec![Block::one_way(
            "platform",
            Vec2::new(0.0, 100.0),
            Vec2::new(200.0, 16.0),
        )]);
        let mut b = body(Vec2::new(50.0, 0.0));
        for _ in 0..30 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(
            b.on_ground,
            "expected to land on one-way platform from above"
        );
    }

    #[test]
    fn drop_through_passes_one_way() {
        // Same scene, but drop_through=true → no landing.
        let world = world_with(vec![Block::one_way(
            "platform",
            Vec2::new(0.0, 100.0),
            Vec2::new(200.0, 16.0),
        )]);
        let mut b = body(Vec2::new(50.0, 50.0));
        // First, settle on the platform.
        for _ in 0..20 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(b.on_ground, "precondition: must be on the platform");
        // Now drop through. Past the platform's bottom (y=116 in
        // top-left coords) is the success condition; y=160ish after
        // 20 frames of free-fall is well clear.
        let drop = KinematicInputs { drop_through: true };
        for _ in 0..20 {
            step_kinematic(&mut b, &world, tuning(), drop, 1.0 / 60.0);
        }
        assert!(
            b.pos.y - b.size.y * 0.5 > 116.0,
            "drop_through should clear the platform's bottom edge; body top y={}",
            b.pos.y - b.size.y * 0.5
        );
        assert!(!b.on_ground, "should not be grounded mid-fall");
    }

    #[test]
    fn drop_through_does_not_pass_solid() {
        // Drop-through is a OneWay-only affordance — Solid still blocks.
        let world = world_with(vec![Block::solid(
            "floor",
            Vec2::new(0.0, 100.0),
            Vec2::new(200.0, 32.0),
        )]);
        let mut b = body(Vec2::new(50.0, 50.0));
        let drop = KinematicInputs { drop_through: true };
        for _ in 0..40 {
            step_kinematic(&mut b, &world, tuning(), drop, 1.0 / 60.0);
        }
        assert!(b.on_ground, "Solid must still catch the body");
    }

    #[test]
    fn walks_off_ledge_falls() {
        // Solid ledge that ends at x=100. Body starts on the ledge,
        // walks right past the edge — should fall once it's no longer
        // overlapping the ledge horizontally.
        let world = world_with(vec![Block::solid(
            "ledge",
            Vec2::new(0.0, 100.0),
            Vec2::new(100.0, 32.0),
        )]);
        let mut b = body(Vec2::new(60.0, 50.0));
        // Settle on the ledge.
        for _ in 0..20 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(b.on_ground, "precondition: on ledge");
        // Walk right past the edge.
        b.vel.x = 200.0;
        for _ in 0..30 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(
            b.pos.x > 110.0,
            "must clear the ledge horizontally; x={}",
            b.pos.x
        );
        assert!(!b.on_ground, "should be airborne after clearing the edge");
        assert!(b.vel.y > 0.0, "should be falling");
    }

    #[test]
    fn a_body_rides_a_horizontally_moving_platform() {
        // A solid floor carrying a rightward per-frame velocity. ANY body resting on
        // it is carried right by that velocity — emergent riding, no per-actor flag.
        let mut platform = Block::solid("platform", Vec2::new(0.0, 100.0), Vec2::new(400.0, 32.0));
        platform.velocity = Vec2::new(3.0, 0.0); // 3 px/frame to the right
        let world = world_with(vec![platform]);
        let mut b = body(Vec2::new(50.0, 50.0));
        for _ in 0..30 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(b.on_ground, "precondition: resting on the platform");
        let x_before = b.pos.x;
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
        assert!(
            (b.pos.x - (x_before + 3.0)).abs() < 1e-3,
            "body should ride +3px right with the platform, got dx={}",
            b.pos.x - x_before
        );
    }

    #[test]
    fn a_body_does_not_ride_static_geometry() {
        // velocity ZERO (the static default) → no carry. Standing on normal ground is
        // byte-identical to before riding existed.
        let world = world_with(vec![Block::solid(
            "floor",
            Vec2::new(0.0, 100.0),
            Vec2::new(200.0, 32.0),
        )]);
        let mut b = body(Vec2::new(50.0, 50.0));
        for _ in 0..30 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        let x_before = b.pos.x;
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
        assert!(
            (b.pos.x - x_before).abs() < 1e-3,
            "a body must NOT drift on static ground, got dx={}",
            b.pos.x - x_before
        );
    }

    #[test]
    fn rising_through_one_way_does_not_get_stuck() {
        // OneWay should never block upward motion. Body starts below
        // the platform with negative vel.y (jumping up).
        let world = world_with(vec![Block::one_way(
            "platform",
            Vec2::new(0.0, 50.0),
            Vec2::new(200.0, 16.0),
        )]);
        let mut b = body(Vec2::new(50.0, 200.0));
        b.vel.y = -800.0;
        // Step a few frames; gravity will reduce vel.y but the body
        // should not be pinned by the one-way platform on the way up.
        let mut min_y = b.pos.y;
        for _ in 0..15 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
            if b.pos.y < min_y {
                min_y = b.pos.y;
            }
        }
        assert!(
            min_y < 60.0,
            "rising body should pass through OneWay; min_y={}",
            min_y
        );
    }
}
