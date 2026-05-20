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
    /// Optional reference to a room-level `KinematicPath` authored in LDtk.
    /// When present, sandbox feature runtime resolves this against
    /// `RoomSpec::kinematic_paths` and uses it instead of the inline
    /// `motion` path below.
    pub path_id: Option<String>,
    /// Legacy inline path authored directly on the damage volume. Kept for
    /// compatibility with existing rooms/specs; new authored hazards should
    /// prefer `path_id` so platforms, NPCs, enemies, and hazards can share
    /// one path definition.
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
            path_id: None,
            motion: None,
            enabled: true,
        }
    }

    pub fn overlaps_hurtbox(&self, hurtbox: &Hurtbox) -> bool {
        self.enabled && hurtbox.accepts(self.damage) && self.aabb.strict_intersects(hurtbox.aabb)
    }
}

/// Player melee intent resolved from input + movement state.
///
/// This is deliberately coarser than animation rows: several intents can share
/// art while still carrying different hitboxes, timing, and movement modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttackIntent {
    Neutral,
    Forward,
    Back,
    Up,
    Down,
    DashForward,
    AirForward,
    AirBack,
    AirUp,
    AirDown,
    WallOut,
}

impl AttackIntent {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Forward => "forward",
            Self::Back => "back",
            Self::Up => "up",
            Self::Down => "down",
            Self::DashForward => "dash_forward",
            Self::AirForward => "air_forward",
            Self::AirBack => "air_back",
            Self::AirUp => "air_up",
            Self::AirDown => "air_down",
            Self::WallOut => "wall_out",
        }
    }
}

/// Coarse lifecycle of a single melee swing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttackPhase {
    Startup,
    Active,
    Recovery,
}

impl AttackPhase {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Active => "active",
            Self::Recovery => "recovery",
        }
    }
}

/// Resolved melee swing parameters. Offsets are in world units and are already
/// signed for the player's current facing direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackSpec {
    pub intent: AttackIntent,
    pub startup_seconds: f32,
    pub active_seconds: f32,
    pub recovery_seconds: f32,
    pub hitbox_offset: Vec2,
    pub hitbox_half_size: Vec2,
    pub self_impulse: Vec2,
    pub knockback: Vec2,
    pub damage_kind: DamageKind,
    pub can_pogo: bool,
}

impl AttackSpec {
    pub fn total_seconds(self) -> f32 {
        self.startup_seconds + self.active_seconds + self.recovery_seconds
    }

    pub fn phase_at(self, elapsed: f32) -> Option<AttackPhase> {
        if elapsed < self.startup_seconds {
            Some(AttackPhase::Startup)
        } else if elapsed < self.startup_seconds + self.active_seconds {
            Some(AttackPhase::Active)
        } else if elapsed < self.total_seconds() {
            Some(AttackPhase::Recovery)
        } else {
            None
        }
    }
}

/// Resolve a directional attack intent from input and current player state.
///
/// `axis_y` follows `InputState`: negative means up, positive means down.
/// `forced_pogo` is used by layouts that expose downward slash/pogo as a
/// dedicated face-button verb rather than requiring down + attack.
pub fn resolve_attack_intent(
    player: &Player,
    axis_x: f32,
    axis_y: f32,
    forced_pogo: bool,
) -> AttackIntent {
    if forced_pogo || axis_y > 0.25 {
        return if player.on_ground {
            AttackIntent::Down
        } else {
            AttackIntent::AirDown
        };
    }
    if axis_y < -0.25 {
        return if player.on_ground {
            AttackIntent::Up
        } else {
            AttackIntent::AirUp
        };
    }

    let forward = axis_x * player.facing > 0.25;
    let back = axis_x * player.facing < -0.25;
    if player.wall_clinging && back {
        return AttackIntent::WallOut;
    }
    if player.dash_timer > 0.0 && !back {
        return AttackIntent::DashForward;
    }
    // Airborne: facing is locked while airborne (see
    // `update_facing_and_control_intent`), so a back-input here means
    // "swing behind me without turning" — a dedicated AirBack rather
    // than reusing the grounded Back spec.
    if !player.on_ground {
        if back && player.abilities.directional_primary {
            return AttackIntent::AirBack;
        }
        return AttackIntent::AirForward;
    }
    if back && player.abilities.directional_primary {
        return AttackIntent::Back;
    }
    if forward {
        AttackIntent::Forward
    } else {
        AttackIntent::Neutral
    }
}

/// Build the attack spec for an already-resolved intent.
pub fn attack_spec(player: &Player, intent: AttackIntent) -> AttackSpec {
    let body = player.aabb();
    let facing = if player.facing < 0.0 { -1.0 } else { 1.0 };
    let half = body.half_size();

    match intent {
        AttackIntent::Up | AttackIntent::AirUp => AttackSpec {
            intent,
            startup_seconds: 0.035,
            active_seconds: 0.105,
            recovery_seconds: 0.150,
            hitbox_offset: Vec2::new(0.0, -half.y - 32.0),
            hitbox_half_size: Vec2::new(26.0, 34.0),
            self_impulse: Vec2::new(0.0, -35.0),
            knockback: Vec2::new(0.0, -300.0),
            damage_kind: DamageKind::Slash,
            can_pogo: false,
        },
        AttackIntent::Down => AttackSpec {
            // Grounded down tilt — kneeling forward poke (Marth/Lucina
            // down-tilt). Low, forward-reaching swipe rather than a
            // downward slam: shouldn't trigger pogo and shouldn't punch
            // a hitbox into the ground beneath the player.
            intent,
            startup_seconds: 0.035,
            active_seconds: 0.090,
            recovery_seconds: 0.180,
            hitbox_offset: Vec2::new(facing * (half.x + 30.0), half.y - 6.0),
            hitbox_half_size: Vec2::new(30.0, 12.0),
            self_impulse: Vec2::new(0.0, 0.0),
            knockback: Vec2::new(facing * 220.0, -80.0),
            damage_kind: DamageKind::Slash,
            can_pogo: false,
        },
        AttackIntent::AirDown => AttackSpec {
            // Aerial down — straight-down spike that drives the player
            // back into the air on contact.
            intent,
            startup_seconds: 0.035,
            active_seconds: 0.110,
            recovery_seconds: 0.165,
            hitbox_offset: Vec2::new(0.0, half.y + 32.0),
            hitbox_half_size: Vec2::new(26.0, 34.0),
            self_impulse: Vec2::new(0.0, 35.0),
            knockback: Vec2::new(0.0, 260.0),
            damage_kind: DamageKind::Pogo,
            can_pogo: true,
        },
        AttackIntent::Back | AttackIntent::WallOut => AttackSpec {
            intent,
            startup_seconds: 0.040,
            active_seconds: 0.090,
            recovery_seconds: 0.170,
            hitbox_offset: Vec2::new(-facing * (half.x + 28.0), -2.0),
            hitbox_half_size: Vec2::new(28.0, 24.0),
            self_impulse: Vec2::new(facing * 120.0, -20.0),
            knockback: Vec2::new(-facing * 280.0, -120.0),
            damage_kind: DamageKind::Slash,
            can_pogo: false,
        },
        AttackIntent::DashForward => AttackSpec {
            intent,
            startup_seconds: 0.020,
            active_seconds: 0.095,
            recovery_seconds: 0.185,
            hitbox_offset: Vec2::new(facing * (half.x + 46.0), -2.0),
            hitbox_half_size: Vec2::new(46.0, 24.0),
            self_impulse: Vec2::new(facing * 55.0, 0.0),
            knockback: Vec2::new(facing * 390.0, -120.0),
            damage_kind: DamageKind::Slash,
            can_pogo: false,
        },
        AttackIntent::AirForward => AttackSpec {
            intent,
            startup_seconds: 0.030,
            active_seconds: 0.105,
            recovery_seconds: 0.155,
            hitbox_offset: Vec2::new(facing * (half.x + 38.0), -2.0),
            hitbox_half_size: Vec2::new(38.0, 26.0),
            self_impulse: Vec2::new(-facing * 45.0, -25.0),
            knockback: Vec2::new(facing * 320.0, -120.0),
            damage_kind: DamageKind::Slash,
            can_pogo: false,
        },
        AttackIntent::AirBack => AttackSpec {
            // Bair: behind-facing swing. Mirrors AirForward across the
            // facing axis. A bit snappier on startup/recovery to reward
            // the player for committing to a fixed-facing reversal.
            intent,
            startup_seconds: 0.028,
            active_seconds: 0.095,
            recovery_seconds: 0.165,
            hitbox_offset: Vec2::new(-facing * (half.x + 38.0), -2.0),
            hitbox_half_size: Vec2::new(38.0, 26.0),
            self_impulse: Vec2::new(facing * 50.0, -25.0),
            knockback: Vec2::new(-facing * 340.0, -120.0),
            damage_kind: DamageKind::Slash,
            can_pogo: false,
        },
        AttackIntent::Forward | AttackIntent::Neutral => AttackSpec {
            intent,
            startup_seconds: 0.035,
            active_seconds: 0.100,
            recovery_seconds: 0.160,
            hitbox_offset: Vec2::new(facing * (half.x + 38.0), -2.0),
            hitbox_half_size: Vec2::new(38.0, 26.0),
            self_impulse: if matches!(intent, AttackIntent::Forward) {
                Vec2::new(facing * 30.0, 0.0)
            } else {
                Vec2::new(-facing * 65.0, 0.0)
            },
            knockback: Vec2::new(facing * 320.0, -120.0),
            damage_kind: DamageKind::Slash,
            can_pogo: false,
        },
    }
}

/// Hitbox for an attack spec at the player's current position.
pub fn attack_hitbox(player: &Player, spec: AttackSpec) -> Aabb {
    Aabb::new(player.pos + spec.hitbox_offset, spec.hitbox_half_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vec2;

    /// Test helper: resolve the canonical attack pipeline
    /// (`resolve_attack_intent` → `attack_spec` → `attack_hitbox`) into
    /// the final hitbox. The old `slash_hitbox` shortcut was retired
    /// with the engine cleanup; tests use this 3-line helper instead.
    fn slash_hitbox_for_test(player: &Player, axis_y: f32, forced_pogo: bool) -> Aabb {
        let intent = resolve_attack_intent(player, 0.0, axis_y, forced_pogo);
        attack_hitbox(player, attack_spec(player, intent))
    }

    #[test]
    fn forward_slash_is_in_front_of_facing_direction() {
        let mut player = Player::new(Vec2::new(100.0, 100.0));
        player.facing = 1.0;
        let right = slash_hitbox_for_test(&player, 0.0, false);
        player.facing = -1.0;
        let left = slash_hitbox_for_test(&player, 0.0, false);
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
        let pogo = slash_hitbox_for_test(&player, 0.0, true);
        // Pogo slash sits below the body — its center.y is greater
        // (Ambition uses +Y down) than the player's y.
        assert!(pogo.center().y > player.pos.y);
    }

    #[test]
    fn upward_slash_is_above_player() {
        let player = Player::new(Vec2::new(100.0, 100.0));
        let up = slash_hitbox_for_test(&player, -1.0, false);
        assert!(up.center().y < player.pos.y);
    }
}
