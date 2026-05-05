//! Generic projectile primitives.
//!
//! `ProjectileSpec` defines a single projectile fired by the player
//! (or in principle an enemy). `ProjectileKind` distinguishes the
//! variants — today the sandbox uses `Fireball` (cheap, weaker) and
//! `Hadouken` (strong, costs more resource, stronger arc).
//!
//! `ProjectileSpawner` is a tiny stateless helper that converts a
//! "user pressed Projectile + facing right" intent into a
//! `ProjectileSpec` honoring a resource meter and a per-projectile
//! cooldown timer. Sandbox owns the per-frame physics tick because
//! the engine doesn't yet have a generic kinematic-body type.
//!
//! The motion-input recognizer (`MotionInputBuffer`) lives here too so
//! both keyboard and gamepad consumers can detect quarter-circle /
//! half-circle gestures before deciding which `ProjectileKind` to
//! fire.

use std::collections::VecDeque;

use bevy_math::Vec2;
use serde::{Deserialize, Serialize};

use crate::geometry::{aabb_from_min_size, Aabb};
use crate::player_state::ResourceMeter;

/// What kind of projectile to spawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectileKind {
    /// Cheap fireball. Mostly horizontal travel with mild arc.
    Fireball,
    /// Hadouken-style power projectile. Recognized after a
    /// quarter-circle (or half-circle) input motion plus the fire
    /// button. More damage, larger hitbox, larger resource cost.
    Hadouken,
}

impl ProjectileKind {
    /// Resource cost (mana / ammo / charge units) to fire one.
    pub fn cost(self) -> f32 {
        match self {
            Self::Fireball => 1.0,
            Self::Hadouken => 3.0,
        }
    }

    /// Damage dealt on hit.
    pub fn damage(self) -> i32 {
        match self {
            Self::Fireball => 1,
            Self::Hadouken => 3,
        }
    }

    /// Cooldown after firing, in seconds. The Hadouken cooldown is
    /// longer so the player can't bypass the cost by spamming.
    pub fn cooldown(self) -> f32 {
        match self {
            Self::Fireball => 0.30,
            Self::Hadouken => 0.55,
        }
    }

    /// Initial speed in pixels-per-second.
    pub fn speed(self) -> f32 {
        match self {
            Self::Fireball => 360.0,
            Self::Hadouken => 520.0,
        }
    }

    /// Maximum lifetime in seconds. A projectile that hasn't hit
    /// anything by this time despawns and emits `ProjectileExpired`.
    pub fn max_lifetime(self) -> f32 {
        match self {
            Self::Fireball => 1.20,
            Self::Hadouken => 1.60,
        }
    }

    /// Hitbox half-extent (pixels). Hadouken is chunkier.
    pub fn half_extent(self) -> Vec2 {
        match self {
            Self::Fireball => Vec2::new(8.0, 6.0),
            Self::Hadouken => Vec2::new(14.0, 10.0),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Fireball => "fireball",
            Self::Hadouken => "hadouken",
        }
    }
}

/// Authored intent for a single new projectile. Sandbox spawns an
/// entity carrying this spec plus its current pos / vel; `ProjectileBody`
/// (below) is the per-frame state it advances.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileSpec {
    pub kind: ProjectileKind,
    /// Initial center position.
    pub origin: Vec2,
    /// Unit-length direction vector. (1, 0) fires right.
    pub direction: Vec2,
    /// Damage to apply on hit.
    pub damage: i32,
    /// Initial speed in px/s.
    pub speed: f32,
    /// Maximum lifetime.
    pub max_lifetime: f32,
    /// Half-extent of the hitbox.
    pub half_extent: Vec2,
    /// Vertical acceleration applied each frame (px/s^2). Mario
    /// fireball-style arc: positive value pulls down (recall +Y is
    /// down in the sandbox simulation).
    pub gravity: f32,
}

impl ProjectileSpec {
    pub fn new(
        kind: ProjectileKind,
        origin: Vec2,
        direction: Vec2,
        damage_multiplier: f32,
    ) -> Self {
        Self {
            kind,
            origin,
            direction: direction.normalize_or(Vec2::new(1.0, 0.0)),
            damage: ((kind.damage() as f32) * damage_multiplier).round().max(1.0) as i32,
            speed: kind.speed(),
            max_lifetime: kind.max_lifetime(),
            half_extent: kind.half_extent(),
            gravity: match kind {
                ProjectileKind::Fireball => 360.0,
                ProjectileKind::Hadouken => 0.0,
            },
        }
    }

    pub fn initial_velocity(&self) -> Vec2 {
        self.direction * self.speed
    }
}

/// Per-frame physics state of an in-flight projectile. Sandbox owns
/// the world-collision check; this struct only owns position, velocity,
/// and lifetime.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileBody {
    pub kind: ProjectileKind,
    pub pos: Vec2,
    pub vel: Vec2,
    pub age: f32,
    pub max_lifetime: f32,
    pub gravity: f32,
    pub half_extent: Vec2,
    pub damage: i32,
    pub bounces_remaining: u8,
}

impl ProjectileBody {
    pub fn from_spec(spec: ProjectileSpec) -> Self {
        Self {
            kind: spec.kind,
            pos: spec.origin,
            vel: spec.initial_velocity(),
            age: 0.0,
            max_lifetime: spec.max_lifetime,
            gravity: spec.gravity,
            half_extent: spec.half_extent,
            damage: spec.damage,
            // Mario fireballs bounce off the floor a couple of times.
            bounces_remaining: matches!(spec.kind, ProjectileKind::Fireball) as u8 * 2,
        }
    }

    pub fn aabb(&self) -> Aabb {
        aabb_from_min_size(
            Vec2::new(self.pos.x - self.half_extent.x, self.pos.y - self.half_extent.y),
            Vec2::new(self.half_extent.x * 2.0, self.half_extent.y * 2.0),
        )
    }

    /// Step the projectile forward by `dt`. Returns `true` if the
    /// projectile is still alive after the tick. Collision against
    /// solids / breakables is the caller's responsibility (sandbox).
    pub fn tick(&mut self, dt: f32) -> bool {
        self.age += dt;
        if self.age >= self.max_lifetime {
            return false;
        }
        // Apply gravity (positive = downward in sandbox conventions).
        self.vel.y += self.gravity * dt;
        self.pos += self.vel * dt;
        true
    }

    pub fn is_expired(&self) -> bool {
        self.age >= self.max_lifetime
    }
}

/// Snapshot of a single recorded directional sample, captured by
/// `MotionInputBuffer` for motion-input recognition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionSample {
    /// Discrete direction; quantized to one of the 8 cardinals so
    /// recognition is robust against noisy analog input.
    pub dir: MotionDirection,
    /// Time when this sample was recorded, in arbitrary monotonic
    /// seconds. The buffer prunes samples older than its window.
    pub time: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MotionDirection {
    Neutral,
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl MotionDirection {
    /// Quantize an axis vector to a cardinal direction. `threshold`
    /// is the magnitude below which the direction is `Neutral`.
    pub fn from_axis(x: f32, y: f32, threshold: f32) -> Self {
        let mag = (x * x + y * y).sqrt();
        if mag < threshold {
            return Self::Neutral;
        }
        let xs = x.abs() > threshold * 0.5;
        let ys = y.abs() > threshold * 0.5;
        match (xs, ys, x.signum(), y.signum()) {
            (true, true, sx, sy) if sx > 0.0 && sy < 0.0 => Self::UpRight,
            (true, true, sx, sy) if sx < 0.0 && sy < 0.0 => Self::UpLeft,
            (true, true, sx, sy) if sx > 0.0 && sy > 0.0 => Self::DownRight,
            (true, true, sx, sy) if sx < 0.0 && sy > 0.0 => Self::DownLeft,
            (true, _, sx, _) if sx > 0.0 => Self::Right,
            (true, _, _, _) => Self::Left,
            (_, _, _, sy) if sy < 0.0 => Self::Up,
            _ => Self::Down,
        }
    }
}

/// Rolling buffer of recent directional samples. Used by motion-input
/// recognizers to test for quarter-circle / half-circle gestures.
///
/// Records samples even when the direction is `Neutral`; that lets the
/// recognizer require a Neutral pause between distinct gestures so a
/// constant Right hold is not interpreted as repeated half-circles.
#[derive(Clone, Debug)]
pub struct MotionInputBuffer {
    pub samples: VecDeque<MotionSample>,
    /// Maximum age in seconds for samples to be retained.
    pub window: f32,
}

impl MotionInputBuffer {
    pub fn new(window: f32) -> Self {
        Self {
            samples: VecDeque::with_capacity(64),
            window,
        }
    }

    /// Record one sample at `now`. Prunes anything older than
    /// `now - window`. Collapses repeats so a held direction does
    /// not flood the buffer.
    pub fn push(&mut self, dir: MotionDirection, now: f32) {
        match self.samples.back() {
            Some(prev) if prev.dir == dir => {
                // Same direction continues — update only the time of
                // the most recent occurrence so the window math sees
                // a fresh sample.
                let last = self.samples.back_mut().unwrap();
                last.time = now;
            }
            _ => {
                self.samples.push_back(MotionSample { dir, time: now });
            }
        }
        let cutoff = now - self.window;
        while let Some(front) = self.samples.front() {
            if front.time < cutoff {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    /// Iterator over recent (oldest-first) directions, ignoring time.
    pub fn directions(&self) -> impl Iterator<Item = MotionDirection> + '_ {
        self.samples.iter().map(|s| s.dir)
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    /// Recognize a `Down → DownRight → Right` quarter-circle (or its
    /// mirror image) finishing recently. Returns `Some(facing)` where
    /// facing is +1 (right) or -1 (left) to match the player's
    /// `facing` field.
    ///
    /// We don't require strict adjacency; intermediate Neutral or
    /// extra cardinal samples are tolerated as long as the three key
    /// directions appear in order within the buffer window.
    pub fn detect_quarter_circle(&self) -> Option<f32> {
        if let Some(facing) = self.detect_sequence(&[
            MotionDirection::Down,
            MotionDirection::DownRight,
            MotionDirection::Right,
        ]) {
            return Some(facing);
        }
        if let Some(facing) = self.detect_sequence(&[
            MotionDirection::Down,
            MotionDirection::DownLeft,
            MotionDirection::Left,
        ]) {
            return Some(facing);
        }
        None
    }

    /// Recognize a half-circle: `Right → DownRight → Down → DownLeft → Left`
    /// (or mirror). Treated as a stronger gesture than the quarter
    /// circle and used in the sandbox to upgrade `Fireball` to
    /// `Hadouken`. The mirror form returns `-1.0`.
    pub fn detect_half_circle(&self) -> Option<f32> {
        if let Some(facing) = self.detect_sequence(&[
            MotionDirection::Right,
            MotionDirection::DownRight,
            MotionDirection::Down,
            MotionDirection::DownLeft,
            MotionDirection::Left,
        ]) {
            return Some(-facing);
        }
        if let Some(facing) = self.detect_sequence(&[
            MotionDirection::Left,
            MotionDirection::DownLeft,
            MotionDirection::Down,
            MotionDirection::DownRight,
            MotionDirection::Right,
        ]) {
            return Some(-facing);
        }
        None
    }

    /// Detect an ordered subsequence in the recent samples. Returns
    /// `Some(facing)` based on the final direction (`+1.0` for right,
    /// `-1.0` for left, `+1.0` for up/down ambiguity).
    fn detect_sequence(&self, expected: &[MotionDirection]) -> Option<f32> {
        if expected.is_empty() {
            return None;
        }
        let mut idx = 0;
        for sample in self.samples.iter() {
            if sample.dir == expected[idx] {
                idx += 1;
                if idx == expected.len() {
                    let last = expected[expected.len() - 1];
                    return Some(match last {
                        MotionDirection::Right
                        | MotionDirection::UpRight
                        | MotionDirection::DownRight => 1.0,
                        MotionDirection::Left
                        | MotionDirection::UpLeft
                        | MotionDirection::DownLeft => -1.0,
                        _ => 1.0,
                    });
                }
            }
        }
        None
    }
}

/// Spawner state. Owns the per-projectile cooldown timer and a
/// `ResourceMeter` that mechanics can refill from rooms / pickups.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileSpawner {
    pub meter: ResourceMeter,
    pub cooldown_remaining: f32,
}

impl ProjectileSpawner {
    pub fn new(max_resource: f32, regen_rate: f32) -> Self {
        Self {
            meter: ResourceMeter::new(max_resource, regen_rate, 0.0),
            cooldown_remaining: 0.0,
        }
    }

    /// Tick down the cooldown timer and regen the resource meter.
    pub fn tick(&mut self, dt: f32) {
        self.cooldown_remaining = (self.cooldown_remaining - dt).max(0.0);
        self.meter.tick_regen(dt);
    }

    /// Try to fire a projectile of the given kind from `origin`
    /// pointing in `direction`. Returns the `ProjectileSpec` to
    /// spawn on success. Failure modes:
    ///
    /// - `cooldown_remaining > 0.0` → `Err(SpawnFailure::Cooldown)`
    /// - resource meter doesn't have enough for `kind.cost()` →
    ///   `Err(SpawnFailure::OutOfResource)`
    pub fn try_spawn(
        &mut self,
        kind: ProjectileKind,
        origin: Vec2,
        direction: Vec2,
        outgoing_damage_multiplier: f32,
    ) -> Result<ProjectileSpec, SpawnFailure> {
        if self.cooldown_remaining > 0.0 {
            return Err(SpawnFailure::Cooldown);
        }
        if !self.meter.try_spend(kind.cost()) {
            return Err(SpawnFailure::OutOfResource);
        }
        self.cooldown_remaining = kind.cooldown();
        Ok(ProjectileSpec::new(
            kind,
            origin,
            direction,
            outgoing_damage_multiplier,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnFailure {
    Cooldown,
    OutOfResource,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_buffer_recognizes_quarter_circle_right() {
        let mut buf = MotionInputBuffer::new(0.5);
        let mut t = 0.0;
        for dir in [
            MotionDirection::Down,
            MotionDirection::DownRight,
            MotionDirection::Right,
        ] {
            buf.push(dir, t);
            t += 0.05;
        }
        assert_eq!(buf.detect_quarter_circle(), Some(1.0));
    }

    #[test]
    fn motion_buffer_recognizes_quarter_circle_left() {
        let mut buf = MotionInputBuffer::new(0.5);
        let mut t = 0.0;
        for dir in [
            MotionDirection::Down,
            MotionDirection::DownLeft,
            MotionDirection::Left,
        ] {
            buf.push(dir, t);
            t += 0.05;
        }
        assert_eq!(buf.detect_quarter_circle(), Some(-1.0));
    }

    #[test]
    fn motion_buffer_recognizes_half_circle() {
        let mut buf = MotionInputBuffer::new(0.6);
        let mut t = 0.0;
        for dir in [
            MotionDirection::Right,
            MotionDirection::DownRight,
            MotionDirection::Down,
            MotionDirection::DownLeft,
            MotionDirection::Left,
        ] {
            buf.push(dir, t);
            t += 0.04;
        }
        // Half circle right-to-left: facing of the player should be left.
        assert_eq!(buf.detect_half_circle(), Some(1.0));
    }

    #[test]
    fn quarter_circle_tolerates_extra_samples() {
        let mut buf = MotionInputBuffer::new(1.0);
        let mut t = 0.0;
        for dir in [
            MotionDirection::Neutral,
            MotionDirection::Down,
            MotionDirection::DownRight,
            MotionDirection::Up, // noise
            MotionDirection::DownRight,
            MotionDirection::Right,
        ] {
            buf.push(dir, t);
            t += 0.04;
        }
        assert_eq!(buf.detect_quarter_circle(), Some(1.0));
    }

    #[test]
    fn motion_buffer_window_prunes_old_samples() {
        let mut buf = MotionInputBuffer::new(0.20);
        buf.push(MotionDirection::Down, 0.0);
        buf.push(MotionDirection::DownRight, 0.05);
        // Push something far in the future — old samples should be pruned.
        buf.push(MotionDirection::Right, 1.0);
        // Quarter circle should NOT detect because the older two
        // samples were dropped.
        assert_eq!(buf.detect_quarter_circle(), None);
    }

    #[test]
    fn projectile_spawner_blocks_when_on_cooldown() {
        let mut spawner = ProjectileSpawner::new(10.0, 0.0);
        let _ = spawner
            .try_spawn(
                ProjectileKind::Fireball,
                Vec2::ZERO,
                Vec2::new(1.0, 0.0),
                1.0,
            )
            .unwrap();
        let err = spawner
            .try_spawn(
                ProjectileKind::Fireball,
                Vec2::ZERO,
                Vec2::new(1.0, 0.0),
                1.0,
            )
            .unwrap_err();
        assert_eq!(err, SpawnFailure::Cooldown);
    }

    #[test]
    fn projectile_spawner_blocks_when_out_of_resource() {
        let mut spawner = ProjectileSpawner::new(0.5, 0.0);
        let err = spawner
            .try_spawn(
                ProjectileKind::Fireball,
                Vec2::ZERO,
                Vec2::new(1.0, 0.0),
                1.0,
            )
            .unwrap_err();
        assert_eq!(err, SpawnFailure::OutOfResource);
    }

    #[test]
    fn projectile_body_expires_after_max_lifetime() {
        let spec = ProjectileSpec::new(
            ProjectileKind::Fireball,
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ProjectileBody::from_spec(spec);
        let mut alive = true;
        for _ in 0..200 {
            alive = body.tick(0.016);
            if !alive {
                break;
            }
        }
        assert!(!alive);
        assert!(body.is_expired());
    }

    #[test]
    fn fireball_arcs_downward() {
        let spec = ProjectileSpec::new(
            ProjectileKind::Fireball,
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ProjectileBody::from_spec(spec);
        for _ in 0..30 {
            body.tick(0.016);
        }
        assert!(body.pos.y > 0.0, "fireball should arc downward, got {}", body.pos.y);
        assert!(body.pos.x > 0.0);
    }

    #[test]
    fn hadouken_travels_straight_horizontally() {
        let spec = ProjectileSpec::new(
            ProjectileKind::Hadouken,
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ProjectileBody::from_spec(spec);
        for _ in 0..30 {
            body.tick(0.016);
        }
        assert!(body.pos.y.abs() < 1e-3);
        assert!(body.pos.x > 0.0);
    }

    #[test]
    fn motion_direction_quantization() {
        assert_eq!(
            MotionDirection::from_axis(0.05, 0.05, 0.2),
            MotionDirection::Neutral
        );
        assert_eq!(
            MotionDirection::from_axis(0.8, 0.0, 0.2),
            MotionDirection::Right
        );
        assert_eq!(
            MotionDirection::from_axis(0.6, 0.6, 0.2),
            MotionDirection::DownRight
        );
        assert_eq!(
            MotionDirection::from_axis(-0.7, 0.7, 0.2),
            MotionDirection::DownLeft
        );
    }

    #[test]
    fn outgoing_damage_multiplier_scales_damage() {
        let spec = ProjectileSpec::new(
            ProjectileKind::Hadouken,
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            2.0,
        );
        // Hadouken default is 3 damage; 2x = 6.
        assert_eq!(spec.damage, 6);
    }
}
