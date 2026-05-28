//! Reusable actor taxonomy for Ambition rooms.
//!
//! This module is intentionally data-first. Patch 2 establishes the shared
//! vocabulary for enemies, bosses, NPCs, moving hazards, and other authored
//! entities before any one feature grows a bespoke sandbox-only system.

use crate::engine_core::Vec2;

/// Coarse category for room entities that have identity or behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActorKind {
    Player,
    Enemy,
    Boss,
    Npc,
    MovingPlatform,
    Hazard,
    Projectile,
    Pickup,
    Breakable,
    Debug,
}

/// Damage/team relationship used by hitboxes and hurtboxes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActorFaction {
    Player,
    Enemy,
    Neutral,
    Environment,
}

impl ActorFaction {
    /// True when damage from `self` is allowed to affect `target` by default.
    pub fn can_damage(self, target: Self) -> bool {
        match (self, target) {
            (Self::Player, Self::Enemy) => true,
            (Self::Enemy, Self::Player) => true,
            (Self::Environment, Self::Player | Self::Enemy | Self::Neutral) => true,
            (Self::Neutral, _) => false,
            _ => false,
        }
    }
}

/// Lightweight identity/name payload for authored actors.
#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub id: String,
    pub name: String,
    pub kind: ActorKind,
    pub faction: ActorFaction,
}

impl Actor {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: ActorKind,
        faction: ActorFaction,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            faction,
        }
    }
}

/// Generic hit-point component for enemies, bosses, breakables, and the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Health {
    pub current: i32,
    pub max: i32,
    pub invulnerable: bool,
}

impl Health {
    pub fn new(max: i32) -> Self {
        let max = max.max(1);
        Self {
            current: max,
            max,
            invulnerable: false,
        }
    }

    pub fn alive(self) -> bool {
        self.current > 0
    }

    pub fn ratio(self) -> f32 {
        if self.max <= 0 {
            0.0
        } else {
            (self.current.max(0) as f32 / self.max as f32).clamp(0.0, 1.0)
        }
    }

    /// Apply positive damage and return whether this call killed the entity.
    pub fn damage(&mut self, amount: i32) -> bool {
        if self.invulnerable || amount <= 0 || !self.alive() {
            return false;
        }
        self.current = (self.current - amount).max(0);
        self.current == 0
    }

    pub fn heal(&mut self, amount: i32) {
        if amount > 0 {
            self.current = (self.current + amount).min(self.max);
        }
    }

    pub fn reset(&mut self) {
        self.current = self.max;
        self.invulnerable = false;
    }
}

/// How temporary/destructible entities return after being consumed or killed.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum RespawnPolicy {
    /// Never respawn inside the current run/session.
    #[default]
    Never,
    /// Respawn after a timer in simulation seconds.
    AfterSeconds(f32),
    /// Respawn when the room is re-entered.
    OnRoomReload,
    /// The object is persistent and controlled by story/save state.
    Persistent,
}

/// Declarative movement path for moving platforms, spike balls, patrol dummies,
/// and later scripted boss hazards.
#[derive(Clone, Debug, PartialEq)]
pub struct KinematicPath {
    pub points: Vec<Vec2>,
    pub speed: f32,
    pub mode: KinematicPathMode,
    pub start_offset_seconds: f32,
}

impl KinematicPath {
    pub fn line(a: Vec2, b: Vec2, speed: f32) -> Self {
        Self {
            points: vec![a, b],
            speed,
            mode: KinematicPathMode::PingPong,
            start_offset_seconds: 0.0,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.points.len() >= 2 && self.speed > 0.0
    }
}

/// Playback style for a kinematic path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KinematicPathMode {
    Once,
    Loop,
    PingPong,
}

/// Authored enemy behavior tag. The sandbox maps `Custom(name)` to its
/// own `EnemyArchetype` via `EnemyArchetype::from_brain`; the engine
/// only carries this enum as a typed payload between LDtk authoring
/// and sandbox dispatch.
#[derive(Clone, Debug, PartialEq)]
pub enum EnemyBrain {
    Passive,
    Patrol { path_id: Option<String> },
    Guard { leash_radius: f32 },
    Custom(String),
}

/// Authored boss behavior tag. Same shape and contract as
/// `EnemyBrain`: the engine doesn't simulate against the variants;
/// the sandbox decides per-boss behavior from the payload.
#[derive(Clone, Debug, PartialEq)]
pub enum BossBrain {
    Dormant,
    PhaseScript { script_id: String },
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_reports_kill_once() {
        let mut health = Health::new(3);
        assert!(!health.damage(2));
        assert_eq!(health.current, 1);
        assert!(health.damage(1));
        assert!(!health.damage(1));
    }

    #[test]
    fn environment_damage_affects_player_and_enemy() {
        assert!(ActorFaction::Environment.can_damage(ActorFaction::Player));
        assert!(ActorFaction::Environment.can_damage(ActorFaction::Enemy));
        assert!(!ActorFaction::Player.can_damage(ActorFaction::Player));
    }

    #[test]
    fn health_invulnerable_drops_damage() {
        let mut health = Health::new(5);
        health.invulnerable = true;
        assert!(!health.damage(3));
        assert_eq!(health.current, 5);
        // Disabling invuln re-enables damage.
        health.invulnerable = false;
        assert!(!health.damage(3));
        assert_eq!(health.current, 2);
    }

    #[test]
    fn health_damage_zero_or_negative_is_no_op() {
        let mut health = Health::new(5);
        assert!(!health.damage(0));
        assert!(!health.damage(-3));
        assert_eq!(health.current, 5);
    }

    #[test]
    fn health_heal_clamps_to_max() {
        let mut health = Health::new(10);
        health.damage(7);
        assert_eq!(health.current, 3);
        health.heal(50); // tries to over-heal
        assert_eq!(health.current, 10);
    }

    #[test]
    fn health_heal_zero_or_negative_is_no_op() {
        let mut health = Health::new(10);
        health.damage(4);
        let before = health.current;
        health.heal(0);
        assert_eq!(health.current, before);
        health.heal(-5);
        assert_eq!(health.current, before);
    }

    #[test]
    fn health_ratio_within_envelope() {
        let mut health = Health::new(10);
        assert_eq!(health.ratio(), 1.0);
        health.damage(5);
        assert!((health.ratio() - 0.5).abs() < 1e-6);
        health.damage(50); // overkill
        assert_eq!(health.ratio(), 0.0);
    }

    #[test]
    fn health_reset_restores_max_and_clears_invuln() {
        let mut health = Health::new(8);
        health.damage(5);
        health.invulnerable = true;
        health.reset();
        assert_eq!(health.current, 8);
        assert!(!health.invulnerable);
    }

    #[test]
    fn health_alive_tracks_current() {
        let mut health = Health::new(2);
        assert!(health.alive());
        health.damage(1);
        assert!(health.alive());
        health.damage(1);
        assert!(!health.alive());
    }

    #[test]
    fn health_new_clamps_max_to_minimum_of_one() {
        // Negative or zero max becomes 1 so health.alive() is always
        // meaningful (a 0-max entity is degenerate).
        let h = Health::new(0);
        assert_eq!(h.max, 1);
        assert!(h.alive());

        let h = Health::new(-5);
        assert_eq!(h.max, 1);
    }
}
