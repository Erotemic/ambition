//! Combat helpers and reusable damage volumes.
//!
//! Bevy should render slash previews, play hit sounds, and spawn particles, but
//! the shape of an attack is game logic. Keeping hitbox and damage semantics
//! here lets tests and future headless validators reason about combat without a
//! renderer.

use crate::actor::{ActorFaction, KinematicPath, RespawnPolicy};
use crate::geometry::{Aabb, AabbExt};
use crate::movement::Player;
use crate::Vec2;

/// The broad gameplay category of damage. This is intentionally separate from
/// presentation so hazards, attacks, and projectiles can share damage handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DamageKind {
    Slash,
    Pogo,
    Contact,
    Hazard,
    Projectile,
    Environmental,
    Custom,
}

/// Damage payload shared by hitboxes and persistent damage volumes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Damage {
    pub amount: i32,
    pub knockback: Vec2,
    pub kind: DamageKind,
    pub source: ActorFaction,
    pub hitstop_seconds: f32,
}

impl Damage {
    pub fn new(amount: i32, kind: DamageKind, source: ActorFaction) -> Self {
        Self {
            amount,
            kind,
            source,
            knockback: Vec2::ZERO,
            hitstop_seconds: 0.0,
        }
    }

    pub fn with_knockback(mut self, knockback: Vec2) -> Self {
        self.knockback = knockback;
        self
    }

    pub fn can_affect(self, target: ActorFaction) -> bool {
        self.source.can_damage(target)
    }
}

/// Short-lived attack rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct Hitbox {
    pub id: String,
    pub aabb: Aabb,
    pub damage: Damage,
    pub active_seconds: f32,
    pub one_hit_per_target: bool,
}

impl Hitbox {
    pub fn new(id: impl Into<String>, aabb: Aabb, damage: Damage) -> Self {
        Self {
            id: id.into(),
            aabb,
            damage,
            active_seconds: 0.08,
            one_hit_per_target: true,
        }
    }
}

/// Damageable body area.
#[derive(Clone, Debug, PartialEq)]
pub struct Hurtbox {
    pub id: String,
    pub aabb: Aabb,
    pub faction: ActorFaction,
    pub enabled: bool,
}

impl Hurtbox {
    pub fn new(id: impl Into<String>, aabb: Aabb, faction: ActorFaction) -> Self {
        Self {
            id: id.into(),
            aabb,
            faction,
            enabled: true,
        }
    }

    pub fn accepts(&self, damage: Damage) -> bool {
        self.enabled && damage.can_affect(self.faction)
    }
}

/// Persistent or reusable damaging area: spikes, lasers, spike balls, boss
/// patterns, enemy contact damage, and similar hazards.
#[derive(Clone, Debug, PartialEq)]
pub struct DamageVolume {
    pub id: String,
    pub aabb: Aabb,
    pub damage: Damage,
    pub respawn: RespawnPolicy,
    pub motion: Option<KinematicPath>,
    pub enabled: bool,
}

impl DamageVolume {
    pub fn new(id: impl Into<String>, aabb: Aabb, amount: i32) -> Self {
        Self {
            id: id.into(),
            aabb,
            damage: Damage::new(amount, DamageKind::Hazard, ActorFaction::Environment),
            respawn: RespawnPolicy::Never,
            motion: None,
            enabled: true,
        }
    }

    pub fn overlaps_hurtbox(&self, hurtbox: &Hurtbox) -> bool {
        self.enabled && hurtbox.accepts(self.damage) && self.aabb.strict_intersects(hurtbox.aabb)
    }
}

/// Compute the current slash/pogo hitbox for a player.
///
/// `axis_y` follows `InputState`: negative means up, positive means down.
/// `forced_pogo` is used by layouts that expose downward slash/pogo as a
/// dedicated face-button verb rather than requiring down + attack.
pub fn slash_hitbox(player: &Player, axis_y: f32, forced_pogo: bool) -> Aabb {
    let body = player.aabb();
    if forced_pogo || axis_y > 0.25 {
        Aabb::new(
            Vec2::new(body.center().x, body.bottom() + 24.0),
            Vec2::new(body.half_size().x * 0.95, 26.0),
        )
    } else if axis_y < -0.25 {
        Aabb::new(
            Vec2::new(body.center().x, body.top() - 22.0),
            Vec2::new(body.half_size().x * 1.10, 24.0),
        )
    } else {
        Aabb::new(
            Vec2::new(
                body.center().x + player.facing * (body.half_size().x + 30.0),
                body.center().y - 2.0,
            ),
            Vec2::new(34.0, 24.0),
        )
    }
}

/// Build a structured player slash hitbox from the legacy slash shape helper.
pub fn player_slash_hitbox(
    player: &Player,
    axis_y: f32,
    forced_pogo: bool,
    damage_amount: i32,
) -> Hitbox {
    let kind = if forced_pogo || axis_y > 0.25 {
        DamageKind::Pogo
    } else {
        DamageKind::Slash
    };
    Hitbox::new(
        "player_slash",
        slash_hitbox(player, axis_y, forced_pogo),
        Damage::new(damage_amount, kind, ActorFaction::Player),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vec2;

    #[test]
    fn forward_slash_is_in_front_of_facing_direction() {
        let mut player = Player::new(Vec2::new(100.0, 100.0));
        player.facing = 1.0;
        let right = slash_hitbox(&player, 0.0, false);
        player.facing = -1.0;
        let left = slash_hitbox(&player, 0.0, false);
        assert!(right.center().x > player.pos.x);
        assert!(left.center().x < player.pos.x);
    }

    #[test]
    fn hazard_volume_overlaps_player_hurtbox() {
        let hazard = DamageVolume::new(
            "spike",
            Aabb::new(Vec2::new(20.0, 20.0), Vec2::new(10.0, 10.0)),
            1,
        );
        let hurtbox = Hurtbox::new(
            "player",
            Aabb::new(Vec2::new(25.0, 20.0), Vec2::new(10.0, 10.0)),
            ActorFaction::Player,
        );
        assert!(hazard.overlaps_hurtbox(&hurtbox));
    }

    #[test]
    fn damage_with_knockback_chains_builder() {
        let damage = Damage::new(2, DamageKind::Slash, ActorFaction::Player)
            .with_knockback(Vec2::new(100.0, -50.0));
        assert_eq!(damage.amount, 2);
        assert_eq!(damage.knockback, Vec2::new(100.0, -50.0));
    }

    #[test]
    fn damage_can_affect_respects_faction() {
        let player_dmg = Damage::new(1, DamageKind::Slash, ActorFaction::Player);
        // Player damage affects enemies but not other players.
        assert!(player_dmg.can_affect(ActorFaction::Enemy));
        assert!(!player_dmg.can_affect(ActorFaction::Player));
        // Environment damage affects player + enemy.
        let env_dmg = Damage::new(1, DamageKind::Hazard, ActorFaction::Environment);
        assert!(env_dmg.can_affect(ActorFaction::Player));
        assert!(env_dmg.can_affect(ActorFaction::Enemy));
    }

    #[test]
    fn hurtbox_accepts_only_compatible_damage() {
        let hurtbox = Hurtbox::new(
            "player",
            Aabb::new(Vec2::ZERO, Vec2::new(10.0, 10.0)),
            ActorFaction::Player,
        );
        // Player hurtbox accepts enemy / environment damage.
        let enemy_dmg = Damage::new(1, DamageKind::Slash, ActorFaction::Enemy);
        assert!(hurtbox.accepts(enemy_dmg));
        // Player hurtbox rejects player damage (no friendly fire).
        let self_dmg = Damage::new(1, DamageKind::Slash, ActorFaction::Player);
        assert!(!hurtbox.accepts(self_dmg));
    }

    #[test]
    fn forced_pogo_slash_is_below_player() {
        let player = Player::new(Vec2::new(100.0, 100.0));
        let pogo = slash_hitbox(&player, 0.0, true);
        // Pogo slash sits below the body — its center.y is greater
        // (Ambition uses +Y down) than the player's y.
        assert!(pogo.center().y > player.pos.y);
    }

    #[test]
    fn upward_slash_is_above_player() {
        let player = Player::new(Vec2::new(100.0, 100.0));
        let up = slash_hitbox(&player, -1.0, false);
        assert!(up.center().y < player.pos.y);
    }
}
