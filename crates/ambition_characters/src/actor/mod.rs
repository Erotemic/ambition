//! Reusable, content-free actor vocabulary: identity + the control contract.
//!
//! Data-first shared vocabulary for enemies, bosses, NPCs, moving hazards,
//! and other authored entities. Owns [`ActorKind`]/[`DamageTeam`] identity,
//! the [`control`] `ActorControl`/`ActorControlFrame` contract that brains
//! write and simulation consumes, the [`ai`] intent layer
//! (`CharacterAiIntent`), [`pose`] (`ActorPose`/`ActorFaction`), and the
//! [`character_catalog`] cast data.

pub mod pose;
pub use pose::{ActorFaction, ActorPose};
pub mod ai;
pub mod body;
pub use body::{BodyCombat, BodyHealth, BodyWallet};
pub mod character_catalog;
pub mod control;

use ambition_entity_catalog::placements::DamageTeam;

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

/// Lightweight identity/name payload for authored actors.
#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub id: String,
    pub name: String,
    pub kind: ActorKind,
    pub faction: DamageTeam,
}

impl Actor {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: ActorKind,
        faction: DamageTeam,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
