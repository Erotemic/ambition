//! Combat helpers and reusable damage volumes.
//!
//! Bevy should render slash previews, play hit sounds, and spawn particles, but
//! the shape of an attack is game logic. Keeping hitbox and damage semantics
//! here lets tests and future headless validators reason about combat without a
//! renderer.

pub mod slots;

use ambition_engine_core::{Aabb, AabbExt, KinematicPath, Vec2};
use ambition_entity_catalog::placements::{DamageKind, DamageTeam, HazardRespawn};

/// Damage payload shared by hitboxes and persistent damage volumes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Damage {
    pub amount: i32,
    pub knockback: Vec2,
    pub kind: DamageKind,
    pub source: DamageTeam,
    pub hitstop_seconds: f32,
}

impl Damage {
    pub fn new(amount: i32, kind: DamageKind, source: DamageTeam) -> Self {
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

    pub fn can_affect(self, target: DamageTeam) -> bool {
        self.source.can_damage(target)
    }
}

// NOTE: the old spec-layer `Hitbox` / `Hurtbox` structs were removed (2026-06-15).
// They were never constructed at runtime — the live model is `ambition_vfx::Hitbox`
// (transient strike volume) for dealing damage and the `DamageableVolumes` component
// for receiving it; every hit path resolves through `Aabb::strict_intersects`.

/// Persistent or reusable damaging area: spikes, lasers, spike balls, boss
/// patterns, enemy contact damage, and similar hazards.
#[derive(Clone, Debug, PartialEq)]
pub struct DamageVolume {
    pub id: String,
    pub aabb: Aabb,
    pub damage: Damage,
    pub respawn: HazardRespawn,
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
            damage: Damage::new(amount, DamageKind::Hazard, DamageTeam::Environment),
            respawn: HazardRespawn::Never,
            path_id: None,
            motion: None,
            enabled: true,
        }
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

/// Resolved melee swing parameters. Offsets/half-extents/impulses are authored
/// in the controlled body's local frame (`x` side/right, `y` toward feet) and
/// signed for the body's current local facing direction. Runtime callers rotate
/// them into world space with [`AttackSpec::into_world_frame`] before spawning
/// hitboxes or applying impulses.
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
    /// When set (a held weapon's swing), overrides the default per-hit damage.
    /// `None` falls back to the player's `offense.damage_multiplier`.
    pub damage_override: Option<i32>,
}

impl AttackSpec {
    pub fn total_seconds(self) -> f32 {
        self.startup_seconds + self.active_seconds + self.recovery_seconds
    }

    /// Rotate this body-local attack spec into the provided acceleration frame.
    ///
    /// Combat authoring remains relative to the controlled body: forward/back are
    /// local side, up/down are away-from-feet/toward-feet. This conversion is the
    /// single runtime seam that turns those local values into world AABBs and
    /// impulses.
    pub fn into_world_frame(mut self, frame: ambition_engine_core::AccelerationFrame) -> Self {
        self.hitbox_offset = frame.to_world(self.hitbox_offset);
        self.hitbox_half_size = frame.to_world_half(self.hitbox_half_size);
        self.self_impulse = frame.to_world(self.self_impulse);
        self.knockback = frame.to_world(self.knockback);
        self
    }

    /// Re-tune this swing to a held melee weapon's spec (the axe etc.): the
    /// weapon's windup / active / recover timing and its damage. Non-`Swipe`
    /// melee variants leave the swing unchanged. This is how a held item
    /// *replaces* the default attack with its own feel rather than just gating
    /// whether a swing happens. (Hitbox geometry is left to the directional
    /// `attack_spec_from_view` default — the player's reach already exceeds the
    /// enemy-scale `reach_px`, so importing it would *shrink* the swing.)
    pub fn with_held_melee(mut self, melee: ambition_characters::brain::MeleeActionSpec) -> Self {
        let ambition_characters::brain::MeleeActionSpec::Swipe(s) = melee else {
            return self;
        };
        self.startup_seconds = s.windup_s.max(0.0);
        self.active_seconds = s.active_s.max(0.02);
        self.recovery_seconds = s.recover_s.max(0.0);
        self.damage_override = Some(s.damage.max(1));
        self
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

/// Read-only snapshot of the player fields the combat helpers
/// (`resolve_attack_intent_from_view`, `attack_spec_from_view`,
/// `attack_hitbox_from_view`) consult. Lets cluster-aware callers
/// drive the helpers without materializing an `ae::Player`.
#[derive(Clone, Copy, Debug)]
pub struct AttackView {
    pub pos: Vec2,
    pub size: Vec2,
    pub facing: f32,
    pub on_ground: bool,
    pub wall_clinging: bool,
    pub dash_timer: f32,
    pub abilities_directional_primary: bool,
}

impl AttackView {
    fn aabb(&self) -> Aabb {
        Aabb::new(self.pos, self.size * 0.5)
    }
}

/// Resolve a directional attack intent from input + view state.
pub fn resolve_attack_intent_from_view(
    view: &AttackView,
    axis_x: f32,
    axis_y: f32,
    forced_pogo: bool,
) -> AttackIntent {
    if forced_pogo || axis_y > 0.25 {
        return if view.on_ground {
            AttackIntent::Down
        } else {
            AttackIntent::AirDown
        };
    }
    if axis_y < -0.25 {
        return if view.on_ground {
            AttackIntent::Up
        } else {
            AttackIntent::AirUp
        };
    }

    let forward = axis_x * view.facing > 0.25;
    let back = axis_x * view.facing < -0.25;
    if view.wall_clinging && back {
        return AttackIntent::WallOut;
    }
    if view.dash_timer > 0.0 && !back {
        return AttackIntent::DashForward;
    }
    if !view.on_ground {
        if back && view.abilities_directional_primary {
            return AttackIntent::AirBack;
        }
        return AttackIntent::AirForward;
    }
    if back && view.abilities_directional_primary {
        return AttackIntent::Back;
    }
    if forward {
        AttackIntent::Forward
    } else {
        AttackIntent::Neutral
    }
}

/// Build the attack spec for an already-resolved intent.
pub fn attack_spec_from_view(view: &AttackView, intent: AttackIntent) -> AttackSpec {
    let body = view.aabb();
    let facing = if view.facing < 0.0 { -1.0 } else { 1.0 };
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
            damage_override: None,
            can_pogo: false,
        },
        AttackIntent::Down => AttackSpec {
            intent,
            startup_seconds: 0.035,
            active_seconds: 0.090,
            recovery_seconds: 0.180,
            hitbox_offset: Vec2::new(facing * (half.x + 30.0), half.y - 6.0),
            hitbox_half_size: Vec2::new(30.0, 12.0),
            self_impulse: Vec2::new(0.0, 0.0),
            knockback: Vec2::new(facing * 220.0, -80.0),
            damage_kind: DamageKind::Slash,
            damage_override: None,
            can_pogo: false,
        },
        AttackIntent::AirDown => AttackSpec {
            intent,
            startup_seconds: 0.035,
            active_seconds: 0.110,
            recovery_seconds: 0.165,
            hitbox_offset: Vec2::new(0.0, half.y + 32.0),
            hitbox_half_size: Vec2::new(26.0, 34.0),
            self_impulse: Vec2::new(0.0, 35.0),
            knockback: Vec2::new(0.0, 260.0),
            damage_kind: DamageKind::Pogo,
            damage_override: None,
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
            damage_override: None,
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
            damage_override: None,
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
            damage_override: None,
            can_pogo: false,
        },
        AttackIntent::AirBack => AttackSpec {
            intent,
            startup_seconds: 0.028,
            active_seconds: 0.095,
            recovery_seconds: 0.165,
            hitbox_offset: Vec2::new(-facing * (half.x + 38.0), -2.0),
            hitbox_half_size: Vec2::new(38.0, 26.0),
            self_impulse: Vec2::new(facing * 50.0, -25.0),
            knockback: Vec2::new(-facing * 340.0, -120.0),
            damage_kind: DamageKind::Slash,
            damage_override: None,
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
            damage_override: None,
            can_pogo: false,
        },
    }
}

/// Hitbox for an attack spec at the player's current position.
pub fn attack_hitbox_from_view(view: &AttackView, spec: AttackSpec) -> Aabb {
    Aabb::new(view.pos + spec.hitbox_offset, spec.hitbox_half_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core::Vec2;

    /// Test helper: resolve the canonical attack pipeline
    /// (`resolve_attack_intent` → `attack_spec` → `attack_hitbox`) into
    /// the final hitbox. The old `slash_hitbox` shortcut was retired
    /// with the engine cleanup; tests use this 3-line helper instead.
    fn slash_hitbox_for_view(view: &AttackView, axis_y: f32, forced_pogo: bool) -> Aabb {
        let intent = resolve_attack_intent_from_view(view, 0.0, axis_y, forced_pogo);
        attack_hitbox_from_view(view, attack_spec_from_view(view, intent))
    }

    fn view_at(pos: Vec2, facing: f32) -> AttackView {
        AttackView {
            pos,
            size: ambition_engine_core::movement::default_player_body_size(),
            facing,
            on_ground: false,
            wall_clinging: false,
            dash_timer: 0.0,
            abilities_directional_primary: ambition_engine_core::AbilitySet::sandbox_all()
                .directional_primary,
        }
    }

    #[test]
    fn held_axe_retunes_swing_timing_reach_and_damage() {
        let view = view_at(Vec2::new(0.0, 0.0), 1.0);
        let base = attack_spec_from_view(&view, AttackIntent::Forward);
        let axe = base.with_held_melee(ambition_characters::brain::MeleeActionSpec::Swipe(
            ambition_characters::brain::SwipeSpec {
                windup_s: 0.22,
                active_s: 0.12,
                recover_s: 0.30,
                damage: 3,
                reach_px: 64.0,
            },
        ));
        assert!(
            (axe.startup_seconds - 0.22).abs() < 1e-6,
            "windup -> startup"
        );
        assert!(
            (axe.recovery_seconds - 0.30).abs() < 1e-6,
            "recover -> recovery"
        );
        assert_eq!(axe.damage_override, Some(3), "axe carries its own damage");
        assert!(
            axe.startup_seconds > base.startup_seconds,
            "the axe winds up slower than the default swing"
        );
    }

    #[test]
    fn forward_slash_is_in_front_of_facing_direction() {
        let pos = Vec2::new(100.0, 100.0);
        let right = slash_hitbox_for_view(&view_at(pos, 1.0), 0.0, false);
        let left = slash_hitbox_for_view(&view_at(pos, -1.0), 0.0, false);
        assert!(right.center().x > pos.x);
        assert!(left.center().x < pos.x);
    }

    #[test]
    fn damage_with_knockback_chains_builder() {
        let damage = Damage::new(2, DamageKind::Slash, DamageTeam::Player)
            .with_knockback(Vec2::new(100.0, -50.0));
        assert_eq!(damage.amount, 2);
        assert_eq!(damage.knockback, Vec2::new(100.0, -50.0));
    }

    #[test]
    fn damage_can_affect_respects_faction() {
        let player_dmg = Damage::new(1, DamageKind::Slash, DamageTeam::Player);
        // Player damage affects enemies but not other players.
        assert!(player_dmg.can_affect(DamageTeam::Enemy));
        assert!(!player_dmg.can_affect(DamageTeam::Player));
        // Environment damage affects player + enemy.
        let env_dmg = Damage::new(1, DamageKind::Hazard, DamageTeam::Environment);
        assert!(env_dmg.can_affect(DamageTeam::Player));
        assert!(env_dmg.can_affect(DamageTeam::Enemy));
    }

    #[test]
    fn forced_pogo_slash_is_below_player() {
        let pos = Vec2::new(100.0, 100.0);
        let pogo = slash_hitbox_for_view(&view_at(pos, 1.0), 0.0, true);
        // Pogo slash sits below the body — its center.y is greater
        // (Ambition uses +Y down) than the player's y.
        assert!(pogo.center().y > pos.y);
    }

    #[test]
    fn upward_slash_is_above_player() {
        let pos = Vec2::new(100.0, 100.0);
        let up = slash_hitbox_for_view(&view_at(pos, 1.0), -1.0, false);
        assert!(up.center().y < pos.y);
    }

    #[test]
    fn attack_spec_world_conversion_is_frame_equivalent() {
        let local_view = view_at(Vec2::ZERO, 1.0);
        let local = attack_spec_from_view(&local_view, AttackIntent::AirDown);
        for gravity_dir in [
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(-1.0, 0.0),
        ] {
            let frame = ambition_engine_core::AccelerationFrame::new(gravity_dir);
            let world = local.into_world_frame(frame);
            let offset_local = frame.to_local(world.hitbox_offset);
            let impulse_local = frame.to_local(world.self_impulse);
            let knock_local = frame.to_local(world.knockback);
            assert!((offset_local - local.hitbox_offset).length() < 1e-3);
            assert!((impulse_local - local.self_impulse).length() < 1e-3);
            assert!((knock_local - local.knockback).length() < 1e-3);
        }
    }

    /// C4 symmetry: the slash effect orients to the world `hitbox_offset`
    /// (player→hitbox), so under each of the symmetry-room's four gravities a
    /// down-tilt stays a ~horizontal forward poke, a down-air points toward the
    /// feet, and an up-attack points toward the head — i.e. the effect lives in
    /// the player's reference frame, not screen space.
    #[test]
    fn slash_strike_direction_is_gravity_relative_under_c4() {
        let view = view_at(Vec2::ZERO, 1.0);
        for gravity_dir in [
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(-1.0, 0.0),
        ] {
            let frame = ambition_engine_core::AccelerationFrame::new(gravity_dir);
            let dir = |intent| {
                attack_spec_from_view(&view, intent)
                    .into_world_frame(frame)
                    .hitbox_offset
                    .normalize_or_zero()
            };
            // down-tilt: mostly perpendicular to gravity (a forward/horizontal poke).
            assert!(
                dir(AttackIntent::Down).dot(gravity_dir).abs() < 0.5,
                "down-tilt should read ~horizontal under gravity {gravity_dir:?}"
            );
            // down-air: toward the feet (along gravity).
            assert!(
                dir(AttackIntent::AirDown).dot(gravity_dir) > 0.7,
                "down-air should point toward feet under gravity {gravity_dir:?}"
            );
            // up: toward the head (opposite gravity).
            assert!(
                dir(AttackIntent::Up).dot(gravity_dir) < -0.7,
                "up should point toward head under gravity {gravity_dir:?}"
            );
        }
    }
}
